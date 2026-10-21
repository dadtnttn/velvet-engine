//! The miner's shaft drawn: its blocks, built into a mesh per texture and band of depth (rebuilt
//! where she digs), the ores and gems showing in the rock, the room round its mouth with the winch
//! and its lamps, the pick in her hand and the block she aims at. Also the winch's head-frame by the
//! Tendero's stall in the mine, where she goes down.
use super::*;
use crate::pozo::{self, Find, Pozo, Rock, LEDGE, ROOM_HEIGHT, WIDTH};
use std::collections::HashMap;

/// Depth bands of the shaft's meshes.
const BAND: i32 = 6;

/// One thing showing in an open face of rock: an ore's nugget, a gem, a chest's corner.
#[derive(Clone, Copy)]
struct Nugget {
    at: Vec3,
    normal: Vec3,
    color: [f32; 3],
    glow: f32,
    size: f32,
    chest: bool,
}

/// What the shaft keeps on the GPU.
pub(super) struct PozoArt {
    seed: u64,
    /// earth, stone, dark rock, planks
    textures: [Option<TextureHandle>; 4],
    bands: HashMap<(i32, usize), MeshHandle>,
    nuggets: HashMap<i32, Vec<Nugget>>,
    built: HashMap<i32, u64>,
    room: Vec<(MeshHandle, usize)>,
    spare: Vec<MeshHandle>,
    version: u64,
}

fn load_texture(
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
        let textures = [
            load_texture(renderer, device, queue, "brown_mud_rocks_01"),
            load_texture(renderer, device, queue, "rock_face_04"),
            load_texture(renderer, device, queue, "dark_rock_02"),
            load_texture(renderer, device, queue, "old_planks_02"),
        ];
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
        let dark = [1.3, 1.26, 1.2];
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
        for (mesh, texture) in [(stone, 1usize), (planks, 3), (walls, 2)] {
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
        let mut meshes = [empty_mesh(), empty_mesh(), empty_mesh()];
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
                    let base = block.rock.color();
                    let shade = (0.84 + 0.2 * hash3(x, k, z, 7))
                        * if inside { 1.0 } else { 0.62 }
                        * (1.0 - 0.5 * wear);
                    let color = [base[0] * shade, base[1] * shade, base[2] * shade];
                    let texture = if inside { block.rock.texture() } else { 2 };
                    // the scanned textures are dark (the mine lifts them too): the darkest most
                    let lift = [1.25, 1.35, 2.3][texture];
                    let color = [color[0] * lift, color[1] * lift, color[2] * lift];
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
                        quad(&mut meshes[texture], corner, u, v, n, color);
                        if !inside {
                            continue;
                        }
                        let center = corner + (u + v) * 0.5;
                        match block.find {
                            Some(Find::Ore(i)) => {
                                let o = &pozo::ORES[i];
                                for j in 0..4 {
                                    let du = hash3(x, k, z, 11 + j * 3) - 0.5;
                                    let dv = hash3(x, k, z, 12 + j * 3) - 0.5;
                                    nuggets.push(Nugget {
                                        at: center + u * (du * 0.75) + v * (dv * 0.75),
                                        normal: n,
                                        color: o.color,
                                        glow: o.glow,
                                        size: 0.07 + 0.06 * hash3(x, k, z, 13 + j * 3),
                                        chest: false,
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
                            }),
                            Some(Find::Fossil) => nuggets.push(Nugget {
                                at: center,
                                normal: n,
                                color: [0.86, 0.8, 0.66],
                                glow: 0.0,
                                size: 0.16,
                                chest: false,
                            }),
                            Some(Find::Chest) => nuggets.push(Nugget {
                                at: center,
                                normal: n,
                                color: [0.5, 0.32, 0.16],
                                glow: 0.0,
                                size: 0.3,
                                chest: true,
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

    /// The winch's head-frame over a shaft's cage: four posts, the beams, the wheel, the rope and
    /// the cage; `foot` its middle on the ground, `yaw` the way it faces.
    pub(super) fn draw_headframe(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        foot: Vec3,
        yaw: f32,
        elapsed: f32,
        cage: bool,
    ) {
        let wood = [0.42, 0.27, 0.14];
        let dark = [0.26, 0.17, 0.09];
        let (s, c) = yaw.sin_cos();
        let at = |x: f32, y: f32, z: f32| foot + Vec3::new(x * c + z * s, y, -x * s + z * c);
        let top = 4.4;
        for (x, z) in [(-1.3, -1.1), (1.3, -1.1), (-1.3, 1.1), (1.3, 1.1)] {
            self.beam(draws, at(x, 0.0, z), at(x * 0.82, top, z * 0.82), 0.2, wood);
        }
        for (a, b) in [
            ((-1.07, -0.9), (1.07, -0.9)),
            ((-1.07, 0.9), (1.07, 0.9)),
            ((-1.07, -0.9), (-1.07, 0.9)),
            ((1.07, -0.9), (1.07, 0.9)),
        ] {
            self.beam(draws, at(a.0, top, a.1), at(b.0, top, b.1), 0.22, dark);
            self.beam(
                draws,
                at(a.0 * 1.2, 1.9, a.1 * 1.2),
                at(b.0 * 1.2, 1.9, b.1 * 1.2),
                0.14,
                wood,
            );
        }
        // the wheel, turning while the cage runs
        let hub = at(0.0, top + 0.55, 0.0);
        let turn = elapsed * 0.6;
        for k in 0..8 {
            let a = turn + k as f32 * std::f32::consts::FRAC_PI_4;
            let spoke = Vec3::new(a.cos() * c, a.sin(), -a.cos() * s) * 0.6;
            self.beam(draws, hub, hub + spoke, 0.06, [0.3, 0.3, 0.32]);
            let next = turn + (k + 1) as f32 * std::f32::consts::FRAC_PI_4;
            let rim = Vec3::new(next.cos() * c, next.sin(), -next.cos() * s) * 0.6;
            self.beam(draws, hub + spoke, hub + rim, 0.08, [0.36, 0.36, 0.38]);
        }
        self.beam(draws, at(-0.9, top, 0.0), at(0.9, top, 0.0), 0.16, dark);
        self.beam(
            draws,
            hub + Vec3::Y * -0.6,
            at(0.0, 2.3, 0.0),
            0.03,
            [0.62, 0.52, 0.34],
        );
        if cage {
            let iron = [0.3, 0.3, 0.32];
            for (x, z) in [(-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
                self.beam(draws, at(x, 0.0, z), at(x, 2.3, z), 0.06, iron);
            }
            for y in [0.05, 1.1, 2.3] {
                for (a, b) in [
                    ((-0.7, -0.7), (0.7, -0.7)),
                    ((-0.7, 0.7), (0.7, 0.7)),
                    ((-0.7, -0.7), (-0.7, 0.7)),
                ] {
                    self.beam(draws, at(a.0, y, a.1), at(b.0, y, b.1), 0.05, iron);
                }
            }
        }
        // a lantern on a post
        let lamp = at(1.3, 2.6, -1.25);
        self.push_box(
            draws,
            oriented(lamp, Vec3::X, Vec3::Y, Vec3::new(0.14, 0.2, 0.14)),
            [1.0, 0.78, 0.4],
            3.5,
        );
        let flicker = 0.9 + 0.1 * (elapsed * 9.0).sin();
        lights.push(PointLight3D::new(
            lamp,
            [1.0, 0.66, 0.32],
            2.6 * flicker,
            9.0,
        ));
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
        self.push_box(
            draws,
            oriented(
                (a + b) * 0.5,
                right,
                blade,
                Vec3::new(0.034, (b - a).length(), 0.034),
            ),
            [0.45, 0.3, 0.16],
            0.0,
        );
        let glow = if pick.tier >= 4 { 0.6 } else { 0.0 };
        self.push_box(
            draws,
            oriented(
                b + across * 0.04,
                right,
                across,
                Vec3::new(0.045, 0.38, 0.05),
            ),
            pick.color,
            glow,
        );
        self.push_box(
            draws,
            oriented(
                b + across * 0.23,
                right,
                across,
                Vec3::new(0.026, 0.09, 0.03),
            ),
            pick.color,
            glow,
        );
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
        // the bands she can see: from the mouth (looking down from the top) to well below her
        let near = (depth as i32 - 18).max(0).div_euclid(BAND);
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
                    material = material.with_emission(n.glow * 1.4);
                }
                draws.push(MeshDraw3D {
                    mesh: self.crystal,
                    transform: oriented(n.at, x, n.normal, Vec3::new(n.size, n.size * 1.5, n.size)),
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
        for (_, at, color, glow) in glows.into_iter().take(5) {
            lights.push(PointLight3D::new(at, color, 0.9 * glow, 3.2));
        }
        // the rim of the shaft: timbers round the mouth; lamps at its corners
        let rim = [0.4, 0.26, 0.13];
        let w = WIDTH as f32;
        for (a, b) in [
            (Vec3::new(-0.1, 0.06, -0.1), Vec3::new(w + 0.1, 0.06, -0.1)),
            (
                Vec3::new(-0.1, 0.06, w + 0.1),
                Vec3::new(w + 0.1, 0.06, w + 0.1),
            ),
            (Vec3::new(-0.1, 0.06, -0.1), Vec3::new(-0.1, 0.06, w + 0.1)),
            (
                Vec3::new(w + 0.1, 0.06, -0.1),
                Vec3::new(w + 0.1, 0.06, w + 0.1),
            ),
        ] {
            self.beam(&mut draws, a, b, 0.2, rim);
        }
        let flicker = |k: f32| 0.88 + 0.12 * (run.elapsed * 7.0 + k).sin();
        for (k, (x, z)) in [
            (-1.4, -1.4),
            (w + 1.4, -1.4),
            (-1.4, w + 1.4),
            (w + 1.4, w + 1.4),
        ]
        .into_iter()
        .enumerate()
        {
            let foot = Vec3::new(x, 0.0, z);
            self.beam(&mut draws, foot, foot + Vec3::Y * 2.4, 0.16, rim);
            let lamp = foot + Vec3::Y * 2.5;
            self.push_box(
                &mut draws,
                oriented(lamp, Vec3::X, Vec3::Y, Vec3::new(0.16, 0.22, 0.16)),
                [1.0, 0.76, 0.4],
                3.2,
            );
            lights.push(PointLight3D::new(
                lamp,
                [1.0, 0.64, 0.3],
                2.4 * flicker(k as f32),
                12.0,
            ));
        }
        let (arrival, yaw) = Pozo::arrival();
        self.draw_headframe(&mut draws, &mut lights, arrival, yaw, run.elapsed, true);
        // her helmet lamp: the shaft is dark below the lamps of its mouth
        let lamp =
            eye + Vec3::Y * 0.12 + (camera.target - camera.position).normalize_or_zero() * 0.05;
        lights.push(PointLight3D::new(
            lamp,
            [1.0, 0.86, 0.66],
            if run.torch { 4.0 } else { 3.0 },
            if run.torch { 15.0 } else { 11.0 },
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
        let ambient = 0.085 - 0.03 * (depth / 40.0).clamp(0.0, 1.0);
        environment.lighting = Lighting3D {
            direction_to_light: Vec3::new(-0.3, 0.8, 0.2).normalize_or_zero(),
            key_color: [ambient * 0.5, ambient * 0.46, ambient * 0.42],
            ambient_color: [ambient, ambient * 0.94, ambient * 0.88],
            bounce_color: [0.01, 0.008, 0.006],
            shadow_extent: 20.0,
        };
        environment.atmosphere = Atmosphere3D {
            fog_color: fog,
            fog_start: 8.0,
            fog_end: 40.0,
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
