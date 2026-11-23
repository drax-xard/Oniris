//! Deterministic DSP building blocks for procedural content generation.
//!
//! Everything here is offline: allocations are fine, speed matters only enough
//! that regenerating the packs stays under a few seconds in a test build.

use std::f32::consts::PI;

pub const RATE: u32 = 48_000;

/// Direct-Form-II-transposed biquad.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn from_coeffs(b: [f32; 3], a: [f32; 3]) -> Self {
        let a0 = a[0];
        Self {
            b0: b[0] / a0,
            b1: b[1] / a0,
            b2: b[2] / a0,
            a1: a[1] / a0,
            a2: a[2] / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// RBJ cookbook low-pass.
    pub fn lowpass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let (w0, cos_w0, alpha) = rbj_terms(sample_rate, freq, q);
        let b1 = 1.0 - cos_w0;
        Self::from_coeffs(
            [(1.0 - cos_w0) / 2.0, b1, (1.0 - cos_w0) / 2.0],
            [1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha],
        )
        .with_terms(w0)
    }

    /// RBJ cookbook high-pass.
    pub fn highpass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let (_, cos_w0, alpha) = rbj_terms(sample_rate, freq, q);
        Self::from_coeffs(
            [(1.0 + cos_w0) / 2.0, -(1.0 + cos_w0), (1.0 + cos_w0) / 2.0],
            [1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha],
        )
    }

    /// RBJ cookbook constant-0 dB-peak band-pass.
    pub fn bandpass(sample_rate: f32, freq: f32, q: f32) -> Self {
        let (_, cos_w0, alpha) = rbj_terms(sample_rate, freq, q);
        Self::from_coeffs(
            [alpha, 0.0, -alpha],
            [1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha],
        )
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    /// Only used to keep the RBJ helpers symmetrical; coefficients are already
    /// normalised by `from_coeffs`.
    fn with_terms(self, _w0: f32) -> Self {
        self
    }
}

fn rbj_terms(sample_rate: f32, freq: f32, q: f32) -> (f32, f32, f32) {
    let freq = freq.clamp(10.0, sample_rate * 0.45);
    let w0 = 2.0 * PI * freq / sample_rate;
    (w0, w0.cos(), w0.sin() / (2.0 * q.max(0.05)))
}

/// Removes DC and very low frequency drift from an integrator-based source.
#[derive(Clone, Copy, Debug)]
pub struct DcBlocker {
    previous_input: f32,
    previous_output: f32,
    pole: f32,
}

impl DcBlocker {
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let pole = (-2.0 * PI * cutoff_hz / sample_rate).exp();
        Self {
            previous_input: 0.0,
            previous_output: 0.0,
            pole,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.previous_input + self.pole * self.previous_output;
        self.previous_input = x;
        self.previous_output = y;
        y
    }
}

/// Paul Kellet's refined pink-noise filter: -3 dB/octave, roughly unity RMS.
#[derive(Clone, Copy, Debug, Default)]
pub struct PinkNoise {
    b: [f32; 7],
}

impl PinkNoise {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn process(&mut self, white: f32) -> f32 {
        self.b[0] = 0.99886 * self.b[0] + white * 0.0555179;
        self.b[1] = 0.99332 * self.b[1] + white * 0.0750759;
        self.b[2] = 0.96900 * self.b[2] + white * 0.153_852;
        self.b[3] = 0.86650 * self.b[3] + white * 0.310_485_6;
        self.b[4] = 0.55000 * self.b[4] + white * 0.532_952_2;
        self.b[5] = -0.7616 * self.b[5] - white * 0.0168980;
        let pink = self.b[0]
            + self.b[1]
            + self.b[2]
            + self.b[3]
            + self.b[4]
            + self.b[5]
            + self.b[6]
            + white * 0.5362;
        self.b[6] = white * 0.115_926;
        pink * 0.11
    }
}

/// -6 dB/octave source, with the DC tail removed by the caller.
#[derive(Clone, Copy, Debug, Default)]
pub struct BrownNoise {
    value: f32,
}

impl BrownNoise {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn process(&mut self, white: f32) -> f32 {
        self.value = (self.value + 0.02 * white) / 1.02;
        self.value
    }
}

/// Linear attack into an exponential decay, evaluated at `time` seconds.
pub fn hit_envelope(time: f32, attack: f32, decay: f32) -> f32 {
    if time < 0.0 {
        return 0.0;
    }
    let rise = if attack > 0.0 {
        (time / attack).min(1.0)
    } else {
        1.0
    };
    rise * (-time / decay.max(1e-5)).exp()
}

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub fn gain_to_db(gain: f32) -> f32 {
    20.0 * gain.max(1e-9).log10()
}

// --- Metering ---------------------------------------------------------------

/// BS.1770 K-weighting at 48 kHz (shelf then RLB high-pass).
fn k_weight(channels: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let shelf = (
        [1.535_124_9, -2.691_696_2, 1.198_392_8],
        [1.0, -1.690_659_3, 0.732_480_8],
    );
    let highpass = ([1.0, -2.0, 1.0], [1.0, -1.990_047_5, 0.990_072_2]);
    channels
        .iter()
        .map(|channel| {
            let mut first = Biquad::from_coeffs(shelf.0, shelf.1);
            let mut second = Biquad::from_coeffs(highpass.0, highpass.1);
            channel
                .iter()
                .map(|&sample| second.process(first.process(sample)))
                .collect()
        })
        .collect()
}

fn block_loudness(weighted: &[Vec<f32>], start: usize, len: usize) -> f32 {
    let mut sum = 0.0f64;
    for channel in weighted {
        let mut acc = 0.0f64;
        for &sample in &channel[start..start + len] {
            acc += (sample as f64) * (sample as f64);
        }
        sum += acc / len as f64;
    }
    if sum <= 0.0 {
        f32::NEG_INFINITY
    } else {
        -0.691 + 10.0 * (sum.log10() as f32)
    }
}

fn block_mean_square(weighted: &[Vec<f32>], start: usize, len: usize) -> f64 {
    let mut sum = 0.0f64;
    for channel in weighted {
        let mut acc = 0.0f64;
        for &sample in &channel[start..start + len] {
            acc += (sample as f64) * (sample as f64);
        }
        sum += acc / len as f64;
    }
    sum
}

/// Gated integrated loudness (BS.1770-4): 400 ms blocks, 100 ms hop, absolute
/// -70 LUFS gate then a -10 LU relative gate.
pub fn integrated_lufs(channels: &[Vec<f32>], sample_rate: u32) -> f32 {
    assert_eq!(
        sample_rate, RATE,
        "K-weighting coefficients are 48 kHz only"
    );
    let weighted = k_weight(channels);
    let frames = weighted[0].len();
    let block = (RATE as usize * 4) / 10;
    let hop = block / 4;
    if frames < block {
        return block_loudness(&weighted, 0, frames.max(1));
    }

    let mut blocks: Vec<(f32, f64)> = Vec::new();
    let mut start = 0;
    while start + block <= frames {
        let loudness = block_loudness(&weighted, start, block);
        blocks.push((loudness, block_mean_square(&weighted, start, block)));
        start += hop;
    }

    let absolute: Vec<&(f32, f64)> = blocks.iter().filter(|(l, _)| *l > -70.0).collect();
    if absolute.is_empty() {
        return f32::NEG_INFINITY;
    }
    let loudness_of = |mean: f64| -> f32 {
        if mean <= 0.0 {
            f32::NEG_INFINITY
        } else {
            -0.691 + 10.0 * (mean.log10() as f32)
        }
    };
    let mean_of = |set: &[&(f32, f64)]| -> f64 {
        set.iter().map(|(_, ms)| *ms).sum::<f64>() / set.len() as f64
    };
    let relative_threshold = loudness_of(mean_of(&absolute)) - 10.0;
    let gated: Vec<&(f32, f64)> = absolute
        .into_iter()
        .filter(|(loudness, _)| *loudness > relative_threshold)
        .collect();
    if gated.is_empty() {
        f32::NEG_INFINITY
    } else {
        loudness_of(mean_of(&gated))
    }
}

/// True peak with 4x oversampling, evaluated only around likely peaks.
pub fn true_peak_dbtp(channels: &[Vec<f32>]) -> f32 {
    let mut peak = 0.0f32;
    for channel in channels {
        let sample_peak = channel.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        if sample_peak <= 0.0 {
            continue;
        }
        let candidate_threshold = sample_peak * 0.5;
        for index in 4..channel.len().saturating_sub(4) {
            let value = channel[index];
            if value.abs() < candidate_threshold {
                continue;
            }
            peak = peak.max(value.abs());
            for phase in 1..4 {
                let fraction = phase as f32 / 4.0;
                let mut interpolated = 0.0f32;
                for tap in -4i32..=3 {
                    let distance = fraction - tap as f32;
                    let window = 0.5 + 0.5 * (PI * distance / 4.0).cos();
                    interpolated +=
                        channel[(index as i32 + tap) as usize] * sinc(distance) * window;
                }
                peak = peak.max(interpolated.abs());
            }
        }
    }
    gain_to_db(peak)
}

fn sinc(x: f32) -> f32 {
    if x.abs() < 1e-6 {
        1.0
    } else {
        let pi_x = PI * x;
        pi_x.sin() / pi_x
    }
}

fn scale(channels: &mut [Vec<f32>], gain: f32) {
    for channel in channels.iter_mut() {
        for sample in channel.iter_mut() {
            *sample *= gain;
        }
    }
}

/// Scale to `target_lufs`, then pull back if that breaks `tp_limit`.
///
/// Returns the measured (loudness, true peak) after normalisation. A high-crest
/// asset can end up quieter than requested; that is the honest trade rather than
/// shaving the transients with a limiter.
pub fn finalize(channels: &mut [Vec<f32>], target_lufs: f32, tp_limit: f32) -> (f32, f32) {
    let loudness = integrated_lufs(channels, RATE);
    if loudness.is_finite() {
        scale(channels, db_to_gain(target_lufs - loudness));
    }
    let true_peak = true_peak_dbtp(channels);
    if true_peak > tp_limit {
        scale(channels, db_to_gain(tp_limit - true_peak));
    }
    (integrated_lufs(channels, RATE), true_peak_dbtp(channels))
}

#[cfg(test)]
pub mod testkit {
    //! Spectral analysis used by the generator's character tests. Same
    //! methodology as tools/audio-qa/measure.py so the numbers are comparable.

    pub const BANDS: [(f32, f32); 8] = [
        (0.0, 20.0),
        (20.0, 60.0),
        (60.0, 120.0),
        (120.0, 300.0),
        (300.0, 1000.0),
        (1000.0, 3000.0),
        (3000.0, 8000.0),
        (8000.0, 20000.0),
    ];

    struct Fft {
        len: usize,
        cos: Vec<f32>,
        sin: Vec<f32>,
    }

    impl Fft {
        fn new(len: usize) -> Self {
            let cos = (0..len)
                .map(|i| (2.0 * std::f32::consts::PI * i as f32 / len as f32).cos())
                .collect();
            let sin = (0..len)
                .map(|i| (2.0 * std::f32::consts::PI * i as f32 / len as f32).sin())
                .collect();
            Self { len, cos, sin }
        }

        fn transform(&self, real: &mut [f32], imag: &mut [f32]) {
            let n = self.len;
            let mut j = 0usize;
            for i in 1..n {
                let mut bit = n >> 1;
                while j & bit != 0 {
                    j ^= bit;
                    bit >>= 1;
                }
                j |= bit;
                if i < j {
                    real.swap(i, j);
                    imag.swap(i, j);
                }
            }
            let mut size = 2;
            while size <= n {
                let step = n / size;
                for start in (0..n).step_by(size) {
                    for k in 0..size / 2 {
                        let angle = k * step;
                        let wr = self.cos[angle];
                        let wi = -self.sin[angle];
                        let a = start + k;
                        let b = a + size / 2;
                        let tr = real[b] * wr - imag[b] * wi;
                        let ti = real[b] * wi + imag[b] * wr;
                        real[b] = real[a] - tr;
                        imag[b] = imag[a] - ti;
                        real[a] += tr;
                        imag[a] += ti;
                    }
                }
                size <<= 1;
            }
        }
    }

    /// Energy-weighted spectral centroid plus band shares, in percent.
    pub fn analyze(mono: &[f32], sample_rate: f32) -> (f32, [f32; 8]) {
        let window_len = 8192usize;
        let hop = window_len / 2;
        let fft = Fft::new(window_len);
        let window: Vec<f32> = (0..window_len)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / window_len as f32).cos())
            .collect();
        let bins = window_len / 2 + 1;
        let mut psd = vec![0.0f32; bins];
        let mut frames = 0usize;

        let mut start = 0;
        while start + window_len <= mono.len() {
            let mut real: Vec<f32> = (0..window_len)
                .map(|i| mono[start + i] * window[i])
                .collect();
            let mut imag = vec![0.0f32; window_len];
            fft.transform(&mut real, &mut imag);
            for k in 0..bins {
                let magnitude = (real[k] * real[k] + imag[k] * imag[k]).sqrt();
                psd[k] += magnitude * magnitude;
            }
            frames += 1;
            start += hop;
        }
        if frames == 0 {
            return (0.0, [0.0; 8]);
        }
        let total: f32 = psd.iter().sum();
        let mut shares = [0.0f32; 8];
        for (band, (low, high)) in BANDS.iter().enumerate() {
            let mut energy = 0.0f32;
            for (k, value) in psd.iter().enumerate() {
                let freq = k as f32 * sample_rate / window_len as f32;
                if freq >= *low && freq < *high {
                    energy += *value;
                }
            }
            shares[band] = if total > 0.0 {
                100.0 * energy / total
            } else {
                0.0
            };
        }
        // Energy-weighted centroid of the accumulated spectrum: unlike a mean of
        // per-frame centroids it is not skewed by a handful of loud transients.
        let mut weighted = 0.0f64;
        for (k, value) in psd.iter().enumerate() {
            let freq = k as f32 * sample_rate / window_len as f32;
            weighted += (*value as f64) * (freq as f64);
        }
        let centroid = if total > 0.0 {
            (weighted / total as f64) as f32
        } else {
            0.0
        };
        (centroid, shares)
    }

    /// 300 Hz - 4 kHz share, the band a phone speaker can reproduce.
    pub fn phone_band_share(shares: &[f32; 8]) -> f32 {
        shares[4] + shares[5]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn noise(len: usize, seed: u64) -> Vec<f32> {
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        (0..len).map(|_| rng.gen_range(-1.0f32..1.0)).collect()
    }

    #[test]
    fn lowpass_rejects_high_frequencies() {
        let mut filter = Biquad::lowpass(RATE as f32, 500.0, 0.707);
        let high = (0..RATE)
            .map(|i| (2.0 * PI * 8000.0 * i as f32 / RATE as f32).sin())
            .collect::<Vec<_>>();
        let out: Vec<f32> = high.iter().map(|&s| filter.process(s)).collect();
        let rms = |x: &[f32]| (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt();
        assert!(rms(&out[4800..]) < 0.02, "rms={}", rms(&out[4800..]));
    }

    #[test]
    fn pink_noise_is_not_white() {
        let mut rng = rand::thread_rng();
        let mut pink = PinkNoise::new();
        let signal: Vec<f32> = (0..RATE as usize)
            .map(|_| pink.process(rng.gen_range(-1.0f32..1.0)))
            .collect();
        let (centroid, shares) = testkit::analyze(&signal, RATE as f32);
        // Pink noise is far darker than white: white sits around 10-12 kHz here.
        // Its power piles up at low frequencies, so the 300 Hz-4 kHz share stays
        // modest even though the midrange is perfectly present.
        assert!(centroid < 5_000.0, "centroid={centroid}");
        assert!(testkit::phone_band_share(&shares) > 10.0);
    }

    #[test]
    fn k_weighted_loudness_matches_a_known_sine() {
        // -20 dBFS 1 kHz sine in both channels measures about -20 LUFS.
        let amplitude = 0.1f32;
        let sine: Vec<f32> = (0..RATE as usize * 3)
            .map(|i| amplitude * (2.0 * PI * 1000.0 * i as f32 / RATE as f32).sin())
            .collect();
        let lufs = integrated_lufs(&[sine.clone(), sine], RATE);
        assert!((lufs + 20.0).abs() < 1.0, "lufs={lufs}");
    }

    #[test]
    fn true_peak_sees_inter_sample_overs() {
        // Nyquist-rate square wave: sample peaks are 0.5, but the reconstructed
        // waveform overshoots by ~1.5 dB, which true-peak metering must catch.
        let len = 4800;
        let square: Vec<f32> = (0..len)
            .map(|i| if (i / 2) % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let sample_peak = gain_to_db(0.5);
        let true_peak = true_peak_dbtp(&[square]);
        assert!(
            true_peak > sample_peak + 0.5,
            "sample={sample_peak} true={true_peak}"
        );
    }

    #[test]
    fn finalize_respects_both_limits() {
        let mut channels = vec![noise(RATE as usize * 2, 7), noise(RATE as usize * 2, 8)];
        let (lufs, true_peak) = finalize(&mut channels, -23.0, -1.5);
        assert!((lufs + 23.0).abs() < 0.5, "lufs={lufs}");
        assert!(true_peak <= -1.5, "tp={true_peak}");
    }
}
