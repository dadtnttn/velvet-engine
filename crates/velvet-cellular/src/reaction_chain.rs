//! Multi-hop material reaction chains for authors (fire↔oil↔steam, acid chains, etc.).

use crate::cell::{Cell, CellFlags, MaterialId};
use crate::material::Phase;
use crate::world::World;

/// One reaction rule: when catalyst touches reactant → product.
#[derive(Debug, Clone)]
pub struct ReactionRule {
    /// Catalyst material key.
    pub catalyst: &'static str,
    /// Reactant key.
    pub reactant: &'static str,
    /// Product key (air = destroy).
    pub product: &'static str,
    /// Chance 0..=1.
    pub chance: f32,
    /// Heat added to product.
    pub heat: f32,
    /// Also spawn product in air neighbor.
    pub spawn_neighbor: bool,
}

/// Built-in chemistry table (compact, data-driven).
pub const CHAIN_RULES: &[ReactionRule] = &[
    ReactionRule {
        catalyst: "fire",
        reactant: "oil",
        product: "fire",
        chance: 0.8,
        heat: 200.0,
        spawn_neighbor: true,
    },
    ReactionRule {
        catalyst: "fire",
        reactant: "alcohol",
        product: "fire",
        chance: 0.9,
        heat: 150.0,
        spawn_neighbor: true,
    },
    ReactionRule {
        catalyst: "fire",
        reactant: "water",
        product: "steam",
        chance: 0.5,
        heat: 50.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "lava",
        reactant: "water",
        product: "steam",
        chance: 0.9,
        heat: 100.0,
        spawn_neighbor: true,
    },
    ReactionRule {
        catalyst: "lava",
        reactant: "sand",
        product: "glass",
        chance: 0.2,
        heat: 0.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "acid",
        reactant: "flesh",
        product: "blood",
        chance: 0.7,
        heat: 0.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "acid",
        reactant: "metal",
        product: "air",
        chance: 0.4,
        heat: 0.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "acid",
        reactant: "copper",
        product: "air",
        chance: 0.5,
        heat: 0.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "water",
        reactant: "fire",
        product: "steam",
        chance: 0.7,
        heat: -100.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "coolant",
        reactant: "lava",
        product: "stone",
        chance: 0.6,
        heat: -200.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "napalm",
        reactant: "wood",
        product: "fire",
        chance: 0.85,
        heat: 300.0,
        spawn_neighbor: true,
    },
    ReactionRule {
        catalyst: "napalm",
        reactant: "paper",
        product: "fire",
        chance: 0.95,
        heat: 250.0,
        spawn_neighbor: true,
    },
    ReactionRule {
        catalyst: "poison",
        reactant: "water",
        product: "poison",
        chance: 0.3,
        heat: 0.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "blood",
        reactant: "water",
        product: "blood",
        chance: 0.15,
        heat: 0.0,
        spawn_neighbor: false,
    },
    ReactionRule {
        catalyst: "seed",
        reactant: "dirt",
        product: "grass",
        chance: 0.1,
        heat: 0.0,
        spawn_neighbor: false,
    },
];

/// Apply reaction chains in a region; returns number of reactions.
pub fn apply_reaction_chains(
    world: &mut World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    max_ops: usize,
) -> usize {
    let mut ops = 0usize;
    // snapshot contacts
    let mut contacts = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let a = world.get(x, y);
            if a.is_air() {
                continue;
            }
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let bx = x + dx;
                let by = y + dy;
                let b = world.get(bx, by);
                if b.is_air() {
                    continue;
                }
                contacts.push((x, y, bx, by, a.material, b.material));
            }
        }
    }
    for (ax, ay, bx, by, am, bm) in contacts {
        if ops >= max_ops {
            break;
        }
        let akey = world.materials.get(am).key.clone();
        let bkey = world.materials.get(bm).key.clone();
        for rule in CHAIN_RULES {
            let (cx, cy, rx, ry) = if akey == rule.catalyst && bkey == rule.reactant {
                (ax, ay, bx, by)
            } else if bkey == rule.catalyst && akey == rule.reactant {
                (bx, by, ax, ay)
            } else {
                continue;
            };
            if !world.chance(rule.chance) {
                continue;
            }
            let product = if rule.product == "air" {
                MaterialId::AIR
            } else {
                let p = world.mat(rule.product);
                if p.is_air() && rule.product != "air" {
                    continue;
                }
                p
            };
            if product.is_air() {
                world.set(rx, ry, Cell::air());
            } else {
                let mut cell = Cell::of(product).with_temp(
                    world.get(rx, ry).temp + rule.heat,
                );
                if rule.product == "fire" {
                    cell.flags.insert(CellFlags::BURNING);
                    cell.life = 16;
                    cell.temp = cell.temp.max(600.0);
                }
                world.set(rx, ry, cell);
            }
            if rule.spawn_neighbor {
                for (dx, dy) in [(-1, 0), (1, 0), (0, 1)] {
                    if world.get(cx + dx, cy + dy).is_air() && !product.is_air() {
                        world.set(
                            cx + dx,
                            cy + dy,
                            Cell::of(product).with_temp(400.0).with_life(10),
                        );
                        break;
                    }
                }
            }
            let _ = (cx, cy);
            ops += 1;
            break;
        }
    }
    ops
}

/// Detect if any catalyst-reactant pair is adjacent in region.
pub fn count_reactive_contacts(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> usize {
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let a = world.get(x, y);
            if a.is_air() {
                continue;
            }
            let akey = world.materials.get(a.material).key.as_str();
            for (dx, dy) in [(1, 0), (0, 1)] {
                let b = world.get(x + dx, y + dy);
                if b.is_air() {
                    continue;
                }
                let bkey = world.materials.get(b.material).key.as_str();
                for rule in CHAIN_RULES {
                    if (akey == rule.catalyst && bkey == rule.reactant)
                        || (bkey == rule.catalyst && akey == rule.reactant)
                    {
                        n += 1;
                        break;
                    }
                }
            }
        }
    }
    n
}

/// Extinguish fires in radius with water-like materials.
pub fn extinguish_radius(world: &mut World, cx: i32, cy: i32, r: i32) -> u32 {
    let mut n = 0u32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy > r * r {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            let c = world.get(x, y);
            if c.flags.contains(CellFlags::BURNING)
                || world.materials.phase(c.material) == Phase::Plasma
            {
                world.set(x, y, Cell::air());
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::material_catalog::register_catalog_materials;
    use crate::world::WorldConfig;

    #[test]
    fn fire_oil_chain_reacts() {
        let (mut reg, ids) = builtin_registry();
        register_catalog_materials(&mut reg).unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        world.set(0, 1, Cell::of(ids.oil));
        world.set(1, 1, Cell::of(ids.fire).with_life(20).with_temp(900.0));
        let contacts = count_reactive_contacts(&world, -2, 0, 4, 4);
        assert!(contacts >= 1);
        let mut total = 0;
        for _ in 0..20 {
            total += apply_reaction_chains(&mut world, -2, 0, 4, 4, 32);
        }
        assert!(total > 0 || world.get(0, 1).material == ids.fire);
    }
}
