//! Typed double-buffered event channels.

use std::marker::PhantomData;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::Event;

/// A double-buffered queue of events of type `E`.
#[derive(Debug)]
pub struct EventWriter<E: Event> {
    inner: Arc<Mutex<EventBuffer<E>>>,
}

/// Reader cursor for a typed event stream.
#[derive(Debug)]
pub struct EventReader<E: Event> {
    inner: Arc<Mutex<EventBuffer<E>>>,
    last_read_frame: u64,
    /// Optional: only deliver events matching predicate (type-level filters live in registry).
    cursor: usize,
    _marker: PhantomData<E>,
}

#[derive(Debug)]
pub(crate) struct EventBuffer<E: Event> {
    pub(crate) current: Vec<E>,
    pub(crate) previous: Vec<E>,
    pub(crate) frame: u64,
    /// Events sent this frame before swap (stats).
    pub(crate) sent_total: u64,
}

impl<E: Event> Default for EventBuffer<E> {
    fn default() -> Self {
        Self {
            current: Vec::new(),
            previous: Vec::new(),
            frame: 0,
            sent_total: 0,
        }
    }
}

impl<E: Event> EventWriter<E> {
    /// Create a connected writer/reader pair.
    pub fn new_pair() -> (Self, EventReader<E>) {
        let inner = Arc::new(Mutex::new(EventBuffer::default()));
        (
            Self {
                inner: Arc::clone(&inner),
            },
            EventReader {
                inner,
                last_read_frame: 0,
                cursor: 0,
                _marker: PhantomData,
            },
        )
    }

    /// Create writer only (subscribe later).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventBuffer::default())),
        }
    }

    /// Send an event into the current frame buffer.
    pub fn send(&self, event: E) {
        let mut buf = self.inner.lock();
        buf.current.push(event);
        buf.sent_total = buf.sent_total.saturating_add(1);
    }

    /// Send a batch.
    pub fn send_batch<I: IntoIterator<Item = E>>(&self, events: I) {
        let mut buf = self.inner.lock();
        let before = buf.current.len();
        buf.current.extend(events);
        let added = buf.current.len() - before;
        buf.sent_total = buf.sent_total.saturating_add(added as u64);
    }

    /// Send if predicate on current pending count holds (e.g. rate limit).
    pub fn send_if(&self, event: E, mut pred: impl FnMut(usize) -> bool) -> bool {
        let mut buf = self.inner.lock();
        if pred(buf.current.len()) {
            buf.current.push(event);
            buf.sent_total = buf.sent_total.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Create an additional reader on the same channel.
    pub fn subscribe(&self) -> EventReader<E> {
        EventReader {
            inner: Arc::clone(&self.inner),
            last_read_frame: 0,
            cursor: 0,
            _marker: PhantomData,
        }
    }

    /// Swap buffers: previous becomes what was current; current is cleared.
    ///
    /// Call once per frame after systems that produce events for this type,
    /// or via [`crate::Events::update`].
    pub fn update(&self) {
        let mut buf = self.inner.lock();
        let EventBuffer {
            current,
            previous,
            frame,
            ..
        } = &mut *buf;
        std::mem::swap(previous, current);
        current.clear();
        *frame = frame.saturating_add(1);
    }

    /// Number of events queued for the current frame (not yet swapped).
    pub fn len_pending(&self) -> usize {
        self.inner.lock().current.len()
    }

    /// Number of events in the previous (readable) buffer.
    pub fn len_readable(&self) -> usize {
        self.inner.lock().previous.len()
    }

    /// Lifetime sent count.
    pub fn sent_total(&self) -> u64 {
        self.inner.lock().sent_total
    }

    /// Current channel frame counter.
    pub fn frame(&self) -> u64 {
        self.inner.lock().frame
    }

    /// Clear both buffers.
    pub fn clear(&self) {
        let mut buf = self.inner.lock();
        buf.current.clear();
        buf.previous.clear();
    }

    /// Drain previous into a vec without cloning (moves out).
    pub fn take_previous(&self) -> Vec<E> {
        let mut buf = self.inner.lock();
        std::mem::take(&mut buf.previous)
    }

    /// Peek previous without cloning (callback).
    pub fn with_previous<R>(&self, f: impl FnOnce(&[E]) -> R) -> R {
        let buf = self.inner.lock();
        f(&buf.previous)
    }
}

impl<E: Event> Default for EventWriter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Event> Clone for EventWriter<E> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<E: Event> EventReader<E> {
    /// Iterate events from the last completed frame (after `update`).
    pub fn read(&mut self) -> Vec<E>
    where
        E: Clone,
    {
        let buf = self.inner.lock();
        self.last_read_frame = buf.frame;
        self.cursor = buf.previous.len();
        buf.previous.clone()
    }

    /// Read only events matching `pred`.
    pub fn read_filtered<F: FnMut(&E) -> bool>(&mut self, mut pred: F) -> Vec<E>
    where
        E: Clone,
    {
        let buf = self.inner.lock();
        self.last_read_frame = buf.frame;
        self.cursor = buf.previous.len();
        buf.previous.iter().filter(|e| pred(e)).cloned().collect()
    }

    /// Borrow-map over previous events.
    pub fn for_each<F: FnMut(&E)>(&mut self, mut f: F) {
        let buf = self.inner.lock();
        self.last_read_frame = buf.frame;
        self.cursor = buf.previous.len();
        for e in &buf.previous {
            f(e);
        }
    }

    /// Number of events available to read (previous buffer).
    pub fn len(&self) -> usize {
        self.inner.lock().previous.len()
    }

    /// Whether the previous buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Last frame this reader observed.
    pub fn last_read_frame(&self) -> u64 {
        self.last_read_frame
    }

    /// Whether the channel has advanced past the last read (new snapshot available).
    pub fn has_unread(&self) -> bool {
        let buf = self.inner.lock();
        buf.frame != self.last_read_frame && !buf.previous.is_empty()
    }

    /// Clear local cursor metadata (does not clear channel).
    pub fn reset_cursor(&mut self) {
        self.cursor = 0;
        self.last_read_frame = 0;
    }
}

/// Double-buffer helpers for non-shared local queues (no mutex).
#[derive(Debug, Clone)]
pub struct LocalEventQueue<E> {
    current: Vec<E>,
    previous: Vec<E>,
    frame: u64,
}

impl<E> Default for LocalEventQueue<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> LocalEventQueue<E> {
    /// Empty queue.
    pub fn new() -> Self {
        Self {
            current: Vec::new(),
            previous: Vec::new(),
            frame: 0,
        }
    }

    /// Push to current.
    pub fn send(&mut self, event: E) {
        self.current.push(event);
    }

    /// Extend current.
    pub fn send_batch<I: IntoIterator<Item = E>>(&mut self, events: I) {
        self.current.extend(events);
    }

    /// Swap buffers.
    pub fn update(&mut self) {
        std::mem::swap(&mut self.previous, &mut self.current);
        self.current.clear();
        self.frame = self.frame.saturating_add(1);
    }

    /// Read previous slice.
    pub fn previous(&self) -> &[E] {
        &self.previous
    }

    /// Pending current.
    pub fn pending(&self) -> &[E] {
        &self.current
    }

    /// Frame counter.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Clear both.
    pub fn clear(&mut self) {
        self.current.clear();
        self.previous.clear();
    }

    /// Drain previous.
    pub fn drain_previous(&mut self) -> std::vec::Drain<'_, E> {
        self.previous.drain(..)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Ping(u32);

    #[test]
    fn double_buffer_visibility() {
        let (w, mut r) = EventWriter::<Ping>::new_pair();
        w.send(Ping(1));
        assert!(r.is_empty());
        w.update();
        let events = r.read();
        assert_eq!(events, vec![Ping(1)]);
        w.update();
        assert!(r.read().is_empty());
    }

    #[test]
    fn filtered_read() {
        let (w, mut r) = EventWriter::<Ping>::new_pair();
        w.send_batch([Ping(1), Ping(2), Ping(3)]);
        w.update();
        let odds = r.read_filtered(|p| p.0 % 2 == 1);
        assert_eq!(odds, vec![Ping(1), Ping(3)]);
    }

    #[test]
    fn send_if_rate_limit() {
        let w = EventWriter::<Ping>::new();
        assert!(w.send_if(Ping(1), |n| n < 1));
        assert!(!w.send_if(Ping(2), |n| n < 1));
        assert_eq!(w.len_pending(), 1);
    }

    #[test]
    fn local_queue() {
        let mut q = LocalEventQueue::new();
        q.send(1);
        q.send(2);
        assert!(q.previous().is_empty());
        q.update();
        assert_eq!(q.previous(), &[1, 2]);
    }

    #[test]
    fn has_unread() {
        let (w, mut r) = EventWriter::<Ping>::new_pair();
        w.send(Ping(9));
        w.update();
        assert!(r.has_unread());
        let _ = r.read();
        assert!(!r.has_unread());
    }
}
