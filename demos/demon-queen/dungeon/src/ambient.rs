//! Per-biome floating dust, spores, embers and fireflies. One dynamic mesh
//! covers the portal-visible rooms; the semantic plan never affects layout.
use super::visual;
use velvet_dungeon::{rng::Rng, Blueprint, Catalog, Room};
use velvet_math::Vec3;
use velvet_render::{Camera3D, Material3D, Mesh3D, MeshVertex3D};

pub const MAX_PARTICLES: usize = 256;
const SIDES: usize = 8;
const VERTICES_PER_PARTICLE: usize = SIDES + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dust,
    Spores,
    Embers,
    Fireflies,
}

impl Kind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "dust" => Some(Self::Dust),
            "spores" => Some(Self::Spores),
            "embers" => Some(Self::Embers),
            "fireflies" => Some(Self::Fireflies),
            _ => None,
        }
    }

    fn count(self) -> usize {
        match self {
            Self::Embers => 34,
            Self::Fireflies => 30,
            _ => 26,
        }
    }

    fn size(self) -> f32 {
        match self {
            Self::Embers => 0.055,
            Self::Fireflies => 0.065,
            _ => 0.045,
        }
    }

    fn opacity(self) -> f32 {
        match self {
            Self::Dust => 0.30,
            Self::Spores => 0.56,
            Self::Embers => 0.68,
            Self::Fireflies => 0.72,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub origin: Vec3,
    pub floor: f32,
    pub height: f32,
    pub speed: f32,
    pub phase: f32,
    pub kind: Kind,
    pub color: [f32; 3],
}

pub fn for_room(map: &Blueprint, catalog: &Catalog, room: &Room, glow: [f32; 3]) -> Vec<Particle> {
    let Some(biome) = catalog.biomes.get(room.biome) else {
        return Vec::new();
    };
    let Some(kind) = Kind::from_name(&biome.particles) else {
        return Vec::new();
    };
    let mut rng = Rng::stream(map.seed, "ambient_particles_v1", room.id as u64);
    let color = match kind {
        Kind::Dust => [0.9, 0.78, 0.55],
        Kind::Fireflies => [0.85, 1.0, 0.45],
        _ => glow,
    };
    (0..kind.count())
        .map(|_| {
            let mut origin = room.center;
            for _ in 0..16 {
                let x = room.center.x + rng.range(-0.82, 0.82) * room.half[0];
                let z = room.center.z + rng.range(-0.82, 0.82) * room.half[1];
                if room.contains(x, z, 0.15) {
                    origin.x = x;
                    origin.z = z;
                    break;
                }
            }
            origin.y = room.center.y + 0.3 + rng.range(0.0, (room.height - 0.75).max(0.1));
            Particle {
                origin,
                floor: room.center.y,
                height: room.height,
                speed: rng.range(0.18, 0.58),
                phase: rng.range(0.0, std::f32::consts::TAU),
                kind,
                color,
            }
        })
        .collect()
}

/// Static triangle topology. During play only its positions and vertex alpha
/// are changed; invisible capacity has zero-alpha degenerate triangles.
pub fn mesh() -> Mesh3D {
    let mut mesh = visual::empty();
    mesh.material = Material3D::rgb(1.0, 1.0, 1.0);
    mesh.vertices = vec![
        MeshVertex3D {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0; 2],
            color: [0.0; 4],
        };
        MAX_PARTICLES * VERTICES_PER_PARTICLE
    ];
    for i in 0..MAX_PARTICLES {
        let base = (i * VERTICES_PER_PARTICLE) as u32;
        for side in 0..SIDES {
            mesh.indices.extend([
                base,
                base + 1 + side as u32,
                base + 1 + ((side + 1) % SIDES) as u32,
            ]);
        }
    }
    mesh
}

pub fn animated_vertices(particles: &[Particle], camera: Camera3D, time: f32) -> Vec<MeshVertex3D> {
    let mut vertices = vec![
        MeshVertex3D {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0; 2],
            color: [0.0; 4],
        };
        MAX_PARTICLES * VERTICES_PER_PARTICLE
    ];
    let front = (camera.target - camera.position).normalize_or_zero();
    let right = front.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(front).normalize_or_zero();
    let normal = (-front).to_array();
    for (index, particle) in particles.iter().take(MAX_PARTICLES).enumerate() {
        let mut center = particle.origin;
        center.y = if particle.kind == Kind::Embers {
            let span = (particle.height - 0.6).max(0.1);
            particle.floor
                + 0.3
                + (particle.origin.y - particle.floor + time * particle.speed * 0.6)
                    .rem_euclid(span)
        } else {
            particle.origin.y + (time * particle.speed * 4.0 + particle.phase).sin() * 0.28
        };
        let pulse = 0.82 + 0.18 * (time * (2.0 + particle.speed) + particle.phase).sin();
        let radius = particle.kind.size() * pulse;
        let alpha = particle.kind.opacity() * pulse;
        let base = index * VERTICES_PER_PARTICLE;
        vertices[base] = MeshVertex3D {
            position: center.to_array(),
            normal,
            uv: [0.5, 0.5],
            color: [
                particle.color[0],
                particle.color[1],
                particle.color[2],
                alpha,
            ],
        };
        for side in 0..SIDES {
            let angle = side as f32 * std::f32::consts::TAU / SIDES as f32;
            let p = center + (right * angle.cos() + up * angle.sin()) * radius;
            vertices[base + 1 + side] = MeshVertex3D {
                position: p.to_array(),
                normal,
                uv: [0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5],
                color: [particle.color[0], particle.color[1], particle.color[2], 0.0],
            };
        }
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambient_plan_is_seeded_and_inside_its_room() {
        let catalog = Catalog::bundled();
        for seed in 0..80 {
            let map =
                velvet_dungeon::generate(seed, &Default::default(), &catalog, &Default::default())
                    .unwrap();
            for room in &map.rooms {
                let a = for_room(&map, &catalog, room, [0.5, 0.6, 0.7]);
                let b = for_room(&map, &catalog, room, [0.5, 0.6, 0.7]);
                assert_eq!(a.len(), b.len());
                for (left, right) in a.iter().zip(&b) {
                    assert_eq!(left.origin, right.origin);
                    assert!(room.contains(left.origin.x, left.origin.z, 0.0));
                    assert!(left.origin.y >= room.center.y);
                }
            }
        }
    }

    #[test]
    fn billboard_mesh_has_fixed_capacity_and_finite_animation() {
        let mesh = mesh();
        assert_eq!(mesh.vertices.len(), MAX_PARTICLES * VERTICES_PER_PARTICLE);
        assert_eq!(mesh.indices.len(), MAX_PARTICLES * SIDES * 3);
        let camera = Camera3D::default();
        let vertices = animated_vertices(&[], camera, 3.0);
        assert_eq!(vertices.len(), mesh.vertices.len());
        assert!(vertices
            .iter()
            .all(|v| v.position.iter().all(|n| n.is_finite())));
    }
}
