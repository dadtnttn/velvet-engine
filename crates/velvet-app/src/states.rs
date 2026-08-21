//! Typed application state machine.
//!
//! Games move between coarse modes (Menu, Loading, Playing, Paused). Register a
//! state enum once and drive transitions through [`crate::App`]; pending
//! transitions are applied at the top of each frame and announced through the
//! double-buffered event registry.
//!
//! ```ignore
//! #[derive(Clone, PartialEq, Debug)]
//! enum GamePhase { Menu, Loading, Playing }
//!
//! app.init_state(GamePhase::Menu);
//! app.set_state(GamePhase::Loading)?;
//! // next frame: current == Loading, StateTransitioned sent
//! ```

use crate::app::App;
use std::fmt::{Debug, Display};

/// Trait bound for application state values.
pub trait State: Clone + PartialEq + Send + Sync + Debug + 'static {}
impl<T: Clone + PartialEq + Send + Sync + Debug + 'static> State for T {}

/// Event emitted after a pending transition is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateTransitioned<S: State> {
    /// State before the transition.
    pub from: S,
    /// State after the transition.
    pub to: S,
}

/// Resource holding the current, previous, and queued state of type `S`.
#[derive(Debug)]
pub struct States<S: State> {
    current: S,
    previous: Option<S>,
    pending: Option<S>,
}

impl<S: State> States<S> {
    /// Create a container starting at `initial`.
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            previous: None,
            pending: None,
        }
    }

    /// Current state.
    pub fn get(&self) -> &S {
        &self.current
    }

    /// State before the last applied transition, if any.
    pub fn previous(&self) -> Option<&S> {
        self.previous.as_ref()
    }

    /// Queued transition target, if any.
    pub fn pending(&self) -> Option<&S> {
        self.pending.as_ref()
    }

    /// Queue a transition to be applied on the next frame start.
    ///
    /// Replaces any previously queued (not yet applied) transition.
    pub fn set(&mut self, next: S) {
        self.pending = Some(next);
    }

    /// Take the queued transition target.
    pub fn take_pending(&mut self) -> Option<S> {
        self.pending.take()
    }

    /// Apply `next` as the current state; returns `(from, to)` when the state
    /// actually changed.
    pub fn apply(&mut self, next: S) -> Option<(S, S)> {
        if self.current == next {
            return None;
        }
        let from = self.current.clone();
        self.previous = Some(std::mem::replace(&mut self.current, next));
        Some((from, self.current.clone()))
    }
}

/// Error returned when operating on an unregistered state type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateError {
    /// Unregistered state value type name.
    pub type_name: &'static str,
}

impl Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "state type `{}` is not initialized; call App::init_state first",
            self.type_name
        )
    }
}

pub(crate) type StateFlusher = Box<dyn Fn(&mut App) + Send + Sync>;

pub(crate) fn flush_registered_states(app: &mut App) {
    let mut flushers = std::mem::take(&mut app.state_flushers);
    for flusher in flushers.iter_mut() {
        flusher(app);
    }
    app.state_flushers = flushers;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Eq, Debug)]
    enum Phase {
        Menu,
        Playing,
        Paused,
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    enum Net {
        Offline,
        Online,
    }

    #[test]
    fn queue_then_apply_transitions_and_records_previous() {
        let mut st = States::new(Phase::Menu);
        assert_eq!(st.get(), &Phase::Menu);
        st.set(Phase::Playing);

        let pending = st.take_pending().unwrap();
        let change = st.apply(pending).unwrap();
        assert_eq!(change, (Phase::Menu, Phase::Playing));
        assert_eq!(st.get(), &Phase::Playing);
        assert_eq!(st.previous(), Some(&Phase::Menu));
        assert!(st.pending().is_none());
    }

    #[test]
    fn applying_same_state_is_noop() {
        let mut st = States::new(Phase::Menu);
        st.set(Phase::Menu);
        let pending = st.take_pending().unwrap();
        assert!(st.apply(pending).is_none());
        assert!(st.previous().is_none());
    }

    #[test]
    fn later_queue_replaces_earlier_pending() {
        let mut st = States::new(Phase::Menu);
        st.set(Phase::Playing);
        st.set(Phase::Paused);
        assert_eq!(st.pending(), Some(&Phase::Paused));
    }

    #[test]
    fn init_state_registers_and_flushes_with_events() {
        let mut app = App::new();
        app.init_state(Phase::Menu);
        assert_eq!(app.state::<Phase>().unwrap(), &Phase::Menu);
        assert!(!app.in_state(&Phase::Playing));

        app.set_state(Phase::Playing).unwrap();
        // Not applied until the frame ticks.
        assert_eq!(app.state::<Phase>().unwrap(), &Phase::Menu);
        app.tick_frame(1.0 / 60.0);
        assert_eq!(app.state::<Phase>().unwrap(), &Phase::Playing);
        assert!(app.in_state(&Phase::Playing));
        assert_eq!(
            app.state_container::<Phase>().unwrap().previous(),
            Some(&Phase::Menu)
        );

        // tick_frame ends with events.update(), so the transition is already
        // in the previous buffer a fresh reader sees.
        let mut reader = app.events_mut().reader::<StateTransitioned<Phase>>();
        assert_eq!(
            reader.read(),
            vec![StateTransitioned {
                from: Phase::Menu,
                to: Phase::Playing,
            }]
        );
    }

    #[test]
    fn independent_state_types_flush_separately() {
        let mut app = App::new();
        app.init_state(Phase::Menu);
        app.init_state(Net::Offline);

        app.set_state(Phase::Paused).unwrap();
        app.set_state(Net::Online).unwrap();
        app.tick_frame(1.0 / 60.0);

        assert_eq!(app.state::<Phase>().unwrap(), &Phase::Paused);
        assert_eq!(app.state::<Net>().unwrap(), &Net::Online);
    }

    #[test]
    fn set_state_without_init_errors() {
        let mut app = App::new();
        let err = app.set_state(Phase::Menu).unwrap_err();
        assert!(err.to_string().contains("not initialized"));
    }

    #[test]
    fn re_init_keeps_existing_state() {
        let mut app = App::new();
        app.init_state(Phase::Menu);
        app.set_state(Phase::Playing).unwrap();
        app.tick_frame(1.0 / 60.0);
        app.init_state(Phase::Menu);
        assert_eq!(app.state::<Phase>().unwrap(), &Phase::Playing);
    }
}
