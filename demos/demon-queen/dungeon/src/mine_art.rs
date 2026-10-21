//! The hand-made mine's art in the scene: its rock and props as modelled (authored.rs names the
//! files), each mesh drawn when it can be seen, and its lamps lit nearest first.
//!
//! The meshes keep the model's textures and the ambient occlusion baked into their vertex
//! colours. A few materials are lifted or made to glow by name (the meshes are named
//! `<kit>:<material>:<cell>` by the mine's build), as the prototype's page does.
use std::collections::BTreeMap;

use velvet_math::{Mat4, Vec3};
use velvet_render::*;

use super::authored::Authored;
use super::Dungeon;
use velvet_dungeon::gameplay::SpawnKind;

/// Beyond this a mesh is not drawn (the fog has it by then).
const FAR: f32 = 70.0;
/// Lamps further than this from the eye are left dark.
const LAMP_REACH: f32 = 34.0;

/// The Heart's chamber's pieces that move (the build names their meshes after them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    /// The Heart's body and the veins of light on its skin: it beats.
    Heart,
    /// The arteries from it into the roof and the floor: they throb with it.
    Vessels,
    /// The veins over the floor out to the walls: they light up just after each beat.
    Veins,
    /// The vast ribcage arching over the chamber: it breathes, and shakes when the Heart rages.
    Ribs,
    /// The bones and arms of those who reached it: they tremble at each beat.
    Remains,
    /// The kneeling iron knight: its eye burns with the beat.
    Knight,
}

fn group_of(name: &str) -> Option<Group> {
    Some(match name.rsplit(':').next()? {
        "heart" => Group::Heart,
        "vessels" => Group::Vessels,
        "veins" => Group::Veins,
        "ribs" => Group::Ribs,
        "remains" => Group::Remains,
        "knight" => Group::Knight,
        _ => return None,
    })
}

/// How the Heart's chamber moves this frame.
#[derive(Clone, Copy, Debug)]
pub struct HeartPose {
    /// The beat: lub-dub, 0 between beats, about 1 at its peak.
    pub thump: f32,
    /// The same beat a moment later, as it runs along the veins.
    pub echo: f32,
    /// 1 while its guardian lives, fading to 0 after it falls (the Heart stops).
    pub alive: f32,
    /// How much it rages (its guardian enraged, the blood rain).
    pub rage: f32,
    pub time: f32,
    pub center: Vec3,
    pub floor: Vec3,
}

/// The Heart's pose from the run: beating fast while she fights its guardian, slow while it waits.
pub fn heart_pose(run: &Dungeon) -> Option<HeartPose> {
    let (center, floor) = run.authored.as_ref()?.heart?;
    let guardian = run
        .actors
        .iter()
        .find(|a| run.map.spawns[a.spawn].kind == SpawnKind::Boss);
    let alive = guardian.map_or(1.0, |a| {
        if a.health > 0.0 {
            1.0
        } else {
            (1.0 - a.death / 3.0).clamp(0.0, 1.0)
        }
    });
    let enraged = guardian.is_some_and(|a| a.enraged && a.health > 0.0);
    let (since, _) = if run.heart.awake {
        let period = if enraged { 0.78 } else { 1.1 };
        ((period - run.heart.beat_in).clamp(0.0, period), period)
    } else {
        (run.elapsed % 1.5, 1.5)
    };
    let lub = |t: f32, at: f32| (-((t - at) / 0.05).powi(2)).exp();
    Some(HeartPose {
        thump: (lub(since, 0.05) + 0.7 * lub(since, 0.27)) * alive,
        echo: (lub(since, 0.17) + 0.7 * lub(since, 0.39)) * alive,
        alive,
        rage: ((run.heart.rain + if enraged { 0.6 } else { 0.0 }) * alive).min(1.0),
        time: run.elapsed,
        center,
        floor,
    })
}

/// Where a moving piece is this frame (a transform before its own) and how much its glow is worth.
fn moved(group: Group, pose: &HeartPose) -> (Mat4, f32) {
    let around = |pivot: Vec3, scale: Vec3| {
        Mat4::from_translation(pivot) * Mat4::from_scale(scale) * Mat4::from_translation(-pivot)
    };
    let t = pose.time;
    let dim = 0.2 + 0.8 * pose.alive;
    match group {
        Group::Heart => {
            let s = 1.0 + 0.075 * pose.thump;
            (
                around(pose.center, Vec3::new(s, 1.0 + 0.09 * pose.thump, s)),
                (0.35 + 1.6 * pose.thump) * dim,
            )
        }
        Group::Vessels => {
            let s = 1.0 + 0.02 * pose.thump;
            (around(pose.center, Vec3::new(s, s, s)), 1.0)
        }
        Group::Veins => (Mat4::IDENTITY, (0.25 + 1.8 * pose.echo) * dim),
        Group::Ribs => {
            let breath = 0.012 * (t * 0.9).sin() * (0.3 + 0.7 * pose.alive);
            let shake = 0.006 * pose.rage * (t * 23.0).sin();
            let s = 1.0 + breath + shake;
            (around(pose.floor, Vec3::new(s, 1.0 + breath * 0.6, s)), 1.0)
        }
        Group::Remains => {
            let lift = 0.03 * pose.thump + 0.025 * pose.rage * (t * 29.0).sin().abs();
            (Mat4::from_translation(Vec3::new(0.0, lift, 0.0)), 1.0)
        }
        Group::Knight => (Mat4::IDENTITY, (0.35 + 1.3 * pose.thump) * dim),
    }
}

struct Part {
    group: Option<Group>,
    mesh: MeshHandle,
    transform: Mat4,
    material: Material3D,
    texture: Option<TextureHandle>,
    center: Vec3,
    radius: f32,
    triangles: usize,
}

/// The mine's meshes on the GPU.
pub struct MineArt {
    /// The rock's file: the art belongs to that mine.
    pub key: String,
    parts: Vec<Part>,
}

/// How a material of the mine is shown: its colour lifted (the scanned rock is very dark) and how
/// much it glows.
fn look(material: &str) -> (f32, f32) {
    let lift = match material {
        "rock" => 2.1,
        "rock_hall" => 2.3,
        "cliff" => 2.6,
        "rock_grotto" => 1.4,
        "floor" => 1.35,
        "ground" => 1.5,
        "stone" | "rock_root" | "rock_heart" | "stone_dark" => 1.5,
        "stone_white" => 1.4,
        "mud" => 1.2,
        _ => 1.0,
    };
    let glow = match material {
        "flame" => 5.0,
        "ember" => 2.4,
        "candle" => 0.6,
        "glass" => 1.3,
        "crystal" => 0.32,
        "crystal_dim" => 0.18,
        "heart_crystal" => 0.5,
        "heartglow" => 0.7,
        "rune" => 1.4,
        "holy" => 1.0,
        "arcane" => 1.0,
        "slime_blue" | "slime_green" => 0.5,
        "slime_core" => 1.0,
        "fungus_cap" | "fungus_cap_violet" => 0.7,
        "fungus_spot" => 0.8,
        _ => 0.0,
    };
    (lift, glow)
}

/// The material's name in a mesh's name (`props:crystal:3_5` → `crystal`).
fn material_of(name: &str) -> &str {
    let mut parts = name.split(':');
    let first = parts.next().unwrap_or("");
    parts.next().unwrap_or(first)
}

impl MineArt {
    /// Reads the mine's rock and props and puts them on the GPU.
    pub fn load(
        mine: &Authored,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<Self> {
        let started = std::time::Instant::now();
        let root = super::assets::root();
        let mut parts = Vec::new();
        let mut uploaded: BTreeMap<(usize, usize, Option<usize>), TextureHandle> = BTreeMap::new();
        for (file_index, file) in [&mine.rock, &mine.props].into_iter().enumerate() {
            let model = GltfModel::load(root.join(file))?;
            for (i, mesh) in model.meshes.iter().enumerate() {
                if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                    continue;
                }
                let name = model.mesh_names.get(i).map(String::as_str).unwrap_or("");
                let (lift, glow) = look(material_of(name));
                let transform = model.mesh_transform(i);
                let texture = match mesh
                    .texture_index
                    .and_then(|t| model.textures.get(t).map(|d| (t, d)))
                {
                    Some((t, data)) => {
                        let key = (file_index, t, mesh.normal_texture_index);
                        match uploaded.get(&key) {
                            Some(handle) => Some(*handle),
                            None => {
                                let normal = mesh
                                    .normal_texture_index
                                    .and_then(|n| model.textures.get(n));
                                let prepared = PreparedTexture3D::new(data, normal)?;
                                let handle =
                                    renderer.upload_prepared_texture(device, queue, &prepared);
                                uploaded.insert(key, handle);
                                Some(handle)
                            }
                        }
                    }
                    None => None,
                };
                let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                let mut high = -low;
                for v in &mesh.vertices {
                    let p = transform.transform_point3(Vec3::new(
                        v.position[0],
                        v.position[1],
                        v.position[2],
                    ));
                    low = Vec3::new(low.x.min(p.x), low.y.min(p.y), low.z.min(p.z));
                    high = Vec3::new(high.x.max(p.x), high.y.max(p.y), high.z.max(p.z));
                }
                let mut material = mesh.material;
                for c in 0..3 {
                    material.base_color[c] = (material.base_color[c] * lift).min(1.0);
                }
                material = if glow > 0.0 {
                    material.with_emission(glow)
                } else {
                    material.with_ambient_occlusion(0.35)
                };
                parts.push(Part {
                    group: group_of(name),
                    mesh: renderer.upload_mesh(device, mesh)?,
                    transform,
                    material,
                    texture,
                    center: (low + high) * 0.5,
                    radius: (high - low).length() * 0.5,
                    triangles: mesh.indices.len() / 3,
                });
            }
        }
        let load_ms = started.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "[mina] {} meshes, {} textures in {:.0} ms",
            parts.len(),
            uploaded.len(),
            load_ms
        );
        Ok(Self {
            key: mine.rock.clone(),
            parts,
        })
    }

    /// Adds what the eye can see: near enough and inside the view's cone. Returns the triangles.
    pub fn draw(
        &self,
        camera: Camera3D,
        aspect: f32,
        heart: Option<&HeartPose>,
        draws: &mut Vec<MeshDraw3D>,
    ) -> usize {
        let eye = camera.position;
        let forward = (camera.target - eye).normalize_or_zero();
        // the cone round the view: half its diagonal angle
        let half_y = camera.fov_y_radians * 0.5;
        let half = (half_y.tan() * (1.0 + aspect * aspect).sqrt()).atan();
        let mut triangles = 0;
        for part in &self.parts {
            let to = part.center - eye;
            let distance = to.length();
            if distance - part.radius > FAR {
                continue;
            }
            if distance > part.radius {
                let angle = (to.dot(forward) / distance).clamp(-1.0, 1.0).acos();
                let spread = (part.radius / distance).clamp(-1.0, 1.0).asin();
                if angle - spread > half {
                    continue;
                }
            }
            let (transform, material) = match (part.group, heart) {
                (Some(group), Some(pose)) => {
                    let (motion, glow) = moved(group, pose);
                    let mut material = part.material;
                    if material.occlusion_window[0] > 0.0 {
                        material.occlusion_window[0] *= glow;
                    }
                    (motion * part.transform, material)
                }
                _ => (part.transform, part.material),
            };
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform,
                material,
                texture: part.texture,
            });
            triangles += part.triangles;
        }
        triangles
    }
}

/// The mine's lamps near the eye, flickering where they burn.
pub fn lamps(
    mine: &Authored,
    eye: Vec3,
    elapsed: f32,
    heart: Option<&HeartPose>,
    lights: &mut Vec<PointLight3D>,
) {
    for (i, lamp) in mine.lamps.iter().enumerate() {
        if (lamp.position - eye).length() > LAMP_REACH + lamp.distance * 0.5 {
            continue;
        }
        let flicker = match lamp.kind.as_str() {
            "lantern" | "candle" | "fire" | "brazier" | "forge" => {
                let t = elapsed * 9.1 + i as f32 * 1.7;
                1.0 - 0.07 * (0.5 + 0.5 * t.sin() * (elapsed * 5.3 + i as f32).sin())
            }
            _ => 1.0,
        };
        // the page's lamps are physical; these are the game's: the Heart's red is far stronger here
        // the Heart's own light swells with each beat and dies with it
        let beat = match (lamp.kind.as_str(), heart) {
            ("heart", Some(pose)) => (0.45 + 1.1 * pose.thump) * (0.25 + 0.75 * pose.alive),
            _ => 1.0,
        };
        let strength = beat
            * match lamp.kind.as_str() {
                "heart" => 0.3,
                "fill" => 0.8,
                "crystal" | "slime" | "fungus" => 0.75,
                _ => 1.0,
            };
        lights.push(PointLight3D::new(
            lamp.position,
            lamp.color,
            lamp.intensity * flicker * strength,
            lamp.distance,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_names_give_their_material() {
        assert_eq!(material_of("props:crystal:3_5"), "crystal");
        assert_eq!(material_of("cave"), "cave");
        assert_eq!(look("flame").1, 5.0);
        assert!(look("rock").0 > 1.0 && look("rock").1 == 0.0);
    }
}
