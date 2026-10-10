//! Baked masonry architecture: pilasters, vault ribs spanning the room and
//! blind wall arches between pilasters. Everything is appended to the room's
//! single static chunk mesh, so it costs no extra draw calls. Art only: the
//! walkable polygon and ceiling used by collision are unchanged. Pilasters
//! project at most `PROJECTION` into the room (less than the player radius
//! plus the camera near plane) and every rib stays above jump headroom.
use super::visual::{quad, tint};
use velvet_dungeon::{horizontal, Role, Room, Shape, PLAYER_HEIGHT};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

/// How far pilasters stand proud of the wall line into the room.
pub const PROJECTION: f32 = 0.2;
const BAY: f32 = 3.2;
const DOOR_CLEARANCE: f32 = 0.95;
/// Highest point a jumping player's head can reach above the floor.
const JUMP_HEAD: f32 = PLAYER_HEIGHT + 0.82;

/// One wall pilaster anchor on the wall line at floor height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pilaster {
    pub base: Vec3,
    pub inward: Vec3,
    pub tangent: Vec3,
    pub edge: usize,
}

/// Space between two neighbouring pilasters on the same wall: the recess
/// that frames a banner, and whose foot can hold barrels or rubble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bay {
    pub center: Vec3,
    pub inward: Vec3,
    pub tangent: Vec3,
    pub width: f32,
}

pub struct Layout {
    pub spring: f32,
    pub pilasters: Vec<Pilaster>,
    pub bays: Vec<Bay>,
    /// Transverse ribs: (start on wall, end on wall) at spring height.
    pub ribs: Vec<(Vec3, Vec3)>,
}

/// Every walled chamber gets the ornate architectural finish: moulded
/// pilasters, plinth and cornice, a ridge rib with carved bosses, ashlar
/// paving and arched doorways. Bridge rooms keep their open chasm walls.
/// Returns the accent colour of bosses and gilded bands: gold for the
/// treasury, violet for the secret vault, otherwise the biome's metal.
pub fn ornate(room: &Room) -> Option<[f32; 3]> {
    if room.shape == Shape::Bridge {
        return None;
    }
    Some(match room.role {
        Role::Treasure => [0.9, 0.58, 0.16],
        Role::Secret => [0.42, 0.3, 0.62],
        _ => match room.biome {
            0 => [0.72, 0.5, 0.24],
            1 => [0.2, 0.58, 0.6],
            _ => [0.78, 0.3, 0.14],
        },
    })
}

fn spring_height(room: &Room) -> f32 {
    (room.height - 1.35).min((room.height * 0.62).max(3.05))
}

fn edge(room: &Room, i: usize) -> (Vec3, Vec3, f32, Vec3, Vec3) {
    let a = room.outline[i];
    let b = room.outline[(i + 1) % room.outline.len()];
    let start = room.center + Vec3::new(a[0], 0.0, a[1]);
    let end = room.center + Vec3::new(b[0], 0.0, b[1]);
    let delta = end - start;
    let length = horizontal(delta).length();
    let tangent = horizontal(delta).normalize_or_zero();
    let inward = Vec3::new(-tangent.z, 0.0, tangent.x);
    (start, end, length, tangent, inward)
}

/// Sanctuary shrine walls keep their centre free of pilasters for the dais.
fn shrine_clearance(room: &Room, p: Vec3, inward: Vec3) -> bool {
    room.role == Role::Rest
        && velvet_dungeon::gameplay::doorless_walls(room)
            .first()
            .is_some_and(|(mid, wall_in, _)| {
                wall_in.dot(inward) > 0.9 && horizontal(p - *mid).length() < 2.3
            })
}

fn blocked_by_socket(room: &Room, p: Vec3) -> bool {
    room.sockets
        .iter()
        .any(|s| horizontal(s.position - p).length() < s.width * 0.5 + DOOR_CLEARANCE)
}

/// Deterministic layout for a room; empty for bridge rooms, whose walls drop
/// into the pit and whose deck is the only floor.
pub fn layout(room: &Room) -> Layout {
    let spring = spring_height(room);
    let mut out = Layout {
        spring,
        pilasters: vec![],
        bays: vec![],
        ribs: vec![],
    };
    if room.shape == Shape::Bridge || room.outline.len() != 8 {
        return out;
    }
    // Straight walls are the even outline edges; odd edges are chamfers.
    for i in (0..8).step_by(2) {
        let (start, _end, length, tangent, inward) = edge(room, i);
        let usable = length - 1.0;
        if usable < 1.5 {
            continue;
        }
        let count = ((usable / BAY).round() as usize).max(1);
        let step = usable / count as f32;
        let mut previous: Option<Vec3> = None;
        for k in 0..=count {
            let p = start + tangent * (0.5 + step * k as f32);
            if blocked_by_socket(room, p) || shrine_clearance(room, p, inward) {
                previous = None;
                continue;
            }
            out.pilasters.push(Pilaster {
                base: Vec3::new(p.x, room.center.y, p.z),
                inward,
                tangent,
                edge: i,
            });
            if let Some(q) = previous {
                let center = (p + q) * 0.5;
                if !blocked_by_socket(room, center) {
                    out.bays.push(Bay {
                        center: Vec3::new(center.x, room.center.y, center.z),
                        inward,
                        tangent,
                        width: horizontal(p - q).length(),
                    });
                }
            }
            previous = Some(p);
        }
    }
    // Ribs span the shorter dimension between facing pilasters.
    let (from, to) = if room.half[0] <= room.half[1] {
        (2usize, 6usize)
    } else {
        (0usize, 4usize)
    };
    for a in out.pilasters.iter().filter(|p| p.edge == from) {
        if let Some(b) = out.pilasters.iter().filter(|p| p.edge == to).find(|b| {
            let across = horizontal(b.base - a.base);
            across.normalize_or_zero().dot(a.inward) > 0.995
        }) {
            let s = a.base + a.inward * PROJECTION;
            let e = b.base + b.inward * PROJECTION;
            out.ribs.push((s + Vec3::Y * spring, e + Vec3::Y * spring));
        }
    }
    out
}

/// Box with arbitrary orthonormal axes (`x`, `y`, `z`) and half extents.
#[allow(clippy::too_many_arguments)]
pub(super) fn block(m: &mut Mesh3D, c: Vec3, x: Vec3, y: Vec3, z: Vec3, h: Vec3, color: [f32; 3]) {
    let p = |sx: f32, sy: f32, sz: f32| c + x * (sx * h.x) + y * (sy * h.y) + z * (sz * h.z);
    let faces = [
        (
            z,
            [
                p(-1., -1., 1.),
                p(1., -1., 1.),
                p(1., 1., 1.),
                p(-1., 1., 1.),
            ],
            1.10,
        ),
        (
            -z,
            [
                p(1., -1., -1.),
                p(-1., -1., -1.),
                p(-1., 1., -1.),
                p(1., 1., -1.),
            ],
            0.92,
        ),
        (
            x,
            [
                p(1., -1., 1.),
                p(1., -1., -1.),
                p(1., 1., -1.),
                p(1., 1., 1.),
            ],
            1.0,
        ),
        (
            -x,
            [
                p(-1., -1., -1.),
                p(-1., -1., 1.),
                p(-1., 1., 1.),
                p(-1., 1., -1.),
            ],
            1.0,
        ),
        (
            y,
            [
                p(-1., 1., 1.),
                p(1., 1., 1.),
                p(1., 1., -1.),
                p(-1., 1., -1.),
            ],
            1.18,
        ),
        (
            -y,
            [
                p(-1., -1., -1.),
                p(1., -1., -1.),
                p(1., -1., 1.),
                p(-1., -1., 1.),
            ],
            0.7,
        ),
    ];
    for (normal, corners, shade) in faces {
        quad(m, corners, normal, tint(color, shade));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};

    #[test]
    fn architecture_keeps_doors_headroom_and_walkable_space() {
        let catalog = Catalog::bundled();
        let mut ribs = 0;
        for seed in 0..32 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let plan = layout(room);
                assert_eq!(plan.pilasters, layout(room).pilasters);
                if room.shape != Shape::Bridge {
                    assert!(plan.spring >= JUMP_HEAD, "rib springs inside jump headroom");
                    assert!(plan.spring + 0.4 <= room.height);
                }
                for p in &plan.pilasters {
                    for s in &room.sockets {
                        assert!(
                            horizontal(s.position - p.base).length() > s.width * 0.5 + 0.9,
                            "pilaster crowds a doorway in room {}",
                            room.id
                        );
                    }
                    // The pilaster face stays within the player's radius of the wall.
                    let face = p.base + p.inward * (PROJECTION + 0.08);
                    assert!(room.contains(face.x, face.z, 0.0));
                    assert!(!room.contains(face.x, face.z, velvet_dungeon::PLAYER_RADIUS + 0.01));
                }
                ribs += plan.ribs.len();
            }
        }
        assert!(
            ribs > 100,
            "vaulted rooms should get transverse ribs: {ribs}"
        );
    }
}
