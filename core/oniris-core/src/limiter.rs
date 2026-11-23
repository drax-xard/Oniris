//! A stereo lookahead limiter.
//!
//! The mixer can sum to well above full scale, so the master chain needs a
//! limiter that never lets a sample through above the threshold. A short
//! lookahead lets the gain start moving before a transient arrives, which
//! avoids the hard instant-attack "grab" of a plain feed-forward limiter.
//!
//! This is still a sample-peak limiter, not a mastering-grade true-peak (inter-
//! sample) limiter. It is deliberately simple: no oversampling, no allocation
//! on the audio thread, and a deterministic amount of latency.

use std::collections::VecDeque;

/// Delay the signal by this much so the gain envelope can anticipate peaks.
const LOOKAHEAD_MS: f32 = 5.0;

/// Time constant for recovering gain after a peak.
const RELEASE_SECONDS: f32 = 0.100;

pub struct Limiter {
    threshold: f32,
    release: f32,
    lookahead: usize,
    /// Delayed input frames, `lookahead` frames long.
    delay: Vec<(f32, f32)>,
    delay_pos: usize,
    /// Monotonic queue of `(input index, target gain)` covering the lookahead
    /// window. The front is always the minimum target gain in that window.
    targets: VecDeque<(u64, f32)>,
    index: u64,
    gain: f32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        // -1.5 dBFS
        let threshold = 10f32.powf(-1.5 / 20.0);
        let release = (-1.0 / (RELEASE_SECONDS * sample_rate as f32)).exp();
        let lookahead = ((sample_rate as f32 * LOOKAHEAD_MS / 1000.0).round() as usize).max(1);
        Self {
            threshold,
            release,
            lookahead,
            delay: vec![(0.0, 0.0); lookahead],
            delay_pos: 0,
            targets: VecDeque::with_capacity(lookahead + 1),
            index: 0,
            gain: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.gain = 1.0;
        self.delay.fill((0.0, 0.0));
        self.delay_pos = 0;
        self.targets.clear();
        self.index = 0;
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let peak = left.abs().max(right.abs());
        let target = if peak > self.threshold {
            self.threshold / peak
        } else {
            1.0
        };

        // Keep a window of `lookahead + 1` target gains, with the minimum at the
        // front. Prune expired entries before pushing so the queue never grows
        // past its reserved capacity (no allocation on the audio thread).
        while let Some(&(oldest, _)) = self.targets.front() {
            if oldest + self.lookahead as u64 >= self.index {
                break;
            }
            self.targets.pop_front();
        }
        // Dropping dominated entries keeps this O(1) amortized per sample.
        while let Some(&(_, last)) = self.targets.back() {
            if last >= target {
                self.targets.pop_back();
            } else {
                break;
            }
        }
        self.targets.push_back((self.index, target));
        debug_assert!(self.targets.len() <= self.lookahead + 1);

        let target_min = self.targets.front().map_or(1.0, |(_, gain)| *gain);

        // Attack is immediate (bounded by the lookahead window); release is a
        // one-pole ramp. Because the window always includes the sample that is
        // about to leave the delay line, `gain <= target` for that sample and
        // the output cannot overshoot the threshold.
        if target_min < self.gain {
            self.gain = target_min;
        } else {
            self.gain = self.release * self.gain + (1.0 - self.release) * target_min;
        }

        let delayed = self.delay[self.delay_pos];
        self.delay[self.delay_pos] = (left, right);
        self.delay_pos += 1;
        if self.delay_pos == self.delay.len() {
            self.delay_pos = 0;
        }

        self.index += 1;

        (
            (delayed.0 * self.gain).clamp(-1.0, 1.0),
            (delayed.1 * self.gain).clamp(-1.0, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_peaks() {
        let mut limiter = Limiter::new(48_000);
        let mut peak = 0.0f32;
        for _ in 0..10_000 {
            let (left, right) = limiter.process(4.0, -4.0);
            peak = peak.max(left.abs()).max(right.abs());
        }
        assert!(peak <= limiter.threshold + 1e-4);
        assert!(peak > 0.5);
    }

    #[test]
    fn never_overshoots_on_transients() {
        let mut limiter = Limiter::new(48_000);
        let mut peak = 0.0f32;
        for frame in 0..48_000 {
            let value = if frame % 480 == 0 { 8.0 } else { 0.05 };
            let (left, right) = limiter.process(value, -value);
            peak = peak.max(left.abs()).max(right.abs());
        }
        assert!(peak <= limiter.threshold + 1e-4, "peak was {peak}");
    }

    #[test]
    fn passes_quiet_signal_after_lookahead() {
        let mut limiter = Limiter::new(48_000);
        let mut last = (0.0, 0.0);
        for _ in 0..1_000 {
            last = limiter.process(0.1, -0.1);
        }
        assert!((last.0 - 0.1).abs() < 0.001);
        assert!((last.1 + 0.1).abs() < 0.001);
    }

    #[test]
    fn releases_gain_after_a_peak() {
        let mut limiter = Limiter::new(48_000);
        // Drive the limiter into heavy reduction, then let it recover.
        for _ in 0..4_800 {
            limiter.process(8.0, 8.0);
        }
        let reduced = limiter.gain;
        for _ in 0..4_800 {
            limiter.process(0.05, 0.05);
        }
        assert!(reduced < 0.2);
        assert!(limiter.gain > reduced * 4.0);
        assert!(limiter.gain <= 1.0);
    }
}
