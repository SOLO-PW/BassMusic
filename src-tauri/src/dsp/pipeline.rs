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
    /// 压缩器阈值（dB），默认 -20.0
    #[serde(default = "default_threshold_db")]
    pub threshold_db: f64,
    /// 压缩器启动时间（ms），默认 10.0
    #[serde(default = "default_attack_ms")]
    pub attack_ms: f64,
    /// 压缩器释放时间（ms），默认 100.0
    #[serde(default = "default_release_ms")]
    pub release_ms: f64,
    /// 输出音量，默认 1.0（100%）
    pub output_volume: f64,
}

fn default_threshold_db() -> f64 { -20.0 }
fn default_attack_ms() -> f64 { 10.0 }
fn default_release_ms() -> f64 { 100.0 }

impl Default for EnhanceParams {
    fn default() -> Self {
        Self {
            bass_gain_db: 6.0,
            cutoff_freq: 300.0,
            shift_ratio: 0.5,
            compress_ratio: 3.0,
            threshold_db: -20.0,
            attack_ms: 10.0,
            release_ms: 100.0,
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

    let boosted = bass_boost::apply(samples, sample_rate, params.bass_gain_db, params.cutoff_freq);

    let shifted = frequency_shift::apply(&boosted, sample_rate, params.shift_ratio);

    let compressed = compressor::apply(
        &shifted,
        sample_rate,
        params.compress_ratio,
        params.threshold_db,
        params.attack_ms,
        params.release_ms,
    );

    apply_volume(&compressed, params.output_volume)
}

/// 分块处理，用于实时音频流场景
/// 每个 chunk 独立处理，适合流式音频场景
/// - `chunk`: 当前音频块
/// - `sample_rate`: 采样率
/// - `params`: 增强参数
pub fn process_chunk(chunk: &[f64], sample_rate: u32, params: &EnhanceParams) -> Vec<f64> {
    process(chunk, sample_rate, params)
}

/// 对采样数据施加音量缩放
fn apply_volume(samples: &[f64], volume: f64) -> Vec<f64> {
    samples.iter().map(|&s| s * volume).collect()
}
