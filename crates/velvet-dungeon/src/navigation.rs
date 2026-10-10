//! Local A* for encounters. The logical graph routes between rooms; this grid
//! solves cover and pit avoidance inside a room. It is generated on demand,
//! never a second, independently carved version of the dungeon.
use super::*;
use std::{cmp::Reverse, collections::BinaryHeap};
pub fn room_path(map: &Blueprint, id: usize, start: Vec3, goal: Vec3, radius: f32) -> Vec<Vec3> {
    let Some(room) = map.rooms.get(id) else {
        return vec![];
    };
    let step = 0.65;
    let nx = (room.half[0] * 2.0 / step).floor() as usize + 1;
    let nz = (room.half[1] * 2.0 / step).floor() as usize + 1;
    let point = |i: usize| {
        room.center
            + Vec3::new(
                -room.half[0] + (i % nx) as f32 * step,
                0.0,
                -room.half[1] + (i / nx) as f32 * step,
            )
    };
    let valid = |p: Vec3| {
        room.contains(p.x, p.z, radius + 0.03)
            && [-radius, radius].into_iter().all(|x| {
                [-radius, radius]
                    .into_iter()
                    .all(|z| (room.floor_y(p.x + x, p.z + z) - p.y).abs() < 0.3)
            })
            && !map.obstacles.iter().filter(|o| o.room == id).any(|o| {
                (p.x - o.center.x).abs() < o.half[0] + radius
                    && (p.z - o.center.z).abs() < o.half[1] + radius
            })
    };
    let walk = (0..nx * nz).map(|i| valid(point(i))).collect::<Vec<_>>();
    let nearest = |p: Vec3| {
        (0..walk.len()).filter(|i| walk[*i]).min_by(|a, b| {
            (point(*a) - p)
                .length_squared()
                .total_cmp(&(point(*b) - p).length_squared())
        })
    };
    let (Some(from), Some(to)) = (nearest(start), nearest(goal)) else {
        return vec![];
    };
    let heuristic =
        |i: usize| ((i % nx).abs_diff(to % nx) + (i / nx).abs_diff(to / nx)) as u32 * 10;
    let mut distance = vec![u32::MAX; walk.len()];
    let mut prev = vec![usize::MAX; walk.len()];
    let mut open = BinaryHeap::new();
    distance[from] = 0;
    open.push(Reverse((heuristic(from), 0u32, from)));
    while let Some(Reverse((_, g, i))) = open.pop() {
        if g != distance[i] {
            continue;
        }
        if i == to {
            break;
        }
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let x = (i % nx) as i32 + dx;
            let z = (i / nx) as i32 + dz;
            if x < 0 || z < 0 || x >= nx as i32 || z >= nz as i32 {
                continue;
            }
            let next = z as usize * nx + x as usize;
            if !walk[next] || g + 10 >= distance[next] {
                continue;
            }
            distance[next] = g + 10;
            prev[next] = i;
            open.push(Reverse((g + 10 + heuristic(next), g + 10, next)));
        }
    }
    if distance[to] == u32::MAX {
        return vec![];
    }
    let mut path = vec![point(to)];
    let mut at = to;
    while at != from {
        at = prev[at];
        path.push(point(at));
    }
    path.reverse();
    path
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_goes_around_cover_and_respects_pits() {
        let map = generate(
            42,
            &Config::default(),
            &Catalog::bundled(),
            &Default::default(),
        )
        .unwrap();
        for r in &map.rooms {
            for s in &r.sockets {
                let end = s.position - s.outward * 0.75;
                let path = room_path(&map, r.id, r.center, end, 0.4);
                assert!(!path.is_empty(), "room {}", r.id);
                for p in path {
                    assert!(map.capsule_floor(p, 0.4).is_some());
                }
            }
        }
    }
}
