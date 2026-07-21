//! Grid A* pathfinding for agents/enemies through non-solid cells.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::material::Phase;
use crate::world::World;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    f: i32,
    x: i32,
    y: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then_with(|| self.x.cmp(&other.x))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Cost to enter a cell (None = blocked).
pub fn enter_cost(world: &World, x: i32, y: i32) -> Option<i32> {
    let c = world.get(x, y);
    if c.is_air() {
        return Some(1);
    }
    match world.materials.phase(c.material) {
        Phase::Liquid => Some(2),
        Phase::Gas | Phase::Plasma => Some(1),
        Phase::Powder => Some(3),
        Phase::Solid | Phase::Static => None,
    }
}

fn heuristic(x: i32, y: i32, gx: i32, gy: i32) -> i32 {
    (x - gx).abs() + (y - gy).abs()
}

/// A* path; max_expand bounds work.
pub fn astar(
    world: &World,
    start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
) -> Option<Vec<(i32, i32)>> {
    enter_cost(world, goal.0, goal.1)?;
    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    g_score.insert(start, 0);
    open.push(Node {
        f: heuristic(start.0, start.1, goal.0, goal.1),
        x: start.0,
        y: start.1,
    });
    let mut expanded = 0usize;
    while let Some(Node { x, y, .. }) = open.pop() {
        expanded += 1;
        if expanded > max_expand {
            break;
        }
        if (x, y) == goal {
            let mut path = vec![goal];
            let mut cur = goal;
            while cur != start {
                cur = came[&cur];
                path.push(cur);
            }
            path.reverse();
            return Some(path);
        }
        let g0 = g_score[&(x, y)];
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x + dx;
            let ny = y + dy;
            let Some(cost) = enter_cost(world, nx, ny) else {
                continue;
            };
            let ng = g0 + cost;
            let e = g_score.entry((nx, ny)).or_insert(i32::MAX);
            if ng < *e {
                *e = ng;
                came.insert((nx, ny), (x, y));
                open.push(Node {
                    f: ng + heuristic(nx, ny, goal.0, goal.1),
                    x: nx,
                    y: ny,
                });
            }
        }
    }
    None
}

/// Straight-line walkable check.
pub fn line_clear(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if enter_cost(world, x, y).is_none() {
            return false;
        }
        if x == x1 && y == y1 {
            return true;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::cell::Cell;
    use crate::world::WorldConfig;

    #[test]
    fn path_around_wall() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        // vertical wall with gap
        for y in 0..10 {
            world.set(5, y, Cell::of(ids.stone));
        }
        world.set(5, 4, Cell::air());
        let path = astar(&world, (0, 4), (10, 4), 500).expect("path");
        assert!(path.len() > 5);
        assert!(path.contains(&(5, 4)));
    }
}
