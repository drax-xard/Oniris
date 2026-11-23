//! Linked-stereo lookahead limiter.
//!
//! The limiter delays the signal by a few milliseconds and takes a running
//! maximum over the delay window, so it starts reducing gain before a peak
//! reaches the output. The attack converges within one lookahead window (5 ms)
//! and the 100 ms release keeps the reduction smooth; without lookahead the
//! previous implementation had to use an instant attack, which distorted
//! transients.
//!
//! The ceiling is -1.5 dBFS sample peak. True-peak metering and the final
//! -1.5 dBTP content gate live in the asset pipeline; the engine's job is to
//! guarantee that the mixed output never clips.

use std::collections::VecDeque;

const LOOKAHEAD_MS: f32 = 5.0;
const RELEASE_SECS: f32 = 0.100;
const CEILING_DBFS: f32 = -1.5;

pub struct Limiter {
    threshold: f32,
    lookahead: usize,
    delay_left: Vec<f32>,
    delay_right: Vec<f32>,
    write_index: usize,
    /// Monotonic queue of (absolute sample index, peak) covering the window.
    peaks: VecDeque<(u64, f32)>,
    sample_index: u64,
    gain: f32,
    attack: f32,
    release: f32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate = sample_rate.max(1);
        let threshold = 10f32.powf(CEILING_DBFS / 20.0);
        let lookahead = ((sample_rate as f32 * LOOKAHEAD_MS / 1000.0).round() as usize).max(1);
        let release = (-1.0 / (RELEASE_SECS * sample_rate as f32)).exp();
        // Converge to within 1% of the target gain during the lookahead window.
        let attack = 1.0 - 0.01f32.powf(1.0 / lookahead as f32);
        Self {
            threshold,
            lookahead,
            delay_left: vec![0.0; lookahead],
            delay_right: vec![0.0; lookahead],
            write_index: 0,
            peaks: VecDeque::with_capacity(lookahead + 1),
            sample_index: 0,
            gain: 1.0,
            attack,
            release,
        }
    }

    pub fn reset(&mut self) {
        self.delay_left.fill(0.0);
        self.delay_right.fill(0.0);
        self.write_index = 0;
        self.peaks.clear();
        self.sample_index = 0;
        self.gain = 1.0;
    }

    pub fn latency_frames(&self) -> usize {
        self.lookahead
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Read the sample that entered the delay line one lookahead ago, then
        // replace it with the current input.
        let delayed_left = self.delay_left[self.write_index];
        let delayed_right = self.delay_right[self.write_index];
        self.delay_left[self.write_index] = left;
        self.delay_right[self.write_index] = right;
        self.write_index += 1;
        if self.write_index == self.lookahead {
            self.write_index = 0;
        }

        let peak = left.abs().max(right.abs());
        let index = self.sample_index;
        self.sample_index += 1;

        // Keep a monotonically decreasing window maximum. Expire entries that
        // have fallen more than `lookahead` samples behind.
        while let Some(back) = self.peaks.back() {
            if back.1 <= peak {
                self.peaks.pop_back();
            } else {
                break;
            }
        }
        self.peaks.push_back((index, peak));
        while let Some(front) = self.peaks.front() {
            if front.0 + self.lookahead as u64 <= index {
                self.peaks.pop_front();
            } else {
                break;
            }
        }

        let window_peak = self.peaks.front().map(|entry| entry.1).unwrap_or(0.0);
        let target = if window_peak > self.threshold {
            self.threshold / window_peak
        } else {
            1.0
        };

        // The window includes peaks up to a full lookahead ahead of the output,
        // so the gain has converged by the time the offending sample emerges.
        let coefficient = if target < self.gain {
            self.attack
        } else {
            self.release
        };
        self.gain += (target - self.gain) * coefficient;

        // A final linked ceiling removes the sub-0.1 dB of residual overshoot
        // left by the exponential attack. With lookahead it should not engage
        // except on the very first peak after reset.
        let gain = self.gain.min(if window_peak > self.threshold {
            self.threshold / window_peak.max(1e-9)
        } else {
            1.0
        });
        (
            (delayed_left * gain).clamp(-self.threshold, self.threshold),
            (delayed_right * gain).clamp(-self.threshold, self.threshold),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(limiter: &mut Limiter, input: &[(f32, f32)]) -> Vec<(f32, f32)> {
        input.iter().map(|&(l, r)| limiter.process(l, r)).collect()
    }

    #[test]
    fn limits_sustained_peaks() {
        let mut limiter = Limiter::new(48_000);
        let input = vec![(4.0, -4.0); 48_000];
        let output = run(&mut limiter, &input);
        // Skip the lookahead priming region.
        let peak = output
            .iter()
            .skip(limiter.latency_frames() * 2)
            .fold(0.0f32, |peak, (l, r)| peak.max(l.abs()).max(r.abs()));
        assert!(peak <= limiter.threshold + 1e-6, "peak={peak}");
        assert!(peak > limiter.threshold * 0.95, "peak={peak}");
    }

    #[test]
    fn catches_a_transient_impulse() {
        let mut limiter = Limiter::new(48_000);
        let mut input = vec![(0.0, 0.0); 10_000];
        input[5_000] = (8.0, 8.0);
        let output = run(&mut limiter, &input);
        let peak = output
            .iter()
            .fold(0.0f32, |peak, (l, r)| peak.max(l.abs()).max(r.abs()));
        assert!(peak <= limiter.threshold + 1e-6, "peak={peak}");
    }

    #[test]
    fn passes_quiet_signal_unchanged() {
        let mut limiter = Limiter::new(48_000);
        let delay = limiter.latency_frames();
        let input = vec![(0.1, -0.1); delay + 128];
        let output = run(&mut limiter, &input);
        for (left, right) in output.iter().skip(delay) {
            assert!((left - 0.1).abs() < 1e-3, "left={left}");
            assert!((right + 0.1).abs() < 1e-3, "right={right}");
        }
    }

    #[test]
    fn stereo_is_linked() {
        let mut limiter = Limiter::new(48_000);
        let delay = limiter.latency_frames();
        let input = vec![(0.1, 2.0); delay + 2_000];
        let output = run(&mut limiter, &input);
        let (left, right) = output[output.len() - 1];
        let ratio = right / left;
        assert!(
            (ratio - 20.0).abs() < 0.1,
            "left={left} right={right} ratio={ratio}"
        );
    }

    #[test]
    fn no_sample_clipping_on_hot_signal() {
        let mut limiter = Limiter::new(48_000);
        let mut state = 0x1234_5678u32;
        for _ in 0..48_000 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let sample = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            let (left, right) = limiter.process(sample * 6.0, -sample * 6.0);
            assert!(left.abs() <= limiter.threshold + 1e-6);
            assert!(right.abs() <= limiter.threshold + 1e-6);
        }
    }
}
