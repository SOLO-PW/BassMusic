/// 动态范围压缩模块
/// 压缩音频动态范围，使弱信号更易感知，避免强信号刺耳

/// 标准动态范围压缩器（envelope follower + gain computation）
/// - `samples`: 输入音频采样数据
/// - `sample_rate`: 采样率
/// - `ratio`: 压缩比，范围 1~10，默认 3（表示 3:1）
/// - `threshold_db`: 阈值（dB），默认 -20
/// - `attack_ms`: 启动时间（ms），默认 10
/// - `release_ms`: 释放时间（ms），默认 100
pub fn apply(
    samples: &[f64],
    sample_rate: u32,
    ratio: f64,
    threshold_db: f64,
    attack_ms: f64,
    release_ms: f64,
) -> Vec<f64> {
    if samples.is_empty() {
        return Vec::new();
    }

    let attack_coeff = time_to_coeff(attack_ms, sample_rate);
    let release_coeff = time_to_coeff(release_ms, sample_rate);

    let mut envelope_db = -120.0_f64; // 初始包络设为极低值
    let mut output = Vec::with_capacity(samples.len());

    for &sample in samples {
        let input_db = amplitude_to_db(sample.abs());

        // 包络跟随：根据信号是否超过当前包络选择 attack/release
        if input_db > envelope_db {
            envelope_db = attack_coeff * envelope_db + (1.0 - attack_coeff) * input_db;
        } else {
            envelope_db = release_coeff * envelope_db + (1.0 - release_coeff) * input_db;
        }

        // 计算增益衰减量
        let gain_db = if envelope_db > threshold_db {
            // 超过阈值部分按压缩比衰减
            (threshold_db - envelope_db) * (1.0 - 1.0 / ratio)
        } else {
            0.0
        };

        let gain_linear = db_to_amplitude(gain_db);
        output.push(sample * gain_linear);
    }

    output
}

/// 将时间常数（ms）转换为 IIR 滤波器系数
/// 系数越接近 1，响应越慢（平滑）
fn time_to_coeff(time_ms: f64, sample_rate: u32) -> f64 {
    let time_secs = time_ms / 1000.0;
    if time_secs <= 0.0 {
        return 0.0;
    }
    (-1.0 / (time_secs * sample_rate as f64)).exp()
}

/// 振幅转 dB，避免 log(0)
fn amplitude_to_db(amplitude: f64) -> f64 {
    if amplitude <= 0.0 {
        -120.0
    } else {
        20.0 * amplitude.log10()
    }
}

/// dB 转振幅
fn db_to_amplitude(db: f64) -> f64 {
    10.0_f64.powf(db / 20.0)
}
