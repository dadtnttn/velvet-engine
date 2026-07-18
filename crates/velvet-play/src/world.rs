//! High-level play world: map + entities + simulation step.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::trace;
use velvet_math::Vec2;

use crate::camera::{Camera2dFollow, CameraBounds, PlayCamera};
use crate::checkpoint::CheckpointStore;
use crate::collider::{Collider, CollisionLayer, CollisionMask};
use crate::components::{Facing, Interactable, KinematicBody, PlayerTag, Speed, Trigger, Velocity};
use crate::interaction::{InteractEvent, InteractionSystem};
use crate::map::TileMap;
use crate::physics::PhysicsWorld;
use velvet_math::Transform2D;

/// World config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayWorldConfig {
    /// Fixed physics dt override (None uses frame dt).
    pub fixed_dt: Option<f32>,
}

/// Simple entity storage for play demos (without full ECS wiring).
#[derive(Debug, Clone)]
pub struct PlayEntity {
    /// Id.
    pub id: usize,
    /// Transform.
    pub transform: Transform2D,
    /// Velocity.
    pub velocity: Velocity,
    /// Optional collider.
    pub collider: Option<Collider>,
    /// Kinematic.
    pub kinematic: Option<KinematicBody>,
    /// Speed.
    pub speed: Option<Speed>,
    /// Facing.
    pub facing: Facing,
    /// Player.
    pub player: bool,
    /// Trigger.
    pub trigger: Option<Trigger>,
    /// Interactable.
    pub interactable: Option<Interactable>,
    /// Alive.
    pub alive: bool,
}

impl PlayEntity {
    /// Position helper.
    pub fn position(&self) -> Vec2 {
        self.transform.translation
    }
}

/// Trigger enter/exit events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerEvent {
    /// Trigger entity id.
    pub trigger_id: usize,
    /// Trigger string id.
    pub trigger_name: String,
    /// Other entity id.
    pub other_id: usize,
    /// Enter (true) or exit (false).
    pub entered: bool,
}

/// Play simulation world.
#[derive(Debug)]
pub struct PlayWorld {
    /// Config.
    pub config: PlayWorldConfig,
    /// Tilemap.
    pub map: TileMap,
    /// Entities.
    pub entities: HashMap<usize, PlayEntity>,
    next_id: usize,
    /// Camera.
    pub camera: PlayCamera,
    /// Follow settings.
    pub follow: Camera2dFollow,
    /// Checkpoints.
    pub checkpoints: CheckpointStore,
    /// Physics scratch.
    physics: PhysicsWorld,
    /// Trigger events this frame.
    pub trigger_events: Vec<TriggerEvent>,
    /// Interact events.
    pub interact_events: Vec<InteractEvent>,
    /// Paused.
    pub paused: bool,
}

impl PlayWorld {
    /// Create with map.
    pub fn new(map: TileMap) -> Self {
        let bounds = CameraBounds {
            rect: velvet_math::Rect::from_pos_size(
                Vec2::ZERO,
                Vec2::new(map.world_width(), map.world_height()),
            ),
        };
        let camera = PlayCamera {
            bounds: Some(bounds),
            ..Default::default()
        };
        Self {
            config: PlayWorldConfig::default(),
            map,
            entities: HashMap::new(),
            next_id: 1,
            camera,
            follow: Camera2dFollow::default(),
            checkpoints: CheckpointStore::default(),
            physics: PhysicsWorld::default(),
            trigger_events: Vec::new(),
            interact_events: Vec::new(),
            paused: false,
        }
    }

    /// Spawn entity; returns id.
    pub fn spawn(&mut self, mut e: PlayEntity) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        e.id = id;
        self.entities.insert(id, e);
        id
    }

    /// Spawn player at position.
    pub fn spawn_player(&mut self, position: Vec2, speed: f32) -> usize {
        self.spawn(PlayEntity {
            id: 0,
            transform: Transform2D::from_translation(position),
            velocity: Velocity::ZERO,
            collider: Some(Collider {
                layer: CollisionLayer::PLAYER,
                mask: CollisionMask::from_layers(CollisionLayer::WORLD | CollisionLayer::TRIGGER),
                ..Collider::aabb(Vec2::splat(6.0))
            }),
            kinematic: Some(KinematicBody::default()),
            speed: Some(Speed(speed)),
            facing: Facing::default(),
            player: true,
            trigger: None,
            interactable: None,
            alive: true,
        })
    }

    /// Get player entity id if any.
    pub fn player_id(&self) -> Option<usize> {
        self.entities.values().find(|e| e.player).map(|e| e.id)
    }

    /// Set player move intent (-1..=1 axes).
    pub fn set_player_input(&mut self, axis: Vec2) {
        let Some(pid) = self.player_id() else {
            return;
        };
        let e = self.entities.get_mut(&pid).unwrap();
        let speed = e.speed.map(|s| s.0).unwrap_or(120.0);
        let dir = axis.clamp_length_max(1.0);
        e.velocity.linear = dir * speed;
        if dir.length_squared() > 1e-4 {
            e.facing.dir = dir.normalize_or_zero();
        }
    }

    /// Simulate one frame.
    pub fn step(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        let dt = self.config.fixed_dt.unwrap_or(dt).max(0.0);
        self.trigger_events.clear();
        self.interact_events.clear();
        self.rebuild_physics_solids();

        // Move kinematics
        let ids: Vec<usize> = self.entities.keys().copied().collect();
        for id in ids {
            let (vel, kin, col, pos) = {
                let e = match self.entities.get(&id) {
                    Some(e) if e.alive => e,
                    _ => continue,
                };
                (
                    e.velocity.linear,
                    e.kinematic,
                    e.collider.clone(),
                    e.position(),
                )
            };
            if kin.is_none() {
                continue;
            }
            let Some(col) = col else {
                if let Some(e) = self.entities.get_mut(&id) {
                    e.transform.translation += vel * dt;
                }
                continue;
            };
            let slide = kin.map(|k| k.slide).unwrap_or(true);
            let result = self.physics.move_body(id, pos, vel, dt, &col, slide);
            if let Some(e) = self.entities.get_mut(&id) {
                e.transform.translation = result.position;
                if kin.map(|k| k.stop_on_hit).unwrap_or(false) && !result.hits.is_empty() {
                    e.velocity.linear = Vec2::ZERO;
                }
            }
        }

        self.update_triggers();
        self.update_camera(dt);
    }

    fn rebuild_physics_solids(&mut self) {
        self.physics.clear();
        // Tile solids near all movers (full map for small demos)
        let world_rect = velvet_math::Rect::from_pos_size(
            Vec2::ZERO,
            Vec2::new(self.map.world_width(), self.map.world_height()),
        );
        // Use high ids for tiles
        let mut tile_id = 1_000_000usize;
        for (pos, col) in self.map.solid_colliders_in_aabb(world_rect) {
            self.physics.push_solid(tile_id, pos, col);
            tile_id += 1;
        }
        for e in self.entities.values() {
            if !e.alive {
                continue;
            }
            if let Some(col) = &e.collider {
                if !col.is_sensor {
                    self.physics.push_solid(e.id, e.position(), col.clone());
                }
            }
        }
    }

    fn update_triggers(&mut self) {
        let movers: Vec<(usize, Vec2, Collider)> = self
            .entities
            .values()
            .filter(|e| e.alive && e.player)
            .filter_map(|e| e.collider.clone().map(|c| (e.id, e.position(), c)))
            .collect();
        let triggers: Vec<(usize, Vec2, Trigger, Collider)> = self
            .entities
            .values()
            .filter(|e| e.alive && e.trigger.is_some())
            .filter_map(|e| {
                Some((
                    e.id,
                    e.position(),
                    e.trigger.clone()?,
                    e.collider
                        .clone()
                        .unwrap_or_else(|| Collider::sensor_aabb(Vec2::splat(8.0))),
                ))
            })
            .collect();

        for (tid, tpos, mut trig, tcol) in triggers {
            let mut any = false;
            for (oid, opos, ocol) in &movers {
                if crate::physics::overlap(*opos, ocol, tpos, &tcol).is_some() {
                    any = true;
                    if !(trig.active || (trig.once && trig.fired)) {
                        self.trigger_events.push(TriggerEvent {
                            trigger_id: tid,
                            trigger_name: trig.id.clone(),
                            other_id: *oid,
                            entered: true,
                        });
                        trig.fired = true;
                    }
                }
            }
            if trig.active && !any {
                self.trigger_events.push(TriggerEvent {
                    trigger_id: tid,
                    trigger_name: trig.id.clone(),
                    other_id: 0,
                    entered: false,
                });
            }
            if let Some(e) = self.entities.get_mut(&tid) {
                if let Some(t) = &mut e.trigger {
                    t.active = any;
                    t.fired = trig.fired;
                }
            }
        }
    }

    fn update_camera(&mut self, dt: f32) {
        if let Some(pid) = self.player_id() {
            if let Some(p) = self.entities.get(&pid) {
                self.follow
                    .update(&mut self.camera, p.position(), p.velocity.linear, dt);
            }
        }
    }

    /// Try player interact.
    pub fn try_player_interact(&mut self, confirm: bool) {
        let Some(pid) = self.player_id() else {
            return;
        };
        let player_pos = self.entities.get(&pid).map(|e| e.position()).unwrap();
        let cands: Vec<(usize, Vec2, Interactable)> = self
            .entities
            .values()
            .filter(|e| e.alive && e.interactable.is_some())
            .map(|e| (e.id, e.position(), e.interactable.clone().unwrap()))
            .collect();
        let refs: Vec<(usize, Vec2, &Interactable)> =
            cands.iter().map(|(i, p, a)| (*i, *p, a)).collect();
        if let Some(ev) = InteractionSystem::try_interact(player_pos, &refs, confirm) {
            trace!(?ev, "interact");
            self.interact_events.push(ev);
        }
    }
}

// Silence unused import of PlayerTag in this module (used by API consumers).
const _: Option<PlayerTag> = None;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::TileMap;

    #[test]
    fn player_stopped_by_wall() {
        let map = TileMap::from_ascii(
            "\
########
#......#
#......#
########",
            16.0,
        )
        .unwrap();
        let mut world = PlayWorld::new(map);
        let pid = world.spawn_player(Vec2::new(40.0, 40.0), 200.0);
        // Move left into wall
        world.set_player_input(Vec2::new(-1.0, 0.0));
        for _ in 0..30 {
            world.step(1.0 / 60.0);
        }
        let x = world.entities.get(&pid).unwrap().position().x;
        assert!(x > 16.0, "player x={x} should be blocked by left wall");
    }

    #[test]
    fn trigger_fires_on_enter() {
        let map = TileMap::from_ascii("....\n....", 16.0).unwrap();
        let mut world = PlayWorld::new(map);
        let pid = world.spawn_player(Vec2::new(8.0, 8.0), 100.0);
        world.spawn(PlayEntity {
            id: 0,
            transform: Transform2D::from_translation(Vec2::new(48.0, 8.0)),
            velocity: Velocity::ZERO,
            collider: Some(Collider::sensor_aabb(Vec2::splat(12.0))),
            kinematic: None,
            speed: None,
            facing: Facing::default(),
            player: false,
            trigger: Some(Trigger::once("zone_a")),
            interactable: None,
            alive: true,
        });
        world.set_player_input(Vec2::X);
        let mut saw_enter = false;
        for _ in 0..40 {
            world.step(1.0 / 60.0);
            if world
                .trigger_events
                .iter()
                .any(|e| e.trigger_name == "zone_a" && e.entered)
            {
                saw_enter = true;
                break;
            }
        }
        assert!(
            saw_enter,
            "expected trigger enter, player at {:?}",
            world.entities.get(&pid).map(|e| e.position())
        );
    }
}
