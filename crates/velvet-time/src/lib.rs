//! # velvet-time
//!
//! Frame timing, fixed timesteps, stopwatches, pause layers, and frame limiters.

#![deny(missing_docs)]

mod fixed;
mod limiter;
mod pause;
mod timer;

use std::time::{Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use fixed::{
    AccumulatorOverflow, FixedDrainReport, FixedSchedule, FixedTime, DEFAULT_FIXED_HZ,
};
pub use limiter::{delta_secs_for_fps, frames_in, FrameLimitConfig, FrameLimiter};
pub use pause::{layers, LayeredClock, PauseLayer, PauseMask, PauseStack};
pub use timer::{Timer, TimerBank};

/// High-level time resource updated once per frame by the app loop.
#[derive(Debug, Clone)]
pub struct Time {
    /// Instant when the app started (or clock reset).
    start: Instant,
    /// Instant of the previous frame.
    last_frame: Instant,
    /// Delta seconds for the last frame (variable).
    delta_secs: f32,
    /// Clamped delta used for gameplay (avoids spiral of death after hitches).
    delta_secs_clamped: f32,
    /// Maximum allowed delta clamp (seconds).
    max_delta_secs: f32,
    /// Time scale multiplier (1.0 = real time, 0.0 = paused).
    time_scale: f32,
    /// Total elapsed seconds since start (scaled).
    elapsed_secs: f64,
    /// Total raw elapsed seconds since start (unscaled).
    elapsed_secs_unscaled: f64,
    /// Monotonic frame index.
    frame_count: u64,
    /// Whether the clock is paused (scale forced to 0 for scaled time).
    paused: bool,
    /// Optional pause stack (layered). When gameplay is paused, scaled delta is 0.
    pause_stack: PauseStack,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

impl Time {
    /// Create a new clock at the current instant.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_frame: now,
            delta_secs: 0.0,
            delta_secs_clamped: 0.0,
            max_delta_secs: 0.25,
            time_scale: 1.0,
            elapsed_secs: 0.0,
            elapsed_secs_unscaled: 0.0,
            frame_count: 0,
            paused: false,
            pause_stack: PauseStack::new(),
        }
    }

    /// Advance the clock using wall time. Call once per frame.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let raw = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.advance(raw);
    }

    /// Advance by an explicit raw delta (useful for tests and determinism).
    pub fn advance(&mut self, raw_delta_secs: f32) {
        let raw = raw_delta_secs.max(0.0);
        self.delta_secs = raw;
        self.delta_secs_clamped = raw.min(self.max_delta_secs);
        let scale = self.effective_time_scale();
        let scaled = self.delta_secs_clamped * scale;
        self.elapsed_secs_unscaled += f64::from(raw);
        self.elapsed_secs += f64::from(scaled);
        self.frame_count = self.frame_count.saturating_add(1);
    }

    fn effective_time_scale(&self) -> f32 {
        if self.paused || self.pause_stack.gameplay_paused() {
            0.0
        } else {
            self.time_scale
        }
    }

    /// Variable frame delta in seconds (unclamped wall delta).
    #[inline]
    pub fn delta_secs(&self) -> f32 {
        self.delta_secs
    }

    /// Clamped delta for gameplay integration.
    #[inline]
    pub fn delta_secs_clamped(&self) -> f32 {
        self.delta_secs_clamped
    }

    /// Scaled delta (`delta_secs_clamped * time_scale`, 0 if paused).
    pub fn scaled_delta_secs(&self) -> f32 {
        self.delta_secs_clamped * self.effective_time_scale()
    }

    /// Elapsed scaled seconds since start.
    #[inline]
    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed_secs
    }

    /// Elapsed unscaled seconds since start.
    #[inline]
    pub fn elapsed_secs_unscaled(&self) -> f64 {
        self.elapsed_secs_unscaled
    }

    /// Frames processed.
    #[inline]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Time scale factor.
    #[inline]
    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    /// Set time scale (negative values are clamped to 0).
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }

    /// Pause scaled time.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume scaled time.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Whether paused (flag or pause stack gameplay).
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused || self.pause_stack.gameplay_paused()
    }

    /// Borrow pause stack.
    pub fn pause_stack(&self) -> &PauseStack {
        &self.pause_stack
    }

    /// Mutable pause stack.
    pub fn pause_stack_mut(&mut self) -> &mut PauseStack {
        &mut self.pause_stack
    }

    /// Push a pause layer.
    pub fn push_pause(&mut self, layer: impl Into<PauseLayer>, mask: PauseMask) {
        self.pause_stack.push(layer, mask);
    }

    /// Pop a pause layer by id.
    pub fn pop_pause(&mut self, layer: &PauseLayer) -> bool {
        self.pause_stack.pop(layer)
    }

    /// Maximum delta clamp.
    pub fn set_max_delta_secs(&mut self, max: f32) {
        self.max_delta_secs = max.max(0.0);
    }

    /// Instant of engine start.
    pub fn start_instant(&self) -> Instant {
        self.start
    }
}

/// Simple stopwatch for profiling sections.
#[derive(Debug, Clone)]
pub struct Stopwatch {
    start: Option<Instant>,
    elapsed: Duration,
    running: bool,
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Stopwatch {
    /// Create a stopped stopwatch.
    pub fn new() -> Self {
        Self {
            start: None,
            elapsed: Duration::ZERO,
            running: false,
        }
    }

    /// Start or resume.
    pub fn start(&mut self) {
        if !self.running {
            self.start = Some(Instant::now());
            self.running = true;
        }
    }

    /// Pause and accumulate.
    pub fn stop(&mut self) {
        if self.running {
            if let Some(s) = self.start.take() {
                self.elapsed += s.elapsed();
            }
            self.running = false;
        }
    }

    /// Reset to zero and stop.
    pub fn reset(&mut self) {
        self.start = None;
        self.elapsed = Duration::ZERO;
        self.running = false;
    }

    /// Restart: reset and start.
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }

    /// Elapsed duration including current run segment.
    pub fn elapsed(&self) -> Duration {
        if self.running {
            self.elapsed + self.start.map(|s| s.elapsed()).unwrap_or_default()
        } else {
            self.elapsed
        }
    }

    /// Elapsed seconds as f64.
    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// Elapsed milliseconds.
    pub fn elapsed_millis(&self) -> f64 {
        self.elapsed_secs() * 1000.0
    }

    /// Whether running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Instantaneous FPS estimator with exponential smoothing.
#[derive(Debug, Clone)]
pub struct FpsCounter {
    fps: f32,
    smoothing: f32,
    samples: u64,
    min_fps: f32,
    max_fps: f32,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl FpsCounter {
    /// `smoothing` closer to 0 reacts faster; typical 0.05–0.2.
    pub fn new(smoothing: f32) -> Self {
        Self {
            fps: 0.0,
            smoothing: smoothing.clamp(0.0, 1.0),
            samples: 0,
            min_fps: f32::MAX,
            max_fps: 0.0,
        }
    }

    /// Update with frame delta seconds.
    pub fn update(&mut self, delta_secs: f32) {
        if delta_secs <= 1e-9 {
            return;
        }
        let sample = 1.0 / delta_secs;
        if self.fps <= 0.0 {
            self.fps = sample;
        } else {
            self.fps = self.fps * (1.0 - self.smoothing) + sample * self.smoothing;
        }
        self.samples = self.samples.saturating_add(1);
        self.min_fps = self.min_fps.min(sample);
        self.max_fps = self.max_fps.max(sample);
    }

    /// Smoothed FPS.
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Minimum observed instant FPS.
    pub fn min_fps(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.min_fps
        }
    }

    /// Maximum observed instant FPS.
    pub fn max_fps(&self) -> f32 {
        self.max_fps
    }

    /// Sample count.
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// Reset stats.
    pub fn reset_stats(&mut self) {
        self.min_fps = f32::MAX;
        self.max_fps = 0.0;
        self.samples = 0;
    }
}

/// Manual clock that only advances when told (deterministic tests).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ManualClock {
    /// Elapsed seconds.
    pub elapsed: f64,
    /// Last delta.
    pub delta: f32,
    /// Frame index.
    pub frame: u64,
}

impl Default for ManualClock {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            delta: 0.0,
            frame: 0,
        }
    }
}

impl ManualClock {
    /// Advance by delta.
    pub fn tick(&mut self, delta: f32) {
        let d = delta.max(0.0);
        self.delta = d;
        self.elapsed += f64::from(d);
        self.frame = self.frame.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_time_drains_steps() {
        let mut fixed = FixedTime::from_hz(60.0);
        let steps = fixed.drain_steps(1.0 / 30.0);
        assert_eq!(steps, 2);
        assert!(fixed.alpha() < 1.0);
    }

    #[test]
    fn time_scale_and_pause() {
        let mut t = Time::new();
        t.set_time_scale(0.5);
        t.advance(0.1);
        assert!((t.scaled_delta_secs() - 0.05).abs() < 1e-5);
        t.pause();
        t.advance(0.1);
        assert_eq!(t.scaled_delta_secs(), 0.0);
    }

    #[test]
    fn time_pause_layer() {
        let mut t = Time::new();
        t.push_pause(layers::game(), PauseMask::GAMEPLAY_ONLY);
        t.advance(0.1);
        assert_eq!(t.scaled_delta_secs(), 0.0);
        assert!(t.is_paused());
        t.pop_pause(&layers::game());
        t.advance(0.1);
        assert!((t.scaled_delta_secs() - 0.1_f32.min(t.delta_secs_clamped())).abs() < 1e-5);
    }

    #[test]
    fn timer_once_and_repeat() {
        let mut once = Timer::once(1.0);
        assert!(!once.tick(0.4));
        assert!(once.tick(0.7));
        assert!(!once.tick(1.0));

        let mut rep = Timer::repeating(0.5);
        assert!(rep.tick(0.5));
        assert!(rep.tick(0.5));
    }

    #[test]
    fn stopwatch_runs() {
        let mut s = Stopwatch::new();
        s.start();
        assert!(s.is_running());
        s.stop();
        assert!(!s.is_running());
    }

    #[test]
    fn fps_counter_minmax() {
        let mut f = FpsCounter::new(0.1);
        f.update(1.0 / 30.0);
        f.update(1.0 / 120.0);
        assert!(f.min_fps() <= f.max_fps());
        assert!(f.fps() > 0.0);
    }

    #[test]
    fn manual_clock() {
        let mut c = ManualClock::default();
        c.tick(0.5);
        assert_eq!(c.frame, 1);
        assert!((c.elapsed - 0.5).abs() < 1e-9);
    }

    #[test]
    fn frame_limiter_config() {
        assert!(FrameLimitConfig::fps(60).frame_period().is_some());
        assert!((delta_secs_for_fps(20) - 0.05).abs() < 1e-6);
    }
}
