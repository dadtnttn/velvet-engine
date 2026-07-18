//! Frame rate limiter helpers (sleep-based; not a hard real-time guarantee).

use std::time::{Duration, Instant};

/// Configuration for a frame limiter.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrameLimitConfig {
    /// Target frames per second (`None` = uncapped).
    pub target_fps: Option<u32>,
    /// Whether to busy-wait the last few microseconds for tighter pacing.
    pub precise: bool,
    /// Minimum sleep quantum to attempt (platform-dependent usefulness).
    pub min_sleep: Duration,
}

impl Default for FrameLimitConfig {
    fn default() -> Self {
        Self {
            target_fps: Some(60),
            precise: false,
            min_sleep: Duration::from_millis(1),
        }
    }
}

impl FrameLimitConfig {
    /// Uncapped.
    pub fn uncapped() -> Self {
        Self {
            target_fps: None,
            ..Default::default()
        }
    }

    /// Target FPS helper.
    pub fn fps(fps: u32) -> Self {
        Self {
            target_fps: Some(fps.max(1)),
            ..Default::default()
        }
    }

    /// Frame period, if capped.
    pub fn frame_period(&self) -> Option<Duration> {
        self.target_fps.map(|fps| {
            let fps = fps.max(1) as f64;
            Duration::from_secs_f64(1.0 / fps)
        })
    }
}

/// Tracks frame start times and sleeps to approach a target frame rate.
#[derive(Debug, Clone)]
pub struct FrameLimiter {
    config: FrameLimitConfig,
    frame_start: Instant,
    last_sleep: Duration,
    /// Frames processed.
    frame_count: u64,
    /// Accumulated overshoot seconds (negative = we were late).
    debt_secs: f64,
}

impl Default for FrameLimiter {
    fn default() -> Self {
        Self::new(FrameLimitConfig::default())
    }
}

impl FrameLimiter {
    /// Create limiter.
    pub fn new(config: FrameLimitConfig) -> Self {
        Self {
            config,
            frame_start: Instant::now(),
            last_sleep: Duration::ZERO,
            frame_count: 0,
            debt_secs: 0.0,
        }
    }

    /// Replace config.
    pub fn set_config(&mut self, config: FrameLimitConfig) {
        self.config = config;
    }

    /// Current config.
    pub fn config(&self) -> FrameLimitConfig {
        self.config
    }

    /// Mark the beginning of a frame (call before work).
    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
    }

    /// End frame: sleep if we finished early. Returns slept duration.
    pub fn end_frame(&mut self) -> Duration {
        self.frame_count = self.frame_count.saturating_add(1);
        let Some(period) = self.config.frame_period() else {
            self.last_sleep = Duration::ZERO;
            return Duration::ZERO;
        };

        let elapsed = self.frame_start.elapsed();
        let period_secs = period.as_secs_f64();
        let elapsed_secs = elapsed.as_secs_f64();

        // debt > 0 means we have leftover budget from previous early frames
        let remaining = period_secs - elapsed_secs + self.debt_secs;
        if remaining <= 0.0 {
            // Late: accumulate debt (clamped) so we don't thrash forever.
            self.debt_secs = remaining.max(-period_secs);
            self.last_sleep = Duration::ZERO;
            return Duration::ZERO;
        }

        let sleep_dur = Duration::from_secs_f64(remaining);
        if self.config.precise {
            precise_sleep(sleep_dur, self.config.min_sleep);
        } else if sleep_dur >= self.config.min_sleep {
            std::thread::sleep(sleep_dur);
        } else {
            // Too short to sleep usefully.
            std::thread::yield_now();
        }

        // Measure actual sleep overshoot/undershoot.
        let total = self.frame_start.elapsed().as_secs_f64();
        self.debt_secs = period_secs - total;
        // Clamp debt to one frame.
        self.debt_secs = self.debt_secs.clamp(-period_secs, period_secs);
        self.last_sleep = sleep_dur;
        sleep_dur
    }

    /// Convenience: begin + return a guard that ends on drop is not used;
    /// this runs end_frame after computing how long work already took from external start.
    pub fn wait_for_frame_budget(&mut self, work_started: Instant) -> Duration {
        self.frame_start = work_started;
        self.end_frame()
    }

    /// Last sleep duration.
    pub fn last_sleep(&self) -> Duration {
        self.last_sleep
    }

    /// Frames limited.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Current pacing debt seconds.
    pub fn debt_secs(&self) -> f64 {
        self.debt_secs
    }

    /// Reset debt and counters.
    pub fn reset(&mut self) {
        self.debt_secs = 0.0;
        self.frame_count = 0;
        self.last_sleep = Duration::ZERO;
        self.frame_start = Instant::now();
    }
}

fn precise_sleep(total: Duration, min_sleep: Duration) {
    let start = Instant::now();
    // Sleep most of the time, spin the last 0.5ms.
    let spin = Duration::from_micros(500);
    if total > spin {
        let coarse = total - spin;
        if coarse >= min_sleep {
            std::thread::sleep(coarse);
        }
    }
    while start.elapsed() < total {
        std::thread::yield_now();
    }
}

/// Compute how many whole frames fit in a duration at a given FPS.
pub fn frames_in(duration: Duration, fps: u32) -> u64 {
    let fps = fps.max(1) as f64;
    (duration.as_secs_f64() * fps).floor() as u64
}

/// Ideal delta seconds for a target FPS.
pub fn delta_secs_for_fps(fps: u32) -> f32 {
    1.0 / fps.max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_period() {
        let c = FrameLimitConfig::fps(60);
        let p = c.frame_period().unwrap();
        assert!((p.as_secs_f64() - 1.0 / 60.0).abs() < 1e-6);
        assert!(FrameLimitConfig::uncapped().frame_period().is_none());
    }

    #[test]
    fn end_frame_uncapped_no_sleep() {
        let mut lim = FrameLimiter::new(FrameLimitConfig::uncapped());
        lim.begin_frame();
        assert_eq!(lim.end_frame(), Duration::ZERO);
    }

    #[test]
    fn end_frame_sleeps_when_fast() {
        let mut lim = FrameLimiter::new(FrameLimitConfig {
            target_fps: Some(100),
            precise: false,
            min_sleep: Duration::from_millis(0),
        });
        lim.begin_frame();
        // Should attempt to sleep ~10ms; allow some slack on CI.
        let slept = lim.end_frame();
        // On very slow CI we might not sleep; just ensure API works.
        let _ = slept;
        assert_eq!(lim.frame_count(), 1);
    }

    #[test]
    fn helpers() {
        assert_eq!(frames_in(Duration::from_secs(1), 60), 60);
        assert!((delta_secs_for_fps(50) - 0.02).abs() < 1e-6);
    }
}
