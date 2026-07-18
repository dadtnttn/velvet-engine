//! Volume triggers: AABB enter/exit event tracking.

use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

/// Kind of trigger event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeTriggerEventKind {
    /// Entity entered the volume.
    Enter,
    /// Entity exited the volume.
    Exit,
}

/// One enter/exit event produced this frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeTriggerEvent {
    /// Trigger volume id.
    pub trigger_id: String,
    /// Entity that entered/exited.
    pub entity_id: usize,
    /// Enter or exit.
    pub kind: VolumeTriggerEventKind,
}

/// An axis-aligned volume that tracks overlapping entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolumeTrigger {
    /// Stable id.
    pub id: String,
    /// World-space AABB.
    pub bounds: Rect,
    /// Fire only on first enter until all leave (edge-triggered once).
    pub once: bool,
    /// Whether once-mode has already fired enter.
    pub once_fired: bool,
    /// Enabled.
    pub enabled: bool,
    /// Entities currently inside.
    inside: Vec<usize>,
}

impl VolumeTrigger {
    /// Create a volume.
    pub fn new(id: impl Into<String>, bounds: Rect) -> Self {
        Self {
            id: id.into(),
            bounds,
            once: false,
            once_fired: false,
            enabled: true,
            inside: Vec::new(),
        }
    }

    /// One-shot enter.
    pub fn once(id: impl Into<String>, bounds: Rect) -> Self {
        Self {
            once: true,
            ..Self::new(id, bounds)
        }
    }

    /// Entities currently overlapping.
    pub fn inside(&self) -> &[usize] {
        &self.inside
    }

    /// Whether entity is inside.
    pub fn contains_entity(&self, entity_id: usize) -> bool {
        self.inside.contains(&entity_id)
    }

    /// Point overlap test.
    pub fn contains_point(&self, p: Vec2) -> bool {
        self.bounds.contains_point(p)
    }

    /// Reset once flag and occupancy.
    pub fn reset(&mut self) {
        self.once_fired = false;
        self.inside.clear();
    }
}

/// Manager for many volume triggers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeTriggerSystem {
    /// Volumes.
    volumes: Vec<VolumeTrigger>,
}

impl VolumeTriggerSystem {
    /// Empty system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a volume; returns index.
    pub fn add(&mut self, trigger: VolumeTrigger) -> usize {
        self.volumes.push(trigger);
        self.volumes.len() - 1
    }

    /// All volumes.
    pub fn volumes(&self) -> &[VolumeTrigger] {
        &self.volumes
    }

    /// Mutable volumes.
    pub fn volumes_mut(&mut self) -> &mut [VolumeTrigger] {
        &mut self.volumes
    }

    /// Get by id.
    pub fn get(&self, id: &str) -> Option<&VolumeTrigger> {
        self.volumes.iter().find(|v| v.id == id)
    }

    /// Get mut by id.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut VolumeTrigger> {
        self.volumes.iter_mut().find(|v| v.id == id)
    }

    /// Update occupancy from entity positions and produce enter/exit events.
    ///
    /// `entities` is `(entity_id, world_position)` pairs. Points inside the AABB count as overlap.
    pub fn update(&mut self, entities: &[(usize, Vec2)]) -> Vec<VolumeTriggerEvent> {
        let mut events = Vec::new();
        for vol in &mut self.volumes {
            if !vol.enabled {
                continue;
            }
            let mut now_inside = Vec::new();
            for (id, pos) in entities {
                if vol.bounds.contains_point(*pos) {
                    now_inside.push(*id);
                }
            }
            now_inside.sort_unstable();
            now_inside.dedup();

            // Enters
            for id in &now_inside {
                if !vol.inside.contains(id) {
                    let allow = if vol.once {
                        if vol.once_fired {
                            false
                        } else {
                            vol.once_fired = true;
                            true
                        }
                    } else {
                        true
                    };
                    if allow {
                        events.push(VolumeTriggerEvent {
                            trigger_id: vol.id.clone(),
                            entity_id: *id,
                            kind: VolumeTriggerEventKind::Enter,
                        });
                    }
                }
            }
            // Exits
            if !vol.once {
                for id in &vol.inside {
                    if !now_inside.contains(id) {
                        events.push(VolumeTriggerEvent {
                            trigger_id: vol.id.clone(),
                            entity_id: *id,
                            kind: VolumeTriggerEventKind::Exit,
                        });
                    }
                }
            } else {
                // once volumes still track exit for occupancy but do not re-enter
                for id in &vol.inside {
                    if !now_inside.contains(id) {
                        events.push(VolumeTriggerEvent {
                            trigger_id: vol.id.clone(),
                            entity_id: *id,
                            kind: VolumeTriggerEventKind::Exit,
                        });
                    }
                }
            }

            vol.inside = now_inside;
        }
        events
    }

    /// Clear all volumes.
    pub fn clear(&mut self) {
        self.volumes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_at(origin: Vec2) -> Rect {
        Rect::from_pos_size(origin, Vec2::splat(10.0))
    }

    #[test]
    fn enter_and_exit() {
        let mut sys = VolumeTriggerSystem::new();
        sys.add(VolumeTrigger::new("door", unit_at(Vec2::ZERO)));
        let e1 = sys.update(&[(1, Vec2::new(5.0, 5.0))]);
        assert_eq!(e1.len(), 1);
        assert_eq!(e1[0].kind, VolumeTriggerEventKind::Enter);
        assert_eq!(e1[0].entity_id, 1);
        let e2 = sys.update(&[(1, Vec2::new(5.0, 5.0))]);
        assert!(e2.is_empty());
        let e3 = sys.update(&[(1, Vec2::new(100.0, 100.0))]);
        assert_eq!(e3.len(), 1);
        assert_eq!(e3[0].kind, VolumeTriggerEventKind::Exit);
    }

    #[test]
    fn once_only_first_enter() {
        let mut sys = VolumeTriggerSystem::new();
        sys.add(VolumeTrigger::once("chest", unit_at(Vec2::ZERO)));
        let _ = sys.update(&[(1, Vec2::new(2.0, 2.0))]);
        let _ = sys.update(&[(1, Vec2::new(50.0, 50.0))]);
        let e = sys.update(&[(1, Vec2::new(2.0, 2.0))]);
        assert!(e.iter().all(|ev| ev.kind != VolumeTriggerEventKind::Enter));
    }

    #[test]
    fn multiple_entities() {
        let mut sys = VolumeTriggerSystem::new();
        sys.add(VolumeTrigger::new(
            "zone",
            Rect::from_pos_size(Vec2::ZERO, Vec2::splat(20.0)),
        ));
        let e = sys.update(&[(1, Vec2::new(1.0, 1.0)), (2, Vec2::new(5.0, 5.0))]);
        assert_eq!(e.len(), 2);
        let vol = sys.get("zone").unwrap();
        assert_eq!(vol.inside().len(), 2);
    }

    #[test]
    fn disabled_skips() {
        let mut sys = VolumeTriggerSystem::new();
        let mut t = VolumeTrigger::new("x", unit_at(Vec2::ZERO));
        t.enabled = false;
        sys.add(t);
        let e = sys.update(&[(1, Vec2::new(1.0, 1.0))]);
        assert!(e.is_empty());
    }
}
