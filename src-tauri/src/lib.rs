/// BassMusic 应用核心入口
/// 注册所有 Tauri commands 并启动应用

pub mod audio;
pub mod dsp;
pub mod realtime;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

/// 应用全局状态，持有各子模块的共享状态
struct AppState {
    realtime: realtime::RealtimeState,
    /// 文件转化进行中标志，防止重复转化
    converting: Arc<AtomicBool>,
}

/// 音频文件低频增强转化命令（异步）
/// 接收输入文件路径、输出路径和增强参数，在独立线程执行解码→DSP增强→编码流程
/// 通过 Tauri Event 推送处理进度
#[tauri::command]
async fn convert_audio_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input_path: String,
    output_path: String,
    params: dsp::pipeline::EnhanceParams,
) -> Result<String, String> {
    if state.converting.load(Ordering::SeqCst) {
        return Err("已有文件正在转化中，请等待完成".to_string());
    }

    if !audio::is_supported_format(&input_path) {
        return Err(
            "不支持的音频格式，请使用 WAV、MP3 或 FLAC 文件".to_string(),
        );
    }

    state.converting.store(true, Ordering::SeqCst);

    let app_clone = app.clone();
    let converting = state.converting.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let _ = app_clone.emit("convert-progress", 0u32);

        let (samples, sample_rate) = match audio::decode_audio_file(&input_path) {
            Ok(r) => r,
            Err(e) => {
                converting.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };
        let _ = app_clone.emit("convert-progress", 10u32);

        let total_samples = samples.len();
        let chunk_size = 8192; // 增大块大小，减少处理次数
        let mut enhanced = Vec::with_capacity(total_samples);
        let chunks = (total_samples + chunk_size - 1) / chunk_size;

        for (i, chunk) in samples.chunks(chunk_size).enumerate() {
            let processed = dsp::pipeline::process_chunk(chunk, sample_rate, &params);
            enhanced.extend_from_slice(&processed);

            // 减少进度更新频率，减少 IPC 开销
            if (i + 1) % 4 == 0 || (i + 1) == chunks {
                let progress = 10 + ((i + 1) * 80 / chunks.max(1)) as u32;
                let _ = app_clone.emit("convert-progress", progress.min(90));
            }
        }

        let _ = app_clone.emit("convert-progress", 92u32);

        if let Err(e) = audio::encode_wav_file(&output_path, &enhanced, sample_rate) {
            converting.store(false, Ordering::SeqCst);
            return Err(e);
        }

        let _ = app_clone.emit("convert-progress", 100u32);
        converting.store(false, Ordering::SeqCst);

        Ok(format!("转化完成！输出文件: {}", output_path))
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?;

    result
}

/// 启动实时音频增强
#[tauri::command]
fn start_realtime(state: tauri::State<'_, AppState>) -> Result<String, String> {
    realtime::start_realtime_enhance(&state.realtime)?;
    Ok("实时增强已启动".to_string())
}

/// 停止实时音频增强
#[tauri::command]
fn stop_realtime(state: tauri::State<'_, AppState>) -> Result<String, String> {
    realtime::stop_realtime_enhance(&state.realtime)?;
    Ok("实时增强已停止".to_string())
}

/// 热更新实时增强参数
#[tauri::command]
fn update_realtime_params(
    state: tauri::State<'_, AppState>,
    params: dsp::pipeline::EnhanceParams,
) -> Result<String, String> {
    let mut current = state.realtime.params.lock().unwrap();
    *current = params;
    Ok("参数已更新".to_string())
}

/// 应用主入口函数，桌面端和移动端共享
/// mobile_entry_point 宏处理移动端编译时的入口适配
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::<tauri::Wry>::new()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            realtime: realtime::RealtimeState::new(),
            converting: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            convert_audio_file,
            start_realtime,
            stop_realtime,
            update_realtime_params
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用时出错");
}
