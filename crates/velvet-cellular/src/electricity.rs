//! Conductive pathfinding and shock propagation on the grid.

use crate::cell::{Cell, CellFlags, MaterialId};
use crate::material::Phase;
use crate::world::World;

/// Whether material is treated as conductive.
pub fn is_conductive(world: &World, id: MaterialId) -> bool {
    if id.is_air() {
        return false;
    }
    let def = world.materials.get(id);
    if def.tags.iter().any(|t| t == "conductive" || t == "metal") {
        return true;
    }
    // salt water / shock gel
    matches!(
        def.key.as_str(),
        "copper" | "iron" | "steel" | "gold" | "silver" | "copper_wire" | "water_salt" | "shock_gel" | "metal"
    ) || (def.phase == Phase::Liquid && def.physics.conductivity > 0.5)
}

/// BFS path between two conductive cells; returns path or None.
pub fn find_conductive_path(
    world: &World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    max_nodes: usize,
) -> Option<Vec<(i32, i32)>> {
    if !is_conductive(world, world.get(x0, y0).material) {
        return None;
    }
    if !is_conductive(world, world.get(x1, y1).material) {
        return None;
    }
    use std::collections::{HashMap, VecDeque};
    let mut q = VecDeque::new();
    let mut prev: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    q.push_back((x0, y0));
    prev.insert((x0, y0), (x0, y0));
    let mut visited = 0usize;
    while let Some((x, y)) = q.pop_front() {
        visited += 1;
        if visited > max_nodes {
            break;
        }
        if x == x1 && y == y1 {
            // reconstruct
            let mut path = vec![(x1, y1)];
            let mut cur = (x1, y1);
            while cur != (x0, y0) {
                cur = prev[&cur];
                path.push(cur);
            }
            path.reverse();
            return Some(path);
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x + dx;
            let ny = y + dy;
            if prev.contains_key(&(nx, ny)) {
                continue;
            }
            if is_conductive(world, world.get(nx, ny).material) {
                prev.insert((nx, ny), (x, y));
                q.push_back((nx, ny));
            }
        }
    }
    None
}

/// Shock along a path: heat + damage life of organics / flammables.
pub fn shock_path(world: &mut World, path: &[(i32, i32)], energy: f32) -> u32 {
    let mut hit = 0u32;
    for &(x, y) in path {
        let mut c = world.get(x, y);
        if c.is_air() {
            continue;
        }
        c.temp += energy;
        let def = world.materials.get(c.material);
        if def.reaction.flammable && c.temp >= def.reaction.ignite_temp {
            c.flags.insert(CellFlags::BURNING);
            if c.life == 0 {
                c.life = def.reaction.burn_life.max(1);
            }
        }
        // zap flesh/blood
        if def.tags.iter().any(|t| t == "organic") || def.key == "flesh" || def.key == "blood" {
            c.life = c.life.saturating_sub((energy as u16).max(1));
            if c.life == 0 {
                world.set(x, y, Cell::air());
                hit += 1;
                continue;
            }
        }
        world.set(x, y, c);
        hit += 1;
    }
    hit
}

/// Arc: if path exists within manhattan range, shock it.
pub fn try_arc(
    world: &mut World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    max_len: usize,
    energy: f32,
) -> bool {
    if ((x0 - x1).abs() + (y0 - y1).abs()) as usize > max_len * 2 {
        return false;
    }
    if let Some(path) = find_conductive_path(world, x0, y0, x1, y1, max_len * 8) {
        if path.len() <= max_len {
            shock_path(world, &path, energy);
            // paint plasma trail on air neighbors
            if let Ok(plasma) = world.materials.id("plasma_arc").or_else(|_| world.materials.id("fire")) {
                for &(x, y) in path.iter().step_by(2) {
                    if world.get(x, y + 1).is_air() {
                        world.set(x, y + 1, Cell::of(plasma).with_life(6).with_temp(800.0));
                    }
                }
            }
            return true;
        }
    }
    false
}

/// Spread charge pressure into conductive neighbors (diffusion).
pub fn diffuse_charge(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32) {
    // use meta byte as charge 0..255 stored in unused way — use pressure for conductive
    let mut updates = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let c = world.get(x, y);
            if !is_conductive(world, c.material) {
                continue;
            }
            let mut acc = c.pressure;
            let mut n = 1.0f32;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let o = world.get(x + dx, y + dy);
                if is_conductive(world, o.material) {
                    acc += o.pressure;
                    n += 1.0;
                }
            }
            let p = acc / n;
            if (p - c.pressure).abs() > 0.01 {
                updates.push((x, y, p));
            }
        }
    }
    for (x, y, p) in updates {
        let mut c = world.get(x, y);
        c.pressure = p;
        world.set(x, y, c);
    }
}

/// Inject charge at a cell.
pub fn inject_charge(world: &mut World, x: i32, y: i32, amount: f32) {
    let mut c = world.get(x, y);
    if is_conductive(world, c.material) {
        c.pressure += amount;
        world.set(x, y, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::material_catalog::register_catalog_materials;
    use crate::world::WorldConfig;

    #[test]
    fn metal_path_and_shock() {
        let (mut reg, ids) = builtin_registry();
        register_catalog_materials(&mut reg).unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        let copper = world.mat("copper");
        assert!(!copper.is_air());
        // wire line
        for x in 0..8 {
            world.set(x, 2, Cell::of(copper));
        }
        let path = find_conductive_path(&world, 0, 2, 7, 2, 64).expect("path");
        assert!(path.len() >= 8);
        let hit = shock_path(&mut world, &path, 30.0);
        assert!(hit >= 1);
        assert!(try_arc(&mut world, 0, 2, 7, 2, 16, 20.0));
        let _ = ids;
    }
}
