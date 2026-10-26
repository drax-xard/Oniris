//! PCM decoding and format normalization.
//!
//! The production format is Opus. The scaffold decodes WAV so the mixer can be
//! developed and tested without a native codec dependency. `decode_audio` is the
//! single extension point for adding Opus/Vorbis later.

use std::path::Path;

use hound::{SampleFormat, WavReader};

use crate::error::EngineError;

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
        other => Err(EngineError::UnsupportedFormat(format!(".{other}"))),
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

    #[test]
    fn resample_preserves_duration() {
        let input = vec![0i16; 48_000 * 2];
        let output = resample(&input, 2, 48_000, 44_100);
        assert_eq!(output.len(), 44_100 * 2);
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
