//! Procedural pack generation.
//!
//! These are development packs, but they follow the rules launch content will
//! follow: layered sources instead of raw white noise, band-limited transients,
//! no subsonic waste, whole-cycle modulation inside the loop, and measured
//! loudness / true peak written into the manifest.
//!
//! Quality targets live in `docs/PACK_QUALITY_REVIEW.md` and are asserted by the
//! test at the bottom of this file.

use std::f32::consts::PI;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use hound::{WavSpec, WavWriter};
use oniris_pack::Manifest;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::dsp::{self, db_to_gain, hit_envelope, Biquad, BrownNoise, DcBlocker, PinkNoise};

const RATE: u32 = dsp::RATE;
const RAIN_SECONDS: usize = 45;
const FIRE_SECONDS: usize = 40;
const FADE_SECONDS: f32 = 1.5;

/// Bed loudness target. Events carry their own target because a thunderclap
/// needs peak headroom a steady bed does not.
const BED_LUFS: f32 = -23.0;
const THUNDER_LUFS: f32 = -20.0;
const CRACKLE_LUFS: f32 = -20.0;
const TRUE_PEAK_LIMIT: f32 = -1.5;

pub fn generate_test_packs(out: &Path) -> Result<()> {
    fs::create_dir_all(out).with_context(|| format!("failed to create {}", out.display()))?;

    let rain_dir = out.join("test-rain");
    generate_rain(&rain_dir)?;
    println!("generated {}", rain_dir.display());

    let fire_dir = out.join("test-fire");
    generate_fire(&fire_dir)?;
    println!("generated {}", fire_dir.display());

    Ok(())
}

fn generate_rain(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;
    let frames = RATE as usize * RAIN_SECONDS;
    let bed = rain_bed(0xA1);
    write_wav(&dir.join("audio/rain.wav"), 2, &bed)?;

    let thunder_a = thunder(0x7A0, 6.0);
    let thunder_b = thunder(0x7A1, 7.5);
    write_wav(&dir.join("audio/thunder-01.wav"), 1, &thunder_a)?;
    write_wav(&dir.join("audio/thunder-02.wav"), 1, &thunder_b)?;

    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "test-rain",
        "name": { "en": "Test Rain" },
        "description": {
            "en": "Procedural rain bed with granular droplets and rolling thunder."
        },
        "version": "0.2.0",
        "license": "CC0-1.0",
        "tags": ["test", "water", "storm"],
        "loudnessLufs": BED_LUFS,
        "truePeakDbtp": TRUE_PEAK_LIMIT,
        "beds": [{
            "id": "rain",
            "file": "audio/rain.wav",
            "loop": { "startSample": 0, "endSample": frames as u64 },
            "gainDb": 0.0,
            "lfo": { "target": "gain", "depth": 1.5, "periodSec": 47.0 }
        }],
        "events": [{
            "id": "thunder",
            "files": ["audio/thunder-01.wav", "audio/thunder-02.wav"],
            "minIntervalSec": 14.0,
            "maxIntervalSec": 34.0,
            "initialDelaySec": 6.0,
            "gainDb": [-7.0, -2.0],
            "pan": [-0.35, 0.35],
            "fadeInMs": 400,
            "fadeOutMs": 1500,
            "maxVoices": 1
        }]
    });
    write_manifest(dir, &manifest)
}

fn generate_fire(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir.join("audio"))?;
    let frames = RATE as usize * FIRE_SECONDS;
    let bed = fire_bed(0xF12E);
    write_wav(&dir.join("audio/fire.wav"), 2, &bed)?;

    let crackle_a = crackle(0xC0A1, 1.4);
    let crackle_b = crackle(0xC0A2, 1.1);
    write_wav(&dir.join("audio/crackle-01.wav"), 1, &crackle_a)?;
    write_wav(&dir.join("audio/crackle-02.wav"), 1, &crackle_b)?;

    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "test-fire",
        "name": { "en": "Test Fire" },
        "description": {
            "en": "Procedural fire bed with a low roar, hiss, and crackling embers."
        },
        "version": "0.2.0",
        "license": "CC0-1.0",
        "tags": ["test", "fire", "night"],
        "loudnessLufs": BED_LUFS,
        "truePeakDbtp": TRUE_PEAK_LIMIT,
        "beds": [{
            "id": "fire",
            "file": "audio/fire.wav",
            "loop": { "startSample": 0, "endSample": frames as u64 },
            "gainDb": 0.0,
            "lfo": { "target": "gain", "depth": 1.0, "periodSec": 31.0 }
        }],
        "events": [{
            "id": "crackle",
            "files": ["audio/crackle-01.wav", "audio/crackle-02.wav"],
            "minIntervalSec": 3.0,
            "maxIntervalSec": 9.0,
            "gainDb": [-9.0, -3.0],
            "pan": [-0.7, 0.7],
            "fadeInMs": 10,
            "fadeOutMs": 120,
            "maxVoices": 2
        }]
    });
    write_manifest(dir, &manifest)
}

fn write_manifest(dir: &Path, manifest: &serde_json::Value) -> Result<()> {
    let text = serde_json::to_string_pretty(manifest)?;
    Manifest::from_json_str(&text).context("generated manifest is invalid")?;
    fs::write(dir.join("manifest.json"), text)?;
    Ok(())
}

fn fade_frames() -> usize {
    (RATE as f32 * FADE_SECONDS) as usize
}

// --- Rain -------------------------------------------------------------------

/// Stereo bed: pink wash, granular droplets, and a quiet low body.
fn rain_bed(seed: u64) -> Vec<f32> {
    let frames = RATE as usize * RAIN_SECONDS;
    let total = frames + fade_frames();

    let mut left = vec![0.0f32; total];
    let mut right = vec![0.0f32; total];

    // Wash: correlated mid plus a quieter decorrelated side, both band-limited
    // so nothing below 120 Hz or above 7 kHz reaches the file.
    let mut rng_mid = ChaCha8Rng::seed_from_u64(seed);
    let mut rng_side = ChaCha8Rng::seed_from_u64(seed ^ 0x51DE);
    let mut pink_mid = PinkNoise::new();
    let mut pink_side = PinkNoise::new();
    let mut hp_mid = Biquad::highpass(RATE as f32, 200.0, 0.707);
    let mut lp_mid = Biquad::lowpass(RATE as f32, 7_000.0, 0.707);
    let mut hp_side = Biquad::highpass(RATE as f32, 220.0, 0.707);
    let mut lp_side = Biquad::lowpass(RATE as f32, 7_000.0, 0.707);

    // Six whole swells across the loop, so the amplitude pattern does not
    // repeat at the wrap.
    let swell_hz = 6.0 / RAIN_SECONDS as f32;
    for frame in 0..total {
        let mid = lp_mid.process(hp_mid.process(pink_mid.process(rng_mid.gen_range(-1.0f32..1.0))));
        let side =
            lp_side.process(hp_side.process(pink_side.process(rng_side.gen_range(-1.0f32..1.0))));
        let swell = 1.0 + 0.10 * (2.0 * PI * swell_hz * frame as f32 / RATE as f32).sin();
        let mid = mid * 0.8 * swell;
        let side = side * 0.55 * swell;
        left[frame] = mid + side;
        right[frame] = mid - side;
    }

    // Droplets: short band-passed bursts, log-normal amplitude, random pan.
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xD40F);
    let mut time = 0.0f32;
    while time < RAIN_SECONDS as f32 {
        time += -((1.0 - rng.gen::<f32>()).ln()) / 95.0; // ~95 droplets/s
        let start = (time * RATE as f32) as usize;
        if start + 2_000 >= total {
            break;
        }
        let amplitude = 0.055 * (-1.4 * rng.gen::<f32>()).exp() * rng.gen_range(0.5f32..1.0);
        let center = rng.gen_range(2_000.0f32..5_000.0);
        let decay = rng.gen_range(0.0015f32..0.006);
        let length = ((decay * 6.0) * RATE as f32) as usize + 32;
        let (gain_l, gain_r) = pan_gains(rng.gen_range(-0.9f32..0.9));
        let mut bandpass = Biquad::bandpass(RATE as f32, center, 1.4);
        let mut burst_rng = ChaCha8Rng::seed_from_u64(rng.gen());
        for offset in 0..length {
            let t = offset as f32 / RATE as f32;
            let env = hit_envelope(t, 0.0004, decay);
            let sample = bandpass.process(burst_rng.gen_range(-1.0f32..1.0)) * env * amplitude;
            left[start + offset] += sample * gain_l;
            right[start + offset] += sample * gain_r;
        }
    }

    // Low body: brown noise under 150 Hz, quiet. Gives weight without the
    // subsonic waste the old 38 Hz one-pole produced.
    let mut rng_body = ChaCha8Rng::seed_from_u64(seed ^ 0xB0D1);
    let mut brown_l = BrownNoise::new();
    let mut brown_r = BrownNoise::new();
    let mut dc_l = DcBlocker::new(RATE as f32, 25.0);
    let mut dc_r = DcBlocker::new(RATE as f32, 25.0);
    let mut lp_l = Biquad::lowpass(RATE as f32, 150.0, 0.707);
    let mut lp_r = Biquad::lowpass(RATE as f32, 150.0, 0.707);
    let body_gain = db_to_gain(-27.0);
    for frame in 0..total {
        let l = lp_l.process(dc_l.process(brown_l.process(rng_body.gen_range(-1.0f32..1.0))));
        let r = lp_r.process(dc_r.process(brown_r.process(rng_body.gen_range(-1.0f32..1.0))));
        left[frame] += l * body_gain;
        right[frame] += r * body_gain;
    }

    finish_bed(vec![left, right])
}

// --- Fire -------------------------------------------------------------------

/// Stereo bed: low roar, thin hiss, and clustered resonant crackles.
fn fire_bed(seed: u64) -> Vec<f32> {
    let frames = RATE as usize * FIRE_SECONDS;
    let total = frames + fade_frames();

    let mut left = vec![0.0f32; total];
    let mut right = vec![0.0f32; total];

    // Roar: pink + brown band-passed to 100-600 Hz, breathing in whole cycles.
    let mut rng_mid = ChaCha8Rng::seed_from_u64(seed);
    let mut rng_side = ChaCha8Rng::seed_from_u64(seed ^ 0x51DE);
    let mut pink_mid = PinkNoise::new();
    let mut pink_side = PinkNoise::new();
    let mut brown_mid = BrownNoise::new();
    let mut brown_side = BrownNoise::new();
    let mut hp_mid = Biquad::highpass(RATE as f32, 100.0, 0.6);
    let mut lp_mid = Biquad::lowpass(RATE as f32, 550.0, 0.7);
    let mut hp_side = Biquad::highpass(RATE as f32, 110.0, 0.6);
    let mut lp_side = Biquad::lowpass(RATE as f32, 570.0, 0.7);
    let cycles_a = 5.0 / FIRE_SECONDS as f32;
    let cycles_b = 12.0 / FIRE_SECONDS as f32;
    for frame in 0..total {
        let time = frame as f32 / RATE as f32;
        let mid_source = 0.6 * pink_mid.process(rng_mid.gen_range(-1.0f32..1.0))
            + 0.4 * brown_mid.process(rng_mid.gen_range(-1.0f32..1.0));
        let side_source = 0.6 * pink_side.process(rng_side.gen_range(-1.0f32..1.0))
            + 0.4 * brown_side.process(rng_side.gen_range(-1.0f32..1.0));
        let mid = lp_mid.process(hp_mid.process(mid_source));
        let side = lp_side.process(hp_side.process(side_source));
        let breathe = 1.0
            + 0.16 * (2.0 * PI * cycles_a * time).sin()
            + 0.10 * (2.0 * PI * cycles_b * time + 1.1).sin();
        let mid = mid * breathe;
        let side = side * 0.5 * breathe;
        left[frame] = mid + side;
        right[frame] = mid - side;
    }

    // Hiss: quiet band above 2.5 kHz, what a fire sounds like up close.
    let mut rng_hiss = ChaCha8Rng::seed_from_u64(seed ^ 0x1155);
    let mut pink_l = PinkNoise::new();
    let mut pink_r = PinkNoise::new();
    let mut hp_l = Biquad::highpass(RATE as f32, 2_500.0, 0.707);
    let mut hp_r = Biquad::highpass(RATE as f32, 2_500.0, 0.707);
    let hiss_gain = db_to_gain(-18.0);
    for frame in 0..total {
        left[frame] += hp_l.process(pink_l.process(rng_hiss.gen_range(-1.0f32..1.0))) * hiss_gain;
        right[frame] += hp_r.process(pink_r.process(rng_hiss.gen_range(-1.0f32..1.0))) * hiss_gain;
    }

    // Crackles: clustered Poisson events, resonant ticks 15-60 ms long.
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xC4AC);
    let mut time = 0.0f32;
    while time < FIRE_SECONDS as f32 {
        let density = if rng.gen_bool(0.3) { 22.0 } else { 6.0 };
        time += -((1.0 - rng.gen::<f32>()).ln()) / density;
        let start = (time * RATE as f32) as usize;
        if start + 4_000 >= total {
            break;
        }
        // A loud snap often pulls two or three quieter ones with it.
        let burst = if rng.gen_bool(0.35) {
            rng.gen_range(2..5)
        } else {
            1
        };
        for index in 0..burst {
            let offset = (index as f32 * rng.gen_range(0.02f32..0.09) * RATE as f32) as usize;
            let event_start = start + offset;
            if event_start + 4_000 >= total {
                continue;
            }
            let base = if index == 0 { 0.45 } else { 0.18 };
            let amplitude = base * (-1.2 * rng.gen::<f32>()).exp();
            let center = rng.gen_range(1_200.0f32..3_500.0);
            let decay = rng.gen_range(0.015f32..0.06);
            let length = ((decay * 7.0) * RATE as f32) as usize + 64;
            let (gain_l, gain_r) = pan_gains(rng.gen_range(-0.8f32..0.8));
            let mut bandpass = Biquad::bandpass(RATE as f32, center, 2.5);
            let mut burst_rng = ChaCha8Rng::seed_from_u64(rng.gen());
            for frame in 0..length {
                let t = frame as f32 / RATE as f32;
                let env = hit_envelope(t, 0.0015, decay);
                let carrier =
                    0.65 * (2.0 * PI * center * t).sin() + 0.35 * burst_rng.gen_range(-1.0f32..1.0);
                let sample = bandpass.process(carrier) * env * amplitude;
                left[event_start + frame] += sample * gain_l;
                right[event_start + frame] += sample * gain_r;
            }
        }
    }

    finish_bed(vec![left, right])
}

/// Crossfade the tail into the head, normalise to the bed target, interleave.
fn finish_bed(mut channels: Vec<Vec<f32>>) -> Vec<f32> {
    let total = channels[0].len();
    let fade = fade_frames().min(total / 4);
    for channel in channels.iter_mut() {
        for frame in 0..fade {
            let t = frame as f32 / fade as f32;
            let head = (t * PI * 0.5).sin();
            let tail = (t * PI * 0.5).cos();
            channel[frame] = channel[frame] * head + channel[total - fade + frame] * tail;
        }
        channel.truncate(total - fade);
    }
    let (loudness, true_peak) = dsp::finalize(&mut channels, BED_LUFS, TRUE_PEAK_LIMIT);
    println!("  bed: {:.1} LUFS, {:.1} dBTP", loudness, true_peak);
    interleave(&channels)
}

// --- Events -----------------------------------------------------------------

/// Mono thunder: a short crack over a rolling low rumble with a second roll.
fn thunder(seed: u64, seconds: f32) -> Vec<f32> {
    let frames = (seconds * RATE as f32) as usize;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut output = vec![0.0f32; frames];

    let mut crack_hp = Biquad::highpass(RATE as f32, 200.0, 0.7);
    let mut crack_lp = Biquad::lowpass(RATE as f32, 1_200.0, 0.7);
    let mut mid_hp = Biquad::highpass(RATE as f32, 200.0, 0.7);
    let mut mid_lp = Biquad::lowpass(RATE as f32, 900.0, 0.7);
    let mut body_hi = Biquad::bandpass(RATE as f32, 380.0, 0.8);
    let mut body_lo = Biquad::bandpass(RATE as f32, 160.0, 0.9);
    let mut rumble_lp = Biquad::lowpass(RATE as f32, 160.0, 0.7);
    let mut rumble_hp = Biquad::highpass(RATE as f32, 55.0, 0.7);
    let mut roll_lp = Biquad::lowpass(RATE as f32, 140.0, 0.7);
    let mut roll_hp = Biquad::highpass(RATE as f32, 28.0, 0.7);
    let mut brown_rumble = BrownNoise::new();
    let mut brown_roll = BrownNoise::new();
    let mut dc_rumble = DcBlocker::new(RATE as f32, 18.0);
    let mut dc_roll = DcBlocker::new(RATE as f32, 18.0);
    let roll_start = 1.6 + rng.gen::<f32>() * 0.8;

    for (frame, sample) in output.iter_mut().enumerate() {
        let time = frame as f32 / RATE as f32;
        let white = rng.gen_range(-1.0f32..1.0);

        // Onset crack: broadband but rolled off, gone in ~60 ms.
        let crack =
            crack_lp.process(crack_hp.process(white)) * hit_envelope(time, 0.004, 0.06) * 0.6;

        // Main rumble with a slow roll.
        let rumble_source =
            rumble_lp.process(rumble_hp.process(dc_rumble.process(brown_rumble.process(white))));
        let rumble = rumble_source
            * hit_envelope(time, 0.03, 3.6)
            * (1.0 + 0.35 * (2.0 * PI * 1.1 * time + 0.4).sin());

        // Mid body so it still reads as thunder on a phone speaker.
        let body = body_hi.process(body_lo.process(white)) * hit_envelope(time, 0.02, 2.2) * 0.5;

        // Second roll.
        let roll_time = time - roll_start;
        let roll = roll_lp.process(roll_hp.process(dc_roll.process(brown_roll.process(white))))
            * hit_envelope(roll_time, 0.05, 1.7)
            * (1.0 + 0.3 * (2.0 * PI * 0.9 * roll_time).sin())
            * 0.6;

        let mid_band = mid_hp.process(mid_lp.process(white)) * hit_envelope(time, 0.03, 2.4) * 0.45;
        *sample = crack + rumble * 1.3 + body * 0.7 + roll + mid_band;
    }

    let mut channels = vec![output];
    dsp::finalize(&mut channels, THUNDER_LUFS, TRUE_PEAK_LIMIT);
    channels.pop().unwrap()
}

/// Mono crackle one-shot: several resonant snaps of varying weight.
fn crackle(seed: u64, seconds: f32) -> Vec<f32> {
    let frames = (seconds * RATE as f32) as usize;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut output = vec![0.0f32; frames];

    let mut time = 0.02f32;
    while time < seconds - 0.08 {
        let start = (time * RATE as f32) as usize;
        let amplitude = 0.5 * (-1.5 * rng.gen::<f32>()).exp() + 0.08;
        let center = rng.gen_range(1_500.0f32..3_500.0);
        let decay = rng.gen_range(0.02f32..0.06);
        let length = ((decay * 7.0) * RATE as f32) as usize + 64;
        let mut bandpass = Biquad::bandpass(RATE as f32, center, 3.0);
        let mut burst_rng = ChaCha8Rng::seed_from_u64(rng.gen());
        for frame in 0..length {
            if start + frame >= frames {
                break;
            }
            let t = frame as f32 / RATE as f32;
            let env = hit_envelope(t, 0.0015, decay);
            let carrier =
                0.7 * (2.0 * PI * center * t).sin() + 0.3 * burst_rng.gen_range(-1.0f32..1.0);
            output[start + frame] += bandpass.process(carrier) * env * amplitude;
        }
        time += rng.gen_range(0.06f32..0.18);
    }

    let mut channels = vec![output];
    dsp::finalize(&mut channels, CRACKLE_LUFS, TRUE_PEAK_LIMIT);
    channels.pop().unwrap()
}

// --- Helpers ----------------------------------------------------------------

fn pan_gains(pan: f32) -> (f32, f32) {
    let theta = (pan.clamp(-1.0, 1.0) + 1.0) * PI * 0.25;
    (theta.cos(), theta.sin())
}

fn interleave(channels: &[Vec<f32>]) -> Vec<f32> {
    let frames = channels[0].len();
    let mut out = Vec::with_capacity(frames * channels.len());
    for frame in 0..frames {
        for channel in channels {
            out.push(channel[frame]);
        }
    }
    out
}

fn write_wav(path: &Path, channels: u16, samples: &[f32]) -> Result<()> {
    let spec = WavSpec {
        channels,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .with_context(|| format!("failed to create {}", path.display()))?;
    for sample in samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * 32767.0).round() as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::testkit;

    fn read_wav(path: &Path) -> (Vec<Vec<f32>>, u32) {
        let mut reader = hound::WavReader::open(path).expect("open generated wav");
        let spec = reader.spec();
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.expect("sample") as f32 / 32768.0)
            .collect();
        let channels = spec.channels as usize;
        let mut split = vec![Vec::with_capacity(samples.len() / channels); channels];
        for (index, sample) in samples.iter().enumerate() {
            split[index % channels].push(*sample);
        }
        (split, spec.sample_rate)
    }

    fn mono(channels: &[Vec<f32>]) -> Vec<f32> {
        if channels.len() == 1 {
            return channels[0].clone();
        }
        channels[0]
            .iter()
            .zip(&channels[1])
            .map(|(l, r)| 0.5 * (l + r))
            .collect()
    }

    fn correlation(stereo: &[Vec<f32>]) -> f32 {
        let (l, r) = (&stereo[0], &stereo[1]);
        let n = l.len() as f32;
        let mean_l = l.iter().sum::<f32>() / n;
        let mean_r = r.iter().sum::<f32>() / n;
        let (mut cov, mut var_l, mut var_r) = (0.0f32, 0.0f32, 0.0f32);
        for (a, b) in l.iter().zip(r) {
            let da = a - mean_l;
            let dbb = b - mean_r;
            cov += da * dbb;
            var_l += da * da;
            var_r += dbb * dbb;
        }
        cov / (var_l.sqrt() * var_r.sqrt())
    }

    fn assert_clean(channels: &[Vec<f32>], label: &str) {
        let peak = channels
            .iter()
            .flat_map(|c| c.iter())
            .fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(
            peak < 0.999,
            "{label} has samples at full scale (peak {peak})"
        );
        let true_peak = dsp::true_peak_dbtp(channels);
        assert!(
            true_peak <= TRUE_PEAK_LIMIT + 0.05,
            "{label} true peak {true_peak}"
        );
        for channel in channels {
            let dc: f32 = channel.iter().sum::<f32>() / channel.len() as f32;
            assert!(dc.abs() < 0.001, "{label} DC offset {dc}");
        }
        if channels.len() == 2 {
            let seam = (channels[0][0] - channels[0][channels[0].len() - 1]).abs();
            assert!(seam < 0.05, "{label} loop seam {seam}");
        }
    }

    /// The pack-quality targets from docs/PACK_QUALITY_REVIEW.md, in one place.
    #[test]
    fn procedural_packs_meet_quality_targets() {
        let dir = tempfile::tempdir().unwrap();
        generate_test_packs(dir.path()).unwrap();

        let (rain, rate) = read_wav(&dir.path().join("test-rain/audio/rain.wav"));
        assert_eq!(rate, RATE);
        assert_eq!(rain.len(), 2, "rain bed is stereo");
        let (centroid, shares) = testkit::analyze(&mono(&rain), RATE as f32);
        let phone = testkit::phone_band_share(&shares);
        assert!(
            (1_800.0..=3_200.0).contains(&centroid),
            "rain centroid {centroid}"
        );
        assert!(shares[0] < 2.0, "rain subsonic {}%", shares[0]);
        assert!((40.0..=75.0).contains(&phone), "rain phone band {phone}%");
        assert!(shares[7] < 8.0, "rain >8 kHz {}%", shares[7]);
        let rain_corr = correlation(&rain);
        assert!(
            (0.2..0.6).contains(&rain_corr),
            "rain stereo correlation {rain_corr}"
        );
        assert_clean(&rain, "rain");

        let (fire, _) = read_wav(&dir.path().join("test-fire/audio/fire.wav"));
        assert_eq!(fire.len(), 2, "fire bed is stereo");
        let (centroid, shares) = testkit::analyze(&mono(&fire), RATE as f32);
        let phone = testkit::phone_band_share(&shares);
        assert!(
            (900.0..=2_000.0).contains(&centroid),
            "fire centroid {centroid}"
        );
        assert!(shares[0] < 3.0, "fire subsonic {}%", shares[0]);
        assert!((30.0..=60.0).contains(&phone), "fire phone band {phone}%");
        assert!(shares[7] < 5.0, "fire >8 kHz {}%", shares[7]);
        assert_clean(&fire, "fire");

        for name in ["thunder-01.wav", "thunder-02.wav"] {
            let (channels, _) = read_wav(&dir.path().join("test-rain/audio").join(name));
            assert_eq!(channels.len(), 1, "{name} is mono");
            let (centroid, shares) = testkit::analyze(&channels[0], RATE as f32);
            let phone = testkit::phone_band_share(&shares);
            assert!(
                (200.0..=550.0).contains(&centroid),
                "{name} centroid {centroid}"
            );
            assert!(shares[0] < 5.0, "{name} subsonic {}%", shares[0]);
            assert!((25.0..=60.0).contains(&phone), "{name} phone band {phone}%");
            assert!(shares[7] < 3.0, "{name} >8 kHz {}%", shares[7]);
            assert_clean(&channels, name);
        }

        for name in ["crackle-01.wav", "crackle-02.wav"] {
            let (channels, _) = read_wav(&dir.path().join("test-fire/audio").join(name));
            let (centroid, shares) = testkit::analyze(&channels[0], RATE as f32);
            let phone = testkit::phone_band_share(&shares);
            assert!(
                (1_500.0..=4_000.0).contains(&centroid),
                "{name} centroid {centroid}"
            );
            assert!(phone >= 50.0, "{name} phone band {phone}%");
            assert_clean(&channels, name);
        }
    }
}
