//! El combate contra Varrak como lugar (`varrak_fight.rs` es lo que pasa en él): entrar después de
//! su llegada, la primera persona con el ratón capturado, cómo se dibuja (la plaza, él con su
//! cuchilla, la manada, lo que vuela, los brazos de la Reina con la espada y el sello, la chispa) y
//! salir: al ganar, su despertar (`acto1.varrak_final`) junto a la piedra del Peñascal; al caer, de
//! vuelta a la plaza, a intentarlo otra vez cuando quiera.
use super::*;
use dungeon::combat_fx;
use dungeon::fp;
use varrak_fight::{BladeMode, Clip, Fight, Phase, VState};
use velvet_render::{Finish3D, ToonFire3D};

const PI: f32 = std::f32::consts::PI;

/// One drawn piece of a GLB.
#[derive(Clone, Copy)]
struct Part {
    mesh: MeshHandle,
    transform: Mat4,
    material: Material3D,
    texture: Option<TextureHandle>,
    /// Its mesh's own box (least and greatest corner).
    lo: Vec3,
    hi: Vec3,
}

/// How far below its origin a mesh with box `lo..hi` reaches once turned and scaled by `m` (no
/// translation): what lifts it so it rests on the ground.
fn lowest(m: Mat4, lo: Vec3, hi: Vec3) -> f32 {
    let mut least = f32::MAX;
    for i in 0..8 {
        let c = Vec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        );
        least = least.min(m.transform_point3(c).y);
    }
    if least == f32::MAX {
        0.0
    } else {
        least
    }
}

fn load_parts(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    file: &str,
) -> Result<Vec<(String, Part)>> {
    let path = runtime_3d_root().join("penascal").join(file);
    let asset = upload_static_asset(renderer, device, queue, &path, cache)?;
    let bounds = asset.part_bounds;
    Ok(asset
        .parts
        .into_iter()
        .zip(asset.part_names)
        .enumerate()
        .map(|(i, (part, name))| {
            let (lo, hi) = bounds.get(i).copied().unwrap_or(([0.0; 3], [0.0; 3]));
            let material = if name.ends_with("lampGlass") {
                part.material.with_emission(1.3)
            } else {
                part.material.with_ambient_occlusion(0.18)
            };
            (
                name,
                Part {
                    mesh: part.mesh,
                    transform: part.local_transform,
                    material,
                    texture: part.texture,
                    lo: Vec3::new(lo[0], lo[1], lo[2]),
                    hi: Vec3::new(hi[0], hi[1], hi[2]),
                },
            )
        })
        .collect())
}

fn push(draws: &mut Vec<MeshDraw3D>, parts: &[Part], world: Mat4) {
    for p in parts {
        draws.push(MeshDraw3D {
            mesh: p.mesh,
            transform: world * p.transform,
            material: p.material,
            texture: p.texture,
        });
    }
}

/// "#rrggbb" as linear RGB.
fn colour(hex: &str) -> [f32; 3] {
    let h = hex.trim_start_matches('#');
    let channel = |i: usize| {
        u8::from_str_radix(h.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255) as f32 / 255.0
    };
    let lin = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [lin(channel(0)), lin(channel(2)), lin(channel(4))]
}

fn texture(w: usize, h: usize, alpha: impl Fn(f32, f32) -> f32) -> TextureData3D {
    let mut rgba = Vec::with_capacity(w * h * 4);
    for j in 0..h {
        for i in 0..w {
            let a = alpha((i as f32 + 0.5) / w as f32, (j as f32 + 0.5) / h as f32);
            rgba.extend_from_slice(&[255, 255, 255, (a.clamp(0.0, 1.0) * 255.0) as u8]);
        }
    }
    TextureData3D {
        width: w as u32,
        height: h as u32,
        rgba,
    }
}

/// The crown of the Queen's army (his chest's emblem), as the prototype draws it.
fn crown_texture() -> TextureData3D {
    let shape: [(f32, f32); 9] = [
        (-50.0, 30.0),
        (-56.0, -22.0),
        (-30.0, 2.0),
        (-14.0, -36.0),
        (0.0, -6.0),
        (14.0, -36.0),
        (30.0, 2.0),
        (56.0, -22.0),
        (50.0, 30.0),
    ];
    let inside = |x: f32, y: f32| {
        let mut c = false;
        let mut j = shape.len() - 1;
        for i in 0..shape.len() {
            let (xi, yi) = shape[i];
            let (xj, yj) = shape[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                c = !c;
            }
            j = i;
        }
        c
    };
    texture(128, 128, |u, v| {
        // the picture's units round its middle (the prototype's 128 px canvas, scale 0.9)
        let (x, y) = ((u * 128.0 - 64.0) / 0.9, (v * 128.0 - 66.0) / 0.9);
        let band = (-52.0..52.0).contains(&x) && (34.0..46.0).contains(&y);
        let jewel = [(-56.0, -26.0), (-14.0, -40.0), (14.0, -40.0), (56.0, -26.0)]
            .iter()
            .any(|&(cx, cy)| (x - cx).hypot(y - cy) < 7.0);
        if inside(x, y) || band || jewel {
            1.0
        } else {
            0.0
        }
    })
}

/// A unit quad standing in the x/y plane (turned to the camera when drawn).
fn quad_mesh() -> Mesh3D {
    let v =
        |x: f32, y: f32, u: f32, w: f32| MeshVertex3D::new([x, y, 0.0], [0.0, 0.0, 1.0], [u, w]);
    Mesh3D {
        vertices: vec![
            v(-0.5, -0.5, 0.0, 1.0),
            v(0.5, -0.5, 1.0, 1.0),
            v(0.5, 0.5, 1.0, 0.0),
            v(-0.5, 0.5, 0.0, 0.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2],
        ..Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0))
    }
}

/// Two crossed upright quads (a column of light seen from anywhere), 1 m wide and tall.
fn column_mesh() -> Mesh3D {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (dx, dz) in [(1.0f32, 0.0f32), (0.0, 1.0)] {
        let base = vertices.len() as u32;
        for (s, y) in [(-1.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
            vertices.push(MeshVertex3D::new(
                [dx * s * 0.5, y, dz * s * 0.5],
                [dz, 0.0, -dx],
                [(s + 1.0) * 0.5, 1.0 - y],
            ));
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    Mesh3D {
        vertices,
        indices,
        ..Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0))
    }
}

/// Where the moon hangs (from the plaza): low in the west, as the night's light comes from.
const MOON_DIR: Vec3 = Vec3::new(-0.55, 0.42, -0.72);

/// The night over the crags, as a dome round her eyes (radius 1; drawn 170 m out): deep indigo
/// overhead, violet lower down, a smoulder of rose and ember along the horizon (brightest under the
/// moon), and stars, more of them high up, as tiny bright quads on the dome. Its faces look in.
fn sky_mesh() -> Mesh3D {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let moon = MOON_DIR.normalize_or_zero();
    let (rings, segments) = (28u32, 64u32);
    let colour = |d: Vec3| -> [f32; 4] {
        let up = d.y.clamp(-0.2, 1.0);
        let zenith = Vec3::new(0.020, 0.012, 0.052);
        let mid = Vec3::new(0.085, 0.042, 0.165);
        let horizon = Vec3::new(0.24, 0.07, 0.13);
        let ember = Vec3::new(0.42, 0.13, 0.06);
        let toward_moon = d.dot(moon).max(0.0);
        let low = (1.0 - (up / 0.35).clamp(0.0, 1.0)).powf(1.6);
        let high = (up / 0.8).clamp(0.0, 1.0);
        let mut c = mid.lerp(zenith, high);
        c = c.lerp(horizon.lerp(ember, toward_moon.powf(3.0) * 0.8), low);
        // The moon's glow over the sky round it.
        c += Vec3::new(0.10, 0.08, 0.16) * toward_moon.powf(18.0);
        [c.x, c.y, c.z, 1.0]
    };
    for i in 0..=rings {
        // From a little below the horizon to the top.
        let a = -0.25 + (i as f32 / rings as f32) * (std::f32::consts::FRAC_PI_2 + 0.25);
        for j in 0..=segments {
            let b = j as f32 / segments as f32 * std::f32::consts::TAU;
            let d = Vec3::new(a.cos() * b.cos(), a.sin(), a.cos() * b.sin());
            let mut v = MeshVertex3D::new(d.to_array(), (-d).to_array(), [0.0, 0.0]);
            v.color = colour(d);
            vertices.push(v);
        }
    }
    let row = segments + 1;
    for i in 0..rings {
        for j in 0..segments {
            let (a, b, c, d) = (
                i * row + j,
                i * row + j + 1,
                (i + 1) * row + j + 1,
                (i + 1) * row + j,
            );
            indices.extend([a, c, b, a, d, c]);
        }
    }
    // The stars.
    let mut seed = 0x2545_f491u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32) / (u32::MAX as f32)
    };
    for _ in 0..420 {
        let b = next() * std::f32::consts::TAU;
        let y: f32 = 0.08 + next().powf(0.7) * 0.92;
        let r = (1.0 - y * y).max(0.0).sqrt();
        let d = Vec3::new(r * b.cos(), y, r * b.sin()).normalize_or_zero();
        if d.dot(moon) > 0.97 {
            continue;
        }
        let size = 0.0012 + next() * next() * 0.0034;
        let bright = 0.6 + next() * 2.4;
        let tint = if next() > 0.75 {
            [1.0, 0.85, 0.7]
        } else if next() > 0.6 {
            [0.75, 0.8, 1.0]
        } else {
            [0.95, 0.9, 1.0]
        };
        let side = d.cross(Vec3::Y).normalize_or_zero();
        let up = side.cross(d).normalize_or_zero();
        let at = d * 0.995;
        let base = vertices.len() as u32;
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = at + side * (sx * size) + up * (sy * size);
            let mut v = MeshVertex3D::new(p.to_array(), (-d).to_array(), [0.0, 0.0]);
            v.color = [tint[0] * bright, tint[1] * bright, tint[2] * bright, 1.0];
            vertices.push(v);
        }
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh3D {
        vertices,
        indices,
        ..Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0))
    }
}

/// A spark of the seal in the air: where, how fast, how long it has and has had, its size, gold
/// or violet.
#[derive(Clone, Copy)]
struct Mote {
    at: Vec3,
    v: Vec3,
    life: f32,
    age: f32,
    size: f32,
    gold: bool,
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

/// An ember rising from a brazier: where it is, how it drifts, how long it has and has had.
#[derive(Clone, Copy)]
struct Ember {
    at: Vec3,
    v: Vec3,
    life: f32,
    age: f32,
    size: f32,
}

/// The spark's ribbon: `n` points, two vertices each, both faces.
const BEAM_POINTS: usize = 28;
fn beam_mesh() -> Mesh3D {
    let vertices = vec![MeshVertex3D::new([0.0; 3], [0.0, 1.0, 0.0], [0.0, 0.0]); BEAM_POINTS * 2];
    let mut indices = Vec::new();
    for i in 0..BEAM_POINTS as u32 - 1 {
        let a = i * 2;
        indices.extend([a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        indices.extend([a, a + 2, a + 1, a + 1, a + 2, a + 3]);
    }
    Mesh3D {
        vertices,
        indices,
        ..Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0))
    }
}

/// The dungeon's arm in the fight's colours (the prototype's): a violet sleeve, lavender gloves,
/// the gold kept.
fn recoloured(mut mesh: Mesh3D) -> Mesh3D {
    for v in &mut mesh.vertices {
        let [r, g, b, a] = v.color;
        if r > 0.5 && g > 0.35 && b < 0.4 {
            continue;
        }
        v.color = if r > 0.15 && r > g * 2.5 {
            // the crimson sleeve, its folds kept
            let k = r / 0.42;
            [0.085 * k * 1.6, 0.055 * k * 1.6, 0.12 * k * 1.6, a]
        } else {
            // the dark glove, lavender as hers
            let k = ((r + g + b) / 3.0 / 0.095).clamp(0.4, 2.0);
            [0.5 * k * 0.55, 0.5 * k * 0.55, 0.78 * k * 0.55, a]
        };
    }
    mesh
}

/// The wolf's parts, one set per fur.
#[derive(Default)]
struct WolfParts {
    body: Vec<Part>,
    head: Vec<Part>,
    tail: Vec<Part>,
    upper: Vec<Part>,
    lower: Vec<Part>,
}

/// A breakable thing's piece flying after he broke it.
struct Piece {
    part: Part,
    at: Vec3,
    v: Vec3,
    rot: Vec3,
    w: Vec3,
    rest: bool,
}

/// Varrak's model: his clips (in `Clip::ALL` order), their lengths, his meshes, and where on his
/// bones the emblem, the eyes and the cleaver's place on his back are.
struct VarrakModel {
    model: GltfModel,
    clips: Vec<Option<usize>>,
    durations: Vec<f32>,
    meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    emblem: Vec3,
    eyes: [Vec3; 2],
    back: Mat4,
}

fn load_varrak(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<VarrakModel> {
    let file = runtime_3d_root().join("characters").join("varrak.glb");
    let model = GltfModel::load(&file).with_context(|| format!("Varrak: {}", file.display()))?;
    let clips: Vec<Option<usize>> = Clip::ALL
        .iter()
        .map(|c| model.animation_index(c.name()))
        .collect();
    let durations = clips
        .iter()
        .map(|c| {
            c.and_then(|i| model.animation_clips.get(i))
                .map_or(1.0, |c| c.duration)
        })
        .collect();
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    };
    let mut cache = HashMap::new();
    let mut meshes = Vec::with_capacity(model.meshes.len());
    for mesh in &model.meshes {
        let handle = renderer.upload_skinned_mesh(device, mesh)?;
        let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
        let normal = mesh
            .normal_texture_index
            .and_then(|i| model.textures.get(i));
        let texture =
            upload_texture_pair(renderer, device, queue, albedo, normal, &mut cache, &white)?;
        meshes.push((handle, mesh.material, texture));
    }
    // Where the prototype attached things, in the bind pose (its offsets are in world metres).
    let s = 1.0 / varrak_fight::VARRAK_SCALE;
    let at = |m: Mat4| m.transform_point3(Vec3::ZERO);
    let chest = model
        .node_transform("CC_Base_Spine02")
        .unwrap_or(Mat4::IDENTITY);
    let head = model
        .node_transform("CC_Base_Head")
        .unwrap_or(Mat4::IDENTITY);
    let inv_chest = chest.inverse().unwrap_or(Mat4::IDENTITY);
    let inv_head = head.inverse().unwrap_or(Mat4::IDENTITY);
    let chest_at = at(chest);
    let emblem = inv_chest.transform_point3(chest_at + Vec3::new(0.0, 0.1, 0.62) * s);
    let eye = |name: &str| {
        model
            .node_transform(name)
            .map(|e| inv_head.transform_point3(at(e) + Vec3::new(0.0, 0.0, 0.07) * s))
            .unwrap_or(Vec3::ZERO)
    };
    let eyes = [eye("CC_Base_L_Eye"), eye("CC_Base_R_Eye")];
    // Hanging down his back from between the shoulder blades (on all fours it trails behind him).
    let back = inv_chest
        * Mat4::from_translation(chest_at + Vec3::new(0.05, 0.45, -0.75) * s)
        * Mat4::from_rotation_x(PI - 0.12)
        * Mat4::from_rotation_z(0.22);
    Ok(VarrakModel {
        model,
        clips,
        durations,
        meshes,
        emblem,
        eyes,
        back,
    })
}

/// A transform's rotation and position, its scale taken out.
fn unscaled(m: Mat4) -> Mat4 {
    let col = |c: [f32; 4]| Vec3::new(c[0], c[1], c[2]).normalize_or_zero();
    let (x, y, z) = (col(m.x_axis), col(m.y_axis), col(m.z_axis));
    Mat4::from_cols(
        [x.x, x.y, x.z, 0.0],
        [y.x, y.y, y.z, 0.0],
        [z.x, z.y, z.z, 0.0],
        m.w_axis,
    )
}

/// The part of the way from `a` to `b` (positions straight, the frame's axes turned and squared up).
fn blend(a: Mat4, b: Mat4, k: f32) -> Mat4 {
    let v = |c: [f32; 4]| Vec3::new(c[0], c[1], c[2]);
    let x = v(a.x_axis).lerp(v(b.x_axis), k).normalize_or_zero();
    let y0 = v(a.y_axis).lerp(v(b.y_axis), k);
    let y = (y0 - x * y0.dot(x)).normalize_or_zero();
    let z = x.cross(y);
    let p = v(a.w_axis).lerp(v(b.w_axis), k);
    Mat4::from_cols(
        [x.x, x.y, x.z, 0.0],
        [y.x, y.y, y.z, 0.0],
        [z.x, z.y, z.z, 0.0],
        [p.x, p.y, p.z, 1.0],
    )
}

/// Her arms as she sees them (`tools/fp_queen_blender.py` → `penascal/fp_queen.glb`): her hands
/// closed round what they hold, her gown's sleeves, the royal sword with its runes and the seal's
/// sceptre with its stone. All in the grip frame of `dungeon::fp`; the sleeves in its limb space.
struct Arms {
    hand_r: Vec<Part>,
    hand_l: Vec<Part>,
    sleeve: Vec<Part>,
    sleeve_upper: Vec<Part>,
    sword: Vec<Part>,
    runes: Vec<Part>,
    sceptre: Vec<Part>,
    gem: Vec<Part>,
}

/// The fight's GPU side.
pub(super) struct BossScene {
    arms: Option<Arms>,
    sky: MeshHandle,
    embers: Vec<Ember>,
    /// The spark's forks, its sparks in the air, the ring of light, the memories seen and when
    /// the last one burst.
    forks: [MeshHandle; 4],
    motes: Vec<Mote>,
    mote_seed: u32,
    ring: TextureHandle,
    memories_seen: usize,
    burst_at: f32,
    /// The objective on show and since when (it slides in when it changes).
    objective: (String, f32),
    ember_seed: u32,
    statics: Vec<Part>,
    breakables: Vec<Vec<Part>>,
    piece_r: HashMap<String, f32>,
    piece_names: Vec<Vec<String>>,
    cleaver_ground: Vec<Part>,
    cleaver: Vec<Part>,
    wolves: [WolfParts; 2],
    jaw: Vec<Part>,
    sword: Vec<Part>,
    seal_rod: Vec<Part>,
    seal_face: Vec<Part>,
    seal_gem: Vec<Part>,
    rock: Vec<Part>,
    varrak: VarrakModel,
    fist_r: MeshHandle,
    fist_l: MeshHandle,
    cuff: MeshHandle,
    upper: MeshHandle,
    trail: MeshHandle,
    fx: combat_fx::FxMeshes,
    quad: MeshHandle,
    column: MeshHandle,
    halo: TextureHandle,
    moon: TextureHandle,
    crown: TextureHandle,
    light_column: TextureHandle,
    flame: MeshHandle,
    fires: Vec<ToonFire3D>,
    /// The cauldron's oil burning on the ground once it has gone over.
    spill_fires: Vec<ToonFire3D>,
    beam: [MeshHandle; 3],
    pieces: Vec<Piece>,
    breaks_seen: usize,
    last_time: f32,
    blade_last: Option<Mat4>,
    blade_from: Option<Mat4>,
    blade_mode: BladeMode,
    blade_lying: Option<Mat4>,
    edge: Vec3,
    last_hand: Vec3,
    /// His chest's emblem and a point over his head this frame (sent to the fight).
    pub(super) chest: Vec3,
    pub(super) over: Vec3,
}

#[derive(Deserialize, Default)]
struct PieceSizes {
    #[serde(default)]
    pieces: HashMap<String, f32>,
}

impl BossScene {
    fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self> {
        let started = Instant::now();
        let mut cache = HashMap::new();
        let statics = load_parts(renderer, device, queue, &mut cache, "combate_static.glb")?
            .into_iter()
            .map(|(_, p)| p)
            .collect();
        let n = varrak_fight::plan().breakables.len();
        let mut breakables: Vec<Vec<Part>> = (0..n).map(|_| Vec::new()).collect();
        let mut piece_names: Vec<Vec<String>> = (0..n).map(|_| Vec::new()).collect();
        for (name, part) in load_parts(
            renderer,
            device,
            queue,
            &mut cache,
            "combate_breakables.glb",
        )? {
            // `b<i>.<k>:<material>`
            let tag = name.split(':').next().unwrap_or("");
            let Some(i) = tag
                .strip_prefix('b')
                .and_then(|t| t.split('.').next())
                .and_then(|i| i.parse::<usize>().ok())
                .filter(|&i| i < n)
            else {
                continue;
            };
            breakables[i].push(part);
            piece_names[i].push(tag.to_owned());
        }
        let piece_r = serde_json::from_str::<PieceSizes>(&plan_text("penascal/combate.json"))
            .map(|s| s.pieces)
            .unwrap_or_default();
        let mut cleaver_ground = Vec::new();
        let mut cleaver = Vec::new();
        let mut wolves: [WolfParts; 2] = Default::default();
        let mut jaw = Vec::new();
        let mut sword = Vec::new();
        let (mut seal_rod, mut seal_face, mut seal_gem, mut rock) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        // The loose rock modelled with the plaza (Blender), if it is there: textured like the cliffs.
        let new_rock: Vec<Part> = if runtime_3d_root()
            .join("penascal")
            .join("combate_rock.glb")
            .exists()
        {
            load_parts(renderer, device, queue, &mut cache, "combate_rock.glb")?
                .into_iter()
                .map(|(_, p)| p)
                .collect()
        } else {
            Vec::new()
        };
        for (name, part) in load_parts(renderer, device, queue, &mut cache, "combate_parts.glb")? {
            let tag = name.split(':').next().unwrap_or("");
            match tag {
                "cleaver_ground" => cleaver_ground.push(part),
                "cleaver" => cleaver.push(part),
                "wolf_jaw" => jaw.push(part),
                "sword" => sword.push(part),
                "seal_rod" => seal_rod.push(part),
                "seal_face" => seal_face.push(part),
                "seal_gem" => seal_gem.push(Part {
                    material: part.material.with_emission(1.0),
                    ..part
                }),
                "rock" => rock.push(part),
                t if t.starts_with("wolf_") => {
                    let (what, fur) = t[5..].rsplit_once('_').unwrap_or((t, "0"));
                    let w = &mut wolves[usize::from(fur == "1")];
                    match what {
                        "body" => w.body.push(part),
                        "head" => w.head.push(part),
                        "tail" => w.tail.push(part),
                        "upper" => w.upper.push(part),
                        "lower" => w.lower.push(part),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        let varrak = load_varrak(renderer, device, queue)?;
        // Her arms, if they are made (else the dungeon's).
        let arms = match load_parts(renderer, device, queue, &mut cache, "fp_queen.glb") {
            Ok(parts) => {
                let mut arms = Arms {
                    hand_r: Vec::new(),
                    hand_l: Vec::new(),
                    sleeve: Vec::new(),
                    sleeve_upper: Vec::new(),
                    sword: Vec::new(),
                    runes: Vec::new(),
                    sceptre: Vec::new(),
                    gem: Vec::new(),
                };
                for (name, part) in parts {
                    let list = match name.split(':').next().unwrap_or("") {
                        "hand_r" => &mut arms.hand_r,
                        "hand_l" => &mut arms.hand_l,
                        "sleeve" => &mut arms.sleeve,
                        "sleeve_upper" => &mut arms.sleeve_upper,
                        "sword" => &mut arms.sword,
                        "sword_runes" => &mut arms.runes,
                        "sceptre" => &mut arms.sceptre,
                        "sceptre_gem" => &mut arms.gem,
                        _ => continue,
                    };
                    // Gems and runes shine; what is gold or steel keeps its own light.
                    let part = if name.ends_with(":gem") || name.ends_with(":rune") {
                        Part {
                            material: part.material.with_emission(1.2),
                            ..part
                        }
                    } else {
                        part
                    };
                    list.push(part);
                }
                (!arms.hand_r.is_empty() && !arms.sword.is_empty()).then_some(arms)
            }
            Err(error) => {
                eprintln!(
                    "[3d] the fight's arms are not made yet (tools/fp_queen_blender.py): {error:#}"
                );
                None
            }
        };
        let white = Material3D::rgb(1.0, 1.0, 1.0);
        let cube = renderer.upload_mesh(device, &Mesh3D::cube(1.0, white))?;
        let orb = renderer.upload_mesh(device, &Mesh3D::uv_sphere(1.0, 12, 8, white))?;
        let fx = combat_fx::FxMeshes {
            cube,
            crystal: cube,
            ring: renderer.upload_mesh(device, &combat_fx::ring_mesh(0.86, 48))?,
            fissure: cube,
            orb,
        };
        let halo = renderer.upload_texture(
            device,
            queue,
            &texture(64, 64, |u, v| {
                let d = ((u - 0.5).hypot(v - 0.5) * 2.0).min(1.0);
                (1.0 - d).powf(2.2)
            }),
        )?;
        let light_column = renderer.upload_texture(
            device,
            queue,
            &texture(16, 128, |u, v| {
                let y = (1.0 - v) * 140.0;
                let rise = ((y - 2.5) / 6.5).clamp(0.0, 1.0);
                let fade = 1.0 - ((y - 60.0) / 80.0).clamp(0.0, 1.0);
                rise * rise * (3.0 - 2.0 * rise) * fade * (1.0 - ((u - 0.5).abs() * 2.0).powi(2))
            }),
        )?;
        let crown = renderer.upload_texture(device, queue, &crown_texture())?;
        // The moon: a full disc with a soft rim, its seas a little dimmer.
        let moon = renderer.upload_texture(
            device,
            queue,
            &texture(128, 128, |u, v| {
                let (x, y) = (u - 0.5, v - 0.5);
                let d = (x * x + y * y).sqrt() * 2.0;
                if d > 1.0 {
                    return 0.0;
                }
                let rim = ((1.0 - d) / 0.05).clamp(0.0, 1.0);
                let sea = |cx: f32, cy: f32, r: f32| {
                    let e = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / r;
                    (1.0 - e).clamp(0.0, 1.0).powf(0.6)
                };
                let seas = sea(-0.12, -0.08, 0.17) * 0.32
                    + sea(0.12, 0.10, 0.13) * 0.26
                    + sea(0.05, -0.2, 0.08) * 0.2;
                rim * (0.95 - seas - d * d * 0.15)
            }),
        )?;
        let mut fires = Vec::new();
        for f in varrak_fight::plan()
            .flames
            .iter()
            .filter(|f| f.kind == "brazier")
        {
            fires.push(ToonFire3D::new(
                renderer,
                device,
                Vec3::new(f.x, f.y - 0.05, f.z),
                0.62,
            )?);
        }
        // The cauldron's oil, if it goes over: fires round where it stood.
        let mut spill_fires = Vec::new();
        if let Some(c) = varrak_fight::plan()
            .breakables
            .iter()
            .find(|b| b.kind == "cauldron")
        {
            for k in 0..10 {
                let (r, a) = match k {
                    0 => (0.0, 0.0),
                    1..=4 => (1.4, k as f32 * 1.57 + 0.3),
                    _ => (2.55, k as f32 * 1.26 + 0.7),
                };
                spill_fires.push(ToonFire3D::new(
                    renderer,
                    device,
                    Vec3::new(c.x + a.cos() * r, 0.02, c.z + a.sin() * r),
                    if k == 0 { 0.95 } else { 0.7 },
                )?);
            }
        }
        let scene = Self {
            spill_fires,
            arms,
            sky: renderer.upload_mesh(device, &sky_mesh())?,
            forks: [
                renderer.upload_mesh(device, &beam_mesh())?,
                renderer.upload_mesh(device, &beam_mesh())?,
                renderer.upload_mesh(device, &beam_mesh())?,
                renderer.upload_mesh(device, &beam_mesh())?,
            ],
            motes: Vec::new(),
            mote_seed: 0x1234_5679,
            ring: renderer.upload_texture(
                device,
                queue,
                &texture(128, 128, |u, v| {
                    let d = ((u - 0.5).hypot(v - 0.5) * 2.0).min(1.0);
                    let band = 1.0 - ((d - 0.82) / 0.12).abs().min(1.0);
                    band * band * (3.0 - 2.0 * band) + (1.0 - d).max(0.0) * 0.08
                }),
            )?,
            memories_seen: 0,
            burst_at: -99.0,
            objective: (String::new(), 0.0),
            embers: Vec::new(),
            ember_seed: 0x9e37_79b9,
            statics,
            breakables,
            piece_r,
            piece_names,
            cleaver_ground,
            cleaver,
            wolves,
            jaw,
            sword,
            seal_rod,
            seal_face,
            seal_gem,
            rock: if new_rock.is_empty() { rock } else { new_rock },
            varrak,
            fist_r: renderer.upload_mesh(device, &recoloured(fp::fist_mesh(true)))?,
            fist_l: renderer.upload_mesh(device, &recoloured(fp::fist_mesh(false)))?,
            cuff: renderer.upload_mesh(device, &recoloured(fp::sleeve_mesh(true)))?,
            upper: renderer.upload_mesh(device, &recoloured(fp::sleeve_mesh(false)))?,
            trail: renderer.upload_mesh(device, &fp::trail_mesh())?,
            fx,
            quad: renderer.upload_mesh(device, &quad_mesh())?,
            column: renderer.upload_mesh(device, &column_mesh())?,
            halo,
            moon,
            crown,
            light_column,
            flame: orb,
            fires,
            beam: [
                renderer.upload_mesh(device, &beam_mesh())?,
                renderer.upload_mesh(device, &beam_mesh())?,
                renderer.upload_mesh(device, &beam_mesh())?,
            ],
            pieces: Vec::new(),
            breaks_seen: 0,
            last_time: 0.0,
            blade_last: None,
            blade_from: None,
            blade_mode: BladeMode::Ground,
            blade_lying: None,
            edge: Vec3::new(1.0, 0.0, 0.0),
            last_hand: Vec3::ZERO,
            chest: Vec3::ZERO,
            over: Vec3::ZERO,
        };
        eprintln!(
            "[3d] the fight with Varrak loaded in {:.0} ms ({} breakables, {} clips)",
            started.elapsed().as_secs_f64() * 1000.0,
            n,
            scene.varrak.clips.iter().flatten().count()
        );
        Ok(scene)
    }

    /// A new fight: everything whole again.
    fn reset(&mut self) {
        self.pieces.clear();
        self.breaks_seen = 0;
        self.blade_last = None;
        self.blade_from = None;
        self.blade_mode = BladeMode::Ground;
        self.blade_lying = None;
        self.motes.clear();
        self.memories_seen = 0;
        self.burst_at = -99.0;
    }
}

/// A bit of noise from a number (for the pieces' throw and the beam's kinks).
fn hash(n: f32) -> f32 {
    let s = (n * 127.1 + 311.7).sin() * 43_758.547;
    s - s.floor()
}

/// The arm from a shoulder off screen to the fist, around what it holds (the dungeon's own).
#[allow(clippy::too_many_arguments)]
fn view_arm(
    draws: &mut Vec<MeshDraw3D>,
    fist: MeshHandle,
    forearm: MeshHandle,
    upper_arm: MeshHandle,
    hand: Vec3,
    blade: Vec3,
    shoulder: Vec3,
    pole: Vec3,
    right: bool,
    roll: f32,
) -> Mat4 {
    let grip = fp::grip_frame(hand, blade, shoulder, roll);
    let wrist = fp::wrist(grip, right);
    let (shoulder, elbow) = fp::arm_ik(shoulder, wrist, pole);
    let white = Material3D::rgb(1.0, 1.0, 1.0);
    let s = fp::VIEW_SCALE;
    for (mesh, transform) in [
        (upper_arm, fp::limb_transform(elbow, shoulder)),
        (forearm, fp::limb_transform(wrist, elbow)),
        (fist, grip * Mat4::from_scale(Vec3::new(s, s, s))),
    ] {
        draws.push(MeshDraw3D {
            mesh,
            transform,
            material: white,
            texture: None,
        });
    }
    grip
}

/// The arm from a shoulder off screen to the hand, round what it holds, made of her own parts (the
/// sleeves in `fp::limb_transform`'s space, the hand in the grip frame).
#[allow(clippy::too_many_arguments)]
fn view_arm_parts(
    draws: &mut Vec<MeshDraw3D>,
    hand_parts: &[Part],
    fore: &[Part],
    upper: &[Part],
    hand: Vec3,
    blade: Vec3,
    shoulder: Vec3,
    pole: Vec3,
    right: bool,
    roll: f32,
) -> Mat4 {
    let grip = fp::grip_frame(hand, blade, shoulder, roll);
    let wrist = fp::wrist(grip, right);
    let (shoulder, elbow) = fp::arm_ik(shoulder, wrist, pole);
    let s = fp::VIEW_SCALE;
    push(draws, upper, fp::limb_transform(elbow, shoulder));
    push(draws, fore, fp::limb_transform(wrist, elbow));
    push(
        draws,
        hand_parts,
        grip * Mat4::from_scale(Vec3::new(s, s, s)),
    );
    grip
}

/// Night over the plaza, lit the way a night scene is (one dominant cold light, warm pools that
/// lead the eye, little fill so the shadows keep their shape): the moonlight comes from behind the
/// throne, the side the moon hangs in the sky, but high enough to clear the north cliffs (from as
/// low as the moon itself their shadow covers the whole plaza); it backlights Varrak and throws
/// the spires' shadows towards her. The fill is dim and cold, so the fires' pools read; a violet
/// haze lays the crest and the far peaks back.
fn mood(env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    let mut e = env;
    let moon = MOON_DIR.normalize_or_zero();
    let key = Vec3::new(moon.x * 0.62, 0.82, moon.z * 0.62).normalize_or_zero();
    e.lighting.direction_to_light = key;
    e.lighting.key_color = [0.38, 0.42, 0.78];
    e.lighting.ambient_color = [0.18, 0.18, 0.33];
    e.lighting.bounce_color = [0.07, 0.04, 0.06];
    e.lighting.shadow_extent = 26.0;
    e.celestial.moon_direction = key;
    e.celestial.moon_color = [0.55, 0.64, 1.0];
    e.celestial.moon_intensity = e.celestial.moon_intensity.min(0.12);
    e.celestial.reflection_color = [0.45, 0.55, 1.0];
    e.celestial.reflection_strength = 0.10;
    e.celestial.horizon_strength = 0.0;
    e.atmosphere.fog_color = [0.030, 0.020, 0.065];
    e.atmosphere.fog_start = 24.0;
    e.atmosphere.fog_end = 240.0;
    e.atmosphere.fog_density = 0.0035;
    e.atmosphere.height_falloff = 0.012;
    // Black behind the sky dome (it is added over it).
    e.clear_color = [0.0, 0.0, 0.0, 1.0];
    e
}

/// The first-person camera: her eyes, where she looks, the flinch of a blow.
pub(super) fn boss_camera(fight: &Fight) -> Camera3D {
    let own = eye_camera(fight);
    let Some(ride) = fight.ride else {
        return own;
    };
    // Up on his back (looking down over his head at his nape) or held in his fist before his
    // face; it eases in from her own eyes, and shakes as he bucks.
    let v = &fight.varrak;
    let (s, c) = v.yaw.sin_cos();
    let root = v.root();
    let t = fight.clock;
    let shake = Vec3::new(
        (t * 23.0).sin() + (t * 41.0).sin() * 0.5,
        (t * 31.0).sin() * 0.6,
        (t * 17.0).cos(),
    ) * (0.025 + 0.11 * ride.buck);
    let (eye, target, fov) = if ride.kind == varrak_fight::QteKind::Grab {
        let face = fight.face();
        (
            Vec3::new(root.x + s * 2.6, root.y + 2.85, root.z + c * 2.6) + shake,
            face - Vec3::Y * 0.15 + shake * 0.5,
            60.0f32,
        )
    } else {
        let nape = fight.nape();
        let rear = 0.18 * ride.buck * (t * 9.0).sin();
        (
            Vec3::new(root.x - s * 1.0, root.y + 3.85 + rear, root.z - c * 1.0) + shake,
            nape + Vec3::new(s * 1.4, -0.35, c * 1.4),
            70.0f32,
        )
    };
    let k = ease_out_cubic((ride.t / 0.4).clamp(0.0, 1.0));
    let position = own.position.lerp(eye, k);
    let target =
        (own.position + (own.target - own.position).normalize_or_zero() * 3.0).lerp(target, k);
    Camera3D {
        position,
        target,
        up: Vec3::Y,
        fov_y_radians: (72.0 + (fov - 72.0) * k).to_radians(),
        ..own
    }
}

/// Her own eyes.
fn eye_camera(fight: &Fight) -> Camera3D {
    let p = &fight.player;
    let eye = Vec3::new(p.x, p.y + varrak_fight::EYE + p.bob, p.z);
    let yaw = p.yaw + p.shake.y;
    let pitch = (p.pitch + p.shake.x).clamp(-1.45, 1.45);
    let forward = Vec3::new(
        -yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = (Vec3::Y * p.shake.z.cos() + right * p.shake.z.sin()).normalize_or_zero();
    Camera3D {
        position: eye,
        target: eye + forward,
        up,
        fov_y_radians: 72.0f32.to_radians(),
        orthographic_height: None,
        near: 0.05,
        far: 400.0,
    }
}

impl RenderState {
    /// Loads the fight the first time it is wanted; whether it is there.
    pub(super) fn ensure_boss_loaded(&mut self) -> bool {
        if !self.boss_load_attempted {
            self.boss_load_attempted = true;
            if varrak_fight::available() {
                match BossScene::load(&mut self.renderer, &self.device, &self.queue) {
                    Ok(scene) => self.boss_scene = Some(scene),
                    Err(error) => eprintln!("[3d] the fight failed to load: {error:#}"),
                }
            } else {
                eprintln!("[3d] the fight is not exported yet (demon-queen-penascal-threejs: node tools/export-combate.mjs)");
            }
        }
        if let Some(scene) = self.boss_scene.as_mut() {
            scene.reset();
        }
        self.boss_scene.is_some()
    }

    /// The fight seen through her eyes, and its HUD.
    pub(super) fn render_boss(&mut self, frame: &Frame, fight: &Fight) {
        let Some(mut scene) = self.boss_scene.take() else {
            return;
        };
        let camera = frame.camera;
        let dt = (fight.time - scene.last_time).clamp(0.0, 0.1);
        scene.last_time = fight.time;
        let t = fight.time;
        let plan = varrak_fight::plan();
        let mut draws = Vec::with_capacity(4096);
        let mut lights = Vec::new();
        // The night: the dome round her eyes, and the moon in it with its halo.
        draws.push(MeshDraw3D {
            mesh: scene.sky,
            transform: Mat4::from_translation(camera.position)
                * Mat4::from_scale(Vec3::new(170.0, 170.0, 170.0)),
            // Added over a black clear, unlit (the scene's ambient would wash it lavender); its
            // haze at 170 m (≈0.3) made up for.
            material: Material3D::additive([1.45, 1.45, 1.45, 1.0]),
            texture: None,
        });
        {
            let moon_at = camera.position + MOON_DIR.normalize_or_zero() * 150.0;
            let f = (camera.position - moon_at).normalize_or_zero();
            let right = Vec3::Y.cross(f).normalize_or_zero();
            let up = f.cross(right).normalize_or_zero();
            let billboard = |size: f32| {
                Mat4::from_cols(
                    [right.x * size, right.y * size, right.z * size, 0.0],
                    [up.x * size, up.y * size, up.z * size, 0.0],
                    [f.x, f.y, f.z, 0.0],
                    [moon_at.x, moon_at.y, moon_at.z, 1.0],
                )
            };
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: billboard(70.0),
                material: Material3D::additive([0.40, 0.30, 0.62, 0.55]),
                texture: Some(scene.halo),
            });
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: billboard(13.0),
                material: Material3D::additive([3.2, 3.0, 3.6, 1.0]),
                texture: Some(scene.moon),
            });
        }
        // The plaza; what is still whole; what flies.
        push(&mut draws, &scene.statics, Mat4::IDENTITY);
        for (i, parts) in scene.breakables.iter().enumerate() {
            if !fight.arena.broken.get(i).copied().unwrap_or(false) {
                push(&mut draws, parts, Mat4::IDENTITY);
            }
        }
        for b in fight.breaks.iter().skip(scene.breaks_seen) {
            for (k, part) in scene
                .breakables
                .get(b.index)
                .map(|p| p.as_slice())
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
                let at = part.transform.transform_point3(Vec3::ZERO);
                let name = scene
                    .piece_names
                    .get(b.index)
                    .and_then(|n| n.get(k))
                    .cloned()
                    .unwrap_or_default();
                let r = scene.piece_r.get(&name).copied().unwrap_or(0.35).max(0.05);
                let seed = b.index as f32 * 17.0 + k as f32 * 3.1;
                let mut out = Vec3::new(at.x - b.from.0, 0.0, at.z - b.from.1);
                if out.length_squared() < 1e-4 {
                    out = Vec3::new(hash(seed) - 0.5, 0.0, hash(seed + 1.0) - 0.5);
                }
                let out = out.normalize_or_zero();
                let heavy = (0.6 / r).min(1.0);
                let v = out * (b.force * (0.4 + hash(seed + 2.0) * 0.8) * (0.5 + heavy * 0.5))
                    + Vec3::new(
                        (hash(seed + 3.0) - 0.5) * 2.0,
                        b.lift * (0.4 + hash(seed + 4.0) * 0.8) * (0.4 + heavy * 0.6),
                        (hash(seed + 5.0) - 0.5) * 2.0,
                    );
                let w = Vec3::new(
                    hash(seed + 6.0) - 0.5,
                    hash(seed + 7.0) - 0.5,
                    hash(seed + 8.0) - 0.5,
                ) * (b.spin * (0.5 + heavy));
                scene.pieces.push(Piece {
                    part: Part {
                        transform: Mat4::IDENTITY,
                        ..*part
                    },
                    at,
                    v,
                    rot: Vec3::ZERO,
                    w,
                    rest: false,
                });
            }
        }
        scene.breaks_seen = fight.breaks.len();
        for p in &mut scene.pieces {
            if !p.rest {
                p.v.y -= 18.0 * dt;
                p.at = p.at + p.v * dt;
                p.rot = p.rot + p.w * dt;
                // Its lowest corner, as it lies now, on the ground.
                let turn = Mat4::from_rotation_z(p.rot.z)
                    * Mat4::from_rotation_y(p.rot.y)
                    * Mat4::from_rotation_x(p.rot.x);
                let floor = -lowest(turn, p.part.lo, p.part.hi);
                if p.at.y < floor {
                    p.at.y = floor;
                    p.v.y = if p.v.y < -2.5 { -p.v.y * 0.3 } else { 0.0 };
                    p.v = Vec3::new(p.v.x * 0.55, p.v.y, p.v.z * 0.55);
                    p.w = p.w * 0.5;
                    if p.v.length() < 0.4 {
                        p.rest = true;
                    }
                }
            }
            draws.push(MeshDraw3D {
                mesh: p.part.mesh,
                transform: Mat4::from_translation(p.at)
                    * Mat4::from_rotation_z(p.rot.z)
                    * Mat4::from_rotation_y(p.rot.y)
                    * Mat4::from_rotation_x(p.rot.x),
                material: p.part.material,
                texture: p.part.texture,
            });
        }
        if fight.varrak.blade == BladeMode::Ground {
            push(&mut draws, &scene.cleaver_ground, Mat4::IDENTITY);
        }
        // Varrak.
        let v = &fight.varrak;
        let vm = &scene.varrak;
        let s = varrak_fight::VARRAK_SCALE;
        // His body carried, not stiff: leaning into a run, rolled into a turn, thrown back by a
        // blow, his chest rising with his breath.
        let breath = (v.life * 2.3).sin();
        let flinch = v.flinch * v.flinch;
        let root = Mat4::from_translation(v.root() - Vec3::new(0.0, flinch * 0.05, 0.0))
            * Mat4::from_rotation_y(v.yaw + (v.life * 37.0).sin() * 0.04 * flinch)
            * Mat4::from_rotation_z(v.tilt)
            * Mat4::from_rotation_x(v.lean - 0.2 * flinch)
            * Mat4::from_scale(Vec3::new(
                s * (1.0 - 0.006 * breath),
                s * (1.0 + 0.014 * breath),
                s * (1.0 + 0.008 * breath),
            ));
        let sample = |clip: Clip, time: f32, backwards: bool| {
            let i = clip.index();
            let index = vm.clips.get(i).copied().flatten()?;
            let duration = vm.durations.get(i).copied().unwrap_or(1.0);
            vm.model.pose(
                index,
                varrak_fight::Anim::sample(clip, time, backwards, duration),
            )
        };
        let a = v.anim;
        let pose = sample(a.clip, a.t, a.backwards).map(|now| match a.from {
            Some((clip, time, backwards)) if a.fade < 1.0 => match sample(clip, time, backwards) {
                Some(before) => before.blend(&now, a.fade),
                None => now,
            },
            _ => now,
        });
        let bone = |name: &str| {
            pose.as_ref()
                .and_then(|p| vm.model.pose_node_transform(p, name))
        };
        if let Some(pose) = &pose {
            pose_skinned_meshes(&mut self.renderer, &self.queue, &vm.model, &vm.meshes, pose);
        }
        // Struck: a white flash over him for an instant.
        let struck = if a.clip == Clip::Hit {
            (1.0 - a.t / 0.14).clamp(0.0, 1.0)
        } else {
            0.0
        };
        for (index, (mesh, material, texture)) in vm.meshes.iter().enumerate() {
            let mut material = (*material).with_ambient_occlusion(0.28);
            if struck > 0.0 {
                material = material.with_emission(struck * 1.8);
            }
            draws.push(MeshDraw3D {
                mesh: *mesh,
                transform: root * vm.model.mesh_transform(index),
                material,
                texture: *texture,
            });
        }
        // A cold rim of moonlight behind him, so his bulk stands out of the dark plaza, and a
        // low warm light on the side she sees (the fires' glow on him): the boss lit like the
        // focus of the scene, cold edge and warm face.
        {
            let to = Vec3::new(v.x - camera.position.x, 0.0, v.z - camera.position.z)
                .normalize_or_zero();
            lights.push(PointLight3D::new(
                Vec3::new(v.x, 0.0, v.z) + to * 2.6 + Vec3::new(0.0, 3.2, 0.0),
                [0.55, 0.64, 1.0],
                8.0,
                7.5,
            ));
            let side = Vec3::new(-to.z, 0.0, to.x);
            lights.push(PointLight3D::new(
                Vec3::new(v.x, 0.0, v.z) - to * 2.4 + side * 1.2 + Vec3::new(0.0, 1.9, 0.0),
                [1.0, 0.66, 0.42],
                3.2,
                5.5,
            ));
        }
        let chest_bone = bone("CC_Base_Spine02").unwrap_or(Mat4::IDENTITY);
        let head_bone = bone("CC_Base_Head").unwrap_or(Mat4::IDENTITY);
        let emblem = (root * chest_bone).transform_point3(vm.emblem);
        let head_at = (root * head_bone).transform_point3(Vec3::ZERO);
        scene.chest = emblem;
        scene.over = head_at + Vec3::new(0.0, 0.75, 0.0);
        let toward = (camera.position - camera.target).normalize_or_zero();
        let face = |at: Vec3, size: f32| {
            let f = (camera.position - at).normalize_or_zero();
            let right = Vec3::Y.cross(f).normalize_or_zero();
            let up = f.cross(right).normalize_or_zero();
            Mat4::from_cols(
                [right.x * size, right.y * size, right.z * size, 0.0],
                [up.x * size, up.y * size, up.z * size, 0.0],
                [f.x, f.y, f.z, 0.0],
                [at.x, at.y, at.z, 1.0],
            )
        };
        // The emblem: brighter with the spark, beating fast while it is open; the eyes, red to amber.
        let spark = v.spark / 100.0;
        let open = if v.open() { 1.0 } else { 0.0 };
        let beat = 0.5 + 0.5 * (v.t * if v.open() { 9.0 } else { 2.2 }).sin();
        let crown_colour = if spark > 0.99 {
            [1.0, 0.82, 0.48]
        } else {
            [0.69, 0.44, 1.0]
        };
        let crown_alpha = 0.3 + spark * 0.6 + open * 0.1;
        draws.push(MeshDraw3D {
            mesh: scene.quad,
            transform: face(emblem + toward * 0.05, 0.34),
            material: Material3D::additive([
                crown_colour[0] * 1.6,
                crown_colour[1] * 1.6,
                crown_colour[2] * 1.6,
                crown_alpha,
            ]),
            texture: Some(scene.crown),
        });
        let glow = 0.3 + spark * 0.9 + beat * 0.08 + open * 0.35;
        draws.push(MeshDraw3D {
            mesh: scene.quad,
            transform: face(emblem + toward * 0.04, glow * 2.0),
            material: Material3D::additive([
                0.63 * 1.4,
                0.38 * 1.4,
                1.0 * 1.4,
                0.12 + spark * 0.45 + beat * 0.08 + open * 0.2,
            ]),
            texture: Some(scene.halo),
        });
        lights.push(PointLight3D::new(
            emblem,
            [0.63, 0.44, 1.0],
            0.6 + spark * 6.0 + beat * 0.8 + open * 3.0,
            6.0,
        ));
        let amber = if matches!(v.state, VState::Kneel | VState::Awake) {
            (v.t / 1.5).min(1.0)
        } else {
            0.0
        };
        let eye_colour = [
            1.0,
            0.22 + (0.72 - 0.22) * amber,
            0.16 + (0.2 - 0.16) * amber,
        ];
        let eye_alpha = if v.state == VState::Stunned {
            0.4
        } else {
            0.95
        };
        for e in vm.eyes {
            let at = (root * head_bone).transform_point3(e);
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: face(at + toward * 0.03, 0.26),
                material: Material3D::additive([
                    eye_colour[0] * 2.0,
                    eye_colour[1] * 2.0,
                    eye_colour[2] * 2.0,
                    eye_alpha,
                ]),
                texture: Some(scene.halo),
            });
        }
        // His cleaver: in his fist along the forearm, its edge leading the swing; on his back on all
        // fours; lying on the ground once he wakes.
        if v.blade != BladeMode::Ground {
            let target = match v.blade {
                BladeMode::Dropped => *scene.blade_lying.get_or_insert_with(|| {
                    Mat4::from_translation(Vec3::new(
                        v.x - v.yaw.cos() * 1.7,
                        0.06,
                        v.z + v.yaw.sin() * 1.7,
                    )) * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_rotation_z(v.yaw + 0.8)
                }),
                BladeMode::Back => unscaled(root * chest_bone * vm.back),
                _ => {
                    let world =
                        |name: &str| bone(name).map(|b| (root * b).transform_point3(Vec3::ZERO));
                    let hand = world("CC_Base_R_Hand").unwrap_or(emblem);
                    let elbow = world("CC_Base_R_Forearm").unwrap_or(hand - Vec3::Y);
                    let mid = world("CC_Base_R_Index1").unwrap_or(hand);
                    let axis = (mid - elbow).normalize_or_zero();
                    let vel = hand - scene.last_hand;
                    scene.last_hand = hand;
                    let lateral = vel - axis * vel.dot(axis);
                    if lateral.length() > 0.01 {
                        scene.edge = scene.edge.lerp(lateral.normalize_or_zero(), 0.35);
                    }
                    let mut edge = (scene.edge - axis * scene.edge.dot(axis)).normalize_or_zero();
                    if edge.length_squared() < 0.5 {
                        edge = Vec3::new(0.0, -1.0, 0.0);
                    }
                    let flat = edge.cross(axis).normalize_or_zero();
                    let p = hand.lerp(mid, 0.7);
                    Mat4::from_cols(
                        [edge.x, edge.y, edge.z, 0.0],
                        [axis.x, axis.y, axis.z, 0.0],
                        [flat.x, flat.y, flat.z, 0.0],
                        [p.x, p.y, p.z, 1.0],
                    )
                }
            };
            // A quick swing from where it was to where it goes.
            if v.blade != scene.blade_mode {
                scene.blade_from = scene.blade_last.or_else(|| {
                    let c = plan.cleaver_spot;
                    Some(Mat4::from_translation(Vec3::new(c.x, 0.9, c.z)))
                });
                scene.blade_mode = v.blade;
            }
            let shown = match scene.blade_from {
                Some(from) if v.blade_fly > 0.0 => {
                    let k = 1.0 - v.blade_fly.max(0.0) / 0.3;
                    blend(from, target, k * k)
                }
                _ => {
                    scene.blade_from = None;
                    target
                }
            };
            scene.blade_last = Some(shown);
            push(&mut draws, &scene.cleaver, shown);
        }
        // The pack.
        for w in fight.pack.wolves.iter().filter(|w| w.visible && !w.gone) {
            let parts = &scene.wolves[usize::from(w.variant == 1)];
            let root = Mat4::from_translation(Vec3::new(w.x, 0.0, w.z))
                * Mat4::from_rotation_y(w.yaw)
                * Mat4::from_scale(Vec3::new(1.15, 1.15, 1.15));
            let body = root
                * Mat4::from_translation(Vec3::new(0.0, w.body_y, 0.0))
                * Mat4::from_rotation_x(w.body_pitch);
            push(&mut draws, &parts.body, body);
            let head = body
                * Mat4::from_translation(Vec3::new(0.0, 0.16, 0.66))
                * Mat4::from_rotation_x(w.neck)
                * Mat4::from_translation(Vec3::new(0.0, 0.16, 0.18));
            push(&mut draws, &parts.head, head);
            push(
                &mut draws,
                &scene.jaw,
                head * Mat4::from_translation(Vec3::new(0.0, -0.12, 0.2))
                    * Mat4::from_rotation_x(PI / 2.0 + w.jaw - 0.1),
            );
            push(
                &mut draws,
                &parts.tail,
                body * Mat4::from_translation(Vec3::new(0.0, 0.1, -0.68))
                    * Mat4::from_rotation_x(w.tail),
            );
            for (k, (lx, lz, _)) in varrak_fight::WOLF_LEGS.iter().enumerate() {
                let (hip, knee) = w.legs[k];
                let hip_m = body
                    * Mat4::from_translation(Vec3::new(*lx, -0.08, *lz))
                    * Mat4::from_rotation_x(hip);
                push(&mut draws, &parts.upper, hip_m);
                push(
                    &mut draws,
                    &parts.lower,
                    hip_m
                        * Mat4::from_translation(Vec3::new(0.0, -0.38, 0.0))
                        * Mat4::from_rotation_x(knee),
                );
            }
            for side in [-1.0f32, 1.0] {
                let at = head.transform_point3(Vec3::new(side * 0.085, 0.03, 0.15));
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: face(at + toward * 0.02, 0.09),
                    material: Material3D::additive([2.0, 0.4, 0.25, 0.95]),
                    texture: Some(scene.halo),
                });
            }
        }
        // What flies: rocks, the smash's wave, sparks and dust.
        for (i, r) in fight.rocks.iter().enumerate() {
            let k = r.r / 0.5;
            let turn = if r.rest { 0.4 } else { t } + i as f32 * 1.7;
            let shape = Mat4::from_rotation_x(r.spin.x * turn * 0.3 + i as f32)
                * Mat4::from_rotation_y(r.spin.y * turn * 0.3)
                * Mat4::from_rotation_z(r.spin.z * turn * 0.3)
                * Mat4::from_scale(Vec3::new(k, k, k));
            // Resting, its lowest point on the ground (not its middle at a guessed height); in the
            // air, never through it.
            let low = scene
                .rock
                .iter()
                .map(|part| lowest(shape * part.transform, part.lo, part.hi))
                .fold(0.0f32, f32::min);
            let y = if r.rest { -low } else { r.at.y.max(-low) };
            push(
                &mut draws,
                &scene.rock,
                Mat4::from_translation(Vec3::new(r.at.x, y, r.at.z)) * shape,
            );
        }
        for w in &fight.waves {
            let k = (w.r / w.reach).clamp(0.0, 1.0);
            draws.push(MeshDraw3D {
                mesh: scene.fx.ring,
                transform: Mat4::from_translation(Vec3::new(w.x, 0.06, w.z))
                    * Mat4::from_scale(Vec3::new(w.r, 1.0, w.r)),
                material: Material3D::additive([1.25, 0.82, 0.45, 0.85 * (1.0 - k)]),
                texture: None,
            });
        }
        combat_fx::draw(&fight.fx, &scene.fx, &mut draws, &mut lights);
        // The torches (a tongue and a halo turned to her) and the braziers' fire.
        let flat_toward = Vec3::new(toward.x, 0.0, toward.z).normalize_or_zero();
        for (i, f) in plan
            .flames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.kind == "torch")
        {
            let c = colour(&f.color);
            let k =
                1.0 + 0.12 * (t * 9.0 + i as f32 * 1.3).sin() + 0.07 * (t * 23.0 + i as f32).sin();
            let s = f.size;
            let base = Vec3::new(f.x, f.y + 0.1 * s, f.z);
            draws.push(MeshDraw3D {
                mesh: scene.flame,
                transform: Mat4::from_translation(base)
                    * Mat4::from_scale(Vec3::new(
                        0.05 * s * (2.0 - k),
                        0.13 * s * k,
                        0.05 * s * (2.0 - k),
                    )),
                material: Material3D::rgb(1.0, 0.82 + 0.1 * c[1], 0.5 * c[2] + 0.3)
                    .with_emission(2.4),
                texture: None,
            });
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: face(base + toward * 0.05, 0.9 * s),
                material: Material3D::additive([c[0] * 1.6, c[1] * 1.6, c[2] * 1.6, 0.4 * k]),
                texture: Some(scene.halo),
            });
        }
        for fire in &scene.fires {
            fire.draws(t, flat_toward, &mut draws);
        }
        // Where the stones will land: a red ring closing on its spot, brighter as it nears.
        for &(mx, mz, r, at) in &fight.marks {
            let left = (at - t).max(0.0);
            let k = (1.0 - left / 1.8).clamp(0.0, 1.0);
            let pulse = 1.0 + 0.08 * (t * (10.0 + 14.0 * k)).sin();
            for (size, alpha) in [
                (r * 2.0 * pulse, 0.22 + 0.3 * k),
                (r * 2.0 * (1.0 - 0.55 * k) + 0.4, 0.14 + 0.24 * k),
            ] {
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: Mat4::from_cols(
                        [size, 0.0, 0.0, 0.0],
                        [0.0, 0.0, -size, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [mx, 0.05, mz, 1.0],
                    ),
                    material: Material3D::additive([1.3, 0.3, 0.18, alpha]),
                    texture: Some(scene.ring),
                });
            }
        }
        if let Some((sx, sz, _, until)) = fight.spill {
            for fire in &scene.spill_fires {
                fire.draws(t, flat_toward, &mut draws);
            }
            let k = ((until - t) / 2.0).clamp(0.0, 1.0);
            let flicker = 1.0 + 0.15 * (t * 11.0).sin() + 0.08 * (t * 27.0).sin();
            lights.push(PointLight3D::new(
                Vec3::new(sx, 1.3, sz),
                [1.0, 0.55, 0.22],
                9.0 * k * flicker,
                11.0,
            ));
        }
        // Embers rising off the braziers, drifting and winking out.
        {
            let next = |seed: &mut u32| {
                *seed ^= *seed << 13;
                *seed ^= *seed >> 17;
                *seed ^= *seed << 5;
                (*seed as f32) / (u32::MAX as f32)
            };
            let braziers: Vec<Vec3> = plan
                .flames
                .iter()
                .filter(|f| f.kind == "brazier")
                .map(|f| Vec3::new(f.x, f.y + 0.2, f.z))
                .collect();
            if !braziers.is_empty() && dt > 0.0 {
                let mut want = dt * 14.0 * braziers.len() as f32;
                while want > 0.0 && scene.embers.len() < 260 {
                    if want < 1.0 && next(&mut scene.ember_seed) > want {
                        break;
                    }
                    want -= 1.0;
                    let k = (next(&mut scene.ember_seed) * braziers.len() as f32) as usize;
                    let at = braziers[k.min(braziers.len() - 1)]
                        + Vec3::new(
                            (next(&mut scene.ember_seed) - 0.5) * 0.5,
                            0.0,
                            (next(&mut scene.ember_seed) - 0.5) * 0.5,
                        );
                    scene.embers.push(Ember {
                        at,
                        v: Vec3::new(
                            (next(&mut scene.ember_seed) - 0.5) * 0.5,
                            1.0 + next(&mut scene.ember_seed) * 1.4,
                            (next(&mut scene.ember_seed) - 0.5) * 0.5,
                        ),
                        life: 1.6 + next(&mut scene.ember_seed) * 2.2,
                        age: 0.0,
                        size: 0.025 + next(&mut scene.ember_seed) * 0.04,
                    });
                }
            }
            for (i, e) in scene.embers.iter_mut().enumerate() {
                e.age += dt;
                let sway = Vec3::new(
                    (t * 1.7 + i as f32 * 0.9).sin(),
                    0.0,
                    (t * 1.3 + i as f32 * 1.7).cos(),
                ) * 0.35;
                e.at += (e.v + sway) * dt;
                e.v.y *= 1.0 - 0.25 * dt;
            }
            scene.embers.retain(|e| e.age < e.life);
            for (i, e) in scene.embers.iter().enumerate() {
                let k = e.age / e.life;
                let wink = 0.65 + 0.35 * (t * 13.0 + i as f32 * 2.1).sin();
                let fade = (1.0 - k) * (k / 0.1).min(1.0) * wink;
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: face(e.at, e.size * (1.0 - 0.5 * k)),
                    material: Material3D::additive([3.0, 1.2 + 0.6 * (1.0 - k), 0.35, fade]),
                    texture: Some(scene.halo),
                });
            }
        }
        // Won: the seal's light rising over the crags, seen from far away.
        if let Some(at) = fight.won_at {
            let k = ((t - at) / 2.5).min(1.0);
            let w = (0.4 + k) * 2.6;
            draws.push(MeshDraw3D {
                mesh: scene.column,
                transform: Mat4::from_translation(Vec3::new(v.x, 0.0, v.z))
                    * Mat4::from_scale(Vec3::new(w, 140.0, w)),
                material: Material3D::additive([0.69 * 1.8, 0.5 * 1.8, 1.0 * 1.8, 0.7 * k]),
                texture: Some(scene.light_column),
            });
            lights.push(PointLight3D::new(
                Vec3::new(v.x, 3.0, v.z),
                [0.7, 0.5, 1.0],
                12.0 * k,
                14.0,
            ));
        }
        // Her arms: the sword in her right, the seal in her left.
        let p = &fight.player;
        let held = fp::world_pose(camera, p.pose, 0.0, 0.0, 0.0);
        let pole_r = fp::camera_vector(camera, Vec3::new(0.7, -1.0, 0.25)).normalize_or_zero();
        let vs = fp::VIEW_SCALE;
        match &scene.arms {
            Some(arms) => {
                let grip = view_arm_parts(
                    &mut draws,
                    &arms.hand_r,
                    &arms.sleeve,
                    &arms.sleeve_upper,
                    held.hand,
                    held.blade,
                    held.shoulder,
                    pole_r,
                    true,
                    p.pose.roll,
                );
                let world = grip * Mat4::from_scale(Vec3::new(vs, vs, vs));
                push(&mut draws, &arms.sword, world);
                // The runes: a low violet glow, flaring as the blade cuts.
                let cutting = match p.swing {
                    Some((combo, st)) => ((st / fp::SWINGS[combo].duration).min(1.0) * PI).sin(),
                    None => 0.0,
                };
                for part in &arms.runes {
                    draws.push(MeshDraw3D {
                        mesh: part.mesh,
                        transform: world * part.transform,
                        material: part.material.with_emission(1.0 + cutting * 2.2),
                        texture: part.texture,
                    });
                }
                if cutting > 0.2 {
                    lights.push(PointLight3D::new(
                        world.transform_point3(Vec3::new(0.0, 0.45, 0.0)),
                        [0.66, 0.45, 1.0],
                        cutting * 1.6,
                        1.6,
                    ));
                }
            }
            None => {
                let grip = view_arm(
                    &mut draws,
                    scene.fist_r,
                    scene.cuff,
                    scene.upper,
                    held.hand,
                    held.blade,
                    held.shoulder,
                    pole_r,
                    true,
                    p.pose.roll,
                );
                push(
                    &mut draws,
                    &scene.sword,
                    grip * Mat4::from_scale(Vec3::new(vs, vs, vs)),
                );
            }
        }
        if let Some((combo, st)) = p.swing {
            let clip = fp::SWINGS[combo];
            if (clip.trail.0..=clip.trail.1).contains(&st) {
                let vertices = fp::trail_vertices(camera, combo, st, p.from, 0.0, 0.0, 0.0);
                if self
                    .renderer
                    .update_mesh_vertices(&self.queue, scene.trail, &vertices)
                    .is_ok()
                {
                    let mut material = Material3D::rgb(0.9, 0.8, 1.0).with_emission(1.2);
                    material.base_color[3] = 0.36;
                    draws.push(MeshDraw3D {
                        mesh: scene.trail,
                        transform: Mat4::IDENTITY,
                        material,
                        texture: None,
                    });
                }
            }
        }
        let pace = if p.running { 13.0 } else { 9.5 };
        let bob = if p.moving {
            (p.time * pace + 1.2).sin()
        } else {
            0.0
        };
        let duck = match p.swing {
            Some((combo, st)) => {
                ((st / fp::SWINGS[combo].duration).min(1.0) * PI).sin() * 0.035 * (1.0 - p.cast)
            }
            None => 0.0,
        };
        let k = p.cast * p.cast * (3.0 - 2.0 * p.cast);
        let seal_hand = Vec3::new(-0.5, -0.29, -0.58).lerp(Vec3::new(-0.2, -0.17, -0.6), k);
        let seal_up = Vec3::new(-0.12, 1.0, -0.4)
            .normalize_or_zero()
            .lerp(Vec3::new(0.08, 0.45, -1.0).normalize_or_zero(), k)
            .normalize_or_zero();
        let shake = if p.channeling {
            0.004 * (p.time * 61.0).sin()
        } else {
            0.0
        };
        let local = seal_hand
            + Vec3::new(
                bob * 0.01 - duck * 0.4 + shake,
                if p.moving { bob.abs() * -0.014 } else { 0.0 }
                    + (p.time * 1.3 + 0.7).sin() * 0.004
                    - duck
                    + shake,
                duck * 0.6,
            );
        let hand_l = camera.position + fp::camera_vector(camera, local);
        let up_l = fp::camera_vector(camera, seal_up).normalize_or_zero();
        let shoulder_l = camera.position + fp::camera_vector(camera, Vec3::new(-0.22, -0.48, 0.06));
        let pole_l = fp::camera_vector(camera, Vec3::new(-0.7, -1.0, 0.25)).normalize_or_zero();
        let pulse = 0.5 + 0.5 * (p.time * if p.channeling { 22.0 } else { 3.0 }).sin();
        let gem_glow = 0.7 + p.glow * (0.9 + pulse * 0.5) + p.cast * 0.3;
        let tip = match &scene.arms {
            Some(arms) => {
                let grip_l = view_arm_parts(
                    &mut draws,
                    &arms.hand_l,
                    &arms.sleeve,
                    &arms.sleeve_upper,
                    hand_l,
                    up_l,
                    shoulder_l,
                    pole_l,
                    false,
                    0.0,
                );
                let fist_l = grip_l * Mat4::from_scale(Vec3::new(vs, vs, vs));
                push(&mut draws, &arms.sceptre, fist_l);
                for part in &arms.gem {
                    draws.push(MeshDraw3D {
                        mesh: part.mesh,
                        transform: fist_l * part.transform,
                        material: part.material.with_emission(gem_glow * 1.6),
                        texture: part.texture,
                    });
                }
                // Its stone always lights her hand a little.
                let stone = fist_l.transform_point3(Vec3::new(0.0, 0.255, 0.0));
                lights.push(PointLight3D::new(
                    stone,
                    [0.63, 0.44, 1.0],
                    0.35 + gem_glow * 0.25,
                    0.9,
                ));
                stone
            }
            None => {
                let grip_l = view_arm(
                    &mut draws,
                    scene.fist_l,
                    scene.cuff,
                    scene.upper,
                    hand_l,
                    up_l,
                    shoulder_l,
                    pole_l,
                    false,
                    0.0,
                );
                let fist_l = grip_l * Mat4::from_scale(Vec3::new(vs, vs, vs));
                push(&mut draws, &scene.seal_rod, fist_l);
                let spin = p.time * (0.4 + p.glow * 6.0);
                let face_m = fist_l
                    * Mat4::from_translation(Vec3::new(0.0, 0.13, 0.0))
                    * Mat4::from_rotation_y(spin)
                    * Mat4::from_rotation_z(-PI / 2.0);
                push(&mut draws, &scene.seal_face, face_m);
                for part in &scene.seal_gem {
                    draws.push(MeshDraw3D {
                        mesh: part.mesh,
                        transform: face_m * part.transform,
                        material: part.material.with_emission(gem_glow * 1.4),
                        texture: part.texture,
                    });
                }
                face_m.transform_point3(Vec3::new(0.0, 0.014, 0.0))
            }
        };
        if p.glow > 0.0 {
            lights.push(PointLight3D::new(
                tip,
                [0.63, 0.44, 1.0],
                p.glow * (2.0 + pulse * 1.2),
                2.4,
            ));
        }
        // The spark. While she holds the seal its stone gathers light: motes spiral into it and a
        // ring of light turns round it. Sent, a beam of violet round a white-gold core runs to his
        // chest, branching in short-lived forks, with beads of light racing along it; where it
        // strikes, the emblem flares, rings of light spread from it and sparks spray off. Each
        // memory it wakes (a third of the spark) bursts in a great ring and a shower; off target,
        // it crackles off the stone.
        let charging = p.channeling || p.glow > 0.05;
        let charge = p
            .glow
            .clamp(0.0, 1.0)
            .max(if p.channeling { 0.35 } else { 0.0 });
        if charging {
            let n = 10;
            for m in 0..n {
                let seed = m as f32 * 1.618;
                let u = ((t * (0.9 + hash(seed) * 0.6) + hash(seed + 3.0)) % 1.0).abs();
                let r = 0.16 * (1.0 - u) * (0.6 + charge * 0.7);
                let a = seed * 2.4 + t * (5.0 + 3.0 * hash(seed + 1.0)) + u * 4.0;
                let right = fp::camera_vector(camera, Vec3::X);
                let up = fp::camera_vector(camera, Vec3::Y);
                let at = tip + (right * a.cos() + up * a.sin()) * r;
                let c = if m % 3 == 0 {
                    [1.0, 0.85, 0.55]
                } else {
                    [0.72, 0.52, 1.0]
                };
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: face(at, 0.012 + 0.016 * u),
                    material: Material3D::additive([
                        c[0] * 3.0,
                        c[1] * 3.0,
                        c[2] * 3.0,
                        u * charge,
                    ]),
                    texture: Some(scene.halo),
                });
            }
            let turn = t * 2.4;
            let f = (camera.position - tip).normalize_or_zero();
            let right = Vec3::Y.cross(f).normalize_or_zero();
            let up = f.cross(right).normalize_or_zero();
            let size = 0.07 + 0.05 * charge + 0.01 * (t * 9.0).sin();
            let (s, c) = turn.sin_cos();
            let (rx, ry) = (right * c + up * s, up * c - right * s);
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: Mat4::from_cols(
                    [rx.x * size, rx.y * size, rx.z * size, 0.0],
                    [ry.x * size, ry.y * size, ry.z * size, 0.0],
                    [f.x, f.y, f.z, 0.0],
                    [tip.x, tip.y, tip.z, 1.0],
                ),
                material: Material3D::additive([1.6, 1.2, 2.6, 0.5 + 0.4 * charge]),
                texture: Some(scene.ring),
            });
        }
        if fight.beam > 0.01 {
            let to = if fight.flowing {
                emblem
            } else {
                let d = (emblem - tip).normalize_or_zero();
                tip + d * (0.6 + hash(t * 13.0) * 0.5)
            };
            let dir = (to - tip).normalize_or_zero();
            let len = (to - tip).length();
            let side = dir.cross(Vec3::Y).normalize_or_zero();
            let up = side.cross(dir).normalize_or_zero();
            // A path through the air: the jagged line the beam takes this frame (it changes some 25
            // times a second, as a spark does), so the layers and the beads follow the same bolt.
            let snap = (t * 25.0).floor();
            let path = |u: f32, k: f32| {
                let amp = (u * PI).sin() * (0.10 + k * 0.06) * (len / 4.0).min(1.0);
                let n = |w: f32| (hash(snap * 1.37 + u * w + k * 5.1) - 0.5) * 2.0;
                tip.lerp(to, u)
                    + side * ((t * 9.0 + u * 11.0 + k).sin() * amp * 0.5 + n(7.3) * amp)
                    + up * ((t * 7.0 + u * 13.0 + k * 2.0).cos() * amp * 0.5 + n(9.1) * amp)
            };
            let ribbon = |mesh: MeshHandle,
                          from_u: f32,
                          to_u: f32,
                          width: f32,
                          k: f32,
                          offset: &dyn Fn(f32) -> Vec3,
                          renderer: &mut Wgpu3DRenderer,
                          queue: &wgpu::Queue|
             -> bool {
                let mut vertices = Vec::with_capacity(BEAM_POINTS * 2);
                for i in 0..BEAM_POINTS {
                    let s = i as f32 / (BEAM_POINTS - 1) as f32;
                    let u = from_u + (to_u - from_u) * s;
                    let at = path(u, k) + offset(s);
                    let to_cam = (camera.position - at).normalize_or_zero();
                    let taper = (s * PI).sin().max(0.25);
                    let wv = dir.cross(to_cam).normalize_or_zero() * (width * taper);
                    for sgn in [1.0f32, -1.0] {
                        let q = at + wv * sgn;
                        vertices.push(MeshVertex3D::new(q.to_array(), [0.0, 1.0, 0.0], [s, 0.0]));
                    }
                }
                renderer
                    .update_mesh_vertices(queue, mesh, &vertices)
                    .is_ok()
            };
            let none = |_: f32| Vec3::ZERO;
            let surge = 0.85 + 0.15 * (t * 31.0).sin();
            // The violet sheath, the lilac glow and the white-gold core.
            for (k, (width, colour, alpha)) in [
                (0.11, [0.55, 0.30, 1.0], 0.35),
                (0.05, [0.80, 0.55, 1.0], 0.55),
                (0.018, [1.0, 0.92, 0.72], 1.0),
            ]
            .into_iter()
            .enumerate()
            {
                if ribbon(
                    scene.beam[k],
                    0.0,
                    1.0,
                    width * surge,
                    0.0,
                    &none,
                    &mut self.renderer,
                    &self.queue,
                ) {
                    draws.push(MeshDraw3D {
                        mesh: scene.beam[k],
                        transform: Mat4::IDENTITY,
                        material: Material3D::additive([
                            colour[0] * 2.2,
                            colour[1] * 2.2,
                            colour[2] * 2.2,
                            fight.beam * alpha,
                        ]),
                        texture: None,
                    });
                }
            }
            // The forks: short bolts leaving the beam, each living a few frames.
            for (f, mesh) in scene.forks.iter().enumerate() {
                let life = (snap + f as f32 * 3.0) / 3.0;
                let seed = life.floor() * 7.1 + f as f32 * 13.3;
                let start = 0.15 + hash(seed) * 0.7;
                let reach = 0.25 + hash(seed + 1.0) * 0.45;
                let a = hash(seed + 2.0) * std::f32::consts::TAU;
                let out = (side * a.cos() + up * a.sin()) * reach;
                let offset = move |s: f32| {
                    let along = dir * (s * 0.25);
                    let wiggle = (side * (hash(seed + s * 9.0) - 0.5)
                        + up * (hash(seed + s * 5.0) - 0.5))
                        * 0.08;
                    out * s + along + wiggle
                };
                if ribbon(
                    *mesh,
                    start,
                    start,
                    0.012,
                    0.0,
                    &offset,
                    &mut self.renderer,
                    &self.queue,
                ) {
                    draws.push(MeshDraw3D {
                        mesh: *mesh,
                        transform: Mat4::IDENTITY,
                        material: Material3D::additive([1.6, 1.2, 2.6, fight.beam * 0.8]),
                        texture: None,
                    });
                }
            }
            // Beads racing along it.
            for m in 0..7 {
                let u = (t * (1.6 + 0.3 * hash(m as f32)) + m as f32 / 7.0) % 1.0;
                let gold = m % 2 == 0;
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: face(path(u, 0.0), 0.05 + 0.03 * (u * PI).sin()),
                    material: Material3D::additive(if gold {
                        [3.0, 2.4, 1.4, fight.beam]
                    } else {
                        [2.0, 1.4, 3.2, fight.beam]
                    }),
                    texture: Some(scene.halo),
                });
            }
            // At the stone: a flare.
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: face(tip, 0.16 + 0.04 * (t * 23.0).sin()),
                material: Material3D::additive([2.6, 2.2, 3.4, 0.8 * fight.beam]),
                texture: Some(scene.halo),
            });
            lights.push(PointLight3D::new(
                tip.lerp(to, 0.5),
                [0.7, 0.5, 1.0],
                3.5 * fight.beam * surge,
                5.0,
            ));
            if fight.flowing {
                // Where it strikes: the flare, rings spreading off his chest, sparks spraying.
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: face(emblem + toward * 0.08, 0.7 + 0.15 * (t * 17.0).sin()),
                    material: Material3D::additive([2.4, 1.9, 3.2, 0.7 * fight.beam]),
                    texture: Some(scene.halo),
                });
                for r in 0..3 {
                    let k = (t * 1.8 + r as f32 / 3.0) % 1.0;
                    draws.push(MeshDraw3D {
                        mesh: scene.quad,
                        transform: face(emblem + toward * 0.06, 0.3 + k * 1.6),
                        material: Material3D::additive([
                            1.6,
                            1.1,
                            2.6,
                            (1.0 - k) * 0.7 * fight.beam,
                        ]),
                        texture: Some(scene.ring),
                    });
                }
                for _ in 0..3 {
                    let s = scene
                        .mote_seed
                        .wrapping_mul(1_664_525)
                        .wrapping_add(1_013_904_223);
                    scene.mote_seed = s;
                    let r = |n: u32| ((s.rotate_left(n) >> 8) as f32) / 16_777_216.0;
                    let out =
                        (toward * 0.6 + side * (r(3) - 0.5) * 2.0 + Vec3::Y * (r(7) * 1.2 - 0.2))
                            .normalize_or_zero();
                    scene.motes.push(Mote {
                        at: emblem,
                        v: out * (2.0 + r(11) * 3.0),
                        life: 0.35 + r(13) * 0.4,
                        age: 0.0,
                        size: 0.025 + r(17) * 0.03,
                        gold: r(19) > 0.5,
                    });
                }
            } else if fight.fizzle {
                // Off target: it crackles off the stone.
                for _ in 0..2 {
                    let s = scene
                        .mote_seed
                        .wrapping_mul(1_664_525)
                        .wrapping_add(1_013_904_223);
                    scene.mote_seed = s;
                    let r = |n: u32| ((s.rotate_left(n) >> 8) as f32) / 16_777_216.0;
                    let out = Vec3::new(r(3) - 0.5, r(5) * 0.8, r(7) - 0.5).normalize_or_zero();
                    scene.motes.push(Mote {
                        at: tip,
                        v: out * (1.0 + r(11) * 1.5),
                        life: 0.2 + r(13) * 0.25,
                        age: 0.0,
                        size: 0.015 + r(17) * 0.015,
                        gold: r(19) > 0.6,
                    });
                }
            }
        }
        // A memory woken (a third of the spark): a great ring of light, a flash, a shower.
        if v.memories > scene.memories_seen {
            scene.memories_seen = v.memories;
            scene.burst_at = t;
            for i in 0..70 {
                let s = scene
                    .mote_seed
                    .wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223 + i);
                scene.mote_seed = s;
                let r = |n: u32| ((s.rotate_left(n) >> 8) as f32) / 16_777_216.0;
                let a = r(3) * std::f32::consts::TAU;
                let e = r(5) * 1.4 - 0.4;
                let out = Vec3::new(a.cos() * e.cos(), e.sin() + 0.3, a.sin() * e.cos())
                    .normalize_or_zero();
                scene.motes.push(Mote {
                    at: emblem,
                    v: out * (3.0 + r(11) * 5.0),
                    life: 0.6 + r(13) * 0.9,
                    age: 0.0,
                    size: 0.03 + r(17) * 0.05,
                    gold: r(19) > 0.4,
                });
            }
        }
        if v.memories < scene.memories_seen {
            scene.memories_seen = v.memories;
        }
        let since = t - scene.burst_at;
        if (0.0..1.2).contains(&since) {
            let k = since / 1.2;
            for (grow, alpha) in [(6.0, 0.9), (3.5, 0.6)] {
                draws.push(MeshDraw3D {
                    mesh: scene.quad,
                    transform: face(emblem + toward * 0.1, 0.4 + ease_out_cubic(k) * grow),
                    material: Material3D::additive([2.6, 2.0, 3.4, (1.0 - k) * alpha]),
                    texture: Some(scene.ring),
                });
            }
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: face(emblem + toward * 0.12, 2.6 * (1.0 - k * 0.5)),
                material: Material3D::additive([3.0, 2.5, 3.6, (1.0 - k).powi(2) * 0.9]),
                texture: Some(scene.halo),
            });
            lights.push(PointLight3D::new(
                emblem,
                [0.85, 0.65, 1.0],
                18.0 * (1.0 - k).powi(2),
                12.0,
            ));
        }
        // The sparks in the air: flung out, falling, winking out.
        for m in &mut scene.motes {
            m.age += dt;
            m.v.y -= 6.0 * dt;
            m.v = m.v * (1.0 - 1.6 * dt);
            m.at = m.at + m.v * dt;
        }
        scene.motes.retain(|m| m.age < m.life);
        if scene.motes.len() > 400 {
            let extra = scene.motes.len() - 400;
            scene.motes.drain(0..extra);
        }
        for m in &scene.motes {
            let k = m.age / m.life;
            let c = if m.gold {
                [3.0, 2.3, 1.2]
            } else {
                [2.0, 1.4, 3.4]
            };
            draws.push(MeshDraw3D {
                mesh: scene.quad,
                transform: face(m.at, m.size * (1.0 - 0.6 * k)),
                material: Material3D::additive([c[0], c[1], c[2], 1.0 - k]),
                texture: Some(scene.halo),
            });
        }
        // The lamps nearest her (the braziers first: they light the throne).
        let me = Vec3::new(p.x, 1.0, p.z);
        let mut near: Vec<(f32, usize)> = plan
            .lamps
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    (l.x - me.x).hypot(l.z - me.z) - if l.power > 0.9 { 30.0 } else { 0.0 },
                    i,
                )
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        // As many as the frame has room for (the renderer takes 32; two kept for what flares
        // late). The big fires reach far; a torch or a lantern lights a pool round itself, so the
        // rock between them stays in the moon's cold light.
        let room = 30usize.saturating_sub(lights.len());
        for (_, i) in near.into_iter().take(room) {
            let l = &plan.lamps[i];
            let flicker =
                1.0 + (t * 7.0 + i as f32 * 1.7).sin() * 0.07 + (t * 17.0 + i as f32).sin() * 0.04;
            let (strength, reach) = if l.power > 0.9 {
                (9.5, 12.0)
            } else {
                (7.0, 7.5)
            };
            lights.push(PointLight3D::new(
                Vec3::new(l.x, l.y, l.z),
                colour(&l.color),
                strength * l.power * flicker,
                reach,
            ));
        }
        if std::env::var_os("VELVET_BOSS_LIGHTS").is_some() {
            eprintln!("[boss-lights] {} lights", lights.len());
        }
        let saved = self.renderer.replace_finish(Finish3D {
            bloom: 0.62,
            bloom_threshold: 1.0,
            contrast: 1.12,
            saturation: 1.08,
            gi: 1.5,
            ..penascal3d::finish()
        });
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            mood(frame.environment),
            &lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
        self.renderer.replace_finish(saved);
        // The HUD.
        let float = fight.float.and_then(|(text, growl, until)| {
            let over = scene.over;
            let (x, y) = project_npc_anchor(camera, over)?;
            Some((
                text.to_owned(),
                growl,
                [x, y],
                ((until - t) / 0.5).clamp(0.0, 1.0),
            ))
        });
        let card = match fight.phase {
            Phase::Intro => Some(map_overlay::BossCard {
                kicker: "La plaza del Peñascal · al caer la noche".into(),
                title: "Varrak, el Colmillo".into(),
                text: "Despiértalo, no lo mates. Rómpele la guardia con la espada y, cuando quede aturdido, mándale la chispa con el sello. Si se pone a cuatro patas y carga, llévalo contra un peñasco.".into(),
                keys: true,
                actions: vec![("Clic".into(), "Entrar en la plaza".into())],
            }),
            Phase::Won if fight.card => Some(map_overlay::BossCard {
                kicker: "La chispa ha prendido".into(),
                title: "Varrak despierta".into(),
                text: "«Majestad. Ha tardado usted.» Cae de rodillas, con el emblema encendido en el pecho. La luz del sello sube por encima de las peñas: se ha visto desde muy lejos.".into(),
                keys: false,
                actions: vec![("Clic".into(), "Seguir".into())],
            }),
            Phase::Lost if fight.card => Some(map_overlay::BossCard {
                kicker: "La Reina cae".into(),
                title: "Todavía no".into(),
                text: "Chiquita te saca a rastras de la plaza. Varrak vuelve a sentarse en su piedra, con la cuchilla clavada a su lado. Esperará.".into(),
                keys: false,
                actions: vec![
                    ("Clic".into(), "Repetir el combate".into()),
                    ("Clic dcho.".into(), "Volver al Peñascal".into()),
                ],
            }),
            _ => None,
        };
        let view = map_overlay::BossView {
            time: t,
            bars: fight.phase != Phase::Intro,
            spark: v.spark,
            guard: if v.open() { 0.0 } else { v.guard },
            guard_label: if v.open() {
                "Aturdido"
            } else if v.state == VState::Dazed {
                "Atontado"
            } else {
                "Guardia"
            },
            stunned: v.open(),
            reached: [0, 1, 2].map(|i| v.spark >= varrak_fight::MEMORIES[i].at),
            notches: [0, 1, 2].map(|i| varrak_fight::MEMORIES[i].label),
            health: p.health / p.max_health,
            crosshair: if fight.flowing {
                2
            } else if v.open() {
                1
            } else {
                0
            },
            float,
            line: fight.line_typed(),
            vignette: fight.vignette,
            flash: fight.flash,
            card,
            objective: fight.objective().map(|(head, detail, urgency)| {
                if scene.objective.0 != head {
                    scene.objective = (head.clone(), t);
                }
                (head, detail, urgency, scene.objective.1)
            }),
            running: p.running,
            channeling: p.channeling,
            qte: fight.qte.as_ref().map(|q| {
                use varrak_fight::QteAsk;
                let at = project_npc_anchor(camera, q.anchor)
                    .map(|(x, y)| [x, y])
                    .unwrap_or([WINDOW_WIDTH as f32 * 0.5, WINDOW_HEIGHT as f32 * 0.62]);
                map_overlay::BossQte {
                    key: q.key.to_owned(),
                    text: q.text.to_owned(),
                    sub: q.sub.to_owned(),
                    at,
                    ring: match q.ask {
                        QteAsk::Beat { beat, slack } => Some(((beat - q.t) / beat, slack / beat)),
                        _ => None,
                    },
                    fill: matches!(q.ask, QteAsk::Hold { .. } | QteAsk::Mash { .. })
                        .then_some(q.fill),
                    left: (q.window > 0.0).then(|| (1.0 - q.t / q.window).clamp(0.0, 1.0)),
                    step: q.step,
                    steps: q.steps,
                    danger: matches!(
                        q.kind,
                        varrak_fight::QteKind::Dodge | varrak_fight::QteKind::Grab
                    ),
                    age: q.t,
                }
            }),
            stamp: fight.stamp.map(|(good, at)| (good, fight.clock - at)),
            slow: fight.time_scale,
        };
        self.boss_scene = Some(scene);
        self.map_overlay.set_boss(Some(view));
        let Frame {
            player,
            yaw,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            shop_preview_rotation,
            venue,
            elapsed,
            ..
        } = *frame;
        self.map_overlay.draw(
            &self.device,
            &self.queue,
            &self.target_view,
            (WINDOW_WIDTH, WINDOW_HEIGHT),
            player,
            yaw,
            false,
            false,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            elapsed,
            shop_preview_rotation,
            &[],
            Some(venue),
            None,
            None,
            false,
        );
        self.map_overlay.set_boss(None);
    }
}

/// What the mouse did in the fight since the last step.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BossInput {
    pub attack: bool,
    pub jump: bool,
    pub channel: bool,
    /// The right button pressed (a card's second choice).
    pub back: bool,
}

impl App {
    /// The fight with Varrak in first person (after his arrival is told).
    pub(super) fn start_boss_fight(&mut self) {
        if !self.state.as_mut().is_some_and(|s| s.ensure_boss_loaded()) {
            self.queen_thought = Some((
                "Varrak no se mueve de su piedra… todavía.",
                self.elapsed + 4.0,
            ));
            return;
        }
        let hunted = self.story.get("apagados_caza") > 0;
        let wolves = if hunted {
            2
        } else if self.story.get("apagados_echar") > 0 {
            6
        } else {
            4
        };
        let seed = (self.elapsed * 1000.0) as u32 ^ 0x5eed_1234;
        self.boss = Some(Fight::new(wolves, hunted, seed));
        // Nyxara's gifts: a quicker spark, longer dazes, more vitality.
        let gift = |on: bool, value: f32| if on { value } else { 1.0 };
        let spark = gift(self.has_gift(court::DON_CHISPA), 1.25);
        let stun = if self.has_gift(court::DON_SELLO) {
            1.0
        } else {
            0.0
        };
        let vitality = gift(self.has_gift(court::DON_AGUANTE), 1.25);
        if let Some(fight) = self.boss.as_mut() {
            fight.set_gifts(spark, stun, vitality);
        }
        // The sword she wears: its weight and what it carries (and her skill with it).
        self.apply_equipment();
        self.boss_input = BossInput::default();
        // Night is falling over the hills.
        self.environment_cycle.set_phase(0.86);
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
        self.dungeon_mouse_captured = self.capture_dungeon_mouse();
        eprintln!("[acto1] the fight with Varrak begins ({wolves} wolves)");
    }

    /// One step of the fight: her keys and mouse into it, its sounds out, the cards' clicks.
    pub(super) fn tick_boss(&mut self, dt: f32) {
        let input = std::mem::take(&mut self.boss_input);
        self.boss_input.channel = input.channel;
        let Some(fight) = self.boss.as_mut() else {
            return;
        };
        match fight.phase {
            Phase::Intro if input.attack => {
                fight.begin();
            }
            Phase::Won if fight.card && input.attack => {
                self.finish_boss(true);
                return;
            }
            Phase::Lost if fight.card && (input.attack || input.back) => {
                if input.attack {
                    // Again, from the gate.
                    self.boss = None;
                    self.start_boss_fight();
                } else {
                    self.finish_boss(false);
                }
                return;
            }
            _ => {}
        }
        let Some(fight) = self.boss.as_mut() else {
            return;
        };
        let begun = !matches!(fight.phase, Phase::Intro);
        let step = varrak_fight::Input {
            forward: self.move_forward as u8 as f32 - self.move_back as u8 as f32,
            strafe: self.move_right as u8 as f32 - self.move_left as u8 as f32,
            run: self.sprint,
            jump: input.jump,
            attack: input.attack && begun,
            channel: input.channel,
        };
        fight.update(step, dt);
        let sounds = std::mem::take(&mut fight.sounds);
        let (eye, forward) = (fight.player.eye(), fight.player.forward());
        self.player = Vec3::new(fight.player.x, 0.0, fight.player.z);
        self.poll_audio_boot();
        if !self.audio_tried {
            self.audio_tried = true;
            self.audio = game_audio::GameAudio::new();
        }
        if let Some(audio) = &mut self.audio {
            for sound in &sounds {
                audio.play(sound, eye, forward);
            }
        }
        self.elapsed += dt;
    }

    /// Out of the fight: won, his waking by the stone; lost, back in the plaza.
    pub(super) fn finish_boss(&mut self, won: bool) {
        self.boss = None;
        self.boss_input = BossInput::default();
        self.release_dungeon_mouse();
        self.clear_input();
        self.enter_penascal_by_the_stone();
        let practice = std::mem::take(&mut self.boss_practice);
        if won && practice {
            self.float_chiquita_after(
                "—Ensayo superado. Cuando llegue la de verdad, quiero verte igual de rápida.",
                mascot::Cue::Talk,
                0.6,
            );
        } else if won {
            self.story.values.insert("varrak_vencido".to_owned(), 1);
            self.start_talk_at("acto1.varrak_final");
        } else {
            self.float_chiquita_after(
                "—Respira. Varrak no se va a mover de su piedra: vuelve cuando quieras.",
                mascot::Cue::Talk,
                0.6,
            );
        }
        self.persist_3d_state(true);
    }

    /// The fight's keys: walking and running as anywhere, Space to jump, F or Enter as a click.
    pub(super) fn boss_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        self.set_key(code, pressed);
        if !pressed || repeat {
            return;
        }
        match code {
            KeyCode::Space => self.boss_input.jump = true,
            KeyCode::KeyF | KeyCode::Enter => self.boss_input.attack = true,
            KeyCode::KeyQ => self.quick_cure(),
            _ => {}
        }
    }

    /// The fight's mouse buttons.
    pub(super) fn boss_button(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left if pressed => self.boss_input.attack = true,
            MouseButton::Right => {
                self.boss_input.channel = pressed;
                if pressed {
                    self.boss_input.back = true;
                }
            }
            _ => {}
        }
        if pressed && !self.dungeon_mouse_captured {
            self.dungeon_mouse_captured = self.capture_dungeon_mouse();
        }
    }
}

/// The fight as a place.
pub(super) struct BossZone;

impl Zone for BossZone {
    fn id(&self) -> ZoneId {
        ZoneId::Boss
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_boss(dt);
    }
    fn prompt(&self, _app: &App) -> Option<Prompt> {
        None
    }
    fn interact(&self, _app: &mut App) -> bool {
        false
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Cave
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        if let Some(fight) = frame.boss {
            state.render_boss(frame, fight);
        }
    }
    fn title(&self, _app: &App) -> (&'static str, &'static str) {
        ("Combate: Varrak, el Colmillo", "")
    }
}
