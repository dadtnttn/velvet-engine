//! Runtime diagnostics and frame statistics.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use velvet_time::FpsCounter;

use crate::profiling::{DiagnosticSpan, Profiler, SpanLog};

/// Per-frame statistics snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrameStats {
    /// Frame index.
    pub frame: u64,
    /// Wall delta seconds.
    pub delta_secs: f32,
    /// Smoothed FPS.
    pub fps: f32,
    /// Fixed steps this frame.
    pub fixed_steps: u32,
    /// Approximate CPU frame time seconds (if measured).
    pub cpu_frame_secs: f32,
    /// Draw calls (filled by render plugin).
    pub draw_calls: u32,
    /// Sprites submitted.
    pub sprites: u32,
    /// Active entities (filled by ECS).
    pub entities: u32,
    /// Script instructions executed this frame.
    pub script_instructions: u64,
    /// Audio voices active.
    pub audio_voices: u32,
    /// Event channels with pending events.
    pub events_pending: u32,
}

/// Rolling diagnostics buffer with optional profiler and span log.
#[derive(Debug)]
pub struct Diagnostics {
    fps: FpsCounter,
    history: VecDeque<FrameStats>,
    capacity: usize,
    last: FrameStats,
    enabled: bool,
    /// Hierarchical CPU profiler.
    pub profiler: Profiler,
    /// Lightweight span log for diagnostics.
    pub span_log: SpanLog,
    /// Wall time of last record.
    last_record_at: Option<Instant>,
    /// Counters by name for ad-hoc metrics.
    counters: Vec<(String, i64)>,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new(120)
    }
}

impl Clone for Diagnostics {
    fn clone(&self) -> Self {
        Self {
            fps: self.fps.clone(),
            history: self.history.clone(),
            capacity: self.capacity,
            last: self.last.clone(),
            enabled: self.enabled,
            profiler: Profiler::new(),
            span_log: self.span_log.clone(),
            last_record_at: self.last_record_at,
            counters: self.counters.clone(),
        }
    }
}

impl Diagnostics {
    /// Create with history capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            fps: FpsCounter::new(0.1),
            history: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
            last: FrameStats::default(),
            enabled: true,
            profiler: Profiler::new(),
            span_log: SpanLog::new(64),
            last_record_at: None,
            counters: Vec::new(),
        }
    }

    /// Enable or disable collection.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.profiler.set_enabled(enabled);
    }

    /// Whether enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record end-of-frame stats.
    pub fn record(&mut self, mut stats: FrameStats) {
        if !self.enabled {
            return;
        }
        self.fps.update(stats.delta_secs);
        stats.fps = self.fps.fps();
        self.last = stats.clone();
        if self.history.len() >= self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(stats);
        self.last_record_at = Some(Instant::now());
        self.profiler.begin_frame();
    }

    /// Last frame stats.
    pub fn last(&self) -> &FrameStats {
        &self.last
    }

    /// Average FPS over history.
    pub fn average_fps(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.history.iter().map(|s| s.fps).sum();
        sum / self.history.len() as f32
    }

    /// Average CPU frame seconds over history.
    pub fn average_cpu_frame_secs(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.history.iter().map(|s| s.cpu_frame_secs).sum();
        sum / self.history.len() as f32
    }

    /// Peak sprites in history.
    pub fn peak_sprites(&self) -> u32 {
        self.history.iter().map(|s| s.sprites).max().unwrap_or(0)
    }

    /// History length.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Immutable history iteration (oldest first).
    pub fn history(&self) -> impl Iterator<Item = &FrameStats> {
        self.history.iter()
    }

    /// Current smoothed FPS.
    pub fn fps(&self) -> f32 {
        self.fps.fps()
    }

    /// Push a finished diagnostic span into the log.
    pub fn push_span(&mut self, span: DiagnosticSpan) {
        if self.enabled {
            self.span_log.push(span);
        }
    }

    /// Begin a named diagnostic span (returns handle to finish).
    pub fn start_span(&self, name: impl Into<String>) -> DiagnosticSpan {
        DiagnosticSpan::start(name)
    }

    /// Increment a named counter.
    pub fn counter_add(&mut self, name: &str, delta: i64) {
        if let Some((_, v)) = self.counters.iter_mut().find(|(n, _)| n == name) {
            *v = v.saturating_add(delta);
        } else {
            self.counters.push((name.into(), delta));
        }
    }

    /// Get counter value.
    pub fn counter(&self, name: &str) -> i64 {
        self.counters
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }

    /// Reset counters.
    pub fn reset_counters(&mut self) {
        self.counters.clear();
    }

    /// Time since last record, if any.
    pub fn since_last_record(&self) -> Option<Duration> {
        self.last_record_at.map(|t| t.elapsed())
    }

    /// One-line summary for overlays.
    pub fn summary_line(&self) -> String {
        format!(
            "fps={:.1} frame={} sprites={} draws={} cpu={:.2}ms",
            self.fps(),
            self.last.frame,
            self.last.sprites,
            self.last.draw_calls,
            self.last.cpu_frame_secs * 1000.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_history() {
        let mut d = Diagnostics::new(3);
        for i in 0..5 {
            d.record(FrameStats {
                frame: i,
                delta_secs: 1.0 / 60.0,
                ..Default::default()
            });
        }
        assert_eq!(d.history_len(), 3);
        assert!(d.fps() > 0.0);
        assert!(!d.summary_line().is_empty());
    }

    #[test]
    fn counters_and_spans() {
        let mut d = Diagnostics::new(8);
        d.counter_add("hits", 3);
        d.counter_add("hits", 2);
        assert_eq!(d.counter("hits"), 5);
        let mut s = d.start_span("test");
        s.finish();
        d.push_span(s);
        assert_eq!(d.span_log.len(), 1);
    }
}
