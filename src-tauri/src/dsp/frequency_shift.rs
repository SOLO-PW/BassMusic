/// 频移模块
/// 将高频段信号下移到低频区域，帮助高频听力损失用户感知高频信息

use crate::dsp::next_power_of_two;
use rustfft::{FftPlanner, num_complex::Complex64};

/// 将高频段信号下移到低频区域（频域下移）
/// - `samples`: 输入音频采样数据
/// - `sample_rate`: 采样率
/// - `shift_ratio`: 频移比率，范围 0.1~1.0，默认 0.5
///   值为 0.5 时，原频率 f 的信号会被移到 f*0.5 的位置
pub fn apply(samples: &[f64], _sample_rate: u32, shift_ratio: f64) -> Vec<f64> {
    if samples.is_empty() {
        return Vec::new();
    }

    let original_len = samples.len();
    let fft_len = next_power_of_two(original_len);

    let mut buffer: Vec<Complex64> = samples
        .iter()
        .map(|&s| Complex64::new(s, 0.0))
        .collect();
    buffer.resize(fft_len, Complex64::new(0.0, 0.0));

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);
    fft.process(&mut buffer);

    let half = fft_len / 2;
    let mut shifted = vec![Complex64::new(0.0, 0.0); fft_len];

    for i in 0..=half {
        let src_bin = ((i as f64) / shift_ratio).round() as usize;
        if src_bin <= half {
            shifted[i] = buffer[src_bin];
            if i > 0 && i < half {
                shifted[fft_len - i] = buffer[fft_len - src_bin];
            }
        }
    }

    let ifft = planner.plan_fft_inverse(fft_len);
    ifft.process(&mut shifted);

    let scale = 1.0 / fft_len as f64;
    shifted[..original_len]
        .iter()
        .map(|c| c.re * scale)
        .collect()
}
