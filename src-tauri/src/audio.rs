/// 音频文件处理模块
/// 提供音频文件的解码、编码和格式验证功能

use std::fs::File;
use std::path::Path;
use symphonia::core::audio::Signal;

/// 检查文件扩展名是否为支持的音频格式
/// 支持 WAV、MP3、FLAC 三种格式
pub fn is_supported_format(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    matches!(ext.as_deref(), Some("wav") | Some("mp3") | Some("flac"))
}

/// 解码音频文件为单声道 f64 采样数据
/// 支持 WAV、MP3、FLAC 格式，立体声自动混合为单声道
/// 返回 (采样数据, 采样率) 或中文错误信息
pub fn decode_audio_file(path: &str) -> Result<(Vec<f64>, u32), String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    let probed = symphonia::default::get_probe()
        .format(
            &Default::default(),
            mss,
            &Default::default(),
            &Default::default(),
        )
        .map_err(|e| format!("无法识别音频格式: {}", e))?;

    let mut format_reader = probed.format;
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("音频文件中未找到有效音轨")?;

    let codec_params = &track.codec_params;
    let sample_rate = codec_params.sample_rate.ok_or("无法获取采样率")?;

    let mut decoder = symphonia::default::get_codecs()
        .make(codec_params, &Default::default())
        .map_err(|e| format!("无法创建解码器: {}", e))?;

    let track_id = track.id;
    let mut all_samples: Vec<f64> = Vec::new();

    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) if p.track_id() != track_id => continue,
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(format!("解码出错: {}", e)),
        };

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("解码帧失败: {}", e))?;

        let channels = decoded.spec().channels.count();
        let frames = decoded.frames();

        match decoded {
            symphonia::core::audio::AudioBufferRef::F32(ref buf) => {
                decode_frames(&mut all_samples, channels, frames, buf);
            }
            symphonia::core::audio::AudioBufferRef::S16(ref buf) => {
                decode_frames(&mut all_samples, channels, frames, buf);
            }
            symphonia::core::audio::AudioBufferRef::U16(ref buf) => {
                decode_frames(&mut all_samples, channels, frames, buf);
            }
            symphonia::core::audio::AudioBufferRef::S24(ref buf) => {
                for i in 0..frames {
                    if channels > 1 {
                        let sum: f64 = (0..channels)
                            .map(|ch| buf.chan(ch)[i].0 as f64 / 8388608.0)
                            .sum();
                        all_samples.push(sum / channels as f64);
                    } else {
                        all_samples.push(buf.chan(0)[i].0 as f64 / 8388608.0);
                    }
                }
            }
            symphonia::core::audio::AudioBufferRef::S32(ref buf) => {
                decode_frames(&mut all_samples, channels, frames, buf);
            }
            symphonia::core::audio::AudioBufferRef::F64(ref buf) => {
                decode_frames(&mut all_samples, channels, frames, buf);
            }
            _ => return Err("不支持的音频采样格式".to_string()),
        }
    }

    if all_samples.is_empty() {
        return Err("音频文件为空或解码失败".to_string());
    }

    Ok((all_samples, sample_rate))
}

/// 从 AudioBuffer 中解码帧数据并混合为单声道
/// 将多声道采样取均值后追加到输出缓冲区
fn decode_frames<T: symphonia::core::sample::Sample + Into<f64>>(
    out: &mut Vec<f64>,
    channels: usize,
    frames: usize,
    buf: &symphonia::core::audio::AudioBuffer<T>,
) {
    for i in 0..frames {
        if channels > 1 {
            let sum: f64 = (0..channels).map(|ch| buf.chan(ch)[i].into()).sum();
            out.push(sum / channels as f64);
        } else {
            out.push(buf.chan(0)[i].into());
        }
    }
}

/// 编码 f64 采样数据为 16-bit PCM WAV 文件
/// 采样值会被 clamp 到 [-1.0, 1.0] 范围后转换为 i16
pub fn encode_wav_file(
    path: &str,
    samples: &[f64],
    sample_rate: u32,
) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("无法创建输出文件: {}", e))?;

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_sample = (clamped * i16::MAX as f64) as i16;
        writer
            .write_sample(i16_sample)
            .map_err(|e| format!("写入采样失败: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(())
}
