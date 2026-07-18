//! Typed event queues for gameplay / systems communication.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Double-buffered event channel for type `E`.
#[derive(Debug)]
pub struct EventWriter<'a, E: Send + Sync + 'static> {
    queue: &'a mut Events<E>,
}

impl<'a, E: Send + Sync + 'static> EventWriter<'a, E> {
    /// Send an event into the current write buffer.
    pub fn send(&mut self, event: E) {
        self.queue.send(event);
    }

    /// Send multiple events.
    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) {
        for e in events {
            self.queue.send(e);
        }
    }
}

/// Read-only view of events from the last update.
#[derive(Debug)]
pub struct EventReader<'a, E: Send + Sync + 'static> {
    queue: &'a Events<E>,
    cursor: usize,
}

impl<'a, E: Send + Sync + 'static> EventReader<'a, E> {
    /// Iterate events not yet consumed by this reader cursor.
    pub fn read(&mut self) -> impl Iterator<Item = &'a E> + '_ {
        let slice = &self.queue.read_buf[self.cursor..];
        self.cursor = self.queue.read_buf.len();
        slice.iter()
    }

    /// Peek all events without advancing.
    pub fn peek(&self) -> impl Iterator<Item = &'a E> + '_ {
        self.queue.read_buf.iter()
    }

    /// Number of pending unread events.
    pub fn len(&self) -> usize {
        self.queue.read_buf.len().saturating_sub(self.cursor)
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reset cursor to start of buffer.
    pub fn reset(&mut self) {
        self.cursor = 0;
    }
}

/// Double-buffered events of type `E`.
#[derive(Debug)]
pub struct Events<E: Send + Sync + 'static> {
    write_buf: Vec<E>,
    read_buf: Vec<E>,
    _marker: PhantomData<E>,
}

impl<E: Send + Sync + 'static> Default for Events<E> {
    fn default() -> Self {
        Self {
            write_buf: Vec::new(),
            read_buf: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<E: Send + Sync + 'static> Events<E> {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push event to write buffer.
    pub fn send(&mut self, event: E) {
        self.write_buf.push(event);
    }

    /// Swap buffers so written events become readable; clears previous read buffer.
    pub fn update(&mut self) {
        self.read_buf.clear();
        std::mem::swap(&mut self.read_buf, &mut self.write_buf);
    }

    /// Clear both buffers.
    pub fn clear(&mut self) {
        self.write_buf.clear();
        self.read_buf.clear();
    }

    /// Number of events currently readable.
    pub fn len(&self) -> usize {
        self.read_buf.len()
    }

    /// Empty readable buffer.
    pub fn is_empty(&self) -> bool {
        self.read_buf.is_empty()
    }

    /// Iterate readable events.
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.read_buf.iter()
    }

    /// Writer handle.
    pub fn writer(&mut self) -> EventWriter<'_, E> {
        EventWriter { queue: self }
    }

    /// Reader handle at start of buffer.
    pub fn reader(&self) -> EventReader<'_, E> {
        EventReader {
            queue: self,
            cursor: 0,
        }
    }

    /// Drain readable events into a vec (does not clear write buffer).
    pub fn drain_readable(&mut self) -> Vec<E> {
        std::mem::take(&mut self.read_buf)
    }
}

/// Type-erased registry of event queues, stored as a world resource typically.
#[derive(Default)]
pub struct EventQueue {
    queues: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl EventQueue {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure<E: Send + Sync + 'static>(&mut self) -> &mut Events<E> {
        let id = TypeId::of::<E>();
        self.queues
            .entry(id)
            .or_insert_with(|| Box::new(Events::<E>::new()));
        self.queues
            .get_mut(&id)
            .unwrap()
            .downcast_mut::<Events<E>>()
            .expect("event type")
    }

    fn get<E: Send + Sync + 'static>(&self) -> Option<&Events<E>> {
        self.queues
            .get(&TypeId::of::<E>())
            .and_then(|b| b.downcast_ref::<Events<E>>())
    }

    /// Send an event of type `E`.
    pub fn send<E: Send + Sync + 'static>(&mut self, event: E) {
        self.ensure::<E>().send(event);
    }

    /// Update all known queues — only typed access can update individual queues.
    /// Prefer [`Self::update_typed`].
    pub fn update_typed<E: Send + Sync + 'static>(&mut self) {
        self.ensure::<E>().update();
    }

    /// Read events of type `E`.
    pub fn iter<E: Send + Sync + 'static>(&self) -> impl Iterator<Item = &E> + '_ {
        self.get::<E>().into_iter().flat_map(|q| q.iter())
    }

    /// Readable count.
    pub fn len<E: Send + Sync + 'static>(&self) -> usize {
        self.get::<E>().map(|q| q.len()).unwrap_or(0)
    }

    /// Whether the typed queue is empty.
    pub fn is_empty<E: Send + Sync + 'static>(&self) -> bool {
        self.len::<E>() == 0
    }

    /// Clear typed queue.
    pub fn clear<E: Send + Sync + 'static>(&mut self) {
        if let Some(q) = self.queues.get_mut(&TypeId::of::<E>()) {
            if let Some(q) = q.downcast_mut::<Events<E>>() {
                q.clear();
            }
        }
    }

    /// Number of registered event types.
    pub fn type_count(&self) -> usize {
        self.queues.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Damaged {
        amount: i32,
    }

    #[test]
    fn double_buffer_swap() {
        let mut events = Events::<Damaged>::new();
        events.send(Damaged { amount: 1 });
        events.send(Damaged { amount: 2 });
        assert!(events.is_empty()); // not yet readable
        events.update();
        assert_eq!(events.len(), 2);
        events.send(Damaged { amount: 3 });
        events.update();
        assert_eq!(events.len(), 1);
        assert_eq!(events.iter().next().unwrap().amount, 3);
    }

    #[test]
    fn reader_cursor() {
        let mut events = Events::<i32>::new();
        events.send(1);
        events.send(2);
        events.update();
        let mut reader = events.reader();
        let collected: Vec<_> = reader.read().copied().collect();
        assert_eq!(collected, vec![1, 2]);
        assert!(reader.is_empty());
        reader.reset();
        assert_eq!(reader.len(), 2);
    }

    #[test]
    fn event_queue_typed() {
        let mut q = EventQueue::new();
        q.send(Damaged { amount: 9 });
        q.update_typed::<Damaged>();
        let vals: Vec<_> = q.iter::<Damaged>().map(|d| d.amount).collect();
        assert_eq!(vals, vec![9]);
    }
}
