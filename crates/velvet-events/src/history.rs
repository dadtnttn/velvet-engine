//! Ring-buffer event history for replay, debugging, and rollback windows.

use std::collections::VecDeque;
use std::fmt;

/// A stamped event kept in history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricEvent<E> {
    /// Frame index when the event was recorded.
    pub frame: u64,
    /// Monotonic sequence within the history stream.
    pub sequence: u64,
    /// Payload.
    pub event: E,
}

impl<E> HistoricEvent<E> {
    /// Create.
    pub fn new(frame: u64, sequence: u64, event: E) -> Self {
        Self {
            frame,
            sequence,
            event,
        }
    }
}

/// Fixed-capacity ring of recent events.
#[derive(Debug, Clone)]
pub struct EventHistory<E> {
    buf: VecDeque<HistoricEvent<E>>,
    capacity: usize,
    next_sequence: u64,
    /// Total events ever pushed (including those evicted).
    total_pushed: u64,
}

impl<E> Default for EventHistory<E> {
    fn default() -> Self {
        Self::new(256)
    }
}

impl<E> EventHistory<E> {
    /// Create with capacity (minimum 1).
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            buf: VecDeque::with_capacity(capacity),
            capacity,
            next_sequence: 0,
            total_pushed: 0,
        }
    }

    /// Capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current length.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Total pushed lifetime.
    pub fn total_pushed(&self) -> u64 {
        self.total_pushed
    }

    /// Next sequence number that will be assigned.
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Resize capacity; drops oldest if shrinking.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        while self.buf.len() > self.capacity {
            self.buf.pop_front();
        }
    }

    /// Push an event at `frame`.
    pub fn push(&mut self, frame: u64, event: E) {
        let seq = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.total_pushed = self.total_pushed.saturating_add(1);
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(HistoricEvent::new(frame, seq, event));
    }

    /// Push many events at the same frame.
    pub fn push_batch<I: IntoIterator<Item = E>>(&mut self, frame: u64, events: I) {
        for e in events {
            self.push(frame, e);
        }
    }

    /// Clear history (does not reset sequence counters).
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Reset everything including sequence.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.next_sequence = 0;
        self.total_pushed = 0;
    }

    /// Iterate oldest → newest.
    pub fn iter(&self) -> impl Iterator<Item = &HistoricEvent<E>> {
        self.buf.iter()
    }

    /// Events recorded on a specific frame.
    pub fn for_frame(&self, frame: u64) -> impl Iterator<Item = &HistoricEvent<E>> {
        self.buf.iter().filter(move |h| h.frame == frame)
    }

    /// Events with `frame` in `[start, end]` inclusive.
    pub fn for_frame_range(&self, start: u64, end: u64) -> impl Iterator<Item = &HistoricEvent<E>> {
        self.buf
            .iter()
            .filter(move |h| h.frame >= start && h.frame <= end)
    }

    /// Events with sequence ≥ `seq` (for catching up a late reader).
    pub fn since_sequence(&self, seq: u64) -> impl Iterator<Item = &HistoricEvent<E>> {
        self.buf.iter().filter(move |h| h.sequence >= seq)
    }

    /// Newest event.
    pub fn latest(&self) -> Option<&HistoricEvent<E>> {
        self.buf.back()
    }

    /// Oldest event still retained.
    pub fn oldest(&self) -> Option<&HistoricEvent<E>> {
        self.buf.front()
    }

    /// Retain only events with frame ≥ `min_frame`.
    pub fn discard_before_frame(&mut self, min_frame: u64) {
        while self
            .buf
            .front()
            .map(|h| h.frame < min_frame)
            .unwrap_or(false)
        {
            self.buf.pop_front();
        }
    }

    /// Collect cloned payloads for a frame (requires `E: Clone`).
    pub fn collect_frame(&self, frame: u64) -> Vec<E>
    where
        E: Clone,
    {
        self.for_frame(frame).map(|h| h.event.clone()).collect()
    }
}

impl<E: fmt::Debug> fmt::Display for EventHistory<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EventHistory(len={}, cap={}, total={})",
            self.len(),
            self.capacity,
            self.total_pushed
        )
    }
}

/// Multi-type history keyed by type id string labels (for debug tools).
#[derive(Debug, Default)]
pub struct LabeledHistory {
    /// Label → raw debug strings (capacity limited per label).
    streams: Vec<(String, EventHistory<String>)>,
    default_capacity: usize,
}

impl LabeledHistory {
    /// Create with per-stream capacity.
    pub fn new(default_capacity: usize) -> Self {
        Self {
            streams: Vec::new(),
            default_capacity: default_capacity.max(1),
        }
    }

    fn stream_mut(&mut self, label: &str) -> &mut EventHistory<String> {
        if let Some(i) = self.streams.iter().position(|(l, _)| l == label) {
            return &mut self.streams[i].1;
        }
        self.streams
            .push((label.into(), EventHistory::new(self.default_capacity)));
        &mut self.streams.last_mut().unwrap().1
    }

    /// Record a debug line under `label`.
    pub fn record(&mut self, label: &str, frame: u64, message: impl Into<String>) {
        self.stream_mut(label).push(frame, message.into());
    }

    /// Borrow stream.
    pub fn get(&self, label: &str) -> Option<&EventHistory<String>> {
        self.streams
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, h)| h)
    }

    /// Labels present.
    pub fn labels(&self) -> impl Iterator<Item = &str> {
        self.streams.iter().map(|(l, _)| l.as_str())
    }

    /// Clear all streams.
    pub fn clear(&mut self) {
        for (_, h) in &mut self.streams {
            h.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_evicts_oldest() {
        let mut h = EventHistory::new(3);
        h.push(1, "a");
        h.push(2, "b");
        h.push(3, "c");
        h.push(4, "d");
        assert_eq!(h.len(), 3);
        assert_eq!(h.oldest().unwrap().event, "b");
        assert_eq!(h.latest().unwrap().event, "d");
        assert_eq!(h.total_pushed(), 4);
    }

    #[test]
    fn frame_query() {
        let mut h = EventHistory::new(10);
        h.push(1, 10);
        h.push(1, 11);
        h.push(2, 20);
        assert_eq!(h.collect_frame(1), vec![10, 11]);
        assert_eq!(h.for_frame_range(1, 2).count(), 3);
    }

    #[test]
    fn since_sequence() {
        let mut h = EventHistory::new(10);
        h.push(0, "x");
        h.push(1, "y");
        h.push(2, "z");
        let v: Vec<_> = h.since_sequence(1).map(|e| e.event).collect();
        assert_eq!(v, vec!["y", "z"]);
    }

    #[test]
    fn discard_before() {
        let mut h = EventHistory::new(10);
        h.push(1, 1);
        h.push(2, 2);
        h.push(3, 3);
        h.discard_before_frame(3);
        assert_eq!(h.len(), 1);
        assert_eq!(h.oldest().unwrap().event, 3);
    }

    #[test]
    fn labeled() {
        let mut l = LabeledHistory::new(8);
        l.record("input", 1, "jump");
        l.record("input", 2, "attack");
        l.record("audio", 1, "beep");
        assert_eq!(l.get("input").unwrap().len(), 2);
        assert_eq!(l.labels().count(), 2);
    }
}
