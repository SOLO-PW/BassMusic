/// 实时音频捕获与增强模块
/// 通过 WASAPI Loopback 捕获系统音频，经 DSP 管线增强后播放
/// 流程：系统音频捕获 → crossbeam 缓冲区 → DSP 处理 → 输出播放

use crate::dsp;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// cpal::Stream 的线程安全包装器
///
/// cpal 出于跨平台兼容性将 Stream 标记为 !Send（内部含 PhantomData<*mut ()>），
/// 但在 Windows WASAPI 平台上，底层 COM 接口在 MTA 模式下是线程安全的，
/// 可以安全地跨线程传递和 drop。
pub struct SendableStream(#[allow(dead_code)] cpal::Stream);

// SAFETY: Windows WASAPI 的 IAudioClient 等 COM 接口在 MTA 模式下线程安全，
// cpal 的 StreamInner 仅包含 COM 指针和 JoinHandle，均满足 Send 语义。
// 此 unsafe impl 仅在 Windows WASAPI 场景下安全，其他平台需另行评估。
unsafe impl Send for SendableStream {}

/// 预分配的音频处理缓冲区，避免在实时回调中动态分配内存
struct AudioBuffers {
    /// 单声道混合输入缓冲区
    mono_input: Vec<f64>,
}

impl AudioBuffers {
    fn new() -> Self {
        Self {
            mono_input: Vec::new(),
        }
    }

    /// 确保 mono_input 缓冲区至少能容纳 frame_count 个采样
    fn ensure_capacity(&mut self, frame_count: usize) {
        if self.mono_input.capacity() < frame_count {
            self.mono_input.reserve_exact(frame_count - self.mono_input.capacity());
        }
        self.mono_input.clear();
    }
}

/// 实时增强器的共享状态
///
/// 通过 Arc + AtomicBool/Mutex 实现跨线程安全访问：
/// - 音频回调线程读写 running 和 params
/// - Tauri 主线程通过 Command 修改状态
pub struct RealtimeState {
    /// 运行标志，控制音频流的启停
    pub running: Arc<AtomicBool>,
    /// 增强参数，支持运行时热更新
    pub params: Arc<Mutex<dsp::pipeline::EnhanceParams>>,
    /// 音频流句柄（drop 时自动停止流）
    pub streams: Mutex<Vec<SendableStream>>,
    /// 预分配的音频处理缓冲区
    buffers: Arc<Mutex<AudioBuffers>>,
}

impl RealtimeState {
    /// 创建默认的实时增强器状态
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            params: Arc::new(Mutex::new(dsp::pipeline::EnhanceParams::default())),
            streams: Mutex::new(Vec::new()),
            buffers: Arc::new(Mutex::new(AudioBuffers::new())),
        }
    }
}

/// 启动实时音频增强
///
/// 在默认输出设备上创建 WASAPI Loopback 输入流捕获系统音频，
/// 经 DSP 管线处理后通过输出流播放增强音频。
///
/// 注意：在同一设备上同时捕获和播放会产生反馈回路，
/// 实际使用时建议通过耳机播放增强音频以避免此问题。
pub fn start_realtime_enhance(state: &RealtimeState) -> Result<(), String> {
    // 防止重复启动
    if state.running.load(Ordering::SeqCst) {
        return Err("实时增强已在运行中".to_string());
    }

    let host = cpal::host_from_id(cpal::HostId::Wasapi)
        .map_err(|e| format!("无法初始化 WASAPI Host: {}", e))?;

    // 获取默认输出设备（WASAPI Loopback 需要在输出设备上创建输入流）
    let device = host
        .default_output_device()
        .ok_or("无法获取音频输出设备，请检查音频设备连接")?;

    // 获取 Loopback 输入配置（在输出设备上获取输入配置即启用 Loopback 模式）
    let input_config = device
        .default_input_config()
        .map_err(|e| format!("无法获取 Loopback 输入配置: {}。请确认系统支持 WASAPI Loopback", e))?;

    // 获取输出配置
    let output_config = device
        .default_output_config()
        .map_err(|e| format!("无法获取音频输出配置: {}", e))?;

    let sample_rate = input_config.sample_rate().0;
    let in_channels = input_config.channels();
    let out_channels = output_config.channels();

    // 校验输入输出采样率一致
    if input_config.sample_rate().0 != output_config.sample_rate().0 {
        return Err(format!(
            "输入输出采样率不匹配: 输入={}, 输出={}",
            input_config.sample_rate().0,
            output_config.sample_rate().0
        ));
    }

    // 创建有界通道作为音频缓冲区（容量约 2 秒，平衡延迟与抗抖动）
    let buffer_capacity = (sample_rate as usize) * in_channels as usize * 2;
    let (tx, rx) = crossbeam_channel::bounded(buffer_capacity);

    let running = state.running.clone();
    let params = state.params.clone();
    let buffers = state.buffers.clone();

    // 构建输入流（Loopback 捕获系统音频）
    let input_stream = build_input_stream(&device, &input_config, tx, running.clone())?;

    // 构建输出流（播放增强后音频）
    let output_stream = build_output_stream(
        &device,
        &output_config,
        rx,
        running.clone(),
        params,
        buffers,
        sample_rate,
        in_channels,
        out_channels,
    )?;

    // 启动音频流
    input_stream
        .play()
        .map_err(|e| format!("启动音频捕获失败: {}", e))?;
    output_stream
        .play()
        .map_err(|e| format!("启动音频播放失败: {}", e))?;

    // 保存流句柄（cpal::Stream 在 drop 时自动停止）
    *state.streams.lock().unwrap() = vec![
        SendableStream(input_stream),
        SendableStream(output_stream),
    ];
    state.running.store(true, Ordering::SeqCst);

    Ok(())
}

/// 停止实时音频增强
///
/// 设置运行标志为 false，然后清除流句柄触发 drop 自动停止音频流
pub fn stop_realtime_enhance(state: &RealtimeState) -> Result<(), String> {
    if !state.running.load(Ordering::SeqCst) {
        return Err("实时增强未在运行".to_string());
    }

    state.running.store(false, Ordering::SeqCst);
    // 丢弃流句柄，cpal::Stream drop 时自动停止音频流
    state.streams.lock().unwrap().clear();

    Ok(())
}

/// 构建输入流，将捕获的音频采样写入 crossbeam 通道
///
/// 支持 F32/I16/U16 三种采样格式，自动转换为 f64 写入通道
fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    tx: crossbeam_channel::Sender<f64>,
    running: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    let stream_config = config.config();

    match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !running.load(Ordering::SeqCst) {
                        return;
                    }
                    for &sample in data {
                        if tx.try_send(sample as f64).is_err() {
                            break; // 缓冲区满，丢弃剩余采样
                        }
                    }
                },
                |err| eprintln!("音频捕获错误: {}", err),
                None,
            ),
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !running.load(Ordering::SeqCst) {
                        return;
                    }
                    for &sample in data {
                        if tx.try_send(sample as f64 / i16::MAX as f64).is_err() {
                            break;
                        }
                    }
                },
                |err| eprintln!("音频捕获错误: {}", err),
                None,
            ),
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if !running.load(Ordering::SeqCst) {
                        return;
                    }
                    for &sample in data {
                        let normalized = (sample as f64 - 32768.0) / 32768.0;
                        if tx.try_send(normalized).is_err() {
                            break;
                        }
                    }
                },
                |err| eprintln!("音频捕获错误: {}", err),
                None,
            ),
        fmt => return Err(format!("不支持的输入采样格式: {:?}", fmt)),
    }
    .map_err(|e| format!("创建输入流失败: {}", e))
}

/// 构建输出流，从通道读取数据经 DSP 处理后播放
///
/// 读取多声道采样 → 混合为单声道 → DSP 增强 → 扩展为输出声道数
fn build_output_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    rx: crossbeam_channel::Receiver<f64>,
    running: Arc<AtomicBool>,
    params: Arc<Mutex<dsp::pipeline::EnhanceParams>>,
    buffers: Arc<Mutex<AudioBuffers>>,
    sample_rate: u32,
    in_channels: u16,
    out_channels: u16,
) -> Result<cpal::Stream, String> {
    let stream_config = config.config();

    match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    process_output_buffer(
                        data,
                        &rx,
                        &running,
                        &params,
                        &buffers,
                        sample_rate,
                        in_channels,
                        out_channels,
                    );
                },
                |err| eprintln!("音频播放错误: {}", err),
                None,
            ),
        cpal::SampleFormat::I16 => device
            .build_output_stream(
                &stream_config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let frame_count = data.len() / out_channels as usize;
                    let mut f32_buf = vec![0.0f32; frame_count * out_channels as usize];
                    process_output_buffer(
                        &mut f32_buf,
                        &rx,
                        &running,
                        &params,
                        &buffers,
                        sample_rate,
                        in_channels,
                        out_channels,
                    );
                    for (i, sample) in data.iter_mut().enumerate() {
                        *sample = (f32_buf[i].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    }
                },
                |err| eprintln!("音频播放错误: {}", err),
                None,
            ),
        cpal::SampleFormat::U16 => device
            .build_output_stream(
                &stream_config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let frame_count = data.len() / out_channels as usize;
                    let mut f32_buf = vec![0.0f32; frame_count * out_channels as usize];
                    process_output_buffer(
                        &mut f32_buf,
                        &rx,
                        &running,
                        &params,
                        &buffers,
                        sample_rate,
                        in_channels,
                        out_channels,
                    );
                    for (i, sample) in data.iter_mut().enumerate() {
                        *sample = ((f32_buf[i].clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f32)
                            as u16;
                    }
                },
                |err| eprintln!("音频播放错误: {}", err),
                None,
            ),
        fmt => return Err(format!("不支持的输出采样格式: {:?}", fmt)),
    }
    .map_err(|e| format!("创建输出流失败: {}", e))
}

/// 处理输出缓冲区：从通道读取 → 混合单声道 → DSP 增强 → 填充输出
///
/// 核心实时处理逻辑，在音频回调线程中执行：
/// 1. 从 crossbeam 通道逐帧读取多声道采样并混合为单声道
/// 2. 通过 DSP pipeline 进行增强处理
/// 3. 将单声道增强结果扩展到输出声道数
fn process_output_buffer(
    data: &mut [f32],
    rx: &crossbeam_channel::Receiver<f64>,
    running: &AtomicBool,
    params: &Mutex<dsp::pipeline::EnhanceParams>,
    buffers: &Mutex<AudioBuffers>,
    sample_rate: u32,
    in_channels: u16,
    out_channels: u16,
) {
    if !running.load(Ordering::SeqCst) {
        data.fill(0.0);
        return;
    }

    let frame_count = data.len() / out_channels as usize;

    let current_params = params.lock().unwrap().clone();

    let enhanced = {
        let mut buf = buffers.lock().unwrap();
        buf.ensure_capacity(frame_count);

        for _ in 0..frame_count {
            let mut sum = 0.0;
            let mut count = 0u32;
            for _ in 0..in_channels {
                if let Ok(sample) = rx.try_recv() {
                    sum += sample;
                    count += 1;
                }
            }
            buf.mono_input.push(if count > 0 {
                sum / count as f64
            } else {
                0.0
            });
        }

        dsp::pipeline::process_chunk(&buf.mono_input, sample_rate, &current_params)
    };

    for (i, frame) in data.chunks_mut(out_channels as usize).enumerate() {
        let sample = if i < enhanced.len() {
            enhanced[i] as f32
        } else {
            0.0
        };
        for ch in frame.iter_mut() {
            *ch = sample;
        }
    }
}
