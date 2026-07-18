//! Fixed-timestep accumulator improvements.

/// Default fixed update rate (60 Hz).
pub const DEFAULT_FIXED_HZ: f64 = 60.0;

/// Strategy when the accumulator would exceed `max_steps` in one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AccumulatorOverflow {
    /// Clamp leftover to one step (classic spiral-of-death guard).
    #[default]
    ClampRemainder,
    /// Drop all leftover time (may slow simulation under load).
    DropRemainder,
    /// Keep full remainder (risky — can spiral).
    KeepAll,
    /// Clamp remainder to `max_remainder_secs` explicitly.
    ClampTo {
        /// Maximum leftover seconds retained after draining.
        max_remainder_millis: u32,
    },
}

/// Diagnostics from a single drain.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedDrainReport {
    /// Fixed steps executed.
    pub steps: u32,
    /// Whether max steps was hit.
    pub hit_max_steps: bool,
    /// Accumulator before drain.
    pub accumulator_before: f32,
    /// Accumulator after drain.
    pub accumulator_after: f32,
    /// Seconds of simulation advanced (`steps * step_secs`).
    pub simulated_secs: f32,
}

/// Accumulator-based fixed timestep controller with overflow policies.
#[derive(Debug, Clone)]
pub struct FixedTime {
    /// Fixed step duration in seconds.
    step_secs: f32,
    /// Accumulated leftover time.
    accumulator: f32,
    /// Maximum substeps per frame to avoid spiral of death.
    max_steps: u32,
    /// Number of steps taken last frame.
    steps_last_frame: u32,
    /// Total fixed steps executed.
    total_steps: u64,
    /// Overflow handling policy.
    overflow: AccumulatorOverflow,
    /// Whether the last drain hit the max step cap.
    last_hit_max: bool,
    /// Running count of frames that hit max steps.
    overflow_frame_count: u64,
    /// Optional soft alpha smoothing for render interpolation.
    alpha_smoothing: f32,
    /// Smoothed alpha.
    smoothed_alpha: f32,
}

impl Default for FixedTime {
    fn default() -> Self {
        Self::from_hz(DEFAULT_FIXED_HZ)
    }
}

impl FixedTime {
    /// Create from updates-per-second.
    pub fn from_hz(hz: f64) -> Self {
        let hz = if hz <= 0.0 { DEFAULT_FIXED_HZ } else { hz };
        Self {
            step_secs: (1.0 / hz) as f32,
            accumulator: 0.0,
            max_steps: 8,
            steps_last_frame: 0,
            total_steps: 0,
            overflow: AccumulatorOverflow::default(),
            last_hit_max: false,
            overflow_frame_count: 0,
            alpha_smoothing: 0.0,
            smoothed_alpha: 0.0,
        }
    }

    /// Create from explicit step duration.
    pub fn from_step_secs(step_secs: f32) -> Self {
        Self {
            step_secs: step_secs.max(1e-6),
            accumulator: 0.0,
            max_steps: 8,
            steps_last_frame: 0,
            total_steps: 0,
            overflow: AccumulatorOverflow::default(),
            last_hit_max: false,
            overflow_frame_count: 0,
            alpha_smoothing: 0.0,
            smoothed_alpha: 0.0,
        }
    }

    /// Fixed delta seconds.
    #[inline]
    pub fn step_secs(&self) -> f32 {
        self.step_secs
    }

    /// Set step from Hz.
    pub fn set_hz(&mut self, hz: f64) {
        let hz = if hz <= 0.0 { DEFAULT_FIXED_HZ } else { hz };
        self.step_secs = (1.0 / hz) as f32;
    }

    /// Set max substeps per frame.
    pub fn set_max_steps(&mut self, max_steps: u32) {
        self.max_steps = max_steps.max(1);
    }

    /// Max steps per frame.
    pub fn max_steps(&self) -> u32 {
        self.max_steps
    }

    /// Overflow policy.
    pub fn set_overflow(&mut self, policy: AccumulatorOverflow) {
        self.overflow = policy;
    }

    /// Current overflow policy.
    pub fn overflow(&self) -> AccumulatorOverflow {
        self.overflow
    }

    /// Current accumulator value (seconds).
    pub fn accumulator(&self) -> f32 {
        self.accumulator
    }

    /// Manually set accumulator (tests / save restore).
    pub fn set_accumulator(&mut self, secs: f32) {
        self.accumulator = secs.max(0.0);
    }

    /// Reset accumulator and counters (keeps step config).
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
        self.steps_last_frame = 0;
        self.last_hit_max = false;
        self.smoothed_alpha = 0.0;
    }

    /// Enable exponential smoothing on alpha (`0` = off, typical `0.1..=0.3`).
    pub fn set_alpha_smoothing(&mut self, amount: f32) {
        self.alpha_smoothing = amount.clamp(0.0, 1.0);
    }

    /// Feed scaled frame delta; returns how many fixed steps should run.
    pub fn drain_steps(&mut self, scaled_delta_secs: f32) -> u32 {
        self.drain_detailed(scaled_delta_secs).steps
    }

    /// Drain with a detailed report.
    pub fn drain_detailed(&mut self, scaled_delta_secs: f32) -> FixedDrainReport {
        let before = self.accumulator;
        self.accumulator += scaled_delta_secs.max(0.0);
        let mut steps = 0u32;
        while self.accumulator >= self.step_secs && steps < self.max_steps {
            self.accumulator -= self.step_secs;
            steps += 1;
            self.total_steps = self.total_steps.saturating_add(1);
        }
        let hit_max = steps == self.max_steps && self.accumulator >= self.step_secs;
        self.last_hit_max = hit_max;
        if hit_max {
            self.overflow_frame_count = self.overflow_frame_count.saturating_add(1);
            match self.overflow {
                AccumulatorOverflow::ClampRemainder => {
                    self.accumulator = self.accumulator.min(self.step_secs);
                }
                AccumulatorOverflow::DropRemainder => {
                    self.accumulator = 0.0;
                }
                AccumulatorOverflow::KeepAll => {}
                AccumulatorOverflow::ClampTo {
                    max_remainder_millis,
                } => {
                    let max_r = max_remainder_millis as f32 / 1000.0;
                    self.accumulator = self.accumulator.min(max_r.max(0.0));
                }
            }
        }
        self.steps_last_frame = steps;
        let raw_alpha = self.alpha_raw();
        if self.alpha_smoothing > 0.0 {
            self.smoothed_alpha = self.smoothed_alpha * (1.0 - self.alpha_smoothing)
                + raw_alpha * self.alpha_smoothing;
        } else {
            self.smoothed_alpha = raw_alpha;
        }
        FixedDrainReport {
            steps,
            hit_max_steps: hit_max,
            accumulator_before: before,
            accumulator_after: self.accumulator,
            simulated_secs: steps as f32 * self.step_secs,
        }
    }

    /// Run a closure once per drained step (convenience).
    pub fn run_steps<F: FnMut(u32)>(&mut self, scaled_delta_secs: f32, mut f: F) -> u32 {
        let n = self.drain_steps(scaled_delta_secs);
        for i in 0..n {
            f(i);
        }
        n
    }

    fn alpha_raw(&self) -> f32 {
        (self.accumulator / self.step_secs).clamp(0.0, 1.0)
    }

    /// Interpolation alpha in `[0, 1]` for rendering between fixed steps.
    pub fn alpha(&self) -> f32 {
        if self.alpha_smoothing > 0.0 {
            self.smoothed_alpha.clamp(0.0, 1.0)
        } else {
            self.alpha_raw()
        }
    }

    /// Steps executed in the last drain.
    pub fn steps_last_frame(&self) -> u32 {
        self.steps_last_frame
    }

    /// Total fixed updates.
    pub fn total_steps(&self) -> u64 {
        self.total_steps
    }

    /// Whether the last drain hit the step cap.
    pub fn last_hit_max_steps(&self) -> bool {
        self.last_hit_max
    }

    /// Frames that overflowed.
    pub fn overflow_frame_count(&self) -> u64 {
        self.overflow_frame_count
    }

    /// Measure of how "behind" the sim is (accumulator / step).
    pub fn debt_ratio(&self) -> f32 {
        self.accumulator / self.step_secs.max(1e-6)
    }
}

/// Helper that pairs a display clock rate with fixed simulation rate.
#[derive(Debug, Clone)]
pub struct FixedSchedule {
    /// Fixed controller.
    pub fixed: FixedTime,
    /// Optional display Hz for pacing hints (not enforced here).
    pub display_hz: f64,
}

impl FixedSchedule {
    /// 60 Hz sim, 60 Hz display hint.
    pub fn standard_60() -> Self {
        Self {
            fixed: FixedTime::from_hz(60.0),
            display_hz: 60.0,
        }
    }

    /// 30 Hz sim for heavier games.
    pub fn sim_30_display_60() -> Self {
        Self {
            fixed: FixedTime::from_hz(30.0),
            display_hz: 60.0,
        }
    }

    /// Suggested max frame delta before clamping in the variable clock.
    pub fn suggested_max_delta(&self) -> f32 {
        (self.fixed.max_steps() as f32) * self.fixed.step_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drains_two_steps() {
        let mut fixed = FixedTime::from_hz(60.0);
        let steps = fixed.drain_steps(1.0 / 30.0);
        assert_eq!(steps, 2);
        assert!(fixed.alpha() < 1.0);
    }

    #[test]
    fn overflow_drop() {
        let mut fixed = FixedTime::from_hz(60.0);
        fixed.set_max_steps(2);
        fixed.set_overflow(AccumulatorOverflow::DropRemainder);
        let report = fixed.drain_detailed(1.0); // many steps of time
        assert!(report.hit_max_steps);
        assert_eq!(report.steps, 2);
        assert_eq!(fixed.accumulator(), 0.0);
    }

    #[test]
    fn overflow_clamp() {
        let mut fixed = FixedTime::from_hz(60.0);
        fixed.set_max_steps(1);
        fixed.set_overflow(AccumulatorOverflow::ClampRemainder);
        let _ = fixed.drain_detailed(1.0);
        assert!(fixed.accumulator() <= fixed.step_secs() + 1e-5);
    }

    #[test]
    fn run_steps_invokes() {
        let mut fixed = FixedTime::from_step_secs(0.1);
        let mut n = 0;
        fixed.run_steps(0.35, |_| n += 1);
        assert_eq!(n, 3);
    }

    #[test]
    fn schedule_suggested_delta() {
        let s = FixedSchedule::standard_60();
        assert!(s.suggested_max_delta() > 0.1);
    }
}
