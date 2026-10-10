//! Shared mesh/texture handles and bounded distance LODs. Art never changes
//! the seed's graph or navigation. Both LODs use the HIGH mesh's transform.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use velvet_math::{Mat4, Vec3};
use velvet_render::*;

#[derive(Clone, serde::Deserialize)]
#[serde(untagged)]
enum Definition {
    Legacy(String),
    Model {
        path: String,
        lod: Option<String>,
        rotor_part: Option<usize>,
        hinge: Option<HingeDefinition>,
    },
}

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum HingeAxis {
    X,
    Y,
    Z,
}

impl HingeAxis {
    fn rotation(self, angle: f32) -> Mat4 {
        match self {
            Self::X => Mat4::from_rotation_x(angle),
            Self::Y => Mat4::from_rotation_y(angle),
            Self::Z => Mat4::from_rotation_z(angle),
        }
    }
}

#[derive(Clone, Copy, serde::Deserialize)]
struct HingeDefinition {
    /// Mesh primitive whose node is the moving part of the authored model.
    part: usize,
    /// Pivot in the source glTF model's un-normalized coordinate space.
    pivot: [f32; 3],
    axis: HingeAxis,
    open_angle: f32,
}

#[derive(Clone, Copy)]
struct Hinge {
    part: usize,
    pivot: Vec3,
    axis: HingeAxis,
    open_angle: f32,
}

fn hinge_transform(hinge: Hinge, openness: f32) -> Mat4 {
    let pivot = Mat4::from_translation(hinge.pivot);
    pivot * hinge.axis.rotation(hinge.open_angle * openness) * Mat4::from_translation(-hinge.pivot)
}

struct Part {
    mesh: MeshHandle,
    transform: Mat4,
    material: Material3D,
    texture: Option<TextureHandle>,
    triangles: usize,
}
struct Asset {
    parts: Vec<Part>,
    lod: Vec<Part>,
    normalize: Mat4,
    extent: Vec3,
    rotor_part: Option<usize>,
    hinge: Option<Hinge>,
}
pub struct Assets {
    entries: BTreeMap<String, Asset>,
    /// Optional gallery models are loaded only when nearby, never all at boot.
    pending: BTreeMap<String, Definition>,
    cache: TextureCache,
    /// Decodes gallery models off the render thread.
    loader: Option<Loader>,
    /// Models asked of the loader and not yet brought in.
    requested: usize,
    root: PathBuf,
}

pub fn root() -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/assets/3d");
    let packaged = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("data/assets/3d")));
    packaged.filter(|p| p.is_dir()).unwrap_or(source)
}

type TextureCache = BTreeMap<(u32, u32, u64, u64), TextureHandle>;
type TextureKey = (u32, u32, u64, u64);

/// A model read, decoded and mip-mapped on the CPU (any thread): only the
/// GPU copies are left for the frame that brings it in.
struct PreparedPart {
    mesh: Mesh3D,
    transform: Mat4,
    texture: Option<(TextureKey, std::sync::Arc<PreparedTexture3D>)>,
}
struct PreparedModel {
    parts: Vec<PreparedPart>,
    low: Vec3,
    high: Vec3,
}

fn prepare(path: &Path) -> anyhow::Result<PreparedModel> {
    let model = GltfModel::load(path)?;
    let mut low = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut high = -low;
    let mut textures: BTreeMap<TextureKey, std::sync::Arc<PreparedTexture3D>> = BTreeMap::new();
    let mut parts = Vec::new();
    for (i, mesh) in model.meshes.iter().enumerate() {
        let transform = model.mesh_transform(i);
        for v in &mesh.vertices {
            let p =
                transform.transform_point3(Vec3::new(v.position[0], v.position[1], v.position[2]));
            low = Vec3::new(low.x.min(p.x), low.y.min(p.y), low.z.min(p.z));
            high = Vec3::new(high.x.max(p.x), high.y.max(p.y), high.z.max(p.z));
        }
        let texture = if let Some(t) = mesh.texture_index.and_then(|i| model.textures.get(i)) {
            let normal = mesh
                .normal_texture_index
                .and_then(|i| model.textures.get(i));
            let key = (
                t.width,
                t.height,
                velvet_dungeon::rng::hash(&t.rgba),
                normal
                    .map(|n| velvet_dungeon::rng::hash(&n.rgba))
                    .unwrap_or(0),
            );
            let prepared = match textures.get(&key) {
                Some(p) => p.clone(),
                None => {
                    let p = std::sync::Arc::new(PreparedTexture3D::new(t, normal)?);
                    textures.insert(key, p.clone());
                    p
                }
            };
            Some((key, prepared))
        } else {
            None
        };
        let mut mesh = mesh.clone();
        mesh.material = mesh.material.with_ambient_occlusion(0.25);
        parts.push(PreparedPart {
            mesh,
            transform,
            texture,
        });
    }
    anyhow::ensure!(
        (high - low)
            .to_array()
            .iter()
            .all(|v| v.is_finite() && *v > 0.0001),
        "invalid asset bounds: {}",
        path.display()
    );
    Ok(PreparedModel { parts, low, high })
}

fn commit(
    model: PreparedModel,
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut TextureCache,
) -> anyhow::Result<Vec<Part>> {
    let mut parts = Vec::new();
    for part in model.parts {
        let texture = part.texture.map(|(key, prepared)| {
            *cache
                .entry(key)
                .or_insert_with(|| renderer.upload_prepared_texture(device, queue, &prepared))
        });
        parts.push(Part {
            mesh: renderer.upload_mesh(device, &part.mesh)?,
            transform: part.transform,
            material: part.mesh.material,
            texture,
            triangles: part.mesh.indices.len() / 3,
        });
    }
    Ok(parts)
}

/// Everything about one art entry that can be done off the render thread.
struct PreparedDefinition {
    model: PreparedModel,
    lod: Option<anyhow::Result<PreparedModel>>,
    rotor_part: Option<usize>,
    hinge: Option<HingeDefinition>,
}

fn prepare_definition(def: Definition, root: &Path) -> anyhow::Result<PreparedDefinition> {
    let (path, lod_path, rotor_part, hinge) = match def {
        Definition::Legacy(p) => (p, None, None, None),
        Definition::Model {
            path,
            lod,
            rotor_part,
            hinge,
        } => (path, lod, rotor_part, hinge),
    };
    Ok(PreparedDefinition {
        model: prepare(&root.join(path))?,
        lod: lod_path.map(|p| prepare(&root.join(p))),
        rotor_part,
        hinge,
    })
}

fn finish_definition(
    name: &str,
    prepared: PreparedDefinition,
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut TextureCache,
) -> anyhow::Result<Asset> {
    let PreparedDefinition {
        model,
        lod,
        rotor_part,
        hinge: hinge_definition,
    } = prepared;
    let (low, high) = (model.low, model.high);
    let parts = commit(model, renderer, device, queue, cache)?;
    if let Some(first_rotor_part) = rotor_part {
        anyhow::ensure!(
            first_rotor_part > 0 && first_rotor_part < parts.len(),
            "{name} rotor part {first_rotor_part} is missing from GLB"
        );
    }
    let hinge = hinge_definition
        .map(|definition| -> anyhow::Result<Hinge> {
            anyhow::ensure!(
                definition.part < parts.len(),
                "{name} hinge part {} is missing from GLB",
                definition.part
            );
            anyhow::ensure!(
                definition.pivot.iter().all(|v| v.is_finite())
                    && definition.open_angle.is_finite()
                    && definition.open_angle.abs() <= std::f32::consts::PI * 1.5,
                "{name} hinge has invalid pivot or angle"
            );
            Ok(Hinge {
                part: definition.part,
                pivot: Vec3::new(
                    definition.pivot[0],
                    definition.pivot[1],
                    definition.pivot[2],
                ),
                axis: definition.axis,
                open_angle: definition.open_angle,
            })
        })
        .transpose()?;
    let height = high.y - low.y;
    let normalize = Mat4::from_scale(Vec3::new(1.0 / height, 1.0 / height, 1.0 / height))
        * Mat4::from_translation(Vec3::new(
            -(low.x + high.x) * 0.5,
            -low.y,
            -(low.z + high.z) * 0.5,
        ));
    let lod =
        match lod.map(|lod| lod.and_then(|model| commit(model, renderer, device, queue, cache))) {
            Some(Ok(parts)) => parts,
            Some(Err(e)) => {
                eprintln!("[dungeon] {name} LOD: {e}");
                Vec::new()
            }
            None => Vec::new(),
        };
    Ok(Asset {
        parts,
        lod,
        normalize,
        extent: (high - low) * (1.0 / height),
        rotor_part,
        hinge,
    })
}

type Prepared = (String, anyhow::Result<PreparedDefinition>);

/// Prepares gallery art on its own thread, in request order.
struct Loader {
    requests: std::sync::mpsc::Sender<(String, Definition)>,
    ready: std::sync::mpsc::Receiver<Prepared>,
}

impl Loader {
    fn spawn(root: PathBuf) -> Option<Self> {
        let (requests, inbox) = std::sync::mpsc::channel::<(String, Definition)>();
        let (outbox, ready) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("dungeon-art".into())
            .spawn(move || {
                for (name, def) in inbox {
                    let prepared = prepare_definition(def, &root);
                    if outbox.send((name, prepared)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { requests, ready })
    }
}

impl Assets {
    pub fn load(renderer: &mut Wgpu3DRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let defs: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("assets.json")).expect("dungeon art manifest");
        let root = root();
        // Decode every model in parallel, then upload them in order.
        let defs: Vec<(String, Definition)> = defs.into_iter().collect();
        let workers = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .clamp(1, 8);
        let per_worker = defs.len().div_ceil(workers).max(1);
        let mut prepared: Vec<Option<anyhow::Result<PreparedDefinition>>> =
            (0..defs.len()).map(|_| None).collect();
        std::thread::scope(|scope| {
            for (chunk, out) in defs.chunks(per_worker).zip(prepared.chunks_mut(per_worker)) {
                let root = &root;
                scope.spawn(move || {
                    for ((_, def), slot) in chunk.iter().zip(out) {
                        *slot = Some(prepare_definition(def.clone(), root));
                    }
                });
            }
        });
        let mut entries = BTreeMap::new();
        let mut cache = TextureCache::new();
        for ((name, _), prepared) in defs.into_iter().zip(prepared) {
            let result = prepared
                .unwrap_or_else(|| Err(anyhow::anyhow!("not prepared")))
                .and_then(|p| finish_definition(&name, p, renderer, device, queue, &mut cache));
            match result {
                Ok(asset) => {
                    entries.insert(name, asset);
                }
                Err(e) => eprintln!("[dungeon] optional art {name}: {e}"),
            }
        }
        eprintln!(
            "[dungeon] {} art assets, {} shared textures",
            entries.len(),
            cache.len()
        );
        let pending: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("gallery_assets.json"))
                .expect("gallery art manifest");
        Self {
            entries,
            pending,
            cache,
            loader: Loader::spawn(root.clone()),
            requested: 0,
            root,
        }
    }

    /// Ask for the gallery models these rooms need (decoded on the loader
    /// thread) and bring in at most one prepared model per frame. Extra
    /// authored models never inflate the start-up load, and a new room never
    /// stalls the frame on disk or image work.
    pub fn load_next_visible<'a>(
        &mut self,
        names: impl IntoIterator<Item = &'a str>,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        for name in names {
            if self.entries.contains_key(name) {
                continue;
            }
            let Some(def) = self.pending.remove(name) else {
                continue;
            };
            match &self.loader {
                Some(loader) if loader.requests.send((name.to_owned(), def.clone())).is_ok() => {
                    self.requested += 1;
                }
                // No loader thread: prepare here, one model per frame.
                _ => {
                    let prepared = prepare_definition(def, &self.root);
                    self.finish(name.to_owned(), prepared, renderer, device, queue);
                    break;
                }
            }
        }
        if self.requested > 0 {
            if let Some(Ok((name, prepared))) = self.loader.as_ref().map(|l| l.ready.try_recv()) {
                self.requested -= 1;
                self.finish(name, prepared, renderer, device, queue);
            }
        }
    }

    fn finish(
        &mut self,
        name: String,
        prepared: anyhow::Result<PreparedDefinition>,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        match prepared
            .and_then(|p| finish_definition(&name, p, renderer, device, queue, &mut self.cache))
        {
            Ok(asset) => {
                self.entries.insert(name, asset);
            }
            Err(e) => eprintln!("[dungeon] gallery art {name}: {e}"),
        }
    }
    pub fn draw(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        position: Vec3,
        height: f32,
        yaw: f32,
    ) -> Option<usize> {
        let asset = self.entries.get(name)?;
        self.draw_sized(name, draws, position, asset.extent * height, yaw, 0.0)
    }

    /// Draw an authored, upright item with a full Euler pose. Used by the
    /// first-person weapon so its swing rotates the actual shop model.
    pub fn draw_oriented(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        position: Vec3,
        height: f32,
        rotation: Vec3,
        distance: f32,
    ) -> Option<usize> {
        let asset = self.entries.get(name)?;
        let size = asset.extent * height;
        let world = Transform3D {
            position,
            rotation,
            scale: Vec3::new(
                size.x / asset.extent.x,
                size.y / asset.extent.y,
                size.z / asset.extent.z,
            ),
        }
        .matrix();
        let use_lod = distance > 24.0
            && !asset.lod.is_empty()
            && asset.rotor_part.is_none()
            && asset.hinge.is_none();
        let parts = if use_lod { &asset.lod } else { &asset.parts };
        let mut triangles = 0;
        for part in parts {
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: world * asset.normalize * part.transform,
                material: part.material,
                texture: part.texture,
            });
            triangles += part.triangles;
        }
        Some(triangles)
    }

    /// First-person sword uses the model's measured grip and blade axis.
    /// Bounding-box centering would leave the fist off the hilt.
    /// The sword in the player's hand; `relic` asks for the treasury's relic
    /// sword (`relic_sword` in the manifest), falling back to the wooden one.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_held_sword(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        hand: Vec3,
        blade: Vec3,
        shoulder: Vec3,
        roll: f32,
        relic: bool,
    ) -> Option<usize> {
        let asset = relic
            .then(|| self.entries.get("relic_sword"))
            .flatten()
            .or_else(|| self.entries.get("wooden_sword"))?;
        let world = super::fp::sword_transform(hand, blade, shoulder, roll);
        let mut triangles = 0;
        for part in &asset.parts {
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: world * part.transform,
                material: part.material,
                texture: part.texture,
            });
            triangles += part.triangles;
        }
        Some(triangles)
    }

    /// Draws an asset whose local -Z bounds plane is attached to a world-space
    /// surface point. Height is the same normalized-height scale used by draw.
    pub fn draw_back_mounted(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        surface_anchor: Vec3,
        height: f32,
        yaw: f32,
    ) -> Option<usize> {
        let asset = self.entries.get(name)?;
        let local_anchor = Vec3::new(0.0, 0.0, -asset.extent.z * 0.5);
        let world_offset = Mat4::from_rotation_y(yaw).transform_vector3(local_anchor * height);
        self.draw(name, draws, surface_anchor - world_offset, height, yaw)
    }

    pub fn draw_sized(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        position: Vec3,
        size: Vec3,
        yaw: f32,
        distance: f32,
    ) -> Option<usize> {
        self.draw_sized_rotating(name, draws, position, size, yaw, distance, 0.0, [1.0; 3])
    }
    pub fn draw_sized_rotating(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        position: Vec3,
        size: Vec3,
        yaw: f32,
        distance: f32,
        rotor_angle: f32,
        rotor_tint: [f32; 3],
    ) -> Option<usize> {
        self.draw_sized_articulated(
            name,
            draws,
            position,
            size,
            yaw,
            distance,
            rotor_angle,
            rotor_tint,
            0.0,
        )
    }

    /// Draws the authored moving part of a model around its glTF hinge pivot.
    /// The moving mesh remains part of the original model; no lid geometry is
    /// duplicated or placed over the chest.
    pub fn draw_hinged(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        position: Vec3,
        height: f32,
        yaw: f32,
        distance: f32,
        openness: f32,
    ) -> Option<usize> {
        let size = self.entries.get(name)?.extent * height;
        self.draw_sized_articulated(
            name, draws, position, size, yaw, distance, 0.0, [1.0; 3], openness,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_sized_articulated(
        &self,
        name: &str,
        draws: &mut Vec<MeshDraw3D>,
        position: Vec3,
        size: Vec3,
        yaw: f32,
        distance: f32,
        rotor_angle: f32,
        rotor_tint: [f32; 3],
        openness: f32,
    ) -> Option<usize> {
        let asset = self.entries.get(name)?;
        let world = Transform3D {
            position,
            rotation: Vec3::new(0.0, yaw, 0.0),
            scale: Vec3::new(
                size.x / asset.extent.x,
                size.y / asset.extent.y,
                size.z / asset.extent.z,
            ),
        }
        .matrix();
        let use_lod = distance > 24.0
            && !asset.lod.is_empty()
            && asset.rotor_part.is_none()
            && asset.hinge.is_none();
        let parts = if use_lod { &asset.lod } else { &asset.parts };
        let mut triangles = 0;
        for (index, part) in parts.iter().enumerate() {
            let is_rotor = asset.rotor_part.is_some_and(|first| index >= first);
            let mut material = part.material;
            if is_rotor {
                for (channel, tint) in rotor_tint.iter().enumerate() {
                    material.base_color[channel] *= tint;
                }
            }
            let hinge = asset
                .hinge
                .filter(|hinge| hinge.part == index)
                .map(|hinge| hinge_transform(hinge, openness.clamp(0.0, 1.08)))
                .unwrap_or(Mat4::IDENTITY);
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: world
                    * asset.normalize
                    * hinge
                    * part.transform
                    * if is_rotor {
                        Mat4::from_rotation_z(rotor_angle)
                    } else {
                        Mat4::IDENTITY
                    },
                material,
                texture: part.texture,
            });
            triangles += part.triangles;
        }
        Some(triangles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hinged_mesh_rotates_around_its_authored_pivot() {
        let hinge = Hinge {
            part: 0,
            pivot: Vec3::new(0.0, 0.5, -0.56),
            axis: HingeAxis::X,
            open_angle: -1.9,
        };
        let transform = hinge_transform(hinge, 1.0);
        let fixed = transform.transform_point3(hinge.pivot);
        assert!((fixed - hinge.pivot).length() < 0.0001);

        let front_edge = hinge.pivot + Vec3::Z;
        let opened_edge = transform.transform_point3(front_edge);
        assert!(opened_edge.y > front_edge.y + 0.9);
    }

    #[test]
    fn chest_manifest_targets_the_original_separate_lid_node() {
        let definitions: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("assets.json")).unwrap();
        let Definition::Model {
            path,
            hinge: Some(hinge),
            ..
        } = definitions.get("chest").unwrap()
        else {
            panic!("chest must declare an articulated GLTF part");
        };
        let model = GltfModel::load(root().join(path)).unwrap();
        assert_eq!(model.meshes.len(), 2, "lid and body remain separate meshes");
        assert!(hinge.part < model.meshes.len());

        let pivot = Vec3::new(hinge.pivot[0], hinge.pivot[1], hinge.pivot[2]);
        let authored_node_origin = model
            .mesh_transform(hinge.part)
            .transform_point3(Vec3::ZERO);
        assert!((authored_node_origin - pivot).length() < 0.001);
    }

    #[test]
    fn kaykit_skeleton_weapon_models_and_shared_texture_load() {
        let definitions: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("assets.json")).unwrap();
        for name in [
            "skeleton_staff",
            "skeleton_crossbow",
            "skeleton_axe",
            "skeleton_blade",
            "skeleton_shield",
        ] {
            let Definition::Legacy(path) = definitions.get(name).unwrap() else {
                panic!("{name} should reference one standalone glTF model");
            };
            let model = GltfModel::load(root().join(path)).unwrap();
            assert!(!model.meshes.is_empty(), "{name} has no mesh");
            assert!(
                model.meshes.iter().any(|mesh| !mesh.indices.is_empty()),
                "{name} has no drawable triangles"
            );
            assert!(
                !model.textures.is_empty(),
                "{name} should retain the KayKit painted material"
            );
        }
    }

    #[test]
    fn relic_sword_shares_the_wooden_swords_grip_frame() {
        let definitions: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("assets.json")).unwrap();
        // `fp::sword_transform` was measured on the wooden sword: the relic
        // must also be one unit tall, pommel low on -X, tip high on +X.
        for name in ["wooden_sword", "relic_sword"] {
            let Definition::Legacy(path) = definitions.get(name).unwrap() else {
                panic!("{name} should reference one standalone model");
            };
            let model = GltfModel::load(root().join(path)).unwrap();
            assert!(
                !model.textures.is_empty(),
                "{name} keeps its painted texture"
            );
            let points: Vec<Vec3> = model
                .meshes
                .iter()
                .enumerate()
                .flat_map(|(i, mesh)| {
                    let t = model.mesh_transform(i);
                    mesh.vertices.iter().map(move |v| {
                        t.transform_point3(Vec3::new(v.position[0], v.position[1], v.position[2]))
                    })
                })
                .collect();
            let (low, high) = points
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(l, h), p| {
                    (l.min(p.y), h.max(p.y))
                });
            assert!(
                low.abs() < 0.01 && (high - 1.0).abs() < 0.01,
                "{name} spans y 0..1"
            );
            let along = |p: &Vec3| p.x * 0.631 + p.y * 0.776;
            let pommel = points
                .iter()
                .min_by(|a, b| along(a).total_cmp(&along(b)))
                .unwrap();
            let tip = points
                .iter()
                .max_by(|a, b| along(a).total_cmp(&along(b)))
                .unwrap();
            assert!(
                pommel.x < -0.3 && pommel.y < 0.05,
                "{name} pommel at {pommel:?}"
            );
            assert!(tip.x > 0.3 && tip.y > 0.95, "{name} tip at {tip:?}");
        }
    }

    #[test]
    fn every_gallery_model_is_present_and_dressing_references_the_manifest() {
        use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};
        let gallery: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("gallery_assets.json")).unwrap();
        let core: BTreeMap<String, Definition> =
            serde_json::from_str(include_str!("assets.json")).unwrap();
        for (name, def) in &gallery {
            let path = match def {
                Definition::Legacy(path) => path,
                Definition::Model { path, .. } => path,
            };
            assert!(
                root().join(path).is_file(),
                "missing gallery asset {name}: {path}"
            );
        }
        let catalog = Catalog::bundled();
        for seed in 0..24 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let decor = super::super::decor::room(&map, room);
                if let Some(theme) = &decor.theme {
                    let names = theme.0.mounts.iter().map(|m| m.asset.to_string()).chain(
                        theme
                            .0
                            .weapons
                            .iter()
                            .map(|w| super::super::scene::weapon_asset(w.name).to_string()),
                    );
                    for name in names {
                        assert!(
                            core.contains_key(&name) || gallery.contains_key(&name),
                            "undefined theme art {name}"
                        );
                    }
                }
                for prop in &decor.props {
                    if !prop.asset.is_empty() {
                        assert!(
                            core.contains_key(prop.asset) || gallery.contains_key(prop.asset),
                            "undefined art {}",
                            prop.asset
                        );
                    }
                }
            }
        }
    }
}
