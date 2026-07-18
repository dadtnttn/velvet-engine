//! High-level author session: world + sim + physics + brush + enemies + particles + spells.

use crate::agent::{AgentInput, AgentWorld};
use crate::brush::{
    apply_preset, default_brush_presets, Brush, BrushMode, BrushPreset, BrushShape,
};
use crate::builtin::{builtin_registry, BuiltinIds};
use crate::cell::{Cell, MaterialId};
use crate::enemy::{register_builtin_enemies, EnemyDef, EnemyWorld};
use crate::events::SimEvent;
use crate::forces::ForceWorld;
use crate::hot_sim::{step_hot, HotChunkTracker};
use crate::material::MaterialRegistry;
use crate::material_catalog::register_catalog_materials;
use crate::particles::{ParticleBurst, ParticleConfig, ParticleEmitter, ParticleWorld};
use crate::physics::PhysicsWorld;
use crate::procgen::{
    cave_smooth, generate_arena, generate_caves, generate_platforms, scatter_blobs, CaveOptions,
};
use crate::render_buf::{render_world_window, ColorBuffer};
use crate::rules::splatter_blood;
use crate::save::{SaveError, WorldSave};
use crate::sim::{step, SimConfig};
use crate::spells::{register_builtin_spells, SpellBook};
use crate::world::{World, WorldConfig};

/// Creator-facing simulation session (the usual entry point).
pub struct CellularSession {
    /// Cellular world.
    pub world: World,
    /// Rigid-body layer.
    pub physics: PhysicsWorld,
    /// Enemy layer.
    pub enemies: EnemyWorld,
    /// Free particles.
    pub particles: ParticleWorld,
    /// Agents (player bodies).
    pub agents: AgentWorld,
    /// Force fields.
    pub forces: ForceWorld,
    /// Spell book.
    pub spells: SpellBook,
    /// Active brush (editor).
    pub brush: Brush,
    /// Brush presets palette.
    pub presets: Vec<BrushPreset>,
    /// Sim passes config.
    pub sim: SimConfig,
    /// Hot-chunk tracker.
    pub hot: HotChunkTracker,
    /// Use hot-chunk stepping when true.
    pub use_hot: bool,
    /// Builtin ids if using default pack.
    pub builtin: Option<BuiltinIds>,
    /// Fixed timestep accumulator helper (seconds).
    pub time_accum: f32,
    /// Sim hz.
    pub sim_hz: f32,
    /// Pending agent inputs for next step.
    agent_inputs: Vec<(u32, AgentInput)>,
}

impl CellularSession {
    /// New session with full builtin + extended materials + enemies + spells.
    pub fn with_builtins(config: WorldConfig) -> Self {
        let (mut reg, ids) = builtin_registry();
        let _ = register_catalog_materials(&mut reg);
        let mut enemies = EnemyWorld::new();
        register_builtin_enemies(&mut enemies);
        let mut spells = SpellBook::new();
        register_builtin_spells(&mut spells);
        let mut brush = Brush::default().with_material(ids.sand);
        brush.radius = 3;
        Self {
            world: World::new(reg, config),
            physics: PhysicsWorld::new(),
            enemies,
            particles: ParticleWorld::new(ParticleConfig::default()),
            agents: AgentWorld::new(),
            forces: ForceWorld::new(),
            spells,
            brush,
            presets: default_brush_presets(),
            sim: SimConfig::default(),
            hot: HotChunkTracker::new(),
            use_hot: true,
            builtin: Some(ids),
            time_accum: 0.0,
            sim_hz: 60.0,
            agent_inputs: Vec::new(),
        }
    }

    /// Empty materials (air only).
    pub fn empty(config: WorldConfig) -> Self {
        Self {
            world: World::new(MaterialRegistry::new(), config),
            physics: PhysicsWorld::new(),
            enemies: EnemyWorld::new(),
            particles: ParticleWorld::default(),
            agents: AgentWorld::new(),
            forces: ForceWorld::new(),
            spells: SpellBook::new(),
            brush: Brush::default(),
            presets: default_brush_presets(),
            sim: SimConfig::default(),
            hot: HotChunkTracker::new(),
            use_hot: false,
            builtin: None,
            time_accum: 0.0,
            sim_hz: 60.0,
            agent_inputs: Vec::new(),
        }
    }

    /// Materials registry mut.
    pub fn materials_mut(&mut self) -> &mut MaterialRegistry {
        &mut self.world.materials
    }

    /// Resolve material key.
    pub fn mat(&self, key: &str) -> MaterialId {
        self.world.mat(key)
    }

    /// Paint circle by key.
    pub fn paint(&mut self, x: i32, y: i32, r: i32, material_key: &str) {
        let id = self.mat(material_key);
        self.world.paint_circle(x, y, r, id);
        self.hot.touch(x, y);
    }

    /// Erase.
    pub fn erase(&mut self, x: i32, y: i32, r: i32) {
        self.world.erase_circle(x, y, r);
        self.hot.touch(x, y);
    }

    // —— Brush ——

    /// Select preset.
    pub fn select_preset(&mut self, name: &str) -> bool {
        let name_l = name.to_ascii_lowercase();
        if let Some(p) = self
            .presets
            .iter()
            .find(|p| p.name.to_ascii_lowercase().contains(&name_l))
            .cloned()
        {
            apply_preset(&mut self.brush, &self.world, &p);
            true
        } else {
            false
        }
    }

    /// Brush material by key.
    pub fn brush_material(&mut self, key: &str) {
        self.brush.material = self.mat(key);
        self.brush.mode = BrushMode::Paint;
    }

    /// Brush radius.
    pub fn brush_radius(&mut self, r: i32) {
        self.brush.radius = r.max(0);
    }

    /// Brush shape.
    pub fn brush_shape(&mut self, shape: BrushShape) {
        self.brush.shape = shape;
    }

    /// Brush mode.
    pub fn brush_mode(&mut self, mode: BrushMode) {
        self.brush.mode = mode;
    }

    /// Pointer down.
    pub fn brush_down(&mut self, x: i32, y: i32) -> u32 {
        let n = self.brush.begin_stroke(&mut self.world, x, y);
        self.hot.touch(x, y);
        n
    }

    /// Pointer drag.
    pub fn brush_drag(&mut self, x: i32, y: i32) -> u32 {
        let n = self.brush.move_stroke(&mut self.world, x, y);
        self.hot.touch(x, y);
        n
    }

    /// Pointer up.
    pub fn brush_up(&mut self) {
        self.brush.end_stroke();
    }

    /// Flood fill.
    pub fn brush_flood(&mut self, x: i32, y: i32, max_cells: usize) -> u32 {
        self.hot.touch(x, y);
        self.brush.flood(&mut self.world, x, y, max_cells)
    }

    /// Blood splatter.
    pub fn splatter(&mut self, x: i32, y: i32, radius: i32) {
        splatter_blood(&mut self.world, x, y, radius);
        self.hot.touch(x, y);
    }

    // —— Particles ——

    /// Particle burst of material key.
    pub fn particle_burst(&mut self, x: f32, y: f32, material_key: &str, count: u32) -> u32 {
        let mat = self.mat(material_key);
        let n = self.particles.burst(&ParticleBurst {
            x,
            y,
            material: mat,
            count,
            ..Default::default()
        });
        self.hot.touch(x as i32, y as i32);
        n
    }

    /// Blood particle burst.
    pub fn particle_blood(&mut self, x: f32, y: f32, count: u32) -> u32 {
        let blood = self.mat("blood");
        self.hot.touch(x as i32, y as i32);
        self.particles.burst_blood(x, y, blood, count)
    }

    /// Sparks.
    pub fn particle_sparks(&mut self, x: f32, y: f32, count: u32) -> u32 {
        let fire = self.mat("fire");
        self.particles.burst_sparks(x, y, fire, count)
    }

    /// Add continuous emitter.
    pub fn add_emitter(&mut self, x: f32, y: f32, material_key: &str, rate: f32) -> u32 {
        let mat = self.mat(material_key);
        self.particles
            .add_emitter(ParticleEmitter::new(0, x, y, mat, rate))
    }

    // —— Spells ——

    /// Cast spell by key.
    pub fn cast_spell(&mut self, key: &str, x: f32, y: f32) -> bool {
        let ok = self
            .spells
            .cast(key, &mut self.world, &mut self.particles, x, y);
        if ok {
            self.hot.touch(x as i32, y as i32);
        }
        ok
    }

    // —— Enemies ——

    /// Register enemy.
    pub fn register_enemy(&mut self, def: EnemyDef) {
        self.enemies.register(def);
    }

    /// Spawn enemy.
    pub fn spawn_enemy(&mut self, key: &str, x: f32, y: f32) -> Option<u32> {
        let id = self.enemies.spawn(key, x, y, &mut self.physics)?;
        self.world.events.push(SimEvent::EnemySpawned {
            id,
            def_key: key.into(),
            x,
            y,
        });
        self.hot.touch(x as i32, y as i32);
        Some(id)
    }

    /// Damage enemy.
    pub fn damage_enemy(&mut self, id: u32, amount: f32) -> bool {
        self.enemies
            .damage(id, amount, &mut self.world, &mut self.physics)
    }

    /// Set chase target.
    pub fn set_enemy_target(&mut self, id: Option<u32>, x: f32, y: f32) {
        self.enemies.set_target(id, x, y);
    }

    // —— Agents ——

    /// Spawn player agent.
    pub fn spawn_agent(&mut self, x: f32, y: f32) -> u32 {
        let stone = self.mat("stone");
        let blood = self.mat("blood");
        let id = self.agents.spawn(x, y, &mut self.physics);
        if let Some(a) = self.agents.get_mut(id) {
            a.place_material = stone;
            a.blood_material = blood;
        }
        id
    }

    /// Queue agent input for next step.
    pub fn agent_input(&mut self, id: u32, input: AgentInput) {
        self.agent_inputs.push((id, input));
    }

    // —— Procgen ——

    /// Generate caves.
    pub fn gen_caves(&mut self, mut opt: CaveOptions) {
        if opt.solid.is_air() {
            opt.solid = self.mat("stone");
        }
        if opt.border.is_air() {
            opt.border = self.mat("bedrock");
        }
        generate_caves(&mut self.world, &opt);
        self.hot.touch(opt.x0, opt.y0);
    }

    /// Smooth caves.
    pub fn smooth_caves(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, iterations: u32) {
        let solid = self.mat("stone");
        cave_smooth(&mut self.world, x0, y0, x1, y1, solid, iterations);
    }

    /// Arena.
    pub fn gen_arena(&mut self, cx: i32, floor_y: i32, half_w: i32, height: i32) {
        let wall = self.mat("stone");
        let floor = self.mat("bedrock");
        generate_arena(
            &mut self.world,
            cx,
            floor_y,
            half_w,
            height,
            wall,
            floor,
            true,
        );
    }

    /// Platforms.
    pub fn gen_platforms(&mut self, x0: i32, x1: i32, y0: i32, count: u32, spacing: i32) {
        let m = self.mat("stone");
        let seed = self.world.config.seed;
        generate_platforms(&mut self.world, x0, x1, y0, count, spacing, m, seed);
    }

    /// Scatter blobs.
    pub fn scatter(&mut self, key: &str, count: u32, radius: i32) {
        let m = self.mat(key);
        let seed = self.world.config.seed;
        if let Some((x0, y0, x1, y1)) = self.world.loaded_bounds() {
            scatter_blobs(&mut self.world, x0, y0, x1, y1, m, count, radius, seed ^ 0xABC);
        } else {
            scatter_blobs(&mut self.world, -40, 0, 40, 40, m, count, radius, seed);
        }
    }

    // —— Step ——

    /// One full step: cellular + particles + physics + enemies + agents + forces.
    pub fn step(&mut self) {
        let dt = 1.0 / self.sim_hz.max(1.0);
        if self.use_hot {
            step_hot(&mut self.world, &self.sim, &mut self.hot);
        } else {
            step(&mut self.world, &self.sim);
        }
        self.forces
            .apply(&mut self.world, &mut self.particles, dt);
        self.particles.step(&mut self.world, dt);
        self.physics.step(&mut self.world, dt);
        self.enemies
            .step(&mut self.world, &mut self.physics, dt);
        let inputs = std::mem::take(&mut self.agent_inputs);
        self.agents.step(
            &mut self.world,
            &mut self.physics,
            &mut self.particles,
            &inputs,
            dt,
        );
        // touch hot where particles converted
        for p in self.particles.particles.iter().filter(|p| p.alive).take(32) {
            self.hot.touch(p.x as i32, p.y as i32);
        }
    }

    /// Step n times.
    pub fn step_n(&mut self, n: u32) {
        for _ in 0..n {
            self.step();
        }
    }

    /// Fixed-timestep pump.
    pub fn tick(&mut self, frame_dt: f32) {
        let step_dt = 1.0 / self.sim_hz.max(1.0);
        self.time_accum += frame_dt;
        let mut guard = 0;
        while self.time_accum >= step_dt && guard < 8 {
            self.step();
            self.time_accum -= step_dt;
            guard += 1;
        }
    }

    /// Drain events.
    pub fn drain_events(&mut self) -> Vec<SimEvent> {
        self.world.events.drain()
    }

    /// Render window (+ particle stamp).
    pub fn render(&self, origin_x: i32, origin_y: i32, w: u32, h: u32) -> ColorBuffer {
        let mut buf = render_world_window(&self.world, origin_x, origin_y, w, h);
        self.particles
            .stamp_into_buffer(&mut buf, origin_x, origin_y, &self.world);
        buf
    }

    /// Live particle count.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Save.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), SaveError> {
        let mut snap = WorldSave::capture(&self.world, Some(&self.physics));
        snap.enemies = Some(self.enemies.clone());
        snap.brush = Some(self.brush.clone());
        snap.particles = Some(self.particles.clone());
        snap.write_path(path)
    }

    /// Load.
    pub fn load(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), SaveError> {
        let snap = WorldSave::read_path(path)?;
        let enemies = snap.enemies.clone();
        let brush = snap.brush.clone();
        let particles = snap.particles.clone();
        let (w, p) = snap.restore()?;
        self.world = w;
        if let Some(ph) = p {
            self.physics = ph;
        }
        if let Some(e) = enemies {
            self.enemies = e;
        }
        if let Some(b) = brush {
            self.brush = b;
        }
        if let Some(pw) = particles {
            self.particles = pw;
        }
        Ok(())
    }

    /// Seed demo platform.
    pub fn seed_demo_platform(&mut self) {
        let stone = self.mat("stone");
        let sand = self.mat("sand");
        let water = self.mat("water");
        let bed = self.mat("bedrock");
        self.world.paint_rect(-40, -5, 40, 0, bed);
        self.world.paint_rect(-20, 5, 20, 7, stone);
        self.world.paint_circle(-8, 15, 5, sand);
        self.world.paint_rect(5, 8, 18, 14, water);
        self.hot.touch(0, 10);
    }

    /// Get cell.
    pub fn get(&self, x: i32, y: i32) -> Cell {
        self.world.get(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_buf::opaque_pixel_count;

    #[test]
    fn sand_falls_onto_floor() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.use_hot = false;
        let sand = s.mat("sand");
        let bed = s.mat("bedrock");
        s.world.paint_rect(-5, 0, 5, 1, bed);
        s.world.set(0, 20, Cell::of(sand));
        s.step_n(40);
        let mut found_y = None;
        for y in 0..25 {
            if s.get(0, y).material == sand {
                found_y = Some(y);
            }
        }
        let y = found_y.expect("sand exists");
        assert!(y <= 2, "sand should rest near floor, y={y}");
    }

    #[test]
    fn water_spreads_on_floor() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.use_hot = false;
        let water = s.mat("water");
        let bed = s.mat("bedrock");
        s.world.paint_rect(-15, 0, 15, 1, bed);
        s.world.paint_rect(0, 1, 1, 8, water);
        s.step_n(80);
        let mut span = 0;
        for x in -15..15 {
            if s.get(x, 1).material == water {
                span += 1;
            }
        }
        assert!(span > 3, "water should spread, span={span}");
    }

    #[test]
    fn rigid_body_falls_on_grid() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        let bed = s.mat("bedrock");
        s.world.paint_rect(-10, 0, 10, 2, bed);
        let id = s.physics.spawn_dynamic(0.0, 30.0, 2.0, 2.0, 1.0);
        for _ in 0..120 {
            s.step();
        }
        let b = s.physics.get(id).unwrap();
        assert!(b.y < 15.0, "body should fall, y={}", b.y);
        assert!(b.y >= 2.0, "body should rest above floor, y={}", b.y);
    }

    #[test]
    fn render_buffer_has_pixels() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.seed_demo_platform();
        let buf = s.render(-32, -8, 64, 48);
        assert!(opaque_pixel_count(&buf) > 50);
    }

    #[test]
    fn author_can_register_custom_material() {
        let mut s = CellularSession::empty(WorldConfig::default());
        use crate::material::{MaterialDef, Phase};
        let id = s
            .materials_mut()
            .register(
                MaterialDef::new("goo", "Goo", Phase::Liquid)
                    .density(1.3)
                    .color(0, 255, 0, 255),
            )
            .unwrap();
        s.world.set(0, 5, Cell::of(id));
        assert_eq!(s.get(0, 5).material, id);
    }

    #[test]
    fn brush_stroke_and_preset() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        assert!(s.select_preset("Sand"));
        let n = s.brush_down(0, 10);
        assert!(n > 0);
        s.brush_drag(5, 10);
        s.brush_up();
        assert_eq!(s.get(0, 10).material, s.mat("sand"));
    }

    #[test]
    fn enemy_spawn_and_ai_step() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.seed_demo_platform();
        let id = s.spawn_enemy("slime", 0.0, 12.0).unwrap();
        s.step_n(10);
        assert!(s.enemies.get(id).is_some());
        assert!(s.damage_enemy(id, 999.0));
        assert_eq!(s.enemies.alive_count(), 0);
    }

    #[test]
    fn blood_splatter_exists() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.splatter(0, 5, 3);
        let blood = s.mat("blood");
        assert!(!blood.is_air());
        assert_eq!(s.get(0, 5).material, blood);
    }

    #[test]
    fn particle_burst_and_spell() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.world.paint_rect(-10, 0, 10, 2, s.mat("bedrock"));
        let n = s.particle_burst(0.0, 12.0, "sand", 20);
        assert!(n > 0);
        assert!(s.particle_count() > 0);
        assert!(s.cast_spell("spark_bolt", 2.0, 10.0));
        s.step_n(40);
        // particles converted or still flying; world should have activity
        assert!(s.world.occupied_cells() > 10 || s.particle_count() > 0);
    }

    #[test]
    fn multi_material_fall_burn_dig() {
        let mut s = CellularSession::with_builtins(WorldConfig::default());
        s.use_hot = false;
        let bed = s.mat("bedrock");
        let sand = s.mat("sand");
        let wood = s.mat("wood");
        let fire = s.mat("fire");
        let stone = s.mat("stone");
        s.world.paint_rect(-8, 0, 8, 1, bed);
        s.world.set(0, 12, Cell::of(sand));
        s.world.set(3, 3, Cell::of(wood));
        s.world.set(3, 4, Cell::of(fire).with_life(20).with_temp(900.0));
        s.world.paint_rect(-4, 1, -1, 4, stone);
        let before_stone = s.world.occupied_cells();
        s.step_n(30);
        // dig stone
        crate::agent::dig_at(&mut s.world, &mut s.particles, -2, 2, 2);
        let after = s.world.occupied_cells();
        // sand fell or fire/wood reacted or dig removed cells
        assert!(
            after != before_stone
                || s.get(0, 1).material == sand
                || s.get(0, 2).material == sand
                || s.particles.conversions > 0
                || s.get(3, 3).material != wood,
            "expected multi-material scene to change"
        );
    }
}
