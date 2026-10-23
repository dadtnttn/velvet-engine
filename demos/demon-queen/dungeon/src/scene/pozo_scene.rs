//! The miner's shaft drawn: its blocks, built into a mesh per texture and band of depth (rebuilt
//! where she digs), the ores and gems showing in the rock, the room round its mouth with the winch
//! and its lamps, the pick in her hand and the block she aims at. Also the winch's head-frame by the
//! Tendero's stall in the mine, where she goes down.
use super::*;
use crate::pozo::{self, Find, Pozo, Rock, LEDGE, ROOM_HEIGHT, WIDTH};
use std::collections::HashMap;

/// Depth bands of the shaft's meshes.
const BAND: i32 = 6;

/// One thing showing in an open face of rock: an ore's stud, a gem's crystal, a chest's corner.
#[derive(Clone, Copy)]
struct Nugget {
    at: Vec3,
    normal: Vec3,
    color: [f32; 3],
    glow: f32,
    size: f32,
    chest: bool,
    /// A crystal leans toward `axis` (zero: straight out).
    axis: Vec3,
}

/// The painted strata (`tools/pozo_texturas.py`), in `Rock::ALL`'s order; then the planks.
const ROCK_TEXTURES: [&str; 8] = [
    "pozo_tierra",
    "pozo_piedra",
    "pozo_pizarra",
    "pozo_granito",
    "pozo_basalto",
    "pozo_obsidiana",
    "pozo_abisal",
    "pozo_corazon",
];
const PLANKS: usize = 8;

/// What moves a part of the head-frame, and where it shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    /// the tower, its sign, lanterns and head (in the mine)
    Frame,
    /// the sheave wheel and the winch's drum: they turn about their axles
    Wheel,
    Drum,
    /// the cage (in the mine and down in the shaft's room)
    Cage,
    /// the cart of ore, its rails, the stones and the pick beside it (both)
    Yard,
}

struct HeadPart {
    piece: Piece,
    mesh: MeshHandle,
    base: Mat4,
    material: Material3D,
    texture: Option<TextureHandle>,
}

/// The head-frame's model (tools/castillete_blender.py) on the GPU, and the planks' texture for
/// the timbers drawn round it.
pub(super) struct Headframe {
    parts: Vec<HeadPart>,
    wood: Option<TextureHandle>,
    iron: Option<TextureHandle>,
}

/// The wheel's hub and radius, the lanterns, the cage's shackle, in the model's frame (y up, its
/// open front toward +z; the script prints them).
const HUB: Vec3 = Vec3::new(0.0, 5.22, -0.75);
const WHEEL_R: f32 = 0.72;
const LANTERNS: [Vec3; 2] = [
    Vec3::new(-0.891, 1.98, 1.321),
    Vec3::new(0.891, 1.98, 1.321),
];
const CAGE_TOP: f32 = 2.67;

impl Headframe {
    pub(super) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<Self> {
        let path = super::super::assets::root()
            .join("mina")
            .join("castillete.glb");
        let model = GltfModel::load(&path).ok()?;
        let wood = load_texture(renderer, device, queue, "old_planks_02");
        let iron = load_texture(renderer, device, queue, "rusty_metal_04");
        let mut parts = Vec::new();
        for (index, mesh) in model.meshes.iter().enumerate() {
            let name = model
                .mesh_names
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            let piece = if name.starts_with("rueda") {
                Piece::Wheel
            } else if name.starts_with("tambor") {
                Piece::Drum
            } else if name.starts_with("jaula") {
                Piece::Cage
            } else if ["vagoneta", "carga", "piedras"]
                .iter()
                .any(|p| name.starts_with(p))
                || name.contains("vias")
                || name.contains("pico")
            {
                Piece::Yard
            } else {
                Piece::Frame
            };
            // the scanned textures are dark: lifted, as the mine's
            let (texture, lift) = if name.contains("madera") {
                (wood, 1.35)
            } else if name.contains("hierro") {
                (iron, 1.25)
            } else {
                (None, 1.0)
            };
            let c = mesh.material.base_color;
            let mut material =
                Material3D::rgb(c[0] * lift, c[1] * lift, c[2] * lift).with_ambient_occlusion(0.2);
            if name.starts_with("farol") {
                material = material.with_emission(3.2);
            } else if name.starts_with("carga") && !name.contains("carbon") {
                material = material.with_emission(0.35);
            }
            parts.push(HeadPart {
                piece,
                mesh: renderer.upload_mesh(device, mesh).ok()?,
                base: model.mesh_transform(index),
                material,
                texture,
            });
        }
        (!parts.is_empty()).then_some(Self { parts, wood, iron })
    }
}

/// What the shaft keeps on the GPU.
pub(super) struct PozoArt {
    seed: u64,
    /// the strata's painted rock ([`ROCK_TEXTURES`]), then the planks
    textures: [Option<TextureHandle>; 9],
    bands: HashMap<(i32, usize), MeshHandle>,
    nuggets: HashMap<i32, Vec<Nugget>>,
    built: HashMap<i32, u64>,
    room: Vec<(MeshHandle, usize)>,
    spare: Vec<MeshHandle>,
    version: u64,
}

pub(super) fn load_texture(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    name: &str,
) -> Option<TextureHandle> {
    let dir = super::super::assets::root().join("mina").join("tex");
    let read = |file: String| -> Option<TextureData3D> {
        let img = image::open(dir.join(file)).ok()?.to_rgba8();
        Some(TextureData3D {
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        })
    };
    let diffuse = read(format!("{name}_diff.jpg"))?;
    let normal = read(format!("{name}_nor_gl.jpg"));
    renderer
        .upload_texture_with_normal(device, queue, &diffuse, normal.as_ref())
        .ok()
}

fn empty_mesh() -> Mesh3D {
    visual::empty()
}

/// A quad of a block's face into `mesh`: `corner` and the two edges, its normal, its colour; the
/// texture tiles every two metres.
fn quad(mesh: &mut Mesh3D, corner: Vec3, u: Vec3, v: Vec3, normal: Vec3, color: [f32; 3]) {
    let base = mesh.vertices.len() as u32;
    let uv = |p: Vec3| -> [f32; 2] {
        if normal.y.abs() > 0.5 {
            [p.x * 0.5, p.z * 0.5]
        } else if normal.x.abs() > 0.5 {
            [p.z * 0.5, -p.y * 0.5]
        } else {
            [p.x * 0.5, -p.y * 0.5]
        }
    };
    for p in [corner, corner + u, corner + u + v, corner + v] {
        mesh.vertices.push(MeshVertex3D {
            position: p.to_array(),
            normal: normal.to_array(),
            uv: uv(p),
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    // facing `normal`
    if u.cross(v).dot(normal) >= 0.0 {
        mesh.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    } else {
        mesh.indices
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

fn hash3(x: i32, k: i32, z: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (k as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D)
        ^ salt.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

/// The six sides of a block: the neighbour's offset, the face's normal.
const SIDES: [([i32; 3], [f32; 3]); 6] = [
    ([0, -1, 0], [0.0, 1.0, 0.0]),
    ([0, 1, 0], [0.0, -1.0, 0.0]),
    ([1, 0, 0], [1.0, 0.0, 0.0]),
    ([-1, 0, 0], [-1.0, 0.0, 0.0]),
    ([0, 0, 1], [0.0, 0.0, 1.0]),
    ([0, 0, -1], [0.0, 0.0, -1.0]),
];

/// The corner and edges of a block's face on side `n` (block `[x, k, z]` fills x..x+1, -k-1..-k).
fn face(x: i32, k: i32, z: i32, n: Vec3) -> (Vec3, Vec3, Vec3) {
    let (x0, y0, z0) = (x as f32, -(k as f32) - 1.0, z as f32);
    if n.y > 0.5 {
        (Vec3::new(x0, y0 + 1.0, z0), Vec3::X, Vec3::Z)
    } else if n.y < -0.5 {
        (Vec3::new(x0, y0, z0), Vec3::X, Vec3::Z)
    } else if n.x > 0.5 {
        (Vec3::new(x0 + 1.0, y0, z0), Vec3::Z, Vec3::Y)
    } else if n.x < -0.5 {
        (Vec3::new(x0, y0, z0), Vec3::Z, Vec3::Y)
    } else if n.z > 0.5 {
        (Vec3::new(x0, y0, z0 + 1.0), Vec3::X, Vec3::Y)
    } else {
        (Vec3::new(x0, y0, z0), Vec3::X, Vec3::Y)
    }
}

impl PozoArt {
    fn new(
        pozo: &Pozo,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let textures = std::array::from_fn(|i| {
            load_texture(
                renderer,
                device,
                queue,
                ROCK_TEXTURES.get(i).copied().unwrap_or("old_planks_02"),
            )
        });
        let mut art = Self {
            seed: pozo.seed,
            textures,
            bands: HashMap::new(),
            nuggets: HashMap::new(),
            built: HashMap::new(),
            room: Vec::new(),
            spare: Vec::new(),
            version: u64::MAX,
        };
        art.build_room(renderer, device);
        art
    }

    /// The room round the mouth: the ledge's floor (planks by the cage), its walls and ceiling.
    fn build_room(&mut self, renderer: &mut Wgpu3DRenderer, device: &wgpu::Device) {
        let (lo, hi) = (-LEDGE as f32, (WIDTH + LEDGE) as f32);
        let mut stone = empty_mesh();
        let mut planks = empty_mesh();
        let mut walls = empty_mesh();
        for x in -LEDGE..WIDTH + LEDGE {
            for z in -LEDGE..WIDTH + LEDGE {
                if pozo::in_shaft(x, z) {
                    continue;
                }
                let deck = (2..8).contains(&x) && (-LEDGE..0).contains(&z);
                let shade = 0.82 + 0.18 * hash3(x, 0, z, 1);
                quad(
                    if deck { &mut planks } else { &mut stone },
                    Vec3::new(x as f32, 0.0, z as f32),
                    Vec3::X,
                    Vec3::Z,
                    Vec3::Y,
                    [shade, shade * 0.97, shade * 0.93],
                );
            }
        }
        let dark = [0.82, 0.78, 0.74];
        let span = hi - lo;
        for (corner, u, n) in [
            (Vec3::new(lo, 0.0, lo), Vec3::new(0.0, 0.0, span), Vec3::X),
            (Vec3::new(hi, 0.0, lo), Vec3::new(0.0, 0.0, span), -Vec3::X),
            (Vec3::new(lo, 0.0, lo), Vec3::new(span, 0.0, 0.0), Vec3::Z),
            (Vec3::new(lo, 0.0, hi), Vec3::new(span, 0.0, 0.0), -Vec3::Z),
        ] {
            // in strips, so the texture tiles
            let steps = span as i32;
            for i in 0..steps {
                for j in 0..ROOM_HEIGHT as i32 {
                    let c = corner + u * (i as f32 / steps as f32) + Vec3::Y * j as f32;
                    quad(&mut walls, c, u * (1.0 / steps as f32), Vec3::Y, n, dark);
                }
            }
        }
        for x in 0..span as i32 {
            for z in 0..span as i32 {
                quad(
                    &mut walls,
                    Vec3::new(lo + x as f32, ROOM_HEIGHT, lo + z as f32),
                    Vec3::X,
                    Vec3::Z,
                    -Vec3::Y,
                    [1.05, 1.0, 0.96],
                );
            }
        }
        for (mesh, texture) in [(stone, 1usize), (planks, PLANKS), (walls, 0)] {
            if let Ok(handle) = renderer.upload_mesh(device, &mesh) {
                self.room.push((handle, texture));
            }
        }
    }

    /// The meshes of one band of depth, and what shows in its open faces.
    fn build_band(
        &mut self,
        pozo: &Pozo,
        band: i32,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
    ) {
        let mut meshes: [Mesh3D; 8] = std::array::from_fn(|_| empty_mesh());
        let mut nuggets = Vec::new();
        for k in band * BAND..(band + 1) * BAND {
            for x in -1..=WIDTH {
                for z in -1..=WIDTH {
                    if !pozo.solid(x, k, z) {
                        continue;
                    }
                    let inside = pozo::in_shaft(x, z);
                    let block = pozo.block(x.clamp(0, WIDTH - 1), k, z.clamp(0, WIDTH - 1));
                    let wear = if inside { pozo.wear([x, k, z]) } else { 0.0 };
                    // the painted rock carries its colour: the block only shades it, and an ore's
                    // block takes a little of the ore's
                    let shade = (0.64 + 0.12 * hash3(x, k, z, 7))
                        * if inside { 1.0 } else { 0.62 }
                        * (1.0 - 0.5 * wear);
                    let tint = match block.find {
                        Some(Find::Ore(i)) => {
                            let o = pozo::ORES[i].color;
                            [0.62 + 0.45 * o[0], 0.62 + 0.45 * o[1], 0.62 + 0.45 * o[2]]
                        }
                        _ => [1.0, 1.0, 1.0],
                    };
                    let texture = block.rock.texture();
                    for (offset, normal) in SIDES {
                        let (nx, nk, nz) = (x + offset[0], k + offset[1], z + offset[2]);
                        let open = if nk < 0 {
                            inside
                        } else {
                            pozo::in_shaft(nx, nz) && pozo.open(nx, nk, nz)
                        };
                        if !open {
                            continue;
                        }
                        let n = Vec3::new(normal[0], normal[1], normal[2]);
                        let (corner, u, v) = face(x, k, z, n);
                        // floors catch the lamps, ceilings stay in shadow
                        let side = if n.y > 0.5 {
                            1.1
                        } else if n.y < -0.5 {
                            0.74
                        } else {
                            0.96
                        };
                        let color = [
                            tint[0] * shade * side,
                            tint[1] * shade * side,
                            tint[2] * shade * side,
                        ];
                        quad(&mut meshes[texture], corner, u, v, n, color);
                        if !inside {
                            continue;
                        }
                        let center = corner + (u + v) * 0.5;
                        match block.find {
                            Some(Find::Ore(i)) if pozo::ORES[i].glow >= 0.5 => {
                                // a gem: a cluster of crystals leaning out of the rock
                                let o = &pozo::ORES[i];
                                let count = 3 + (hash3(x, k, z, 31) * 3.0) as u32;
                                for j in 0..count {
                                    let du = hash3(x, k, z, 11 + j * 3) - 0.5;
                                    let dv = hash3(x, k, z, 12 + j * 3) - 0.5;
                                    let lean = u * (du * 0.9) + v * (dv * 0.9);
                                    nuggets.push(Nugget {
                                        at: center + u * (du * 0.45) + v * (dv * 0.45),
                                        normal: n,
                                        color: o.color,
                                        glow: o.glow,
                                        size: 0.1 + 0.12 * hash3(x, k, z, 13 + j * 3),
                                        chest: false,
                                        axis: lean,
                                    });
                                }
                            }
                            Some(Find::Ore(i)) => {
                                // a metal: its block takes its colour, studs of it set in the face
                                let o = &pozo::ORES[i];
                                for j in 0..9 {
                                    let du = hash3(x, k, z, 11 + j * 3) - 0.5;
                                    let dv = hash3(x, k, z, 12 + j * 3) - 0.5;
                                    nuggets.push(Nugget {
                                        at: center + u * (du * 0.8) + v * (dv * 0.8),
                                        normal: n,
                                        color: o.color,
                                        glow: o.glow,
                                        size: 0.05 + 0.08 * hash3(x, k, z, 13 + j * 3),
                                        chest: false,
                                        axis: Vec3::ZERO,
                                    });
                                }
                            }
                            Some(Find::Geode) => nuggets.push(Nugget {
                                at: center,
                                normal: n,
                                color: [0.72, 0.42, 1.0],
                                glow: 1.0,
                                size: 0.2,
                                chest: false,
                                axis: Vec3::ZERO,
                            }),
                            Some(Find::Fossil) => nuggets.push(Nugget {
                                at: center,
                                normal: n,
                                color: [0.86, 0.8, 0.66],
                                glow: 0.0,
                                size: 0.16,
                                chest: false,
                                axis: Vec3::ZERO,
                            }),
                            Some(Find::Chest) => nuggets.push(Nugget {
                                at: center,
                                normal: n,
                                color: [0.5, 0.32, 0.16],
                                glow: 0.0,
                                size: 0.3,
                                chest: true,
                                axis: Vec3::ZERO,
                            }),
                            _ => {}
                        }
                    }
                }
            }
        }
        for (texture, mesh) in meshes.into_iter().enumerate() {
            let key = (band, texture);
            if mesh.indices.is_empty() {
                if let Some(old) = self.bands.remove(&key) {
                    self.spare.push(old);
                }
                continue;
            }
            match self.bands.get(&key) {
                Some(&handle) => {
                    let _ = renderer.replace_mesh(device, handle, &mesh);
                }
                None => {
                    let handle = match self.spare.pop() {
                        Some(h) => renderer.replace_mesh(device, h, &mesh).ok().map(|_| h),
                        None => renderer.upload_mesh(device, &mesh).ok(),
                    };
                    if let Some(handle) = handle {
                        self.bands.insert(key, handle);
                    }
                }
            }
        }
        self.nuggets.insert(band, nuggets);
        self.built.insert(band, pozo.version);
    }

    /// Brings the meshes up to the shaft as it is: every band down to what she can see, and the
    /// bands round the block she last worked rebuilt.
    fn update(&mut self, pozo: &Pozo, renderer: &mut Wgpu3DRenderer, device: &wgpu::Device) {
        let last = pozo.depth_shown().div_euclid(BAND);
        for band in 0..=last {
            if !self.built.contains_key(&band) {
                self.build_band(pozo, band, renderer, device);
            }
        }
        if self.version != pozo.version {
            self.version = pozo.version;
            if let Some(b) = pozo.touched {
                let around: BTreeSet<i32> = [b[1] - 1, b[1], b[1] + 1]
                    .into_iter()
                    .filter(|k| *k >= 0)
                    .map(|k| k.div_euclid(BAND))
                    .collect();
                for band in around {
                    self.build_band(pozo, band, renderer, device);
                }
            }
        }
    }
}

/// A box from its middle along three axes (each scaled by its size).
fn oriented(center: Vec3, x: Vec3, y: Vec3, size: Vec3) -> Mat4 {
    let z = x.cross(y).normalize_or_zero();
    let (x, y, z) = (x * size.x, y * size.y, z * size.z);
    Mat4::from_cols(
        [x.x, x.y, x.z, 0.0],
        [y.x, y.y, y.z, 0.0],
        [z.x, z.y, z.z, 0.0],
        [center.x, center.y, center.z, 1.0],
    )
}

impl Scene {
    /// The head-frame's model, loaded the first time it shows.
    pub(super) fn ensure_headframe(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if !self.headframe_tried {
            self.headframe_tried = true;
            self.headframe = Headframe::load(renderer, device, queue);
        }
    }

    fn push_box(&self, draws: &mut Vec<MeshDraw3D>, transform: Mat4, color: [f32; 3], glow: f32) {
        let mut material = Material3D::rgb(color[0], color[1], color[2]);
        if glow > 0.0 {
            material = material.with_emission(glow);
        }
        draws.push(MeshDraw3D {
            mesh: self.cube,
            transform,
            material,
            texture: None,
        });
    }

    /// A beam of wood between two points.
    fn beam(&self, draws: &mut Vec<MeshDraw3D>, a: Vec3, b: Vec3, thick: f32, color: [f32; 3]) {
        let d = b - a;
        let y = d.normalize_or_zero();
        let side = if y.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
        let x = y.cross(side).normalize_or_zero();
        self.push_box(
            draws,
            oriented((a + b) * 0.5, x, y, Vec3::new(thick, d.length(), thick)),
            color,
            0.0,
        );
    }

    /// A timber between two points, its planks' texture on it (`shade` darkens it).
    fn timber(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        wood: Option<TextureHandle>,
        a: Vec3,
        b: Vec3,
        thick: f32,
        shade: f32,
    ) {
        let d = b - a;
        let y = d.normalize_or_zero();
        let side = if y.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
        let x = y.cross(side).normalize_or_zero();
        let lift = 1.35 * shade;
        draws.push(MeshDraw3D {
            mesh: self.cube,
            transform: oriented((a + b) * 0.5, x, y, Vec3::new(thick, d.length(), thick)),
            material: Material3D::rgb(lift, lift * 0.94, lift * 0.88).with_ambient_occlusion(0.2),
            texture: wood,
        });
    }

    /// A lantern hanging at `at`: its iron cage and lit glass, and its light.
    fn lantern(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        at: Vec3,
        flicker: f32,
        reach: f32,
    ) {
        let iron = [0.16, 0.15, 0.15];
        self.push_box(
            draws,
            oriented(
                at + Vec3::Y * 0.15,
                Vec3::X,
                Vec3::Y,
                Vec3::new(0.22, 0.05, 0.22),
            ),
            iron,
            0.0,
        );
        self.push_box(
            draws,
            oriented(
                at - Vec3::Y * 0.15,
                Vec3::X,
                Vec3::Y,
                Vec3::new(0.24, 0.05, 0.24),
            ),
            iron,
            0.0,
        );
        self.push_box(
            draws,
            oriented(at, Vec3::X, Vec3::Y, Vec3::new(0.15, 0.26, 0.15)),
            [1.0, 0.78, 0.42],
            3.2,
        );
        lights.push(PointLight3D::new(
            at,
            [1.0, 0.66, 0.32],
            5.5 * flicker,
            reach,
        ));
    }

    /// The head-frame (tools/castillete_blender.py). In the mine all of it at `foot` facing `yaw`;
    /// down in the shaft's room (`below`) only its cage, come down its guides from the ceiling,
    /// and the cart and stones beside it. The wheel and the winch's drum turn by `turn`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_headframe(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        foot: Vec3,
        yaw: f32,
        elapsed: f32,
        below: bool,
        turn: f32,
    ) {
        let Some(frame) = self.headframe.as_ref() else {
            return;
        };
        let root = Mat4::from_translation(foot) * Mat4::from_rotation_y(yaw);
        for part in &frame.parts {
            let shown = match part.piece {
                Piece::Cage | Piece::Yard => true,
                _ => !below,
            };
            if !shown {
                continue;
            }
            let local = match part.piece {
                Piece::Wheel => part.base * Mat4::from_rotation_x(turn),
                // the drum winds faster than the big wheel turns
                Piece::Drum => part.base * Mat4::from_rotation_x(-turn * 2.4),
                _ => part.base,
            };
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: root * local,
                material: part.material,
                texture: part.texture,
            });
        }
        let rope = [0.72, 0.58, 0.36];
        let flicker = |k: f32| 0.88 + 0.12 * (elapsed * 8.0 + k).sin();
        if below {
            // the cage's guides and rope go up into the ceiling, through a collar of timber
            let top = ROOM_HEIGHT;
            for (x, z) in [(-0.8, -0.8), (0.8, -0.8), (-0.8, 0.8), (0.8, 0.8)] {
                let a = root.transform_point3(Vec3::new(x, 0.0, z));
                self.timber(draws, frame.wood, a, a + Vec3::Y * top, 0.16, 0.8);
            }
            for (a, b) in [
                ((-1.0, -1.0), (1.0, -1.0)),
                ((-1.0, 1.0), (1.0, 1.0)),
                ((-1.0, -1.0), (-1.0, 1.0)),
                ((1.0, -1.0), (1.0, 1.0)),
            ] {
                let pa = root.transform_point3(Vec3::new(a.0, top - 0.15, a.1));
                let pb = root.transform_point3(Vec3::new(b.0, top - 0.15, b.1));
                self.timber(draws, frame.wood, pa, pb, 0.3, 0.7);
            }
            let hook = root.transform_point3(Vec3::new(0.0, CAGE_TOP, 0.0));
            self.beam(
                draws,
                hook,
                Vec3::new(hook.x, foot.y + top, hook.z),
                0.035,
                rope,
            );
        } else {
            // the rope over the wheel's front and down to the cage
            let over = root.transform_point3(HUB + Vec3::new(0.0, 0.0, WHEEL_R));
            let hook = root.transform_point3(Vec3::new(0.0, CAGE_TOP, HUB.z + WHEEL_R));
            self.beam(draws, over, hook, 0.035, rope);
            for (k, l) in LANTERNS.iter().enumerate() {
                let at = root.transform_point3(*l);
                lights.push(PointLight3D::new(
                    at,
                    [1.0, 0.66, 0.32],
                    2.4 * flicker(k as f32),
                    9.0,
                ));
            }
        }
    }

    /// The pick in her right hand: raised, brought down and back with each blow.
    fn draw_pick(&self, draws: &mut Vec<MeshDraw3D>, run: &Dungeon, pozo: &Pozo, camera: Camera3D) {
        let pick = run.gear.pick();
        let t = 1.0 - (pozo.swing / 0.3).clamp(0.0, 1.0);
        let swinging = pozo.swing > 0.0;
        let lerp = |a: Vec3, b: Vec3, s: f32| a + (b - a) * (s * s * (3.0 - 2.0 * s));
        let rest = (Vec3::new(0.3, -0.38, -0.6), Vec3::new(-0.28, 0.42, -0.86));
        let raised = (Vec3::new(0.3, -0.22, -0.5), Vec3::new(-0.05, 0.92, 0.38));
        let struck = (Vec3::new(0.2, -0.42, -0.74), Vec3::new(-0.15, -0.38, -0.91));
        let (hand, blade) = if !swinging {
            let bob = (run.elapsed * 1.6).sin() * 0.006;
            (rest.0 + Vec3::new(0.0, bob, 0.0), rest.1)
        } else if t < 0.3 {
            let s = t / 0.3;
            (lerp(rest.0, raised.0, s), lerp(rest.1, raised.1, s))
        } else if t < 0.55 {
            let s = (t - 0.3) / 0.25;
            (lerp(raised.0, struck.0, s), lerp(raised.1, struck.1, s))
        } else {
            let s = (t - 0.55) / 0.45;
            (lerp(struck.0, rest.0, s), lerp(struck.1, rest.1, s))
        };
        let hand = camera.position + fp::camera_vector(camera, hand);
        let blade = fp::camera_vector(camera, blade).normalize_or_zero();
        let shoulder = camera.position + fp::camera_vector(camera, Vec3::new(0.2, -0.3, 0.06));
        draw_view_arm(
            draws,
            self.fist_right,
            self.sleeve_cuff,
            self.sleeve_upper,
            hand,
            blade,
            shoulder,
            fp::camera_vector(camera, Vec3::new(0.7, -1.0, 0.25)).normalize_or_zero(),
            true,
            0.0,
        );
        // the haft, its grip at the fist, and the head across its end
        let right = fp::camera_vector(camera, Vec3::X).normalize_or_zero();
        let across = blade.cross(right).normalize_or_zero();
        let a = hand - blade * 0.08;
        let b = hand + blade * 0.48;
        let (wood, iron) = self
            .headframe
            .as_ref()
            .map_or((None, None), |h| (h.wood, h.iron));
        let mut part =
            |transform: Mat4, color: [f32; 3], texture: Option<TextureHandle>, glow: f32| {
                let mut material = Material3D::rgb(color[0], color[1], color[2]);
                if glow > 0.0 {
                    material = material.with_emission(glow);
                }
                draws.push(MeshDraw3D {
                    mesh: self.cube,
                    transform,
                    material,
                    texture,
                });
            };
        // the haft, wood with a leather grip at the fist
        part(
            oriented(
                (a + b) * 0.5,
                right,
                blade,
                Vec3::new(0.036, (b - a).length(), 0.036),
            ),
            [1.1, 0.86, 0.66],
            wood,
            0.0,
        );
        part(
            oriented(
                hand + blade * 0.02,
                right,
                blade,
                Vec3::new(0.044, 0.14, 0.044),
            ),
            [0.26, 0.15, 0.09],
            None,
            0.0,
        );
        // the head: an iron eye round the haft's end, a long spike and a short blade
        let glow = if pick.tier >= 4 { 0.6 } else { 0.0 };
        let metal = [
            pick.color[0] * 1.5 + 0.1,
            pick.color[1] * 1.5 + 0.1,
            pick.color[2] * 1.5 + 0.1,
        ];
        part(
            oriented(b, right, blade, Vec3::new(0.06, 0.09, 0.06)),
            [0.5, 0.48, 0.46],
            iron,
            0.0,
        );
        for (along, length, thick, tip) in [(0.2, 0.3, 0.05, 0.1), (-0.13, 0.17, 0.055, 0.07)] {
            let mid = b + across * along;
            part(
                oriented(mid, right, across, Vec3::new(thick, length, thick * 1.1)),
                metal,
                iron,
                glow,
            );
            // and its point
            let end = mid + across * (length * 0.5 + tip * 0.45) * along.signum();
            part(
                oriented(
                    end,
                    right,
                    across,
                    Vec3::new(thick * 0.55, tip, thick * 0.6),
                ),
                metal,
                iron,
                glow,
            );
        }
    }

    /// The shaft and the room round its mouth, then the HUD.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_pozo(
        &mut self,
        run: &Dungeon,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
    ) {
        let Some(pozo) = run.pozo.as_deref() else {
            return;
        };
        if self
            .pozo_art
            .as_ref()
            .is_none_or(|art| art.seed != pozo.seed)
        {
            self.pozo_art = Some(PozoArt::new(pozo, renderer, device, queue));
        }
        let mut art = self.pozo_art.take().expect("just made");
        self.ensure_headframe(renderer, device, queue);
        art.update(pozo, renderer, device);
        let mut draws = Vec::new();
        let mut lights = Vec::new();
        for &(mesh, texture) in &art.room {
            draws.push(MeshDraw3D {
                mesh,
                transform: Mat4::IDENTITY,
                material: Material3D::rgb(1.0, 1.0, 1.0),
                texture: art.textures[texture],
            });
        }
        let depth = Pozo::depth_of(run.controller.position);
        // the bands she can see: from the mouth (she looks up its walls from the bottom) to well
        // below her
        let near = 0;
        let far = (depth as i32 + 30).div_euclid(BAND);
        for (&(band, texture), &mesh) in &art.bands {
            if band < near || band > far {
                continue;
            }
            draws.push(MeshDraw3D {
                mesh,
                transform: Mat4::IDENTITY,
                material: Material3D::rgb(1.0, 1.0, 1.0),
                texture: art.textures[texture],
            });
        }
        // ore, gems and finds in the open faces; the nearest glowing ones light the rock
        let eye = camera.position;
        let mut glows: Vec<(f32, Vec3, [f32; 3], f32)> = Vec::new();
        for band in near..=far {
            for n in art.nuggets.get(&band).map(Vec::as_slice).unwrap_or(&[]) {
                if (n.at - eye).length_squared() > 30.0 * 30.0 {
                    continue;
                }
                let side = if n.normal.y.abs() > 0.5 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                let x = n.normal.cross(side).normalize_or_zero();
                if n.chest {
                    let at = n.at + n.normal * 0.02;
                    self.push_box(
                        &mut draws,
                        oriented(at, x, side, Vec3::new(n.size, n.size * 0.7, 0.06)),
                        n.color,
                        0.0,
                    );
                    self.push_box(
                        &mut draws,
                        oriented(
                            at + n.normal * 0.02,
                            x,
                            side,
                            Vec3::new(n.size * 1.02, 0.04, 0.07),
                        ),
                        [1.0, 0.78, 0.3],
                        0.4,
                    );
                    continue;
                }
                let mut material = Material3D::rgb(n.color[0], n.color[1], n.color[2]);
                if n.glow > 0.0 {
                    material = material.with_emission(n.glow * 1.8);
                } else {
                    material = material.with_emission(0.12);
                }
                // a gem's crystals lean out of the rock; a metal's studs stand straight out
                let up = (n.normal + n.axis).normalize_or_zero();
                let side = up.cross(x).normalize_or_zero();
                let x = side.cross(up).normalize_or_zero();
                let gem = n.glow >= 0.5;
                draws.push(MeshDraw3D {
                    mesh: self.crystal,
                    transform: oriented(
                        n.at + up * n.size * if gem { 0.3 } else { 0.02 },
                        x,
                        up,
                        Vec3::new(n.size, n.size * if gem { 2.2 } else { 0.7 }, n.size),
                    ),
                    material,
                    texture: None,
                });
                if n.glow > 0.5 {
                    glows.push((
                        (n.at - eye).length(),
                        n.at + n.normal * 0.25,
                        n.color,
                        n.glow,
                    ));
                }
            }
        }
        glows.sort_by(|a, b| a.0.total_cmp(&b.0));
        // the nearest gems light the rock round them
        glows.dedup_by(|a, b| (a.1 - b.1).length_squared() < 0.36);
        for (_, at, color, glow) in glows.into_iter().take(8) {
            lights.push(PointLight3D::new(at, color, 1.4 * glow, 4.2));
        }
        let wood = art.textures[PLANKS];
        let w = WIDTH as f32;
        let (lo, hi) = (-LEDGE as f32, (WIDTH + LEDGE) as f32);
        let flicker = |k: f32| 0.88 + 0.12 * (run.elapsed * 7.0 + k).sin();
        // the rim of the shaft: timbers round its mouth
        for (a, b) in [
            (
                Vec3::new(-0.15, 0.08, -0.15),
                Vec3::new(w + 0.15, 0.08, -0.15),
            ),
            (
                Vec3::new(-0.15, 0.08, w + 0.15),
                Vec3::new(w + 0.15, 0.08, w + 0.15),
            ),
            (
                Vec3::new(-0.15, 0.08, -0.15),
                Vec3::new(-0.15, 0.08, w + 0.15),
            ),
            (
                Vec3::new(w + 0.15, 0.08, -0.15),
                Vec3::new(w + 0.15, 0.08, w + 0.15),
            ),
        ] {
            self.timber(&mut draws, wood, a, b, 0.28, 0.75);
        }
        // the room is timbered: posts along its walls every three metres, caps across its ceiling
        let mut posts = Vec::new();
        let mut z = lo + 1.5;
        while z < hi {
            posts.push((Vec3::new(lo + 0.2, 0.0, z), Vec3::new(hi - 0.2, 0.0, z)));
            z += 3.0;
        }
        for (k, (a, b)) in posts.iter().enumerate() {
            for p in [*a, *b] {
                self.timber(&mut draws, wood, p, p + Vec3::Y * ROOM_HEIGHT, 0.32, 0.8);
            }
            let cap = Vec3::Y * (ROOM_HEIGHT - 0.2);
            self.timber(&mut draws, wood, *a + cap, *b + cap, 0.34, 0.65);
            // braces from each post up to its cap
            for (p, d) in [(*a, 1.0f32), (*b, -1.0)] {
                self.timber(
                    &mut draws,
                    wood,
                    p + Vec3::new(0.0, ROOM_HEIGHT - 1.4, 0.0),
                    p + Vec3::new(d * 1.2, ROOM_HEIGHT - 0.25, 0.0),
                    0.16,
                    0.7,
                );
            }
            // a lantern on every other post
            if k % 2 == 0 {
                for (p, d) in [(*a, 1.0f32), (*b, -1.0)] {
                    self.lantern(
                        &mut draws,
                        &mut lights,
                        p + Vec3::new(d * 0.32, 2.6, 0.0),
                        flicker(k as f32 + d),
                        13.0,
                    );
                }
            }
        }
        // two lamps hung from the caps over the shaft's mouth
        for (k, x) in [(0usize, w * 0.3), (1, w * 0.7)] {
            let z = posts
                .iter()
                .map(|(a, _)| a.z)
                .min_by(|a, b| (a - w * 0.5).abs().total_cmp(&(b - w * 0.5).abs()))
                .unwrap_or(w * 0.5);
            let hook = Vec3::new(x, ROOM_HEIGHT - 0.35, z);
            self.beam(
                &mut draws,
                hook,
                hook - Vec3::Y * 1.2,
                0.02,
                [0.2, 0.19, 0.18],
            );
            self.lantern(
                &mut draws,
                &mut lights,
                hook - Vec3::Y * 1.45,
                flicker(9.0 + k as f32),
                16.0,
            );
        }
        // rings of timber down the shaft every ten metres, in its walls (the rock hides them
        // until she digs to them), a lamp at their corners
        let deepest = pozo.deepest.max(0);
        let mut ring = 5;
        while ring <= pozo.depth_shown() {
            let y = -(ring as f32) + 0.18;
            if (ring - depth as i32).abs() < 40 {
                for (a, b) in [
                    (Vec3::new(0.0, y, 0.0), Vec3::new(w, y, 0.0)),
                    (Vec3::new(0.0, y, w), Vec3::new(w, y, w)),
                    (Vec3::new(0.0, y, 0.0), Vec3::new(0.0, y, w)),
                    (Vec3::new(w, y, 0.0), Vec3::new(w, y, w)),
                ] {
                    self.timber(&mut draws, wood, a, b, 0.36, 0.7);
                }
                if ring <= deepest + 1 {
                    for (x, z) in [
                        (0.35, 0.35),
                        (w - 0.35, 0.35),
                        (0.35, w - 0.35),
                        (w - 0.35, w - 0.35),
                    ] {
                        let k = ring.min(deepest);
                        let bx = (x as i32).clamp(0, WIDTH - 1);
                        let bz = (z as i32).clamp(0, WIDTH - 1);
                        if pozo.open(bx, k, bz) {
                            let at = Vec3::new(x, y - 0.32, z);
                            self.push_box(
                                &mut draws,
                                oriented(at, Vec3::X, Vec3::Y, Vec3::new(0.14, 0.22, 0.14)),
                                [1.0, 0.76, 0.4],
                                3.0,
                            );
                            lights.push(PointLight3D::new(
                                at,
                                [1.0, 0.66, 0.34],
                                2.8 * flicker(ring as f32 + x),
                                10.0,
                            ));
                        }
                    }
                }
            }
            ring += 5;
        }
        // the miner's ladder down the west wall, as deep as she has dug
        let (top, bottom) = (
            (-(depth - 26.0)).min(0.0),
            -((deepest + 1) as f32).min(depth + 26.0),
        );
        if bottom < top {
            for z in [4.62, 5.38] {
                self.timber(
                    &mut draws,
                    wood,
                    Vec3::new(0.05, top + 0.9, z),
                    Vec3::new(0.05, bottom, z),
                    0.07,
                    0.7,
                );
            }
            let mut y = top - 0.25;
            while y > bottom {
                self.timber(
                    &mut draws,
                    wood,
                    Vec3::new(0.06, y, 4.62),
                    Vec3::new(0.06, y, 5.38),
                    0.05,
                    0.7,
                );
                y -= 0.45;
            }
        }
        let (arrival, yaw) = Pozo::arrival();
        // the cage stands where she arrives, its open side toward the shaft
        let turn = if pozo.hauling.is_some() {
            run.elapsed * 4.0
        } else {
            0.0
        };
        self.draw_headframe(
            &mut draws,
            &mut lights,
            arrival,
            yaw + std::f32::consts::PI,
            run.elapsed,
            true,
            turn,
        );
        // her helmet lamp: the shaft is dark below the lamps of its mouth
        let lamp =
            eye + Vec3::Y * 0.12 + (camera.target - camera.position).normalize_or_zero() * 0.05;
        lights.push(PointLight3D::new(
            lamp,
            [1.0, 0.88, 0.7],
            if run.torch { 5.0 } else { 3.8 },
            if run.torch { 17.0 } else { 13.0 },
        ));
        // the block she aims at: its edges lit
        if let Some(b) = pozo.target {
            let lo = Vec3::new(b[0] as f32, -(b[1] as f32) - 1.0, b[2] as f32);
            let mark = if pozo.diggable(b) {
                [1.0, 0.95, 0.8]
            } else {
                [0.8, 0.35, 0.3]
            };
            for (a, d) in [
                (Vec3::ZERO, Vec3::X),
                (Vec3::Z, Vec3::X),
                (Vec3::Y, Vec3::X),
                (Vec3::Y + Vec3::Z, Vec3::X),
                (Vec3::ZERO, Vec3::Z),
                (Vec3::X, Vec3::Z),
                (Vec3::Y, Vec3::Z),
                (Vec3::Y + Vec3::X, Vec3::Z),
                (Vec3::ZERO, Vec3::Y),
                (Vec3::X, Vec3::Y),
                (Vec3::Z, Vec3::Y),
                (Vec3::X + Vec3::Z, Vec3::Y),
            ] {
                let start = lo + a * 1.01 - Vec3::new(0.005, 0.005, 0.005);
                self.push_box(
                    &mut draws,
                    oriented(
                        start + d * 0.505,
                        if d == Vec3::Y { Vec3::X } else { Vec3::Y },
                        d,
                        Vec3::new(0.018, 1.02, 0.018),
                    ),
                    mark,
                    0.9,
                );
            }
        }
        self.draw_pick(&mut draws, run, pozo, camera);
        let fx_meshes = super::super::combat_fx::FxMeshes {
            cube: self.cube,
            crystal: self.crystal,
            ring: self.fx_ring,
            fissure: self.spell_fissure,
            orb: self.fx_orb,
        };
        super::super::combat_fx::draw(&run.fx, &fx_meshes, &mut draws, &mut lights);
        lights.sort_by(|a, b| {
            (a.position - eye)
                .length_squared()
                .total_cmp(&(b.position - eye).length_squared())
        });
        lights.truncate(16);
        // deeper, the air darkens and takes the rock's colour
        let rock = Rock::at(depth as i32, 0);
        let tint = rock.color();
        let deep = (depth / 120.0).clamp(0.0, 1.0);
        let fog = [
            0.03 + tint[0] * 0.02 * deep,
            0.025 + tint[1] * 0.015 * deep,
            0.022 + tint[2] * 0.02 * deep,
        ];
        let mut environment = EnvironmentFrame3D::default();
        environment.clear_color = [fog[0], fog[1], fog[2], 1.0];
        environment.celestial = Celestial3D::default();
        // the room round the mouth is lamp-lit; the shaft darkens as it goes down
        let ambient = 0.12 - 0.03 * (depth / 40.0).clamp(0.0, 1.0)
            + 0.05 * (1.0 - depth / 6.0).clamp(0.0, 1.0);
        environment.lighting = Lighting3D {
            direction_to_light: Vec3::new(-0.3, 0.8, 0.2).normalize_or_zero(),
            key_color: [ambient * 0.7, ambient * 0.64, ambient * 0.58],
            ambient_color: [ambient, ambient * 0.95, ambient * 0.9],
            bounce_color: [0.05, 0.04, 0.03],
            shadow_extent: 20.0,
        };
        environment.atmosphere = Atmosphere3D {
            fog_color: fog,
            fog_start: 14.0,
            fog_end: 55.0,
            fog_density: 0.02,
            height_falloff: 0.0,
        };
        self.stats.draws = draws.len();
        let previous_toon = renderer.replace_toon_style(ToonStyle3D {
            softness: 0.12,
            specular: 0.22,
            normal_detail: 0.45,
            shadow_tint: [0.58, 0.66, 0.92],
            rim: 0.03,
        });
        let previous_shadows = renderer.replace_directional_shadows(false);
        renderer.render_with_environment_and_lights(
            device,
            queue,
            &self.finish.scene,
            size,
            camera,
            environment,
            &lights,
            &draws,
        );
        renderer.replace_toon_style(previous_toon);
        renderer.replace_directional_shadows(previous_shadows);
        self.finish.render(device, queue, target);
        self.hud
            .render(run, self.stats, device, queue, target, size);
        self.pozo_art = Some(art);
    }
}
