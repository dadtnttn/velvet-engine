//! # velvet-scene
//!
//! Scene management: load/unload, additive scenes, prefabs, hierarchy tags.

#![deny(missing_docs)]

mod async_load;
mod hierarchy;
mod manager;
mod prefab;
mod prefab_variants;
mod scene;
mod serde_components;
mod transition_graph;

pub mod prelude;

pub use async_load::{AsyncLoadError, AsyncSceneLoader, LoadJob, LoadPhase};
pub use hierarchy::{
    ancestors, attach_child, detach_child, find_by_name, walk_descendants, Children, Name, Parent,
};
pub use manager::{SceneBlueprint, SceneEvent, SceneManager, SceneManagerError};
pub use prefab::{
    Prefab, PrefabComponent, PrefabError, PrefabId, PrefabLibrary, PrefabNode, PrefabValue,
};
pub use prefab_variants::{
    difficulty_variant, offset_variant, skin_variant, PrefabVariant, VariantError, VariantLibrary,
    VariantPatch,
};
pub use scene::{Scene, SceneId, SceneState};
pub use transition_graph::{
    eval_condition, MapTransitionContext, SceneEdge, SceneNode, SceneTransitionGraph,
    TransitionCondition, TransitionContext, TransitionEffect, TransitionGraphError,
};
