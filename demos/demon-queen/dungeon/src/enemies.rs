//! Data-driven KayKit enemy models and their shared animation rig.
use std::{collections::BTreeMap, fs};

use serde::Deserialize;
use velvet_math::{Mat4, Vec3};
use velvet_render::*;

use super::anim::{attacks, rig};
use super::assets::root;
use super::combat::Loadout;

#[derive(Deserialize)]
struct Manifest {
    animation_sets: Vec<String>,
    models: Vec<ModelDefinition>,
    roles: Vec<RoleDefinition>,
}

#[derive(Deserialize)]
struct ModelDefinition {
    id: String,
    path: String,
    height_m: Option<f32>,
    /// A body with its own proportions on the shared rig's bones (the realistic skeletons): the
    /// shared clips only turn its bones (see `GltfModel::append_rotations_from`).
    #[serde(default)]
    own_proportions: bool,
    /// The model it stands in for: once loaded it takes that one's id, so every foe asking for the
    /// old model (and the weapons fitted to it) gets this one.
    #[serde(default)]
    replaces: Option<String>,
}

#[derive(Deserialize)]
struct RoleDefinition {
    id: String,
    variants: Vec<String>,
    /// The models the role falls back on when none of its variants loaded.
    #[serde(default)]
    fallback: Vec<String>,
}

struct Model {
    source: GltfModel,
    normalize: Mat4,
    target_height: f32,
    fracture_anchor: Vec3,
    /// The body's fracture anchor in the model's own units.
    raw_fracture_anchor: Option<Vec3>,
    /// Lowest and highest rest-pose points of the bare body.
    rest_bounds: (Vec3, Vec3),
    textures: Vec<TextureHandle>,
    triangles: Vec<usize>,
    parts: Vec<EnemyPart>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum EnemyPart {
    LeftArm,
    RightArm,
    Head,
    #[default]
    Other,
}

impl EnemyPart {
    fn from_gltf_name(name: &str) -> Self {
        if name.ends_with("_ArmLeft") {
            Self::LeftArm
        } else if name.ends_with("_ArmRight") {
            Self::RightArm
        } else if name.ends_with("_Head") || name.ends_with("_Skull") || name.ends_with("_Jaw") {
            Self::Head
        } else {
            Self::Other
        }
    }
}

#[derive(Default)]
struct Instance {
    model_id: String,
    meshes: Vec<MeshHandle>,
    transforms: Vec<Mat4>,
    sampled_pose: Option<(usize, i64, u8)>,
    last_clip: Option<usize>,
    last_animation_time: f32,
    /// The pose a cross-fade starts from (the old clip, frozen where it was left).
    blend_from: Option<velvet_render::Pose3D>,
    hand_slots: Option<HandSlots>,
    blend_hand_slots_from: Option<HandSlots>,
    blend_started_at: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct HandPose {
    /// The animated hand slot in model space (weapons hang from it).
    matrix: Mat4,
}

/// A skeleton weapon model, drawn in a hand slot as the web does.
struct Weapon {
    source: GltfModel,
    meshes: Vec<MeshHandle>,
    textures: Vec<TextureHandle>,
    triangles: usize,
}

/// The weapon models the skeletons carry (web `WEAPON_PATHS`): the skeletons' own (tools/armas_blender.py,
/// real weapons in KayKit's frame and reach) when they are there, else KayKit's.
const WEAPON_FILES: [(&str, &str, &str); 5] = [
    (
        "staff",
        "../esqueletos/armas/staff.glb",
        "weapons/Skeleton_Staff.gltf",
    ),
    (
        "crossbow",
        "../esqueletos/armas/crossbow.glb",
        "weapons/Skeleton_Crossbow.gltf",
    ),
    (
        "axe",
        "../esqueletos/armas/axe.glb",
        "weapons/Skeleton_Axe.gltf",
    ),
    (
        "blade",
        "../esqueletos/armas/blade.glb",
        "weapons/Skeleton_Blade.gltf",
    ),
    (
        "shield",
        "../esqueletos/armas/shield.glb",
        "weapons/Skeleton_Shield_Large_A.gltf",
    ),
];

/// Every loadout each model is dressed in (web `enemyLoadout`), with
/// whether its staff fights in melee.
const MODEL_LOADOUTS: [(&str, &str, Option<&str>, bool); 6] = [
    ("minion", "blade", None, false),
    ("rogue", "blade", None, false),
    ("rogue", "crossbow", None, false),
    ("warrior", "axe", Some("shield"), false),
    ("mage", "staff", None, false),
    ("mage", "staff", None, true),
];

/// KayKit's shared clips hold a chibi's arms well out from its wide body; a body of human build
/// keeps them by its sides. Each key of an upper arm has its reach out to the side cut (its swing
/// forward and back kept); an arm raised over the shoulder stays as it is. The bone runs along its
/// local +y; `q` turns it in its parent's (the chest's) frame.
fn arm_closer(q: [f32; 4]) -> [f32; 4] {
    let along = quat_rotate(q, [0.0, 1.0, 0.0]);
    let raised = ((along[1] + 0.1) / 0.4).clamp(0.0, 1.0);
    let keep = 0.4 + 0.6 * raised * raised * (3.0 - 2.0 * raised);
    let side = along[0] * keep;
    let rest = (1.0 - side * side - along[2] * along[2]).max(0.0).sqrt();
    let to = [side, if along[1] < 0.0 { -rest } else { rest }, along[2]];
    quat_mul(quat_arc(along, to), q)
}

fn quat_rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = Vec3::new(q[0], q[1], q[2]);
    let v = Vec3::new(v[0], v[1], v[2]);
    let r = u * (2.0 * u.dot(v)) + v * (q[3] * q[3] - u.dot(u)) + u.cross(v) * (2.0 * q[3]);
    [r.x, r.y, r.z]
}

fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

/// The shortest turn taking unit vector `a` to unit vector `b`.
fn quat_arc(a: [f32; 3], b: [f32; 3]) -> [f32; 4] {
    let (a, b) = (Vec3::new(a[0], a[1], a[2]), Vec3::new(b[0], b[1], b[2]));
    let c = a.cross(b);
    let w = 1.0 + a.dot(b);
    if w < 1.0e-5 {
        return [0.0, 0.0, 1.0, 0.0];
    }
    let n = (c.x * c.x + c.y * c.y + c.z * c.z + w * w).sqrt();
    [c.x / n, c.y / n, c.z / n, w / n]
}

/// A weapon's pose in its hand slot (web `cloneEnemy`).
fn weapon_attach(weapon: &str, left: bool) -> Mat4 {
    let (rotation, position) = rig::attach_for(weapon, usize::from(left));
    let scale = if weapon == "shield" {
        0.63
    } else {
        rig::WEAPON_SCALE as f32
    };
    Mat4::from_translation(Vec3::new(
        position[0] as f32,
        position[1] as f32,
        position[2] as f32,
    )) * Mat4::from_rotation_x(rotation[0] as f32)
        * Mat4::from_rotation_y(rotation[1] as f32)
        * Mat4::from_rotation_z(rotation[2] as f32)
        * Mat4::from_scale(Vec3::new(scale, scale, scale))
}

#[derive(Clone, Copy, Debug, Default)]
struct HandSlots {
    right: HandPose,
    left: HandPose,
}

/// The held weapon's edge in world space, from 35% of its length (near the
/// grip) to the tip — what a weapon trail follows.
#[derive(Clone, Copy, Debug)]
pub(super) struct WeaponEdge {
    pub base: Vec3,
    pub tip: Vec3,
}

/// Render metadata shared with the procedural weapon layer.
#[derive(Clone, Copy, Debug)]
pub(super) struct EnemyDrawResult {
    pub triangles: usize,
    pub fracture_anchor: Vec3,
    pub weapon_edge: Option<WeaponEdge>,
}

/// Runtime models for dungeon enemies, kept independent from gameplay content.
#[derive(Default)]
pub struct Enemies {
    models: BTreeMap<String, Model>,
    roles: BTreeMap<String, Vec<String>>,
    instances: BTreeMap<usize, Instance>,
    weapons: BTreeMap<&'static str, Weapon>,
    /// Instances of fallen foes, their meshes ready for the next one drawn.
    spare: Vec<Instance>,
    /// Unit-height fit of each dressed model, weapons included (the web fits
    /// the whole armed skeleton to the body height).
    fits: BTreeMap<(String, &'static str, Option<&'static str>), Mat4>,
}

type TextureCache = BTreeMap<(u32, u32, u64, u64), TextureHandle>;

impl Enemies {
    /// Load character models, shared animation clips, and role mappings.
    pub fn load(renderer: &mut Wgpu3DRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let directory = root().join("enemies/kaykit_skeletons");
        let manifest_path = directory.join("manifest.json");
        let manifest = match fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Manifest>(&text).ok())
        {
            Some(manifest) => manifest,
            None => {
                eprintln!(
                    "[dungeon] enemy model manifest unavailable: {}",
                    manifest_path.display()
                );
                return Self::default();
            }
        };

        let mut animation_sets = Vec::new();
        for path in &manifest.animation_sets {
            match GltfModel::load(directory.join(path)) {
                Ok(model) => animation_sets.push(model),
                Err(error) => eprintln!("[dungeon] enemy animations {path}: {error}"),
            }
        }

        let mut models = BTreeMap::new();
        let mut texture_cache = TextureCache::new();
        // Models standing in for others load after them and take their ids.
        let mut definitions = manifest.models;
        definitions.sort_by_key(|definition| definition.replaces.is_some());
        let mut replaced: Vec<String> = Vec::new();
        for definition in definitions {
            let path = directory.join(&definition.path);
            let result = (|| -> anyhow::Result<Model> {
                let mut source = GltfModel::load(&path)?;
                let first = source.animation_clips.len();
                for animations in &animation_sets {
                    if definition.own_proportions {
                        source.append_rotations_from(animations, &["root", "hips"])?;
                    } else {
                        source.append_animations_from(animations)?;
                    }
                }
                if definition.own_proportions {
                    for arm in ["upperarm.l", "upperarm.r"] {
                        source.map_rotation_keys(first, arm, arm_closer);
                    }
                }
                anyhow::ensure!(!source.meshes.is_empty(), "model contains no meshes");
                anyhow::ensure!(
                    source.animation_index("Idle_A").is_some()
                        && source.animation_index("Walking_A").is_some(),
                    "shared rig has no idle or walk clip"
                );
                // a model standing in for another fights with that one's loadouts
                bake_combat_clips(
                    &mut source,
                    definition.replaces.as_deref().unwrap_or(&definition.id),
                )?;

                let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                let mut high = -low;
                for (index, mesh) in source.meshes.iter().enumerate() {
                    let transform = source.mesh_transform(index);
                    for vertex in &mesh.vertices {
                        let point = transform.transform_point3(Vec3::new(
                            vertex.position[0],
                            vertex.position[1],
                            vertex.position[2],
                        ));
                        low = Vec3::new(low.x.min(point.x), low.y.min(point.y), low.z.min(point.z));
                        high = Vec3::new(
                            high.x.max(point.x),
                            high.y.max(point.y),
                            high.z.max(point.z),
                        );
                    }
                }
                let rest_bounds = (low, high);
                let height = high.y - low.y;
                anyhow::ensure!(height.is_finite() && height > 0.001, "invalid model height");
                let target_height = definition
                    .height_m
                    .filter(|value| value.is_finite())
                    .unwrap_or(1.7)
                    .clamp(0.6, 3.0);
                let uniform_scale = target_height / height;
                let normalize =
                    Mat4::from_scale(Vec3::new(uniform_scale, uniform_scale, uniform_scale))
                        * Mat4::from_translation(Vec3::new(
                            -(low.x + high.x) * 0.5,
                            -low.y,
                            -(low.z + high.z) * 0.5,
                        ));

                let body_anchor = |normalize: Mat4| {
                    source
                        .mesh_names
                        .iter()
                        .position(|name| name.ends_with("_Body"))
                        .and_then(|index| {
                            let mesh = source.meshes.get(index)?;
                            let transform = normalize * source.mesh_transform(index);
                            let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                            let mut high = -low;
                            for vertex in &mesh.vertices {
                                let point = transform.transform_point3(Vec3::new(
                                    vertex.position[0],
                                    vertex.position[1],
                                    vertex.position[2],
                                ));
                                low = Vec3::new(
                                    low.x.min(point.x),
                                    low.y.min(point.y),
                                    low.z.min(point.z),
                                );
                                high = Vec3::new(
                                    high.x.max(point.x),
                                    high.y.max(point.y),
                                    high.z.max(point.z),
                                );
                            }
                            (low.to_array().into_iter().all(f32::is_finite)
                                && high.to_array().into_iter().all(f32::is_finite))
                            .then_some(Vec3::new(
                                (low.x + high.x) * 0.5,
                                low.y + (high.y - low.y) * 0.55,
                                high.z + 0.004,
                            ))
                        })
                };
                let fracture_anchor =
                    body_anchor(normalize).unwrap_or(Vec3::new(0.0, target_height * 0.58, 0.14));
                let raw_fracture_anchor = body_anchor(Mat4::IDENTITY);

                let mut textures = Vec::with_capacity(source.textures.len());
                for texture in &source.textures {
                    let key = (
                        texture.width,
                        texture.height,
                        velvet_dungeon::rng::hash(&texture.rgba),
                        0,
                    );
                    let handle = if let Some(handle) = texture_cache.get(&key).copied() {
                        handle
                    } else {
                        let handle =
                            renderer.upload_texture_with_normal(device, queue, texture, None)?;
                        texture_cache.insert(key, handle);
                        handle
                    };
                    textures.push(handle);
                }

                let triangles = source
                    .meshes
                    .iter()
                    .map(|mesh| mesh.indices.len() / 3)
                    .collect();
                let parts = source
                    .mesh_names
                    .iter()
                    .map(|name| EnemyPart::from_gltf_name(name))
                    .collect();
                Ok(Model {
                    source,
                    normalize,
                    target_height,
                    fracture_anchor,
                    raw_fracture_anchor,
                    rest_bounds,
                    textures,
                    triangles,
                    parts,
                })
            })();

            match result {
                Ok(model) => {
                    if let Some(old) = &definition.replaces {
                        replaced.push(old.clone());
                    }
                    models.insert(definition.replaces.unwrap_or(definition.id), model);
                }
                Err(error) => eprintln!("[dungeon] enemy model {}: {error}", path.display()),
            }
        }
        if !replaced.is_empty() {
            eprintln!("[dungeon] new models stand in for {}", replaced.join(", "));
        }

        let roles = manifest
            .roles
            .into_iter()
            .filter_map(|role| {
                let loaded = |list: Vec<String>| {
                    list.into_iter()
                        .filter(|variant| models.contains_key(variant))
                        .collect::<Vec<_>>()
                };
                let mut variants = loaded(role.variants);
                if variants.is_empty() {
                    variants = loaded(role.fallback);
                }
                (!variants.is_empty()).then_some((role.id, variants))
            })
            .collect::<BTreeMap<_, _>>();

        let mut weapons = BTreeMap::new();
        for (name, own, kaykit) in WEAPON_FILES {
            let path = if directory.join(own).is_file() {
                own
            } else {
                kaykit
            };
            let result = (|| -> anyhow::Result<Weapon> {
                let source = GltfModel::load(directory.join(path))?;
                let mut meshes = Vec::with_capacity(source.meshes.len());
                for mesh in &source.meshes {
                    meshes.push(renderer.upload_mesh(device, mesh)?);
                }
                let mut textures = Vec::with_capacity(source.textures.len());
                for texture in &source.textures {
                    let key = (
                        texture.width,
                        texture.height,
                        velvet_dungeon::rng::hash(&texture.rgba),
                        0,
                    );
                    let handle = match texture_cache.get(&key).copied() {
                        Some(handle) => handle,
                        None => {
                            let handle = renderer
                                .upload_texture_with_normal(device, queue, texture, None)?;
                            texture_cache.insert(key, handle);
                            handle
                        }
                    };
                    textures.push(handle);
                }
                let triangles = source
                    .meshes
                    .iter()
                    .map(|mesh| mesh.indices.len() / 3)
                    .sum();
                Ok(Weapon {
                    source,
                    meshes,
                    textures,
                    triangles,
                })
            })();
            match result {
                Ok(weapon) => {
                    weapons.insert(name, weapon);
                }
                Err(error) => eprintln!("[dungeon] enemy weapon {path}: {error}"),
            }
        }
        let mut fits = BTreeMap::new();
        for (model_id, weapon, offhand, _) in MODEL_LOADOUTS {
            if let Some(model) = models.get(model_id) {
                fits.insert(
                    (model_id.to_string(), weapon, offhand),
                    armed_fit(model, &weapons, weapon, offhand),
                );
            }
        }

        eprintln!(
            "[dungeon] {} KayKit enemy models, {} weapons, {} shared textures, {} roles",
            models.len(),
            weapons.len(),
            texture_cache.len(),
            roles.len()
        );
        Self {
            models,
            roles,
            instances: BTreeMap::new(),
            weapons,
            spare: Vec::new(),
            fits,
        }
    }

    /// Hand the GPU meshes of foes that fell back for reuse. The instance
    /// keeps its handles; the next foe drawn re-fills them in place.
    pub fn release(&mut self, fallen: impl IntoIterator<Item = usize>) {
        for spawn_id in fallen {
            if let Some(mut instance) = self.instances.remove(&spawn_id) {
                // Force a re-upload of the new foe's model into these meshes.
                instance.model_id.clear();
                self.spare.push(instance);
            }
        }
    }

    /// Draw a living enemy as the web does: its loadout's skeleton fitted
    /// (weapons included) to `height` metres, weapons in its hand slots.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_armed(
        &mut self,
        loadout: Loadout,
        spawn_id: usize,
        position: Vec3,
        yaw: f32,
        height: f32,
        animation: &str,
        elapsed: f32,
        world_time: f32,
        distance: f32,
        damage_stage: u8,
        hit_reaction: f32,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draws: &mut Vec<MeshDraw3D>,
    ) -> Option<EnemyDrawResult> {
        let fit = *self
            .fits
            .get(&(loadout.model.to_string(), loadout.weapon, loadout.offhand))?;
        let normalize = Mat4::from_scale(Vec3::new(height, height, height)) * fit;
        let mut result = self.draw_model(
            loadout.model,
            spawn_id,
            position,
            yaw,
            1.0,
            Some((normalize, height)),
            animation,
            elapsed,
            world_time,
            distance,
            damage_stage,
            hit_reaction,
            renderer,
            device,
            queue,
            draws,
        )?;
        let instance = self.instances.get(&spawn_id)?;
        let hands = instance.hand_slots?;
        let reaction_t = ((0.42 - hit_reaction.clamp(0.0, 0.42)) / 0.42).clamp(0.0, 1.0);
        let stagger = if hit_reaction > 0.0 {
            (reaction_t * std::f32::consts::PI).sin() * 0.17
        } else {
            0.0
        };
        let root = Mat4::from_translation(position)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_z(-stagger);
        for (name, left) in [(Some(loadout.weapon), false), (loadout.offhand, true)] {
            let Some(name) = name else { continue };
            let Some(weapon) = self.weapons.get(name) else {
                continue;
            };
            let (hand, part) = if left {
                (hands.left, EnemyPart::LeftArm)
            } else {
                (hands.right, EnemyPart::RightArm)
            };
            let world = root
                * fracture_pose(part, damage_stage, height)
                * normalize
                * hand.matrix
                * weapon_attach(name, left);
            for (index, mesh) in weapon.meshes.iter().enumerate() {
                let source = &weapon.source.meshes[index];
                draws.push(MeshDraw3D {
                    mesh: *mesh,
                    transform: world * weapon.source.mesh_transform(index),
                    material: source.material.with_ambient_occlusion(0.22),
                    texture: source
                        .texture_index
                        .and_then(|texture| weapon.textures.get(texture).copied()),
                });
            }
            result.triangles += weapon.triangles;
            if !left {
                // Web `weaponSegment(held, 0.35, 1)`.
                let axis = rig::weapon_axis(name).unwrap_or([0.0, 1.0, 0.0]);
                let reach = rig::weapon_reach(name) as f32;
                let along = Vec3::new(axis[0] as f32, axis[1] as f32, axis[2] as f32) * reach;
                result.weapon_edge = Some(WeaponEdge {
                    base: world.transform_point3(along * 0.35),
                    tip: world.transform_point3(along),
                });
            }
        }
        Some(result)
    }

    /// Draw one living enemy using its deterministic role variant and animation.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        role: &str,
        seed: u64,
        spawn_id: usize,
        position: Vec3,
        yaw: f32,
        scale: f32,
        animation: &str,
        elapsed: f32,
        world_time: f32,
        distance: f32,
        damage_stage: u8,
        hit_reaction: f32,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draws: &mut Vec<MeshDraw3D>,
    ) -> Option<EnemyDrawResult> {
        let variants = self.roles.get(role)?;
        let choice_key = stable_key(seed, spawn_id, role);
        let model_id = variants[(choice_key as usize) % variants.len()].clone();
        self.draw_model(
            &model_id,
            spawn_id,
            position,
            yaw,
            scale,
            None,
            animation,
            elapsed,
            world_time,
            distance,
            damage_stage,
            hit_reaction,
            renderer,
            device,
            queue,
            draws,
        )
    }

    /// Draw one skeleton model; `fitted` overrides its manifest fit with a
    /// (normalize, body height) pair.
    #[allow(clippy::too_many_arguments)]
    fn draw_model(
        &mut self,
        model_id: &str,
        spawn_id: usize,
        position: Vec3,
        yaw: f32,
        scale: f32,
        fitted: Option<(Mat4, f32)>,
        animation: &str,
        elapsed: f32,
        world_time: f32,
        distance: f32,
        damage_stage: u8,
        hit_reaction: f32,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draws: &mut Vec<MeshDraw3D>,
    ) -> Option<EnemyDrawResult> {
        let model = self.models.get(model_id)?;
        let (normalize, model_height) = fitted.unwrap_or((model.normalize, model.target_height));
        let choice_key = stable_key(0, spawn_id, model_id);
        let clip = model.source.animation_index(animation);
        // A death plays once and holds its last pose.
        let elapsed = match clip.and_then(|c| model.source.animation_clips.get(c)) {
            Some(c) if animation.starts_with("Death") => elapsed.min(c.duration * 0.97),
            _ => elapsed,
        };

        // Nearby enemies animate at 30 fps. Distant portal-visible rooms use
        // progressively cheaper pose updates without disappearing or popping.
        // (Posing is cheap now the GPU deforms the mesh: full rate close up.)
        let sample_fps = if distance > 32.0 { 30 } else { 60 };
        // One-shot attack clips are driven by the state machine's elapsed
        // attack time; a random idle phase would make them start mid-swing.
        let phase = if matches!(
            animation,
            "Throw" | "Hit_A" | "Hit_B" | "Death_A" | "Death_B"
        ) || attacks::is_attack_clip(animation)
        {
            0.0
        } else {
            ((choice_key.rotate_left(23) % 1000) as f32) / 1000.0
        };
        let animation_time = if elapsed.is_finite() {
            elapsed.max(0.0) + phase
        } else {
            phase
        };
        let sample_frame = (animation_time * f32::from(sample_fps)).floor() as i64;

        // A new foe takes a fallen one's GPU meshes rather than uploading
        // more: the Heart keeps raising minions for as long as it lives.
        if !self.instances.contains_key(&spawn_id) {
            let recycled = self.spare.pop().unwrap_or_default();
            self.instances.insert(spawn_id, recycled);
        }
        let instance = self
            .instances
            .get_mut(&spawn_id)
            .expect("instance just ensured");
        let model_changed = instance.model_id != *model_id;
        for (index, source_mesh) in model.source.meshes.iter().enumerate() {
            if model_changed {
                if let Some(handle) = instance.meshes.get(index).copied() {
                    renderer.replace_mesh(device, handle, source_mesh).ok()?;
                } else {
                    instance
                        .meshes
                        .push(renderer.upload_skinned_mesh(device, source_mesh).ok()?);
                }
            } else if index >= instance.meshes.len() {
                instance
                    .meshes
                    .push(renderer.upload_skinned_mesh(device, source_mesh).ok()?);
            }
        }
        if model_changed {
            instance.model_id = model_id.to_string();
            instance.sampled_pose = None;
            instance.last_clip = None;
            instance.last_animation_time = 0.0;
            instance.blend_from = None;
            instance.hand_slots = None;
            instance.blend_hand_slots_from = None;
            instance.transforms = (0..model.source.meshes.len())
                .map(|index| model.source.mesh_transform(index))
                .collect();
        }

        if let Some(clip) = clip {
            if instance.last_clip.is_some_and(|previous| previous != clip) {
                instance.blend_from = instance
                    .last_clip
                    .and_then(|previous| model.source.pose(previous, instance.last_animation_time));
                instance.blend_hand_slots_from = instance.hand_slots;
                instance.blend_started_at = world_time;
            }
            let pose_key = (clip, sample_frame, sample_fps);
            // Resample while a cross-fade runs, even if the clip time holds.
            if instance.sampled_pose != Some(pose_key) || instance.blend_from.is_some() {
                let sample_time = sample_frame as f32 / f32::from(sample_fps);
                let mut pose = model.source.pose(clip, sample_time)?;
                let mut hand_slots = sample_hand_slots(&model.source, clip, sample_time);
                let blend = ((world_time - instance.blend_started_at) / 0.16).clamp(0.0, 1.0);
                if let Some(from) = instance.blend_from.as_ref().filter(|_| blend < 1.0) {
                    pose = from.blend(&pose, blend);
                } else if instance.blend_from.is_some() {
                    instance.blend_from = None;
                }
                if let (Some(from), Some(to)) = (
                    instance.blend_hand_slots_from.filter(|_| blend < 1.0),
                    hand_slots,
                ) {
                    hand_slots = Some(blend_hand_slots(from, to, blend));
                } else if blend >= 1.0 {
                    instance.blend_hand_slots_from = None;
                }
                // The GPU deforms the skinned parts; rigid parts follow their nodes.
                for (index, handle) in instance
                    .meshes
                    .iter()
                    .enumerate()
                    .take(model.source.meshes.len())
                {
                    if let Some(joints) = model.source.joint_matrices(index, &pose) {
                        renderer.set_skin_joints(queue, *handle, &joints).ok()?;
                    }
                }
                instance.transforms = model.source.pose_mesh_transforms(&pose);
                instance.hand_slots = hand_slots;
                instance.sampled_pose = Some(pose_key);
            }
            instance.last_clip = Some(clip);
            instance.last_animation_time = sample_frame as f32 / f32::from(sample_fps);
        }

        let reaction_t = ((0.42 - hit_reaction.clamp(0.0, 0.42)) / 0.42).clamp(0.0, 1.0);
        let stagger = if hit_reaction > 0.0 {
            (reaction_t * std::f32::consts::PI).sin() * 0.17
        } else {
            0.0
        };
        let stagger_pose = Mat4::from_rotation_z(-stagger);
        let mut triangle_count = 0;
        for (index, source_mesh) in model.source.meshes.iter().enumerate() {
            let part = model.parts.get(index).copied().unwrap_or_default();
            let fracture = fracture_pose(part, damage_stage, model_height);
            let transform = Mat4::from_translation(position)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(scale, scale, scale))
                * stagger_pose
                * fracture
                * normalize
                * instance.transforms[index];
            let mut material = source_mesh.material.with_ambient_occlusion(0.22);
            if hit_reaction > 0.0 {
                material = material.with_emission(0.20 + 0.75 * (1.0 - reaction_t));
            }
            draws.push(MeshDraw3D {
                mesh: instance.meshes[index],
                transform,
                material,
                texture: source_mesh
                    .texture_index
                    .and_then(|texture| model.textures.get(texture).copied()),
            });
            triangle_count += model.triangles[index];
        }
        let fracture_anchor = match (fitted, model.raw_fracture_anchor) {
            (None, _) => model.fracture_anchor,
            (Some(_), Some(raw)) => normalize.transform_point3(raw) + Vec3::Z * 0.004,
            (Some(_), None) => Vec3::new(0.0, model_height * 0.58, 0.14),
        };
        Some(EnemyDrawResult {
            triangles: triangle_count,
            fracture_anchor,
            weapon_edge: None,
        })
    }

    /// Return a stable late-death pose for a decorative corpse, held in place
    /// instead of looping the fall animation back to its standing frame.
    pub(super) fn corpse_pose(
        &self,
        role: &str,
        seed: u64,
        spawn_id: usize,
    ) -> Option<(&'static str, f32)> {
        let variants = self.roles.get(role)?;
        let key = stable_key(seed, spawn_id, role);
        let model_id = &variants[(key as usize) % variants.len()];
        let model = self.models.get(model_id)?;
        let animation = if key & 1 == 0 { "Death_A" } else { "Death_B" };
        let clip_index = model.source.animation_index(animation)?;
        let duration = model.source.animation_clips.get(clip_index)?.duration;
        duration
            .is_finite()
            .then_some((animation, (duration * 0.94).max(0.0)))
    }
}

/// A damaged skeleton stays in one piece: limbs never come off its body
/// (which left weapons floating in the air). Damage shows as cracks, chips
/// and sounds instead. Kept as a hook for posture changes per stage.
fn fracture_pose(_part: EnemyPart, _damage_stage: u8, _model_height: f32) -> Mat4 {
    Mat4::IDENTITY
}

/// KayKit skeletons face local +Z; return yaw that points that axis at target.
pub(super) fn facing_yaw(toward: Vec3) -> f32 {
    toward.x.atan2(toward.z)
}

fn stable_key(seed: u64, spawn_id: usize, role: &str) -> u64 {
    velvet_dungeon::rng::hash(&seed.to_le_bytes())
        ^ velvet_dungeon::rng::hash(&(spawn_id as u64).to_le_bytes()).rotate_left(17)
        ^ velvet_dungeon::rng::hash(role.as_bytes()).rotate_left(37)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kaykit_front_axis_faces_the_player() {
        for toward in [
            Vec3::new(3.0, 0.0, 4.0),
            Vec3::new(-4.0, 0.0, 2.0),
            Vec3::new(1.0, 0.0, -5.0),
        ] {
            let yaw = facing_yaw(toward);
            let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
            let expected = Vec3::new(toward.x, 0.0, toward.z).normalize_or_zero();
            assert!((forward - expected).length() < 1.0e-5);
        }
    }

    #[test]
    fn animation_pose_blend_interpolates_the_rig_without_a_hard_snap() {
        // Cross-fades blend poses joint by joint now (the GPU deforms the mesh): halfway
        // between two clips every joint sits between its two places.
        let directory = root().join("enemies/kaykit_skeletons");
        let Ok(text) = fs::read_to_string(directory.join("manifest.json")) else {
            return;
        };
        let manifest: Manifest = serde_json::from_str(&text).expect("valid enemy manifest");
        let Some(definition) = manifest.models.into_iter().next() else {
            return;
        };
        let mut model = GltfModel::load(directory.join(definition.path)).expect("skeleton model");
        for path in &manifest.animation_sets {
            model
                .append_animations_from(&GltfModel::load(directory.join(path)).expect("set"))
                .expect("same rig");
        }
        if model.animation_clips.len() < 2 {
            return;
        }
        let a = model.pose(0, 0.0).unwrap();
        let b = model.pose(1, 0.5).unwrap();
        let mesh = (0..model.meshes.len())
            .find(|&i| model.meshes[i].skinning.is_some())
            .expect("a skinned part");
        let ja = model.joint_matrices(mesh, &a).unwrap();
        let jb = model.joint_matrices(mesh, &b).unwrap();
        let jm = model.joint_matrices(mesh, &a.blend(&b, 0.35)).unwrap();
        for k in 0..ja.len() {
            let pa = ja[k].transform_point3(Vec3::ZERO);
            let pb = jb[k].transform_point3(Vec3::ZERO);
            let pm = jm[k].transform_point3(Vec3::ZERO);
            assert!(
                (pm - pa).length() <= (pb - pa).length() + 1e-3,
                "joint {k} stays between its two poses"
            );
        }
    }

    #[test]
    fn kaykit_models_accept_shared_idle_walk_and_action_clips() {
        let directory = root().join("enemies/kaykit_skeletons");
        let manifest: Manifest = serde_json::from_str(
            &fs::read_to_string(directory.join("manifest.json")).expect("enemy manifest"),
        )
        .expect("valid enemy manifest");
        let animations = manifest
            .animation_sets
            .iter()
            .map(|path| GltfModel::load(directory.join(path)).expect("shared animation set"))
            .collect::<Vec<_>>();

        // the four KayKit skeletons, and whatever stands in for them
        assert_eq!(
            manifest
                .models
                .iter()
                .filter(|definition| definition.replaces.is_none())
                .count(),
            4
        );
        for definition in manifest.models {
            let mut model =
                GltfModel::load(directory.join(definition.path)).expect("skeleton model");
            assert_eq!(model.mesh_names.len(), model.meshes.len());
            assert!(model
                .mesh_names
                .iter()
                .any(|name| name.ends_with("_ArmLeft")));
            assert!(model
                .mesh_names
                .iter()
                .any(|name| name.ends_with("_ArmRight")));
            assert!(model.meshes.iter().any(|mesh| mesh.skinning.is_some()));
            for animation_set in &animations {
                if definition.own_proportions {
                    model
                        .append_rotations_from(animation_set, &["root", "hips"])
                        .expect("animation rig node names match");
                } else {
                    model
                        .append_animations_from(animation_set)
                        .expect("animation rig node names match");
                }
            }
            for clip_name in [
                "Idle_A",
                "Walking_A",
                "Running_A",
                "Hit_A",
                "Hit_B",
                "Throw",
                "Death_A",
                "Death_B",
            ] {
                let clip = model
                    .animation_index(clip_name)
                    .expect("required animation clip");
                let duration = model.animation_clips[clip].duration;
                let pose = model
                    .sample_animation_meshes(
                        clip,
                        if clip_name.starts_with("Death_") {
                            duration * 0.94
                        } else {
                            0.23
                        },
                    )
                    .expect("all mesh parts animate");
                assert_eq!(pose.len(), model.meshes.len());
                assert!(pose.iter().all(|part| !part.vertices.is_empty()));
            }
            let idle = model.animation_index("Idle_A").expect("idle clip");
            let idle_right = model
                .sample_animation_node_transform(idle, 0.23, "handslot.r")
                .expect("right weapon socket exists");
            let idle_left = model
                .sample_animation_node_transform(idle, 0.23, "handslot.l")
                .expect("left weapon socket exists");
            assert!(model.node_transform("handslot.r").is_some());
            assert!(model.node_transform("handslot.l").is_some());
            let right_animates = ["Walking_A", "Hit_A", "Hit_B", "Throw"]
                .into_iter()
                .any(|name| {
                    let clip = model.animation_index(name).expect("weapon action clip");
                    model
                        .sample_animation_node_transform(clip, 0.23, "handslot.r")
                        .is_some_and(|pose| {
                            (pose.transform_point3(Vec3::ZERO)
                                - idle_right.transform_point3(Vec3::ZERO))
                            .length()
                                > 0.001
                        })
                });
            let left_animates = ["Walking_A", "Hit_A", "Hit_B", "Throw"]
                .into_iter()
                .any(|name| {
                    let clip = model.animation_index(name).expect("weapon action clip");
                    model
                        .sample_animation_node_transform(clip, 0.23, "handslot.l")
                        .is_some_and(|pose| {
                            (pose.transform_point3(Vec3::ZERO)
                                - idle_left.transform_point3(Vec3::ZERO))
                            .length()
                                > 0.001
                        })
                });
            assert!(
                right_animates,
                "{} right weapon hand must animate",
                definition.id
            );
            assert!(
                left_animates,
                "{} left weapon hand must animate",
                definition.id
            );
            let walk = model.animation_index("Walking_A").expect("walk clip");
            let start = model
                .sample_animation_meshes(walk, 0.0)
                .expect("walking start pose");
            let stride = model
                .sample_animation_meshes(walk, 0.23)
                .expect("walking stride pose");
            let largest_vertex_delta = start
                .iter()
                .zip(&stride)
                .flat_map(|(a, b)| a.vertices.iter().zip(&b.vertices))
                .map(|(a, b)| {
                    (Vec3::new(a.position[0], a.position[1], a.position[2])
                        - Vec3::new(b.position[0], b.position[1], b.position[2]))
                    .length()
                })
                .fold(0.0_f32, f32::max);
            assert!(
                largest_vertex_delta > 0.001,
                "{} walking clip must deform its rig",
                definition.id
            );
        }
    }

    #[test]
    fn baked_combat_clips_register_on_every_model_and_move_the_rig() {
        let directory = root().join("enemies/kaykit_skeletons");
        for (model_id, file) in [
            ("minion", "Minion"),
            ("rogue", "Rogue"),
            ("warrior", "Warrior"),
            ("mage", "Mage"),
        ] {
            let mut model =
                GltfModel::load(directory.join(format!("characters/Skeleton_{file}.glb")))
                    .expect("skeleton model");
            bake_combat_clips(&mut model, model_id).expect("clips target the skeleton's bones");
            for (owner, weapon, offhand, melee) in MODEL_LOADOUTS {
                if owner != model_id {
                    continue;
                }
                let moves = super::super::combat::moves_for(
                    Loadout {
                        model: owner,
                        weapon,
                        offhand,
                    },
                    melee,
                    true,
                );
                for name in moves
                    .iter()
                    .map(|m| m.name)
                    .chain([super::super::scene::guard_clip(weapon)])
                {
                    let clip = model
                        .animation_index(name)
                        .unwrap_or_else(|| panic!("{model_id} lacks {name}"));
                    assert_eq!(model.animation_clips[clip].name, name);
                    let start = model.sample_animation_meshes(clip, 0.0).expect("pose");
                    let mid = model
                        .sample_animation_meshes(clip, model.animation_clips[clip].duration * 0.45)
                        .expect("pose");
                    let moved = start
                        .iter()
                        .zip(&mid)
                        .flat_map(|(a, b)| a.vertices.iter().zip(&b.vertices))
                        .map(|(a, b)| {
                            (0..3)
                                .map(|i| (a.position[i] - b.position[i]).abs())
                                .fold(0.0f32, f32::max)
                        })
                        .fold(0.0f32, f32::max);
                    assert!(moved > 0.005, "{model_id} {name} must deform the rig");
                }
            }
        }
    }

    #[test]
    fn role_variant_is_stable_for_a_seed_and_spawn() {
        assert_eq!(stable_key(42, 7, "lurker"), stable_key(42, 7, "lurker"));
        assert_ne!(stable_key(42, 7, "lurker"), stable_key(43, 7, "lurker"));
        assert_ne!(stable_key(42, 7, "lurker"), stable_key(42, 8, "lurker"));
    }

    #[test]
    fn kaykit_mesh_names_resolve_real_breakable_skeleton_parts() {
        assert_eq!(
            EnemyPart::from_gltf_name("Skeleton_Rogue_ArmLeft"),
            EnemyPart::LeftArm
        );
        assert_eq!(
            EnemyPart::from_gltf_name("Skeleton_Warrior_ArmRight"),
            EnemyPart::RightArm
        );
        assert_eq!(
            EnemyPart::from_gltf_name("Skeleton_Mage_Skull"),
            EnemyPart::Head
        );
        assert_eq!(
            EnemyPart::from_gltf_name("Skeleton_Minion_Body"),
            EnemyPart::Other
        );
    }

    #[test]
    fn damaged_skeletons_keep_their_limbs_and_weapons_in_hand() {
        for part in [
            EnemyPart::LeftArm,
            EnemyPart::RightArm,
            EnemyPart::Head,
            EnemyPart::Other,
        ] {
            for stage in 0..=3 {
                assert_eq!(fracture_pose(part, stage, 1.6), Mat4::IDENTITY);
            }
        }
    }
}

fn sample_hand_slots(model: &GltfModel, clip: usize, time: f32) -> Option<HandSlots> {
    let animated =
        model.sample_animation_node_transforms(clip, time, &["handslot.r", "handslot.l"])?;
    Some(HandSlots {
        right: HandPose {
            matrix: animated[0],
        },
        left: HandPose {
            matrix: animated[1],
        },
    })
}

fn blend_hand_slots(from: HandSlots, to: HandSlots, alpha: f32) -> HandSlots {
    let blend_pose = |from: HandPose, to: HandPose| HandPose {
        matrix: {
            let (a, b) = (from.matrix.to_cols_array(), to.matrix.to_cols_array());
            Mat4::from_cols_array(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * alpha))
        },
    };
    HandSlots {
        right: blend_pose(from.right, to.right),
        left: blend_pose(from.left, to.left),
    }
}

/// Bake the web prototype's combat clips (guard loops and attacks) for every
/// loadout this model is dressed in, and register them on the model.
fn bake_combat_clips(source: &mut GltfModel, model_id: &str) -> anyhow::Result<()> {
    let nodes: Vec<_> = source
        .rest_nodes()
        .into_iter()
        .map(|(name, parent, t, r)| {
            (
                name,
                parent,
                [t.x as f64, t.y as f64, t.z as f64],
                r.map(f64::from),
            )
        })
        .collect();
    // Natural grip: blades and axes are held by the hilt across the fist.
    let bare = rig::Rig {
        natural_grip: true,
        ..rig::rig_from_nodes(&nodes)
    };
    for (model, weapon, offhand, melee) in MODEL_LOADOUTS {
        if model != model_id {
            continue;
        }
        // Champion sets are a superset: every foe of this dress finds its clips.
        let set = attacks::combat_clips(&bare, attacks::Loadout { weapon, offhand }, melee, true);
        for clip in std::iter::once(&set.guard).chain(set.attacks.iter().map(|(_, clip)| clip)) {
            let rotations = clip
                .rotations
                .iter()
                .map(|(key, values)| {
                    let keys = clip
                        .times
                        .iter()
                        .zip(values)
                        .map(|(t, q)| (*t as f32, q.map(|c| c as f32)))
                        .collect();
                    (bare.bones[key].name.clone(), keys)
                })
                .collect();
            let translations = bare
                .bones
                .get("hips")
                .filter(|_| !clip.hips.is_empty())
                .map(|hips| {
                    let keys = clip
                        .times
                        .iter()
                        .zip(&clip.hips)
                        .map(|(t, p)| (*t as f32, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)))
                        .collect();
                    vec![(hips.name.clone(), keys)]
                })
                .unwrap_or_default();
            source.add_animation_clip(&clip.name, clip.duration as f32, rotations, translations)?;
        }
    }
    Ok(())
}

/// Unit-height fit of an armed model in its rest pose: scale so the whole
/// figure, weapons included, is one unit tall, standing on its lowest point
/// (web `normalizeToHeight`).
fn armed_fit(
    model: &Model,
    weapons: &BTreeMap<&'static str, Weapon>,
    weapon: &str,
    offhand: Option<&str>,
) -> Mat4 {
    let (mut low, mut high) = model.rest_bounds;
    if let Some(slots) = model.source.node_transforms(&["handslot.r", "handslot.l"]) {
        for (name, left) in [(Some(weapon), false), (offhand, true)] {
            let Some(held) = name.and_then(|name| weapons.get(name)) else {
                continue;
            };
            let attach = slots[usize::from(left)] * weapon_attach(name.unwrap_or_default(), left);
            for (index, mesh) in held.source.meshes.iter().enumerate() {
                let transform = attach * held.source.mesh_transform(index);
                for vertex in &mesh.vertices {
                    let p = transform.transform_point3(Vec3::new(
                        vertex.position[0],
                        vertex.position[1],
                        vertex.position[2],
                    ));
                    low = Vec3::new(low.x.min(p.x), low.y.min(p.y), low.z.min(p.z));
                    high = Vec3::new(high.x.max(p.x), high.y.max(p.y), high.z.max(p.z));
                }
            }
        }
    }
    let height = (high.y - low.y).max(0.001);
    Mat4::from_scale(Vec3::new(1.0 / height, 1.0 / height, 1.0 / height))
        * Mat4::from_translation(Vec3::new(0.0, -low.y, 0.0))
}
