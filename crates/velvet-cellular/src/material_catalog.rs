//! Compact data-driven material catalog (no copy-paste MaterialDef blocks).
//!
//! Tables expand at runtime into real [`MaterialDef`] entries used by simulation.

use crate::cell::MaterialId;
use crate::material::{MaterialDef, MaterialError, MaterialRegistry, Phase};

/// Compact material row.
#[derive(Clone, Copy)]
struct MatRow {
    key: &'static str,
    name: &'static str,
    phase: Phase,
    density: f32,
    color: [u8; 4],
    flammable: bool,
    ignite: f32,
    burn_life: u16,
    viscosity: f32,
    tag: &'static str,
}

const ROWS: &[MatRow] = &[
    // metals
    MatRow {
        key: "copper",
        name: "Copper",
        phase: Phase::Solid,
        density: 8.9,
        color: [184, 115, 51, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "iron",
        name: "Iron",
        phase: Phase::Solid,
        density: 7.8,
        color: [140, 140, 145, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "steel",
        name: "Steel",
        phase: Phase::Solid,
        density: 7.85,
        color: [160, 165, 170, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "gold",
        name: "Gold",
        phase: Phase::Solid,
        density: 19.3,
        color: [255, 200, 50, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "silver",
        name: "Silver",
        phase: Phase::Solid,
        density: 10.5,
        color: [200, 205, 210, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "lead",
        name: "Lead",
        phase: Phase::Solid,
        density: 11.3,
        color: [90, 90, 100, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "bronze",
        name: "Bronze",
        phase: Phase::Solid,
        density: 8.7,
        color: [170, 120, 60, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    MatRow {
        key: "rust",
        name: "Rust",
        phase: Phase::Powder,
        density: 5.2,
        color: [150, 70, 40, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "metal",
    },
    // rocks / ores
    MatRow {
        key: "granite",
        name: "Granite",
        phase: Phase::Solid,
        density: 2.7,
        color: [100, 100, 105, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "basalt",
        name: "Basalt",
        phase: Phase::Solid,
        density: 2.9,
        color: [50, 50, 55, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "obsidian",
        name: "Obsidian",
        phase: Phase::Solid,
        density: 2.4,
        color: [20, 10, 30, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "limestone",
        name: "Limestone",
        phase: Phase::Solid,
        density: 2.5,
        color: [190, 185, 170, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "coal_ore",
        name: "Coal Ore",
        phase: Phase::Solid,
        density: 1.4,
        color: [30, 30, 30, 255],
        flammable: true,
        ignite: 300.0,
        burn_life: 60,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "iron_ore",
        name: "Iron Ore",
        phase: Phase::Solid,
        density: 4.0,
        color: [90, 50, 40, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "crystal",
        name: "Crystal",
        phase: Phase::Solid,
        density: 2.6,
        color: [180, 220, 255, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    MatRow {
        key: "gravel",
        name: "Gravel",
        phase: Phase::Powder,
        density: 1.6,
        color: [120, 115, 110, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "rock",
    },
    // powders
    MatRow {
        key: "dust",
        name: "Dust",
        phase: Phase::Powder,
        density: 0.4,
        color: [160, 150, 140, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "soot",
        name: "Soot",
        phase: Phase::Powder,
        density: 0.3,
        color: [40, 40, 40, 255],
        flammable: true,
        ignite: 250.0,
        burn_life: 15,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "snow",
        name: "Snow",
        phase: Phase::Powder,
        density: 0.3,
        color: [240, 245, 255, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "flour",
        name: "Flour",
        phase: Phase::Powder,
        density: 0.5,
        color: [245, 240, 220, 255],
        flammable: true,
        ignite: 180.0,
        burn_life: 10,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "sulfur",
        name: "Sulfur",
        phase: Phase::Powder,
        density: 1.1,
        color: [220, 210, 40, 255],
        flammable: true,
        ignite: 150.0,
        burn_life: 20,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "saltpeter",
        name: "Saltpeter",
        phase: Phase::Powder,
        density: 1.2,
        color: [230, 230, 235, 255],
        flammable: true,
        ignite: 120.0,
        burn_life: 8,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "sawdust",
        name: "Sawdust",
        phase: Phase::Powder,
        density: 0.25,
        color: [180, 140, 80, 255],
        flammable: true,
        ignite: 200.0,
        burn_life: 18,
        viscosity: 0.0,
        tag: "powder",
    },
    MatRow {
        key: "ember",
        name: "Ember",
        phase: Phase::Powder,
        density: 0.6,
        color: [255, 100, 20, 255],
        flammable: true,
        ignite: 50.0,
        burn_life: 30,
        viscosity: 0.0,
        tag: "powder",
    },
    // liquids
    MatRow {
        key: "alcohol",
        name: "Alcohol",
        phase: Phase::Liquid,
        density: 0.79,
        color: [200, 220, 255, 160],
        flammable: true,
        ignite: 80.0,
        burn_life: 25,
        viscosity: 0.1,
        tag: "liquid",
    },
    MatRow {
        key: "honey",
        name: "Honey",
        phase: Phase::Liquid,
        density: 1.4,
        color: [210, 150, 30, 220],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.85,
        tag: "liquid",
    },
    MatRow {
        key: "tar",
        name: "Tar",
        phase: Phase::Liquid,
        density: 1.2,
        color: [20, 15, 10, 255],
        flammable: true,
        ignite: 220.0,
        burn_life: 40,
        viscosity: 0.9,
        tag: "liquid",
    },
    MatRow {
        key: "bile",
        name: "Bile",
        phase: Phase::Liquid,
        density: 1.05,
        color: [140, 180, 40, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.4,
        tag: "liquid",
    },
    MatRow {
        key: "coolant",
        name: "Coolant",
        phase: Phase::Liquid,
        density: 1.1,
        color: [40, 200, 220, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.2,
        tag: "liquid",
    },
    MatRow {
        key: "nectar",
        name: "Nectar",
        phase: Phase::Liquid,
        density: 1.15,
        color: [255, 200, 100, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.5,
        tag: "liquid",
    },
    // organics
    MatRow {
        key: "moss",
        name: "Moss",
        phase: Phase::Solid,
        density: 0.4,
        color: [40, 120, 50, 255],
        flammable: true,
        ignite: 160.0,
        burn_life: 12,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "vine",
        name: "Vine",
        phase: Phase::Solid,
        density: 0.5,
        color: [30, 100, 40, 255],
        flammable: true,
        ignite: 170.0,
        burn_life: 14,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "mushroom",
        name: "Mushroom",
        phase: Phase::Solid,
        density: 0.35,
        color: [160, 100, 140, 255],
        flammable: true,
        ignite: 190.0,
        burn_life: 10,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "spore",
        name: "Spore",
        phase: Phase::Powder,
        density: 0.15,
        color: [200, 180, 220, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "leaf",
        name: "Leaf",
        phase: Phase::Solid,
        density: 0.2,
        color: [60, 140, 40, 255],
        flammable: true,
        ignite: 140.0,
        burn_life: 8,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "seed",
        name: "Seed",
        phase: Phase::Powder,
        density: 0.55,
        color: [120, 90, 40, 255],
        flammable: true,
        ignite: 200.0,
        burn_life: 12,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "meat_raw",
        name: "Raw Meat",
        phase: Phase::Solid,
        density: 1.05,
        color: [180, 60, 70, 255],
        flammable: true,
        ignite: 260.0,
        burn_life: 30,
        viscosity: 0.0,
        tag: "organic",
    },
    MatRow {
        key: "chitin",
        name: "Chitin",
        phase: Phase::Solid,
        density: 1.2,
        color: [100, 80, 50, 255],
        flammable: true,
        ignite: 280.0,
        burn_life: 22,
        viscosity: 0.0,
        tag: "organic",
    },
    // hazards / magic
    MatRow {
        key: "napalm",
        name: "Napalm",
        phase: Phase::Liquid,
        density: 0.9,
        color: [255, 140, 20, 230],
        flammable: true,
        ignite: 40.0,
        burn_life: 80,
        viscosity: 0.7,
        tag: "hazard",
    },
    MatRow {
        key: "thermite",
        name: "Thermite",
        phase: Phase::Powder,
        density: 2.0,
        color: [180, 80, 40, 255],
        flammable: true,
        ignite: 100.0,
        burn_life: 50,
        viscosity: 0.0,
        tag: "hazard",
    },
    MatRow {
        key: "shock_gel",
        name: "Shock Gel",
        phase: Phase::Liquid,
        density: 1.1,
        color: [80, 160, 255, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.35,
        tag: "hazard",
    },
    MatRow {
        key: "magic_dust",
        name: "Magic Dust",
        phase: Phase::Powder,
        density: 0.4,
        color: [200, 100, 255, 220],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "magic",
    },
    MatRow {
        key: "mana",
        name: "Mana",
        phase: Phase::Liquid,
        density: 0.85,
        color: [80, 120, 255, 180],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.15,
        tag: "magic",
    },
    MatRow {
        key: "ectoplasm",
        name: "Ectoplasm",
        phase: Phase::Liquid,
        density: 0.7,
        color: [160, 255, 180, 160],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.25,
        tag: "magic",
    },
    MatRow {
        key: "void_dust",
        name: "Void Dust",
        phase: Phase::Powder,
        density: 0.2,
        color: [30, 0, 40, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "magic",
    },
    MatRow {
        key: "unstable",
        name: "Unstable",
        phase: Phase::Powder,
        density: 1.0,
        color: [255, 0, 128, 255],
        flammable: true,
        ignite: 60.0,
        burn_life: 5,
        viscosity: 0.0,
        tag: "hazard",
    },
    // construction
    MatRow {
        key: "brick",
        name: "Brick",
        phase: Phase::Solid,
        density: 1.8,
        color: [160, 70, 50, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "concrete",
        name: "Concrete",
        phase: Phase::Solid,
        density: 2.4,
        color: [130, 130, 130, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "wood_plank",
        name: "Wood Plank",
        phase: Phase::Solid,
        density: 0.65,
        color: [150, 100, 50, 255],
        flammable: true,
        ignite: 230.0,
        burn_life: 35,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "rubber",
        name: "Rubber",
        phase: Phase::Solid,
        density: 0.95,
        color: [40, 40, 40, 255],
        flammable: true,
        ignite: 280.0,
        burn_life: 40,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "cloth",
        name: "Cloth",
        phase: Phase::Solid,
        density: 0.3,
        color: [200, 200, 210, 255],
        flammable: true,
        ignite: 150.0,
        burn_life: 15,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "paper",
        name: "Paper",
        phase: Phase::Solid,
        density: 0.25,
        color: [240, 235, 220, 255],
        flammable: true,
        ignite: 120.0,
        burn_life: 8,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "wax",
        name: "Wax",
        phase: Phase::Solid,
        density: 0.9,
        color: [240, 230, 180, 255],
        flammable: true,
        ignite: 100.0,
        burn_life: 40,
        viscosity: 0.0,
        tag: "build",
    },
    MatRow {
        key: "ice_block",
        name: "Ice Block",
        phase: Phase::Solid,
        density: 0.92,
        color: [180, 220, 255, 230],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "build",
    },
    // conductive specials for electricity system
    MatRow {
        key: "copper_wire",
        name: "Copper Wire",
        phase: Phase::Solid,
        density: 8.5,
        color: [200, 120, 40, 255],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "conductive",
    },
    MatRow {
        key: "water_salt",
        name: "Salt Water",
        phase: Phase::Liquid,
        density: 1.03,
        color: [70, 150, 220, 200],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.12,
        tag: "conductive",
    },
    MatRow {
        key: "plasma_arc",
        name: "Plasma Arc",
        phase: Phase::Plasma,
        density: 0.05,
        color: [180, 220, 255, 220],
        flammable: false,
        ignite: 0.0,
        burn_life: 0,
        viscosity: 0.0,
        tag: "conductive",
    },
];

/// Number of compact catalog rows.
pub const CATALOG_ROW_COUNT: usize = ROWS.len();

/// Register all catalog materials (skips keys already present).
pub fn register_catalog_materials(
    reg: &mut MaterialRegistry,
) -> Result<Vec<(String, MaterialId)>, MaterialError> {
    let mut out = Vec::with_capacity(ROWS.len());
    for row in ROWS {
        if reg.id(row.key).is_ok() {
            continue;
        }
        let mut m = MaterialDef::new(row.key, row.name, row.phase)
            .density(row.density)
            .color(row.color[0], row.color[1], row.color[2], row.color[3])
            .tag(row.tag);
        m.physics.viscosity = row.viscosity;
        m.affected_by_gravity = matches!(row.phase, Phase::Powder | Phase::Liquid | Phase::Plasma);
        if row.phase == Phase::Gas {
            m.affected_by_gravity = false;
        }
        if row.flammable {
            m = m.flammable(row.ignite, row.burn_life);
            m.reaction.burn_heat = 12.0;
        }
        if row.tag == "metal" {
            m.physics.conductivity = 0.9;
            m.physics.melt_point = Some(900.0 + row.density * 40.0);
        }
        if row.key == "ice_block" || row.key == "snow" {
            m.physics.melt_point = Some(0.0);
        }
        let id = reg.register(m)?;
        out.push((row.key.to_string(), id));
    }
    // patch melt targets after all registered
    if let (Ok(ice), Ok(water)) = (reg.id("ice_block"), reg.id("water")) {
        if let Some(mut d) = reg.try_get(ice).cloned() {
            d.physics.melt_into = Some(water);
            reg.set_def(ice, d);
        }
    }
    if let (Ok(snow), Ok(water)) = (reg.id("snow"), reg.id("water")) {
        if let Some(mut d) = reg.try_get(snow).cloned() {
            d.physics.melt_into = Some(water);
            d.physics.melt_point = Some(0.0);
            reg.set_def(snow, d);
        }
    }
    Ok(out)
}

/// Keys in the catalog (for tests/UI).
pub fn catalog_keys() -> impl Iterator<Item = &'static str> {
    ROWS.iter().map(|r| r.key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtin_materials;
    use crate::cell::Cell;
    use crate::sim::{step, SimConfig};
    use crate::world::{World, WorldConfig};

    #[test]
    fn catalog_registers_and_simulates_fall() {
        let mut reg = MaterialRegistry::new();
        register_builtin_materials(&mut reg).unwrap();
        let added = register_catalog_materials(&mut reg).unwrap();
        assert!(added.len() >= 40, "added={}", added.len());
        assert!(reg.id("copper").is_ok());
        assert!(reg.id("napalm").is_ok());
        assert!(reg.id("shock_gel").is_ok());

        // exercise copper gravel sand-like powder rust falling
        let rust = reg.id("rust").unwrap();
        let bed = reg.id("bedrock").unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-3, 0, 3, 1, bed);
        world.set(0, 10, Cell::of(rust));
        let cfg = SimConfig {
            parallel: false,
            ..SimConfig::default()
        };
        for _ in 0..40 {
            step(&mut world, &cfg);
        }
        let mut found_y = None;
        for y in 0..12 {
            if world.get(0, y).material == rust {
                found_y = Some(y);
            }
        }
        let y = found_y.expect("rust powder should exist");
        assert!(y < 10, "rust should fall, y={y}");
    }

    #[test]
    fn catalog_flammable_ignites_when_hot() {
        let mut reg = MaterialRegistry::new();
        register_builtin_materials(&mut reg).unwrap();
        register_catalog_materials(&mut reg).unwrap();
        let paper = reg.id("paper").unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        // superheat paper directly then step fire rules
        world.set(0, 2, Cell::of(paper).with_temp(500.0));
        let cfg = SimConfig::default();
        for _ in 0..30 {
            step(&mut world, &cfg);
        }
        let c = world.get(0, 2);
        let def = world.materials.get(paper);
        // either still paper but burning/hot, converted, or ash/air/fire
        assert!(
            c.material != paper
                || c.flags.contains(crate::cell::CellFlags::BURNING)
                || c.temp >= def.reaction.ignite_temp
                || c.is_air(),
            "paper should interact with heat path, cell={c:?}"
        );
    }

    #[test]
    fn catalog_materials_diggable_and_particle_convert() {
        let mut reg = MaterialRegistry::new();
        register_builtin_materials(&mut reg).unwrap();
        register_catalog_materials(&mut reg).unwrap();
        let gravel = reg.id("gravel").unwrap();
        let copper = reg.id("copper").unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-3, 0, 3, 1, world.mat("bedrock"));
        world.set(0, 1, Cell::of(copper));
        world.set(1, 5, Cell::of(gravel));
        // dig copper
        let mut particles = crate::particles::ParticleWorld::default();
        crate::agent::dig_at(&mut world, &mut particles, 0, 1, 1);
        assert!(world.get(0, 1).is_air(), "copper should dig");
        // gravel falls
        let cfg = SimConfig {
            parallel: false,
            ..SimConfig::default()
        };
        for _ in 0..40 {
            step(&mut world, &cfg);
        }
        let mut gy = None;
        for y in 0..8 {
            if world.get(1, y).material == gravel {
                gy = Some(y);
            }
        }
        assert!(gy.is_some() && gy.unwrap() < 5, "gravel should fall");
    }
}
