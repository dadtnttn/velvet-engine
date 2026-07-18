//! Enemy creation API for cellular games — data-driven spawns, not a full game.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::events::SimEvent;
use crate::physics::PhysicsWorld;
use crate::world::World;

/// How an enemy body is represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EnemyBodyKind {
    /// Pure grid footprint (no rigid body).
    #[default]
    GridOnly,
    /// AABB rigid body coupled to position.
    Rigid,
}

/// Simple AI brain for creators to extend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EnemyAi {
    /// Idle.
    #[default]
    Idle,
    /// Pace left/right on solids.
    Patrol,
    /// Move toward a target point each step.
    Chase,
    /// Flee from target.
    Flee,
    /// Wander randomly.
    Wander,
}

/// Blueprint authors register / instantiate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyDef {
    /// Stable key: `"slime"`, `"bat"`.
    pub key: String,
    /// Display name.
    pub name: String,
    /// Body kind.
    pub body: EnemyBodyKind,
    /// Half-width in cells.
    pub hw: f32,
    /// Half-height.
    pub hh: f32,
    /// Mass if rigid.
    pub mass: f32,
    /// Max HP.
    pub max_hp: f32,
    /// Contact damage to player (game uses this).
    pub contact_damage: f32,
    /// Move speed cells/s.
    pub speed: f32,
    /// Default AI.
    pub ai: EnemyAi,
    /// Material to paint on death (blood/gore).
    pub death_material: String,
    /// Blood burst radius on death.
    pub death_blood_radius: i32,
    /// Optional material footprint stamp while alive.
    pub footprint_material: Option<String>,
    /// Tags for filters.
    pub tags: Vec<String>,
}

impl EnemyDef {
    /// Builder.
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            body: EnemyBodyKind::Rigid,
            hw: 1.0,
            hh: 1.0,
            mass: 1.0,
            max_hp: 10.0,
            contact_damage: 1.0,
            speed: 8.0,
            ai: EnemyAi::Patrol,
            death_material: "blood".into(),
            death_blood_radius: 4,
            footprint_material: None,
            tags: Vec::new(),
        }
    }

    /// HP.
    pub fn hp(mut self, hp: f32) -> Self {
        self.max_hp = hp;
        self
    }

    /// AI.
    pub fn ai(mut self, ai: EnemyAi) -> Self {
        self.ai = ai;
        self
    }

    /// Size.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.hw = w * 0.5;
        self.hh = h * 0.5;
        self
    }

    /// Tag.
    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.tags.push(t.into());
        self
    }
}

/// Live enemy instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    /// Instance id.
    pub id: u32,
    /// Def key.
    pub def_key: String,
    /// Center X.
    pub x: f32,
    /// Center Y.
    pub y: f32,
    /// HP.
    pub hp: f32,
    /// Max HP.
    pub max_hp: f32,
    /// AI.
    pub ai: EnemyAi,
    /// Patrol direction.
    pub facing: f32,
    /// Chase/flee target.
    pub target: Option<(f32, f32)>,
    /// Linked rigid body id if any.
    pub body_id: Option<u32>,
    /// Alive.
    pub alive: bool,
    /// Stun timer (steps).
    pub stun: u32,
    /// Contact damage.
    pub contact_damage: f32,
    /// Speed.
    pub speed: f32,
    /// Death material key.
    pub death_material: String,
    /// Death blood radius.
    pub death_blood_radius: i32,
    /// Body half extents.
    pub hw: f32,
    /// Body half height.
    pub hh: f32,
}

/// Registry + instances for creators.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnemyWorld {
    /// Blueprints by key.
    pub defs: Vec<EnemyDef>,
    /// Live enemies.
    pub enemies: Vec<Enemy>,
    next_id: u32,
}

impl EnemyWorld {
    /// New.
    pub fn new() -> Self {
        Self {
            defs: Vec::new(),
            enemies: Vec::new(),
            next_id: 1,
        }
    }

    /// Register blueprint (replace if same key).
    pub fn register(&mut self, def: EnemyDef) {
        if let Some(slot) = self.defs.iter_mut().find(|d| d.key == def.key) {
            *slot = def;
        } else {
            self.defs.push(def);
        }
    }

    /// Get def.
    pub fn def(&self, key: &str) -> Option<&EnemyDef> {
        self.defs.iter().find(|d| d.key == key)
    }

    /// Spawn from def key at position. Returns enemy id.
    pub fn spawn(
        &mut self,
        key: &str,
        x: f32,
        y: f32,
        physics: &mut PhysicsWorld,
    ) -> Option<u32> {
        let def = self.def(key)?.clone();
        let id = self.next_id;
        self.next_id += 1;
        let body_id = if def.body == EnemyBodyKind::Rigid {
            let bid = physics.spawn_dynamic(x, y, def.hw * 2.0, def.hh * 2.0, def.mass);
            Some(bid)
        } else {
            None
        };
        self.enemies.push(Enemy {
            id,
            def_key: def.key,
            x,
            y,
            hp: def.max_hp,
            max_hp: def.max_hp,
            ai: def.ai,
            facing: 1.0,
            target: None,
            body_id,
            alive: true,
            stun: 0,
            contact_damage: def.contact_damage,
            speed: def.speed,
            death_material: def.death_material,
            death_blood_radius: def.death_blood_radius,
            hw: def.hw,
            hh: def.hh,
        });
        Some(id)
    }

    /// Damage enemy; returns true if killed.
    pub fn damage(
        &mut self,
        id: u32,
        amount: f32,
        world: &mut World,
        physics: &mut PhysicsWorld,
    ) -> bool {
        let Some(e) = self.enemies.iter_mut().find(|e| e.id == id && e.alive) else {
            return false;
        };
        e.hp -= amount;
        e.stun = e.stun.max(3);
        if e.hp > 0.0 {
            return false;
        }
        e.alive = false;
        let ex = e.x;
        let ey = e.y;
        let r = e.death_blood_radius;
        let mat_key = e.death_material.clone();
        let body_id = e.body_id;
        if let Some(bid) = body_id {
            physics.remove(bid);
        }
        // gore
        let mid = world.mat(&mat_key);
        if !mid.is_air() {
            world.paint_circle(ex as i32, ey as i32, r, mid);
        }
        world.events.push(SimEvent::EnemyDied {
            id,
            x: ex,
            y: ey,
            def_key: e.def_key.clone(),
        });
        true
    }

    /// Set chase target for all Chase AIs or one id.
    pub fn set_target(&mut self, id: Option<u32>, tx: f32, ty: f32) {
        for e in &mut self.enemies {
            if !e.alive {
                continue;
            }
            if let Some(i) = id {
                if e.id != i {
                    continue;
                }
            }
            if matches!(e.ai, EnemyAi::Chase | EnemyAi::Flee) || id.is_some() {
                e.target = Some((tx, ty));
            }
        }
    }

    /// Step AI + sync rigid bodies. `dt` in seconds.
    pub fn step(&mut self, world: &mut World, physics: &mut PhysicsWorld, dt: f32) {
        for e in &mut self.enemies {
            if !e.alive {
                continue;
            }
            if e.stun > 0 {
                e.stun -= 1;
                continue;
            }
            // sync from body if present
            if let Some(bid) = e.body_id {
                if let Some(b) = physics.get(bid) {
                    e.x = b.x;
                    e.y = b.y;
                }
            }
            let speed = e.speed * dt;
            match e.ai {
                EnemyAi::Idle => {}
                EnemyAi::Patrol => {
                    let nx = e.x + e.facing * speed;
                    let ground = world.get(nx as i32, (e.y - e.hh - 0.1) as i32);
                    let wall = world.get((nx + e.facing * e.hw) as i32, e.y as i32);
                    if ground.is_air() || !wall.is_air() {
                        e.facing = -e.facing;
                    } else {
                        e.x = nx;
                    }
                }
                EnemyAi::Chase => {
                    if let Some((tx, ty)) = e.target {
                        let dx = tx - e.x;
                        let dy = ty - e.y;
                        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                        e.x += dx / len * speed;
                        e.y += dy / len * speed * 0.35;
                        e.facing = if dx >= 0.0 { 1.0 } else { -1.0 };
                    }
                }
                EnemyAi::Flee => {
                    if let Some((tx, ty)) = e.target {
                        let dx = e.x - tx;
                        let dy = e.y - ty;
                        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                        e.x += dx / len * speed;
                        e.y += dy / len * speed * 0.35;
                    }
                }
                EnemyAi::Wander => {
                    if world.chance(0.02) {
                        e.facing = if world.chance(0.5) { 1.0 } else { -1.0 };
                    }
                    e.x += e.facing * speed * 0.6;
                    if world.chance(0.05) {
                        e.y += if world.chance(0.5) { speed } else { -speed } * 0.3;
                    }
                }
            }
            // write back to body
            if let Some(bid) = e.body_id {
                if let Some(b) = physics.get_mut(bid) {
                    // pull body toward AI position
                    b.vx += (e.x - b.x) * 8.0 * dt;
                    b.vy += (e.y - b.y) * 8.0 * dt;
                    // clamp mild
                    b.vx = b.vx.clamp(-e.speed, e.speed);
                }
            }
            // footprint
            if let Some(def) = self.defs.iter().find(|d| d.key == e.def_key) {
                if let Some(ref fm) = def.footprint_material {
                    let mid = world.mat(fm);
                    if !mid.is_air() && world.chance(0.15) {
                        let fx = e.x as i32;
                        let fy = (e.y - e.hh) as i32;
                        if world.get(fx, fy).is_air() {
                            world.set(fx, fy, Cell::of(mid));
                        }
                    }
                }
            }
        }
        self.enemies.retain(|e| e.alive || e.hp > -1000.0); // keep dead briefly? drop dead
        self.enemies.retain(|e| e.alive);
    }

    /// Alive count.
    pub fn alive_count(&self) -> usize {
        self.enemies.iter().filter(|e| e.alive).count()
    }

    /// Get mut.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Enemy> {
        self.enemies.iter_mut().find(|e| e.id == id)
    }

    /// Get.
    pub fn get(&self, id: u32) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.id == id)
    }
}

/// Install a small default enemy pack for sandboxes.
pub fn register_builtin_enemies(ew: &mut EnemyWorld) {
    ew.register(
        EnemyDef::new("slime", "Slime")
            .size(2.0, 2.0)
            .hp(8.0)
            .ai(EnemyAi::Patrol)
            .tag("organic"),
    );
    ew.register({
        let mut d = EnemyDef::new("bat", "Bat")
            .size(1.5, 1.5)
            .hp(5.0)
            .ai(EnemyAi::Wander)
            .tag("flying");
        d.body = EnemyBodyKind::GridOnly;
        d.death_blood_radius = 3;
        d
    });
    ew.register(
        EnemyDef::new("brute", "Brute")
            .size(3.0, 4.0)
            .hp(40.0)
            .ai(EnemyAi::Chase)
            .tag("heavy"),
    );
    ew.register({
        let mut d = EnemyDef::new("crawler", "Crawler")
            .size(2.5, 1.2)
            .hp(12.0)
            .ai(EnemyAi::Patrol);
        d.footprint_material = Some("slime_trail".into());
        d.death_material = "acid".into();
        d
    });
}

/// Helper: hit-test enemies at point.
pub fn enemy_at(ew: &EnemyWorld, x: f32, y: f32) -> Option<u32> {
    for e in &ew.enemies {
        if !e.alive {
            continue;
        }
        if (x - e.x).abs() <= e.hw && (y - e.y).abs() <= e.hh {
            return Some(e.id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::WorldConfig;

    #[test]
    fn spawn_damage_death_spawns_blood() {
        let (reg, _) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut phys = PhysicsWorld::new();
        let mut ew = EnemyWorld::new();
        register_builtin_enemies(&mut ew);
        let id = ew.spawn("slime", 0.0, 10.0, &mut phys).unwrap();
        assert_eq!(ew.alive_count(), 1);
        assert!(ew.damage(id, 100.0, &mut world, &mut phys));
        assert_eq!(ew.alive_count(), 0);
        let blood = world.mat("blood");
        assert!(!blood.is_air());
        // some blood nearby
        let mut found = false;
        for y in 6..14 {
            for x in -5..5 {
                if world.get(x, y).material == blood {
                    found = true;
                }
            }
        }
        assert!(found, "death should paint blood");
    }
}
