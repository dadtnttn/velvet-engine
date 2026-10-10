//! Deterministic placement for procedural cave formations. Layout/navigation
//! RNG never depends on art; formations stay clear of sockets and headroom.
use velvet_dungeon::{rng::Rng, Blueprint, Shape, Socket};
use velvet_math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub feature: Feature,
    pub position: Vec3,
    pub size: Vec3,
    pub yaw: f32,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    WallOutcrop,
    Stalagmites,
    Stalactites,
    CrystalCluster,
}

// The torch hangs on the wall itself, just outside the procedural arch post
// (`visual::doorway_frame`: posts end at width/2 + 0.48 m, 0.29 m deep).
// Wall relief only recedes behind the outline, so the outline is the face.
const WALL_INSET: f32 = 0.03;
const POST_CLEARANCE: f32 = 0.74;
const TORCH_BASE_HEIGHT: f32 = 1.59;
const TORCH_FLAME_HEIGHT: f32 = 0.63;
const TORCH_FLAME_REACH: f32 = 0.10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallTorchMount {
    /// Back plane of the mounted torch, touching the wall beside the arch.
    pub backplate: Vec3,
    /// Center of the flame and its point light.
    pub flame: Vec3,
    pub inward: Vec3,
    pub yaw: f32,
}

/// Torch on one side (`-1` left / `+1` right of the opening, seen from the
/// room). The caller rejects a side whose wall segment is too short.
pub fn wall_torch_on_side(s: &Socket, sign: f32) -> WallTorchMount {
    let outward = Vec3::new(s.outward.x, 0.0, s.outward.z).normalize_or_zero();
    let inward = -outward;
    let side = Vec3::new(-outward.z, 0.0, outward.x);
    let wall_face = s.position + inward * WALL_INSET;
    let backplate = wall_face
        + side * (sign.signum() * (s.width * 0.5 + POST_CLEARANCE))
        + Vec3::Y * TORCH_BASE_HEIGHT;
    let yaw = inward.x.atan2(inward.z);
    WallTorchMount {
        backplate,
        flame: backplate + inward * TORCH_FLAME_REACH + Vec3::Y * TORCH_FLAME_HEIGHT,
        inward,
        yaw,
    }
}

/// First side whose mount lies on the straight wall carrying the socket, i.e.
/// the point 25 cm to either side along the wall is still on the outline.
pub fn wall_torch_for(room: &velvet_dungeon::Room, s: &Socket) -> Option<WallTorchMount> {
    let outward = Vec3::new(s.outward.x, 0.0, s.outward.z).normalize_or_zero();
    let side = Vec3::new(-outward.z, 0.0, outward.x);
    [-1.0, 1.0]
        .into_iter()
        .map(|sign| wall_torch_on_side(s, sign))
        .find(|mount| {
            [-0.25, 0.25].into_iter().all(|offset| {
                let p = mount.backplate + side * offset + mount.inward * 0.12;
                room.contains(p.x, p.z, 0.02)
            })
        })
}

pub fn room(map: &Blueprint, id: usize) -> Vec<Placement> {
    let r = &map.rooms[id];
    let mut rng = Rng::stream(map.seed, "tripo_dressing_v1", id as u64);
    let mut out = Vec::new();
    let density = map.config.decoration_density.clamp(0.0, 2.0);
    if density == 0.0 {
        return out;
    }
    if r.shape != Shape::Vault {
        for i in 0..r.outline.len() {
            let a = r.center + Vec3::new(r.outline[i][0], 0.0, r.outline[i][1]);
            let q = r.outline[(i + 1) % r.outline.len()];
            let b = r.center + Vec3::new(q[0], 0.0, q[1]);
            let length = (b - a).length();
            let tangent = (b - a).normalize_or_zero();
            let inward = Vec3::new(-tangent.z, 0.0, tangent.x);
            let count = (length / 6.5 * density).ceil() as usize;
            for j in 0..count {
                // Accent formations, not identical columns covering every wall.
                if rng.chance(0.35) {
                    continue;
                }
                let at = length * (j as f32 + rng.range(0.35, 0.65)) / count as f32;
                let mut p = a + tangent * at;
                p.y = r.floor_y(p.x, p.z);
                let width = (length / count as f32 * 0.80).min(rng.range(2.3, 3.6));
                // Reserve whole art footprint plus landing clearance at doors.
                if r.sockets.iter().any(|s| {
                    let d = p - s.position;
                    d.length() < width * 0.5 + s.width * 0.5 + 1.35
                }) {
                    continue;
                }
                let height = rng.range(1.7, r.height.min(3.5));
                // Shallow wall relief, not a freestanding boulder shoved behind
                // the shell. Keeps the sculpted face visible without changing
                // the validated walking volume or producing clipped fragments.
                let depth = 0.34;
                out.push(Placement {
                    feature: Feature::WallOutcrop,
                    position: p - inward * (depth * 0.5 - 0.10),
                    size: Vec3::new(width, height, depth),
                    yaw: inward.x.atan2(inward.z),
                    seed: rng.next_u64(),
                });
                // Smaller formations at the foot of the wall, also entirely
                // behind its capsule boundary (at most 10 cm of relief).
                let height = rng.range(0.55, 1.05);
                let depth = 0.26;
                out.push(Placement {
                    feature: if r.biome == 1 && j % 2 == 0 {
                        Feature::CrystalCluster
                    } else {
                        Feature::Stalagmites
                    },
                    position: p - inward * (depth * 0.5 - 0.10),
                    size: Vec3::new(width.min(1.5), height, depth),
                    yaw: inward.x.atan2(inward.z),
                    seed: rng.next_u64(),
                });
            }
        }
    }
    // Ceiling islands vary without filling the player's sightlines. No more
    // than six groups per room and none in a doorway's landing zone.
    for _ in 0..(3.0 * density).ceil().min(6.0) as usize {
        let p = r.center
            + Vec3::new(
                rng.range(-0.62, 0.62) * r.half[0],
                0.0,
                rng.range(-0.62, 0.62) * r.half[1],
            );
        if !r.contains(p.x, p.z, 1.2) || r.sockets.iter().any(|s| (s.position - p).length() < 2.6) {
            continue;
        }
        // Bury the cut cap beyond the maximum recessed roof backing (0.9 m).
        // A larger hidden portion is cheaper and safer than a floating flat cap.
        let top = r.center.y + r.height + 0.93;
        let h = rng.range(0.8, 1.65).min(top - r.center.y - 3.2);
        out.push(Placement {
            feature: Feature::Stalactites,
            position: Vec3::new(p.x, top - h, p.z),
            size: Vec3::new(h * 1.4, h, h * 1.25),
            yaw: rng.range(0.0, std::f32::consts::TAU),
            seed: rng.next_u64(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config, PLAYER_HEIGHT};
    #[test]
    fn wall_torches_hang_on_the_socket_wall_beside_the_arch() {
        let catalog = Catalog::bundled();
        let mut mounted = 0;
        for seed in 0..32 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for r in &map.rooms {
                for s in &r.sockets {
                    let Some(mount) = wall_torch_for(r, s) else {
                        continue;
                    };
                    mounted += 1;
                    let outward = Vec3::new(s.outward.x, 0.0, s.outward.z).normalize_or_zero();
                    // Backplate lies on the wall plane (3 cm proud), not in the room.
                    let depth = (mount.backplate - s.position).dot(-outward);
                    assert!(
                        (depth - WALL_INSET).abs() < 1.0e-3,
                        "torch floats {depth} m off the wall"
                    );
                    // Beside the procedural arch post, never over the opening.
                    let side = Vec3::new(-outward.z, 0.0, outward.x);
                    let lateral = (mount.backplate - s.position).dot(side).abs();
                    assert!(lateral >= s.width * 0.5 + 0.55);
                    assert!(r.contains(mount.flame.x, mount.flame.z, 0.05));
                }
            }
        }
        assert!(
            mounted > 200,
            "most sockets should carry a torch: {mounted}"
        );
    }

    #[test]
    fn dressing_is_repeatable_and_keeps_headroom() {
        let catalog = Catalog::bundled();
        for seed in 0..32 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for r in &map.rooms {
                let a = room(&map, r.id);
                assert_eq!(a, room(&map, r.id));
                assert!(a.len() < 80);
                for p in &a {
                    assert!(p.size.to_array().iter().all(|v| v.is_finite() && *v > 0.0));
                    if p.feature == Feature::Stalactites {
                        // Max jump rise = 5.4^2/(2*18) = 0.81 m.
                        assert!(p.position.y - r.center.y > PLAYER_HEIGHT + 0.81 + 0.5);
                        assert!(p.position.y + p.size.y > r.center.y + r.height + 0.9);
                    } else {
                        // Every wall-art bounding corner stays outside the
                        // inner 11 cm of the walkable polygon, even at bends.
                        for x in [-0.5, 0.5] {
                            for z in [-0.5, 0.5] {
                                let dx = x * p.size.x;
                                let dz = z * p.size.z;
                                let corner = p.position
                                    + Vec3::new(
                                        dx * p.yaw.cos() + dz * p.yaw.sin(),
                                        0.0,
                                        -dx * p.yaw.sin() + dz * p.yaw.cos(),
                                    );
                                assert!(
                                    !r.contains(corner.x, corner.z, 0.111),
                                    "wall art intrudes into room {} seed {seed}",
                                    r.id
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
