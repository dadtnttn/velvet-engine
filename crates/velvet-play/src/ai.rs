//! Lightweight AI helpers: state machines, steering, update budget.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Named FSM state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BehaviorState(pub String);

impl BehaviorState {
    /// Create.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Simple finite state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMachine {
    /// Current state.
    pub current: BehaviorState,
    /// Allowed transitions: from -> list of to.
    transitions: HashMap<String, Vec<String>>,
}

impl StateMachine {
    /// Create with initial state.
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            current: BehaviorState::new(initial),
            transitions: HashMap::new(),
        }
    }

    /// Allow transition.
    pub fn allow(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.transitions
            .entry(from.into())
            .or_default()
            .push(to.into());
    }

    /// Try transition; returns true if applied.
    pub fn try_transition(&mut self, to: impl Into<String>) -> bool {
        let to = to.into();
        let ok = self
            .transitions
            .get(&self.current.0)
            .map(|v| v.iter().any(|t| t == &to))
            .unwrap_or(false);
        if ok {
            self.current = BehaviorState::new(to);
        }
        ok
    }

    /// Force state (debug / script).
    pub fn force(&mut self, state: impl Into<String>) {
        self.current = BehaviorState::new(state);
    }
}

/// Steering utilities.
pub struct Steering;

impl Steering {
    /// Seek desired velocity.
    pub fn seek(position: Vec2, target: Vec2, max_speed: f32) -> Vec2 {
        (target - position).normalize_or_zero() * max_speed
    }

    /// Flee.
    pub fn flee(position: Vec2, threat: Vec2, max_speed: f32) -> Vec2 {
        (position - threat).normalize_or_zero() * max_speed
    }

    /// Arrive with slowdown radius.
    pub fn arrive(position: Vec2, target: Vec2, max_speed: f32, slow_radius: f32) -> Vec2 {
        let offset = target - position;
        let dist = offset.length();
        if dist < 1e-4 {
            return Vec2::ZERO;
        }
        let speed = if dist < slow_radius {
            max_speed * (dist / slow_radius.max(1e-4))
        } else {
            max_speed
        };
        offset * (speed / dist)
    }
}

/// Distributes AI updates across frames.
#[derive(Debug, Clone)]
pub struct AiBudget {
    /// Max agents processed per tick.
    pub max_per_tick: usize,
    /// Cursor for round-robin.
    cursor: usize,
    /// Time budget seconds (soft).
    pub max_seconds: f32,
}

impl Default for AiBudget {
    fn default() -> Self {
        Self {
            max_per_tick: 16,
            cursor: 0,
            max_seconds: 0.002,
        }
    }
}

impl AiBudget {
    /// Select slice of indices to update this frame.
    pub fn take_indices(&mut self, count: usize) -> Vec<usize> {
        if count == 0 {
            return Vec::new();
        }
        let n = self.max_per_tick.min(count);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push((self.cursor + i) % count);
        }
        self.cursor = (self.cursor + n) % count;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsm_transition() {
        let mut sm = StateMachine::new("idle");
        sm.allow("idle", "chase");
        sm.allow("chase", "idle");
        assert!(sm.try_transition("chase"));
        assert_eq!(sm.current.0, "chase");
        assert!(!sm.try_transition("attack"));
    }

    #[test]
    fn budget_round_robin() {
        let mut b = AiBudget {
            max_per_tick: 2,
            ..Default::default()
        };
        let a = b.take_indices(5);
        let c = b.take_indices(5);
        assert_eq!(a, vec![0, 1]);
        assert_eq!(c, vec![2, 3]);
    }
}
