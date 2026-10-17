//! The court's life in the castle. What Lira's works restore and what the throne decides shows in
//! its rooms: quilts on the infirmary's cots and its medicines, the armoury's forge lit and its
//! smith, the day's food on the kitchen's table, a guard at the gate and in the barracks, the
//! library's fallen roof (until it is cleared); in the yard Bertila and her kids, the refugees'
//! camp, the smith's new swords or her ploughs, the wedding's garlands, the feast's lanterns and
//! the stone with the capital's dead. The flags come from the story ([`life`]); the castle keeps
//! them in its state (what stands in her way, the kitchen's fire, the library's books), and what
//! stands where is in [`castle3d`]'s consts.
use std::f32::consts::{PI, TAU};

use super::*;
use castle3d::Life;
use demon_queen_war as wr;
use velvet_render::{crowd3d::part, CrowdInstance3D, CrowdMesh3D, ToonFire3D};
use wr::data::{rgb, Faction};
use wr::models::{self, Piece};
use wr::prims::{self, Builder, Shape};

/// What the story says the court has done (see [`Life`]). `aud_<id>` is the answer given to a
/// petition (1, 2, 3…), the works are `obra_<room>`.
pub(crate) fn life(v: &guion::Vars) -> Life {
    let aud = |id: &str| v.get(&format!("aud_{id}"));
    let goat = aud("cabra_grubbo");
    let smith = aud("herrera_armaduras");
    Life {
        infirmary: v.get("obra_enfermeria") > 0,
        armory: v.get("obra_armeria") > 0,
        kitchen: v.get("obra_cocina") > 0,
        barracks: v.get("obra_barracones") > 0,
        library: v.get("obra_biblioteca") > 0,
        goat: match goat {
            0 => 0,
            3 => 2,
            _ => 1,
        },
        kids: match (goat, aud("cabra_trono")) {
            (0, _) | (_, 0) => 0,
            (_, 3) => 2,
            _ => 1,
        },
        refugees: match aud("refugiados_norte") {
            1 => 1,
            2 => 2,
            _ => 0,
        },
        swords: matches!(smith, 2 | 3),
        ploughs: matches!(smith, 1 | 3),
        wedding: matches!(aud("boda_orca"), 1 | 2),
        feast: aud("dulces_villa") == 2,
        memorial: aud("lapida_capital") == 1,
    }
}

// ------------------------------------------------------------------ what is built

/// The pieces of set dressing, each shown by a flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    InfirmaryBefore,
    InfirmaryAfter,
    ForgeCold,
    ForgeLit,
    Kitchen,
    LibraryRuin,
    Camp,
    CampWork,
    Swords,
    Ploughs,
    Wedding,
    Feast,
    Memorial,
}

impl Which {
    const ALL: [Which; 13] = [
        Which::InfirmaryBefore,
        Which::InfirmaryAfter,
        Which::ForgeCold,
        Which::ForgeLit,
        Which::Kitchen,
        Which::LibraryRuin,
        Which::Camp,
        Which::CampWork,
        Which::Swords,
        Which::Ploughs,
        Which::Wedding,
        Which::Feast,
        Which::Memorial,
    ];

    fn shown(self, l: &Life) -> bool {
        match self {
            Which::InfirmaryBefore => !l.infirmary,
            Which::InfirmaryAfter => l.infirmary,
            Which::ForgeCold => !l.armory,
            Which::ForgeLit => l.armory,
            Which::Kitchen => l.kitchen,
            Which::LibraryRuin => !l.library,
            Which::Camp => l.refugees > 0,
            Which::CampWork => l.refugees == 2,
            Which::Swords => l.swords,
            Which::Ploughs => l.ploughs,
            Which::Wedding => l.wedding,
            Which::Feast => l.feast,
            Which::Memorial => l.memorial,
        }
    }

    /// Where it is: the upper floor (1), a room of the ground floor (0, hidden from upstairs) or
    /// the yard (open to the sky: seen from both).
    fn floor(self) -> Floor {
        match self {
            Which::InfirmaryBefore | Which::InfirmaryAfter | Which::ForgeCold | Which::ForgeLit => {
                Floor::Upper
            }
            Which::Kitchen | Which::LibraryRuin => Floor::Ground,
            _ => Floor::Yard,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Floor {
    Ground,
    Upper,
    Yard,
}

impl Floor {
    fn in_view(self, view: u8) -> bool {
        match self {
            Floor::Ground => view == 0,
            Floor::Upper => view == 1,
            Floor::Yard => true,
        }
    }
}

/// Plain shapes (vertex colours) and glowing ones (embers, lanterns, potions).
#[derive(Default)]
struct Kit {
    solid: Builder,
    glow: Builder,
}

impl Kit {
    fn add(&mut self, shape: Shape, hex: u32) {
        self.solid.add(&shape, rgb(hex));
    }

    fn shine(&mut self, shape: Shape, hex: u32) {
        self.glow.add(&shape, rgb(hex));
    }

    /// A rod of radius r from p to q.
    fn rod(&mut self, p: Vec3, q: Vec3, r: f32, hex: u32) {
        self.add(rod(p, q, r), hex);
    }
}

fn rod(p: Vec3, q: Vec3, r: f32) -> Shape {
    let d = q - p;
    let len = d.length().max(1e-4);
    let tilt = (d.y / len).clamp(-1.0, 1.0).acos();
    let yaw = d.x.atan2(d.z);
    let mid = (p + q) * 0.5;
    prims::cylinder(r, r, len, 5)
        .rotate_x(tilt)
        .rotate_y(yaw)
        .translate(mid.x, mid.y, mid.z)
}

/// A rope hanging from a to b, `sag` lower in the middle: its points.
fn hanging(a: Vec3, b: Vec3, sag: f32, n: usize) -> Vec<Vec3> {
    (0..=n)
        .map(|i| {
            let u = i as f32 / n as f32;
            a + (b - a) * u - Vec3::new(0.0, sag * 4.0 * u * (1.0 - u), 0.0)
        })
        .collect()
}

fn b(w: f32, h: f32, d: f32, x: f32, y: f32, z: f32) -> Shape {
    prims::boxed(w, h, d).translate(x, y + h / 2.0, z)
}

fn cyl(r: f32, h: f32, x: f32, y: f32, z: f32, seg: usize) -> Shape {
    prims::cylinder(r, r, h, seg).translate(x, y + h / 2.0, z)
}

fn build(which: Which) -> Kit {
    let up = castle3d::plan().level_h;
    let mut k = Kit::default();
    match which {
        Which::InfirmaryBefore => {
            // the cabinet without its door: what fell out of it, broken on the floor
            let mut seed = 7u32;
            for i in 0..9 {
                let a = i as f32 * 2.3;
                let r = 0.25 + (i % 3) as f32 * 0.22;
                let (x, z) = (-13.5 + a.cos() * r, 9.75 + a.sin().abs() * r * 0.8);
                let shard = prims::plate(&[[0.0, 0.0], [0.09, 0.02], [0.05, 0.08]], 0.012)
                    .rotate_x(-PI / 2.0)
                    .rotate_y(a)
                    .translate(x, up + 0.01, z);
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                k.add(
                    shard,
                    [0x6a8a5a, 0x9a6a3a, 0xb8c8c0][(seed >> 16) as usize % 3],
                );
            }
            k.add(
                prims::cylinder(0.05, 0.055, 0.14, 8)
                    .rotate_z(PI / 2.0)
                    .rotate_y(0.6)
                    .translate(-13.0, up + 0.05, 10.3),
                0x6a8a5a,
            );
            // the two cots that were not of use: their mattresses rolled up at the foot
            for z in [17.6, 20.6] {
                k.add(
                    prims::cylinder(0.16, 0.16, 0.86, 10)
                        .rotate_x(PI / 2.0)
                        .translate(-23.6, up + 0.62, z),
                    0xb8ae9e,
                );
            }
        }
        Which::InfirmaryAfter => {
            // a quilt on every cot (her colours), the foot's gold stripe, the sheet turned down
            for z in [11.6, 14.6, 17.6, 20.6] {
                k.add(b(1.42, 0.07, 1.0, -24.13, up + 0.6, z), 0x5b2b86);
                k.add(b(0.1, 0.075, 1.01, -23.52, up + 0.6, z), 0xd8a840);
                k.add(b(0.16, 0.08, 1.0, -24.86, up + 0.6, z), 0xf2eee6);
            }
            // the medicines on Nyxara's desk: red cures, bandages
            for (x, z) in [
                (-14.85, 14.85),
                (-14.55, 14.8),
                (-14.75, 16.55),
                (-14.45, 16.6),
            ] {
                k.shine(cyl(0.05, 0.12, x, up + 0.82, z, 8), 0xff2a3a);
                k.add(cyl(0.02, 0.06, x, up + 0.94, z, 6), 0xd8e0e8);
                k.add(cyl(0.024, 0.03, x, up + 1.0, z, 6), 0x8a6a44);
            }
            for z in [15.95, 16.15] {
                k.add(
                    prims::cylinder(0.05, 0.05, 0.1, 10)
                        .rotate_z(PI / 2.0)
                        .translate(-14.35, up + 0.87, z),
                    0xf4f0e8,
                );
            }
            // a basin of clean water by the beds
            k.add(cyl(0.26, 0.16, -21.4, up, 21.2, 12), 0x8a8a94);
            k.shine(cyl(0.23, 0.01, -21.4, up + 0.155, 21.2, 12), 0x5aa8d8);
        }
        Which::ForgeCold | Which::ForgeLit => {
            // the forge at the back, under the torch: a stone hearth with a low hood
            let (x, z) = castle3d::FORGE;
            k.add(b(1.5, 0.85, 0.8, x, up, z), 0x6a6470);
            k.add(b(1.6, 0.08, 0.9, x, up + 0.85, z), 0x4a4550);
            k.add(
                prims::cylinder(0.3, 0.72, 0.62, 4)
                    .rotate_y(PI / 4.0)
                    .scale(1.05, 1.0, 0.6)
                    .translate(x, up + 1.72, z - 0.1),
                0x56515c,
            );
            for s in [-1.0f32, 1.0] {
                k.add(
                    b(0.12, 0.55, 0.12, x + s * 0.68, up + 0.93, z + 0.32),
                    0x4a4550,
                );
            }
            if which == Which::ForgeLit {
                k.shine(b(1.0, 0.06, 0.48, x, up + 0.93, z + 0.02), 0xff5a1a);
            } else {
                k.add(b(1.0, 0.06, 0.48, x, up + 0.93, z + 0.02), 0x55504c);
                // a cobweb in the hood's corner
                k.add(
                    prims::plate(&[[0.0, 0.0], [0.3, 0.0], [0.0, 0.3]], 0.005).translate(
                        x - 0.7,
                        up + 1.0,
                        z + 0.36,
                    ),
                    0xc8c4cc,
                );
            }
            // the water bucket, the anvil on its stump
            k.add(cyl(0.2, 0.3, x + 1.15, up, z + 0.05, 10), 0x7a5634);
            k.add(cyl(0.17, 0.01, x + 1.15, up + 0.29, z + 0.05, 10), 0x2a4a6a);
            let (ax, az) = castle3d::ANVIL;
            k.add(
                prims::cylinder(0.24, 0.27, 0.5, 10).translate(ax, up + 0.25, az),
                0x6a4a2a,
            );
            k.add(b(0.3, 0.1, 0.16, ax, up + 0.5, az), 0x3e414a);
            k.add(b(0.58, 0.16, 0.24, ax, up + 0.6, az), 0x4a4d58);
            k.add(
                prims::cone(0.085, 0.26, 8)
                    .rotate_z(-PI / 2.0)
                    .translate(ax + 0.42, up + 0.69, az),
                0x4a4d58,
            );
            if which == Which::ForgeLit {
                // new blades cooling on the anvil's side, the hammer's rack
                for i in 0..3 {
                    k.add(
                        b(
                            0.85,
                            0.012,
                            0.06,
                            x - 0.1,
                            up + 0.89,
                            z + 0.38 + i as f32 * 0.002,
                        )
                        .translate(0.0, i as f32 * 0.014, 0.0),
                        0xc8ccd8,
                    );
                }
                k.rod(
                    Vec3::new(ax - 0.45, up + 0.02, az + 0.25),
                    Vec3::new(ax - 0.5, up + 0.75, az + 0.15),
                    0.03,
                    0x5a3d26,
                );
            }
        }
        Which::Kitchen => {
            // the day's food on the work table: loaves, a stew, the bundles for the road
            let top = 0.9;
            for (x, z, s) in [(23.35, 2.75, 1.0), (23.75, 3.0, 0.85), (23.45, 3.25, 0.9)] {
                k.add(
                    prims::sphere(0.16 * s, 10, 7)
                        .scale(1.0, 0.55, 1.5)
                        .translate(x, top + 0.07, z),
                    0xc08040,
                );
            }
            k.add(cyl(0.21, 0.22, 23.6, top, 4.0, 14), 0x34323a);
            k.shine(cyl(0.19, 0.01, 23.6, top + 0.21, 4.0, 14), 0xb8642a);
            k.add(b(0.46, 0.03, 0.3, 23.55, top, 4.55), 0x8a6a44);
            for (i, z) in [2.2f32, 2.45, 2.7].iter().enumerate() {
                k.add(
                    b(0.24, 0.14, 0.2, 23.95, top, *z),
                    [0xd8c8a8, 0xc8a878, 0xe0d0b0][i],
                );
                k.add(b(0.25, 0.02, 0.04, 23.95, top + 0.14, *z), 0x8a3a3a);
            }
            for (x, z, hex) in [
                (23.3, 4.5, 0xe07020),
                (23.25, 4.62, 0x6aa040),
                (23.38, 4.66, 0xc03030),
            ] {
                k.add(prims::sphere(0.055, 8, 6).translate(x, top + 0.055, z), hex);
            }
        }
        Which::LibraryRuin => {
            // half the roof on the floor: two heaps of stone and plaster with their beams,
            // books swollen with damp, the first demon's window in pieces between the tables
            for (cx, cz, n, seed) in [(-23.2f32, -1.8f32, 15usize, 3u32), (-12.7, 5.5, 12, 11)] {
                // the plaster's dust round the heap
                k.add(
                    prims::cylinder(1.25, 1.35, 0.02, 14)
                        .scale(1.0, 1.0, 0.8)
                        .translate(cx, 0.01, cz),
                    0xa8a098,
                );
                for i in 0..n {
                    let a = i as f32 * 2.399 + seed as f32;
                    let r = 0.1 + (i as f32 / n as f32) * 0.75;
                    let s = 0.58 - (i as f32 / n as f32) * 0.36;
                    let stone = prims::dodecahedron(s, 0)
                        .jagged(0.25, seed + i as u32)
                        .scale(1.0, 0.62, 1.0)
                        .translate(cx + a.cos() * r, s * 0.45, cz + a.sin() * r * 0.8);
                    k.add(stone, [0x8a8078, 0x6d655e, 0xb8aea0][i % 3]);
                }
                k.rod(
                    Vec3::new(cx - 1.05, 0.12, cz - 0.5),
                    Vec3::new(cx + 0.95, 0.62, cz + 0.35),
                    0.11,
                    0x5a3d26,
                );
                k.rod(
                    Vec3::new(cx - 0.4, 0.08, cz + 0.7),
                    Vec3::new(cx + 0.7, 0.38, cz - 0.55),
                    0.09,
                    0x4a321e,
                );
            }
            for (i, (x, z)) in [
                (-24.6f32, 0.6f32),
                (-24.3, 1.3),
                (-24.7, 3.4),
                (-20.6, -4.8),
                (-21.3, -4.6),
                (-15.2, -4.7),
            ]
            .into_iter()
            .enumerate()
            {
                k.add(
                    prims::boxed(0.17, 0.05, 0.24)
                        .rotate_y(i as f32 * 1.3)
                        .translate(x, 0.025, z),
                    [0x8a2a3a, 0x2a5a3a, 0x2a3a6a, 0x6a4a2a][i % 4],
                );
            }
            for i in 0..14 {
                let a = i as f32 * 2.1;
                let r = 0.2 + (i % 5) as f32 * 0.24;
                let shard = prims::plate(&[[0.0, 0.0], [0.13, 0.03], [0.06, 0.14]], 0.01)
                    .rotate_x(-PI / 2.0)
                    .rotate_y(a)
                    .translate(-16.6 + a.cos() * r, 0.012, -4.5 + a.sin() * r * 0.5);
                k.add(shard, [0xc0304a, 0x3050c0, 0xd8a030, 0x7a3ac0][i % 4]);
            }
        }
        Which::Camp => {
            // the fire's ring of stones and its logs
            let (fx, fz) = castle3d::CAMPFIRE;
            for i in 0..9 {
                let a = i as f32 / 9.0 * TAU;
                k.add(
                    prims::dodecahedron(0.12, 0).jagged(0.2, i).translate(
                        fx + a.cos() * 0.46,
                        0.07,
                        fz + a.sin() * 0.46,
                    ),
                    0x6a6470,
                );
            }
            for a in [0.3f32, 1.9] {
                k.rod(
                    Vec3::new(fx - a.cos() * 0.4, 0.06, fz - a.sin() * 0.4),
                    Vec3::new(fx + a.cos() * 0.4, 0.1, fz + a.sin() * 0.4),
                    0.06,
                    0x5a3d26,
                );
            }
            k.shine(cyl(0.3, 0.02, fx, 0.04, fz, 10), 0xff6a2a);
            // their bundles and sacks by the tents, a blanket on the ground
            for (x, z, s, hex) in [
                (16.9, 31.7, 0.22, 0xa08860u32),
                (17.2, 31.4, 0.17, 0x8a7a5a),
                (19.9, 32.6, 0.2, 0x9a7a52),
                (14.0, 31.9, 0.18, 0x7a6a4a),
            ] {
                k.add(
                    prims::sphere(s, 9, 6)
                        .scale(1.0, 0.8, 1.1)
                        .translate(x, s * 0.8, z),
                    hex,
                );
            }
            k.add(b(1.3, 0.03, 0.9, 18.3, 0.0, 30.6), 0x7a4a5a);
            k.add(b(1.3, 0.035, 0.12, 18.3, 0.0, 30.2), 0xc8a050);
        }
        Which::CampWork => {
            // working for their roof: hoes and a rake against a tent, baskets of what they dug
            for (i, x) in [16.7f32, 16.95, 17.2].into_iter().enumerate() {
                k.rod(
                    Vec3::new(x, 0.0, 29.5),
                    Vec3::new(x - 0.25, 1.45, 29.85),
                    0.025,
                    0x7a5634,
                );
                k.add(
                    b(0.18, 0.04, 0.05, x, 0.0, 29.45),
                    [0x8a8d98, 0x8a8d98, 0x6a4a2a][i],
                );
            }
            for (x, z) in [(19.6f32, 29.6f32), (20.2, 29.3)] {
                k.add(cyl(0.24, 0.28, x, 0.0, z, 10), 0xb48a52);
                for j in 0..5 {
                    let a = j as f32 * 1.26;
                    k.add(
                        prims::sphere(0.07, 7, 5).translate(
                            x + a.cos() * 0.1,
                            0.3,
                            z + a.sin() * 0.1,
                        ),
                        [0xc8a040, 0x8a6a3a, 0xd06030][j % 3],
                    );
                }
            }
        }
        Which::Swords => {
            // the smith's new swords, standing in a crate by the yard's rack
            let (x, z) = (-22.9, 27.7);
            k.add(b(0.9, 0.5, 0.6, x, 0.0, z), 0x8a6a44);
            k.add(b(0.94, 0.06, 0.64, x, 0.5, z), 0x6a4a2a);
            for i in 0..6 {
                let sx = x - 0.32 + (i % 3) as f32 * 0.32;
                let sz = z - 0.12 + (i / 3) as f32 * 0.24;
                k.add(b(0.055, 0.62, 0.014, sx, 0.4, sz), 0xc8ccd8);
                k.add(b(0.24, 0.035, 0.05, sx, 1.02, sz), 0xd8a840);
                k.add(cyl(0.02, 0.2, sx, 1.05, sz, 6), 0x4a321e);
                k.add(prims::sphere(0.035, 6, 4).translate(sx, 1.27, sz), 0xd8a840);
            }
        }
        Which::Ploughs => {
            // two ploughs from the old breastplates, against the yard's north wall
            for (i, x) in [-8.1f32, -6.7].into_iter().enumerate() {
                let foot = Vec3::new(x, 0.05, 23.25);
                let top = Vec3::new(x + 0.35, 1.25, 22.55);
                k.rod(foot, top, 0.05, 0x7a5634);
                k.rod(
                    top + Vec3::new(-0.25, 0.0, 0.0),
                    top + Vec3::new(0.25, 0.0, 0.0),
                    0.03,
                    0x7a5634,
                );
                k.add(
                    prims::plate(&[[-0.18, 0.0], [0.18, 0.0], [0.0, 0.32]], 0.03)
                        .rotate_x(-0.4)
                        .translate(x, 0.02, 23.3),
                    [0xa8acb8, 0x9a9ea8][i],
                );
            }
        }
        Which::Wedding => {
            // Durgo and Pimpa's garlands: over the gate and over the castle's door
            for (a, bb, sag) in [
                (Vec3::new(-2.3, 3.3, 37.55), Vec3::new(2.3, 3.3, 37.55), 0.7),
                (Vec3::new(-1.9, 2.9, 22.45), Vec3::new(1.9, 2.9, 22.45), 0.5),
            ] {
                let pts = hanging(a, bb, sag, 22);
                for w in pts.windows(2) {
                    k.rod(w[0], w[1], 0.03, 0x4a7a3a);
                }
                for (i, p) in pts.iter().enumerate() {
                    let hex = [0xf4ecf0, 0xe880a8, 0xf0c040][i % 3];
                    k.add(
                        prims::sphere(0.085, 7, 5).translate(p.x, p.y - 0.03, p.z),
                        hex,
                    );
                    if i % 2 == 0 {
                        k.add(
                            prims::sphere(0.06, 6, 4).scale(1.4, 0.5, 1.0).translate(
                                p.x + 0.06,
                                p.y + 0.02,
                                p.z,
                            ),
                            0x5a9a4a,
                        );
                    }
                }
                for end in [a, bb] {
                    k.add(b(0.1, 0.9, 0.02, end.x, end.y - 0.95, end.z), 0xf4ecf0);
                    k.add(
                        b(0.1, 0.7, 0.02, end.x + 0.12, end.y - 0.75, end.z),
                        0xe880a8,
                    );
                }
            }
        }
        Which::Feast => {
            // the village's lanterns, strung between the yard's lamp posts
            let posts = [(-2.6f32, 25.4f32), (2.6, 25.4), (2.6, 30.6), (-2.6, 30.6)];
            for i in 0..4 {
                let (p, q) = (posts[i], posts[(i + 1) % 4]);
                let a = Vec3::new(p.0, 2.75, p.1);
                let bb = Vec3::new(q.0, 2.75, q.1);
                let pts = hanging(a, bb, 0.35, 12);
                for w in pts.windows(2) {
                    k.rod(w[0], w[1], 0.012, 0x2a2228);
                }
                for (j, p) in pts.iter().enumerate().skip(1).step_by(2) {
                    let hex = [0xff7a3a, 0xffc14a, 0xd25aff, 0xff4a6a][(i + j) % 4];
                    k.shine(
                        prims::sphere(0.11, 8, 6).scale(1.0, 1.25, 1.0).translate(
                            p.x,
                            p.y - 0.17,
                            p.z,
                        ),
                        hex,
                    );
                    k.add(cyl(0.05, 0.04, p.x, p.y - 0.06, p.z, 6), 0x2a2228);
                }
            }
        }
        Which::Memorial => {
            // the stone for the capital's dead, its names, flowers and candles
            let (x, z) = (-18.0, 36.35);
            k.add(b(1.1, 0.18, 0.55, x, 0.0, z), 0x6a6876);
            k.add(
                prims::boxed(0.9, 1.15, 0.26)
                    .jagged(0.03, 5)
                    .translate(x, 0.18 + 0.575, z),
                0x8c8a94,
            );
            for row in 0..5 {
                k.add(
                    b(0.62, 0.025, 0.01, x, 0.55 + row as f32 * 0.14, z - 0.135),
                    0x3a3844,
                );
            }
            for i in 0..7 {
                let fx = x - 0.4 + i as f32 * 0.13;
                k.add(
                    prims::sphere(0.05, 6, 4).translate(fx, 0.22, z - 0.38),
                    [0xf4ecf0, 0xf0c040, 0xb05ad8][i % 3],
                );
            }
            for fx in [x - 0.42, x, x + 0.42] {
                k.add(cyl(0.035, 0.16, fx, 0.18, z - 0.2, 6), 0xf0ead8);
                k.shine(
                    prims::sphere(0.03, 6, 4)
                        .scale(1.0, 1.6, 1.0)
                        .translate(fx, 0.38, z - 0.2),
                    0xffb050,
                );
            }
        }
    }
    k
}

// ------------------------------------------------------------------ Bertila and her kids

/// A goat's colours: coat, belly and snout, horns, the kid's forehead spots (if any).
struct Coat {
    coat: u32,
    light: u32,
    snout: u32,
    horns: bool,
    spots: bool,
}

/// A goat in pieces, each in its own joint's space (facing +z): body, head (from the neck), a leg
/// (from the hip down), the tail.
struct Goat {
    body: MeshHandle,
    head: MeshHandle,
    leg: MeshHandle,
    tail: MeshHandle,
}

/// Where her neck, hips and tail sit on her body.
const NECK: Vec3 = Vec3::new(0.0, 0.72, 0.34);
const HIPS: [Vec3; 4] = [
    Vec3::new(0.13, 0.48, 0.24),
    Vec3::new(-0.13, 0.48, 0.24),
    Vec3::new(0.13, 0.48, -0.24),
    Vec3::new(-0.13, 0.48, -0.24),
];
const TAIL: Vec3 = Vec3::new(0.0, 0.76, -0.36);

fn goat_meshes(c: &Coat) -> [Mesh3D; 4] {
    let mut body = Builder::default();
    body.add(
        &prims::sphere(0.3, 12, 8)
            .scale(0.8, 0.72, 1.25)
            .translate(0.0, 0.62, 0.0),
        rgb(c.coat),
    );
    body.add(
        &prims::sphere(0.24, 10, 6)
            .scale(0.8, 0.5, 1.1)
            .translate(0.0, 0.52, 0.02),
        rgb(c.light),
    );
    let mut head = Builder::default();
    head.add(
        &prims::cylinder(0.08, 0.1, 0.3, 8)
            .rotate_x(0.6)
            .translate(0.0, 0.06, 0.04),
        rgb(c.coat),
    );
    head.add(
        &prims::sphere(0.12, 10, 8)
            .scale(0.85, 0.9, 1.25)
            .translate(0.0, 0.2, 0.2),
        rgb(c.coat),
    );
    head.add(
        &prims::sphere(0.08, 8, 6)
            .scale(0.9, 0.8, 1.2)
            .translate(0.0, 0.15, 0.33),
        rgb(c.snout),
    );
    for s in [-1.0f32, 1.0] {
        head.add(
            &prims::sphere(0.022, 6, 4).translate(s * 0.075, 0.24, 0.27),
            rgb(0x2a1c10),
        );
        head.add(
            &prims::boxed(0.15, 0.03, 0.065)
                .rotate_z(s * 0.5)
                .translate(s * 0.12, 0.23, 0.15),
            rgb(c.coat),
        );
        if c.horns {
            head.add(
                &prims::cone(0.034, 0.24, 6)
                    .rotate_x(-1.0)
                    .translate(s * 0.05, 0.33, 0.12),
                rgb(0x8a7a66),
            );
        } else {
            head.add(
                &prims::cone(0.025, 0.06, 6).translate(s * 0.045, 0.3, 0.18),
                rgb(0xb8a890),
            );
        }
        if c.spots {
            head.add(
                &prims::sphere(0.032, 6, 4)
                    .scale(1.0, 0.45, 1.0)
                    .translate(s * 0.042, 0.3, 0.255),
                rgb(0x1a1418),
            );
        }
    }
    if c.horns {
        head.add(
            &prims::cone(0.042, 0.15, 6)
                .rotate_x(PI)
                .translate(0.0, 0.03, 0.34),
            rgb(c.light),
        );
    }
    let mut leg = Builder::default();
    leg.add(
        &prims::cylinder(0.042, 0.036, 0.44, 6).translate(0.0, -0.22, 0.0),
        rgb(c.coat),
    );
    leg.add(
        &prims::cylinder(0.04, 0.043, 0.06, 6).translate(0.0, -0.45, 0.0),
        rgb(0x3a302a),
    );
    let mut tail = Builder::default();
    tail.add(
        &prims::cone(0.05, 0.15, 6)
            .rotate_x(-0.5)
            .translate(0.0, 0.06, -0.02),
        rgb(c.coat),
    );
    [body, head, leg, tail].map(|b| b.build().expect("a goat's piece"))
}

/// Her crown (on her head, from the neck) and her bell (on her chest).
fn crown_mesh() -> Mesh3D {
    let mut b = Builder::default();
    b.add(
        &prims::cylinder(0.085, 0.09, 0.05, 10).translate(0.0, 0.33, 0.19),
        rgb(0xe0b040),
    );
    for i in 0..5 {
        let a = i as f32 / 5.0 * TAU;
        b.add(
            &prims::cone(0.022, 0.07, 5).translate(a.sin() * 0.075, 0.385, 0.19 + a.cos() * 0.075),
            rgb(0xe0b040),
        );
    }
    b.add(
        &prims::sphere(0.02, 6, 4).translate(0.0, 0.35, 0.28),
        rgb(0xc0304a),
    );
    b.build().expect("a crown")
}

fn bell_mesh() -> Mesh3D {
    let mut b = Builder::default();
    b.add(
        &prims::cylinder(0.13, 0.13, 0.035, 12)
            .rotate_x(1.1)
            .translate(0.0, 0.7, 0.36),
        rgb(0x8a2a3a),
    );
    b.add(
        &prims::sphere(0.05, 8, 6).translate(0.0, 0.58, 0.47),
        rgb(0xe0b040),
    );
    b.build().expect("a bell")
}

/// Bertila's walk round the yard by the well: an oval she goes round a little at a time,
/// grazing between.
const GOAT_CENTRE: (f32, f32) = (8.0, 27.6);
const GOAT_RADII: (f32, f32) = (3.0, 1.25);
const GOAT_CYCLE: f32 = 16.0;
const GOAT_WALK: f32 = 7.0;
const GOAT_SPEED: f32 = 0.34;

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where along her oval (radians) she is at time t, how fast she goes (0–1) and how far down her
/// head is (0–1).
fn goat_at(t: f32) -> (f32, f32, f32) {
    let round = (t / GOAT_CYCLE).floor();
    let r = t - round * GOAT_CYCLE;
    let walked = round * GOAT_WALK + r.min(GOAT_WALK);
    let (rx, rz) = GOAT_RADII;
    let perimeter = TAU * ((rx * rx + rz * rz) / 2.0).sqrt();
    let theta = walked * GOAT_SPEED / perimeter * TAU;
    let mv = if r < GOAT_WALK {
        smooth(r.min(GOAT_WALK - r) / 0.6)
    } else {
        0.0
    };
    let graze = if r >= GOAT_WALK {
        smooth((r - GOAT_WALK).min(GOAT_CYCLE - r) / 0.7)
    } else {
        0.0
    };
    (theta, mv, graze)
}

fn oval(theta: f32) -> (Vec3, f32) {
    let (cx, cz) = GOAT_CENTRE;
    let (rx, rz) = GOAT_RADII;
    let at = Vec3::new(cx + rx * theta.cos(), 0.0, cz + rz * theta.sin());
    let (dx, dz) = (-rx * theta.sin(), rz * theta.cos());
    (at, dx.atan2(dz))
}

// ------------------------------------------------------------------ the folk

struct Crowd {
    mesh: MeshHandle,
    cap: usize,
    buf: Vec<CrowdInstance3D>,
}

const ORC: usize = 0;
const GOBLIN: usize = 1;
const FOLK_A: usize = 2;
const FOLK_B: usize = 3;

fn shape_piece(shape: &Shape, hex: u32, part: f32, pivot: [f32; 3]) -> Piece {
    let c = rgb(hex);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for t in shape.tris.chunks_exact(3) {
        let (a, b, cc) = (
            Vec3::new(t[0][0], t[0][1], t[0][2]),
            Vec3::new(t[1][0], t[1][1], t[1][2]),
            Vec3::new(t[2][0], t[2][1], t[2][2]),
        );
        let n = (b - a).cross(cc - a).normalize_or_zero();
        for p in t {
            positions.push(*p);
            normals.push([n.x, n.y, n.z]);
        }
    }
    let indices = (0..positions.len() as u32).collect();
    Piece {
        node: "BODY__folk".into(),
        part,
        pivot,
        material: "folk".into(),
        color: [c[0], c[1], c[2], 1.0],
        positions,
        normals,
        indices,
    }
}

/// Villagers from the war's spearman: no spear, shield or helmet, in their own clothes, with a
/// hood (a) or a straw hat (b).
fn folk(spearman: &[Piece], hat: bool) -> CrowdMesh3D {
    let (tunic, trousers, rope) = if hat {
        (0x6a7a5a, 0x4a3e34, 0x8a6a40)
    } else {
        (0x9a7a52, 0x5a4a3a, 0x6a4a2a)
    };
    let mut pieces: Vec<Piece> = spearman
        .iter()
        .filter(|p| !p.node.starts_with("THRUST"))
        .filter(|p| p.material != "metal_light")
        .filter(|p| {
            !(p.node.starts_with("ARM_S") && matches!(p.material.as_str(), "trim" | "cloth"))
        })
        .cloned()
        .map(|mut p| {
            let hex = match p.material.as_str() {
                "mail" => Some(tunic),
                "ally_blue" => Some(trousers),
                "trim" => Some(rope),
                "cloth" => Some(tunic),
                _ => None,
            };
            if let Some(hex) = hex {
                let c = rgb(hex);
                p.color = [c[0], c[1], c[2], 1.0];
                p.material = "folk".into();
            }
            p
        })
        .collect();
    if hat {
        pieces.push(shape_piece(
            &prims::cylinder(0.27, 0.27, 0.03, 12).translate(0.0, 1.8, 0.0),
            0xd8c080,
            part::BODY,
            [0.0; 3],
        ));
        pieces.push(shape_piece(
            &prims::cylinder(0.08, 0.13, 0.13, 10).translate(0.0, 1.87, 0.0),
            0xd8c080,
            part::BODY,
            [0.0; 3],
        ));
    } else {
        pieces.push(shape_piece(
            &prims::sphere(0.15, 10, 6)
                .scale(1.0, 0.9, 1.05)
                .translate(0.0, 1.73, -0.02),
            0x7a5a3e,
            part::BODY,
            [0.0; 3],
        ));
        pieces.push(shape_piece(
            &prims::cone(0.17, 0.4, 8).translate(0.0, 1.45, -0.08),
            0x7a5a3e,
            part::BODY,
            [0.0; 3],
        ));
    }
    models::crowd_mesh("spearman", &pieces, Faction::Reina)
}

fn crowd_from(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    model: &CrowdMesh3D,
    cap: usize,
) -> Option<Crowd> {
    match renderer.upload_crowd(device, model, cap) {
        Ok(mesh) => Some(Crowd {
            mesh,
            cap,
            buf: Vec::with_capacity(cap),
        }),
        Err(e) => {
            eprintln!("[castillo] crowd: {e:?}");
            None
        }
    }
}

/// A villager or a guard standing (idle), facing `yaw`.
fn standing(x: f32, y: f32, z: f32, yaw: f32, scale: f32, seed: f32) -> CrowdInstance3D {
    let mut i = CrowdInstance3D::at([x, y, z], yaw, scale);
    i.anim_a = [seed * 2.7, 0.0, 0.0, 0.0];
    i
}

// ------------------------------------------------------------------ the scene

pub(crate) struct Scene {
    sets: Vec<(Which, Option<MeshHandle>, Option<MeshHandle>)>,
    goats: Vec<Goat>,
    crown: MeshHandle,
    bell: MeshHandle,
    crowds: Vec<Option<Crowd>>,
    tent: Option<(MeshHandle, f32)>,
    campfire: ToonFire3D,
    forge: ToonFire3D,
}

impl Scene {
    pub(crate) fn load(renderer: &mut Wgpu3DRenderer, device: &wgpu::Device) -> Result<Self> {
        let started = Instant::now();
        let mut up = |m: Option<Mesh3D>| -> Result<Option<MeshHandle>> {
            match m {
                Some(m) => Ok(Some(
                    renderer
                        .upload_mesh(device, &m)
                        .map_err(|e| anyhow::anyhow!("{e:?}"))?,
                )),
                None => Ok(None),
            }
        };
        let mut sets = Vec::new();
        for which in Which::ALL {
            let kit = build(which);
            sets.push((which, up(kit.solid.build())?, up(kit.glow.build())?));
        }
        let coats = [
            Coat {
                coat: 0xefe7d6,
                light: 0xfaf6ee,
                snout: 0xc9a8a0,
                horns: true,
                spots: false,
            },
            Coat {
                coat: 0xf2ebdc,
                light: 0xfaf6ee,
                snout: 0xd8b4ac,
                horns: false,
                spots: true,
            },
            Coat {
                coat: 0x9a6a44,
                light: 0xe8dcc8,
                snout: 0xb8907a,
                horns: false,
                spots: false,
            },
        ];
        let mut goats = Vec::new();
        for c in &coats {
            let [body, head, leg, tail] = goat_meshes(c);
            goats.push(Goat {
                body: up(Some(body))?.expect("a body"),
                head: up(Some(head))?.expect("a head"),
                leg: up(Some(leg))?.expect("a leg"),
                tail: up(Some(tail))?.expect("a tail"),
            });
        }
        let crown = up(Some(crown_mesh()))?.expect("a crown");
        let bell = up(Some(bell_mesh()))?.expect("a bell");
        let troops = exploration3d_state::data_dir().join("assets/war/troops");
        let mut crowds = Vec::new();
        for kind in ["orc", "goblin"] {
            crowds.push(
                models::load_glb(&troops.join(format!("{kind}.glb")))
                    .map_err(|e| eprintln!("[castillo] {kind}: {e:#}"))
                    .ok()
                    .and_then(|p| {
                        crowd_from(
                            renderer,
                            device,
                            &models::crowd_mesh(kind, &p, Faction::Reina),
                            8,
                        )
                    }),
            );
        }
        let spearman = models::load_glb(&troops.join("spearman.glb"))
            .map_err(|e| eprintln!("[castillo] spearman: {e:#}"))
            .unwrap_or_default();
        for hat in [false, true] {
            crowds.push(if spearman.is_empty() {
                None
            } else {
                crowd_from(renderer, device, &folk(&spearman, hat), 8)
            });
        }
        let props = exploration3d_state::data_dir().join("assets/war/props");
        let tent = models::load_glb(&props.join("tent.glb"))
            .map_err(|e| eprintln!("[castillo] tent: {e:#}"))
            .ok()
            .and_then(|pieces| {
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for p in pieces.iter().flat_map(|p| p.positions.iter()) {
                    for k in 0..3 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
                let wide = (hi[0] - lo[0]).max(hi[2] - lo[2]).max(0.1);
                let mesh = models::static_mesh(&pieces, |_| true, |_| None)?;
                let handle = renderer.upload_mesh(device, &mesh).ok()?;
                Some((handle, 2.25 / wide))
            });
        let (fx, fz) = castle3d::CAMPFIRE;
        let campfire = ToonFire3D::new(renderer, device, Vec3::new(fx, 0.05, fz), 0.34)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let level_h = castle3d::plan().level_h;
        let (forge_x, forge_z) = castle3d::FORGE;
        let forge = ToonFire3D::new(
            renderer,
            device,
            Vec3::new(forge_x, level_h + 0.95, forge_z + 0.02),
            0.4,
        )
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        eprintln!(
            "[3d] the court's castle life built in {:.0} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(Self {
            sets,
            goats,
            crown,
            bell,
            crowds,
            tent,
            campfire,
            forge,
        })
    }

    fn crowd_push(&mut self, which: usize, i: CrowdInstance3D) {
        if let Some(Some(c)) = self.crowds.get_mut(which) {
            if c.buf.len() < c.cap {
                c.buf.push(i);
            }
        }
    }

    /// What the court's life adds to this frame of the castle (`view`: the floor in view;
    /// `toward`: from the scene to the camera), and its fires' light.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draws(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        life: Life,
        view: u8,
        t: f32,
        toward: Vec3,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
    ) {
        let level_h = castle3d::plan().level_h;
        let solid = Material3D::default().with_ambient_occlusion(0.28);
        let glow = Material3D::default().with_emission(1.6);
        for (which, body, shine) in &self.sets {
            if !which.shown(&life) || !which.floor().in_view(view) {
                continue;
            }
            for (mesh, material) in [(body, solid), (shine, glow)] {
                if let Some(mesh) = mesh {
                    draws.push(MeshDraw3D {
                        mesh: *mesh,
                        transform: Mat4::IDENTITY,
                        material,
                        texture: None,
                    });
                }
            }
        }
        let flicker = 1.0 + 0.08 * (t * 9.0).sin() + 0.05 * (t * 23.0).sin();
        if life.armory && view == 1 {
            self.forge.draws(t, toward, draws);
            let (forge_x, forge_z) = castle3d::FORGE;
            lights.push(PointLight3D::new(
                Vec3::new(forge_x, level_h + 1.3, forge_z + 0.6),
                [1.0, 0.55, 0.25],
                9.0 * flicker,
                7.0,
            ));
        }
        if life.refugees > 0 {
            let (fx, fz) = castle3d::CAMPFIRE;
            self.campfire.draws(t, toward, draws);
            lights.push(PointLight3D::new(
                Vec3::new(fx, 1.0, fz),
                [1.0, 0.6, 0.3],
                7.0 * flicker,
                8.0,
            ));
            if let Some((tent, scale)) = self.tent {
                for (x, z, yaw) in castle3d::TENTS {
                    draws.push(MeshDraw3D {
                        mesh: tent,
                        transform: Mat4::from_translation(Vec3::new(x, 0.0, z))
                            * Mat4::from_rotation_y(yaw)
                            * Mat4::from_scale(Vec3::new(scale, scale, scale)),
                        material: solid,
                        texture: None,
                    });
                }
            }
        }
        if life.feast {
            lights.push(PointLight3D::new(
                Vec3::new(0.0, 2.6, 28.0),
                [1.0, 0.7, 0.45],
                6.0,
                9.0,
            ));
        }
        if life.goat > 0 {
            self.goats_draws(&life, t, draws, solid);
        }
        // the folk
        for c in self.crowds.iter_mut().flatten() {
            c.buf.clear();
        }
        if life.barracks {
            for (i, (x, z, yaw)) in castle3d::GATE_GUARDS.into_iter().enumerate() {
                self.crowd_push(ORC, standing(x, 0.0, z, yaw, 0.92, i as f32));
            }
            if view == 1 {
                let [(gx, gz, gyaw), (ox, oz, oyaw)] = castle3d::BARRACK_GUARDS;
                self.crowd_push(GOBLIN, standing(gx, level_h, gz, gyaw, 1.0, 3.0));
                self.crowd_push(ORC, standing(ox, level_h, oz, oyaw, 0.92, 4.0));
            }
            // one on his round of the yard, between the gate and the castle's door
            let round = [(3.2f32, 24.2f32), (3.2, 34.4), (-3.2, 34.4), (-3.2, 24.2)];
            let length: f32 = (0..4)
                .map(|i| {
                    let (a, b) = (round[i], round[(i + 1) % 4]);
                    (b.0 - a.0).hypot(b.1 - a.1)
                })
                .sum();
            let speed = 1.1;
            let mut left = (t * speed).rem_euclid(length);
            for i in 0..4 {
                let (a, b) = (round[i], round[(i + 1) % 4]);
                let len = (b.0 - a.0).hypot(b.1 - a.1);
                if left <= len {
                    let k = left / len;
                    let (x, z) = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
                    let mut inst =
                        CrowdInstance3D::at([x, 0.0, z], (b.0 - a.0).atan2(b.1 - a.1), 1.0);
                    inst.anim_a = [t * speed * 4.4, 1.0, 0.0, 0.0];
                    self.crowd_push(GOBLIN, inst);
                    break;
                }
                left -= len;
            }
        }
        if life.armory && view == 1 {
            // the smith at her anvil
            let (x, z) = castle3d::SMITH;
            let mut inst = CrowdInstance3D::at([x, level_h, z], PI, 1.0);
            let beat = (t * 0.95).fract();
            inst.anim_a = [1.3, 0.0, 0.02 + beat * 0.96, 0.0];
            self.crowd_push(GOBLIN, inst);
        }
        if life.refugees > 0 {
            let (fx, fz) = castle3d::CAMPFIRE;
            for (i, (x, z)) in castle3d::REFUGEES.into_iter().enumerate() {
                let yaw = (fx - x).atan2(fz - z);
                let which = if i % 2 == 0 { FOLK_A } else { FOLK_B };
                let mut inst = standing(x, 0.0, z, yaw, 0.96 + (i % 3) as f32 * 0.03, i as f32);
                let tint = [1.0, 0.92, 1.05, 0.97][i % 4];
                inst.color = [tint, tint, tint, 1.0];
                self.crowd_push(which, inst);
            }
            // the children run round between the tents
            for k in 0..2 {
                let a = t * 1.3 + k as f32 * PI;
                let (x, z) = (18.4 + a.cos() * 1.2, 31.8 + a.sin() * 1.05);
                let yaw = (-a.sin()).atan2(a.cos() * 1.05 / 1.2);
                let mut inst = CrowdInstance3D::at([x, 0.0, z], yaw, 0.6);
                inst.anim_a = [t * 7.5 + k as f32, 1.0, 0.0, 0.0];
                self.crowd_push(if k == 0 { FOLK_B } else { FOLK_A }, inst);
            }
        }
        for c in self.crowds.iter_mut().flatten() {
            let n = renderer.set_crowd(queue, c.mesh, &c.buf, t);
            if n > 0 {
                draws.push(MeshDraw3D {
                    mesh: c.mesh,
                    transform: Mat4::IDENTITY,
                    material: Material3D::default().with_ambient_occlusion(0.2),
                    texture: None,
                });
            }
        }
    }

    fn goats_draws(&self, life: &Life, t: f32, draws: &mut Vec<MeshDraw3D>, material: Material3D) {
        let (theta, mv, graze) = goat_at(t);
        let mut herd = vec![(0usize, theta, mv, graze, 1.75f32, 0.0f32)];
        if life.kids > 0 {
            for k in 0..2 {
                // the kids keep behind her, and play while she grazes
                let hop = (1.0 - mv) * (t * 5.0 + k as f32 * 1.7).sin().abs() * 0.1;
                herd.push((
                    1 + k,
                    theta - 0.5 - 0.4 * k as f32,
                    mv.max(0.25),
                    0.0,
                    1.0,
                    hop,
                ));
            }
        }
        for (i, (kind, theta, mv, graze, scale, hop)) in herd.into_iter().enumerate() {
            let goat = &self.goats[kind];
            let (at, yaw) = oval(theta);
            let ph = theta * 26.0 + i as f32;
            let bob = (ph * 2.0).sin().abs() * 0.02 * mv;
            let world = Mat4::from_translation(at + Vec3::new(0.0, hop + bob, 0.0))
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(scale, scale, scale));
            let mut push = |mesh: MeshHandle, local: Mat4| {
                draws.push(MeshDraw3D {
                    mesh,
                    transform: world * local,
                    material,
                    texture: None,
                });
            };
            push(goat.body, Mat4::IDENTITY);
            let nibble = 0.06 * (t * 7.0 + i as f32).sin() * graze;
            let dip = graze * 1.05 + nibble - 0.12 * (1.0 - graze) + 0.05 * (ph * 2.0).sin() * mv;
            let head = Mat4::from_translation(NECK) * Mat4::from_rotation_x(dip);
            push(goat.head, head);
            if kind == 0 && life.goat == 2 {
                push(self.crown, head);
            }
            if kind == 0 && life.kids == 2 {
                push(self.bell, Mat4::IDENTITY);
            }
            for (k, hip) in HIPS.into_iter().enumerate() {
                let off = if k == 0 || k == 3 { 0.0 } else { PI };
                push(
                    goat.leg,
                    Mat4::from_translation(hip)
                        * Mat4::from_rotation_x((ph + off).sin() * 0.5 * mv),
                );
            }
            push(
                goat.tail,
                Mat4::from_translation(TAIL)
                    * Mat4::from_rotation_z((t * 6.0 + i as f32).sin() * 0.35),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_story_sets_what_the_castle_shows() {
        let mut v = guion::Vars::default();
        assert_eq!(life(&v), Life::default());
        v.values.insert("obra_cocina".into(), 1);
        v.values.insert("aud_cabra_grubbo".into(), 3);
        v.values.insert("aud_cabra_trono".into(), 3);
        v.values.insert("aud_refugiados_norte".into(), 3);
        v.values.insert("aud_herrera_armaduras".into(), 3);
        let l = life(&v);
        assert!(l.kitchen && !l.armory);
        assert_eq!((l.goat, l.kids, l.refugees), (2, 2, 0));
        assert!(l.swords && l.ploughs);
    }

    #[test]
    fn bertila_keeps_to_her_corner_of_the_yard() {
        let p = castle3d::plan();
        for k in 0..400 {
            let (theta, _, _) = goat_at(k as f32 * 0.37);
            for back in [0.0, 0.5, 0.9] {
                let (at, _) = oval(theta - back);
                assert!(!p.blocked(at.x, at.z, 0.6, 0), "{at:?}");
                assert!((at.x - 9.0).hypot(at.z - 31.0) > 2.0, "the well {at:?}");
            }
        }
    }

    #[test]
    fn every_set_builds() {
        for which in Which::ALL {
            let kit = build(which);
            assert!(!kit.solid.is_empty() || !kit.glow.is_empty(), "{which:?}");
        }
    }
}
