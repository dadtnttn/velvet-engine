//! Built-in material pack authors can start from (or replace entirely).

use crate::cell::MaterialId;
use crate::material::{MaterialDef, MaterialError, MaterialRegistry, Phase};

/// Register the default creative pack. Returns key material ids.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinIds {
    /// Sand.
    pub sand: MaterialId,
    /// Water.
    pub water: MaterialId,
    /// Stone.
    pub stone: MaterialId,
    /// Wood.
    pub wood: MaterialId,
    /// Fire.
    pub fire: MaterialId,
    /// Smoke.
    pub smoke: MaterialId,
    /// Oil.
    pub oil: MaterialId,
    /// Lava.
    pub lava: MaterialId,
    /// Acid.
    pub acid: MaterialId,
    /// Steam.
    pub steam: MaterialId,
    /// Ice.
    pub ice: MaterialId,
    /// Ash.
    pub ash: MaterialId,
    /// Bedrock.
    pub bedrock: MaterialId,
    /// Metal.
    pub metal: MaterialId,
    /// Gunpowder.
    pub gunpowder: MaterialId,
    /// Blood (viscous, clots).
    pub blood: MaterialId,
    /// Dried blood stain.
    pub dried_blood: MaterialId,
    /// Flesh (organic solid, leaks blood).
    pub flesh: MaterialId,
    /// Bone.
    pub bone: MaterialId,
    /// Poison liquid.
    pub poison: MaterialId,
    /// Slime trail.
    pub slime_trail: MaterialId,
    /// Soil / dirt.
    pub dirt: MaterialId,
    /// Grass (flammable solid).
    pub grass: MaterialId,
    /// Salt (powder).
    pub salt: MaterialId,
    /// Glass (solid, brittle dig→sand).
    pub glass: MaterialId,
}

/// Install builtins into an empty-or-air registry.
pub fn register_builtin_materials(reg: &mut MaterialRegistry) -> Result<BuiltinIds, MaterialError> {
    // Placeholders for forward refs — we register in dependency order and patch melt targets after.
    let sand = reg.register(
        MaterialDef::new("sand", "Sand", Phase::Powder)
            .density(1.5)
            .color(194, 178, 128, 255)
            .tag("soil"),
    )?;
    let water = reg.register({
        let mut m = MaterialDef::new("water", "Water", Phase::Liquid)
            .density(1.0)
            .color(64, 140, 255, 180)
            .tag("liquid");
        m.physics.viscosity = 0.15;
        m.physics.freeze_point = Some(0.0);
        m.physics.boil_point = Some(100.0);
        m.reaction.extinguishes = true;
        m
    })?;
    let stone = reg.register(
        MaterialDef::new("stone", "Stone", Phase::Solid)
            .density(2.5)
            .color(110, 110, 115, 255)
            .tag("rock"),
    )?;
    let wood = reg.register({
        let mut m = MaterialDef::new("wood", "Wood", Phase::Solid)
            .density(0.7)
            .color(120, 75, 40, 255)
            .tag("organic")
            .flammable(250.0, 40);
        m.reaction.burn_heat = 12.0;
        m
    })?;
    let fire = reg.register({
        let mut m = MaterialDef::new("fire", "Fire", Phase::Plasma)
            .density(0.05)
            .color(255, 90, 20, 220);
        m.affected_by_gravity = true;
        m.reaction.burn_life = 24;
        m.reaction.burn_heat = 25.0;
        m.physics.conductivity = 0.8;
        m
    })?;
    let smoke = reg.register({
        let mut m = MaterialDef::new("smoke", "Smoke", Phase::Gas)
            .density(0.02)
            .color(80, 80, 80, 120);
        m.affected_by_gravity = false;
        m
    })?;
    let oil = reg.register({
        let mut m = MaterialDef::new("oil", "Oil", Phase::Liquid)
            .density(0.85)
            .color(40, 30, 20, 220)
            .flammable(180.0, 50);
        m.physics.viscosity = 0.4;
        m.reaction.burn_heat = 18.0;
        m
    })?;
    let lava = reg.register({
        let mut m = MaterialDef::new("lava", "Lava", Phase::Liquid)
            .density(2.8)
            .color(255, 60, 10, 255);
        m.physics.viscosity = 0.55;
        m.reaction.burn_heat = 40.0;
        m
    })?;
    let acid = reg.register({
        let mut m = MaterialDef::new("acid", "Acid", Phase::Liquid)
            .density(1.1)
            .color(80, 255, 80, 200);
        m.physics.viscosity = 0.2;
        m.reaction.dissolve_rate = 8;
        m
    })?;
    let steam = reg.register({
        let mut m = MaterialDef::new("steam", "Steam", Phase::Gas)
            .density(0.01)
            .color(200, 200, 220, 100);
        m.affected_by_gravity = false;
        m
    })?;
    let ice = reg.register({
        let mut m = MaterialDef::new("ice", "Ice", Phase::Solid)
            .density(0.92)
            .color(180, 220, 255, 230);
        m.physics.melt_point = Some(0.0);
        m
    })?;
    let ash = reg.register(
        MaterialDef::new("ash", "Ash", Phase::Powder)
            .density(0.4)
            .color(90, 90, 90, 255),
    )?;
    let bedrock = reg.register({
        let mut m = MaterialDef::new("bedrock", "Bedrock", Phase::Static)
            .density(99.0)
            .color(40, 40, 45, 255);
        m.affected_by_gravity = false;
        m
    })?;
    let metal = reg.register({
        let mut m = MaterialDef::new("metal", "Metal", Phase::Solid)
            .density(7.0)
            .color(160, 165, 175, 255)
            .tag("metal");
        m.physics.melt_point = Some(1200.0);
        m.physics.conductivity = 0.9;
        m
    })?;
    let gunpowder = reg.register({
        let mut m = MaterialDef::new("gunpowder", "Gunpowder", Phase::Powder)
            .density(1.2)
            .color(30, 30, 30, 255)
            .flammable(100.0, 5);
        m.reaction.explosive = true;
        m.reaction.explosion_radius = 4;
        m.reaction.burn_heat = 50.0;
        m
    })?;
    let blood = reg.register({
        let mut m = MaterialDef::new("blood", "Blood", Phase::Liquid)
            .density(1.05)
            .color(140, 10, 20, 230)
            .tag("organic")
            .tag("fluid");
        m.physics.viscosity = 0.55;
        m.physics.freeze_point = Some(-5.0);
        m
    })?;
    let dried_blood = reg.register(
        MaterialDef::new("dried_blood", "Dried Blood", Phase::Powder)
            .density(0.9)
            .color(80, 15, 20, 255)
            .tag("organic"),
    )?;
    let flesh = reg.register({
        let mut m = MaterialDef::new("flesh", "Flesh", Phase::Solid)
            .density(1.1)
            .color(180, 70, 80, 255)
            .tag("organic")
            .flammable(280.0, 35);
        m.reaction.burn_heat = 10.0;
        m
    })?;
    let bone = reg.register(
        MaterialDef::new("bone", "Bone", Phase::Solid)
            .density(1.4)
            .color(230, 220, 200, 255)
            .tag("organic"),
    )?;
    let poison = reg.register({
        let mut m = MaterialDef::new("poison", "Poison", Phase::Liquid)
            .density(1.05)
            .color(120, 255, 80, 200)
            .tag("hazard");
        m.physics.viscosity = 0.25;
        m.reaction.dissolve_rate = 4;
        m
    })?;
    let slime_trail = reg.register({
        let mut m = MaterialDef::new("slime_trail", "Slime Trail", Phase::Liquid)
            .density(1.15)
            .color(60, 200, 90, 180);
        m.physics.viscosity = 0.7;
        m
    })?;
    let dirt = reg.register(
        MaterialDef::new("dirt", "Dirt", Phase::Powder)
            .density(1.3)
            .color(90, 60, 30, 255)
            .tag("soil"),
    )?;
    let grass = reg.register({
        let mut m = MaterialDef::new("grass", "Grass", Phase::Solid)
            .density(0.3)
            .color(50, 160, 50, 255)
            .tag("organic")
            .flammable(180.0, 12);
        m.reaction.burn_heat = 8.0;
        m
    })?;
    let salt = reg.register(
        MaterialDef::new("salt", "Salt", Phase::Powder)
            .density(1.2)
            .color(240, 240, 245, 255),
    )?;
    let glass = reg.register({
        let mut m = MaterialDef::new("glass", "Glass", Phase::Solid)
            .density(2.5)
            .color(180, 220, 230, 180);
        m.physics.melt_point = Some(900.0);
        m
    })?;

    // Patch cross-references
    patch(reg, water, |d| {
        d.physics.freeze_into = Some(ice);
        d.physics.boil_into = Some(steam);
    });
    patch(reg, ice, |d| {
        d.physics.melt_into = Some(water);
    });
    patch(reg, wood, |d| {
        d.reaction.burn_product = Some(smoke);
        d.reaction.burn_residue = Some(ash);
    });
    patch(reg, oil, |d| {
        d.reaction.burn_product = Some(smoke);
        d.reaction.burn_residue = Some(ash);
    });
    patch(reg, fire, |d| {
        d.reaction.burn_product = Some(smoke);
        d.reaction.burn_residue = Some(MaterialId::AIR);
    });
    patch(reg, lava, |d| {
        d.physics.freeze_point = Some(600.0);
        d.physics.freeze_into = Some(stone);
        // lava melts ice/wood via heat; dissolves nothing list
    });
    patch(reg, metal, |d| {
        d.physics.melt_into = Some(lava);
    });
    patch(reg, acid, |d| {
        d.reaction.dissolves = vec![wood, metal, sand, flesh, bone, dirt];
    });
    patch(reg, poison, |d| {
        d.reaction.dissolves = vec![flesh, grass];
    });
    patch(reg, blood, |d| {
        d.physics.freeze_into = Some(dried_blood);
    });
    patch(reg, flesh, |d| {
        d.reaction.burn_product = Some(smoke);
        d.reaction.burn_residue = Some(ash);
    });
    patch(reg, grass, |d| {
        d.reaction.burn_product = Some(smoke);
        d.reaction.burn_residue = Some(ash);
    });
    patch(reg, glass, |d| {
        d.physics.melt_into = Some(sand);
    });
    let _ = stone;
    let _ = bedrock;
    let _ = gunpowder;
    let _ = bone;
    let _ = slime_trail;
    let _ = salt;

    Ok(BuiltinIds {
        sand,
        water,
        stone,
        wood,
        fire,
        smoke,
        oil,
        lava,
        acid,
        steam,
        ice,
        ash,
        bedrock,
        metal,
        gunpowder,
        blood,
        dried_blood,
        flesh,
        bone,
        poison,
        slime_trail,
        dirt,
        grass,
        salt,
        glass,
    })
}

fn patch(reg: &mut MaterialRegistry, id: MaterialId, f: impl FnOnce(&mut MaterialDef)) {
    if let Some(d) = reg.try_get(id).cloned() {
        let mut d = d;
        f(&mut d);
        // re-register not possible; mutate internal via unsafe-ish approach:
        // MaterialRegistry doesn't expose mut get — add method
        reg_set(reg, id, d);
    }
}

fn reg_set(reg: &mut MaterialRegistry, id: MaterialId, def: MaterialDef) {
    // use a package-private helper via all() — need mut access
    reg.set_def(id, def);
}

/// Create a full registry with builtins.
pub fn builtin_registry() -> (MaterialRegistry, BuiltinIds) {
    let mut reg = MaterialRegistry::new();
    let ids = register_builtin_materials(&mut reg).expect("builtin register");
    (reg, ids)
}
