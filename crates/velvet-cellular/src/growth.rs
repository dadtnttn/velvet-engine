//! Organic growth CA — vines, moss, mushrooms, seeds.

use crate::cell::Cell;
use crate::world::World;

/// Growth rules config.
#[derive(Debug, Clone)]
pub struct GrowthConfig {
    /// Chance vine grows up into air.
    pub vine_up: f32,
    /// Chance moss spreads sideways on solid.
    pub moss_spread: f32,
    /// Chance seed becomes vine if on solid + water nearby.
    pub seed_sprout: f32,
    /// Chance mushroom spreads on organic.
    pub shroom_spread: f32,
    /// Max growth ops per pass.
    pub max_ops: usize,
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self {
            vine_up: 0.15,
            moss_spread: 0.08,
            seed_sprout: 0.05,
            shroom_spread: 0.06,
            max_ops: 256,
        }
    }
}

/// One growth pass over a region.
pub fn growth_pass(
    world: &mut World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    cfg: &GrowthConfig,
) -> usize {
    let mut ops = 0usize;
    let vine = world.mat("vine");
    let moss = world.mat("moss");
    let grass = world.mat("grass");
    let seed = world.mat("seed");
    let mushroom = world.mat("mushroom");
    let water = world.mat("water");

    // collect candidates first to avoid mid-scan bias
    let mut candidates = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let c = world.get(x, y);
            if c.is_air() {
                continue;
            }
            let key = world.materials.get(c.material).key.as_str();
            if matches!(
                key,
                "vine" | "moss" | "grass" | "seed" | "mushroom" | "spore"
            ) {
                candidates.push((x, y, key.to_string()));
            }
        }
    }

    for (x, y, key) in candidates {
        if ops >= cfg.max_ops {
            break;
        }
        match key.as_str() {
            "vine" | "grass" => {
                if !vine.is_air() && world.get(x, y + 1).is_air() && world.chance(cfg.vine_up) {
                    world.set(
                        x,
                        y + 1,
                        Cell::of(if key == "grass" && !grass.is_air() {
                            grass
                        } else {
                            vine
                        }),
                    );
                    ops += 1;
                }
            }
            "moss" => {
                if moss.is_air() {
                    continue;
                }
                for (dx, dy) in [(-1, 0), (1, 0), (0, 1)] {
                    if world.chance(cfg.moss_spread) && world.get(x + dx, y + dy).is_air() {
                        // moss prefers solid neighbor
                        let solid_near =
                            [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(ox, oy)| {
                                let m = world.get(x + dx + ox, y + dy + oy).material;
                                !m.is_air()
                                    && matches!(
                                        world.materials.phase(m),
                                        crate::material::Phase::Solid
                                            | crate::material::Phase::Static
                                    )
                            });
                        if solid_near {
                            world.set(x + dx, y + dy, Cell::of(moss));
                            ops += 1;
                            break;
                        }
                    }
                }
            }
            "seed" => {
                if seed.is_air() {
                    continue;
                }
                let on_solid = {
                    let b = world.get(x, y - 1).material;
                    !b.is_air()
                        && matches!(
                            world.materials.phase(b),
                            crate::material::Phase::Solid
                                | crate::material::Phase::Static
                                | crate::material::Phase::Powder
                        )
                };
                let wet = [(-1, 0), (1, 0), (0, -1), (0, 1), (0, 1)]
                    .iter()
                    .any(|&(dx, dy)| world.get(x + dx, y + dy).material == water);
                if on_solid && wet && world.chance(cfg.seed_sprout) {
                    let grow = if !vine.is_air() {
                        vine
                    } else if !grass.is_air() {
                        grass
                    } else {
                        continue;
                    };
                    world.set(x, y, Cell::of(grow));
                    ops += 1;
                }
            }
            "mushroom" | "spore" => {
                if mushroom.is_air() {
                    continue;
                }
                for (dx, dy) in [(-1, 0), (1, 0), (0, 1), (0, -1)] {
                    if !world.chance(cfg.shroom_spread) {
                        continue;
                    }
                    let n = world.get(x + dx, y + dy);
                    if n.is_air() {
                        // need organic or dirt below/near
                        let organic = world
                            .materials
                            .get(world.get(x + dx, y + dy - 1).material)
                            .tags
                            .iter()
                            .any(|t| t == "organic" || t == "soil");
                        if organic || world.get(x + dx, y + dy - 1).material == world.mat("dirt") {
                            world.set(x + dx, y + dy, Cell::of(mushroom));
                            ops += 1;
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    ops
}

/// Plant a seed and water it (author helper).
pub fn plant_seed(world: &mut World, x: i32, y: i32) -> bool {
    let seed = world.mat("seed");
    let water = world.mat("water");
    if seed.is_air() {
        return false;
    }
    world.set(x, y, Cell::of(seed));
    if !water.is_air() {
        world.set(x + 1, y, Cell::of(water));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::material_catalog::register_catalog_materials;
    use crate::world::WorldConfig;

    #[test]
    fn vine_grows_upward() {
        let (mut reg, ids) = builtin_registry();
        register_catalog_materials(&mut reg).unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-2, 0, 2, 1, ids.bedrock);
        let vine = world.mat("vine");
        assert!(!vine.is_air());
        world.set(0, 1, Cell::of(vine));
        let mut cfg = GrowthConfig::default();
        cfg.vine_up = 1.0; // force
        cfg.max_ops = 32;
        let mut total = 0;
        for _ in 0..20 {
            total += growth_pass(&mut world, -2, 0, 2, 20, &cfg);
        }
        assert!(total > 0 || world.get(0, 2).material == vine);
        // eventually something above
        let mut height = 0;
        for y in 1..15 {
            if world.get(0, y).material == vine {
                height = y;
            }
        }
        assert!(height >= 1);
    }
}
