//! # velvet-play
//!
//! Shared 2D gameplay: tilemaps, kinematics, collisions, triggers, camera follow,
//! interaction, particles, path following, debug draw, simple AI utilities, and
//! checkpoints.

#![deny(missing_docs)]

mod ai;
mod animation;
mod camera;
mod camera_fx;
mod checkpoint;
mod collider;
mod components;
mod debug_draw;
mod interaction;
mod map;
mod navigation;
mod particles;
mod path;
mod physics;
mod plugin;
mod regions;
mod tile_collision;
mod triggers;
mod world;

pub mod prelude;

pub use ai::{AiBudget, BehaviorState, StateMachine, Steering};
pub use animation::{AnimClip, AnimPlayer, AnimState};
pub use camera::{Camera2dFollow, CameraBounds, PlayCamera};
pub use camera_fx::{CameraShake, CameraTraumaLayers, ZoomPunch};
pub use checkpoint::{Checkpoint, CheckpointId, CheckpointStore};
pub use collider::{Collider, ColliderShape, CollisionLayer, CollisionMask};
pub use components::{
    Facing, Health, Interactable, KinematicBody, PlayerTag, Solid, Speed, Trigger, Velocity,
};
pub use debug_draw::{DebugCircle, DebugDraw, DebugLine, DebugPrim, DebugRect};
pub use interaction::{InteractEvent, InteractionSystem};
pub use map::{
    apply_autotile4, autotile_mask4, flood_fill_walkable, solid_neighbor_count, AutotileMask, Tile,
    TileFlags, TileLayer, TileMap, TileMapError,
};
pub use navigation::{astar, smooth_path, GridNav, HierarchicalNav, NavPoint, Path};
pub use particles::{EmitterConfig, EmitterShape, Particle, ParticleEmitter, ParticleRng};
pub use path::{PathFollowResult, PathFollower, PathLoop};
pub use physics::{
    collide_aabb, move_and_collide, raycast, resolve_penetration, CastHit, CollisionHit,
    PhysicsWorld, Ray,
};
pub use plugin::PlayPlugin;
pub use regions::{MapRegion, RegionEvent, RegionEventKind, RegionSet};
pub use tile_collision::{
    aabb_from_center, move_vs_tiles, overlaps_solid, resolve_tile_penetration, solid_contacts,
    SolidContacts, TileCollisionResult,
};
pub use triggers::{
    VolumeTrigger, VolumeTriggerEvent, VolumeTriggerEventKind, VolumeTriggerSystem,
};
pub use world::{PlayWorld, PlayWorldConfig};
