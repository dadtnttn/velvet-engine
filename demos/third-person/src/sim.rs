//! Headless third-person slice: hero controller, chasing enemy, beacon
//! quest, procedural animation clips, particle bursts, and saves.
//!
//! Pure simulation over `velvet-scene3d` (no window, no GPU): the windowed
//! shell in `main.rs` only renders what this sim computes, and every rule
//! below is covered by unit tests.

use std::collections::BTreeMap;
use std::path::Path;

use velvet_math::Vec3;
use velvet_scene3d::{
    bake_colliders, flatten_scene, AnimClip3D, AnimMixer3D, AnimTrack3D, Channel3D,
    CharacterController3D, ParticleSystem3D, PhysicsWorld3D, Scene3D, SceneSave3D,
};

/// Player input for one tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimInput {
    /// Wish direction on XZ (will be normalized by the caller scale).
    pub move_dir: Vec3,
    /// Speed multiplier.
    pub speed: f32,
    /// Jump pressed this tick.
    pub jump: bool,
}

/// Quest state for the slice (one quest: reach the beacon).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestState {
    /// Beacon not reached yet.
    ReachBeacon,
    /// Beacon reached (burst fired, save written by the shell).
    Done,
}

/// Headless third-person simulation.
pub struct ThirdPersonSim {
    /// Scene document name (for saves).
    pub scene_name: String,
    /// Static collision world baked from the scene.
    pub world: PhysicsWorld3D,
    /// Dynamic bodies by id (`hero`, `enemy`).
    pub bodies: BTreeMap<String, CharacterController3D>,
    /// Beacon world position (quest target).
    pub beacon: Vec3,
    /// Hero spawn (respawn point).
    pub spawn: Vec3,
    /// Hero hit points.
    pub hp: u32,
    /// Max hit points.
    pub max_hp: u32,
    /// Invulnerability seconds remaining.
    pub iframes: f32,
    /// Enemy attack cooldown seconds remaining.
    pub enemy_cooldown: f32,
    /// Quest state.
    pub quest: QuestState,
    /// Elapsed sim time.
    pub time: f32,
    /// Animation mixer (procedural locomotion clips).
    pub mixer: AnimMixer3D,
    /// Particles (goal bursts, hit sparks).
    pub particles: ParticleSystem3D,
}

impl ThirdPersonSim {
    /// Build from a scene document (spawns read from nodes).
    pub fn new(scene: &Scene3D) -> Self {
        let flat = flatten_scene(scene);
        let mut world = PhysicsWorld3D::new();
        world.colliders = bake_colliders(&flat);
        let node_pos = |id: &str| {
            flat.iter()
                .find(|n| n.id == id)
                .map(|n| n.world_position)
                .unwrap_or(Vec3::ZERO)
        };
        let spawn = node_pos("hero_spawn");
        let mut bodies = BTreeMap::new();
        bodies.insert("hero".into(), CharacterController3D::new(spawn, 0.35, 1.7));
        bodies.insert(
            "enemy".into(),
            CharacterController3D::new(node_pos("enemy_spawn"), 0.4, 1.6),
        );
        let mut mixer = AnimMixer3D::new();
        mixer.clips = vec![idle_clip(), run_clip()];
        mixer.play("idle", 0.0);
        Self {
            scene_name: scene.name.clone(),
            world,
            bodies,
            beacon: node_pos("beacon"),
            spawn,
            hp: 3,
            max_hp: 3,
            iframes: 0.0,
            enemy_cooldown: 0.0,
            quest: QuestState::ReachBeacon,
            time: 0.0,
            mixer,
            particles: ParticleSystem3D::new(1234),
        }
    }

    /// Advance simulation by `dt`.
    pub fn tick(&mut self, input: SimInput, dt: f32) {
        let dt = dt.clamp(0.0, 0.05);
        self.time += dt;
        self.iframes = (self.iframes - dt).max(0.0);
        self.enemy_cooldown = (self.enemy_cooldown - dt).max(0.0);

        // Hero locomotion.
        let speed = if input.speed > 0.0 { 6.0 } else { 0.0 };
        let wish = Vec3::new(input.move_dir.x * speed, 0.0, input.move_dir.z * speed);
        let jump = if input.jump { Some(7.5) } else { None };
        if let Some(hero) = self.bodies.get_mut("hero") {
            hero.step_move(&self.world, wish, jump, dt);
        }

        // Enemy chase (XZ toward the hero, attacks in range).
        let hero_pos = self.bodies["hero"].position;
        let enemy_pos = self.bodies["enemy"].position;
        let to_hero = Vec3::new(hero_pos.x - enemy_pos.x, 0.0, hero_pos.z - enemy_pos.z);
        let dist = (to_hero.x * to_hero.x + to_hero.z * to_hero.z).sqrt();
        let chase = if dist > 0.01 && dist < 25.0 {
            Vec3::new(to_hero.x / dist * 3.2, 0.0, to_hero.z / dist * 3.2)
        } else {
            Vec3::ZERO
        };
        if let Some(enemy) = self.bodies.get_mut("enemy") {
            enemy.step_move(&self.world, chase, None, dt);
        }
        if dist < 1.3 && self.enemy_cooldown <= 0.0 && self.iframes <= 0.0 {
            self.hp = self.hp.saturating_sub(1);
            self.iframes = 1.0;
            self.enemy_cooldown = 0.8;
        }
        if self.hp == 0 {
            // Arcade rule: respawn at full health, quest progress kept.
            if let Some(hero) = self.bodies.get_mut("hero") {
                hero.position = self.spawn;
                hero.velocity = Vec3::ZERO;
            }
            self.hp = self.max_hp;
            self.iframes = 1.5;
        }

        // Beacon quest.
        if self.quest == QuestState::ReachBeacon {
            let dx = hero_pos.x - self.beacon.x;
            let dz = hero_pos.z - self.beacon.z;
            if dx * dx + dz * dz < 1.5 * 1.5 {
                self.quest = QuestState::Done;
                self.particles.emitters.push(goal_burst(self.beacon));
                self.particles.burst(0);
            }
        }

        // Locomotion animation: run when moving, idle otherwise.
        let moving = wish.x != 0.0 || wish.z != 0.0;
        let current = self.mixer.layers.first().map(|l| {
            self.mixer
                .clips
                .get(l.clip)
                .map(|c| c.name.as_str())
                .unwrap_or("")
        });
        let want = if moving { "run" } else { "idle" };
        if current != Some(want) {
            self.mixer.play(want, 0.25);
        }
        self.mixer.tick(dt);
        self.particles.tick(dt);
    }

    /// Body world matrix with the locomotion bob applied (cube nodes are
    /// unit-centered, so the matrix lifts by half the body height).
    pub fn body_matrix(&self, id: &str) -> velvet_math::Mat4 {
        use velvet_math::Mat4;
        let body = &self.bodies[id];
        let base = Mat4::from_translation(Vec3::new(
            body.position.x,
            body.position.y + body.height * 0.5,
            body.position.z,
        ));
        let mut bob = Vec3::ZERO;
        if id == "hero" {
            for sample in self.mixer.pose() {
                if sample.node == "hero" {
                    if let Some(t) = sample.translation {
                        bob = t;
                    }
                }
            }
        }
        base * Mat4::from_translation(bob)
    }

    /// Hero world matrix with the locomotion bob applied.
    pub fn hero_matrix(&self) -> velvet_math::Mat4 {
        self.body_matrix("hero")
    }

    /// Capture a save snapshot.
    pub fn save(&self) -> SceneSave3D {
        SceneSave3D::capture(
            &self.scene_name,
            self.time,
            &self.bodies,
            Some(&self.particles),
        )
    }

    /// Write a save file.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        let json = self.save().to_json()?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Restore bodies + clock from a snapshot (quest/hp recomputed by play).
    pub fn load(&mut self, snapshot: &SceneSave3D) {
        snapshot.apply_to(&mut self.bodies);
        self.time = snapshot.time;
        if let Some(particles) = &snapshot.particles {
            self.particles = particles.clone();
        }
    }
}

/// Gentle idle breathing on the hero node.
fn idle_clip() -> AnimClip3D {
    AnimClip3D {
        name: "idle".into(),
        duration: 2.0,
        tracks: vec![(
            "hero".into(),
            Channel3D::Translation,
            AnimTrack3D {
                times: vec![0.0, 1.0, 2.0],
                translations: vec![Vec3::ZERO, Vec3::new(0.0, 0.05, 0.0), Vec3::ZERO],
                rotations: Vec::new(),
                scales: Vec::new(),
            },
        )],
    }
}

/// Run-cycle bob on the hero node.
fn run_clip() -> AnimClip3D {
    AnimClip3D {
        name: "run".into(),
        duration: 0.5,
        tracks: vec![(
            "hero".into(),
            Channel3D::Translation,
            AnimTrack3D {
                times: vec![0.0, 0.25, 0.5],
                translations: vec![Vec3::ZERO, Vec3::new(0.0, 0.12, 0.0), Vec3::ZERO],
                rotations: Vec::new(),
                scales: Vec::new(),
            },
        )],
    }
}

/// Celebration burst at the beacon.
fn goal_burst(at: Vec3) -> velvet_scene3d::Emitter3D {
    velvet_scene3d::Emitter3D {
        origin: at,
        velocity: Vec3::new(0.0, 4.0, 0.0),
        spread: Vec3::new(2.5, 1.5, 2.5),
        rate: 0.0,
        burst_count: 60,
        life: [0.8, 1.6],
        gravity: -6.0,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena() -> Scene3D {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        velvet_scene3d::load_scene_from_path(&root.join("scenes/arena.scene3d.json")).unwrap()
    }

    fn walk_to_beacon(sim: &mut ThirdPersonSim) {
        // Teleport-walk: steer directly at the beacon each tick.
        for _ in 0..600 {
            if sim.quest == QuestState::Done {
                break;
            }
            let h = sim.bodies["hero"].position;
            let d = Vec3::new(sim.beacon.x - h.x, 0.0, sim.beacon.z - h.z);
            let len = (d.x * d.x + d.z * d.z).sqrt().max(1e-6);
            sim.tick(
                SimInput {
                    move_dir: Vec3::new(d.x / len, 0.0, d.z / len),
                    speed: 1.0,
                    jump: false,
                },
                1.0 / 60.0,
            );
        }
    }

    #[test]
    fn loads_arena_scene() {
        let scene = arena();
        assert_eq!(scene.name, "arena");
        assert!(scene.nodes.iter().any(|n| n.id == "beacon"));
        let sim = ThirdPersonSim::new(&scene);
        assert_eq!(sim.hp, 3);
        assert!(sim.world.colliders.len() >= 6);
    }

    #[test]
    fn reaching_beacon_completes_quest_with_burst() {
        let mut sim = ThirdPersonSim::new(&arena());
        // Park the enemy far away so the walk is unobstructed.
        sim.bodies.get_mut("enemy").unwrap().position = Vec3::new(0.0, 0.0, -13.0);
        walk_to_beacon(&mut sim);
        assert_eq!(sim.quest, QuestState::Done);
        assert!(!sim.particles.is_empty());
    }

    #[test]
    fn enemy_chases_hero() {
        let mut sim = ThirdPersonSim::new(&arena());
        let start = {
            let e = sim.bodies["enemy"].position;
            let h = sim.bodies["hero"].position;
            (e.x - h.x).hypot(e.z - h.z)
        };
        for _ in 0..120 {
            sim.tick(SimInput::default(), 1.0 / 60.0);
        }
        let end = {
            let e = sim.bodies["enemy"].position;
            let h = sim.bodies["hero"].position;
            (e.x - h.x).hypot(e.z - h.z)
        };
        assert!(end < start, "start={start} end={end}");
    }

    #[test]
    fn enemy_touch_damages_once_per_iframe_window() {
        let mut sim = ThirdPersonSim::new(&arena());
        let hero_pos = sim.bodies["hero"].position;
        sim.bodies.get_mut("enemy").unwrap().position = hero_pos;
        sim.tick(SimInput::default(), 1.0 / 60.0);
        assert_eq!(sim.hp, 2);
        // Still touching, but iframes block further damage.
        for _ in 0..30 {
            let hero_pos = sim.bodies["hero"].position;
            sim.bodies.get_mut("enemy").unwrap().position = hero_pos;
            sim.tick(SimInput::default(), 1.0 / 60.0);
        }
        assert_eq!(sim.hp, 2);
    }

    #[test]
    fn death_respawns_at_spawn() {
        let mut sim = ThirdPersonSim::new(&arena());
        sim.hp = 1;
        let hero_pos = sim.bodies["hero"].position;
        sim.bodies.get_mut("enemy").unwrap().position = hero_pos;
        sim.tick(SimInput::default(), 1.0 / 60.0);
        assert_eq!(sim.hp, 3);
        assert_eq!(sim.bodies["hero"].position, sim.spawn);
    }

    #[test]
    fn save_load_restores_bodies() {
        let mut sim = ThirdPersonSim::new(&arena());
        for _ in 0..30 {
            sim.tick(
                SimInput {
                    move_dir: Vec3::new(1.0, 0.0, 0.0),
                    speed: 1.0,
                    jump: false,
                },
                1.0 / 60.0,
            );
        }
        let snapshot = sim.save();
        // Serialized form parses back to an equal snapshot.
        let json = snapshot.to_json().unwrap();
        assert!(json.contains("\"scene\""));
        assert!(SceneSave3D::from_json(&json).unwrap() == snapshot);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slot.json");
        sim.save_to(&path).unwrap();
        let mut fresh = ThirdPersonSim::new(&arena());
        assert_ne!(fresh.bodies["hero"].position, sim.bodies["hero"].position);
        let back = SceneSave3D::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, snapshot);
        fresh.load(&back);
        assert_eq!(fresh.bodies["hero"].position, sim.bodies["hero"].position);
        assert_eq!(fresh.time, sim.time);
    }

    #[test]
    fn locomotion_switches_clips() {
        fn clip_name(sim: &ThirdPersonSim) -> String {
            sim.mixer
                .layers
                .first()
                .and_then(|l| sim.mixer.clips.get(l.clip))
                .map(|c| c.name.clone())
                .unwrap_or_default()
        }
        let mut sim = ThirdPersonSim::new(&arena());
        assert_eq!(clip_name(&sim), "idle");
        sim.tick(
            SimInput {
                move_dir: Vec3::new(1.0, 0.0, 0.0),
                speed: 1.0,
                jump: false,
            },
            1.0 / 60.0,
        );
        assert_eq!(clip_name(&sim), "run");
    }

    #[test]
    fn walls_block_hero() {
        let mut sim = ThirdPersonSim::new(&arena());
        // Run west into the wall: the face sits at x=-13.5, so the hero
        // (radius 0.35) must rest at exactly -13.15 — not merely "past -14",
        // which a broken (motionless) controller would also satisfy.
        for _ in 0..600 {
            sim.tick(
                SimInput {
                    move_dir: Vec3::new(-1.0, 0.0, 0.0),
                    speed: 1.0,
                    jump: false,
                },
                1.0 / 60.0,
            );
        }
        let p = sim.bodies["hero"].position;
        assert!((p.x - -13.15).abs() < 0.05, "resting x={}", p.x);
        assert!((p.z - 8.0).abs() < 1e-3, "sliding must not drift z={}", p.z);
    }
}
