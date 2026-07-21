//! # velvet-cellular (**ALPHA**)
//!
//! **Stability: alpha.** Falling-sand / cellular APIs may break between 0.1.0-alpha
//! releases. Not a stable product surface.
//!
//! Fully functional Noita-like **author core**: cellular materials, free particles
//! that convert to grid, spells/wands, agents, fluid/electricity/growth systems,
//! hot-chunk efficiency. No external physics engines. No LOC padding catalogs.
//!
//! ```rust
//! use velvet_cellular::prelude::*;
//!
//! let mut s = CellularSession::with_builtins(WorldConfig::default());
//! s.seed_demo_platform();
//! s.particle_burst(0.0, 20.0, "sand", 24);
//! s.cast_spell("spark_bolt", 2.0, 15.0);
//! s.step_n(30);
//! assert!(s.world.occupied_cells() > 0 || s.particle_count() > 0);
//! ```

#![deny(missing_docs)]

pub mod agent;
pub mod biome;
pub mod brush;
pub mod builtin;
pub mod cell;
pub mod chunk;
pub mod chunk_codec;
pub mod combat;
pub mod electricity;
pub mod enemy;
pub mod events;
pub mod fluid;
pub mod forces;
pub mod growth;
pub mod hot_sim;
pub mod lighting;
pub mod material;
pub mod material_catalog;
pub mod material_io;
pub mod multi_layer;
pub mod particle_presets;
pub mod particles;
pub mod pathfind;
pub mod physics;
pub mod prelude;
pub mod procgen;
pub mod projectile;
pub mod reaction_chain;
pub mod render_buf;
pub mod replay;
pub mod rules;
pub mod save;
pub mod session;
pub mod sim;
pub mod sim_debug;
pub mod spatial;
pub mod spells;
pub mod status;
pub mod wand;
pub mod world;
pub mod world_query;

pub use agent::{dig_at, Agent, AgentInput, AgentWorld};
pub use biome::{stamp_biome, stamp_desert, stamp_forest, BiomeStamp, BiomeStats};
pub use brush::{apply_preset, default_brush_presets, Brush, BrushMode, BrushPreset, BrushShape};
pub use builtin::{builtin_registry, register_builtin_materials, BuiltinIds};
pub use cell::{Cell, CellFlags, MaterialId};
pub use chunk::{Chunk, ChunkCoord, CHUNK_CELLS, CHUNK_SIZE};
pub use chunk_codec::{
    compress_chunk, compress_world_chunks, compression_ratio, decompress_chunk, roundtrip_ok,
    ChunkRle,
};
pub use combat::{fire_hitscan, hitscan, knockback, melee_splash, HitScan};
pub use electricity::{
    diffuse_charge, find_conductive_path, inject_charge, is_conductive, shock_path, try_arc,
};
pub use enemy::{
    enemy_at, register_builtin_enemies, Enemy, EnemyAi, EnemyBodyKind, EnemyDef, EnemyWorld,
};
pub use events::{EventQueue, SimEvent};
pub use fluid::{
    apply_hydrostatic_pressure, count_falling_liquid, drain_liquid, equalize_liquid_columns,
    find_liquid_blobs, fluid_pass, settled_liquid_ratio, try_mix_liquids, FluidPassStats,
    LiquidBlob,
};
pub use forces::{
    gravity_well, heat_zone, wind_field, FieldKind, FieldShape, ForceField, ForceWorld,
};
pub use growth::{growth_pass, plant_seed, GrowthConfig};
pub use hot_sim::{fill_perf_scene, step_hot, timed_steps, HotChunkTracker};
pub use lighting::{average_light, bake_light, cell_opacity, is_emissive, LightMap};
pub use material::{
    MaterialDef, MaterialError, MaterialRegistry, Phase, PhysicalProps, ReactionProps,
};
pub use material_catalog::{catalog_keys, register_catalog_materials, CATALOG_ROW_COUNT};
pub use material_io::{
    export_material_pack, load_material_pack, write_material_pack, MaterialIoError, MaterialPack,
};
pub use multi_layer::{sample_parallax, LayerKind, LayerStack, WorldLayer};
pub use particle_presets::{all_presets, attach_emitter_preset, play_preset, ParticlePreset};
pub use particles::{
    FreeParticle, ParticleBurst, ParticleConfig, ParticleEmitter, ParticleEnd, ParticleWorld,
};
pub use pathfind::{astar, enter_cost, line_clear};
pub use physics::{excavate_under_body, PhysicsWorld, RigidBody};
pub use procgen::{
    cave_smooth, generate_arena, generate_caves, generate_platforms, scatter_blobs, CaveOptions,
};
pub use projectile::{Projectile, ProjectileKind, ProjectileWorld};
pub use reaction_chain::{
    apply_reaction_chains, count_reactive_contacts, extinguish_radius, ReactionRule, CHAIN_RULES,
};
pub use render_buf::{opaque_pixel_count, render_chunk, render_world_window, ColorBuffer};
pub use replay::{
    apply_action, particle_fingerprint, play_log, world_fingerprint, ReplayAction, ReplayLog,
};
pub use rules::splatter_blood;
pub use save::{load_world, save_world, SaveError, WorldSave};
pub use session::CellularSession;
pub use sim::{step, step_chunks, step_n, SimConfig};
pub use sim_debug::{
    active_chunk_ratio, ascii_window, check_integrity, chunk_layout_ok, count_burning,
    material_diversity, IntegrityReport,
};
pub use spatial::{average_bucket_load, separate_particles, HashKey, SpatialHash};
pub use spells::{
    cast_recipe, register_builtin_spells, SpellBook, SpellEffect, SpellRecipe, BUILTIN_SPELL_COUNT,
};
pub use status::{StatusEffect, StatusKind, StatusWorld};
pub use wand::{cast_wand, resolve_wand, starter_wands, CastPlan, Wand, WandModifier, WandSlot};
pub use world::{World, WorldConfig};
pub use world_query::{
    find_material, histogram, nearest_material, non_air_bounds, phase_counts, surface_profile,
    temperature_stats, MaterialHistogram, PhaseCounts, TempStats,
};
