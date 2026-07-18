//! Wand / spell combo system — chain recipes with modifiers (author API).

use serde::{Deserialize, Serialize};

use crate::particles::ParticleWorld;
use crate::spells::{cast_recipe, SpellBook, SpellEffect, SpellRecipe};
use crate::world::World;

/// Modifier applied to the next spell in the chain.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WandModifier {
    /// Multiply power.
    DoublePower,
    /// Add radius.
    PlusRadius(i32),
    /// Reduce cost.
    Cheap,
    /// Scatter cast N times with offset.
    Scatter(u8),
    /// Delay (game uses; cast still immediate here but tags delay).
    Delay,
    /// Trail: also cast dig along path.
    TrailDig,
}

/// One slot in a wand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WandSlot {
    /// Spell key from book.
    Spell(String),
    /// Modifier.
    Mod(WandModifier),
}

/// Author wand: ordered slots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Wand {
    /// Name.
    pub name: String,
    /// Slots.
    pub slots: Vec<WandSlot>,
    /// Mana.
    pub mana: f32,
    /// Max mana.
    pub max_mana: f32,
    /// Cooldown remaining.
    pub cooldown: f32,
}

impl Wand {
    /// Create empty wand.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slots: Vec::new(),
            mana: 100.0,
            max_mana: 100.0,
            cooldown: 0.0,
        }
    }

    /// Push spell.
    pub fn spell(mut self, key: impl Into<String>) -> Self {
        self.slots.push(WandSlot::Spell(key.into()));
        self
    }

    /// Push mod.
    pub fn modify(mut self, m: WandModifier) -> Self {
        self.slots.push(WandSlot::Mod(m));
        self
    }

    /// Tick cooldown.
    pub fn tick(&mut self, dt: f32) {
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.mana = (self.mana + dt * 5.0).min(self.max_mana);
    }
}

/// Resolved cast plan after applying modifiers.
#[derive(Debug, Clone)]
pub struct CastPlan {
    /// Recipe snapshot.
    pub recipe: SpellRecipe,
    /// Scatter count.
    pub scatter: u8,
    /// Also trail dig.
    pub trail_dig: bool,
}

/// Build cast plans from wand slots.
pub fn resolve_wand(book: &SpellBook, wand: &Wand) -> Vec<CastPlan> {
    let mut plans = Vec::new();
    let mut power_mul = 1.0f32;
    let mut radius_add = 0i32;
    let mut scatter = 1u8;
    let mut trail = false;
    let mut cheap = false;

    for slot in &wand.slots {
        match slot {
            WandSlot::Mod(WandModifier::DoublePower) => power_mul *= 2.0,
            WandSlot::Mod(WandModifier::PlusRadius(r)) => radius_add += *r,
            WandSlot::Mod(WandModifier::Scatter(n)) => scatter = (*n).max(1),
            WandSlot::Mod(WandModifier::TrailDig) => trail = true,
            WandSlot::Mod(WandModifier::Cheap) => cheap = true,
            WandSlot::Mod(WandModifier::Delay) => {}
            WandSlot::Spell(key) => {
                if let Some(mut r) = book.get(key).cloned() {
                    r.power = ((r.power as f32) * power_mul).round() as u32;
                    r.radius = (r.radius + radius_add).max(0);
                    if cheap {
                        r.cost *= 0.5;
                    }
                    plans.push(CastPlan {
                        recipe: r,
                        scatter,
                        trail_dig: trail,
                    });
                }
                // reset mods after spell (Noita-like wrap)
                power_mul = 1.0;
                radius_add = 0;
                scatter = 1;
                trail = false;
                cheap = false;
            }
        }
    }
    plans
}

/// Fire wand at position / aim.
pub fn cast_wand(
    book: &SpellBook,
    wand: &mut Wand,
    world: &mut World,
    particles: &mut ParticleWorld,
    x: f32,
    y: f32,
    aim: f32,
) -> u32 {
    if wand.cooldown > 0.0 {
        return 0;
    }
    let plans = resolve_wand(book, wand);
    let mut casts = 0u32;
    let mut total_cost = 0.0f32;
    for plan in &plans {
        total_cost += plan.recipe.cost * plan.scatter as f32;
    }
    if wand.mana < total_cost {
        return 0;
    }
    wand.mana -= total_cost;
    wand.cooldown = 0.15;

    for plan in plans {
        for s in 0..plan.scatter {
            let ang = aim + (s as f32 - plan.scatter as f32 * 0.5) * 0.15;
            let cx = x + ang.cos() * 2.0;
            let cy = y + ang.sin() * 2.0;
            if cast_recipe(&plan.recipe, world, particles, cx, cy) {
                casts += 1;
            }
            if plan.trail_dig {
                let dig = SpellRecipe::new("trail", "Trail", SpellEffect::BurstDig)
                    .power(8)
                    .radius(1);
                cast_recipe(&dig, world, particles, cx, cy);
            }
        }
    }
    casts
}

/// Sample starter wands.
pub fn starter_wands() -> Vec<Wand> {
    vec![
        Wand::new("Spark Wand")
            .spell("spark_bolt")
            .modify(WandModifier::DoublePower),
        Wand::new("Digging Wand")
            .modify(WandModifier::Scatter(3))
            .spell("digging_blast")
            .modify(WandModifier::TrailDig),
        Wand::new("Blood Wand")
            .spell("blood_spray")
            .modify(WandModifier::PlusRadius(2)),
        Wand::new("Combo Wand")
            .modify(WandModifier::Cheap)
            .spell("water_ball")
            .spell("spark_bolt"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::particles::ParticleWorld;
    use crate::spells::register_builtin_spells;
    use crate::world::WorldConfig;

    #[test]
    fn wand_double_power_and_cast() {
        let (reg, _) = builtin_registry();
        let mut world = crate::world::World::new(reg, WorldConfig::default());
        let mut particles = ParticleWorld::default();
        let mut book = SpellBook::new();
        register_builtin_spells(&mut book);
        let mut wand = Wand::new("t")
            .modify(WandModifier::DoublePower)
            .spell("spark_bolt");
        let plans = resolve_wand(&book, &wand);
        assert_eq!(plans.len(), 1);
        assert!(plans[0].recipe.power >= 32);
        let n = cast_wand(
            &book,
            &mut wand,
            &mut world,
            &mut particles,
            0.0,
            10.0,
            1.57,
        );
        assert!(n >= 1);
        assert!(wand.mana < 100.0);
    }
}
