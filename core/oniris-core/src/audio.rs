//! PCM decoding and format normalization.
//!
//! Production packs use Ogg Opus; WAV remains supported for development packs
//! and tests. `decode_audio` is the single extension point for adding other
//! formats.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use hound::{SampleFormat, WavReader};
use ogg::PacketReader;
use opus_decoder::OpusDecoder;

use crate::error::EngineError;

/// Opus always decodes at 48 kHz. Manifest loop points for Opus files are
/// therefore expressed at 48 kHz.
pub const OPUS_SAMPLE_RATE: u32 = 48_000;

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
        "opus" | "ogg" | "oga" => decode_opus(path, target_rate),
        other => Err(EngineError::UnsupportedFormat(format!(".{other}"))),
    }
}

#[derive(Debug, Clone, Copy)]
struct OpusHead {
    channels: u16,
    pre_skip: u16,
    gain_db: f32,
}

/// Decode an Ogg Opus file to interleaved PCM.
///
/// The Ogg Opus spec (RFC 7845) requires decoders to discard `pre-skip` samples
/// from the start and to stop at the final granule position so encoder padding
/// at the end never becomes audible. Both are applied here; loop points in the
/// manifest are relative to this trimmed stream.
fn decode_opus(path: &Path, target_rate: u32) -> Result<DecodedAudio, EngineError> {
    let file = File::open(path).map_err(|error| EngineError::Decode {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut reader = PacketReader::new(BufReader::new(file));

    let head_packet = read_ogg_packet(path, &mut reader, "OpusHead")?;
    let head = parse_opus_head(path, &head_packet)?;

    let tags = read_ogg_packet(path, &mut reader, "OpusTags")?;
    if !tags.starts_with(b"OpusTags") {
        return Err(decode_failure(
            path,
            "expected an OpusTags packet after OpusHead",
        ));
    }

    let channels = head.channels as usize;
    let mut decoder = OpusDecoder::new(OPUS_SAMPLE_RATE, channels)
        .map_err(|error| decode_failure(path, format!("Opus decoder: {error}")))?;
    let mut frame = vec![0i16; OpusDecoder::MAX_FRAME_SIZE_48K * channels];
    let mut pcm: Vec<i16> = Vec::new();
    let mut last_granule = 0u64;

    while let Some(packet) = reader
        .read_packet()
        .map_err(|error| decode_failure(path, format!("Ogg stream: {error}")))?
    {
        let granule = packet.absgp_page();
        if granule != u64::MAX {
            last_granule = granule;
        }

        // Empty packets are packet-loss markers. Decoding them runs Opus PLC
        // using the previous packet's state instead of injecting silence.
        let decoded = decoder
            .decode(&packet.data, &mut frame, false)
            .map_err(|error| decode_failure(path, format!("Opus packet: {error}")))?;
        pcm.extend_from_slice(&frame[..decoded * channels]);
    }

    let decoded_frames = pcm.len() / channels;
    let end = if last_granule > 0 && last_granule != u64::MAX {
        decoded_frames.min(last_granule as usize)
    } else {
        decoded_frames
    };
    let start = (head.pre_skip as usize).min(end);
    pcm.drain(..start * channels);
    pcm.truncate((end - start) * channels);

    if head.gain_db.abs() > 1e-4 {
        let gain = 10f32.powf(head.gain_db / 20.0);
        for sample in &mut pcm {
            *sample = (*sample as f32 * gain)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }

    let source_frames = pcm.len() / channels;
    let samples = if target_rate == OPUS_SAMPLE_RATE {
        pcm
    } else {
        resample(&pcm, head.channels, OPUS_SAMPLE_RATE, target_rate)
    };
    let frames = samples.len() / channels;

    Ok(DecodedAudio {
        samples,
        channels: head.channels,
        sample_rate: target_rate,
        frames,
        source_sample_rate: OPUS_SAMPLE_RATE,
        source_channels: head.channels,
        source_frames,
    })
}

fn read_ogg_packet(
    path: &Path,
    reader: &mut PacketReader<BufReader<File>>,
    what: &str,
) -> Result<Vec<u8>, EngineError> {
    match reader
        .read_packet()
        .map_err(|error| decode_failure(path, format!("Ogg stream: {error}")))?
    {
        Some(packet) if !packet.data.is_empty() => Ok(packet.data),
        Some(_) => Err(decode_failure(path, format!("missing {what} packet"))),
        None => Err(decode_failure(path, format!("missing {what} packet"))),
    }
}

fn parse_opus_head(path: &Path, data: &[u8]) -> Result<OpusHead, EngineError> {
    if data.len() < 19 || &data[..8] != b"OpusHead" {
        return Err(decode_failure(path, "missing OpusHead packet"));
    }
    if data[8] >> 4 != 0 {
        return Err(decode_failure(
            path,
            format!("unsupported OpusHead version {}", data[8]),
        ));
    }
    let channels = data[9] as u16;
    if !(1..=2).contains(&channels) {
        return Err(decode_failure(
            path,
            format!("unsupported Opus channel count {channels}"),
        ));
    }
    let pre_skip = u16::from_le_bytes([data[10], data[11]]);
    let gain_q8 = i16::from_le_bytes([data[16], data[17]]);
    let mapping_family = data[18];

    if mapping_family != 0 {
        // Mapping family 1 is the standard stereo layout: one stream, one
        // coupled stream, channels mapped to [0, 1].
        if mapping_family != 1 || channels != 2 {
            return Err(decode_failure(
                path,
                format!("unsupported Opus channel mapping family {mapping_family}"),
            ));
        }
        if data.len() < 21 + channels as usize {
            return Err(decode_failure(path, "truncated OpusHead channel mapping"));
        }
        let stream_count = data[19];
        let coupled_count = data[20];
        let mapping = &data[21..21 + channels as usize];
        if stream_count != 1 || coupled_count != 1 || mapping != [0, 1] {
            return Err(decode_failure(
                path,
                "unsupported Opus stereo channel mapping",
            ));
        }
    }

    Ok(OpusHead {
        channels,
        pre_skip,
        gain_db: gain_q8 as f32 / 256.0,
    })
}

fn decode_failure(path: &Path, message: impl std::fmt::Display) -> EngineError {
    EngineError::Decode {
        path: path.to_path_buf(),
        message: message.to_string(),
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

/// Band-limited resampling using a windowed-sinc kernel.
///
/// A 32-tap Blackman-Harris window is used at load time. The cutoff follows the
/// lower of the two rates, so downsampling is anti-aliased; upsampling rolls off
/// just below the input Nyquist. This replaces the scaffold's linear
/// interpolation, which mirrored audible high-frequency content on rate
/// conversion. The tap count is fixed rather than scaled by the conversion
/// ratio, which widens the transition band slightly on downsampling; at the
/// 44.1/48 kHz ratios the engine uses, the difference is far below the audible
/// band.
fn resample(input: &[i16], channels: u16, input_rate: u32, output_rate: u32) -> Vec<i16> {
    if input_rate == output_rate || input.is_empty() {
        return input.to_vec();
    }

    let channels = channels as usize;
    let input_frames = input.len() / channels;
    if input_frames == 0 {
        return Vec::new();
    }

    let output_frames = ((input_frames as u64 * output_rate as u64 + input_rate as u64 / 2)
        / input_rate as u64)
        .max(1) as usize;
    let ratio = input_rate as f64 / output_rate as f64;
    let cutoff = (output_rate as f64 / input_rate as f64).min(1.0) * 0.95;
    let half_width = RESAMPLE_HALF_TAPS;
    let mut output = Vec::with_capacity(output_frames * channels);

    for frame in 0..output_frames {
        let center = frame as f64 * ratio;
        let first = center.floor() as i64 - half_width as i64 + 1;
        let mut sums = [0f64; 2];
        let mut weight_sum = 0f64;

        for tap in 0..half_width * 2 {
            let index = first + tap as i64;
            if index < 0 || index >= input_frames as i64 {
                continue;
            }
            let distance = center - index as f64;
            let weight = sinc_kernel(distance, cutoff) * blackman_harris(distance, half_width);
            if weight == 0.0 {
                continue;
            }
            weight_sum += weight;
            let base = index as usize * channels;
            for (channel, sum) in sums.iter_mut().enumerate().take(channels) {
                *sum += input[base + channel] as f64 * weight;
            }
        }

        if weight_sum.abs() < 1e-12 {
            output.resize(output.len() + channels, 0);
            continue;
        }

        for sum in sums.iter().take(channels) {
            output.push(
                (sum / weight_sum)
                    .round()
                    .clamp(i16::MIN as f64, i16::MAX as f64) as i16,
            );
        }
    }

    output
}

const RESAMPLE_HALF_TAPS: usize = 16;

fn sinc_kernel(distance: f64, cutoff: f64) -> f64 {
    if distance.abs() < 1e-9 {
        cutoff
    } else {
        let x = std::f64::consts::PI * cutoff * distance;
        cutoff * x.sin() / x
    }
}

fn blackman_harris(distance: f64, half_width: usize) -> f64 {
    let t = (distance / half_width as f64 + 1.0) * 0.5;
    if !(0.0..=1.0).contains(&t) {
        return 0.0;
    }
    let angle = std::f64::consts::TAU * t;
    0.35875 - 0.48829 * angle.cos() + 0.14128 * (2.0 * angle).cos() - 0.01168 * (3.0 * angle).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn sine(frequency: f64, sample_rate: u32, frames: usize) -> Vec<i16> {
        (0..frames)
            .map(|frame| {
                let phase = std::f64::consts::TAU * frequency * frame as f64 / sample_rate as f64;
                (phase.sin() * 10_000.0).round() as i16
            })
            .collect()
    }

    fn rms(samples: &[i16]) -> f64 {
        let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        (sum / samples.len() as f64).sqrt()
    }

    #[test]
    fn resample_preserves_duration() {
        let input = vec![0i16; 48_000 * 2];
        let output = resample(&input, 2, 48_000, 44_100);
        assert_eq!(output.len(), 44_100 * 2);
    }

    #[test]
    fn resampler_preserves_sine_level() {
        let input = sine(1_000.0, 48_000, 48_000);
        let output = resample(&input, 1, 48_000, 44_100);
        let middle = &output[5_000..output.len() - 5_000];
        let expected = 10_000.0 / std::f64::consts::SQRT_2;
        assert!(
            (rms(middle) - expected).abs() / expected < 0.02,
            "rms={}",
            rms(middle)
        );
    }

    #[test]
    fn resampler_anti_aliases_downsampling() {
        let input = sine(20_000.0, 48_000, 48_000);
        let output = resample(&input, 1, 48_000, 24_000);
        let middle = &output[2_000..output.len() - 2_000];
        assert!(rms(middle) < 200.0, "alias rms={}", rms(middle));
    }

    #[test]
    fn decodes_ogg_opus_and_trims_padding() {
        let audio = decode_audio(&fixture("tone-stereo.opus"), 48_000).unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.source_sample_rate, 48_000);
        assert_eq!(audio.source_channels, 2);
        // 1 s after pre-skip and end-trimming, not the 1.0065 s container length.
        assert_eq!(audio.frames, 48_000);
        let left: Vec<i16> = audio
            .samples
            .chunks_exact(2)
            .map(|frame| frame[0])
            .collect();
        assert!(rms(&left) > 1_000.0, "rms={}", rms(&left));
        // The fixture's right channel is half-scale.
        let right: Vec<i16> = audio
            .samples
            .chunks_exact(2)
            .map(|frame| frame[1])
            .collect();
        let ratio = rms(&right) / rms(&left);
        assert!((ratio - 0.5).abs() < 0.05, "ratio={ratio}");
    }

    #[test]
    fn decodes_mono_opus() {
        let audio = decode_audio(&fixture("tone-mono.opus"), 48_000).unwrap();
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.frames, 14_400);
        assert!(audio.samples.iter().any(|sample| sample.abs() > 1_000));
    }

    #[test]
    fn resamples_opus_to_engine_rate() {
        let audio = decode_audio(&fixture("tone-stereo.opus"), 44_100).unwrap();
        assert_eq!(audio.sample_rate, 44_100);
        assert_eq!(audio.frames, 44_100);
        assert_eq!(audio.source_frames, 48_000);
    }

    #[test]
    fn rejects_non_opus_ogg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.opus");
        std::fs::write(&path, b"OggSnot really an opus file").unwrap();
        assert!(decode_audio(&path, 48_000).is_err());
    }

    fn opus_head(channels: u8, mapping_family: u8, extra: &[u8]) -> Vec<u8> {
        let mut head = Vec::new();
        head.extend_from_slice(b"OpusHead");
        head.push(1);
        head.push(channels);
        head.extend_from_slice(&312u16.to_le_bytes());
        head.extend_from_slice(&48_000u32.to_le_bytes());
        head.extend_from_slice(&0i16.to_le_bytes());
        head.push(mapping_family);
        head.extend_from_slice(extra);
        head
    }

    #[test]
    fn parses_opus_head() {
        let path = Path::new("test.opus");
        let head = parse_opus_head(path, &opus_head(2, 0, &[])).unwrap();
        assert_eq!(head.channels, 2);
        assert_eq!(head.pre_skip, 312);
        assert_eq!(head.gain_db, 0.0);

        let stereo = opus_head(2, 1, &[1, 1, 0, 1]);
        assert!(parse_opus_head(path, &stereo).is_ok());
    }

    #[test]
    fn rejects_invalid_opus_heads() {
        let path = Path::new("test.opus");
        assert!(parse_opus_head(path, b"not opus").is_err());
        assert!(parse_opus_head(path, &opus_head(0, 0, &[])).is_err());
        assert!(parse_opus_head(path, &opus_head(3, 0, &[])).is_err());
        assert!(parse_opus_head(path, &opus_head(2, 2, &[])).is_err());
        // Family 1 must declare one stream and one coupled stream.
        assert!(parse_opus_head(path, &opus_head(2, 1, &[2, 0, 0, 1])).is_err());
    }

    #[test]
    fn recovers_a_missing_opus_packet_with_plc() {
        use ogg::{PacketWriteEndInfo, PacketWriter};

        // Re-mux the fixture with a zero-length packet in the middle of the
        // stream to exercise Opus packet-loss concealment.
        let source = File::open(fixture("tone-stereo.opus")).unwrap();
        let mut reader = PacketReader::new(BufReader::new(source));
        let mut packets = Vec::new();
        while let Some(packet) = reader.read_packet().unwrap() {
            packets.push(packet.data);
        }
        assert!(packets.len() > 8);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gap.opus");
        let file = File::create(&path).unwrap();
        let mut writer = PacketWriter::new(file);
        let serial = 0x0D1_215;
        let mut granule = 0u64;
        let last = packets.len() - 1;
        for (index, data) in packets.iter().enumerate() {
            if index == 6 {
                writer
                    .write_packet(
                        Vec::new(),
                        serial,
                        PacketWriteEndInfo::NormalPacket,
                        granule,
                    )
                    .unwrap();
                granule += 960;
            }
            let end = if index == last {
                PacketWriteEndInfo::EndStream
            } else {
                PacketWriteEndInfo::NormalPacket
            };
            writer
                .write_packet(data.clone(), serial, end, granule)
                .unwrap();
            if index >= 2 {
                granule += 960;
            }
        }
        drop(writer);

        let audio = decode_audio(&path, 48_000).unwrap();
        assert_eq!(audio.channels, 2);
        assert!(audio.frames >= 48_000, "frames={}", audio.frames);
        assert!(rms(&audio.samples) > 1_000.0);
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
