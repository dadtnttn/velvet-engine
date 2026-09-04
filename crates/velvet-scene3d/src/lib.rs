//! Declarative 3D scene documents (`Scene3D`).
//!
//! Authors describe a level as JSON: a node tree (primitives or `.glb`
//! references with TRS transforms), a camera, lights, and environment.
//! The document is renderer-agnostic (plain `[f32; 3]` vectors, degrees
//! for rotations); [`flatten::flatten_scene`] resolves world matrices
//! with `velvet-math`, and [`assets::check_scene_assets`] verifies every
//! referenced `.glb`/`.gltf` exists before play/export.

#![deny(missing_docs)]

pub mod anim;
pub mod assets;
pub mod doc;
pub mod flatten;
pub mod load;
pub mod particles;
pub mod physics;
pub mod save;

pub use anim::{
    normalize_quat, quat_nlerp, sample_clip, sample_track_value, AnimClip3D, AnimLayer3D,
    AnimMixer3D, AnimTrack3D, Channel3D, NodeSample3D, TrackSample, IDENTITY_QUAT,
};
pub use assets::{check_scene_assets, AssetIssue3D, AssetReport3D};
pub use doc::{
    CameraDoc, EnvironmentDoc, LightDoc, MaterialDoc, MeshKind3D, Scene3D, SceneNode3D,
    TransformDoc,
};
pub use flatten::{flatten_scene, local_matrix, FlatNode3D};
pub use load::{load_scene_from_path, load_scene_from_str, validate_scene, SceneError};
pub use particles::{Emitter3D, Particle3D, ParticleSystem3D};
pub use physics::{
    bake_colliders, raycast, transform_aabb, transform_point, Aabb3D, CharacterController3D,
    Collider3D, PhysicsWorld3D, RayHit3D,
};
pub use save::{BodySave3D, SceneSave3D};
