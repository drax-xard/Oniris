//! Golden audio regression tests for the mixer.
//!
//! These render a deterministic synthetic pack and lock in level, brightness,
//! continuity, and seed behaviour. If a DSP change moves these values
//! intentionally, update the golden numbers here.

use std::f64::consts::TAU;
use std::path::Path;
use std::time::Duration;

use hound::{SampleFormat, WavSpec, WavWriter};
use oniris_core::{Engine, MixEntry, MixSpec, DEFAULT_BLOCK_FRAMES};

const RATE: u32 = 48_000;
const SECONDS: f32 = 10.0;
const SEED: u64 = 7;
const SKIP_FRAMES: usize = RATE as usize / 2;

fn write_wav(path: &Path, channels: u16, samples: &[i16]) {
    let spec = WavSpec {
        channels,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
}

fn sine(frames: usize, channels: usize, frequency: f32, amplitude: f32) -> Vec<i16> {
    (0..frames)
        .flat_map(|frame| {
            let value = (frame as f32 * std::f32::consts::TAU * frequency / RATE as f32).sin()
                * amplitude
                * i16::MAX as f32;
            let value = value.round() as i16;
            std::iter::repeat_n(value, channels)
        })
        .collect()
}

fn make_pack(dir: &Path) {
    std::fs::create_dir_all(dir.join("audio")).unwrap();
    // One second is exactly an integer number of cycles at 220 and 330 Hz, so
    // the loop seam is continuous when the engine wraps.
    write_wav(
        &dir.join("audio/a.wav"),
        2,
        &sine(RATE as usize, 2, 220.0, 0.1),
    );
    write_wav(
        &dir.join("audio/b.wav"),
        2,
        &sine(RATE as usize, 2, 330.0, 0.05),
    );
    write_wav(
        &dir.join("audio/chime.wav"),
        1,
        &sine(RATE as usize, 1, 880.0, 0.5),
    );

    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "golden",
        "name": { "en": "Golden Mix" },
        "version": "0.1.0",
        "license": "CC0-1.0",
        "beds": [
            {
                "id": "a",
                "file": "audio/a.wav",
                "loop": { "startSample": 0, "endSample": RATE },
                "gainDb": -3.0
            },
            {
                "id": "b",
                "file": "audio/b.wav",
                "loop": { "startSample": 0, "endSample": RATE },
                "gainDb": -6.0,
                "pan": 0.2,
                "lfo": { "target": "gain", "depth": 1.0, "periodSec": 5.0 }
            }
        ],
        "events": [{
            "id": "chime",
            "files": ["audio/chime.wav"],
            "minIntervalSec": 1.5,
            "maxIntervalSec": 3.0,
            "initialDelaySec": 0.5,
            "gainDb": [-12.0, -6.0],
            "fadeInMs": 100,
            "fadeOutMs": 200,
            "maxVoices": 1
        }]
    });
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

fn render(dir: &Path, seed: u64, seconds: f32) -> Vec<i16> {
    let mut engine = Engine::with_seed(RATE, seed);
    engine.set_fade_in(Duration::from_millis(50));
    let id = engine.load_pack(dir).unwrap();
    engine
        .set_mix(&MixSpec::new(vec![MixEntry {
            id,
            gain_db: 0.0,
            muted: false,
        }]))
        .unwrap();
    engine.play();

    let total = (seconds * RATE as f32).round() as usize;
    let mut output = Vec::with_capacity(total * 2);
    let mut block = vec![0i16; DEFAULT_BLOCK_FRAMES * 2];
    while output.len() / 2 < total {
        let frames = (total - output.len() / 2).min(DEFAULT_BLOCK_FRAMES);
        engine.render_i16(&mut block, frames);
        output.extend_from_slice(&block[..frames * 2]);
    }
    output
}

fn rms(samples: &[i16]) -> f32 {
    let sum: f64 = samples
        .iter()
        .map(|sample| (*sample as f64) * (*sample as f64))
        .sum();
    (sum / samples.len() as f64).sqrt() as f32 / i16::MAX as f32
}

fn peak(samples: &[i16]) -> f32 {
    samples
        .iter()
        .map(|sample| (*sample as i32).unsigned_abs())
        .max()
        .unwrap_or(0) as f32
        / i16::MAX as f32
}

fn max_delta(samples: &[i16], channels: usize, skip_frames: usize) -> i32 {
    let frames = samples.len() / channels;
    let mut maximum = 0;
    for frame in (skip_frames + 1)..frames {
        for channel in 0..channels {
            let previous = samples[(frame - 1) * channels + channel] as i32;
            let current = samples[frame * channels + channel] as i32;
            maximum = maximum.max((current - previous).abs());
        }
    }
    maximum
}

fn goertzel_power(samples: &[f64], coefficient: f64) -> f64 {
    let mut first = 0.0;
    let mut second = 0.0;
    for sample in samples {
        let value = sample + coefficient * first - second;
        second = first;
        first = value;
    }
    first * first + second * second - coefficient * first * second
}

/// Approximate spectral centroid from 64 log-spaced Goertzel bands.
fn spectral_centroid(samples: &[i16], channels: usize) -> f32 {
    let frames = samples.len() / channels;
    let length = 4096.min(frames);
    let start = (frames - length) / 2;

    let mut window = Vec::with_capacity(length);
    for frame in 0..length {
        let left = samples[(start + frame) * channels] as f64;
        let right = if channels > 1 {
            samples[(start + frame) * channels + 1] as f64
        } else {
            left
        };
        let mono = 0.5 * (left + right) / i16::MAX as f64;
        let hann = 0.5 - 0.5 * (TAU * frame as f64 / (length - 1) as f64).cos();
        window.push(mono * hann);
    }

    const BANDS: usize = 64;
    let minimum: f64 = 40.0;
    let maximum: f64 = 12_000.0;
    let mut weighted = 0.0;
    let mut total = 0.0;
    for band in 0..BANDS {
        let ratio = band as f64 / (BANDS - 1) as f64;
        let frequency = minimum * (maximum / minimum).powf(ratio);
        let coefficient = 2.0 * (TAU * frequency / RATE as f64).cos();
        let power = goertzel_power(&window, coefficient).max(0.0);
        weighted += frequency * power;
        total += power;
    }
    if total > 0.0 {
        (weighted / total) as f32
    } else {
        0.0
    }
}

#[test]
fn golden_mix_is_stable_and_click_free() {
    let dir = tempfile::tempdir().unwrap();
    make_pack(dir.path());
    let output = render(dir.path(), SEED, SECONDS);

    assert_eq!(output.len(), (SECONDS * RATE as f32) as usize * 2);

    let rms = rms(&output);
    let peak = peak(&output);
    let centroid = spectral_centroid(&output, 2);
    let delta = max_delta(&output, 2, SKIP_FRAMES);

    // Golden values measured from this deterministic mix. Keep tolerances tight
    // enough to catch level or filter regressions, loose enough for FP noise.
    assert!(
        (rms - 0.0945).abs() < 0.005,
        "rms {rms} (centroid {centroid}, peak {peak})"
    );
    assert!((peak - 0.335).abs() < 0.02, "peak {peak} (rms {rms})");
    assert!(
        (centroid - 226.0).abs() < 30.0,
        "centroid {centroid} (rms {rms})"
    );
    assert!(delta < 4_000, "max delta {delta} (peak {peak})");
}

fn make_loop_pack(dir: &Path) {
    std::fs::create_dir_all(dir.join("audio")).unwrap();
    write_wav(
        &dir.join("audio/a.wav"),
        2,
        &sine(RATE as usize, 2, 220.0, 0.1),
    );
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "id": "loop",
        "name": { "en": "Loop Test" },
        "version": "0.1.0",
        "license": "CC0-1.0",
        "beds": [{
            "id": "a",
            "file": "audio/a.wav",
            "loop": { "startSample": 0, "endSample": RATE },
            "gainDb": -3.0
        }]
    });
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn periodic_beds_loop_without_a_seam() {
    let dir = tempfile::tempdir().unwrap();
    make_loop_pack(dir.path());
    let output = render(dir.path(), SEED, 5.0);

    // A 220 Hz bed at -3 dB has a natural sample-to-sample step of about 95 in
    // i16 units; a loop click would be thousands.
    let delta = max_delta(&output, 2, SKIP_FRAMES);
    assert!(delta < 300, "max delta {delta}");
}

#[test]
fn golden_render_is_deterministic_for_a_seed() {
    let dir = tempfile::tempdir().unwrap();
    make_pack(dir.path());

    let first = render(dir.path(), SEED, 3.0);
    let second = render(dir.path(), SEED, 3.0);
    assert_eq!(first, second);

    let other = render(dir.path(), SEED + 1, 3.0);
    assert_ne!(first, other);
}
