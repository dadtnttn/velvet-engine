//! Author spell / emit recipes — table-driven, not copy-paste registration.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::particles::{ParticleBurst, ParticleEnd, ParticleWorld};
use crate::world::World;

/// Spell effect kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellEffect {
    /// Material particle burst.
    BurstMaterial,
    /// Dig particles.
    BurstDig,
    /// Fire sparks.
    BurstFire,
    /// Blood burst.
    BurstBlood,
    /// Paint circle.
    PaintCircle,
    /// Paint horizontal line.
    PaintLine,
    /// Heat wave.
    HeatWave,
    /// Freeze wave.
    FreezeWave,
    /// Acid spray cone.
    AcidSpray,
    /// Shockwave dig ring.
    Shockwave,
    /// Grow vines upward.
    VineGrow,
    /// Meteor / lava drop.
    Meteor,
    /// Raise wall.
    WallRaise,
    /// Heal flesh paint.
    HealFlesh,
    /// Poison cloud.
    PoisonCloud,
    /// Steam blast.
    SteamBlast,
    /// Sand jet.
    SandJet,
    /// Water jet.
    WaterJet,
    /// Lava spit.
    LavaSpit,
    /// Oil slick.
    OilSlick,
}

/// One castable recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellRecipe {
    /// Key.
    pub key: String,
    /// Display name.
    pub name: String,
    /// Effect.
    pub effect: SpellEffect,
    /// Particle count / power.
    pub power: u32,
    /// Radius cells.
    pub radius: i32,
    /// Duration seconds.
    pub duration: f32,
    /// Material key for payload.
    pub material_key: String,
    /// Mana cost.
    pub cost: f32,
    /// Cooldown seconds.
    pub cooldown: f32,
    /// Tags.
    pub tags: Vec<String>,
}

impl SpellRecipe {
    /// Builder.
    pub fn new(key: impl Into<String>, name: impl Into<String>, effect: SpellEffect) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            effect,
            power: 12,
            radius: 4,
            duration: 1.0,
            material_key: "sand".into(),
            cost: 5.0,
            cooldown: 0.25,
            tags: Vec::new(),
        }
    }

    /// Power.
    pub fn power(mut self, p: u32) -> Self {
        self.power = p;
        self
    }
    /// Radius.
    pub fn radius(mut self, r: i32) -> Self {
        self.radius = r;
        self
    }
    /// Duration.
    pub fn duration(mut self, d: f32) -> Self {
        self.duration = d;
        self
    }
    /// Material.
    pub fn material(mut self, k: impl Into<String>) -> Self {
        self.material_key = k.into();
        self
    }
    /// Tag.
    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.tags.push(t.into());
        self
    }
    /// Cost.
    pub fn cost(mut self, c: f32) -> Self {
        self.cost = c;
        self
    }
}

/// Compact table row for bulk signature + utility spells.
struct SpellRow {
    key: &'static str,
    name: &'static str,
    effect: SpellEffect,
    power: u32,
    radius: i32,
    duration: f32,
    material: &'static str,
    tag: &'static str,
}

const SPELL_TABLE: &[SpellRow] = &[
    SpellRow { key: "spark_bolt", name: "Spark Bolt", effect: SpellEffect::BurstFire, power: 16, radius: 3, duration: 0.6, material: "fire", tag: "signature" },
    SpellRow { key: "blood_spray", name: "Blood Spray", effect: SpellEffect::BurstBlood, power: 28, radius: 4, duration: 1.2, material: "blood", tag: "signature" },
    SpellRow { key: "digging_blast", name: "Digging Blast", effect: SpellEffect::BurstDig, power: 20, radius: 3, duration: 0.35, material: "air", tag: "signature" },
    SpellRow { key: "water_ball", name: "Water Ball", effect: SpellEffect::BurstMaterial, power: 18, radius: 4, duration: 1.0, material: "water", tag: "signature" },
    SpellRow { key: "acid_ball", name: "Acid Ball", effect: SpellEffect::AcidSpray, power: 14, radius: 3, duration: 1.0, material: "acid", tag: "signature" },
    SpellRow { key: "meteor_strike", name: "Meteor Strike", effect: SpellEffect::Meteor, power: 24, radius: 6, duration: 2.0, material: "lava", tag: "signature" },
    SpellRow { key: "wall_of_stone", name: "Wall of Stone", effect: SpellEffect::WallRaise, power: 1, radius: 8, duration: 0.0, material: "stone", tag: "signature" },
    SpellRow { key: "heal_mist", name: "Heal Mist", effect: SpellEffect::HealFlesh, power: 8, radius: 3, duration: 0.0, material: "flesh", tag: "signature" },
    SpellRow { key: "sand_jet", name: "Sand Jet", effect: SpellEffect::SandJet, power: 22, radius: 2, duration: 0.8, material: "sand", tag: "utility" },
    SpellRow { key: "water_jet", name: "Water Jet", effect: SpellEffect::WaterJet, power: 20, radius: 2, duration: 0.9, material: "water", tag: "utility" },
    SpellRow { key: "lava_spit", name: "Lava Spit", effect: SpellEffect::LavaSpit, power: 10, radius: 2, duration: 1.5, material: "lava", tag: "utility" },
    SpellRow { key: "oil_slick", name: "Oil Slick", effect: SpellEffect::OilSlick, power: 1, radius: 5, duration: 0.0, material: "oil", tag: "utility" },
    SpellRow { key: "steam_blast", name: "Steam Blast", effect: SpellEffect::SteamBlast, power: 26, radius: 3, duration: 1.0, material: "steam", tag: "utility" },
    SpellRow { key: "poison_cloud", name: "Poison Cloud", effect: SpellEffect::PoisonCloud, power: 30, radius: 4, duration: 2.0, material: "poison", tag: "utility" },
    SpellRow { key: "heat_wave", name: "Heat Wave", effect: SpellEffect::HeatWave, power: 40, radius: 5, duration: 0.0, material: "fire", tag: "utility" },
    SpellRow { key: "freeze_wave", name: "Freeze Wave", effect: SpellEffect::FreezeWave, power: 40, radius: 5, duration: 0.0, material: "ice", tag: "utility" },
    SpellRow { key: "shockwave", name: "Shockwave", effect: SpellEffect::Shockwave, power: 1, radius: 6, duration: 0.0, material: "air", tag: "utility" },
    SpellRow { key: "vine_grow", name: "Vine Grow", effect: SpellEffect::VineGrow, power: 1, radius: 10, duration: 0.0, material: "grass", tag: "utility" },
    SpellRow { key: "paint_circle", name: "Paint Circle", effect: SpellEffect::PaintCircle, power: 1, radius: 4, duration: 0.0, material: "stone", tag: "utility" },
    SpellRow { key: "paint_line", name: "Paint Line", effect: SpellEffect::PaintLine, power: 1, radius: 12, duration: 0.0, material: "stone", tag: "utility" },
    SpellRow { key: "blood_burst", name: "Blood Burst", effect: SpellEffect::BurstBlood, power: 36, radius: 3, duration: 1.3, material: "blood", tag: "utility" },
    SpellRow { key: "fire_nova", name: "Fire Nova", effect: SpellEffect::BurstFire, power: 40, radius: 4, duration: 0.5, material: "fire", tag: "utility" },
    SpellRow { key: "dig_tunnel", name: "Dig Tunnel", effect: SpellEffect::BurstDig, power: 32, radius: 2, duration: 0.4, material: "air", tag: "utility" },
    SpellRow { key: "mud_ball", name: "Mud Ball", effect: SpellEffect::BurstMaterial, power: 16, radius: 3, duration: 1.1, material: "dirt", tag: "utility" },
];

/// Spell book.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpellBook {
    /// Recipes.
    pub recipes: Vec<SpellRecipe>,
}

impl SpellBook {
    /// Empty.
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
        }
    }

    /// Register / replace.
    pub fn register(&mut self, r: SpellRecipe) {
        if let Some(s) = self.recipes.iter_mut().find(|x| x.key == r.key) {
            *s = r;
        } else {
            self.recipes.push(r);
        }
    }

    /// Get.
    pub fn get(&self, key: &str) -> Option<&SpellRecipe> {
        self.recipes.iter().find(|r| r.key == key)
    }

    /// Cast.
    pub fn cast(
        &self,
        key: &str,
        world: &mut World,
        particles: &mut ParticleWorld,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(r) = self.get(key).cloned() else {
            return false;
        };
        cast_recipe(&r, world, particles, x, y)
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }

    /// All keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.recipes.iter().map(|r| r.key.as_str())
    }
}

/// Install table-driven builtins.
pub fn register_builtin_spells(book: &mut SpellBook) {
    for row in SPELL_TABLE {
        book.register(
            SpellRecipe::new(row.key, row.name, row.effect)
                .power(row.power)
                .radius(row.radius)
                .duration(row.duration)
                .material(row.material)
                .tag(row.tag)
                .tag("builtin"),
        );
    }
}

/// Number of table rows.
pub const BUILTIN_SPELL_COUNT: usize = SPELL_TABLE.len();

/// Execute a recipe against world + particles.
pub fn cast_recipe(
    r: &SpellRecipe,
    world: &mut World,
    particles: &mut ParticleWorld,
    x: f32,
    y: f32,
) -> bool {
    let mat = world.mat(&r.material_key);
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    match r.effect {
        SpellEffect::BurstMaterial => {
            particles.burst(&ParticleBurst {
                x,
                y,
                material: mat,
                count: r.power,
                speed_min: 3.0,
                speed_max: 14.0,
                lifetime: r.duration,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 20.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::BurstDig => {
            particles.burst_dig(x, y, r.power);
            true
        }
        SpellEffect::BurstFire => {
            let fire = world.mat("fire");
            particles.burst_sparks(x, y, fire, r.power);
            true
        }
        SpellEffect::BurstBlood => {
            let blood = world.mat("blood");
            particles.burst_blood(x, y, blood, r.power);
            true
        }
        SpellEffect::PaintCircle => {
            world.paint_circle(ix, iy, r.radius, mat);
            true
        }
        SpellEffect::PaintLine => {
            for dx in 0..=r.radius {
                world.set(ix + dx, iy, Cell::of(mat));
            }
            true
        }
        SpellEffect::HeatWave => {
            for dy in -r.radius..=r.radius {
                for dx in -r.radius..=r.radius {
                    if dx * dx + dy * dy <= r.radius * r.radius {
                        let mut c = world.get(ix + dx, iy + dy);
                        if !c.is_air() {
                            c.temp += 50.0 + r.power as f32;
                            world.set(ix + dx, iy + dy, c);
                        }
                    }
                }
            }
            true
        }
        SpellEffect::FreezeWave => {
            for dy in -r.radius..=r.radius {
                for dx in -r.radius..=r.radius {
                    if dx * dx + dy * dy <= r.radius * r.radius {
                        let mut c = world.get(ix + dx, iy + dy);
                        if !c.is_air() {
                            c.temp -= 40.0 + r.power as f32;
                            world.set(ix + dx, iy + dy, c);
                        }
                    }
                }
            }
            true
        }
        SpellEffect::AcidSpray => {
            let acid = world.mat("acid");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: acid,
                count: r.power,
                speed_min: 6.0,
                speed_max: 18.0,
                lifetime: r.duration,
                full_circle: false,
                angle: -1.2,
                cone: 0.9,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.8,
                temp: 20.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::Shockwave => {
            for dy in -r.radius..=r.radius {
                for dx in -r.radius..=r.radius {
                    let d2 = dx * dx + dy * dy;
                    if d2 <= r.radius * r.radius && d2 > (r.radius - 1).max(0) * (r.radius - 1).max(0)
                    {
                        let c = world.get(ix + dx, iy + dy);
                        if !c.is_air()
                            && world.materials.phase(c.material) != crate::material::Phase::Static
                        {
                            world.set(ix + dx, iy + dy, Cell::air());
                        }
                    }
                }
            }
            true
        }
        SpellEffect::VineGrow => {
            let grass = world.mat("grass");
            let g = if grass.is_air() { mat } else { grass };
            for k in 0..r.radius {
                world.set(ix, iy + k, Cell::of(g));
            }
            true
        }
        SpellEffect::Meteor => {
            let lava = world.mat("lava");
            particles.burst(&ParticleBurst {
                x,
                y: y + r.radius as f32,
                material: if lava.is_air() { mat } else { lava },
                count: r.power,
                speed_min: 2.0,
                speed_max: 6.0,
                lifetime: 2.0,
                full_circle: false,
                angle: -1.57,
                cone: 0.3,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.5,
                temp: 1200.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::WallRaise => {
            let stone = world.mat("stone");
            let m = if stone.is_air() { mat } else { stone };
            for k in 0..r.radius {
                world.set(ix - 1, iy + k, Cell::of(m));
                world.set(ix, iy + k, Cell::of(m));
                world.set(ix + 1, iy + k, Cell::of(m));
            }
            true
        }
        SpellEffect::HealFlesh => {
            let flesh = world.mat("flesh");
            world.paint_circle(
                ix,
                iy,
                r.radius.max(1),
                if flesh.is_air() { mat } else { flesh },
            );
            true
        }
        SpellEffect::PoisonCloud => {
            let poison = world.mat("poison");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: if poison.is_air() { mat } else { poison },
                count: r.power,
                speed_min: 1.0,
                speed_max: 4.0,
                lifetime: r.duration,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.2,
                temp: 20.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::SteamBlast => {
            let steam = world.mat("steam");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: if steam.is_air() { mat } else { steam },
                count: r.power,
                speed_min: 4.0,
                speed_max: 12.0,
                lifetime: 1.0,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: -0.2,
                temp: 120.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::SandJet => {
            let sand = world.mat("sand");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: if sand.is_air() { mat } else { sand },
                count: r.power,
                speed_min: 10.0,
                speed_max: 24.0,
                lifetime: 0.8,
                full_circle: false,
                angle: 0.0,
                cone: 0.25,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.5,
                temp: 20.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::WaterJet => {
            let water = world.mat("water");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: if water.is_air() { mat } else { water },
                count: r.power,
                speed_min: 10.0,
                speed_max: 22.0,
                lifetime: 0.9,
                full_circle: false,
                angle: 0.0,
                cone: 0.3,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.6,
                temp: 15.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::LavaSpit => {
            let lava = world.mat("lava");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: if lava.is_air() { mat } else { lava },
                count: r.power / 2 + 1,
                speed_min: 5.0,
                speed_max: 14.0,
                lifetime: 1.5,
                full_circle: false,
                angle: 1.0,
                cone: 0.5,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 1000.0,
                ..Default::default()
            });
            true
        }
        SpellEffect::OilSlick => {
            let oil = world.mat("oil");
            world.paint_circle(ix, iy, r.radius, if oil.is_air() { mat } else { oil });
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::particles::ParticleWorld;
    use crate::world::WorldConfig;

    #[test]
    fn book_table_and_cast() {
        let (reg, _) = builtin_registry();
        let mut world = crate::world::World::new(reg, WorldConfig::default());
        let mut particles = ParticleWorld::default();
        let mut book = SpellBook::new();
        register_builtin_spells(&mut book);
        assert_eq!(book.len(), BUILTIN_SPELL_COUNT);
        assert!(book.cast("spark_bolt", &mut world, &mut particles, 0.0, 10.0));
        assert!(book.cast("water_ball", &mut world, &mut particles, 2.0, 8.0));
        assert!(particles.len() > 0 || world.occupied_cells() > 0);
    }
}
