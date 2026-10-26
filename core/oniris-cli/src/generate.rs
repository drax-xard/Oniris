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
const RAIN_LOOP_FRAMES: usize = 60 * RATE as usize;
const RAIN_FADE_FRAMES: usize = 2 * RATE as usize;

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
        "procedural": true,
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

struct ProceduralRecipe {
    id: &'static str,
    build: fn(&Path) -> Result<()>,
}

const PROCEDURAL_RECIPES: &[ProceduralRecipe] = &[
    ProceduralRecipe {
        id: "pure-noise",
        build: generate_noise_pack,
    },
    ProceduralRecipe {
        id: "rainy-night-procedural",
        build: generate_procedural_rain_pack,
    },
    ProceduralRecipe {
        id: "deep-space-cabin",
        build: generate_deep_space_pack,
    },
    ProceduralRecipe {
        id: "enchanted-forest",
        build: generate_enchanted_forest_pack,
    },
];

pub fn generate_procedural_packs(out: &Path, only: Option<&str>) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;
    if let Some(id) = only {
        let recipe = PROCEDURAL_RECIPES
            .iter()
            .find(|recipe| recipe.id == id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown procedural pack {id:?}; available: {}",
                    PROCEDURAL_RECIPES
                        .iter()
                        .map(|recipe| recipe.id)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        (recipe.build)(out)
    } else {
        for recipe in PROCEDURAL_RECIPES {
            (recipe.build)(out)?;
        }
        Ok(())
    }
}

pub fn generate_procedural_rain_pack(out: &Path) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;
    let dir = out.join("rainy-night-procedural");
    generate_procedural_rain(&dir, RAIN_LOOP_FRAMES, RAIN_FADE_FRAMES, 5.0, 6, 0.4)?;
    println!("generated {}", dir.display());
    Ok(())
}

fn generate_procedural_rain(
    dir: &Path,
    loop_frames: usize,
    fade_frames: usize,
    thunder_seconds: f32,
    drip_count: usize,
    drip_seconds: f32,
) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;

    let source = procedural_rain_source(0x7A1, loop_frames + fade_frames);
    let rain = crossfade_loop(&source, loop_frames, fade_frames);
    let before = loudness::measure(&rain, 2, RATE);
    if !before.integrated_lufs.is_finite() {
        anyhow::bail!("procedural rain measured as silence");
    }
    let rain = loudness::apply_gain(&rain, -23.0 - before.integrated_lufs);
    let after = loudness::measure(&rain, 2, RATE);

    let rain_file = write_audio(dir, "rain", 2, &rain, AudioFormat::Opus)?;

    let thunder_durations = [
        thunder_seconds,
        thunder_seconds * 1.2,
        thunder_seconds * 0.9,
    ];
    let mut thunder_files = Vec::with_capacity(thunder_durations.len());
    let mut credits = vec![serde_json::json!({
        "asset": &rain_file,
        "title": "Procedural rain",
        "author": "Oniris",
        "source": "synthetic",
        "license": "CC0-1.0",
        "modified": false
    })];
    for (index, seconds) in thunder_durations.iter().enumerate() {
        let event = normalize_peak(&thunder(0x7B0 + index as u64, *seconds), -3.0);
        let name = format!("thunder-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        thunder_files.push(file);
    }

    let mut drip_files = Vec::with_capacity(drip_count);
    for index in 0..drip_count {
        let event = normalize_peak(&drip_event(0xD81 + index as u64, drip_seconds), -6.0);
        let name = format!("drip-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        drip_files.push(file);
    }

    let true_peak = if after.true_peak_dbtp.is_finite() {
        (f64::from(after.true_peak_dbtp) * 10.0).round() / 10.0
    } else {
        -1.5
    };
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "rainy-night-procedural",
        "name": { "en": "Rainy Night (Synthetic)" },
        "description": {
            "en": "A fully synthesized rainstorm with distant thunder and scattered droplets. No recordings."
        },
        "version": "1.0.0",
        "license": "CC0-1.0",
        "procedural": true,
        "tags": ["rain", "storm", "night", "procedural"],
        "loudnessLufs": -23.0,
        "truePeakDbtp": true_peak,
        "beds": [{
            "id": "rain",
            "file": &rain_file,
            "loop": { "startSample": 0, "endSample": loop_frames },
            "gainDb": 0.0,
            "lfo": { "target": "gain", "depth": 0.5, "periodSec": 37.0 }
        }],
        "events": [
            {
                "id": "thunder",
                "files": thunder_files,
                "minIntervalSec": 18.0,
                "maxIntervalSec": 45.0,
                "initialDelaySec": 6.0,
                "gainDb": [-14.0, -6.0],
                "pan": [-0.4, 0.4],
                "fadeInMs": 150,
                "fadeOutMs": 3000,
                "maxVoices": 1
            },
            {
                "id": "drips",
                "files": drip_files,
                "minIntervalSec": 1.5,
                "maxIntervalSec": 6.0,
                "initialDelaySec": 0.5,
                "gainDb": [-24.0, -12.0],
                "pan": [-0.8, 0.8],
                "fadeInMs": 10,
                "fadeOutMs": 80,
                "maxVoices": 3
            }
        ],
        "credits": credits
    });
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

fn procedural_rain_source(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut output = vec![0.0f32; frames * 2];

    // Dense droplets: short damped resonances scattered across the loop.
    let drops = (frames as f32 * 0.0015) as usize;
    for _ in 0..drops {
        let start = rng.gen_range(0..frames.saturating_sub(1));
        let frequency = rng.gen_range(1_500.0f32..6_000.0);
        let amplitude = rng.gen_range(0.008f32..0.060);
        let decay = rng.gen_range(0.004f32..0.018);
        let pan = rng.gen_range(-0.8f32..0.8);
        let (pan_l, pan_r) = pan_gains(pan);
        let length = ((decay * 6.0 * RATE as f32) as usize).max(1);

        for offset in 0..length {
            let index = start + offset;
            if index >= frames {
                break;
            }
            let time = offset as f32 / RATE as f32;
            let envelope = (-time / decay).exp();
            let sample = (TAU * frequency * time).sin() * envelope * amplitude;
            output[index * 2] += sample * pan_l;
            output[index * 2 + 1] += sample * pan_r;
        }
    }

    // Pink-ish hiss with a slow swell. Pink noise is closer to rain than a
    // flat-spectrum hiss and is less fatiguing at low volume.
    let mut low_l = 0.0f32;
    let mut low_r = 0.0f32;
    let mut pink_l = [0.0f32; 7];
    let mut pink_r = [0.0f32; 7];
    let low_pass = 0.96f32;
    for frame in 0..frames {
        let white_l = rng.gen_range(-1.0f32..1.0);
        let white_r = rng.gen_range(-1.0f32..1.0);
        let hiss_l = pink_sample(&mut pink_l, white_l);
        let hiss_r = pink_sample(&mut pink_r, white_r);
        low_l = low_pass * low_l + (1.0 - low_pass) * hiss_l;
        low_r = low_pass * low_r + (1.0 - low_pass) * hiss_r;
        let swell = 0.85 + 0.15 * (TAU * frame as f32 / (RATE as f32 * 9.0)).sin();
        output[frame * 2] += (hiss_l - low_l) * 0.24 * swell;
        output[frame * 2 + 1] += (hiss_r - low_r) * 0.24 * swell;
    }

    for sample in &mut output {
        *sample = sample.clamp(-0.95, 0.95);
    }
    output
}

fn drip_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let frequency = rng.gen_range(1_200.0f32..3_600.0);
    let decay = rng.gen_range(0.015f32..0.060);
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let envelope = (-time / decay).exp();
        let pitch_drop = 1.0 - 0.15 * (time / (decay * 6.0)).min(1.0);
        let mut sample = (TAU * frequency * pitch_drop * time).sin() * envelope;
        if time < 0.002 {
            sample += rng.gen_range(-1.0f32..1.0) * (1.0 - time / 0.002) * 0.3;
        }
        output.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
    }
    output
}

fn normalize_peak(samples: &[i16], target_dbfs: f32) -> Vec<i16> {
    let peak = samples
        .iter()
        .map(|sample| sample.unsigned_abs())
        .max()
        .unwrap_or(0) as f32
        / i16::MAX as f32;
    if peak <= 0.0 {
        return samples.to_vec();
    }
    let gain_db = target_dbfs - 20.0 * peak.log10();
    loudness::apply_gain(samples, gain_db)
}

fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    if pan < 0.0 {
        (1.0, 1.0 + pan)
    } else {
        (1.0 - pan, 1.0)
    }
}

#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn bandpass(center: f32, q: f32) -> Self {
        let w0 = TAU * center / RATE as f32;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            b1: 0.0,
            b2: -alpha / a0,
            a1: (-2.0 * w0.cos()) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn low_pass(center: f32, q: f32) -> Self {
        let w0 = TAU * center / RATE as f32;
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos) * 0.5 / a0,
            b1: (1.0 - cos) / a0,
            b2: (1.0 - cos) * 0.5 / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn high_pass(center: f32, q: f32) -> Self {
        let w0 = TAU * center / RATE as f32;
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 + cos) * 0.5 / a0,
            b1: -(1.0 + cos) / a0,
            b2: (1.0 + cos) * 0.5 / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

fn mix_scaled(left: &[i16], right: &[i16], left_gain: f32, right_gain: f32) -> Vec<i16> {
    left.iter()
        .zip(right)
        .map(|(a, b)| {
            (f32::from(*a) * left_gain + f32::from(*b) * right_gain)
                .round()
                .clamp(-32_768.0, 32_767.0) as i16
        })
        .collect()
}

fn normalize_bed_pair(
    left: &mut Vec<i16>,
    right: &mut Vec<i16>,
    left_gain: f32,
    right_gain: f32,
) -> Result<loudness::Measurement> {
    let combined = mix_scaled(left, right, left_gain, right_gain);
    let before = loudness::measure(&combined, 2, RATE);
    if !before.integrated_lufs.is_finite() {
        anyhow::bail!("procedural beds measured as silence");
    }
    let gain_db = -23.0 - before.integrated_lufs;
    *left = loudness::apply_gain(left, gain_db);
    *right = loudness::apply_gain(right, gain_db);
    Ok(loudness::measure(
        &mix_scaled(left, right, left_gain, right_gain),
        2,
        RATE,
    ))
}

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub fn generate_deep_space_pack(out: &Path) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;
    let dir = out.join("deep-space-cabin");
    generate_deep_space(&dir, 60 * RATE as usize, 2 * RATE as usize)?;
    println!("generated {}", dir.display());
    Ok(())
}

fn generate_deep_space(dir: &Path, loop_frames: usize, fade_frames: usize) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;

    let engine_source = engine_drone(0xE061, loop_frames + fade_frames);
    let air_source = air_bed(0x41A2, loop_frames + fade_frames);
    let mut engine = crossfade_loop(&engine_source, loop_frames, fade_frames);
    let mut air = crossfade_loop(&air_source, loop_frames, fade_frames);
    let after = normalize_bed_pair(
        &mut engine,
        &mut air,
        db_to_linear(-6.0),
        db_to_linear(-12.0),
    )?;

    let engine_file = write_audio(dir, "engine", 2, &engine, AudioFormat::Opus)?;
    let air_file = write_audio(dir, "air", 2, &air, AudioFormat::Opus)?;
    let mut credits = vec![
        serde_json::json!({
            "asset": &engine_file,
            "title": "Engine drone",
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }),
        serde_json::json!({
            "asset": &air_file,
            "title": "Air handling",
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }),
    ];

    let mut beep_files = Vec::new();
    for (index, (seed, seconds)) in [(0xB01u64, 0.5f32), (0xB02, 0.65), (0xB03, 0.4)]
        .iter()
        .enumerate()
    {
        let event = normalize_peak(&beep_event(*seed, *seconds), -6.0);
        let name = format!("beep-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        beep_files.push(file);
    }

    let mut creak_files = Vec::new();
    for (index, (seed, seconds)) in [(0xC01u64, 1.2f32), (0xC02, 1.5), (0xC03, 0.9)]
        .iter()
        .enumerate()
    {
        let event = normalize_peak(&creak_event(*seed, *seconds), -3.0);
        let name = format!("creak-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        creak_files.push(file);
    }

    let mut airlock_files = Vec::new();
    for (index, (seed, seconds)) in [(0xA01u64, 1.0f32), (0xA02, 1.4)].iter().enumerate() {
        let event = normalize_peak(&airlock_event(*seed, *seconds), -3.0);
        let name = format!("airlock-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        airlock_files.push(file);
    }

    let true_peak = rounded_db(after.true_peak_dbtp);
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "deep-space-cabin",
        "name": { "en": "Deep Space Cabin" },
        "description": {
            "en": "A quiet habitat drifting between stars: low engine hum, air handling, and rare soft system tones."
        },
        "version": "1.0.0",
        "license": "CC0-1.0",
        "procedural": true,
        "tags": ["space", "scifi", "drone", "procedural"],
        "loudnessLufs": -23.0,
        "truePeakDbtp": true_peak,
        "beds": [
            {
                "id": "engine",
                "file": &engine_file,
                "loop": { "startSample": 0, "endSample": loop_frames },
                "gainDb": -6.0,
                "lfo": { "target": "gain", "depth": 0.4, "periodSec": 61.0 }
            },
            {
                "id": "air",
                "file": &air_file,
                "loop": { "startSample": 0, "endSample": loop_frames },
                "gainDb": -12.0,
                "pan": 0.15,
                "lfo": { "target": "pan", "depth": 0.2, "periodSec": 43.0 }
            }
        ],
        "events": [
            {
                "id": "beep",
                "files": beep_files,
                "minIntervalSec": 30.0,
                "maxIntervalSec": 90.0,
                "initialDelaySec": 12.0,
                "gainDb": [-18.0, -12.0],
                "pan": [-0.3, 0.3],
                "fadeInMs": 30,
                "fadeOutMs": 300,
                "maxVoices": 1
            },
            {
                "id": "creak",
                "files": creak_files,
                "minIntervalSec": 20.0,
                "maxIntervalSec": 70.0,
                "initialDelaySec": 8.0,
                "gainDb": [-20.0, -12.0],
                "pan": [-0.6, 0.6],
                "fadeInMs": 200,
                "fadeOutMs": 800,
                "maxVoices": 1
            },
            {
                "id": "airlock",
                "files": airlock_files,
                "minIntervalSec": 60.0,
                "maxIntervalSec": 180.0,
                "initialDelaySec": 30.0,
                "gainDb": [-22.0, -16.0],
                "pan": [-0.4, 0.4],
                "fadeInMs": 100,
                "fadeOutMs": 1200,
                "maxVoices": 1
            }
        ],
        "credits": credits
    });
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

pub fn generate_enchanted_forest_pack(out: &Path) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;
    let dir = out.join("enchanted-forest");
    generate_enchanted_forest(&dir, 60 * RATE as usize, 2 * RATE as usize)?;
    println!("generated {}", dir.display());
    Ok(())
}

fn generate_enchanted_forest(dir: &Path, loop_frames: usize, fade_frames: usize) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;

    let wind_source = wind_bed(0xF03E, loop_frames + fade_frames);
    let brook_source = brook_bed(0xB20A, loop_frames + fade_frames);
    let mut wind = crossfade_loop(&wind_source, loop_frames, fade_frames);
    let mut brook = crossfade_loop(&brook_source, loop_frames, fade_frames);
    let after = normalize_bed_pair(
        &mut wind,
        &mut brook,
        db_to_linear(-8.0),
        db_to_linear(-12.0),
    )?;

    let wind_file = write_audio(dir, "wind", 2, &wind, AudioFormat::Opus)?;
    let brook_file = write_audio(dir, "brook", 2, &brook, AudioFormat::Opus)?;
    let mut credits = vec![
        serde_json::json!({
            "asset": &wind_file,
            "title": "Forest wind",
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }),
        serde_json::json!({
            "asset": &brook_file,
            "title": "Forest brook",
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }),
    ];

    let mut chime_files = Vec::new();
    for index in 0..4u64 {
        let event = normalize_peak(&chime_event(0x51A0 + index, 2.5), -6.0);
        let name = format!("chime-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        chime_files.push(file);
    }

    let mut owl_files = Vec::new();
    for index in 0..2u64 {
        let event = normalize_peak(&owl_event(0x0A11 + index, 1.8), -6.0);
        let name = format!("owl-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        owl_files.push(file);
    }

    let mut rustle_files = Vec::new();
    for index in 0..3u64 {
        let event = normalize_peak(&rustle_event(0x2E50 + index, 1.0), -6.0);
        let name = format!("rustle-{:02}", index + 1);
        let file = write_audio(dir, &name, 1, &event, AudioFormat::Opus)?;
        credits.push(serde_json::json!({
            "asset": &file,
            "title": name,
            "author": "Oniris",
            "source": "synthetic",
            "license": "CC0-1.0",
            "modified": false
        }));
        rustle_files.push(file);
    }

    let true_peak = rounded_db(after.true_peak_dbtp);
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "enchanted-forest",
        "name": { "en": "Enchanted Forest" },
        "description": {
            "en": "A moonlit forest with soft wind, a hidden brook, distant owls, and rare chimes."
        },
        "version": "1.0.0",
        "license": "CC0-1.0",
        "procedural": true,
        "tags": ["forest", "fantasy", "magic", "procedural"],
        "loudnessLufs": -23.0,
        "truePeakDbtp": true_peak,
        "beds": [
            {
                "id": "wind",
                "file": &wind_file,
                "loop": { "startSample": 0, "endSample": loop_frames },
                "gainDb": -8.0,
                "lfo": { "target": "gain", "depth": 0.8, "periodSec": 53.0 }
            },
            {
                "id": "brook",
                "file": &brook_file,
                "loop": { "startSample": 0, "endSample": loop_frames },
                "gainDb": -12.0,
                "pan": -0.2
            }
        ],
        "events": [
            {
                "id": "chime",
                "files": chime_files,
                "minIntervalSec": 20.0,
                "maxIntervalSec": 60.0,
                "initialDelaySec": 10.0,
                "gainDb": [-20.0, -14.0],
                "pan": [-0.7, 0.7],
                "fadeInMs": 20,
                "fadeOutMs": 1500,
                "maxVoices": 1
            },
            {
                "id": "owl",
                "files": owl_files,
                "minIntervalSec": 40.0,
                "maxIntervalSec": 120.0,
                "initialDelaySec": 25.0,
                "gainDb": [-20.0, -14.0],
                "pan": [-0.5, 0.5],
                "fadeInMs": 100,
                "fadeOutMs": 600,
                "maxVoices": 1
            },
            {
                "id": "rustle",
                "files": rustle_files,
                "minIntervalSec": 15.0,
                "maxIntervalSec": 50.0,
                "initialDelaySec": 5.0,
                "gainDb": [-24.0, -18.0],
                "pan": [-0.8, 0.8],
                "fadeInMs": 10,
                "fadeOutMs": 400,
                "maxVoices": 2
            }
        ],
        "credits": credits
    });
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

fn rounded_db(value: f32) -> f64 {
    if value.is_finite() {
        (f64::from(value) * 10.0).round() / 10.0
    } else {
        -1.5
    }
}

fn engine_drone(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let partials = [
        (55.0f32, 1.0f32, 0.0008f32),
        (110.0, 0.50, -0.0006),
        (165.0, 0.28, 0.0004),
        (220.0, 0.16, -0.0003),
        (275.0, 0.09, 0.0002),
        (330.0, 0.05, -0.0001),
    ];
    let phases: Vec<f32> = (0..partials.len())
        .map(|_| rng.gen_range(0.0..TAU))
        .collect();
    let periods: Vec<f32> = (0..partials.len())
        .map(|_| rng.gen_range(17.0..53.0))
        .collect();
    let lfo_phases: Vec<f32> = (0..partials.len())
        .map(|_| rng.gen_range(0.0..TAU))
        .collect();
    let mut lp_l = Biquad::low_pass(420.0, 0.7);
    let mut lp_r = Biquad::low_pass(420.0, 0.7);
    let mut output = Vec::with_capacity(frames * 2);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let mut left = 0.0f32;
        let mut right = 0.0f32;
        for (index, (frequency, amplitude, detune)) in partials.iter().enumerate() {
            let lfo = (TAU * time / periods[index] + lfo_phases[index]).sin();
            let drift = 1.0 + detune + 0.0006 * lfo;
            let phase = TAU * frequency * drift * time + phases[index];
            let gain = amplitude * (0.85 + 0.15 * lfo);
            left += phase.sin() * gain;
            right += (phase + 0.04).sin() * gain;
        }
        left += rng.gen_range(-1.0f32..1.0) * 0.01;
        right += rng.gen_range(-1.0f32..1.0) * 0.01;
        output.push((lp_l.process(left) * 0.45).clamp(-0.9, 0.9));
        output.push((lp_r.process(right) * 0.45).clamp(-0.9, 0.9));
    }
    output
}

fn air_bed(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut bp_l = Biquad::bandpass(700.0, 0.7);
    let mut bp_r = Biquad::bandpass(700.0, 0.7);
    let mut lp_l = Biquad::low_pass(1800.0, 0.7);
    let mut lp_r = Biquad::low_pass(1800.0, 0.7);
    let mut rumble_l = Biquad::low_pass(120.0, 0.7);
    let mut rumble_r = Biquad::low_pass(120.0, 0.7);
    let mut output = Vec::with_capacity(frames * 2);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let swell = 0.6 + 0.4 * (0.5 + 0.5 * (TAU * time / 41.0).sin());
        let white_l = rng.gen_range(-1.0f32..1.0);
        let white_r = rng.gen_range(-1.0f32..1.0);
        let air_l = bp_l.process(white_l) * 0.6 + lp_l.process(white_l) * 0.25;
        let air_r = bp_r.process(white_r) * 0.6 + lp_r.process(white_r) * 0.25;
        let low_l = rumble_l.process(white_l) * 0.08;
        let low_r = rumble_r.process(white_r) * 0.08;
        output.push(((air_l + low_l) * swell * 0.55).clamp(-0.9, 0.9));
        output.push(((air_r + low_r) * swell * 0.55).clamp(-0.9, 0.9));
    }
    output
}

fn beep_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let frequency = rng.gen_range(620.0f32..1_180.0);
    let modulator = frequency * rng.gen_range(1.3f32..2.1);
    let index = rng.gen_range(0.6f32..2.0);
    let decay = rng.gen_range(0.08f32..0.22);
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let attack = (time / 0.008).min(1.0);
        let envelope = attack * (-time / decay).exp();
        let sample =
            (TAU * frequency * time + index * (TAU * modulator * time).sin()).sin() * envelope;
        output.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
    }
    output
}

fn creak_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let modes: Vec<(f32, f32, f32)> = (0..5)
        .map(|_| {
            (
                rng.gen_range(140.0f32..900.0),
                rng.gen_range(0.08f32..0.5),
                rng.gen_range(0.2f32..1.0),
            )
        })
        .collect();
    let mut lp = Biquad::low_pass(1800.0, 0.7);
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let attack = (time / 0.02).min(1.0);
        let mut sample = 0.0f32;
        for (frequency, decay, amplitude) in &modes {
            let drift = 1.0 + 0.01 * (TAU * 1.7 * time).sin();
            sample += (TAU * frequency * drift * time).sin() * amplitude * (-time / decay).exp();
        }
        if time < 0.02 {
            sample += rng.gen_range(-1.0f32..1.0) * (1.0 - time / 0.02) * 0.5;
        }
        sample = lp.process(sample) * attack * (-time / 0.6).exp() * 0.4;
        output.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
    }
    output
}

fn airlock_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let frequency = rng.gen_range(45.0f32..75.0);
    let decay = rng.gen_range(0.25f32..0.5);
    let mut lp = Biquad::low_pass(300.0, 0.7);
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let attack = (time / 0.01).min(1.0);
        let envelope = attack * (-time / decay).exp();
        let thump = (TAU * frequency * time).sin() * 0.8;
        let noise = lp.process(rng.gen_range(-1.0f32..1.0)) * 0.3;
        let sample = (thump + noise) * envelope;
        output.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
    }
    output
}

fn wind_bed(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut lp_l = Biquad::low_pass(1400.0, 0.7);
    let mut lp_r = Biquad::low_pass(1400.0, 0.7);
    let mut hp_l = Biquad::high_pass(120.0, 0.7);
    let mut hp_r = Biquad::high_pass(120.0, 0.7);
    let mut leaf_l = Biquad::bandpass(650.0, 2.5);
    let mut leaf_r = Biquad::bandpass(650.0, 2.5);
    let gust_a = rng.gen_range(0.0..TAU);
    let gust_b = rng.gen_range(0.0..TAU);
    let mut output = Vec::with_capacity(frames * 2);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let gust = 0.55
            + 0.45
                * (0.5 + 0.5 * (TAU * time / 43.0 + gust_a).sin())
                * (0.5 + 0.5 * (TAU * time / 67.0 + gust_b).sin());
        let white_l = rng.gen_range(-1.0f32..1.0);
        let white_r = rng.gen_range(-1.0f32..1.0);
        let body_l = hp_l.process(lp_l.process(white_l));
        let body_r = hp_r.process(lp_r.process(white_r));
        let leaf_l_sample = leaf_l.process(white_l) * 0.35;
        let leaf_r_sample = leaf_r.process(white_r) * 0.35;
        output.push(((body_l * 0.6 + leaf_l_sample) * gust * 0.5).clamp(-0.9, 0.9));
        output.push(((body_r * 0.6 + leaf_r_sample) * gust * 0.5).clamp(-0.9, 0.9));
    }
    output
}

fn brook_bed(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut hp_l = Biquad::high_pass(300.0, 0.7);
    let mut hp_r = Biquad::high_pass(300.0, 0.7);
    let mut lp_l = Biquad::low_pass(3200.0, 0.7);
    let mut lp_r = Biquad::low_pass(3200.0, 0.7);
    let mut output = vec![0.0f32; frames * 2];

    for frame in 0..frames {
        let white_l = rng.gen_range(-1.0f32..1.0);
        let white_r = rng.gen_range(-1.0f32..1.0);
        let body_l = lp_l.process(hp_l.process(white_l));
        let body_r = lp_r.process(hp_r.process(white_r));
        output[frame * 2] += body_l * 0.16;
        output[frame * 2 + 1] += body_r * 0.16;
    }

    let bubbles = (frames as f32 / RATE as f32 * 8.0) as usize;
    for _ in 0..bubbles {
        let start = rng.gen_range(0..frames.saturating_sub(1));
        let frequency = rng.gen_range(350.0f32..1_200.0);
        let length = ((rng.gen_range(0.04f32..0.12) * RATE as f32) as usize).max(1);
        let amplitude = rng.gen_range(0.02f32..0.08);
        let pan = rng.gen_range(-0.7f32..0.7);
        let (pan_l, pan_r) = pan_gains(pan);

        for offset in 0..length {
            let index = start + offset;
            if index >= frames {
                break;
            }
            let progress = offset as f32 / length as f32;
            let envelope = (1.0 - progress).powi(2);
            let pitch = frequency * (1.0 + 0.6 * progress);
            let sample = (TAU * pitch * offset as f32 / RATE as f32).sin() * envelope * amplitude;
            output[index * 2] += sample * pan_l;
            output[index * 2 + 1] += sample * pan_r;
        }
    }

    for sample in &mut output {
        *sample = sample.clamp(-0.95, 0.95);
    }
    output
}

fn chime_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let notes = [440.0f32, 523.25, 587.33, 659.25, 783.99, 880.0, 1_046.5];
    let base = notes[rng.gen_range(0..notes.len())] * rng.gen_range(0.98f32..1.02);
    let decay = rng.gen_range(0.8f32..2.0);
    let partials = [
        (1.0f32, 1.0f32, 1.0f32),
        (2.01, 0.45, 0.7),
        (3.03, 0.22, 0.5),
        (4.07, 0.10, 0.35),
        (5.11, 0.05, 0.25),
    ];
    let mut output = Vec::with_capacity(frames);

    for frame in 0..frames {
        let time = frame as f32 / RATE as f32;
        let attack = (time / 0.01).min(1.0);
        let mut sample = 0.0f32;
        for (ratio, amplitude, decay_scale) in &partials {
            sample += (TAU * base * ratio * time).sin()
                * amplitude
                * (-time / (decay * decay_scale)).exp();
        }
        let sample = (sample * attack * 0.3).clamp(-1.0, 1.0);
        output.push((sample * i16::MAX as f32) as i16);
    }
    output
}

fn owl_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let base = rng.gen_range(310.0f32..430.0);
    let hoots = if rng.gen_bool(0.4) { 3 } else { 2 };
    let hoot_length = (0.32 * RATE as f32) as usize;
    let gap = (0.28 * RATE as f32) as usize;
    let mut lp = Biquad::low_pass(900.0, 0.7);
    let mut output = vec![0.0f32; frames];

    for hoot in 0..hoots {
        let start = hoot * (hoot_length + gap);
        for offset in 0..hoot_length {
            let index = start + offset;
            if index >= frames {
                break;
            }
            let time = offset as f32 / RATE as f32;
            let envelope = (offset as f32 / hoot_length as f32 * PI).sin().powi(2);
            let frequency = base * (1.0 + 0.01 * hoot as f32) * (1.0 + 0.02 * (time * 8.0).sin());
            let sample =
                (TAU * frequency * time).sin() + 0.25 * (TAU * 2.0 * frequency * time).sin();
            output[index] = lp.process(sample * envelope * 0.4);
        }
    }
    output
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

fn rustle_event(seed: u64, seconds: f32) -> Vec<i16> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let frames = (seconds * RATE as f32) as usize;
    let grains = 3 + rng.gen_range(0..3);
    let mut bp = Biquad::bandpass(rng.gen_range(1_800.0f32..3_800.0), 0.8);
    let mut output = vec![0.0f32; frames];

    for _ in 0..grains {
        let start = rng.gen_range(0..frames.saturating_sub(1));
        let length = ((rng.gen_range(0.04f32..0.15) * RATE as f32) as usize).max(1);
        let amplitude = rng.gen_range(0.2f32..0.7);
        for offset in 0..length {
            let index = start + offset;
            if index >= frames {
                break;
            }
            let envelope = (offset as f32 / length as f32 * PI).sin();
            let noise = bp.process(rng.gen_range(-1.0f32..1.0));
            output[index] += noise * envelope * amplitude;
        }
    }
    output
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
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
    fn generates_valid_procedural_rain_pack() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("rainy-night-procedural");
        let frames = 10 * RATE as usize;
        generate_procedural_rain(&pack, frames, RATE as usize, 1.2, 4, 0.25).unwrap();

        let manifest = Manifest::load(&pack).unwrap();
        manifest.validate_files(&pack).unwrap();
        assert!(manifest.procedural);
        assert_eq!(manifest.id, "rainy-night-procedural");
        assert_eq!(manifest.beds.len(), 1);
        assert_eq!(manifest.events.len(), 2);
        assert_eq!(manifest.events[0].files.len(), 3);
        assert_eq!(manifest.events[1].files.len(), 4);
        assert_eq!(manifest.credits.len(), 8);

        let audio = decode_audio(&pack.join("audio/rain.opus"), RATE).unwrap();
        assert_eq!(audio.frames, frames);
        let measurement = crate::loudness::measure(&audio.samples, audio.channels, RATE);
        assert!(
            (measurement.integrated_lufs + 23.0).abs() < 1.0,
            "loudness {}",
            measurement.integrated_lufs
        );
    }

    #[test]
    fn generates_valid_deep_space_pack() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("deep-space-cabin");
        let frames = 10 * RATE as usize;
        generate_deep_space(&pack, frames, RATE as usize).unwrap();

        let manifest = Manifest::load(&pack).unwrap();
        manifest.validate_files(&pack).unwrap();
        assert!(manifest.procedural);
        assert_eq!(manifest.id, "deep-space-cabin");
        assert_eq!(manifest.beds.len(), 2);
        assert_eq!(manifest.events.len(), 3);
        assert_eq!(manifest.events[0].files.len(), 3);
        assert_eq!(manifest.events[1].files.len(), 3);
        assert_eq!(manifest.events[2].files.len(), 2);
        assert_eq!(manifest.credits.len(), 10);

        let engine = decode_audio(&pack.join("audio/engine.opus"), RATE).unwrap();
        let air = decode_audio(&pack.join("audio/air.opus"), RATE).unwrap();
        assert_eq!(engine.frames, frames);
        assert_eq!(air.frames, frames);
        let combined = mix_scaled(
            &engine.samples,
            &air.samples,
            db_to_linear(-6.0),
            db_to_linear(-12.0),
        );
        let measurement = crate::loudness::measure(&combined, 2, RATE);
        assert!(
            (measurement.integrated_lufs + 23.0).abs() < 1.5,
            "loudness {}",
            measurement.integrated_lufs
        );
    }

    #[test]
    fn generates_valid_enchanted_forest_pack() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("enchanted-forest");
        let frames = 10 * RATE as usize;
        generate_enchanted_forest(&pack, frames, RATE as usize).unwrap();

        let manifest = Manifest::load(&pack).unwrap();
        manifest.validate_files(&pack).unwrap();
        assert!(manifest.procedural);
        assert_eq!(manifest.id, "enchanted-forest");
        assert_eq!(manifest.beds.len(), 2);
        assert_eq!(manifest.events.len(), 3);
        assert_eq!(manifest.events[0].files.len(), 4);
        assert_eq!(manifest.events[1].files.len(), 2);
        assert_eq!(manifest.events[2].files.len(), 3);
        assert_eq!(manifest.credits.len(), 11);

        let wind = decode_audio(&pack.join("audio/wind.opus"), RATE).unwrap();
        let brook = decode_audio(&pack.join("audio/brook.opus"), RATE).unwrap();
        assert_eq!(wind.frames, frames);
        assert_eq!(brook.frames, frames);
        let combined = mix_scaled(
            &wind.samples,
            &brook.samples,
            db_to_linear(-8.0),
            db_to_linear(-12.0),
        );
        let measurement = crate::loudness::measure(&combined, 2, RATE);
        assert!(
            (measurement.integrated_lufs + 23.0).abs() < 1.5,
            "loudness {}",
            measurement.integrated_lufs
        );
    }

    #[test]
    fn generates_valid_pure_noise_pack() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("pure-noise");
        let frames = 10 * RATE as usize;
        generate_pure_noise(&pack, frames, RATE as usize).unwrap();

        let manifest = Manifest::load(&pack).unwrap();
        manifest.validate_files(&pack).unwrap();

        assert_eq!(manifest.id, "pure-noise");
        assert!(manifest.procedural);
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
