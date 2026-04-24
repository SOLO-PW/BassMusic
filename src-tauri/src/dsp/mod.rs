/// DSP（数字信号处理）模块
/// 为听障人士提供音频增强功能，核心处理链：
/// BassBoost（低频增益）→ FrequencyShift（高频下移）→ Compressor（动态压缩）

pub mod bass_boost;
pub mod compressor;
pub mod frequency_shift;
pub mod pipeline;

pub use pipeline::EnhanceParams;

/// 将输入长度向上对齐到最近的 2 的幂
pub fn next_power_of_two(n: usize) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 生成指定频率的正弦波测试信号
    fn generate_sine(freq: f64, sample_rate: u32, duration_secs: f64) -> Vec<f64> {
        let num_samples = (sample_rate as f64 * duration_secs) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (2.0 * std::f64::consts::PI * freq * t).sin()
            })
            .collect()
    }

    /// 测试 bass_boost 对低频信号确实增强了幅度
    #[test]
    fn test_bass_boost_increases_low_freq_amplitude() {
        let sample_rate = 44100;
        // 生成 100Hz 低频正弦波（在截止频率 300Hz 以下）
        let input = generate_sine(100.0, sample_rate, 0.05);
        let input_rms: f64 = input.iter().map(|s| s * s).sum::<f64>() / input.len() as f64;

        let output = bass_boost::apply(&input, sample_rate, 6.0, 300.0);
        let output_rms: f64 = output.iter().map(|s| s * s).sum::<f64>() / output.len() as f64;

        // 增益 6dB 约等于振幅翻倍，RMS 应该增大约 4 倍
        // 考虑 FFT 窗口效应，至少应明显增大
        assert!(
            output_rms > input_rms,
            "低频信号经 bass_boost 后 RMS 应增大：input_rms={}, output_rms={}",
            input_rms,
            output_rms
        );
    }

    /// 测试 frequency_shift 输出长度与输入一致
    #[test]
    fn test_frequency_shift_output_length() {
        let sample_rate = 44100;
        let input = generate_sine(4000.0, sample_rate, 0.05);
        let output = frequency_shift::apply(&input, sample_rate, 0.5);
        assert_eq!(
            output.len(),
            input.len(),
            "频移输出长度应与输入一致"
        );
    }

    /// 测试 frequency_shift 空输入返回空输出
    #[test]
    fn test_frequency_shift_empty_input() {
        let output = frequency_shift::apply(&[], 44100, 0.5);
        assert!(output.is_empty());
    }

    /// 测试 compressor 输出动态范围小于输入
    /// 使用稳态段 RMS 对比，避免 attack 时间导致峰值不变的问题
    #[test]
    fn test_compressor_reduces_dynamic_range() {
        let sample_rate = 44100;
        // 构造足够长的响亮信号，让压缩器进入稳态
        // 前 500ms 让 envelope 收敛，后 100ms 测量稳态输出
        let loud: Vec<f64> = (0..26460).map(|_| 0.9).collect(); // ~600ms

        let output = compressor::apply(&loud, sample_rate, 3.0, -20.0, 10.0, 100.0);

        // 取最后 4410 个样本（稳态段）的 RMS
        let steady_state = &output[output.len() - 4410..];
        let output_rms: f64 =
            (steady_state.iter().map(|s| s * s).sum::<f64>() / steady_state.len() as f64).sqrt();
        let input_rms: f64 = 0.9; // 输入是恒定 0.9

        // 压缩器应降低响亮信号的 RMS
        // 0.9 ≈ -0.9dB，超过 -20dB 阈值，压缩比 3:1
        // 稳态增益 ≈ (threshold - envelope) * (1 - 1/ratio) ≈ -19.1 * 0.667 ≈ -12.7dB
        assert!(
            output_rms < input_rms,
            "压缩后稳态 RMS 应减小：input_rms={:.4}, output_rms={:.4}",
            input_rms,
            output_rms
        );
    }

    /// 测试 pipeline 默认参数处理不崩溃且输出长度正确
    #[test]
    fn test_pipeline_default_params() {
        let sample_rate = 44100;
        let input = generate_sine(1000.0, sample_rate, 0.05);
        let params = EnhanceParams::default();
        let output = pipeline::process(&input, sample_rate, &params);

        assert_eq!(
            output.len(),
            input.len(),
            "Pipeline 输出长度应与输入一致"
        );
        // 输出不应全为零
        let has_nonzero = output.iter().any(|&s| s.abs() > 1e-10);
        assert!(has_nonzero, "Pipeline 输出不应全为零");
    }

    /// 测试 pipeline 分块处理
    #[test]
    fn test_pipeline_chunk_processing() {
        let sample_rate = 44100;
        let input = generate_sine(1000.0, sample_rate, 0.05);
        let params = EnhanceParams::default();
        let output = pipeline::process_chunk(&input, sample_rate, &params);

        assert_eq!(
            output.len(),
            input.len(),
            "分块处理输出长度应与输入一致"
        );
    }

    /// 测试 EnhanceParams 默认值
    #[test]
    fn test_enhance_params_default() {
        let params = EnhanceParams::default();
        assert!((params.bass_gain_db - 6.0).abs() < 1e-6);
        assert!((params.cutoff_freq - 300.0).abs() < 1e-6);
        assert!((params.shift_ratio - 0.5).abs() < 1e-6);
        assert!((params.compress_ratio - 3.0).abs() < 1e-6);
        assert!((params.threshold_db - (-20.0)).abs() < 1e-6);
        assert!((params.attack_ms - 10.0).abs() < 1e-6);
        assert!((params.release_ms - 100.0).abs() < 1e-6);
        assert!((params.output_volume - 1.0).abs() < 1e-6);
    }

    /// 测试空输入不会 panic
    #[test]
    fn test_empty_input_no_panic() {
        let params = EnhanceParams::default();
        let sr = 44100u32;

        let out_bass = bass_boost::apply(&[], sr, 6.0, 300.0);
        assert!(out_bass.is_empty());

        let out_shift = frequency_shift::apply(&[], sr, 0.5);
        assert!(out_shift.is_empty());

        let out_comp = compressor::apply(&[], sr, 3.0, -20.0, 10.0, 100.0);
        assert!(out_comp.is_empty());

        let out_pipe = pipeline::process(&[], sr, &params);
        assert!(out_pipe.is_empty());
    }
}
