//! Deterministic cave formations and wall torch mounts (web prototype
//! `dressing.js`). Formations stay clear of sockets and headroom; torches
//! hang on the wall beside the openings.
use super::kit::*;
use velvet_dungeon::{Blueprint, Room, Shape, Socket};
use velvet_math::Vec3;

const WALL_INSET: f32 = 0.03;
const POST_CLEARANCE: f32 = 0.74;
const TORCH_BASE_HEIGHT: f32 = 1.59;
const TORCH_FLAME_HEIGHT: f32 = 0.63;
const TORCH_FLAME_REACH: f32 = 0.1;

#[derive(Clone, Copy, Debug)]
pub struct Torch {
    pub backplate: Vec3,
    pub flame: Vec3,
    pub inward: Vec3,
    pub yaw: f32,
}

/// Torch on one side (-1 left / +1 right of the opening, seen from the room).
pub fn wall_torch_on_side(socket: &Socket, sign: f32) -> Torch {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    let inward = -outward;
    let side = v(-outward.z, 0.0, outward.x);
    let face = socket.position + inward * WALL_INSET;
    let backplate = face
        + side * (sign.signum() * (socket.width * 0.5 + POST_CLEARANCE))
        + Y(TORCH_BASE_HEIGHT);
    Torch {
        backplate,
        flame: backplate + inward * TORCH_FLAME_REACH + Y(TORCH_FLAME_HEIGHT),
        inward,
        yaw: inward.x.atan2(inward.z),
    }
}

/// First side whose mount lies on the straight wall carrying the socket.
pub fn wall_torch_for(room: &Room, socket: &Socket) -> Option<Torch> {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    if outward.length_squared() < 0.5 {
        return None;
    }
    let side = v(-outward.z, 0.0, outward.x);
    for sign in [-1.0f32, 1.0] {
        let mount = wall_torch_on_side(socket, sign);
        let on_wall = [-0.25f32, 0.25].iter().all(|&offset| {
            let p = mount.backplate + side * offset + mount.inward * 0.12;
            room.contains(p.x, p.z, 0.02)
        });
        if on_wall {
            return Some(mount);
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Outcrop,
    Stalagmites,
    Crystals,
    Stalactites,
}

#[derive(Clone, Copy, Debug)]
pub struct Formation {
    pub feature: Kind,
    pub position: Vec3,
    pub size: Vec3,
    pub yaw: f32,
    pub seed: Seed,
}

/// Wall outcrops, stalagmites, crystal clusters and ceiling stalactites.
pub fn formations(map: &Blueprint, room: &Room) -> Vec<Formation> {
    let mut rng = ArtRng::stream(map.seed, "tripo_dressing_v1", room.id as u64);
    let mut out = vec![];
    let density = map.config.decoration_density.clamp(0.0, 2.0);
    if density == 0.0 {
        return out;
    }
    if room.shape != Shape::Vault {
        let n = room.outline.len();
        for i in 0..n {
            let a = v(
                room.center.x + room.outline[i][0],
                room.center.y,
                room.center.z + room.outline[i][1],
            );
            let q = room.outline[(i + 1) % n];
            let b = v(room.center.x + q[0], room.center.y, room.center.z + q[1]);
            let length = (b - a).length();
            let tangent = norm(b - a);
            let inward = v(-tangent.z, 0.0, tangent.x);
            let count = (length / 6.5 * density).ceil() as usize;
            for j in 0..count {
                if rng.chance(0.35) {
                    continue;
                }
                let at = length * (j as f32 + rng.range(0.35, 0.65)) / count as f32;
                let mut p = a + tangent * at;
                p = v(p.x, room.floor_y(p.x, p.z), p.z);
                let width = (length / count as f32 * 0.8).min(rng.range(2.3, 3.6));
                if room
                    .sockets
                    .iter()
                    .any(|s| (p - s.position).length() < width * 0.5 + s.width * 0.5 + 1.35)
                {
                    continue;
                }
                let height = rng.range(1.7, room.height.min(3.5));
                let depth = 0.34;
                let yaw = inward.x.atan2(inward.z);
                let seed = rng.next_seed();
                out.push(Formation {
                    feature: Kind::Outcrop,
                    position: p - inward * (depth * 0.5 - 0.1),
                    size: v(width, height, depth),
                    yaw,
                    seed,
                });
                let foot_height = rng.range(0.55, 1.05);
                let foot_depth = 0.26;
                let seed = rng.next_seed();
                out.push(Formation {
                    feature: if room.biome == 1 && j % 2 == 0 {
                        Kind::Crystals
                    } else {
                        Kind::Stalagmites
                    },
                    position: p - inward * (foot_depth * 0.5 - 0.1),
                    size: v(width.min(1.5), foot_height, foot_depth),
                    yaw,
                    seed,
                });
            }
        }
    }
    let islands = ((3.0 * density).ceil() as usize).min(6);
    for _ in 0..islands {
        let x = rng.range(-0.62, 0.62) * room.half[0];
        let z = rng.range(-0.62, 0.62) * room.half[1];
        let p = v(room.center.x + x, room.center.y, room.center.z + z);
        if !room.contains(p.x, p.z, 1.2)
            || room.sockets.iter().any(|s| (s.position - p).length() < 2.6)
        {
            continue;
        }
        // Bury the cut cap beyond the recessed roof backing.
        let top = room.center.y + room.height + 0.93;
        let h = rng.range(0.8, 1.65).min(top - room.center.y - 3.2);
        let yaw = rng.range(0.0, std::f32::consts::TAU);
        let seed = rng.next_seed();
        out.push(Formation {
            feature: Kind::Stalactites,
            position: v(p.x, top - h, p.z),
            size: v(h * 1.4, h, h * 1.25),
            yaw,
            seed,
        });
    }
    out
}
