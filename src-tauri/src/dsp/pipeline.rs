/// 音频增强处理链 Pipeline
/// 组合 BassBoost → FrequencyShift → Compressor 为完整处理链

use crate::dsp::{bass_boost, compressor, frequency_shift};

/// 增强参数结构体，可通过 Tauri IPC 从前端传入
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhanceParams {
    /// 低频增益（dB），范围 0~20，默认 6.0
    pub bass_gain_db: f64,
    /// 截止频率（Hz），范围 100~500，默认 300.0
    pub cutoff_freq: f64,
    /// 频移比率，范围 0.1~1.0，默认 0.5
    pub shift_ratio: f64,
    /// 压缩比，范围 1~10，默认 3.0
    pub compress_ratio: f64,
    /// 输出音量，默认 1.0（100%）
    pub output_volume: f64,
}

impl Default for EnhanceParams {
    fn default() -> Self {
        Self {
            bass_gain_db: 6.0,
            cutoff_freq: 300.0,
            shift_ratio: 0.5,
            compress_ratio: 3.0,
            output_volume: 1.0,
        }
    }
}

/// 完整处理链：BassBoost → FrequencyShift → Compressor → 音量缩放
/// - `samples`: 输入音频采样数据
/// - `sample_rate`: 采样率
/// - `params`: 增强参数
pub fn process(samples: &[f64], sample_rate: u32, params: &EnhanceParams) -> Vec<f64> {
    if samples.is_empty() {
        return Vec::new();
    }

    // 第一步：低频增强
    let boosted = bass_boost::apply(samples, sample_rate, params.bass_gain_db, params.cutoff_freq);

    // 第二步：频移下移
    let shifted = frequency_shift::apply(&boosted, sample_rate, params.shift_ratio);

    // 第三步：动态压缩（阈值 -20dB，启动 10ms，释放 100ms）
    let compressed = compressor::apply(
        &shifted,
        sample_rate,
        params.compress_ratio,
        -20.0,
        10.0,
        100.0,
    );

    // 第四步：输出音量缩放
    apply_volume(&compressed, params.output_volume)
}

/// 分块处理，用于实时音频流场景
/// 每个 chunk 独立处理，适合流式音频场景
/// - `chunk`: 当前音频块
/// - `sample_rate`: 采样率
/// - `params`: 增强参数
pub fn process_chunk(chunk: &[f64], sample_rate: u32, params: &EnhanceParams) -> Vec<f64> {
    // 当前实现：每个 chunk 独立做完整处理链
    // 后续可引入交叉淡化和状态保持以消除块边界伪影
    process(chunk, sample_rate, params)
}

/// 对采样数据施加音量缩放
fn apply_volume(samples: &[f64], volume: f64) -> Vec<f64> {
    samples.iter().map(|&s| s * volume).collect()
}
