//! Synthetic test-pack generation.
//!
//! These are deliberately simple and deterministic. They are not launch
//! content; they exist so the mixer, loop points, and event scheduler can be
//! exercised without shipping binary audio in the repository.

use std::f32::consts::{PI, TAU};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::ValueEnum;
use hound::{WavSpec, WavWriter};
use oniris_pack::Manifest;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{loudness, opus_encode};

const RATE: u32 = 48_000;
const NOISE_LOOP_FRAMES: usize = 60 * RATE as usize;
const NOISE_FADE_FRAMES: usize = 2 * RATE as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AudioFormat {
    /// Ogg/Opus, the production format.
    Opus,
    /// 16-bit WAV, useful as a lossless fallback and for debugging.
    Wav,
}

impl AudioFormat {
    fn extension(self) -> &'static str {
        match self {
            AudioFormat::Opus => "opus",
            AudioFormat::Wav => "wav",
        }
    }
}

pub fn generate_test_packs(out: &Path, format: AudioFormat) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;

    let rain_dir = out.join("test-rain");
    generate_rain(&rain_dir, format)?;
    println!("generated {} ({format:?})", rain_dir.display());

    let fire_dir = out.join("test-fire");
    generate_fire(&fire_dir, format)?;
    println!("generated {} ({format:?})", fire_dir.display());

    Ok(())
}

pub fn generate_noise_pack(out: &Path) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;
    let dir = out.join("pure-noise");
    generate_pure_noise(&dir, NOISE_LOOP_FRAMES, NOISE_FADE_FRAMES)?;
    println!("generated {}", dir.display());
    Ok(())
}

fn generate_pure_noise(dir: &Path, loop_frames: usize, fade_frames: usize) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;

    let pink_source = pink_noise(0x51C0, loop_frames + fade_frames);
    let brown_source = brown_noise(0xB0B0, loop_frames + fade_frames);
    let pink = crossfade_loop(&pink_source, loop_frames, fade_frames);
    let brown = crossfade_loop(&brown_source, loop_frames, fade_frames);

    // Normalize the two-bed sum, not each bed independently, so the pack as
    // mixed by the engine lands on the -23 LUFS target.
    let combined = mix_i16(&pink, &brown);
    let before = loudness::measure(&combined, 2, RATE);
    if !before.integrated_lufs.is_finite() {
        anyhow::bail!("generated noise measured as silence");
    }
    let gain_db = -23.0 - before.integrated_lufs;
    let pink = loudness::apply_gain(&pink, gain_db);
    let brown = loudness::apply_gain(&brown, gain_db);
    let after = loudness::measure(&mix_i16(&pink, &brown), 2, RATE);

    let pink_file = write_audio(dir, "pink-noise", 2, &pink, AudioFormat::Opus)?;
    let brown_file = write_audio(dir, "brown-noise", 2, &brown, AudioFormat::Opus)?;

    let true_peak = if after.true_peak_dbtp.is_finite() {
        (f64::from(after.true_peak_dbtp) * 10.0).round() / 10.0
    } else {
        -1.5
    };
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "pure-noise",
        "name": { "en": "Pure Noise" },
        "description": {
            "en": "Layered pink and brown noise with a slow, almost imperceptible drift."
        },
        "version": "1.0.0",
        "license": "CC0-1.0",
        "tags": ["noise", "sleep", "focus"],
        "loudnessLufs": -23.0,
        "truePeakDbtp": true_peak,
        "beds": [
            {
                "id": "pink",
                "file": &pink_file,
                "loop": { "startSample": 0, "endSample": loop_frames },
                "gainDb": 0.0,
                "lfo": { "target": "gain", "depth": 0.5, "periodSec": 47.0 }
            },
            {
                "id": "brown",
                "file": &brown_file,
                "loop": { "startSample": 0, "endSample": loop_frames },
                "gainDb": 0.0,
                "lfo": { "target": "gain", "depth": 0.3, "periodSec": 61.0 }
            }
        ],
        "credits": [
            {
                "asset": &pink_file,
                "title": "Pink noise",
                "author": "Oniris",
                "source": "synthetic",
                "license": "CC0-1.0",
                "modified": false
            },
            {
                "asset": &brown_file,
                "title": "Brown noise",
                "author": "Oniris",
                "source": "synthetic",
                "license": "CC0-1.0",
                "modified": false
            }
        ]
    });
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

fn mix_i16(left: &[i16], right: &[i16]) -> Vec<i16> {
    left.iter()
        .zip(right)
        .map(|(a, b)| (i32::from(*a) + i32::from(*b)).clamp(-32_768, 32_767) as i16)
        .collect()
}

fn pink_noise(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut left = [0.0f32; 7];
    let mut right = [0.0f32; 7];
    let mut output = Vec::with_capacity(frames * 2);

    for _ in 0..frames {
        output.push(pink_sample(&mut left, rng.gen_range(-1.0f32..1.0)));
        output.push(pink_sample(&mut right, rng.gen_range(-1.0f32..1.0)));
    }
    output
}

/// Paul Kellet's refined pink-noise filter.
fn pink_sample(state: &mut [f32; 7], white: f32) -> f32 {
    state[0] = 0.99886 * state[0] + white * 0.055_518;
    state[1] = 0.99332 * state[1] + white * 0.075_076;
    state[2] = 0.96900 * state[2] + white * 0.153_852;
    state[3] = 0.86650 * state[3] + white * 0.310_486;
    state[4] = 0.55000 * state[4] + white * 0.532_952;
    state[5] = -0.7616 * state[5] - white * 0.016_898;
    let pink =
        state[0] + state[1] + state[2] + state[3] + state[4] + state[5] + state[6] + white * 0.5362;
    state[6] = white * 0.115_926;
    (pink * 0.11).clamp(-0.9, 0.9)
}

fn brown_noise(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut left = 0.0f32;
    let mut right = 0.0f32;
    let mut output = Vec::with_capacity(frames * 2);

    for _ in 0..frames {
        left = (left + rng.gen_range(-1.0f32..1.0) * 0.02) * 0.999;
        right = (right + rng.gen_range(-1.0f32..1.0) * 0.02) * 0.999;
        output.push(left.clamp(-0.9, 0.9));
        output.push(right.clamp(-0.9, 0.9));
    }
    output
}

fn generate_rain(dir: &Path, format: AudioFormat) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;
    let frames = RATE as usize * 30;
    let fade = RATE as usize;
    let source = rain_source(0xA1, frames + fade);
    let bed = crossfade_loop(&source, frames, fade);
    let bed_file = write_audio(dir, "rain", 2, &bed, format)?;

    let thunder_a = thunder(0x7A0, 6.0);
    let thunder_b = thunder(0x7A1, 7.5);
    let thunder_a_file = write_audio(dir, "thunder-01", 1, &thunder_a, format)?;
    let thunder_b_file = write_audio(dir, "thunder-02", 1, &thunder_b, format)?;

    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "test-rain",
        "name": { "en": "Test Rain" },
        "description": { "en": "Synthetic rain and thunder generated by oniris-cli." },
        "version": "0.1.0",
        "license": "CC0-1.0",
        "tags": ["test", "water", "storm"],
        "credits": [{
            "title": "Synthetic rain and thunder",
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }],
        "loudnessLufs": -23.0,
        "truePeakDbtp": -1.5,
        "beds": [{
            "id": "rain",
            "file": bed_file,
            "loop": { "startSample": 0, "endSample": frames as u64 },
            "gainDb": -3.0,
            "lfo": { "target": "gain", "depth": 1.5, "periodSec": 47.0 }
        }],
        "events": [{
            "id": "thunder",
            "files": [thunder_a_file, thunder_b_file],
            "minIntervalSec": 8.0,
            "maxIntervalSec": 20.0,
            "initialDelaySec": 3.0,
            "gainDb": [-12.0, -6.0],
            "pan": [-0.5, 0.5],
            "fadeInMs": 800,
            "fadeOutMs": 1200,
            "maxVoices": 1
        }]
    });
    let text = serde_json::to_string_pretty(&manifest)?;
    Manifest::from_json_str(&text).context("generated rain manifest is invalid")?;
    fs::write(dir.join("manifest.json"), text)?;
    Ok(())
}

fn generate_fire(dir: &Path, format: AudioFormat) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;
    let frames = RATE as usize * 20;
    let fade = RATE as usize;
    let source = fire_source(0xF12E, frames + fade);
    let bed = crossfade_loop(&source, frames, fade);
    let bed_file = write_audio(dir, "fire", 2, &bed, format)?;

    let crackle_a = crackle(0xC0A1, 1.2);
    let crackle_b = crackle(0xC0A2, 0.9);
    let crackle_a_file = write_audio(dir, "crackle-01", 1, &crackle_a, format)?;
    let crackle_b_file = write_audio(dir, "crackle-02", 1, &crackle_b, format)?;

    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "test-fire",
        "name": { "en": "Test Fire" },
        "description": { "en": "Synthetic fire bed and crackles generated by oniris-cli." },
        "version": "0.1.0",
        "license": "CC0-1.0",
        "tags": ["test", "fire", "night"],
        "credits": [{
            "title": "Synthetic fire and crackles",
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }],
        "loudnessLufs": -23.0,
        "truePeakDbtp": -1.5,
        "beds": [{
            "id": "fire",
            "file": bed_file,
            "loop": { "startSample": 0, "endSample": frames as u64 },
            "gainDb": -4.0,
            "lfo": { "target": "gain", "depth": 1.0, "periodSec": 31.0 }
        }],
        "events": [{
            "id": "crackle",
            "files": [crackle_a_file, crackle_b_file],
            "minIntervalSec": 2.0,
            "maxIntervalSec": 7.0,
            "gainDb": [-16.0, -8.0],
            "pan": [-0.7, 0.7],
            "fadeInMs": 20,
            "fadeOutMs": 80,
            "maxVoices": 2
        }]
    });
    let text = serde_json::to_string_pretty(&manifest)?;
    Manifest::from_json_str(&text).context("generated fire manifest is invalid")?;
    fs::write(dir.join("manifest.json"), text)?;
    Ok(())
}

/// Write one asset and return the manifest-relative path.
fn write_audio(
    dir: &Path,
    name: &str,
    channels: u16,
    samples: &[i16],
    format: AudioFormat,
) -> Result<String> {
    let relative = format!("audio/{name}.{}", format.extension());
    let path = dir.join(&relative);
    match format {
        AudioFormat::Wav => write_wav(&path, channels, samples)?,
        AudioFormat::Opus => {
            let bitrate = if channels == 1 { 64_000 } else { 96_000 };
            opus_encode::encode_opus(&path, samples, channels, RATE, bitrate)?;
        }
    }
    Ok(relative)
}

fn rain_source(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut low_l = 0.0f32;
    let mut low_r = 0.0f32;
    let mut output = Vec::with_capacity(frames * 2);

    for frame in 0..frames {
        let white_l = rng.gen_range(-1.0f32..1.0);
        let white_r = rng.gen_range(-1.0f32..1.0);
        low_l = low_l * 0.995 + white_l * 0.005;
        low_r = low_r * 0.995 + white_r * 0.005;

        let swell = 0.85 + 0.15 * (TAU * frame as f32 / (RATE as f32 * 7.0)).sin();
        let left = (white_l * 0.22 + low_l * 3.0) * swell * 0.45;
        let right = (white_r * 0.20 + low_r * 3.0) * swell * 0.45;

        output.push(left.clamp(-0.85, 0.85));
        output.push(right.clamp(-0.85, 0.85));
    }

    output
}

fn fire_source(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut low_l = 0.0f32;
    let mut low_r = 0.0f32;
    let mut pop_l = 0.0f32;
    let mut pop_r = 0.0f32;
    let mut output = Vec::with_capacity(frames * 2);

    for _ in 0..frames {
        let white_l = rng.gen_range(-1.0f32..1.0);
        let white_r = rng.gen_range(-1.0f32..1.0);
        low_l = low_l * 0.998 + white_l * 0.002;
        low_r = low_r * 0.998 + white_r * 0.002;

        if rng.gen_bool(0.00035) {
            pop_l = rng.gen_range(0.3f32..0.9);
        }
        if rng.gen_bool(0.00030) {
            pop_r = rng.gen_range(0.3f32..0.9);
        }
        pop_l *= 0.992;
        pop_r *= 0.992;

        let left = low_l * 1.8 + pop_l * rng.gen_range(-0.6f32..0.6) + white_l * 0.015;
        let right = low_r * 1.8 + pop_r * rng.gen_range(-0.6f32..0.6) + white_r * 0.015;

        output.push(left.clamp(-0.9, 0.9));
        output.push(right.clamp(-0.9, 0.9));
    }

    output
}

fn thunder(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let mut low = 0.0f32;
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let white = rng.gen_range(-1.0f32..1.0);
        low = low * 0.997 + white * 0.003;

        let attack = (time / 0.15).min(1.0);
        let decay = (-time / 1.8).exp();
        let second_rumble = (-(time - 2.2).abs() / 0.7).exp() * 0.35;
        let envelope = attack * decay + second_rumble;
        let sample = (low * 5.0 + white * 0.08) * envelope;

        output.push((sample.clamp(-0.9, 0.9) * i16::MAX as f32) as i16);
    }

    output
}

fn crackle(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let mut pop = 0.0f32;
    let mut output = Vec::with_capacity(frames);

    for _ in 0..frames {
        if rng.gen_bool(0.0015) {
            pop = rng.gen_range(0.2f32..0.8);
        }
        pop *= 0.985;
        let sample = pop * rng.gen_range(-1.0f32..1.0);
        output.push((sample.clamp(-0.9, 0.9) * i16::MAX as f32) as i16);
    }

    output
}

/// Crossfade the tail of a loop into its head. The input has `frames + fade`
/// frames; the output is exactly `frames` frames and loops at a sample boundary.
fn crossfade_loop(source: &[f32], frames: usize, fade: usize) -> Vec<i16> {
    assert!(source.len() >= (frames + fade) * 2);
    let fade = fade.min(frames);
    let mut output = Vec::with_capacity(frames * 2);

    for frame in 0..frames {
        if frame < fade {
            let t = frame as f32 / fade as f32;
            let head = (t * PI * 0.5).sin();
            let tail = (t * PI * 0.5).cos();
            for channel in 0..2 {
                let head_sample = source[frame * 2 + channel];
                let tail_sample = source[(frames + frame) * 2 + channel];
                let sample = (head_sample * head + tail_sample * tail).clamp(-1.0, 1.0);
                output.push((sample * i16::MAX as f32) as i16);
            }
        } else {
            for channel in 0..2 {
                let sample = source[frame * 2 + channel].clamp(-1.0, 1.0);
                output.push((sample * i16::MAX as f32) as i16);
            }
        }
    }

    output
}

fn write_wav(path: &Path, channels: u16, samples: &[i16]) -> Result<()> {
    let spec = WavSpec {
        channels,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .with_context(|| format!("failed to create {}", path.display()))?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oniris_core::decode_audio;

    #[test]
    fn generates_valid_pure_noise_pack() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("pure-noise");
        let frames = 10 * RATE as usize;
        generate_pure_noise(&pack, frames, RATE as usize).unwrap();

        let manifest = Manifest::load(&pack).unwrap();
        manifest.validate_files(&pack).unwrap();

        assert_eq!(manifest.id, "pure-noise");
        assert_eq!(manifest.beds.len(), 2);
        assert_eq!(manifest.credits.len(), 2);
        assert_eq!(
            manifest.credits[0].asset.as_deref(),
            Some("audio/pink-noise.opus")
        );

        let pink = decode_audio(&pack.join("audio/pink-noise.opus"), RATE).unwrap();
        let brown = decode_audio(&pack.join("audio/brown-noise.opus"), RATE).unwrap();
        assert_eq!(pink.frames, frames);
        assert_eq!(brown.frames, frames);

        let combined = mix_i16(&pink.samples, &brown.samples);
        let measurement = crate::loudness::measure(&combined, 2, RATE);
        assert!(
            (measurement.integrated_lufs + 23.0).abs() < 1.0,
            "loudness {}",
            measurement.integrated_lufs
        );
        assert!(measurement.true_peak_dbtp < -1.5);
    }
}
