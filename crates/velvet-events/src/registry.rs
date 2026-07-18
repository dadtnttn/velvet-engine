//! Type-erased event channel registry and type-id filters.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

use crate::channel::EventWriter;
use crate::history::EventHistory;
use crate::Event;

/// Type-erased registry of event channels used by `App`.
#[derive(Default)]
pub struct Events {
    channels: HashMap<TypeId, Box<dyn EventChannel>>,
    /// Optional type-id allow-list applied when updating (empty = all).
    update_filter: Option<HashSet<TypeId>>,
    /// Global frame counter for history stamping.
    frame: u64,
    /// Whether to auto-record into per-type histories.
    record_history: bool,
    histories: HashMap<TypeId, Box<dyn HistorySlot>>,
    history_capacity: usize,
}

trait EventChannel: Send + Sync {
    fn update(&mut self);
    fn clear(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn type_id_of_event(&self) -> TypeId;
    fn pending_len(&self) -> usize;
    fn readable_len(&self) -> usize;
}

trait HistorySlot: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear(&mut self);
}

struct TypedChannel<E: Event> {
    writer: EventWriter<E>,
}

impl<E: Event> EventChannel for TypedChannel<E> {
    fn update(&mut self) {
        self.writer.update();
    }
    fn clear(&mut self) {
        self.writer.clear();
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_id_of_event(&self) -> TypeId {
        TypeId::of::<E>()
    }
    fn pending_len(&self) -> usize {
        self.writer.len_pending()
    }
    fn readable_len(&self) -> usize {
        self.writer.len_readable()
    }
}

struct TypedHistory<E: Event + Clone> {
    history: EventHistory<E>,
}

impl<E: Event + Clone> HistorySlot for TypedHistory<E> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn clear(&mut self) {
        self.history.clear();
    }
}

impl Events {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            history_capacity: 256,
            ..Default::default()
        }
    }

    /// Enable/disable automatic history recording for cloneable events.
    pub fn set_record_history(&mut self, enabled: bool) {
        self.record_history = enabled;
    }

    /// History capacity for newly created history streams.
    pub fn set_history_capacity(&mut self, capacity: usize) {
        self.history_capacity = capacity.max(1);
    }

    /// Current logical frame for history.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Register event type `E` if missing; return a writer.
    pub fn add_event<E: Event>(&mut self) -> EventWriter<E> {
        let id = TypeId::of::<E>();
        if let Some(ch) = self.channels.get(&id) {
            return ch
                .as_any()
                .downcast_ref::<TypedChannel<E>>()
                .expect("type id collision")
                .writer
                .clone();
        }
        let writer = EventWriter::<E>::new();
        let stored = writer.clone();
        self.channels.insert(id, Box::new(TypedChannel { writer }));
        stored
    }

    /// Get a writer for `E`, registering if needed.
    pub fn writer<E: Event>(&mut self) -> EventWriter<E> {
        self.add_event::<E>()
    }

    /// Subscribe a reader for `E`.
    pub fn reader<E: Event>(&mut self) -> crate::channel::EventReader<E> {
        self.add_event::<E>().subscribe()
    }

    /// Whether type `E` is registered.
    pub fn has<E: Event>(&self) -> bool {
        self.channels.contains_key(&TypeId::of::<E>())
    }

    /// Type id of `E`.
    pub fn type_id_of<E: Event>() -> TypeId {
        TypeId::of::<E>()
    }

    /// Restrict subsequent [`Self::update`] calls to only these type ids.
    ///
    /// Pass an empty iterator to clear the filter (update all).
    pub fn set_update_filter<I>(&mut self, ids: I)
    where
        I: IntoIterator<Item = TypeId>,
    {
        let set: HashSet<TypeId> = ids.into_iter().collect();
        if set.is_empty() {
            self.update_filter = None;
        } else {
            self.update_filter = Some(set);
        }
    }

    /// Clear update filter (update all channels).
    pub fn clear_update_filter(&mut self) {
        self.update_filter = None;
    }

    /// Update only channels matching `ids` (does not change the stored filter).
    pub fn update_only(&mut self, ids: &HashSet<TypeId>) {
        for ch in self.channels.values_mut() {
            if ids.contains(&ch.type_id_of_event()) {
                ch.update();
            }
        }
        self.frame = self.frame.saturating_add(1);
    }

    /// Update only type `E`.
    pub fn update_type<E: Event>(&mut self) {
        let id = TypeId::of::<E>();
        if let Some(ch) = self.channels.get_mut(&id) {
            ch.update();
        }
        self.frame = self.frame.saturating_add(1);
    }

    /// Swap all registered channels (end of frame), respecting update filter.
    pub fn update(&mut self) {
        if self.record_history {
            // Snapshot pending into history before swap where possible is hard
            // without Clone bounds on all events; history is filled via record_*.
        }
        for ch in self.channels.values_mut() {
            let id = ch.type_id_of_event();
            if let Some(filter) = &self.update_filter {
                if !filter.contains(&id) {
                    continue;
                }
            }
            ch.update();
        }
        self.frame = self.frame.saturating_add(1);
    }

    /// Record cloneable events into history from the previous buffer of `E`.
    pub fn record_history_from_previous<E: Event + Clone>(&mut self) {
        let id = TypeId::of::<E>();
        let events: Vec<E> = {
            let Some(ch) = self.channels.get(&id) else {
                return;
            };
            let typed = ch
                .as_any()
                .downcast_ref::<TypedChannel<E>>()
                .expect("type id collision");
            typed.writer.with_previous(|prev| prev.to_vec())
        };
        if events.is_empty() {
            return;
        }
        let cap = self.history_capacity;
        let frame = self.frame;
        let slot = self.histories.entry(id).or_insert_with(|| {
            Box::new(TypedHistory::<E> {
                history: EventHistory::new(cap),
            })
        });
        let hist = slot
            .as_any_mut()
            .downcast_mut::<TypedHistory<E>>()
            .expect("history type collision");
        hist.history.push_batch(frame, events);
    }

    /// Borrow history for `E` if present.
    pub fn history<E: Event + Clone>(&self) -> Option<&EventHistory<E>> {
        let id = TypeId::of::<E>();
        self.histories
            .get(&id)
            .and_then(|s| s.as_any().downcast_ref::<TypedHistory<E>>())
            .map(|t| &t.history)
    }

    /// Manually push into history for `E`.
    pub fn history_push<E: Event + Clone>(&mut self, event: E) {
        let id = TypeId::of::<E>();
        let cap = self.history_capacity;
        let frame = self.frame;
        let slot = self.histories.entry(id).or_insert_with(|| {
            Box::new(TypedHistory::<E> {
                history: EventHistory::new(cap),
            })
        });
        let hist = slot
            .as_any_mut()
            .downcast_mut::<TypedHistory<E>>()
            .expect("history type collision");
        hist.history.push(frame, event);
    }

    /// Clear all channels.
    pub fn clear(&mut self) {
        for ch in self.channels.values_mut() {
            ch.clear();
        }
    }

    /// Clear histories.
    pub fn clear_histories(&mut self) {
        for h in self.histories.values_mut() {
            h.clear();
        }
    }

    /// Number of registered event types.
    pub fn type_count(&self) -> usize {
        self.channels.len()
    }

    /// Total pending events across all channels.
    pub fn total_pending(&self) -> usize {
        self.channels.values().map(|c| c.pending_len()).sum()
    }

    /// Total readable (previous) events across all channels.
    pub fn total_readable(&self) -> usize {
        self.channels.values().map(|c| c.readable_len()).sum()
    }

    /// Registered type ids.
    pub fn registered_type_ids(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.channels.keys().copied()
    }

    /// Filter helper: keep only type ids present in the registry.
    pub fn filter_registered(&self, ids: impl IntoIterator<Item = TypeId>) -> HashSet<TypeId> {
        let present: HashSet<TypeId> = self.channels.keys().copied().collect();
        ids.into_iter().filter(|id| present.contains(id)).collect()
    }
}

/// Builds a set of type ids for update filtering.
#[derive(Debug, Default, Clone)]
pub struct TypeIdFilter {
    ids: HashSet<TypeId>,
}

impl TypeIdFilter {
    /// Empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Include type `E`.
    pub fn include<E: Event>(mut self) -> Self {
        self.ids.insert(TypeId::of::<E>());
        self
    }

    /// Include a raw type id.
    pub fn include_id(mut self, id: TypeId) -> Self {
        self.ids.insert(id);
        self
    }

    /// Exclude type `E`.
    pub fn exclude<E: Event>(mut self) -> Self {
        self.ids.remove(&TypeId::of::<E>());
        self
    }

    /// Borrow set.
    pub fn as_set(&self) -> &HashSet<TypeId> {
        &self.ids
    }

    /// Into set.
    pub fn into_set(self) -> HashSet<TypeId> {
        self.ids
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Contains type.
    pub fn contains<E: Event>(&self) -> bool {
        self.ids.contains(&TypeId::of::<E>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Debug, PartialEq)]
    struct Ping(u32);

    #[derive(Clone, Debug, PartialEq)]
    struct Pong(u32);

    #[test]
    fn events_registry() {
        let mut events = Events::new();
        let w = events.writer::<Ping>();
        w.send(Ping(7));
        events.update();
        let mut r = events.reader::<Ping>();
        assert_eq!(r.read(), vec![Ping(7)]);
    }

    #[test]
    fn update_filter_by_type_id() {
        let mut events = Events::new();
        let wp = events.writer::<Ping>();
        let wq = events.writer::<Pong>();
        wp.send(Ping(1));
        wq.send(Pong(2));
        events.set_update_filter([TypeId::of::<Ping>()]);
        events.update();
        let mut rp = events.reader::<Ping>();
        let rq = events.reader::<Pong>();
        assert_eq!(rp.read(), vec![Ping(1)]);
        // Pong not swapped — still pending, previous empty.
        assert!(rq.is_empty());
        assert_eq!(events.writer::<Pong>().len_pending(), 1);
    }

    #[test]
    fn type_id_filter_builder() {
        let f = TypeIdFilter::new().include::<Ping>().include::<Pong>();
        assert!(f.contains::<Ping>());
        assert_eq!(f.as_set().len(), 2);
    }

    #[test]
    fn history_record() {
        let mut events = Events::new();
        let w = events.writer::<Ping>();
        w.send(Ping(3));
        events.update();
        events.record_history_from_previous::<Ping>();
        let h = events.history::<Ping>().unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h.latest().unwrap().event, Ping(3));
    }

    #[test]
    fn totals() {
        let mut events = Events::new();
        events.writer::<Ping>().send(Ping(1));
        events.writer::<Pong>().send(Pong(1));
        events.writer::<Pong>().send(Pong(2));
        assert_eq!(events.total_pending(), 3);
        assert_eq!(events.type_count(), 2);
    }
}
