//! Standing cave water. CPU-updated normals and shoreline colors feed the
//! native translucent mesh pipeline; the blueprint and collision stay fixed.
use super::themes::Theme;
use velvet_dungeon::{rng::Rng, Blueprint, Room, Shape};
use velvet_math::Vec3;
use velvet_render::{Material3D, Mesh3D, MeshVertex3D};

const GRAVITY: f32 = 9.8;
const RING_SPEED: f32 = 0.42;

#[derive(Clone, Copy, Debug)]
pub struct Drip {
    pub x: f32,
    pub z: f32,
    pub top: f32,
    pub level: f32,
    pub period: f32,
    pub phase: f32,
    pub fall: f32,
}
#[derive(Clone, Debug)]
pub struct Surface {
    pub center: Vec3,
    pub outline: Vec<Vec3>,
    pub color: [f32; 3],
    pub reflect: [f32; 3],
    pub depth: f32,
    pub halo: f32,
    pub drip: Option<Drip>,
}
pub struct WaterGpu {
    pub handle: velvet_render::MeshHandle,
    pub base: Vec<MeshVertex3D>,
    pub surface: Surface,
    pub triangles: usize,
}

fn hash(x: f32, z: f32) -> f32 {
    let s = (x * 127.1 + z * 311.7).sin() * 43758.547;
    s - s.floor()
}
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn palette(theme: Theme) -> Option<([f32; 3], [f32; 3], f32, f32, bool)> {
    use Theme::*;
    match theme {
        Overgrowth => Some(([0.024, 0.050, 0.035], [0.08, 0.17, 0.13], 0.9, 1.08, true)),
        Fungal => Some(([0.025, 0.070, 0.080], [0.06, 0.22, 0.24], 0.52, 1.2, true)),
        Mine => Some(([0.035, 0.045, 0.052], [0.13, 0.14, 0.16], 0.58, 1.12, false)),
        Forge => Some(([0.035, 0.045, 0.050], [0.19, 0.11, 0.07], 0.72, 1.0, false)),
        Library => Some(([0.042, 0.049, 0.053], [0.17, 0.14, 0.10], 0.42, 1.16, true)),
        Memory => Some(([0.039, 0.055, 0.062], [0.11, 0.16, 0.18], 0.50, 1.2, true)),
        Crypt => Some(([0.039, 0.048, 0.058], [0.15, 0.14, 0.17], 0.35, 1.18, false)),
        _ => None,
    }
}
/// One authored water beat per eligible room, chosen on its own RNG stream.
/// Placement stays off doors, pillars and the centre navigation lane.
pub fn for_room(map: &Blueprint, room: &Room) -> Option<Surface> {
    if room.shape == Shape::Bridge {
        return None;
    }
    let theme = super::themes::of(map, room)?;
    let (color, reflect, depth, halo, leak) = palette(theme)?;
    let mut rng = Rng::stream(map.seed, "water_v1", room.id as u64);
    if theme == Theme::Crypt && !rng.chance(0.65) {
        return None;
    }
    let radius = match theme {
        Theme::Overgrowth => rng.range(0.85, 1.35),
        Theme::Forge => rng.range(0.48, 0.75),
        _ => rng.range(0.50, 0.96),
    };
    let axis = room
        .sockets
        .first()
        .map(|s| {
            let d = s.position - room.center;
            Vec3::new(d.x, 0.0, d.z).normalize_or_zero()
        })
        .unwrap_or(Vec3::Z);
    let side = Vec3::new(-axis.z, 0.0, axis.x);
    let mut chosen = None;
    for _ in 0..18 {
        let along = rng.range(-0.35, 0.35) * room.half[1].min(5.0);
        let lateral = rng.range(0.33, 0.55)
            * room.half[0].min(6.0)
            * if rng.chance(0.5) { -1.0 } else { 1.0 };
        let candidate = room.center + axis * along + side * lateral;
        let clear =
            room.contains(candidate.x, candidate.z, radius + 0.4)
                && room
                    .sockets
                    .iter()
                    .all(|s| (candidate - s.position).length() > radius + 1.9)
                && map.obstacles.iter().filter(|o| o.room == room.id).all(|o| {
                    (candidate - o.center).length() > radius + o.half[0].max(o.half[1]) + 0.3
                });
        if clear {
            chosen = Some(candidate);
            break;
        }
    }
    let mut center = chosen?;
    center.y = room.floor_y(center.x, center.z) + 0.018;
    let count = 24;
    let outline = (0..count)
        .map(|k| {
            let a = k as f32 / count as f32 * std::f32::consts::TAU;
            let rough = 0.78 + rng.range(0.0, 0.3);
            center + Vec3::new(a.cos() * radius * rough, 0.0, a.sin() * radius * rough)
        })
        .collect::<Vec<_>>();
    let drip = if leak && rng.chance(0.7) {
        let top = room.center.y + room.height - 0.16;
        let period = 1.7 + 1.3 * hash(center.x, center.z);
        let phase = period * hash(center.z + 3.1, center.x - 1.7);
        let fall = ((2.0 * (top - center.y).max(0.2)) / GRAVITY).sqrt();
        Some(Drip {
            x: center.x,
            z: center.z,
            top,
            level: center.y,
            period,
            phase,
            fall,
        })
    } else {
        None
    };
    Some(Surface {
        center,
        outline,
        color,
        reflect,
        depth,
        halo,
        drip,
    })
}

/// A water surface planned by the ported cave art (web `pond`).
pub fn from_pond(p: &super::art::plan::Pond) -> Surface {
    let c = p.center;
    let drip = p.drip.map(|top| {
        let period = 1.7 + 1.3 * hash(c.x, c.z);
        let phase = period * hash(c.z + 3.1, c.x - 1.7);
        Drip {
            x: c.x,
            z: c.z,
            top,
            level: c.y,
            period,
            phase,
            fall: ((2.0 * (top - c.y).max(0.2)) / GRAVITY).sqrt(),
        }
    });
    Surface {
        center: c,
        outline: p.outline.clone(),
        color: p.color,
        reflect: p.reflect,
        depth: p.depth,
        halo: p.halo,
        drip,
    }
}

/// Dense radial tessellation lets normal ripples travel across the surface,
/// not just pulse the centre vertex.
pub fn mesh(surface: &Surface) -> Mesh3D {
    let n = surface.outline.len();
    let rings = 9;
    let mut vertices = Vec::with_capacity((rings + 1) * n);
    let mut indices = Vec::with_capacity(rings * n * 6);
    for ring in 0..=rings {
        let wet = ring.min(rings - 1) as f32 / (rings - 1) as f32;
        let factor = if ring == rings { surface.halo } else { wet };
        let shore = if ring == rings {
            0.0
        } else {
            1.0 + (1.0 - wet)
        };
        for p in &surface.outline {
            let point = surface.center + (*p - surface.center) * factor;
            let mut vertex =
                MeshVertex3D::new(point.to_array(), Vec3::Y.to_array(), [shore, surface.depth]);
            vertex.color = [surface.color[0], surface.color[1], surface.color[2], 1.0];
            vertices.push(vertex);
        }
    }
    for ring in 0..rings {
        for k in 0..n {
            let a = (ring * n + k) as u32;
            let b = (ring * n + (k + 1) % n) as u32;
            let c = ((ring + 1) * n + k) as u32;
            let d = ((ring + 1) * n + (k + 1) % n) as u32;
            // Shoreline input is counter-clockwise viewed from above.
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D {
            base_color: [1.0, 1.0, 1.0, 0.92],
            texture: None,
            occlusion_window: [0.0; 4],
        },
        metallic_factor: 0.0,
        roughness_factor: 0.05,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

pub fn animated_vertices(
    base: &[MeshVertex3D],
    s: &Surface,
    time: f32,
    camera: Vec3,
) -> Vec<MeshVertex3D> {
    base.iter()
        .map(|original| {
            let mut v = *original;
            let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
            let shore = v.uv[0];
            let body = smoothstep(0.96, 1.10, shore);
            let deep = smoothstep(1.0, 2.0, shore) * s.depth;
            let mut slope = Vec3::new(
                0.02 * (p.x * 6.1 + time * 0.9).cos(),
                0.0,
                0.02 * (p.z * 5.3 - time * 0.7).cos(),
            );
            let w = p.x * 4.3 - p.z * 7.9 + time * 1.4;
            slope += Vec3::new(4.3, 0.0, -7.9) * (0.012 * w.cos() / 9.0);
            let w2 = p.x * 11.0 + p.z * 9.0 - time * 2.1;
            slope += Vec3::new(11.0, 0.0, 9.0) * (0.006 * w2.cos() / 14.2);
            if let Some(drip) = s.drip {
                let age = (time + drip.phase).rem_euclid(drip.period);
                let dx = p.x - drip.x;
                let dz = p.z - drip.z;
                let r = (dx * dx + dz * dz).sqrt().max(0.0001);
                for k in 0..2 {
                    let front = (age - k as f32 * 0.16) * RING_SPEED;
                    if front < 0.0 {
                        continue;
                    }
                    let band = r - front;
                    let ring = (band * 55.0).cos()
                        * (-band * band * 900.0).exp()
                        * (-age * 1.7).exp()
                        * (1.0 - k as f32 * 0.45);
                    slope += Vec3::new(dx / r, 0.0, dz / r) * (ring * 0.55);
                }
            }
            let n = Vec3::new(-slope.x * body, 1.0, -slope.z * body).normalize_or_zero();
            v.normal = n.to_array();
            let view = (camera - p).normalize_or_zero();
            let fresnel = (1.0 - n.dot(view).clamp(0.0, 1.0)).powi(4);
            // The native toon pipeline has no environment cubemap; retain the
            // source Fresnel curve and a restrained ambient reflection floor.
            let mirror = (0.48 + 1.3 * fresnel) * body;
            let shore_glint = (-((shore - 1.0) / 0.10).powi(2)).exp();
            let dark = smoothstep(0.0, 1.0, shore);
            let wet = [0.012, 0.011, 0.010];
            for i in 0..3 {
                v.color[i] = (wet[i] * (1.0 - body)
                    + s.color[i] * body
                    + s.reflect[i] * (mirror + shore_glint * 0.88))
                    .clamp(0.0, 1.0);
            }
            v.color[3] =
                ((0.45 * dark) * (1.0 - body) + (0.50 + 0.42 * deep) * body).clamp(0.0, 0.96);
            v
        })
        .collect()
}
pub fn drop_height(drip: Drip, time: f32) -> Option<f32> {
    let phase = (time + drip.phase).rem_euclid(drip.period);
    let start = drip.period - drip.fall;
    if phase < start - 0.35 {
        return None;
    }
    if phase < start {
        return Some(drip.top);
    }
    let falling = phase - start;
    Some(drip.level.max(drip.top - 0.5 * GRAVITY * falling * falling))
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};
    #[test]
    fn water_mesh_and_drips_are_finite_across_seeds() {
        let catalog = Catalog::bundled();
        for seed in 0..100 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let Some(s) = for_room(&map, room) else {
                    continue;
                };
                assert!(s.outline.iter().all(|p| room.contains(p.x, p.z, 0.15)));
                let mesh = mesh(&s);
                assert!(!mesh.indices.is_empty());
                assert!(mesh
                    .indices
                    .iter()
                    .all(|i| (*i as usize) < mesh.vertices.len()));
                let triangle = &mesh.indices[s.outline.len() * 6..s.outline.len() * 6 + 3];
                let point = |i: u32| {
                    let p = mesh.vertices[i as usize].position;
                    Vec3::new(p[0], p[1], p[2])
                };
                let [a, b, c] = [point(triangle[0]), point(triangle[1]), point(triangle[2])];
                assert!((b - a).cross(c - a).y > 0.0, "seed {seed} room {}", room.id);
                let next = animated_vertices(&mesh.vertices, &s, 12.4, room.center + Vec3::Y);
                assert!(next.iter().all(|v| v
                    .position
                    .iter()
                    .chain(v.normal.iter())
                    .chain(v.color.iter())
                    .all(|x| x.is_finite())));
                if let Some(drip) = s.drip {
                    assert!(drop_height(drip, 0.0).is_none_or(|y| y >= drip.level && y <= drip.top));
                }
            }
        }
    }
}
