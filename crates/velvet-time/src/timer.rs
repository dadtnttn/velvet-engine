//! One-shot and repeating timers.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Countdown / interval timer in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Timer {
    duration: f32,
    elapsed: f32,
    finished: bool,
    repeating: bool,
    /// Times the timer has fired since last reset.
    fire_count: u64,
    /// Optional maximum fires for repeating timers (`None` = forever).
    max_fires: Option<u64>,
    paused: bool,
    time_scale: f32,
}

impl Timer {
    /// One-shot timer.
    pub fn once(duration_secs: f32) -> Self {
        Self {
            duration: duration_secs.max(0.0),
            elapsed: 0.0,
            finished: false,
            repeating: false,
            fire_count: 0,
            max_fires: Some(1),
            paused: false,
            time_scale: 1.0,
        }
    }

    /// Repeating timer.
    pub fn repeating(duration_secs: f32) -> Self {
        Self {
            duration: duration_secs.max(0.0),
            elapsed: 0.0,
            finished: false,
            repeating: true,
            fire_count: 0,
            max_fires: None,
            paused: false,
            time_scale: 1.0,
        }
    }

    /// Repeating timer that stops after `n` fires.
    pub fn repeating_n(duration_secs: f32, n: u64) -> Self {
        let mut t = Self::repeating(duration_secs);
        t.max_fires = Some(n.max(1));
        t
    }

    /// Set local time scale (0 pauses this timer only if not using global pause).
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }

    /// Pause without resetting.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Whether paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Tick with delta; returns `true` if the timer fired this tick.
    ///
    /// For repeating timers with large deltas, only one fire is reported per
    /// call (catch-up limited to one). Use [`Self::tick_catch_up`] for multi-fire.
    pub fn tick(&mut self, delta_secs: f32) -> bool {
        self.tick_internal(delta_secs, false) > 0
    }

    /// Tick and return how many times the timer fired (catch-up for repeating).
    pub fn tick_catch_up(&mut self, delta_secs: f32) -> u32 {
        self.tick_internal(delta_secs, true)
    }

    fn tick_internal(&mut self, delta_secs: f32, catch_up: bool) -> u32 {
        if self.paused || self.time_scale <= 0.0 {
            return 0;
        }
        if self.duration <= 0.0 {
            self.finished = true;
            self.fire_count = self.fire_count.saturating_add(1);
            return 1;
        }
        if let Some(max) = self.max_fires {
            if self.fire_count >= max {
                self.finished = true;
                return 0;
            }
        }
        if !self.repeating && self.finished {
            return 0;
        }

        self.elapsed += delta_secs.max(0.0) * self.time_scale;
        let mut fires = 0u32;
        while self.elapsed >= self.duration {
            self.fire_count = self.fire_count.saturating_add(1);
            fires += 1;
            if self.repeating {
                self.elapsed -= self.duration;
                self.finished = true;
                if let Some(max) = self.max_fires {
                    if self.fire_count >= max {
                        self.elapsed = 0.0;
                        break;
                    }
                }
                if !catch_up {
                    break;
                }
            } else {
                self.elapsed = self.duration;
                self.finished = true;
                break;
            }
        }
        if fires == 0 {
            self.finished = false;
        }
        fires
    }

    /// Fraction in `[0, 1]`.
    pub fn fraction(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    /// Remaining seconds.
    pub fn remaining(&self) -> f32 {
        (self.duration - self.elapsed).max(0.0)
    }

    /// Reset elapsed time and fire count.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
        self.fire_count = 0;
    }

    /// Whether finished (one-shot) or last tick fired.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether this is a repeating timer.
    pub fn is_repeating(&self) -> bool {
        self.repeating
    }

    /// Duration seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Change duration (does not reset elapsed).
    pub fn set_duration(&mut self, duration_secs: f32) {
        self.duration = duration_secs.max(0.0);
    }

    /// Times fired since reset.
    pub fn fire_count(&self) -> u64 {
        self.fire_count
    }

    /// Elapsed seconds in current cycle.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }
}

/// Named timer bank for gameplay systems.
#[derive(Debug, Clone, Default)]
pub struct TimerBank {
    timers: Vec<(String, Timer)>,
}

impl TimerBank {
    /// Empty bank.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace by name.
    pub fn insert(&mut self, name: impl Into<String>, timer: Timer) {
        let name = name.into();
        if let Some((_, t)) = self.timers.iter_mut().find(|(n, _)| *n == name) {
            *t = timer;
        } else {
            self.timers.push((name, timer));
        }
    }

    /// Get timer by name.
    pub fn get(&self, name: &str) -> Option<&Timer> {
        self.timers.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    /// Mutable get.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Timer> {
        self.timers
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t)
    }

    /// Tick all; returns names that fired.
    pub fn tick_all(&mut self, delta_secs: f32) -> Vec<String> {
        let mut fired = Vec::new();
        for (name, timer) in &mut self.timers {
            if timer.tick(delta_secs) {
                fired.push(name.clone());
            }
        }
        fired
    }

    /// Number of timers.
    pub fn len(&self) -> usize {
        self.timers.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    /// Remove by name.
    pub fn remove(&mut self, name: &str) -> Option<Timer> {
        if let Some(i) = self.timers.iter().position(|(n, _)| n == name) {
            Some(self.timers.swap_remove(i).1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn catch_up_fires() {
        let mut rep = Timer::repeating(0.1);
        let n = rep.tick_catch_up(0.35);
        assert_eq!(n, 3);
        assert_eq!(rep.fire_count(), 3);
    }

    #[test]
    fn repeating_n_stops() {
        let mut t = Timer::repeating_n(0.1, 2);
        assert!(t.tick(0.1));
        assert!(t.tick(0.1));
        assert!(!t.tick(0.1));
        assert_eq!(t.fire_count(), 2);
    }

    #[test]
    fn pause_blocks() {
        let mut t = Timer::once(1.0);
        t.pause();
        assert!(!t.tick(2.0));
        t.resume();
        assert!(t.tick(1.0));
    }

    #[test]
    fn bank_tick() {
        let mut bank = TimerBank::new();
        bank.insert("a", Timer::once(0.5));
        bank.insert("b", Timer::repeating(0.25));
        let fired = bank.tick_all(0.5);
        assert!(fired.contains(&"a".into()));
        assert!(fired.contains(&"b".into()));
    }
}
