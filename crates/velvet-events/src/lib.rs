//! # velvet-events
//!
//! Double-buffered typed event channels so readers see a stable snapshot for the frame.
//! Also provides event history rings and type-id filters.

#![deny(missing_docs)]

mod channel;
mod history;
mod registry;

pub use channel::{EventReader, EventWriter, LocalEventQueue};
pub use history::{EventHistory, HistoricEvent, LabeledHistory};
pub use registry::{Events, TypeIdFilter};

/// Trait bound for events: sendable, static payloads.
pub trait Event: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Event for T {}

/// Application lifecycle events emitted by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecycleEvent {
    /// Engine finished plugin build.
    Startup,
    /// About to exit main loop.
    Shutdown,
    /// Window focus gained.
    Focused,
    /// Window focus lost.
    Unfocused,
    /// Application paused (mobile / editor).
    Suspended,
    /// Application resumed.
    Resumed,
}

/// Window resize event in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowResized {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

/// Request to exit the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppExit {
    /// Process exit code suggestion.
    pub code: i32,
}

/// Generic resource-changed notification (payload is type name / id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceChanged {
    /// Debug type name.
    pub type_name: &'static str,
    /// Change tick when emitted.
    pub tick: u64,
}

/// Input action edge event for systems that prefer events over polling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionEvent {
    /// Action name.
    pub action: String,
    /// Pressed (true) or released (false).
    pub pressed: bool,
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
    fn events_registry() {
        let mut events = Events::new();
        let w = events.writer::<Ping>();
        w.send(Ping(7));
        events.update();
        let mut r = events.reader::<Ping>();
        // After update, previous has the ping; a new reader still sees previous.
        assert_eq!(r.read(), vec![Ping(7)]);
    }

    #[test]
    fn lifecycle_and_exit() {
        let e = AppExit { code: 0 };
        assert_eq!(e.code, 0);
        let _ = AppLifecycleEvent::Startup;
    }
}
