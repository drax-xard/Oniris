//! Ogg/Opus encoding for test packs and the asset pipeline.
//!
//! `opus-pure` writes a gapless stream when the final packet's duration is set
//! explicitly. The arithmetic below follows RFC 7845 §4.2/§4.4: feed enough
//! silence to flush the encoder delay, then end the granule at pre-skip +
//! the real audio length.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{bail, Context, Result};
use opus_pure::{Application, OggOpusWriter, OpusEncoder, OpusHead, OpusTags, MAX_PACKET_BYTES};

#[derive(Debug, Clone, Copy)]
pub struct EncodeStats {
    pub frames: usize,
    pub packet_count: usize,
}

pub fn encode_opus(
    output: &Path,
    samples: &[i16],
    channels: u16,
    sample_rate: u32,
    bitrate_bps: i32,
) -> Result<EncodeStats> {
    let channels = channels as usize;
    if !matches!(channels, 1 | 2) {
        bail!("Opus supports mono or stereo, not {channels} channels");
    }
    if !matches!(sample_rate, 8_000 | 12_000 | 16_000 | 24_000 | 48_000) {
        bail!("Opus does not support a {sample_rate} Hz sample rate");
    }
    if samples.is_empty() {
        bail!("cannot encode an empty signal");
    }
    if !samples.len().is_multiple_of(channels) {
        bail!("sample buffer is not a whole number of frames");
    }

    // Opus codes 20 ms frames. At 48 kHz this is 960 samples per channel.
    let frame = (sample_rate / 50) as usize;
    let total = samples.len() / channels;

    let mut encoder = OpusEncoder::new(sample_rate as i32, channels, Application::Audio)
        .context("failed to create the Opus encoder")?;
    encoder.bitrate_bps = bitrate_bps;

    let mut tags = OpusTags::new();
    tags.push("ENCODER", concat!("oniris-cli ", env!("CARGO_PKG_VERSION")))
        .context("failed to add the Opus encoder tag")?;

    let head = OpusHead::for_encoder(&encoder, sample_rate);
    let ticks = 48_000 / sample_rate as usize;
    let lookahead = (head.pre_skip as usize).div_ceil(ticks);
    let packet_count = (total + lookahead).div_ceil(frame);
    let final_granule = u64::from(head.pre_skip) + (total * ticks) as u64;

    let file =
        File::create(output).with_context(|| format!("failed to create {}", output.display()))?;
    let mut writer = OggOpusWriter::with_tags(BufWriter::new(file), head, tags)
        .context("failed to create the Ogg Opus writer")?;

    let mut packet = vec![0u8; MAX_PACKET_BYTES];
    let per_frame = frame * channels;
    let mut block = vec![0i16; per_frame];

    for index in 0..packet_count {
        let start = (index * per_frame).min(samples.len());
        let end = (start + per_frame).min(samples.len());
        block[..end - start].copy_from_slice(&samples[start..end]);
        block[end - start..].fill(0);

        let written = encoder
            .encode_s16(&block, frame, &mut packet)
            .with_context(|| format!("failed to encode packet {index}"))?;

        if index + 1 == packet_count {
            let duration = final_granule - writer.granule() as u64;
            writer
                .write_packet_with_duration(&packet[..written], duration as u32)
                .context("failed to write the final Opus packet")?;
        } else {
            writer
                .write_packet(&packet[..written])
                .with_context(|| format!("failed to write packet {index}"))?;
        }
    }

    writer
        .finish()
        .with_context(|| format!("failed to finalize {}", output.display()))?;

    Ok(EncodeStats {
        frames: total,
        packet_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oniris_core::decode_audio;

    fn sine(frames: usize, channels: usize, amplitude: f32) -> Vec<i16> {
        (0..frames)
            .flat_map(|frame| {
                let value = (frame as f32 * std::f32::consts::TAU * 440.0 / 48_000.0).sin()
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

    #[test]
    fn writes_gapless_mono_and_stereo_streams() {
        for channels in [1usize, 2] {
            let frames = 24_000;
            let samples = sine(frames, channels, 0.25);
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("tone.opus");

            let stats = encode_opus(&path, &samples, channels as u16, 48_000, 96_000).unwrap();
            assert_eq!(stats.frames, frames);

            let decoded = decode_audio(&path, 48_000).unwrap();
            assert_eq!(decoded.channels as usize, channels);
            assert_eq!(decoded.frames, frames);
            assert_eq!(decoded.source_frames, frames);
            assert!((rms(&decoded.samples) - 0.25 / 2f32.sqrt()).abs() < 0.03);
        }
    }

    #[test]
    fn rejects_unsupported_input() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.opus");

        assert!(encode_opus(&path, &[], 2, 48_000, 96_000).is_err());
        assert!(encode_opus(&path, &[0; 10], 3, 48_000, 96_000).is_err());
        assert!(encode_opus(&path, &[0; 10], 2, 44_100, 96_000).is_err());
    }
}
