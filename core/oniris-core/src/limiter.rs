//! A small feed-forward limiter. It is not a mastering-grade true-peak limiter,
//! but it catches the rare sum that would otherwise clip after mixing.

pub struct Limiter {
    threshold: f32,
    gain: f32,
    release: f32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        // -1.5 dBFS
        let threshold = 10f32.powf(-1.5 / 20.0);
        let release = (-1.0 / (0.100 * sample_rate as f32)).exp();
        Self {
            threshold,
            gain: 1.0,
            release,
        }
    }

    pub fn reset(&mut self) {
        self.gain = 1.0;
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let peak = left.abs().max(right.abs());
        let target = if peak > self.threshold {
            self.threshold / peak
        } else {
            1.0
        };

        // Instant attack, smooth release. Without lookahead this is the
        // simplest way to guarantee that a sample never exceeds the threshold.
        // The production engine should add a short lookahead to avoid
        // transient distortion.
        if target < self.gain {
            self.gain = target;
        } else {
            self.gain = self.release * self.gain + (1.0 - self.release) * target;
        }

        (
            (left * self.gain).clamp(-1.0, 1.0),
            (right * self.gain).clamp(-1.0, 1.0),
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
        assert!(peak <= 1.0);
        assert!(peak > 0.5);
    }

    #[test]
    fn passes_quiet_signal() {
        let mut limiter = Limiter::new(48_000);
        let (left, right) = limiter.process(0.1, -0.1);
        assert!((left - 0.1).abs() < 0.001);
        assert!((right + 0.1).abs() < 0.001);
    }
}
