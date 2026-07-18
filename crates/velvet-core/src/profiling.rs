//! Simple hierarchical CPU timer tree for profiling spans.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Stable span name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpanName(pub String);

impl SpanName {
    /// Create.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// As str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SpanName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Aggregated stats for one span name.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpanStats {
    /// Times entered.
    pub calls: u64,
    /// Total exclusive time (self, not children) if available; else inclusive.
    pub total: Duration,
    /// Maximum single inclusive duration.
    pub max: Duration,
    /// Minimum single inclusive duration.
    pub min: Duration,
    /// Inclusive total (includes children).
    pub inclusive: Duration,
}

impl SpanStats {
    /// Average inclusive duration.
    pub fn average_inclusive(&self) -> Duration {
        if self.calls == 0 {
            Duration::ZERO
        } else {
            self.inclusive / self.calls as u32
        }
    }

    /// Average exclusive.
    pub fn average_exclusive(&self) -> Duration {
        if self.calls == 0 {
            Duration::ZERO
        } else {
            self.total / self.calls as u32
        }
    }
}

#[derive(Debug)]
struct OpenSpan {
    name: SpanName,
    start: Instant,
    /// Time spent in children while this span was open.
    child_time: Duration,
}

/// Hierarchical CPU profiler (single-threaded).
#[derive(Debug, Default)]
pub struct Profiler {
    stack: Vec<OpenSpan>,
    stats: HashMap<SpanName, SpanStats>,
    enabled: bool,
    /// Frame-scoped samples (name, inclusive).
    frame_samples: Vec<(SpanName, Duration)>,
}

impl Profiler {
    /// Create enabled profiler.
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Enable or disable (disabled spans are no-ops).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.stack.clear();
        }
    }

    /// Whether enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Begin a named span.
    pub fn begin(&mut self, name: impl Into<SpanName>) {
        if !self.enabled {
            return;
        }
        self.stack.push(OpenSpan {
            name: name.into(),
            start: Instant::now(),
            child_time: Duration::ZERO,
        });
    }

    /// End the most recent span.
    pub fn end(&mut self) {
        if !self.enabled {
            return;
        }
        let Some(open) = self.stack.pop() else {
            return;
        };
        let inclusive = open.start.elapsed();
        let exclusive = inclusive.saturating_sub(open.child_time);

        // Attribute child time to parent.
        if let Some(parent) = self.stack.last_mut() {
            parent.child_time += inclusive;
        }

        let entry = self.stats.entry(open.name.clone()).or_default();
        if entry.calls == 0 {
            entry.min = inclusive;
        } else {
            entry.min = entry.min.min(inclusive);
        }
        entry.calls = entry.calls.saturating_add(1);
        entry.total += exclusive;
        entry.inclusive += inclusive;
        entry.max = entry.max.max(inclusive);
        self.frame_samples.push((open.name, inclusive));
    }

    /// Run `f` inside a span.
    pub fn scope<R>(&mut self, name: impl Into<SpanName>, f: impl FnOnce() -> R) -> R {
        self.begin(name);
        let r = f();
        self.end();
        r
    }

    /// Stats for a name.
    pub fn stats(&self, name: &str) -> Option<&SpanStats> {
        self.stats.get(&SpanName::new(name))
    }

    /// All stats.
    pub fn all_stats(&self) -> &HashMap<SpanName, SpanStats> {
        &self.stats
    }

    /// Clear cumulative stats and open stack.
    pub fn reset(&mut self) {
        self.stack.clear();
        self.stats.clear();
        self.frame_samples.clear();
    }

    /// Clear only this frame's sample list (keep cumulative).
    pub fn begin_frame(&mut self) {
        self.frame_samples.clear();
    }

    /// Frame samples (order of completion).
    pub fn frame_samples(&self) -> &[(SpanName, Duration)] {
        &self.frame_samples
    }

    /// Open span depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Top open span name.
    pub fn current(&self) -> Option<&str> {
        self.stack.last().map(|s| s.name.as_str())
    }

    /// Sorted report lines by inclusive time descending.
    pub fn report_lines(&self) -> Vec<String> {
        let mut rows: Vec<_> = self.stats.iter().collect();
        rows.sort_by(|a, b| b.1.inclusive.cmp(&a.1.inclusive));
        rows.into_iter()
            .map(|(name, s)| {
                format!(
                    "{:<32} calls={:<6} incl={:.3}ms excl={:.3}ms max={:.3}ms",
                    name.as_str(),
                    s.calls,
                    s.inclusive.as_secs_f64() * 1000.0,
                    s.total.as_secs_f64() * 1000.0,
                    s.max.as_secs_f64() * 1000.0,
                )
            })
            .collect()
    }

    /// Total inclusive time of root-level samples this frame.
    pub fn frame_total(&self) -> Duration {
        // Approximate: sum samples that completed; nested counted in parents only
        // if we only sum depth-0. We don't track depth in samples; sum unique last roots
        // fallback: max sample.
        self.frame_samples
            .iter()
            .map(|(_, d)| *d)
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

/// RAII span guard — ends span on drop.
pub struct SpanGuard<'a> {
    profiler: &'a mut Profiler,
    active: bool,
}

impl<'a> SpanGuard<'a> {
    /// Begin span.
    pub fn new(profiler: &'a mut Profiler, name: impl Into<SpanName>) -> Self {
        profiler.begin(name);
        Self {
            profiler,
            active: true,
        }
    }
}

impl Drop for SpanGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            self.profiler.end();
        }
    }
}

/// Diagnostics span marker (lighter than profiler; just timestamps for logs).
#[derive(Debug, Clone)]
pub struct DiagnosticSpan {
    /// Name.
    pub name: String,
    /// Start instant.
    pub start: Instant,
    /// Optional end.
    pub end: Option<Instant>,
    /// Optional structured fields (key=value).
    pub fields: Vec<(String, String)>,
}

impl DiagnosticSpan {
    /// Start span now.
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            end: None,
            fields: Vec::new(),
        }
    }

    /// Add field.
    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((key.into(), value.into()));
        self
    }

    /// Finish span.
    pub fn finish(&mut self) {
        self.end = Some(Instant::now());
    }

    /// Duration if finished, else so far.
    pub fn duration(&self) -> Duration {
        match self.end {
            Some(e) => e.saturating_duration_since(self.start),
            None => self.start.elapsed(),
        }
    }
}

/// Ring of recent diagnostic spans.
#[derive(Debug, Clone, Default)]
pub struct SpanLog {
    spans: Vec<DiagnosticSpan>,
    capacity: usize,
}

impl SpanLog {
    /// Create with capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            spans: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    /// Record a finished (or open) span.
    pub fn push(&mut self, span: DiagnosticSpan) {
        if self.spans.len() >= self.capacity {
            self.spans.remove(0);
        }
        self.spans.push(span);
    }

    /// All spans.
    pub fn spans(&self) -> &[DiagnosticSpan] {
        &self.spans
    }

    /// Clear.
    pub fn clear(&mut self) {
        self.spans.clear();
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_spans() {
        let mut p = Profiler::new();
        p.begin_frame();
        p.begin("frame");
        p.begin("update");
        std::thread::sleep(Duration::from_millis(1));
        p.end();
        p.begin("render");
        std::thread::sleep(Duration::from_millis(1));
        p.end();
        p.end();
        assert!(p.stats("update").unwrap().calls == 1);
        assert!(p.stats("frame").unwrap().inclusive >= p.stats("update").unwrap().inclusive);
        assert!(p.depth() == 0);
        let lines = p.report_lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn scope_and_guard() {
        let mut p = Profiler::new();
        let v = p.scope("work", || 42);
        assert_eq!(v, 42);
        {
            let _g = SpanGuard::new(&mut p, "guarded");
            // drop ends
        }
        assert_eq!(p.stats("guarded").unwrap().calls, 1);
    }

    #[test]
    fn disabled_noop() {
        let mut p = Profiler::new();
        p.set_enabled(false);
        p.begin("x");
        p.end();
        assert!(p.all_stats().is_empty());
    }

    #[test]
    fn diagnostic_span_log() {
        let mut span = DiagnosticSpan::start("load").field("path", "a.png");
        span.finish();
        assert!(span.duration() < Duration::from_secs(1));
        let mut log = SpanLog::new(2);
        log.push(span);
        log.push(DiagnosticSpan::start("b"));
        log.push(DiagnosticSpan::start("c"));
        assert_eq!(log.len(), 2);
    }
}
