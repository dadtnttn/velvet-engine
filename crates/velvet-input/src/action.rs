//! Named actions.

use serde::{Deserialize, Serialize};

/// Stable action identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

impl ActionId {
    /// Create from string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow raw id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ActionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ActionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Digital/analog value for an action.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ActionValue {
    /// Button-like.
    Bool(bool),
    /// Axis 1D in roughly `[-1, 1]`.
    Axis1(f32),
    /// Axis 2D.
    Axis2 {
        /// X.
        x: f32,
        /// Y.
        y: f32,
    },
}

impl ActionValue {
    /// Interpret as pressed if bool true or magnitude above threshold.
    pub fn is_active(self, threshold: f32) -> bool {
        match self {
            Self::Bool(b) => b,
            Self::Axis1(v) => v.abs() >= threshold,
            Self::Axis2 { x, y } => (x * x + y * y).sqrt() >= threshold,
        }
    }

    /// As bool.
    pub fn as_bool(self) -> bool {
        self.is_active(0.5)
    }

    /// As axis1.
    pub fn as_axis1(self) -> f32 {
        match self {
            Self::Bool(b) => {
                if b {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Axis1(v) => v,
            Self::Axis2 { x, .. } => x,
        }
    }

    /// As axis2.
    pub fn as_axis2(self) -> (f32, f32) {
        match self {
            Self::Bool(b) => {
                if b {
                    (1.0, 0.0)
                } else {
                    (0.0, 0.0)
                }
            }
            Self::Axis1(v) => (v, 0.0),
            Self::Axis2 { x, y } => (x, y),
        }
    }
}

/// Per-action edge state for a frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ActionState {
    /// Value this frame.
    pub value: Option<ActionValue>,
    /// Was active last frame.
    pub was_active: bool,
    /// Is active this frame.
    pub active: bool,
    /// Just pressed.
    pub just_pressed: bool,
    /// Just released.
    pub just_released: bool,
}

impl ActionState {
    /// Update from a new value.
    pub fn update(&mut self, value: ActionValue, threshold: f32) {
        self.was_active = self.active;
        self.value = Some(value);
        self.active = value.is_active(threshold);
        self.just_pressed = self.active && !self.was_active;
        self.just_released = !self.active && self.was_active;
    }

    /// Clear edge flags after frame if desired (kept for multi-system read).
    pub fn end_frame_edges_only(&mut self) {
        // Edges remain valid until next update.
    }

    /// Magnitude of the action value (0 for inactive bool).
    pub fn magnitude(&self) -> f32 {
        match self.value {
            Some(ActionValue::Bool(b)) => {
                if b {
                    1.0
                } else {
                    0.0
                }
            }
            Some(ActionValue::Axis1(v)) => v.abs(),
            Some(ActionValue::Axis2 { x, y }) => (x * x + y * y).sqrt(),
            None => 0.0,
        }
    }
}

/// Map of action id → bindings helper (for remapping UIs).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionMap {
    /// Ordered action names registered for the game.
    pub actions: Vec<ActionId>,
}

impl ActionMap {
    /// Empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an action id if missing.
    pub fn register(&mut self, id: impl Into<ActionId>) {
        let id = id.into();
        if !self.actions.iter().any(|a| a == &id) {
            self.actions.push(id);
        }
    }

    /// Whether action is registered.
    pub fn contains(&self, id: &ActionId) -> bool {
        self.actions.iter().any(|a| a == id)
    }

    /// All registered actions.
    pub fn iter(&self) -> impl Iterator<Item = &ActionId> + '_ {
        self.actions.iter()
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Register a standard set of gameplay actions.
    pub fn with_gameplay_defaults(mut self) -> Self {
        for name in [
            crate::builtin::CONFIRM,
            crate::builtin::CANCEL,
            crate::builtin::MOVE,
            crate::builtin::ATTACK,
            crate::builtin::INTERACT,
            crate::builtin::OPEN_MENU,
        ] {
            self.register(name);
        }
        self
    }
}
