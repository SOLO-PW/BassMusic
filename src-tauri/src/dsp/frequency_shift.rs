/// 频移模块
/// 将高频段信号下移到低频区域，帮助高频听力损失用户感知高频信息

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

    // 频域下移：将高频 bin 的内容复制到低频 bin 位置
    // shift_ratio 决定了目标频率与原频率的比值
    // 新频谱：new_bin[i] = old_bin[(i / shift_ratio).round()]
    // 只处理前半部分（正频率），后半部分（负频率）取共轭对称
    let half = fft_len / 2;
    let mut shifted = vec![Complex64::new(0.0, 0.0); fft_len];

    for i in 0..=half {
        // 计算该 bin 对应的原始频率位置
        let src_bin = ((i as f64) / shift_ratio).round() as usize;
        if src_bin <= half {
            shifted[i] = buffer[src_bin];
            // 对称的负频率分量
            if i > 0 && i < half {
                shifted[fft_len - i] = buffer[fft_len - src_bin];
            }
        }
    }

    // 逆向 FFT
    let ifft = planner.plan_fft_inverse(fft_len);
    ifft.process(&mut shifted);

    // IFFT 归一化并截断回原始长度
    let scale = 1.0 / fft_len as f64;
    shifted[..original_len]
        .iter()
        .map(|c| c.re * scale)
        .collect()
}
