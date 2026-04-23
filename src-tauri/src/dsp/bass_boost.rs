/// 低频增强模块
/// 基于 FFT 的频域低频增益滤波器，对截止频率以下的频段施加指定增益

use rustfft::{FftPlanner, num_complex::Complex64};

/// 将输入长度向上对齐到最近的 2 的幂
fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut v = n - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v |= v >> 32;
    v + 1
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

    // 零填充到 2 的幂长度
    let mut buffer: Vec<Complex64> = samples
        .iter()
        .map(|&s| Complex64::new(s, 0.0))
        .collect();
    buffer.resize(fft_len, Complex64::new(0.0, 0.0));

    // 正向 FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);
    fft.process(&mut buffer);

    // 计算截止频率对应的 bin 索引
    let bin_resolution = sample_rate as f64 / fft_len as f64;
    let cutoff_bin = (cutoff_freq / bin_resolution).ceil() as usize;

    // 增益线性值
    let gain_linear = 10.0_f64.powf(gain_db / 20.0);

    // 对截止频率以下的频段施加增益（对称处理正负频率）
    for i in 0..cutoff_bin {
        if i < fft_len {
            buffer[i] *= gain_linear;
        }
        // 对称频率分量
        let mirror = fft_len - i;
        if mirror < fft_len && i > 0 {
            buffer[mirror] *= gain_linear;
        }
    }

    // 逆向 FFT
    let ifft = planner.plan_fft_inverse(fft_len);
    ifft.process(&mut buffer);

    // IFFT 结果需要除以 N 归一化，截断回原始长度
    let scale = 1.0 / fft_len as f64;
    buffer[..original_len]
        .iter()
        .map(|c| c.re * scale)
        .collect()
}
