//! 3D save snapshots: bodies, clock, and particles.
//!
//! [`SceneSave3D`] captures the dynamic state that a `.scene3d.json`
//! document does not contain (controller positions/velocities, elapsed
//! time, live particles). Static geometry reloads from the document, so
//! saves stay small and forward-compatible: unknown body ids are ignored
//! on apply, missing ones keep their spawn pose.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use velvet_math::Vec3;

use crate::particles::ParticleSystem3D;
use crate::physics::CharacterController3D;

/// One saved dynamic body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySave3D {
    /// Body id (matches scene node / controller key).
    pub id: String,
    /// Feet position.
    pub position: Vec3,
    /// Velocity.
    pub velocity: Vec3,
}

/// Serializable 3D save snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneSave3D {
    /// Scene document name (for mismatch warnings).
    pub scene: String,
    /// Elapsed play time seconds.
    pub time: f32,
    /// Dynamic bodies.
    #[serde(default)]
    pub bodies: Vec<BodySave3D>,
    /// Live particle system (None = no particles to restore).
    #[serde(default)]
    pub particles: Option<ParticleSystem3D>,
}

impl SceneSave3D {
    /// Capture from live state.
    pub fn capture(
        scene: impl Into<String>,
        time: f32,
        bodies: &BTreeMap<String, CharacterController3D>,
        particles: Option<&ParticleSystem3D>,
    ) -> Self {
        Self {
            scene: scene.into(),
            time: time.max(0.0),
            bodies: bodies
                .iter()
                .map(|(id, body)| BodySave3D {
                    id: id.clone(),
                    position: body.position,
                    velocity: body.velocity,
                })
                .collect(),
            particles: particles.cloned(),
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Parse from JSON.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// Apply to live controllers (unknown ids ignored, missing kept).
    ///
    /// Returns the ids that were restored.
    pub fn apply_to(&self, bodies: &mut BTreeMap<String, CharacterController3D>) -> Vec<String> {
        let mut restored = Vec::new();
        for saved in &self.bodies {
            if let Some(body) = bodies.get_mut(&saved.id) {
                body.position = saved.position;
                body.velocity = saved.velocity;
                restored.push(saved.id.clone());
            }
        }
        restored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_apply_roundtrip() {
        let mut bodies = BTreeMap::new();
        bodies.insert(
            "hero".into(),
            CharacterController3D::new(Vec3::new(1.0, 0.0, 2.0), 0.3, 1.7),
        );
        let save = SceneSave3D::capture("plaza", 12.5, &bodies, None);
        let json = save.to_json().unwrap();
        let back = SceneSave3D::from_json(&json).unwrap();
        assert_eq!(back, save);

        // Move away, then restore.
        let mut live = bodies.clone();
        live.get_mut("hero").unwrap().position = Vec3::new(9.0, 9.0, 9.0);
        let restored = back.apply_to(&mut live);
        assert_eq!(restored, vec!["hero".to_string()]);
        assert_eq!(live["hero"].position, Vec3::new(1.0, 0.0, 2.0));

        // Unknown ids are ignored, missing bodies untouched.
        let mut extra = BTreeMap::new();
        extra.insert(
            "npc".into(),
            CharacterController3D::new(Vec3::ZERO, 0.3, 1.7),
        );
        assert!(back.apply_to(&mut extra).is_empty());
        assert_eq!(extra["npc"].position, Vec3::ZERO);
    }
}
