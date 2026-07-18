//! Prelude for cellular authors.

pub use crate::agent::{dig_at, Agent, AgentInput, AgentWorld};
pub use crate::brush::{
    apply_preset, default_brush_presets, Brush, BrushMode, BrushPreset, BrushShape,
};
pub use crate::builtin::{builtin_registry, register_builtin_materials, BuiltinIds};
pub use crate::cell::{Cell, CellFlags, MaterialId};
pub use crate::chunk::{Chunk, ChunkCoord, CHUNK_CELLS, CHUNK_SIZE};
pub use crate::chunk_codec::{
    compress_chunk, decompress_chunk, roundtrip_ok, ChunkRle,
};
pub use crate::electricity::{find_conductive_path, try_arc};
pub use crate::enemy::{
    enemy_at, register_builtin_enemies, Enemy, EnemyAi, EnemyBodyKind, EnemyDef, EnemyWorld,
};
pub use crate::events::{EventQueue, SimEvent};
pub use crate::fluid::{fluid_pass, FluidPassStats};
pub use crate::forces::{
    gravity_well, heat_zone, wind_field, FieldKind, FieldShape, ForceField, ForceWorld,
};
pub use crate::growth::{growth_pass, plant_seed, GrowthConfig};
pub use crate::hot_sim::{fill_perf_scene, step_hot, timed_steps, HotChunkTracker};
pub use crate::lighting::{average_light, bake_light, LightMap};
pub use crate::material::{
    MaterialDef, MaterialError, MaterialRegistry, Phase, PhysicalProps, ReactionProps,
};
pub use crate::material_catalog::{
    catalog_keys, register_catalog_materials, CATALOG_ROW_COUNT,
};
pub use crate::material_io::{
    export_material_pack, load_material_pack, write_material_pack, MaterialIoError, MaterialPack,
};
pub use crate::particle_presets::{
    all_presets, attach_emitter_preset, play_preset, ParticlePreset,
};
pub use crate::particles::{
    FreeParticle, ParticleBurst, ParticleConfig, ParticleEmitter, ParticleEnd, ParticleWorld,
};
pub use crate::pathfind::astar;
pub use crate::physics::{excavate_under_body, PhysicsWorld, RigidBody};
pub use crate::procgen::{
    cave_smooth, generate_arena, generate_caves, generate_platforms, scatter_blobs, CaveOptions,
};
pub use crate::render_buf::{opaque_pixel_count, render_chunk, render_world_window, ColorBuffer};
pub use crate::replay::{play_log, world_fingerprint, ReplayAction, ReplayLog};
pub use crate::rules::splatter_blood;
pub use crate::save::{load_world, save_world, SaveError, WorldSave};
pub use crate::session::CellularSession;
pub use crate::sim::{step, step_chunks, step_n, SimConfig};
pub use crate::spatial::SpatialHash;
pub use crate::spells::{
    cast_recipe, register_builtin_spells, SpellBook, SpellEffect, SpellRecipe, BUILTIN_SPELL_COUNT,
};
pub use crate::wand::{cast_wand, starter_wands, Wand, WandModifier};
pub use crate::world::{World, WorldConfig};
pub use crate::world_query::{histogram, phase_counts, MaterialHistogram};
