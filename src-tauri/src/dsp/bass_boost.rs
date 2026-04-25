/// 低频增强模块
/// 基于 FFT 的频域低频增益滤波器，对截止频率以下的频段施加指定增益

use crate::dsp::next_power_of_two;
use rustfft::{FftPlanner, num_complex::Complex64};
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// 全局 FFT 计划器缓存
static FFT_PLANNER: Lazy<Mutex<FftPlanner<f64>>> = Lazy::new(|| Mutex::new(FftPlanner::new()));

/// 应用 Hann 窗口函数减少频谱泄漏
fn apply_hann_window(samples: &[f64], buffer: &mut [Complex64]) {
    let n = samples.len();
    for (i, &sample) in samples.iter().enumerate() {
        let window = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos());
        buffer[i] = Complex64::new(sample * window, 0.0);
    }
}

/// 对音频做 FFT，对截止频率以下的频段施加增益，再做 IFFT
/// - `samples`: 输入音频采样数据
/// - `sample_rate`: 采样率
/// - `gain_db`: 增益（dB），范围 0~20，默认 6
/// - `cutoff_freq`: 截止频率（Hz），范围 100~500，默认 300
pub fn apply(samples: &[f64], sample_rate: u32, gain_db: f64, cutoff_freq: f64) -> Vec<f64> {
    if samples.is_empty() {
        return Vec::new();
    }

    let original_len = samples.len();
    let fft_len = next_power_of_two(original_len);

    let mut buffer: Vec<Complex64> = vec![Complex64::new(0.0, 0.0); fft_len];
    apply_hann_window(samples, &mut buffer);

    let mut planner = FFT_PLANNER.lock().unwrap();
    let fft = planner.plan_fft_forward(fft_len);
    fft.process(&mut buffer);

    let bin_resolution = sample_rate as f64 / fft_len as f64;
    let cutoff_bin = (cutoff_freq / bin_resolution).ceil() as usize;

    let gain_linear = 10.0_f64.powf(gain_db / 20.0);

    for i in 0..cutoff_bin {
        if i < fft_len {
            buffer[i] *= gain_linear;
        }
        let mirror = fft_len - i;
        if mirror < fft_len && i > 0 {
            buffer[mirror] *= gain_linear;
        }
    }

    let ifft = planner.plan_fft_inverse(fft_len);
    ifft.process(&mut buffer);

    let scale = 1.0 / fft_len as f64;
    buffer[..original_len]
        .iter()
        .map(|c| c.re * scale)
        .collect()
}
