//! EBU R128 / ITU-R BS.1770 loudness and true-peak measurement.
//!
//! The K-weighting coefficients and 400 ms gating blocks are defined for
//! 48 kHz. Callers should decode at 48 kHz (`decode_audio(path, 48_000)`) so no
//! resampling happens here.
//!
//! This is a content-pipeline tool: it runs offline on full files, not on the
//! audio thread.

/// BS.1770 block length: 400 ms.
const BLOCK_SECONDS: f64 = 0.400;
/// 75 % overlap between blocks.
const BLOCK_OVERLAP: f64 = 4.0;
/// EBU R128 absolute gate.
const ABSOLUTE_GATE_LUFS: f64 = -70.0;
/// Relative gate below the ungated loudness.
const RELATIVE_GATE_LU: f64 = -10.0;
/// Calibration offset from BS.1770.
const LOUDNESS_OFFSET: f64 = -0.691;
const SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    /// Integrated loudness in LUFS. `NEG_INFINITY` for silence.
    pub integrated_lufs: f32,
    /// Oversampled true peak in dBTP. `NEG_INFINITY` for silence.
    pub true_peak_dbtp: f32,
    /// Highest sample value in dBFS. `NEG_INFINITY` for silence.
    pub sample_peak_dbfs: f32,
    /// Number of 400 ms blocks that passed the gates.
    pub gated_blocks: usize,
}

/// Measure a 48 kHz mono or stereo signal.
pub fn measure(samples: &[i16], channels: u16, sample_rate: u32) -> Measurement {
    assert!(
        matches!(channels, 1 | 2),
        "loudness measurement supports mono or stereo, not {channels} channels"
    );
    assert_eq!(
        sample_rate, SAMPLE_RATE,
        "loudness measurement expects 48 kHz input"
    );

    let channels = channels as usize;
    let frames = samples.len() / channels;
    if frames == 0 {
        return silence();
    }

    let mut normalized = vec![0.0f64; samples.len()];
    let mut input_peak = 0.0f64;
    for (output, sample) in normalized.iter_mut().zip(samples) {
        let value = *sample as f64 / 32_768.0;
        input_peak = input_peak.max(value.abs());
        *output = value;
    }

    let filtered = k_weight(&normalized, channels);
    let powers = block_powers(&filtered, channels, frames);
    let integrated_lufs = integrated_loudness(&powers).unwrap_or(f64::NEG_INFINITY);
    let true_peak = true_peak(&normalized, channels, frames);
    let gated_blocks = gated_block_count(&powers);

    Measurement {
        integrated_lufs: integrated_lufs as f32,
        true_peak_dbtp: linear_to_db(true_peak) as f32,
        sample_peak_dbfs: linear_to_db(input_peak) as f32,
        gated_blocks,
    }
}

/// Apply a gain in dB, clamping to the i16 range.
pub fn apply_gain(samples: &[i16], gain_db: f32) -> Vec<i16> {
    let factor = 10f32.powf(gain_db / 20.0);
    samples
        .iter()
        .map(|sample| (*sample as f32 * factor).round().clamp(-32_768.0, 32_767.0) as i16)
        .collect()
}

fn silence() -> Measurement {
    Measurement {
        integrated_lufs: f32::NEG_INFINITY,
        true_peak_dbtp: f32::NEG_INFINITY,
        sample_peak_dbfs: f32::NEG_INFINITY,
        gated_blocks: 0,
    }
}

fn k_weight(samples: &[f64], channels: usize) -> Vec<f64> {
    let frames = samples.len() / channels;
    let mut output = vec![0.0f64; samples.len()];
    for channel in 0..channels {
        let mut shelf = Biquad::shelf();
        let mut high_pass = Biquad::high_pass();
        for frame in 0..frames {
            let index = frame * channels + channel;
            output[index] = high_pass.process(shelf.process(samples[index]));
        }
    }
    output
}

fn block_powers(samples: &[f64], channels: usize, frames: usize) -> Vec<f64> {
    let block = (BLOCK_SECONDS * SAMPLE_RATE as f64) as usize;
    let step = (block as f64 / BLOCK_OVERLAP) as usize;
    let mut powers = Vec::new();

    if frames < block {
        // Short signals still get one block, zero-padded to 400 ms.
        let mut sum = 0.0;
        for sample in &samples[..frames * channels] {
            sum += sample * sample;
        }
        powers.push(sum / block as f64);
        return powers;
    }

    let mut start = 0;
    while start + block <= frames {
        let mut sum = 0.0;
        for sample in &samples[start * channels..(start + block) * channels] {
            sum += sample * sample;
        }
        powers.push(sum / block as f64);
        start += step;
    }
    powers
}

fn integrated_loudness(powers: &[f64]) -> Option<f64> {
    let absolute: Vec<f64> = powers
        .iter()
        .copied()
        .filter(|power| loudness(*power) > ABSOLUTE_GATE_LUFS)
        .collect();
    if absolute.is_empty() {
        return None;
    }

    let absolute_mean = absolute.iter().sum::<f64>() / absolute.len() as f64;
    let relative_gate = loudness(absolute_mean) + RELATIVE_GATE_LU;
    let mut sum = 0.0;
    let mut count = 0usize;
    for power in absolute {
        if loudness(power) > relative_gate {
            sum += power;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some(loudness(sum / count as f64))
}

fn gated_block_count(powers: &[f64]) -> usize {
    let absolute: Vec<f64> = powers
        .iter()
        .copied()
        .filter(|power| loudness(*power) > ABSOLUTE_GATE_LUFS)
        .collect();
    if absolute.is_empty() {
        return 0;
    }
    let absolute_mean = absolute.iter().sum::<f64>() / absolute.len() as f64;
    let relative_gate = loudness(absolute_mean) + RELATIVE_GATE_LU;
    absolute
        .iter()
        .filter(|power| loudness(**power) > relative_gate)
        .count()
}

fn loudness(power: f64) -> f64 {
    if power > 0.0 {
        LOUDNESS_OFFSET + 10.0 * power.log10()
    } else {
        f64::NEG_INFINITY
    }
}

fn linear_to_db(linear: f64) -> f64 {
    if linear > 0.0 {
        20.0 * linear.log10()
    } else {
        f64::NEG_INFINITY
    }
}

/// 4x oversampled peak using a Hann-windowed sinc interpolator.
fn true_peak(samples: &[f64], channels: usize, frames: usize) -> f64 {
    const RADIUS: isize = 8;
    const PHASES: usize = 4;

    let mut peak = samples
        .iter()
        .fold(0.0f64, |peak, sample| peak.max(sample.abs()));
    if frames < (2 * RADIUS) as usize {
        return peak;
    }

    for channel in 0..channels {
        for frame in RADIUS as usize..frames - RADIUS as usize {
            for phase in 1..PHASES {
                let position = frame as f64 + phase as f64 / PHASES as f64;
                let center = position.floor() as isize;
                let mut sum = 0.0;
                let mut weight_sum = 0.0;
                for tap in -RADIUS + 1..=RADIUS {
                    let index = center + tap;
                    if index < 0 || index >= frames as isize {
                        continue;
                    }
                    let distance = index as f64 - position;
                    let window =
                        0.5 * (1.0 + (std::f64::consts::PI * distance / RADIUS as f64).cos());
                    let argument = std::f64::consts::PI * distance;
                    let sinc = if argument.abs() < 1e-12 {
                        1.0
                    } else {
                        argument.sin() / argument
                    };
                    let weight = sinc * window;
                    weight_sum += weight;
                    sum += samples[index as usize * channels + channel] * weight;
                }
                if weight_sum.abs() > 1e-9 {
                    peak = peak.max((sum / weight_sum).abs());
                }
            }
        }
    }
    peak
}

#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    /// BS.1770 pre-filter (high shelf).
    fn shelf() -> Self {
        Self::new(
            1.535_124_859_586_97,
            -2.691_696_189_406_38,
            1.198_392_810_852_85,
            -1.690_659_293_182_41,
            0.732_480_774_215_85,
        )
    }

    /// BS.1770 RLB high-pass.
    fn high_pass() -> Self {
        Self::new(1.0, -2.0, 1.0, -1.990_047_454_833_98, 0.990_072_250_366_21)
    }

    fn new(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frames: usize, channels: usize, frequency: f32, amplitude: f32) -> Vec<i16> {
        (0..frames)
            .flat_map(|frame| {
                let value = (frame as f32 * std::f32::consts::TAU * frequency / 48_000.0).sin()
                    * amplitude
                    * i16::MAX as f32;
                let value = value.round() as i16;
                std::iter::repeat_n(value, channels)
            })
            .collect()
    }

    #[test]
    fn measures_mono_sine_at_minus_23_lufs() {
        let samples = tone(48_000 * 5, 1, 1_000.0, 0.1);
        let measurement = measure(&samples, 1, 48_000);
        assert!(
            (measurement.integrated_lufs + 23.0).abs() < 0.3,
            "integrated {}",
            measurement.integrated_lufs
        );
        assert!(
            (measurement.true_peak_dbtp + 20.0).abs() < 0.5,
            "true peak {}",
            measurement.true_peak_dbtp
        );
        assert!(measurement.gated_blocks > 0);
    }

    #[test]
    fn identical_stereo_channels_add_three_db() {
        let samples = tone(48_000 * 5, 2, 1_000.0, 0.1);
        let measurement = measure(&samples, 2, 48_000);
        assert!(
            (measurement.integrated_lufs + 20.0).abs() < 0.3,
            "integrated {}",
            measurement.integrated_lufs
        );
    }

    #[test]
    fn silence_has_no_loudness() {
        let measurement = measure(&[0i16; 48_000], 1, 48_000);
        assert_eq!(measurement.integrated_lufs, f32::NEG_INFINITY);
        assert_eq!(measurement.true_peak_dbtp, f32::NEG_INFINITY);
        assert_eq!(measurement.gated_blocks, 0);
    }

    #[test]
    fn relative_gate_ignores_quiet_tails() {
        let mut samples = tone(48_000 * 2, 1, 1_000.0, 0.5);
        samples.extend(tone(48_000 * 4, 1, 1_000.0, 0.001));

        let with_tail = measure(&samples, 1, 48_000);
        let loud_only = measure(&tone(48_000 * 2, 1, 1_000.0, 0.5), 1, 48_000);
        assert!(
            (with_tail.integrated_lufs - loud_only.integrated_lufs).abs() < 0.5,
            "with tail {} vs loud only {}",
            with_tail.integrated_lufs,
            loud_only.integrated_lufs
        );
    }

    #[test]
    fn true_peak_is_at_least_the_sample_peak() {
        let samples = tone(48_000, 2, 11_000.0, 0.9);
        let measurement = measure(&samples, 2, 48_000);
        assert!(measurement.true_peak_dbtp >= measurement.sample_peak_dbfs);
    }

    #[test]
    fn apply_gain_moves_loudness() {
        let samples = tone(48_000 * 3, 1, 1_000.0, 0.05);
        let before = measure(&samples, 1, 48_000);
        let gain = -23.0 - before.integrated_lufs;
        let boosted = apply_gain(&samples, gain);
        let after = measure(&boosted, 1, 48_000);
        assert!(
            (after.integrated_lufs + 23.0).abs() < 0.2,
            "after {} (gain {gain})",
            after.integrated_lufs
        );
    }
}
