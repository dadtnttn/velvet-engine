//! Frame times over a run.

/// How long frames take, and where the slow ones were.
#[derive(Clone, Debug, Default)]
pub struct FrameTimes {
    times: Vec<f32>,
    slow: Vec<(f32, [f32; 2])>,
}

impl FrameTimes {
    /// Frames slower than this (ms) are kept with where they happened.
    pub const SLOW_MS: f32 = 50.0;

    /// Notes a frame of `ms` milliseconds drawn with the walker at `at`; true if it was slow.
    pub fn record(&mut self, ms: f32, at: [f32; 2]) -> bool {
        self.times.push(ms);
        let slow = ms > Self::SLOW_MS;
        if slow {
            self.slow.push((ms, at));
        }
        slow
    }

    /// (mean, 95th percentile, worst) in ms; `None` before any frame.
    pub fn stats(&self) -> Option<(f32, f32, f32)> {
        let mut times = self.times.clone();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let worst = *times.last()?;
        let p95 = times[((times.len() as f32 * 0.95) as usize).min(times.len() - 1)];
        let mean = times.iter().sum::<f32>() / times.len() as f32;
        Some((mean, p95, worst))
    }

    /// The `n` slowest of the slow frames, slowest first: (ms, where).
    pub fn slowest(&self, n: usize) -> Vec<(f32, [f32; 2])> {
        let mut slow = self.slow.clone();
        slow.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        slow.truncate(n);
        slow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_and_the_slowest_frames() {
        let mut frames = FrameTimes::default();
        assert_eq!(frames.stats(), None);
        for k in 0..100 {
            frames.record(k as f32 + 1.0, [k as f32, 0.0]);
        }
        let (mean, p95, worst) = frames.stats().unwrap();
        assert_eq!((mean, p95, worst), (50.5, 96.0, 100.0));
        let slowest = frames.slowest(3);
        assert_eq!(
            slowest.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![100.0, 99.0, 98.0]
        );
        assert_eq!(slowest[0].1, [99.0, 0.0]);
        assert!(!frames.record(50.0, [0.0, 0.0]) && frames.record(50.5, [0.0, 0.0]));
    }
}
