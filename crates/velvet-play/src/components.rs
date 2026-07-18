//! Common gameplay components (ECS-friendly plain data).

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Linear velocity in world units per second.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Velocity {
    /// Velocity vector.
    pub linear: Vec2,
}

impl Velocity {
    /// Create.
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            linear: Vec2::new(x, y),
        }
    }

    /// Zero.
    pub const ZERO: Self = Self { linear: Vec2::ZERO };
}

/// Movement speed scalar (units/sec).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Speed(pub f32);

impl Default for Speed {
    fn default() -> Self {
        Self(120.0)
    }
}

/// Facing direction (unit-ish).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Facing {
    /// Direction.
    pub dir: Vec2,
}

impl Default for Facing {
    fn default() -> Self {
        Self { dir: Vec2::Y }
    }
}

/// Marker: player-controlled entity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTag;

/// Marker: solid (blocks movement).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Solid;

/// Kinematic body (moved by velocity, not forces).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KinematicBody {
    /// Max slope / unused reserved.
    pub slide: bool,
    /// If true, stop on collision instead of sliding.
    pub stop_on_hit: bool,
}

impl Default for KinematicBody {
    fn default() -> Self {
        Self {
            slide: true,
            stop_on_hit: false,
        }
    }
}

/// Health for gameplay entities.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Health {
    /// Current.
    pub current: f32,
    /// Maximum.
    pub maximum: f32,
}

impl Health {
    /// Full health.
    pub fn full(maximum: f32) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }

    /// Apply damage; returns true if died this hit.
    pub fn damage(&mut self, amount: f32) -> bool {
        let was_alive = self.current > 0.0;
        self.current = (self.current - amount).max(0.0);
        was_alive && self.current <= 0.0
    }

    /// Alive.
    pub fn is_alive(self) -> bool {
        self.current > 0.0
    }
}

/// Trigger volume (sensor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    /// Trigger id / message.
    pub id: String,
    /// Only fire once until leave.
    pub once: bool,
    /// Currently overlapping.
    #[serde(default)]
    pub active: bool,
    /// Already fired (for once).
    #[serde(default)]
    pub fired: bool,
}

impl Trigger {
    /// Create.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            once: false,
            active: false,
            fired: false,
        }
    }

    /// One-shot.
    pub fn once(id: impl Into<String>) -> Self {
        Self {
            once: true,
            ..Self::new(id)
        }
    }
}

/// Interactable prompt target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interactable {
    /// Prompt text / action id.
    pub action: String,
    /// Interaction radius.
    pub radius: f32,
    /// Enabled.
    pub enabled: bool,
}

impl Interactable {
    /// Create.
    pub fn new(action: impl Into<String>, radius: f32) -> Self {
        Self {
            action: action.into(),
            radius,
            enabled: true,
        }
    }
}
