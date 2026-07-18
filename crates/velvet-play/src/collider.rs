//! Collider shapes and layers.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

bitflags! {
    /// Collision layer bits.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct CollisionLayer: u32 {
        /// Default solid world.
        const WORLD = 1 << 0;
        /// Player.
        const PLAYER = 1 << 1;
        /// Enemy.
        const ENEMY = 1 << 2;
        /// Projectile.
        const PROJECTILE = 1 << 3;
        /// Trigger / sensor.
        const TRIGGER = 1 << 4;
        /// Interactable.
        const INTERACT = 1 << 5;
        /// All layers.
        const ALL = u32::MAX;
    }
}

/// Mask of layers this collider collides with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionMask(pub u32);

impl Default for CollisionMask {
    fn default() -> Self {
        Self(CollisionLayer::ALL.bits())
    }
}

impl CollisionMask {
    /// Create from layers.
    pub fn from_layers(layers: CollisionLayer) -> Self {
        Self(layers.bits())
    }

    /// Whether intersects other layer.
    pub fn hits(self, layer: CollisionLayer) -> bool {
        self.0 & layer.bits() != 0
    }
}

/// Collider shape in local space (centered on entity transform).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColliderShape {
    /// Axis-aligned box half-extents.
    Aabb {
        /// Half size.
        half: Vec2,
    },
    /// Circle radius.
    Circle {
        /// Radius.
        radius: f32,
    },
}

impl ColliderShape {
    /// AABB from full size.
    pub fn box_size(size: Vec2) -> Self {
        Self::Aabb { half: size * 0.5 }
    }

    /// Circle.
    pub fn circle(radius: f32) -> Self {
        Self::Circle { radius }
    }
}

/// Collider component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collider {
    /// Shape.
    pub shape: ColliderShape,
    /// Layer this body belongs to.
    pub layer: CollisionLayer,
    /// Layers this body collides with.
    pub mask: CollisionMask,
    /// Sensor: generates events, no solid resolution.
    pub is_sensor: bool,
    /// Local offset from transform.
    pub offset: Vec2,
}

impl Collider {
    /// Solid AABB.
    pub fn aabb(half: Vec2) -> Self {
        Self {
            shape: ColliderShape::Aabb { half },
            layer: CollisionLayer::WORLD,
            mask: CollisionMask::default(),
            is_sensor: false,
            offset: Vec2::ZERO,
        }
    }

    /// Solid circle.
    pub fn circle(radius: f32) -> Self {
        Self {
            shape: ColliderShape::Circle { radius },
            layer: CollisionLayer::WORLD,
            mask: CollisionMask::default(),
            is_sensor: false,
            offset: Vec2::ZERO,
        }
    }

    /// Sensor AABB.
    pub fn sensor_aabb(half: Vec2) -> Self {
        Self {
            is_sensor: true,
            layer: CollisionLayer::TRIGGER,
            ..Self::aabb(half)
        }
    }

    /// World-space AABB for broadphase (circle uses diameter box).
    pub fn world_aabb(&self, position: Vec2) -> Rect {
        let p = position + self.offset;
        match self.shape {
            ColliderShape::Aabb { half } => Rect::from_center_half_size(p, half),
            ColliderShape::Circle { radius } => Rect::from_center_half_size(p, Vec2::splat(radius)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_hits() {
        let m = CollisionMask::from_layers(CollisionLayer::PLAYER | CollisionLayer::ENEMY);
        assert!(m.hits(CollisionLayer::PLAYER));
        assert!(!m.hits(CollisionLayer::PROJECTILE));
    }
}
