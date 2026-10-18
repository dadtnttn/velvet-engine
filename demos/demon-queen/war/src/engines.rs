//! What armies build, modelled for the battle (low-poly, flat-shaded, as the field's scatter):
//! the catapult, the trebuchet, the ballista and the scorpion, the siege tower and the ram, the
//! stakes and the palisade. Each faces +z (where it shoots or rolls); a line runs along x. What
//! moves comes apart, with the point it turns round: the throwing arms, the ballistas' bows, the
//! tower's bridge, the ram's log.
use velvet_math::{Mat4, Vec3};
use velvet_render::Mesh3D;

use crate::data::{rgb, Faction};
use crate::prims::{self, Builder, Shape};
use crate::siege::Kind;

pub struct EngineModel {
    pub body: Mesh3D,
    /// The moving part and where it turns (in the model's frame); it turns about x.
    pub arm: Option<(Mesh3D, [f32; 3])>,
}

const WOOD: u32 = 0x8a6340;
const WOOD_DARK: u32 = 0x5a3e2a;
const HIDE: u32 = 0xa47a4e;
const IRON: u32 = 0x5c5864;
const ROPE: u32 = 0xb8a070;
const STONE: u32 = 0x8f8496;

/// A squared beam from a to b, `w` thick.
fn bar(a: Vec3, b: Vec3, w: f32) -> Shape {
    let d = b - a;
    let len = d.length().max(0.01);
    let z = d * (1.0 / len);
    let up = if z.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
    let x = up.cross(z).normalize_or_zero();
    let y = z.cross(x);
    let c = (a + b) * 0.5;
    prims::boxed(w, w, len).transform(Mat4::from_cols(
        [x.x, x.y, x.z, 0.0],
        [y.x, y.y, y.z, 0.0],
        [z.x, z.y, z.z, 0.0],
        [c.x, c.y, c.z, 1.0],
    ))
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn wheel(b: &mut Builder, x: f32, y: f32, z: f32, r: f32) {
    b.add(
        &prims::cylinder(r, r, 0.22, 10)
            .rotate_z(std::f32::consts::FRAC_PI_2)
            .translate(x, y, z),
        rgb(WOOD_DARK),
    );
    b.add(
        &prims::cylinder(r * 0.25, r * 0.25, 0.3, 6)
            .rotate_z(std::f32::consts::FRAC_PI_2)
            .translate(x, y, z),
        rgb(IRON),
    );
}

fn banner(b: &mut Builder, at: Vec3, look: Faction) {
    let c = match look {
        Faction::Reina => rgb(0x6a32c0),
        Faction::Alianza => rgb(0x3d6fd0),
        Faction::Rebeldes => rgb(0xa8442a),
        Faction::Libres => rgb(0x6f7f68),
    };
    b.add(&bar(at, at + v(0.0, 2.2, 0.0), 0.08), rgb(WOOD_DARK));
    b.add(
        &prims::boxed(0.05, 0.9, 0.7).translate(at.x, at.y + 1.7, at.z + 0.38),
        c,
    );
}

fn mesh(f: impl FnOnce(&mut Builder)) -> Mesh3D {
    let mut b = Builder::default();
    f(&mut b);
    b.build().expect("a model with something in it")
}

/// The model of `kind` in `look`'s colours.
pub fn model(kind: Kind, look: Faction) -> EngineModel {
    let wood = rgb(WOOD);
    let dark = rgb(WOOD_DARK);
    match kind {
        Kind::Catapulta => EngineModel {
            body: mesh(|b| {
                // a sled frame on four small wheels, two uprights and the padded stop
                for s in [-1.0, 1.0] {
                    b.add(
                        &bar(v(s * 1.1, 0.5, -2.4), v(s * 1.1, 0.5, 2.0), 0.32),
                        dark,
                    );
                    b.add(&bar(v(s * 1.1, 0.5, 0.6), v(s * 0.9, 2.6, 0.9), 0.26), wood);
                    b.add(
                        &bar(v(s * 1.1, 0.5, -0.4), v(s * 0.9, 2.6, 0.9), 0.22),
                        wood,
                    );
                    wheel(b, s * 1.32, 0.42, -1.8, 0.42);
                    wheel(b, s * 1.32, 0.42, 1.5, 0.42);
                }
                b.add(&bar(v(-1.2, 2.6, 0.9), v(1.2, 2.6, 0.9), 0.3), wood);
                b.add(
                    &prims::boxed(1.4, 0.5, 0.4).translate(0.0, 2.75, 0.75),
                    rgb(HIDE),
                );
                for z in [-2.0, 1.6] {
                    b.add(&bar(v(-1.1, 0.5, z), v(1.1, 0.5, z), 0.26), dark);
                }
                // the twisted ropes round the axle
                b.add(
                    &prims::cylinder(0.38, 0.38, 2.0, 8)
                        .rotate_z(std::f32::consts::FRAC_PI_2)
                        .translate(0.0, 0.75, -1.4),
                    rgb(ROPE),
                );
                banner(b, v(-1.1, 0.6, -2.4), look);
            }),
            arm: Some((
                mesh(|b| {
                    // the arm lies back along -z from the axle; the sling's cup at its end
                    b.add(&bar(v(0.0, 0.0, 0.0), v(0.0, 0.15, -3.6), 0.24), wood);
                    b.add(
                        &prims::cylinder(0.45, 0.3, 0.32, 8).translate(0.0, 0.3, -3.7),
                        dark,
                    );
                    b.add(
                        &prims::dodecahedron(0.32, 0).translate(0.0, 0.55, -3.7),
                        rgb(STONE),
                    );
                }),
                [0.0, 0.75, -1.4],
            )),
        },
        Kind::Trabuquete => EngineModel {
            body: mesh(|b| {
                // two A-frames on a cross base, the axle 8 m up
                for s in [-1.0, 1.0] {
                    b.add(&bar(v(s * 1.4, 0.4, -3.6), v(s * 1.4, 0.4, 3.6), 0.4), dark);
                    b.add(
                        &bar(v(s * 1.4, 0.4, -3.2), v(s * 1.1, 8.0, 0.0), 0.34),
                        wood,
                    );
                    b.add(&bar(v(s * 1.4, 0.4, 3.2), v(s * 1.1, 8.0, 0.0), 0.34), wood);
                    b.add(
                        &bar(v(s * 1.4, 2.6, -2.3), v(s * 1.4, 2.6, 2.3), 0.22),
                        wood,
                    );
                }
                for z in [-3.2, 0.0, 3.2] {
                    b.add(&bar(v(-1.6, 0.4, z), v(1.6, 0.4, z), 0.36), dark);
                }
                b.add(&bar(v(-1.4, 8.0, 0.0), v(1.4, 8.0, 0.0), 0.3), rgb(IRON));
                banner(b, v(1.4, 0.5, 3.5), look);
            }),
            arm: Some((
                mesh(|b| {
                    // the long arm back and down (-z), the counterweight's box on the short end
                    b.add(&bar(v(0.0, 0.0, 2.6), v(0.0, 0.0, -9.0), 0.36), wood);
                    b.add(&prims::boxed(2.0, 1.8, 1.8).translate(0.0, -1.4, 2.6), dark);
                    b.add(
                        &prims::boxed(1.6, 0.6, 1.4).translate(0.0, -0.2, 2.6),
                        rgb(STONE),
                    );
                    b.add(&bar(v(0.0, 0.0, -9.0), v(0.0, -2.2, -9.4), 0.06), rgb(ROPE));
                    b.add(
                        &prims::dodecahedron(0.5, 0).translate(0.0, -2.5, -9.4),
                        rgb(STONE),
                    );
                }),
                [0.0, 8.0, 0.0],
            )),
        },
        Kind::Balista | Kind::Escorpion => {
            let k = if kind == Kind::Balista { 1.0 } else { 0.7 };
            EngineModel {
                body: mesh(|b| {
                    // the stand, the stock with its slider and the winch at the back
                    for (x, z) in [(-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
                        b.add(
                            &bar(v(x * k, 0.0, z * k), v(0.0, 1.2 * k, 0.0), 0.16 * k),
                            dark,
                        );
                    }
                    b.add(
                        &prims::boxed(0.4 * k, 0.3 * k, 3.2 * k).translate(0.0, 1.35 * k, 0.2 * k),
                        wood,
                    );
                    b.add(
                        &prims::cylinder(0.14 * k, 0.14 * k, 0.9 * k, 6)
                            .rotate_z(std::f32::consts::FRAC_PI_2)
                            .translate(0.0, 1.35 * k, -1.3 * k),
                        rgb(IRON),
                    );
                    b.add(
                        &bar(v(0.0, 1.55 * k, 1.6 * k), v(0.0, 1.55 * k, -k), 0.08 * k),
                        rgb(IRON),
                    );
                    if kind == Kind::Balista {
                        banner(b, v(0.7, 0.1, -0.7), look);
                    }
                }),
                arm: Some((
                    mesh(|b| {
                        // the bow's two arms and the string drawn back
                        for s in [-1.0, 1.0] {
                            b.add(
                                &bar(v(0.0, 0.0, 0.0), v(s * 1.4 * k, 0.0, -0.5 * k), 0.12 * k),
                                dark,
                            );
                            b.add(
                                &bar(v(s * 1.4 * k, 0.0, -0.5 * k), v(0.0, 0.05, -1.4 * k), 0.03),
                                rgb(ROPE),
                            );
                        }
                    }),
                    [0.0, 1.5 * k, 1.4 * k],
                )),
            }
        }
        Kind::TorreAsedio => EngineModel {
            body: mesh(|b| {
                // a tall timber tower on six wheels, hides hung on its front and sides
                let (w, d, h) = (4.4, 4.4, 14.0);
                for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    b.add(
                        &bar(
                            v(x * w / 2.0, 0.6, z * d / 2.0),
                            v(x * w / 2.2, h, z * d / 2.2),
                            0.34,
                        ),
                        dark,
                    );
                }
                let mut y = 0.6;
                while y < h {
                    for s in [-1.0, 1.0] {
                        b.add(
                            &bar(
                                v(-w / 2.0, y, s * d / 2.0),
                                v(w / 2.0, y, s * d / 2.0),
                                0.22,
                            ),
                            wood,
                        );
                        b.add(
                            &bar(
                                v(s * w / 2.0, y, -d / 2.0),
                                v(s * w / 2.0, y, d / 2.0),
                                0.22,
                            ),
                            wood,
                        );
                    }
                    y += 2.8;
                }
                // hides: front and both sides, up to the fighting floor
                b.add(
                    &prims::boxed(w * 0.96, h - 3.2, 0.12).translate(
                        0.0,
                        (h - 3.2) / 2.0 + 0.9,
                        d / 2.0 + 0.1,
                    ),
                    rgb(HIDE),
                );
                for s in [-1.0, 1.0] {
                    b.add(
                        &prims::boxed(0.12, h - 3.2, d * 0.96).translate(
                            s * (w / 2.0 + 0.1),
                            (h - 3.2) / 2.0 + 0.9,
                            0.0,
                        ),
                        rgb(0x8c6a46),
                    );
                }
                // the floor at the top and its parapet
                b.add(
                    &prims::boxed(w, 0.25, d).translate(0.0, h - 2.6, 0.0),
                    rgb(WOOD),
                );
                for s in [-1.0, 1.0] {
                    b.add(
                        &prims::boxed(0.15, 1.2, d).translate(s * w / 2.0, h - 1.9, 0.0),
                        wood,
                    );
                }
                for x in [-1.6, 0.0, 1.6] {
                    for z in [-1.6, 1.6] {
                        wheel(b, x, 0.6, z, 0.6);
                    }
                }
                banner(b, v(-w / 2.0, h - 2.5, -d / 2.0), look);
            }),
            arm: Some((
                mesh(|b| {
                    // the bridge, up against the tower's front (it turns down, forwards)
                    b.add(
                        &prims::boxed(3.6, 4.2, 0.22).translate(0.0, 2.1, 0.0),
                        rgb(WOOD),
                    );
                    for x in [-1.6, 0.0, 1.6] {
                        b.add(&prims::boxed(0.12, 4.2, 0.12).translate(x, 2.1, 0.17), dark);
                    }
                }),
                [0.0, 11.4, 2.4],
            )),
        },
        Kind::Ariete => EngineModel {
            body: mesh(|b| {
                // a shed on wheels: posts, a pitched roof of hides
                let (w, l, h) = (2.6, 6.2, 2.6);
                for x in [-1.0, 1.0] {
                    for z in [-1.0, 0.0, 1.0] {
                        b.add(
                            &bar(
                                v(x * w / 2.0, 0.5, z * l / 2.0),
                                v(x * w / 2.0, h, z * l / 2.0),
                                0.2,
                            ),
                            dark,
                        );
                    }
                    b.add(
                        &bar(v(x * w / 2.0, h, -l / 2.0), v(x * w / 2.0, h, l / 2.0), 0.2),
                        wood,
                    );
                    b.add(
                        &bar(
                            v(x * w / 2.0, 0.5, -l / 2.0),
                            v(x * w / 2.0, 0.5, l / 2.0),
                            0.24,
                        ),
                        dark,
                    );
                    for z in [-2.2, 2.2] {
                        wheel(b, x * (w / 2.0 + 0.2), 0.55, z, 0.55);
                    }
                }
                for s in [-1.0, 1.0] {
                    let roof = prims::boxed(w * 0.75, 0.12, l + 0.4)
                        .rotate_z(s * 0.6)
                        .translate(s * w * 0.28, h + 0.55, 0.0);
                    b.add(&roof, rgb(HIDE));
                }
                b.add(
                    &bar(
                        v(0.0, h + 1.05, -l / 2.0 - 0.2),
                        v(0.0, h + 1.05, l / 2.0 + 0.2),
                        0.2,
                    ),
                    dark,
                );
                banner(b, v(0.0, h + 1.1, -l / 2.0), look);
            }),
            arm: Some((
                mesh(|b| {
                    // the log on its chains, an iron head
                    b.add(
                        &bar(v(0.0, -1.2, -3.0), v(0.0, -1.2, 3.0), 0.42),
                        rgb(WOOD_DARK),
                    );
                    b.add(
                        &prims::cylinder(0.32, 0.46, 0.6, 8)
                            .rotate_x(std::f32::consts::FRAC_PI_2)
                            .translate(0.0, -1.2, 3.3),
                        rgb(IRON),
                    );
                    for z in [-1.5, 1.5] {
                        b.add(&bar(v(0.0, 0.0, z), v(0.0, -1.2, z), 0.05), rgb(IRON));
                    }
                }),
                [0.0, 2.5, 0.0],
            )),
        },
        Kind::Estacas => EngineModel {
            body: mesh(|b| {
                // two staggered rows of sharpened stakes, leaning out (to +z)
                let n = 14;
                for k in 0..n {
                    let x = -4.4 + 8.8 * k as f32 / (n - 1) as f32;
                    let z = if k % 2 == 0 { 0.3 } else { -0.3 };
                    let lean = 0.65 + (k % 3) as f32 * 0.08;
                    let s = prims::cylinder(0.0, 0.11, 2.0, 5)
                        .translate(0.0, 1.0, 0.0)
                        .rotate_x(lean)
                        .translate(x, 0.0, z);
                    b.add(&s, if k % 4 == 0 { dark } else { wood });
                }
                b.add(&bar(v(-4.5, 0.25, -0.2), v(4.5, 0.25, -0.2), 0.16), dark);
            }),
            arm: None,
        },
        Kind::Empalizada => EngineModel {
            body: mesh(|b| {
                // logs side by side, pointed tops, a rail behind
                let n = 22;
                for k in 0..n {
                    let x = -4.0 + 8.0 * k as f32 / (n - 1) as f32;
                    let h = 3.0 + ((k * 7) % 5) as f32 * 0.12;
                    b.add(
                        &prims::cylinder(0.18, 0.2, h, 6).translate(x, h / 2.0, 0.0),
                        if k % 3 == 0 { dark } else { wood },
                    );
                    b.add(&prims::cone(0.18, 0.5, 6).translate(x, h + 0.25, 0.0), wood);
                }
                for y in [0.9, 2.2] {
                    b.add(&bar(v(-4.1, y, -0.3), v(4.1, y, -0.3), 0.16), dark);
                }
            }),
            arm: None,
        },
    }
}

/// A relic lying on the battlefield: a low stone plinth and a crystal over it (it glows).
pub fn relic() -> (Mesh3D, Mesh3D) {
    let plinth = mesh(|b| {
        b.add(
            &prims::cylinder(0.75, 0.95, 0.5, 8).translate(0.0, 0.25, 0.0),
            rgb(STONE),
        );
        b.add(
            &prims::cylinder(0.55, 0.65, 0.18, 8).translate(0.0, 0.59, 0.0),
            rgb(0x6f6478),
        );
    });
    let crystal = mesh(|b| {
        b.add(
            &prims::icosahedron(0.42, 0)
                .scale(0.75, 1.6, 0.75)
                .translate(0.0, 1.45, 0.0),
            rgb(0xffd27a),
        );
    });
    (plinth, crystal)
}

/// A stone in the air (a catapult's; a trebuchet's is scaled up) and a bolt (along +z).
pub fn stone() -> Mesh3D {
    mesh(|b| b.add(&prims::dodecahedron(0.45, 0).jagged(0.12, 3), rgb(STONE)))
}

pub fn bolt() -> Mesh3D {
    mesh(|b| {
        b.add(
            &bar(v(0.0, 0.0, -1.1), v(0.0, 0.0, 0.9), 0.07),
            rgb(WOOD_DARK),
        );
        b.add(
            &prims::cone(0.1, 0.35, 5)
                .rotate_x(std::f32::consts::FRAC_PI_2)
                .translate(0.0, 0.0, 1.05),
            rgb(IRON),
        );
    })
}

/// How far an engine's arm is turned (radians about x), `fired` seconds after its shot and
/// `reload` long to wind back: the throw (fast), the rest, the winding (slow).
pub fn arm_angle(kind: Kind, fired: f32, reload: f32) -> f32 {
    match kind {
        Kind::Catapulta => {
            // cocked lying back (0), thrown up against the stop
            let throw = (fired / 0.25).clamp(0.0, 1.0);
            let back = ((fired - 1.0) / (reload * 0.8).max(0.5)).clamp(0.0, 1.0);
            1.7 * (throw * (1.0 - back))
        }
        Kind::Trabuquete => {
            let throw = (fired / 0.6).clamp(0.0, 1.0);
            let back = ((fired - 2.0) / (reload * 0.75).max(0.5)).clamp(0.0, 1.0);
            // cocked: the long end down behind (-0.7); thrown: over the top (+2.4)
            -0.75 + 3.1 * (throw * (1.0 - back)).powf(0.8)
        }
        Kind::Balista | Kind::Escorpion => {
            let snap = (fired / 0.12).clamp(0.0, 1.0);
            let back = ((fired - 0.4) / (reload * 0.7).max(0.3)).clamp(0.0, 1.0);
            0.25 * snap * (1.0 - back)
        }
        Kind::Ariete => {
            // the swing back and the blow
            let t = (fired / 0.9).clamp(0.0, 1.0);
            -0.35 * (t * std::f32::consts::PI).sin()
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_has_a_model() {
        for k in Kind::ALL {
            let m = model(k, Faction::Reina);
            assert!(m.body.vertices.len() > 30, "{k:?}");
        }
        assert!(!stone().vertices.is_empty() && !bolt().vertices.is_empty());
    }
}
