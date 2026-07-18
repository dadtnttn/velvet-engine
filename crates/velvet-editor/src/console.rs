//! Ring-buffer console with level filtering for Velvet Studio.

use std::collections::VecDeque;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Log severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Trace / verbose.
    Trace,
    /// Debug.
    Debug,
    /// Informational.
    Info,
    /// Warning.
    Warn,
    /// Error.
    Error,
}

impl LogLevel {
    /// Parse from a short string (`trace`, `debug`, `info`, `warn`, `error`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "trace" | "t" => Some(Self::Trace),
            "debug" | "d" => Some(Self::Debug),
            "info" | "i" => Some(Self::Info),
            "warn" | "warning" | "w" => Some(Self::Warn),
            "error" | "err" | "e" => Some(Self::Error),
            _ => None,
        }
    }

    /// Canonical short name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One console line with timestamp metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleEntry {
    /// Severity.
    pub level: LogLevel,
    /// Message body.
    pub message: String,
    /// Unix millis when the entry was created (best-effort).
    pub timestamp_ms: u64,
    /// Optional source (panel, command, subsystem).
    pub source: Option<String>,
}

impl ConsoleEntry {
    /// Create a new entry with current wall time.
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp_ms: now_ms(),
            source: None,
        }
    }

    /// With source tag.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Format for terminal output.
    pub fn display_line(&self) -> String {
        match &self.source {
            Some(src) => format!("[{}] [{}] {}", self.level, src, self.message),
            None => format!("[{}] {}", self.level, self.message),
        }
    }
}

/// Ring-buffer console.
///
/// When capacity is exceeded the oldest entries are dropped.
#[derive(Debug, Clone)]
pub struct Console {
    entries: VecDeque<ConsoleEntry>,
    capacity: usize,
    /// Minimum level shown by default filters.
    pub min_level: LogLevel,
    /// Optional substring filter (case-insensitive).
    pub text_filter: Option<String>,
}

impl Default for Console {
    fn default() -> Self {
        Self::with_capacity(512)
    }
}

impl Console {
    /// Create with a fixed capacity (minimum 16).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity: capacity.max(16),
            min_level: LogLevel::Trace,
            text_filter: None,
        }
    }

    /// Current capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of stored entries (unfiltered).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Resize capacity, dropping oldest if needed.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(16);
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    /// Push a fully built entry.
    pub fn push(&mut self, entry: ConsoleEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Log at a level.
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.push(ConsoleEntry::new(level, message));
    }

    /// Log with source.
    pub fn log_src(&mut self, level: LogLevel, source: &str, message: impl Into<String>) {
        self.push(ConsoleEntry::new(level, message).with_source(source));
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }

    pub fn debug(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Debug, message);
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Set minimum visible level.
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }

    /// Set free-text filter (None clears).
    pub fn set_text_filter(&mut self, filter: Option<String>) {
        self.text_filter = filter.filter(|s| !s.trim().is_empty());
    }

    /// Iterate all raw entries (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &ConsoleEntry> {
        self.entries.iter()
    }

    /// Entries matching the active filters.
    pub fn filtered(&self) -> Vec<&ConsoleEntry> {
        self.entries.iter().filter(|e| self.passes(e)).collect()
    }

    /// Count of entries matching filters.
    pub fn filtered_count(&self) -> usize {
        self.entries.iter().filter(|e| self.passes(e)).count()
    }

    /// Count by level across all stored entries.
    pub fn count_by_level(&self) -> [(LogLevel, usize); 5] {
        let mut counts = [
            (LogLevel::Trace, 0usize),
            (LogLevel::Debug, 0),
            (LogLevel::Info, 0),
            (LogLevel::Warn, 0),
            (LogLevel::Error, 0),
        ];
        for e in &self.entries {
            let idx = match e.level {
                LogLevel::Trace => 0,
                LogLevel::Debug => 1,
                LogLevel::Info => 2,
                LogLevel::Warn => 3,
                LogLevel::Error => 4,
            };
            counts[idx].1 += 1;
        }
        counts
    }

    /// Format filtered lines for printing.
    pub fn format_filtered(&self) -> Vec<String> {
        self.filtered()
            .into_iter()
            .map(|e| e.display_line())
            .collect()
    }

    /// Dump last `n` filtered lines (or all if n is 0 / larger than len).
    pub fn tail(&self, n: usize) -> Vec<String> {
        let all = self.format_filtered();
        if n == 0 || n >= all.len() {
            return all;
        }
        all[all.len() - n..].to_vec()
    }

    fn passes(&self, entry: &ConsoleEntry) -> bool {
        if entry.level < self.min_level {
            return false;
        }
        if let Some(ref f) = self.text_filter {
            let needle = f.to_ascii_lowercase();
            let hay = entry.message.to_ascii_lowercase();
            let src_ok = entry
                .source
                .as_ref()
                .map(|s| s.to_ascii_lowercase().contains(&needle))
                .unwrap_or(false);
            if !hay.contains(&needle) && !src_ok {
                return false;
            }
        }
        true
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_drops_oldest() {
        let mut c = Console::with_capacity(16);
        for i in 0..20 {
            c.info(format!("m{i}"));
        }
        assert_eq!(c.len(), 16);
        let first = c.iter().next().unwrap();
        assert!(first.message.starts_with("m4") || first.message == "m4");
    }

    #[test]
    fn level_filter() {
        let mut c = Console::default();
        c.info("ok");
        c.warn("careful");
        c.error("boom");
        c.set_min_level(LogLevel::Warn);
        let f = c.filtered();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].level, LogLevel::Warn);
        assert_eq!(f[1].level, LogLevel::Error);
    }

    #[test]
    fn text_filter() {
        let mut c = Console::default();
        c.info("apple pie");
        c.info("banana");
        c.set_text_filter(Some("pie".into()));
        assert_eq!(c.filtered_count(), 1);
    }

    #[test]
    fn parse_levels() {
        assert_eq!(LogLevel::parse("WARN"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("e"), Some(LogLevel::Error));
        assert!(LogLevel::parse("nope").is_none());
    }
}
