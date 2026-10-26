//! PCM decoding and format normalization.
//!
//! Production packs use Ogg/Opus; WAV is kept as a lossless test and fallback
//! format. Both paths normalize to mono/stereo at the engine rate (or the
//! requested target rate) before the mixer sees them.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use hound::{SampleFormat, WavReader};
use opus_pure::{OggOpusReader, Trim, MAX_PACKET_SAMPLES};

use crate::error::EngineError;

/// Opus decodes natively at 48 kHz. Higher or lower target rates are obtained
/// by decoding at 48 kHz and resampling once at load time.
const OPUS_RATE: u32 = 48_000;

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    /// Interleaved samples at `sample_rate`.
    pub samples: Vec<i16>,
    /// 1 (mono) or 2 (stereo).
    pub channels: u16,
    /// Output sample rate after optional resampling.
    pub sample_rate: u32,
    pub frames: usize,
    /// Original file sample rate, used to scale manifest loop points.
    pub source_sample_rate: u32,
    /// Original file channel count.
    pub source_channels: u16,
    /// Original file frame count.
    pub source_frames: usize,
}

impl DecodedAudio {
    pub fn duration_secs(&self) -> f32 {
        self.frames as f32 / self.sample_rate as f32
    }

    pub fn bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<i16>()
    }

    /// Convert mono to duplicated stereo. Stereo is returned unchanged.
    pub fn into_stereo(mut self) -> Self {
        if self.channels == 1 {
            let mut stereo = Vec::with_capacity(self.samples.len() * 2);
            for sample in &self.samples {
                stereo.push(*sample);
                stereo.push(*sample);
            }
            self.samples = stereo;
            self.channels = 2;
            self.frames = self.samples.len() / 2;
        }
        self
    }
}

/// Decode an audio file and normalize it to mono/stereo at `target_rate`.
pub fn decode_audio(path: &Path, target_rate: u32) -> Result<DecodedAudio, EngineError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "wav" => decode_wav(path, target_rate),
        "opus" | "ogg" => decode_opus(path, target_rate),
        other => Err(EngineError::UnsupportedFormat(format!(".{other}"))),
    }
}

/// Decode an Ogg/Opus file, applying the RFC 7845 pre-skip and end-trim so the
/// returned frames match the audio that was encoded.
fn decode_opus(path: &Path, target_rate: u32) -> Result<DecodedAudio, EngineError> {
    let file = File::open(path).map_err(|error| EngineError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut reader =
        OggOpusReader::new(BufReader::new(file)).map_err(|error| opus_error(path, error))?;

    let head = reader.head().clone();
    let channels = head.channel_count as usize;
    if !(1..=2).contains(&channels) {
        return Err(EngineError::InvalidAudio(format!(
            "Opus file {} has {channels} channels; only mono and stereo are supported",
            path.display()
        )));
    }

    let mut decoder = head
        .decoder(OPUS_RATE as i32)
        .map_err(|error| opus_error(path, error))?;
    let mut trim =
        Trim::new(&head, OPUS_RATE as i32, channels).map_err(|error| opus_error(path, error))?;

    let mut block = vec![0i16; MAX_PACKET_SAMPLES * channels];
    let mut samples = Vec::new();
    while let Some(packet) = reader
        .read_packet()
        .map_err(|error| opus_error(path, error))?
    {
        let frames = decoder
            .decode_s16(&packet.data, MAX_PACKET_SAMPLES, &mut block)
            .map_err(|error| opus_error(path, error))?;
        let audio = trim.keep(&packet, &block[..frames * channels]);
        samples.extend_from_slice(audio);
    }

    if samples.is_empty() {
        return Err(EngineError::InvalidAudio(format!(
            "Opus file {} contains no audio",
            path.display()
        )));
    }

    let source_frames = samples.len() / channels;
    let samples = if target_rate == OPUS_RATE {
        samples
    } else {
        resample(&samples, channels as u16, OPUS_RATE, target_rate)
    };
    let frames = samples.len() / channels;

    Ok(DecodedAudio {
        samples,
        channels: channels as u16,
        sample_rate: target_rate,
        frames,
        // Ogg/Opus loop points are authored at 48 kHz. The header's
        // input_sample_rate is informational and does not change decoding.
        source_sample_rate: OPUS_RATE,
        source_channels: channels as u16,
        source_frames,
    })
}

fn opus_error(path: &Path, error: opus_pure::Error) -> EngineError {
    EngineError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

fn decode_wav(path: &Path, target_rate: u32) -> Result<DecodedAudio, EngineError> {
    let mut reader = WavReader::open(path).map_err(|error| EngineError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let spec = reader.spec();

    if spec.channels == 0 {
        return Err(EngineError::InvalidAudio("WAV has zero channels".into()));
    }

    let mut interleaved: Vec<i16> = Vec::new();
    match spec.sample_format {
        SampleFormat::Int => match spec.bits_per_sample {
            8 => {
                for sample in reader.samples::<i8>() {
                    let sample = sample.map_err(|error| decode_error(path, error))?;
                    interleaved.push((sample as i16) << 8);
                }
            }
            16 => {
                for sample in reader.samples::<i16>() {
                    interleaved.push(sample.map_err(|error| decode_error(path, error))?);
                }
            }
            24 => {
                for sample in reader.samples::<i32>() {
                    let sample = sample.map_err(|error| decode_error(path, error))?;
                    interleaved.push((sample >> 8) as i16);
                }
            }
            32 => {
                for sample in reader.samples::<i32>() {
                    let sample = sample.map_err(|error| decode_error(path, error))?;
                    interleaved.push((sample >> 16) as i16);
                }
            }
            bits => {
                return Err(EngineError::InvalidAudio(format!(
                    "unsupported WAV bit depth: {bits}"
                )))
            }
        },
        SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let sample = sample.map_err(|error| decode_error(path, error))?;
                interleaved.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
            }
        }
    }

    let source_frames = interleaved.len() / spec.channels as usize;
    let source_channels = spec.channels;
    let source_sample_rate = spec.sample_rate;

    // Keep mono as mono, take the first two channels of anything wider.
    let (samples, channels) = if source_channels == 1 {
        (interleaved, 1u16)
    } else if source_channels == 2 {
        (interleaved, 2u16)
    } else {
        let mut stereo = Vec::with_capacity(source_frames * 2);
        for frame in 0..source_frames {
            let base = frame * source_channels as usize;
            stereo.push(interleaved[base]);
            stereo.push(interleaved[base + 1]);
        }
        (stereo, 2u16)
    };

    let samples = if source_sample_rate == target_rate {
        samples
    } else {
        resample(&samples, channels, source_sample_rate, target_rate)
    };
    let frames = samples.len() / channels as usize;

    Ok(DecodedAudio {
        samples,
        channels,
        sample_rate: target_rate,
        frames,
        source_sample_rate,
        source_channels,
        source_frames,
    })
}

fn decode_error(path: &Path, error: hound::Error) -> EngineError {
    EngineError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

/// Linear resampling, good enough for load-time normalization. Replace with a
/// windowed-sinc resampler if measurements show it is audible.
fn resample(input: &[i16], channels: u16, input_rate: u32, output_rate: u32) -> Vec<i16> {
    if input_rate == output_rate || input.is_empty() {
        return input.to_vec();
    }

    let channels = channels as usize;
    let input_frames = input.len() / channels;
    if input_frames == 0 {
        return Vec::new();
    }
    let output_frames =
        ((input_frames as u64 * output_rate as u64) / input_rate as u64).max(1) as usize;
    let ratio = input_rate as f64 / output_rate as f64;
    let mut output = Vec::with_capacity(output_frames * channels);

    for frame in 0..output_frames {
        let position = frame as f64 * ratio;
        let index = (position.floor() as usize).min(input_frames - 1);
        let next = (index + 1).min(input_frames - 1);
        let fraction = position - index as f64;

        for channel in 0..channels {
            let a = input[index * channels + channel] as f64;
            let b = input[next * channels + channel] as f64;
            let value = a + (b - a) * fraction;
            output.push(value.round().clamp(i16::MIN as f64, i16::MAX as f64) as i16);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use opus_pure::{Application, OggOpusWriter, OpusEncoder, OpusHead, MAX_PACKET_BYTES};

    /// Encode interleaved PCM as a gapless Ogg/Opus stream. This mirrors the
    /// arithmetic in opus-pure's encoder example: feed enough silence to flush
    /// the encoder delay, then declare the true end through the final granule.
    fn encode_test_opus(samples: &[i16], channels: usize, rate: u32) -> Vec<u8> {
        assert!(matches!(rate, 8_000 | 12_000 | 16_000 | 24_000 | 48_000));
        assert!(matches!(channels, 1 | 2));
        assert_eq!(samples.len() % channels, 0);

        let frame = (rate / 50) as usize;
        let mut encoder = OpusEncoder::new(rate as i32, channels, Application::Audio).unwrap();
        encoder.bitrate_bps = 96_000;

        let head = OpusHead::for_encoder(&encoder, rate);
        let ticks = 48_000 / rate as usize;
        let total = samples.len() / channels;
        let lookahead = (head.pre_skip as usize).div_ceil(ticks);
        let packet_count = (total + lookahead).div_ceil(frame);
        let final_granule = head.pre_skip as u64 + (total * ticks) as u64;

        let mut writer = OggOpusWriter::new(Vec::new(), head).unwrap();
        let mut packet = vec![0u8; MAX_PACKET_BYTES];
        let per_frame = frame * channels;
        let mut block = vec![0i16; per_frame];

        for index in 0..packet_count {
            let start = (index * per_frame).min(samples.len());
            let end = (start + per_frame).min(samples.len());
            block[..end - start].copy_from_slice(&samples[start..end]);
            block[end - start..].fill(0);

            let written = encoder.encode_s16(&block, frame, &mut packet).unwrap();
            if index + 1 == packet_count {
                let duration = final_granule - writer.granule() as u64;
                writer
                    .write_packet_with_duration(&packet[..written], duration as u32)
                    .unwrap();
            } else {
                writer.write_packet(&packet[..written]).unwrap();
            }
        }

        writer.finish().unwrap()
    }

    fn sine(frames: usize, channels: usize, rate: u32, amplitude: f32) -> Vec<i16> {
        (0..frames)
            .flat_map(|frame| {
                let value = (frame as f32 * std::f32::consts::TAU * 440.0 / rate as f32).sin()
                    * amplitude
                    * i16::MAX as f32;
                let value = value.round() as i16;
                std::iter::repeat_n(value, channels)
            })
            .collect()
    }

    fn rms(samples: &[i16]) -> f32 {
        let sum: f64 = samples
            .iter()
            .map(|sample| (*sample as f64) * (*sample as f64))
            .sum();
        (sum / samples.len() as f64).sqrt() as f32 / i16::MAX as f32
    }

    fn write_opus(
        samples: &[i16],
        channels: usize,
        rate: u32,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.opus");
        std::fs::write(&path, encode_test_opus(samples, channels, rate)).unwrap();
        (dir, path)
    }

    #[test]
    fn resample_preserves_duration() {
        let input = vec![0i16; 48_000 * 2];
        let output = resample(&input, 2, 48_000, 44_100);
        assert_eq!(output.len(), 44_100 * 2);
    }

    #[test]
    fn decodes_stereo_opus_without_delay_or_padding() {
        let samples = sine(48_000, 2, 48_000, 0.25);
        let (_dir, path) = write_opus(&samples, 2, 48_000);

        let audio = decode_audio(&path, 48_000).unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.source_sample_rate, 48_000);
        assert_eq!(audio.frames, 48_000);
        assert_eq!(audio.source_frames, 48_000);
        assert!((rms(&audio.samples) - 0.25 / 2f32.sqrt()).abs() < 0.02);
    }

    #[test]
    fn decodes_mono_opus() {
        let samples = sine(24_000, 1, 48_000, 0.2);
        let (_dir, path) = write_opus(&samples, 1, 48_000);

        let audio = decode_audio(&path, 48_000).unwrap();
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.frames, 24_000);
        assert_eq!(audio.source_frames, 24_000);
        assert!((rms(&audio.samples) - 0.2 / 2f32.sqrt()).abs() < 0.02);
    }

    #[test]
    fn decodes_opus_and_resamples_to_engine_rate() {
        let samples = sine(48_000, 2, 48_000, 0.2);
        let (_dir, path) = write_opus(&samples, 2, 48_000);

        let audio = decode_audio(&path, 44_100).unwrap();
        assert_eq!(audio.sample_rate, 44_100);
        assert_eq!(audio.source_sample_rate, 48_000);
        assert_eq!(audio.source_frames, 48_000);
        assert_eq!(audio.frames, 44_100);
    }

    #[test]
    fn rejects_malformed_opus() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.opus");
        std::fs::write(&path, b"not an ogg opus stream").unwrap();
        assert!(matches!(
            decode_audio(&path, 48_000),
            Err(EngineError::Decode { .. })
        ));
    }

    #[test]
    fn mono_to_stereo_duplicates() {
        let audio = DecodedAudio {
            samples: vec![1, 2, 3],
            channels: 1,
            sample_rate: 48_000,
            frames: 3,
            source_sample_rate: 48_000,
            source_channels: 1,
            source_frames: 3,
        };
        let stereo = audio.into_stereo();
        assert_eq!(stereo.channels, 2);
        assert_eq!(stereo.samples, vec![1, 1, 2, 2, 3, 3]);
        assert_eq!(stereo.frames, 3);
    }
}
