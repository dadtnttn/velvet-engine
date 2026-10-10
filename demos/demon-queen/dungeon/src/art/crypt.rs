//! La Cripta (web prototype `galleries/crypt.js`): the burial hall of the old
//! order — a knight's tomb before a winged-skull shrine, sealed, plundered
//! and broken sarcophagi, ossuary bays, grave slabs, candle stands, dust,
//! webs, chains, urns and scattered bones.
use super::kit::*;
use super::stage::*;
use super::themekit::skull;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const DUST: C3 = [0.36, 0.34, 0.38];
const PALL: C3 = [0.36, 0.05, 0.06];
const GILT: C3 = [0.82, 0.56, 0.2];
const PI: f32 = std::f32::consts::PI;

/// Tiny hooded mourner (pleurant) standing in a tomb niche.
fn mourner(m: &mut Mesh3D, base: Vec3, out: Vec3, stone: C3) {
    lathe(
        m,
        base,
        VY,
        &[
            (0.0, 0.055),
            (0.02, 0.06),
            (0.12, 0.05),
            (0.17, 0.04),
            (0.19, 0.045),
            (0.22, 0.03),
            (0.25, 0.0),
        ],
        7,
        tint(stone, 1.12),
        true,
    );
    disc(
        m,
        base + Y(0.2) + out * 0.034,
        out,
        0.016,
        6,
        tint(stone, 0.45),
    );
}

/// The knight's tomb: plinth, arcaded chest of mourners, the armoured
/// effigy with sword and lion, a funeral pall, a wreath and candles.
pub fn knight_tomb(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    let st = |k: f32| tint(stone, k);
    chamfer_box(
        m,
        l(0.0, 0.07, 0.0),
        s,
        VY,
        f,
        v(1.0, 0.07, 1.55),
        0.03,
        st(0.7),
    );
    chamfer_box(
        m,
        l(0.0, 0.19, 0.0),
        s,
        VY,
        f,
        v(0.88, 0.05, 1.42),
        0.02,
        st(0.8),
    );
    chamfer_box(
        m,
        l(0.0, 0.62, 0.0),
        s,
        VY,
        f,
        v(0.62, 0.38, 1.18),
        0.03,
        st(0.86),
    );
    chamfer_box(
        m,
        l(0.0, 0.28, 0.0),
        s,
        VY,
        f,
        v(0.68, 0.04, 1.24),
        0.015,
        st(0.96),
    );
    chamfer_box(
        m,
        l(0.0, 0.97, 0.0),
        s,
        VY,
        f,
        v(0.68, 0.035, 1.24),
        0.012,
        st(1.0),
    );
    chamfer_box(
        m,
        l(0.0, 1.05, 0.0),
        s,
        VY,
        f,
        v(0.66, 0.05, 1.22),
        0.02,
        st(1.04),
    );
    kit_block(
        m,
        l(0.0, 1.106, 0.0),
        s,
        VY,
        f,
        v(0.6, 0.008, 1.16),
        st(1.1),
    );
    for x in [-0.64, 0.64] {
        for z in [-1.2, 1.2] {
            lathe(
                m,
                l(x, 0.31, z),
                VY,
                &[
                    (0.0, 0.06),
                    (0.03, 0.045),
                    (0.58, 0.04),
                    (0.62, 0.058),
                    (0.64, 0.058),
                ],
                8,
                st(1.05),
                true,
            );
        }
    }
    for sgn in [-1.0, 1.0] {
        let out = s * sgn;
        for z in [-0.84, -0.28, 0.28, 0.84] {
            let c = l(sgn * 0.623, 0.5, z);
            let w = 0.2;
            quad(
                m,
                [
                    c + f * -w - Y(0.13),
                    c + f * w - Y(0.13),
                    c + f * w + Y(0.16),
                    c + f * -w + Y(0.16),
                ],
                out,
                st(0.5),
            );
            let apex = c + Y(0.16);
            for k in 0..8 {
                let a0 = PI * k as f32 / 8.0;
                let a1 = PI * (k + 1) as f32 / 8.0;
                tri(
                    m,
                    apex,
                    apex + f * (-a0.cos() * w) + Y(a0.sin() * w),
                    apex + f * (-a1.cos() * w) + Y(a1.sin() * w),
                    out,
                    st(0.5),
                );
            }
            for k in 0..=8 {
                let a = PI * k as f32 / 8.0;
                let p = apex + f * (-a.cos() * (w + 0.02)) + Y(a.sin() * (w + 0.02)) + out * 0.004;
                kit_block(m, p, f, VY, s, v(0.018, 0.018, 0.012), st(1.12));
            }
            mourner(m, c - Y(0.13) + out * 0.03, out, stone);
        }
        for z in [-1.12, -0.56, 0.0, 0.56, 1.12] {
            bar(
                m,
                l(sgn * 0.628, 0.34, z),
                l(sgn * 0.628, 0.86, z),
                0.018,
                0.012,
                s,
                st(1.14),
            );
        }
    }
    let shield_c = l(0.0, 0.62, 1.186);
    let sh = [
        shield_c + s * -0.2 + Y(0.2),
        shield_c + s * 0.2 + Y(0.2),
        shield_c + s * 0.2 - Y(0.06),
        shield_c + s * -0.2 - Y(0.06),
    ];
    quad(m, sh, f, st(1.14));
    tri(m, sh[3], sh[2], shield_c - Y(0.28), f, st(1.14));
    kit_block(
        m,
        shield_c + f * 0.01 + Y(0.02),
        s,
        VY,
        f,
        v(0.03, 0.2, 0.008),
        GILT,
    );
    kit_block(
        m,
        shield_c + f * 0.01 + Y(0.06),
        s,
        VY,
        f,
        v(0.15, 0.03, 0.008),
        GILT,
    );
    for k in 0..4 {
        let w = [0.42, 0.32, 0.46, 0.24][k];
        kit_block(
            m,
            l(0.0, 0.78 - k as f32 * 0.1, -1.184),
            s,
            VY,
            f,
            v(w, 0.012, 0.004),
            st(0.5),
        );
    }
    // The effigy: the knight in armour.
    let y0 = 1.112;
    chamfer_box(
        m,
        l(0.0, y0 + 0.045, 0.86),
        s,
        VY,
        f,
        v(0.22, 0.045, 0.15),
        0.03,
        st(1.14),
    );
    for x in [-0.2, 0.2] {
        for z in [0.73, 0.99] {
            lathe(
                m,
                l(x, y0 + 0.02, z),
                VY,
                &[(0.0, 0.018), (0.03, 0.01)],
                5,
                st(1.2),
                true,
            );
        }
    }
    let helm = l(0.0, y0 + 0.2, 0.84);
    lathe(
        m,
        helm - f * 0.13,
        f,
        &[
            (0.0, 0.02),
            (0.03, 0.1),
            (0.1, 0.122),
            (0.24, 0.12),
            (0.28, 0.1),
            (0.29, 0.0),
        ],
        12,
        st(1.1),
        true,
    );
    kit_block(
        m,
        helm + Y(0.12) + f * 0.02,
        s,
        VY,
        f,
        v(0.085, 0.006, 0.012),
        st(0.3),
    );
    for x in [-0.03, 0.0, 0.03] {
        disc(
            m,
            helm + Y(0.118) + f * -0.04 + s * x,
            VY,
            0.007,
            5,
            st(0.35),
        );
    }
    bar(
        m,
        helm + Y(0.12) + f * 0.13,
        helm + Y(0.12) - f * 0.1,
        0.012,
        0.02,
        s,
        st(1.2),
    );
    const TORSO: [(f32, f32, f32); 6] = [
        (0.66, 0.16, 0.07),
        (0.6, 0.25, 0.11),
        (0.5, 0.27, 0.13),
        (0.3, 0.25, 0.145),
        (0.12, 0.2, 0.12),
        (0.02, 0.19, 0.1),
    ];
    grid_surface(
        m,
        TORSO.len() - 1,
        12,
        &|i, j| {
            let (z, w, h) = TORSO[i];
            let a = (j as f32 / 12.0) * PI;
            l(a.cos() * w, y0 + 0.02 + a.sin() * h, z)
        },
        &|_, j| st(1.02 + 0.08 * ((j as f32 / 12.0) * PI).sin()),
        false,
        Some(&|p, i, _| p - l(0.0, y0, TORSO[i].0)),
        None,
    );
    bar(
        m,
        l(0.0, y0 + 0.165, 0.6),
        l(0.0, y0 + 0.14, 0.08),
        0.01,
        0.01,
        VY,
        st(1.2),
    );
    for sgn in [-1.0, 1.0] {
        lathe(
            m,
            l(sgn * 0.25, y0 + 0.05, 0.55),
            norm(s * sgn + Y(0.4)),
            &[(0.0, 0.11), (0.05, 0.09), (0.09, 0.0)],
            9,
            st(1.08),
            true,
        );
        tube_c(
            m,
            &[
                l(sgn * 0.27, y0 + 0.08, 0.5),
                l(sgn * 0.29, y0 + 0.1, 0.36),
                l(sgn * 0.27, y0 + 0.11, 0.24),
            ],
            0.06,
            7,
            st(1.02),
        );
        tube(
            m,
            &[
                l(sgn * 0.27, y0 + 0.11, 0.24),
                l(sgn * 0.17, y0 + 0.19, 0.32),
                l(sgn * 0.05, y0 + 0.22, 0.4),
            ],
            &|t| 0.055 - t * 0.01,
            7,
            &|_, _| st(1.04),
        );
        chamfer_box(
            m,
            l(sgn * 0.045, y0 + 0.235, 0.42),
            s,
            VY,
            f,
            v(0.04, 0.035, 0.055),
            0.015,
            st(1.1),
        );
        kit_block(
            m,
            l(sgn * 0.1, y0 + 0.09, -0.02),
            s,
            VY,
            f,
            v(0.11, 0.07, 0.06),
            st(0.98),
        );
        tube(
            m,
            &[
                l(sgn * 0.1, y0 + 0.08, -0.1),
                l(sgn * 0.1, y0 + 0.08, -0.45),
                l(sgn * 0.1, y0 + 0.07, -0.86),
            ],
            &|t| 0.078 - t * 0.022,
            8,
            &|_, _| st(1.02),
        );
        lathe(
            m,
            l(sgn * 0.1, y0 + 0.08, -0.46),
            VY,
            &[(0.0, 0.075), (0.06, 0.06), (0.09, 0.0)],
            8,
            st(1.12),
            true,
        );
        chamfer_box(
            m,
            l(sgn * 0.1, y0 + 0.12, -0.93),
            s,
            VY,
            f,
            v(0.05, 0.08, 0.05),
            0.02,
            st(1.06),
        );
    }
    for k in 0..3 {
        let z = 0.0 - k as f32 * 0.07;
        grid_surface(
            m,
            1,
            10,
            &|i, j| {
                let a = (j as f32 / 10.0) * PI;
                let w = 0.22 + k as f32 * 0.008;
                l(
                    a.cos() * w,
                    y0 + 0.02 + a.sin() * (0.11 - k as f32 * 0.012),
                    z - i as f32 * 0.07,
                )
            },
            &|_, _| st(0.96 + k as f32 * 0.04),
            false,
            Some(&|p, _, _| p - l(0.0, y0, z)),
            None,
        );
    }
    // The sword, lying from the folded hands to the knees.
    bar(
        m,
        l(0.0, y0 + 0.24, 0.52),
        l(0.0, y0 + 0.24, 0.36),
        0.016,
        0.016,
        VY,
        st(1.15),
    );
    lathe(
        m,
        l(0.0, y0 + 0.24, 0.52),
        f,
        &[(0.0, 0.03), (0.02, 0.034), (0.045, 0.0)],
        8,
        st(1.2),
        true,
    );
    bar(
        m,
        l(-0.15, y0 + 0.23, 0.345),
        l(0.15, y0 + 0.23, 0.345),
        0.018,
        0.022,
        VY,
        st(1.2),
    );
    let blade = [
        l(-0.04, y0 + 0.2, 0.33),
        l(0.04, y0 + 0.2, 0.33),
        l(0.03, y0 + 0.19, -0.52),
        l(-0.03, y0 + 0.19, -0.52),
    ];
    quad(m, blade, VY, st(1.16));
    tri(
        m,
        blade[3],
        blade[2],
        l(0.0, y0 + 0.185, -0.62),
        VY,
        st(1.16),
    );
    bar(
        m,
        l(0.0, y0 + 0.203, 0.3),
        l(0.0, y0 + 0.193, -0.5),
        0.006,
        0.002,
        VY,
        st(0.85),
    );
    // The lion at his feet.
    let lion = l(0.0, y0 + 0.02, -1.03);
    lathe(
        m,
        lion - s * 0.2,
        s,
        &[
            (0.0, 0.02),
            (0.04, 0.09),
            (0.2, 0.11),
            (0.34, 0.08),
            (0.38, 0.0),
        ],
        9,
        st(1.05),
        true,
    );
    lathe(
        m,
        lion + s * 0.22 + Y(0.08),
        VY,
        &[
            (-0.06, 0.02),
            (-0.04, 0.07),
            (0.04, 0.075),
            (0.08, 0.03),
            (0.09, 0.0),
        ],
        9,
        st(1.1),
        true,
    );
    tube_ring(m, lion + s * 0.2 + Y(0.1), s, 0.085, 0.03, 12, st(1.0));
    for x in [-0.14, 0.1] {
        kit_block(
            m,
            lion + s * x + f * 0.1 + Y(0.02),
            s,
            VY,
            f,
            v(0.04, 0.025, 0.05),
            st(1.08),
        );
    }
    // A funeral pall falls over the foot corner, gilded at its hem.
    const PC: usize = 10;
    const PR: usize = 9;
    let pall = |i: usize, j: usize| {
        let z = lerp(-1.2, -0.42, j as f32 / PC as f32);
        let t = i as f32 / PR as f32;
        let fold = (j as f32 * 1.9 + t * 1.2).sin() * 0.02 * t;
        if t < 0.35 {
            return l(
                lerp(0.28, 0.67, t / 0.35),
                1.118 + 0.004 * (j as f32 * 2.1).sin(),
                z,
            );
        }
        let drop = (t - 0.35) / 0.65;
        l(
            0.675 + fold + 0.02 * drop,
            1.1 - drop * (0.52 + 0.08 * (j as f32 * 0.9).sin()),
            z + fold * 0.6,
        )
    };
    let center_pall = l(0.3, 0.6, -0.8);
    grid_surface(
        m,
        PR,
        PC,
        &pall,
        &|i, j| {
            tint(
                PALL,
                0.85 + 0.2 * (j as f32 * 1.9 + (i as f32 / PR as f32) * 1.2).sin(),
            )
        },
        false,
        Some(&|p, _, _| p - center_pall + Y(0.2)),
        Some(&|_, _| tint(PALL, 0.5)),
    );
    tube_c(
        m,
        &(0..=PC).map(|j| pall(PR, j)).collect::<Vec<_>>(),
        0.012,
        5,
        GILT,
    );
    tube_c(
        m,
        &(0..=PR).map(|i| pall(i, PC)).collect::<Vec<_>>(),
        0.01,
        5,
        GILT,
    );
    // A laurel wreath and candles on the plinth ledge.
    let wreath = l(0.0, 0.26, -1.32);
    tube_ring(m, wreath, VY, 0.14, 0.026, 18, [0.16, 0.24, 0.1]);
    for k in 0..18 {
        let a = k as f32 * TAU / 18.0;
        let d = v(a.cos(), 0.0, a.sin());
        let p = wreath + d * 0.14 + Y(0.025);
        let t = v(-d.z, 0.0, d.x);
        quad(
            m,
            [p - t * 0.03, p + d * 0.03, p + t * 0.03, p - d * 0.03],
            VY,
            tint([0.2, 0.32, 0.12], 0.8 + (k % 3) as f32 * 0.12),
        );
        if k % 4 == 0 {
            disc(m, p + Y(0.012), VY, 0.012, 5, [0.6, 0.08, 0.06]);
        }
    }
    for (x, z) in [(0.78, 1.32), (-0.78, 1.32), (0.78, -1.32), (-0.78, -1.32)] {
        candle_cluster(m, glow, l(x, 0.24, z), 3, 0.08, rng, WAX);
    }
}

/// A sarcophagus: sealed with a carved cross, plundered open, or broken.
pub fn crypt_sarcophagus(
    m: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    variant: usize,
    stone: C3,
    rng: &mut ArtRng,
) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    // Every shade draws from the stream, as in the prototype.
    let st = |rng: &mut ArtRng, k: f32| tint(stone, k * rng.range(0.96, 1.04));
    let c = st(rng, 0.72);
    chamfer_box(m, l(0.0, 0.06, 0.0), s, VY, f, v(0.6, 0.06, 1.12), 0.025, c);
    if variant == 0 {
        let c = st(rng, 0.84);
        chamfer_box(m, l(0.0, 0.48, 0.0), s, VY, f, v(0.5, 0.36, 1.0), 0.03, c);
        let c = st(rng, 0.98);
        chamfer_box(
            m,
            l(0.0, 0.81, 0.0),
            s,
            VY,
            f,
            v(0.55, 0.03, 1.05),
            0.012,
            c,
        );
    } else {
        // Opened: a hollow chest, four walls round a cavity half a metre deep.
        for sgn in [-1.0, 1.0] {
            let c = st(rng, 0.84);
            chamfer_box(
                m,
                l(sgn * 0.465, 0.48, 0.0),
                s,
                VY,
                f,
                v(0.035, 0.36, 1.0),
                0.012,
                c,
            );
            let c = st(rng, 0.8);
            chamfer_box(
                m,
                l(0.0, 0.48, sgn * 0.965),
                s,
                VY,
                f,
                v(0.43, 0.36, 0.035),
                0.012,
                c,
            );
            let c = st(rng, 0.98);
            kit_block(m, l(sgn * 0.5, 0.81, 0.0), s, VY, f, v(0.05, 0.03, 1.05), c);
            let c = st(rng, 0.98);
            kit_block(m, l(0.0, 0.81, sgn * 1.0), s, VY, f, v(0.45, 0.03, 0.05), c);
        }
        let c = st(rng, 0.42);
        chamfer_box(m, l(0.0, 0.26, 0.0), s, VY, f, v(0.43, 0.08, 0.93), 0.01, c);
        blob(
            m,
            l(0.0, 0.34, 0.0),
            0.4,
            tint(DUST, 0.7),
            rng,
            0.004,
            12,
            0.3,
        );
    }
    let c = st(rng, 0.95);
    chamfer_box(
        m,
        l(0.0, 0.15, 0.0),
        s,
        VY,
        f,
        v(0.55, 0.035, 1.05),
        0.012,
        c,
    );
    for sgn in [-1.0, 1.0] {
        for z in [-0.62, 0.0, 0.62] {
            let c = l(sgn * 0.503, 0.48, z);
            let shade = st(rng, 0.62);
            quad(
                m,
                [
                    c + f * -0.24 - Y(0.22),
                    c + f * 0.24 - Y(0.22),
                    c + f * 0.24 + Y(0.22),
                    c + f * -0.24 + Y(0.22),
                ],
                s * sgn,
                shade,
            );
            if z == 0.0 {
                let shade = st(rng, 1.1);
                kit_block(
                    m,
                    c + s * (sgn * 0.006),
                    f,
                    VY,
                    s,
                    v(0.025, 0.17, 0.006),
                    shade,
                );
                let shade = st(rng, 1.1);
                kit_block(
                    m,
                    c + s * (sgn * 0.006) + Y(0.06),
                    f,
                    VY,
                    s,
                    v(0.11, 0.025, 0.006),
                    shade,
                );
            } else {
                let shade = st(rng, 1.05);
                disc(m, c + s * (sgn * 0.004), s * sgn, 0.1, 12, shade);
                let shade = st(rng, 0.7);
                disc(m, c + s * (sgn * 0.006), s * sgn, 0.07, 12, shade);
            }
        }
    }
    let mut lid = empty();
    let c = st(rng, 1.0);
    chamfer_box(
        &mut lid,
        v(0.0, 0.06, 0.0),
        VX,
        VY,
        VZ,
        v(0.55, 0.06, 1.07),
        0.02,
        c,
    );
    let c = st(rng, 1.06);
    quad(
        &mut lid,
        [
            v(-0.5, 0.12, -1.02),
            v(-0.5, 0.12, 1.02),
            v(0.0, 0.2, 1.02),
            v(0.0, 0.2, -1.02),
        ],
        norm(v(-0.3, 1.0, 0.0)),
        c,
    );
    let c = st(rng, 0.96);
    quad(
        &mut lid,
        [
            v(0.5, 0.12, -1.02),
            v(0.5, 0.12, 1.02),
            v(0.0, 0.2, 1.02),
            v(0.0, 0.2, -1.02),
        ],
        norm(v(0.3, 1.0, 0.0)),
        c,
    );
    let c = st(rng, 0.9);
    tri(
        &mut lid,
        v(-0.5, 0.12, 1.02),
        v(0.5, 0.12, 1.02),
        v(0.0, 0.2, 1.02),
        VZ,
        c,
    );
    let c = st(rng, 0.9);
    tri(
        &mut lid,
        v(-0.5, 0.12, -1.02),
        v(0.5, 0.12, -1.02),
        v(0.0, 0.2, -1.02),
        -VZ,
        c,
    );
    let c = st(rng, 1.16);
    kit_block(
        &mut lid,
        v(0.0, 0.215, 0.1),
        VX,
        VY,
        VZ,
        v(0.03, 0.018, 0.6),
        c,
    );
    let c = st(rng, 1.16);
    kit_block(
        &mut lid,
        v(0.0, 0.215, 0.38),
        VX,
        VY,
        VZ,
        v(0.26, 0.018, 0.03),
        c,
    );
    if variant == 0 {
        append(
            m,
            &lid,
            &pose_mapper(l(0.0, 0.84, 0.0), yaw, 0.0, 0.0, 1.0),
            [1.0; 3],
        );
    } else if variant == 1 {
        // Plundered: the lid shoved half off, one corner on the floor.
        append(
            m,
            &lid,
            &pose_mapper(l(0.72, 0.48, 0.18), yaw + 0.22, 0.0, 0.55, 1.0),
            [1.0; 3],
        );
        skull(
            m,
            l(-0.12, 0.43, 0.62),
            0.95,
            yaw + PI + 0.4,
            -0.9,
            tint(BONE, 0.85),
        );
        bar(
            m,
            l(0.02, 0.352, 0.5),
            l(-0.04, 0.352, -0.4),
            0.016,
            0.014,
            VY,
            tint(BONE, 0.74),
        );
        for k in 0..5 {
            bar(
                m,
                l(-0.25 + k as f32 * 0.1, 0.36, 0.2),
                l(-0.2 + k as f32 * 0.09, 0.365, -0.25),
                0.014,
                0.014,
                VY,
                tint(BONE, 0.8),
            );
        }
        for sgn in [-1.0, 1.0] {
            bar(
                m,
                l(sgn * 0.14, 0.355, -0.45),
                l(sgn * 0.2, 0.355, -0.86),
                0.018,
                0.016,
                VY,
                tint(BONE, 0.78),
            );
        }
        corner_web(
            m,
            l(0.42, 0.8, 0.92),
            norm(-f + Y(-0.2)),
            norm(-s + Y(-0.2)),
            0.35,
            rng,
            [0.78, 0.78, 0.8],
        );
    } else {
        // Broken: the lid cracked in two, one half fallen against the chest.
        let mut half = empty();
        let c = st(rng, 1.0);
        chamfer_box(
            &mut half,
            v(0.0, 0.06, 0.0),
            VX,
            VY,
            VZ,
            v(0.55, 0.06, 0.52),
            0.02,
            c,
        );
        let c = st(rng, 1.14);
        kit_block(
            &mut half,
            v(0.0, 0.14, 0.1),
            VX,
            VY,
            VZ,
            v(0.03, 0.02, 0.3),
            c,
        );
        append(
            m,
            &half,
            &pose_mapper(l(0.0, 0.84, 0.5), yaw, 0.05, 0.04, 1.0),
            [1.0; 3],
        );
        append(
            m,
            &half,
            &pose_mapper(l(0.86, 0.34, -0.5), yaw + 0.3, 0.2, 1.05, 1.0),
            [1.0; 3],
        );
        for _ in 0..4 {
            let (x, z) = (rng.range(-0.3, 0.3), rng.range(-0.8, -0.15));
            let size = v(
                rng.range(0.05, 0.1),
                rng.range(0.03, 0.06),
                rng.range(0.05, 0.09),
            );
            let c = st(rng, 0.85);
            let phase = rng.range(0.0, TAU);
            rock(m, l(x, 0.34, z), size, c, phase);
        }
        for _ in 0..7 {
            let (x, z) = (rng.range(0.6, 1.3), rng.range(-1.1, 0.2));
            let size = v(
                rng.range(0.05, 0.12),
                rng.range(0.04, 0.09),
                rng.range(0.05, 0.1),
            );
            let c = st(rng, 0.9);
            let phase = rng.range(0.0, TAU);
            rock(m, l(x, 0.0, z), size, c, phase);
        }
    }
    blob(
        m,
        l(0.0, 0.0, 0.0),
        1.25,
        tint(DUST, 1.0),
        rng,
        0.005,
        14,
        0.3,
    );
}

/// Ossuary shelves in a wall bay: skulls over stacked long bones, and a
/// middle niche with a skull on a cushion by candles.
pub fn ossuary(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    height: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let w = width * 0.5;
    let h = height;
    let d = 0.42;
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    let st = |k: f32| tint(stone, k);
    quad(
        m,
        [
            l(-w, 0.0, 0.02),
            l(w, 0.0, 0.02),
            l(w, h, 0.02),
            l(-w, h, 0.02),
        ],
        inward,
        [0.035, 0.03, 0.035],
    );
    for sgn in [-1.0, 1.0] {
        chamfer_box(
            m,
            l(sgn * (w + 0.1), h * 0.5, d * 0.5),
            tangent,
            VY,
            inward,
            v(0.12, h * 0.5, d * 0.5 + 0.02),
            0.02,
            st(0.9),
        );
    }
    chamfer_box(
        m,
        l(0.0, h + 0.08, d * 0.5),
        tangent,
        VY,
        inward,
        v(w + 0.26, 0.09, d * 0.5 + 0.05),
        0.02,
        st(1.0),
    );
    chamfer_box(
        m,
        l(0.0, 0.06, d * 0.5),
        tangent,
        VY,
        inward,
        v(w + 0.24, 0.06, d * 0.5 + 0.04),
        0.02,
        st(0.8),
    );
    skull(
        m,
        l(0.0, h + 0.1, d + 0.08),
        1.1,
        yaw_of(inward),
        -0.1,
        tint(stone, 1.3),
    );
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            l(sgn * 0.02, h + 0.02, d + 0.1),
            l(sgn * 0.2, h + 0.16, d + 0.1),
            0.02,
            0.02,
            inward,
            tint(stone, 1.25),
        );
    }
    let levels = (((h - 0.2) / 0.52).floor() as usize).max(3);
    let step = (h - 0.14) / levels as f32;
    let yaw = yaw_of(inward);
    for k in 0..levels {
        let y0 = 0.12 + k as f32 * step;
        kit_block(
            m,
            l(0.0, y0, d * 0.5),
            tangent,
            VY,
            inward,
            v(w, 0.025, d * 0.5),
            st(0.95),
        );
        let middle = k == 1 && w > 0.7;
        let style = k % 2;
        let mut x = -w + 0.1;
        while x <= w - 0.09 {
            if !(middle && x.abs() < 0.26) {
                if style == 0 {
                    let jx = rng.range(-0.015, 0.015);
                    let sc = rng.range(0.82, 0.96);
                    let turn = rng.range(-0.25, 0.25);
                    let tilt = rng.range(-0.15, 0.1);
                    let shade = rng.range(0.72, 0.98);
                    skull(
                        m,
                        l(x + jx, y0 + 0.1, d - 0.12),
                        sc,
                        yaw + turn,
                        tilt,
                        tint(BONE, shade),
                    );
                } else {
                    let rows = ((step - 0.08) / 0.07).floor().max(0.0) as usize;
                    for r in 0..rows {
                        let q = l(
                            x + (r % 2) as f32 * 0.035,
                            y0 + 0.06 + r as f32 * 0.068,
                            d - 0.06,
                        );
                        let bone = tint(BONE, rng.range(0.7, 0.95));
                        disc(m, q + tangent * -0.017, inward, 0.024, 6, bone);
                        disc(m, q + tangent * 0.017, inward, 0.022, 6, tint(bone, 0.95));
                        disc(m, q - inward * 0.01, inward, 0.028, 6, tint(bone, 0.6));
                    }
                }
            }
            x += if style == 0 { 0.19 } else { 0.1 };
        }
        if middle {
            chamfer_box(
                m,
                l(0.0, y0 + 0.06, d - 0.14),
                tangent,
                VY,
                inward,
                v(0.14, 0.035, 0.1),
                0.02,
                [0.36, 0.05, 0.06],
            );
            skull(
                m,
                l(0.0, y0 + 0.16, d - 0.14),
                1.05,
                yaw,
                0.0,
                tint(BONE, 0.95),
            );
            wax_candle(
                m,
                glow,
                l(0.2, y0 + 0.025, d - 0.1),
                0.03,
                0.18,
                rng,
                WAX,
                true,
            );
            wax_candle(
                m,
                glow,
                l(-0.19, y0 + 0.025, d - 0.12),
                0.025,
                0.11,
                rng,
                WAX,
                true,
            );
        }
    }
}

/// An inset grave slab with a grooved border and a carved emblem.
pub fn grave_slab(m: &mut Mesh3D, c: Vec3, yaw: f32, variant: usize, stone: C3, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, z: f32, y: f32| c + s * x + f * z + Y(y);
    kit_block(
        m,
        l(0.0, 0.0, 0.012),
        s,
        VY,
        f,
        v(0.38, 0.012, 0.74),
        tint(stone, rng.range(0.82, 0.95)),
    );
    let groove = [0.06, 0.004];
    for sgn in [-1.0, 1.0] {
        kit_block(
            m,
            l(sgn * (0.38 - groove[0]), 0.0, 0.0245),
            s,
            VY,
            f,
            v(0.008, groove[1], 0.74 - groove[0]),
            tint(stone, 0.45),
        );
        kit_block(
            m,
            l(0.0, sgn * (0.74 - groove[0]), 0.0245),
            s,
            VY,
            f,
            v(0.38 - groove[0], groove[1], 0.008),
            tint(stone, 0.45),
        );
    }
    let dark = tint(stone, 0.42);
    if variant == 0 {
        kit_block(m, l(0.0, 0.1, 0.0245), s, VY, f, v(0.02, 0.004, 0.36), dark);
        kit_block(
            m,
            l(0.0, 0.22, 0.0245),
            s,
            VY,
            f,
            v(0.14, 0.004, 0.02),
            dark,
        );
    } else if variant == 1 {
        for k in 0..12 {
            let a = k as f32 * TAU / 12.0;
            kit_block(
                m,
                l(a.cos() * 0.12, 0.25 + a.sin() * 0.12, 0.0245),
                s,
                VY,
                f,
                v(0.018, 0.004, 0.018),
                dark,
            );
        }
        for x in [-0.045, 0.045] {
            disc(m, l(x, 0.27, 0.026), VY, 0.028, 6, dark);
        }
        for k in 0..4 {
            kit_block(
                m,
                l(0.0, -0.12 - k as f32 * 0.1, 0.0245),
                s,
                VY,
                f,
                v(0.22 - k as f32 * 0.03, 0.004, 0.012),
                dark,
            );
        }
    } else {
        for k in 0..6 {
            kit_block(
                m,
                l(0.0, 0.4 - k as f32 * 0.14, 0.0245),
                s,
                VY,
                f,
                v([0.24, 0.18, 0.26, 0.2, 0.12, 0.22][k], 0.004, 0.014),
                dark,
            );
        }
    }
    if rng.chance(0.5) {
        let (x, z) = (rng.range(-0.3, 0.3), rng.range(-0.6, 0.6));
        let turn = rng.range(-1.0, 1.0);
        crack(
            m,
            l(x, z, 0.02),
            f * turn + s,
            0.5,
            rng,
            tint(stone, 0.35),
            0.0,
        );
    }
}

/// Tall iron candle stand: tripod feet, twisted shaft, drip pan, candles.
pub fn candle_stand(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    height: f32,
    arms: usize,
    rng: &mut ArtRng,
) {
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0 + 0.3;
        let d = v(a.cos(), 0.0, a.sin());
        tube_c(
            m,
            &bezier(base + Y(0.22), base + d * 0.2 + Y(0.12), base + d * 0.26, 6),
            0.016,
            5,
            IRON,
        );
        lathe(
            m,
            base + d * 0.26,
            VY,
            &[(0.0, 0.03), (0.02, 0.025), (0.03, 0.0)],
            5,
            tint(IRON, 1.3),
            true,
        );
    }
    lathe(
        m,
        base + Y(0.2),
        VY,
        &[
            (0.0, 0.03),
            (0.06, 0.022),
            (height - 0.28, 0.016),
            (height - 0.2, 0.02),
        ],
        6,
        tint(IRON, 1.2),
        false,
    );
    let mut y = 0.35;
    while y < height - 0.3 {
        tube_ring(m, base + Y(y), VY, 0.02, 0.008, 8, tint(IRON, 1.5));
        y += 0.22;
    }
    let top = base + Y(height);
    lathe(
        m,
        top - Y(0.03),
        VY,
        &[(0.0, 0.03), (0.02, 0.1), (0.035, 0.11), (0.04, 0.1)],
        10,
        tint(IRON, 1.3),
        true,
    );
    blob(
        m,
        top + Y(0.004),
        0.09,
        [0.9, 0.84, 0.66],
        rng,
        0.0,
        10,
        0.4,
    );
    let h = rng.range(0.14, 0.28);
    wax_candle(m, glow, top + Y(0.012), 0.036, h, rng, WAX, true);
    if arms > 1 {
        for sgn in [-1.0, 1.0] {
            let a = rng.range(0.0, TAU);
            let d = v(a.cos(), 0.0, a.sin()) * sgn;
            let tip = top - Y(0.22) + d * 0.2;
            tube_c(
                m,
                &bezier(top - Y(0.3), top - Y(0.34) + d * 0.16, tip, 6),
                0.012,
                5,
                IRON,
            );
            lathe(
                m,
                tip,
                VY,
                &[(0.0, 0.015), (0.02, 0.05), (0.03, 0.05)],
                8,
                tint(IRON, 1.3),
                true,
            );
            let h = rng.range(0.08, 0.2);
            wax_candle(m, glow, tip + Y(0.03), 0.028, h, rng, WAX, true);
        }
    }
    for k in 0..4 {
        kit_block(
            m,
            top - Y(0.08 + k as f32 * 0.05) + v(0.018, 0.0, 0.0),
            VX,
            VY,
            VZ,
            v(0.006, 0.03 + k as f32 * 0.01, 0.006),
            [0.9, 0.84, 0.66],
        );
    }
}

/// Hanging iron cage lantern with a candle inside.
pub fn cage_lantern(m: &mut Mesh3D, glow: &mut Mesh3D, top: Vec3, length: f32, rng: &mut ArtRng) {
    let bottom = chain(m, top, length, rng, IRON);
    let c = bottom - Y(0.2);
    lathe(
        m,
        c + Y(0.16),
        VY,
        &[(0.0, 0.14), (0.05, 0.1), (0.08, 0.03), (0.12, 0.012)],
        8,
        tint(IRON, 1.3),
        true,
    );
    lathe(
        m,
        c - Y(0.2),
        VY,
        &[(0.0, 0.03), (0.03, 0.12), (0.05, 0.13)],
        8,
        tint(IRON, 1.2),
        true,
    );
    for k in 0..8 {
        let a = k as f32 * TAU / 8.0;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            c + d * 0.125 - Y(0.16),
            c + d * 0.13 + Y(0.16),
            0.008,
            0.008,
            d,
            IRON,
        );
    }
    tube_ring(m, c, VY, 0.13, 0.008, 16, tint(IRON, 1.4));
    wax_candle(m, glow, c - Y(0.16), 0.04, 0.16, rng, WAX, true);
}

/// An arched shrine niche with a carved winged skull over candles.
pub fn crypt_shrine(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let hw = 0.85f32.min(width * 0.35);
    let spring = 1.55;
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    quad(
        m,
        [
            l(-hw, 0.5, 0.03),
            l(hw, 0.5, 0.03),
            l(hw, spring, 0.03),
            l(-hw, spring, 0.03),
        ],
        inward,
        [0.04, 0.035, 0.045],
    );
    for k in 0..10 {
        let a0 = PI * k as f32 / 10.0;
        let a1 = PI * (k + 1) as f32 / 10.0;
        tri(
            m,
            l(0.0, spring, 0.03),
            l(-a0.cos() * hw, spring + a0.sin() * hw, 0.03),
            l(-a1.cos() * hw, spring + a1.sin() * hw, 0.03),
            inward,
            [0.04, 0.035, 0.045],
        );
    }
    for sgn in [-1.0, 1.0] {
        chamfer_box(
            m,
            l(sgn * (hw + 0.13), (spring + 0.5) * 0.5 + 0.02, 0.14),
            tangent,
            VY,
            inward,
            v(0.13, (spring - 0.5) * 0.5 + 0.04, 0.14),
            0.02,
            tint(stone, 0.95),
        );
    }
    for k in 0..9 {
        let a0 = PI * k as f32 / 9.0;
        let a1 = PI * (k + 1) as f32 / 9.0;
        let am = (a0 + a1) * 0.5;
        let r = hw + 0.13;
        let p = l(-am.cos() * r, spring + am.sin() * r, 0.14);
        let tan = norm(tangent * am.sin() + Y(am.cos()));
        let rad = norm(tangent * -am.cos() + Y(am.sin()));
        chamfer_box(
            m,
            p,
            tan,
            rad,
            inward,
            v(
                r * (a1 - a0) * 0.5 - 0.01,
                if k == 4 { 0.16 } else { 0.13 },
                0.14,
            ),
            0.015,
            tint(stone, if k == 4 { 1.2 } else { 1.0 }),
        );
    }
    chamfer_box(
        m,
        l(0.0, 0.46, 0.2),
        tangent,
        VY,
        inward,
        v(hw + 0.28, 0.05, 0.22),
        0.015,
        tint(stone, 1.05),
    );
    chamfer_box(
        m,
        l(0.0, 0.22, 0.14),
        tangent,
        VY,
        inward,
        v(hw + 0.16, 0.22, 0.14),
        0.02,
        tint(stone, 0.8),
    );
    let heart = l(0.0, spring + 0.05, 0.08);
    skull(m, heart, 1.25, yaw_of(inward), 0.05, tint(stone, 1.25));
    for sgn in [-1.0, 1.0] {
        for k in 0..6 {
            let a = 0.25 + k as f32 * 0.22;
            let len = 0.46 - k as f32 * 0.04;
            let root = heart + tangent * (sgn * 0.12) + Y(0.02);
            let tip = root + tangent * (sgn * a.cos() * len) + Y(a.sin() * len * 0.7 - 0.08);
            let w = Y(0.035);
            tri(
                m,
                root + w,
                root - w,
                tip,
                inward,
                tint(stone, 1.1 + (k % 2) as f32 * 0.08),
            );
        }
    }
    candle_cluster(m, glow, l(0.0, 0.51, 0.2), 6, 0.2, rng, WAX);
    lathe(
        m,
        l(hw * 0.6, 0.51, 0.24),
        VY,
        &[(0.0, 0.04), (0.04, 0.08), (0.07, 0.085)],
        10,
        [0.3, 0.2, 0.1],
        true,
    );
}

/// Big two-handled burial urn with a lid.
pub fn burial_urn(m: &mut Mesh3D, base: Vec3, h: f32, clay: C3) {
    let shades = [0.7, 0.9, 1.05, 1.0, 0.9, 1.1, 1.0];
    lathe_fn(
        m,
        base,
        VY,
        &[
            (0.0, 0.1 * h),
            (0.06 * h, 0.2 * h),
            (0.35 * h, 0.3 * h),
            (0.62 * h, 0.26 * h),
            (0.8 * h, 0.14 * h),
            (0.86 * h, 0.16 * h),
            (0.9 * h, 0.13 * h),
        ],
        14,
        &|j| tint(clay, shades[j.min(6)]),
        true,
    );
    lathe(
        m,
        base + Y(0.9 * h),
        VY,
        &[
            (0.0, 0.13 * h),
            (0.05 * h, 0.1 * h),
            (0.1 * h, 0.03 * h),
            (0.13 * h, 0.0),
        ],
        10,
        tint(clay, 1.1),
        false,
    );
    tube_ring(
        m,
        base + Y(0.5 * h),
        VY,
        0.285 * h,
        0.012,
        20,
        tint(GILT, 0.7),
    );
    for sgn in [-1.0, 1.0] {
        let a = base + v(sgn * 0.2 * h, 0.78 * h, 0.0);
        tube_c(
            m,
            &bezier(
                a,
                a + v(sgn * 0.14 * h, 0.02 * h, 0.0),
                a + v(sgn * 0.08 * h, -0.2 * h, 0.0),
                6,
            ),
            0.018,
            5,
            tint(clay, 0.9),
        );
    }
}

// ---------------------------------------------------------------- planner

/// Plan the crypt into the plan held by the stage.
pub fn plan(s: &mut Stage) {
    let warm = [1.0, 0.66, 0.36];
    // 1. The knight's tomb before the first door-free wall, far wall first.
    let mut tomb: Option<(Vec3, f32, Edge)> = None;
    let mut walls = s.walls.clone();
    walls.sort_by_key(|w| mm(w.inward.dot(s.inward)));
    for wall in &walls {
        if wall
            .doors
            .iter()
            .any(|d| (d.p - wall.mid).dot(wall.tangent).abs() < 1.9)
        {
            continue;
        }
        let yaw = yaw_of(-wall.inward);
        let cands: Vec<Vec3> = [2.35, 2.7, 3.1]
            .iter()
            .map(|&d| wall.mid + wall.inward * d)
            .collect();
        if let Some(p) = s.place_row(&cands, yaw, 1.55, 1.0, 1.6, 0.9) {
            tomb = Some((p, yaw, wall.clone()));
            break;
        }
    }
    if let Some((tp, tyaw, twall)) = &tomb {
        let (tp, tyaw) = (*tp, *tyaw);
        s.piece(move |m, g, ctx| knight_tomb(m, g, tp, tyaw, ctx.stone, &mut ctx.rng));
        let (f, sd) = frame_of(tyaw);
        for (x, z) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let Some(q) = s.place(
                &[
                    tp + sd * (x * 1.28) + f * (z * 1.7),
                    tp + sd * (x * 1.4) + f * (z * 1.3),
                ],
                0.2,
                0.6,
                0.5,
            ) else {
                continue;
            };
            s.plan.solids.push((q, 0.2));
            let height = s.rng.range(1.25, 1.55);
            let arms = if z > 0.0 { 3 } else { 1 };
            s.piece(move |m, g, ctx| candle_stand(m, g, q, height, arms, &mut ctx.rng));
            s.light(q + Y(height + 0.5), warm, 0.95, 3.2);
        }
        let top = s.top(tp);
        let length = (s.height - 2.9).max(0.6);
        s.piece(move |m, g, ctx| cage_lantern(m, g, top, length, &mut ctx.rng));
        s.light(tp + Y(2.2), warm, 3.4, 7.5);
        let bay = s
            .free_bays(Some(twall.mid), 1.2)
            .into_iter()
            .find(|b| b.inward.dot(twall.inward) > 0.9 && hdist(b.center, twall.mid) < 2.6);
        if let Some(bay) = bay {
            s.take_bay(&bay);
            let (c, inw, tan, w) = (bay.center, bay.inward, bay.tangent, bay.width);
            s.plan.wall_pieces.push(c);
            s.piece(move |m, g, ctx| crypt_shrine(m, g, c, inw, tan, w, ctx.stone, &mut ctx.rng));
            s.solid(bay.center + bay.inward * 0.3, 0.34, true);
            s.light(bay.center + bay.inward * 0.7 + Y(1.0), warm, 1.6, 3.8);
        }
    }
    // 2. Ossuaries fill the bays; sarcophagi stand before the pilasters.
    let mut ossuaries = 0;
    for bay in s.free_bays(None, 1.8) {
        if ossuaries >= 5 {
            break;
        }
        let width = 2.6f32.min(bay.width - 0.55);
        let height = 2.45f32.min(s.spring - 0.35);
        let circles: Vec<Vec3> = [-0.33f32, 0.0, 0.33]
            .iter()
            .map(|&k| bay.center + bay.inward * 0.25 + bay.tangent * (k * width))
            .collect();
        if !circles
            .iter()
            .all(|&p| s.placer.clear(s.floor(p), 0.28, 0.5, 0.4) || s.walkway(p) > 1.2)
        {
            continue;
        }
        if circles.iter().any(|&p| s.crowded(p, 0.28, 0.02)) {
            continue;
        }
        s.take_bay(&bay);
        let (c, inw, tan) = (s.floor(bay.center), bay.inward, bay.tangent);
        s.plan.wall_pieces.push(c);
        s.piece(move |m, g, ctx| {
            ossuary(m, g, c, inw, tan, width, height, ctx.stone, &mut ctx.rng)
        });
        for &p in &circles {
            let q = s.floor(p);
            s.solid(q, 0.28, true);
        }
        s.light(
            bay.center + bay.inward * 1.1 + Y(1.3),
            [0.62, 0.52, 0.95],
            1.1,
            3.4,
        );
        ossuaries += 1;
    }
    let mut sarcophagi: Vec<Vec3> = vec![];
    let mut pilasters = s.pilasters.clone();
    s.rng.shuffle(&mut pilasters);
    for pilaster in &pilasters {
        if sarcophagi.len() >= 5 {
            break;
        }
        let yaw = yaw_of(-pilaster.inward);
        let cands: Vec<Vec3> = [1.55, 1.75]
            .iter()
            .map(|&d| pilaster.base + pilaster.inward * d)
            .collect();
        let Some(p) = s.place_row(&cands, yaw, 1.12, 0.62, 1.4, 0.8) else {
            continue;
        };
        let variant = [0, 1, 2, 0, 1][sarcophagi.len()];
        sarcophagi.push(p);
        s.piece(move |m, _g, ctx| crypt_sarcophagus(m, p, yaw, variant, ctx.stone, &mut ctx.rng));
        let (f, sd) = frame_of(yaw);
        if let Some(q) = s.spot(p + sd * 0.85 + f * 0.8, 0.16, 0.3) {
            let count = s.rng.index(3) + 3;
            s.candle_cluster(q, count, 0.13);
            if s.rng.chance(0.6) {
                s.light(q + Y(0.5), warm, 0.9, 2.6);
            }
        }
        let trunk = s.spot(p - sd * 0.95 + f * 0.4, 0.3, 0.3);
        if let Some(trunk) = trunk {
            if s.rng.chance(0.5) {
                let asset = if s.rng.chance(0.5) {
                    "trunk_small"
                } else {
                    "trunk_medium_b"
                };
                let y = yaw + PI / 2.0 + s.rng.range(-0.3, 0.3);
                s.prop(asset, trunk, y, 0.42, 0.0);
            }
        }
    }
    // Where the pilasters left no room, a row of sarcophagi flanks the aisle.
    'rows: for sgn in [-1.0f32, 1.0] {
        for fwd in [-0.35f32, 0.2] {
            if sarcophagi.len() >= 4 {
                break 'rows;
            }
            let cands: Vec<Vec3> = [0.62f32, 0.5]
                .iter()
                .map(|&r| s.at(fwd * s.depth, sgn * r * s.width))
                .collect();
            let yaw = yaw_of(s.right * sgn);
            let Some(p) = s.place_row(&cands, yaw, 1.12, 0.62, 1.4, 0.8) else {
                continue;
            };
            sarcophagi.push(p);
            let variant = sarcophagi.len() % 3;
            s.piece(move |m, _g, ctx| {
                crypt_sarcophagus(m, p, yaw, variant, ctx.stone, &mut ctx.rng)
            });
        }
    }
    // 3. Grave slabs pave the aisle from the entrance.
    let aisle_end = match &tomb {
        Some((tp, tyaw, _)) => *tp - frame_of(*tyaw).0 * 1.9,
        None => s.at(s.depth - 1.5, 0.0),
    };
    let aisle = hz(aisle_end - s.entry);
    let aisle_yaw = yaw_of(aisle);
    let (mut d, mut k) = (2.0f32, 0usize);
    while d < aisle.length() - 0.6 {
        let side = norm(v(-aisle.z, 0.0, aisle.x)) * if k % 2 == 1 { 0.55 } else { -0.55 };
        let q = s.floor(s.entry + norm(aisle) * d + side);
        if s.inside(q, 0.8) && !s.crowded(q, 0.75, 0.0) {
            let variant = k % 3;
            s.piece(move |m, _g, ctx| {
                grave_slab(m, q, aisle_yaw, variant, ctx.stone, &mut ctx.rng)
            });
        }
        d += 1.75;
        k += 1;
    }
    // 4. Urns and burial goods in the corners.
    for corner in s.corners.clone() {
        let cands: Vec<Vec3> = [1.1, 1.4]
            .iter()
            .map(|&d| corner.mid + corner.inward * d)
            .collect();
        let Some(q) = s.place(&cands, 0.28, 1.2, 0.5) else {
            continue;
        };
        s.plan.solids.push((q, 0.28));
        let h = s.rng.range(0.7, 0.95);
        let clay = if s.rng.chance(0.5) {
            [0.38, 0.33, 0.3]
        } else {
            [0.42, 0.26, 0.18]
        };
        s.piece(move |m, _g, _ctx| burial_urn(m, q, h, clay));
        let side = if s.rng.chance(0.5) { 0.5 } else { -0.5 };
        if let Some(near) = s.spot(q + corner.tangent * side + corner.inward * 0.2, 0.18, 0.25) {
            if s.rng.chance(0.5) {
                let asset = if s.rng.chance(0.5) {
                    "vase_small"
                } else {
                    "chalice"
                };
                let yaw = s.rng.range(0.0, TAU);
                let h = s.rng.range(0.28, 0.4);
                s.prop(asset, near, yaw, h, 0.0);
            } else {
                s.shards(near, 0.3, 9, None);
            }
        }
        let apex = corner.mid + Y(s.spring.min(s.height - 0.6)) + corner.inward * 0.05;
        let size = s.rng.range(0.7, 1.05);
        s.corner_web(
            apex,
            norm(corner.tangent + Y(-0.25)),
            norm(-corner.tangent + Y(-0.25)),
            size,
        );
    }
    // 5. Webs between pilasters and walls, chains from the vault.
    let mut pilasters = s.pilasters.clone();
    s.rng.shuffle(&mut pilasters);
    for pilaster in pilasters.iter().take(5) {
        let side = if s.rng.chance(0.5) { 1.0 } else { -1.0 };
        let up = s.rng.range(1.9, 2.8f32.min(s.spring - 0.2));
        let apex =
            pilaster.base + pilaster.tangent * (side * 0.36) + pilaster.inward * 0.02 + Y(up);
        let size = s.rng.range(0.45, 0.75);
        s.corner_web(
            apex,
            norm(pilaster.tangent * side + Y(-0.3)),
            norm(pilaster.inward + Y(-0.3)),
            size,
        );
    }
    for _ in 0..6 {
        let (a, b) = (s.rng.range(-0.55, 0.55), s.rng.range(-0.55, 0.55));
        let p = s.at(a * s.depth, b * s.width);
        let top = s.top(p);
        let length = s.rng.range(0.8, (s.height - 2.7).max(1.0));
        s.chain_hang(top, length);
    }
    // 6. Floor: dust, cracks, loose bones, candle stubs.
    let floor_only = Scatter {
        walkway: 0.0,
        ..Scatter::default()
    };
    let ring = |a: f32, walk: f32| Scatter {
        walkway: walk,
        ring: [a, 1.0],
        ..Scatter::default()
    };
    s.scatter(18, 0.4, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.22, 0.55);
        s.dust(p, r, None);
    });
    s.scatter(10, 0.1, floor_only, &mut |s, p, _| {
        let dir = v(s.rng.range(-1.0, 1.0), 0.0, s.rng.range(-1.0, 1.0));
        let length = s.rng.range(0.6, 1.5);
        s.crack(p, dir, length, None);
    });
    s.scatter(7, 0.4, ring(0.45, 0.6), &mut |s, p, _| {
        let count = s.rng.index(4) + 3;
        s.loose_bones(p, 0.35, count);
    });
    s.scatter(5, 0.15, ring(0.5, 0.5), &mut |s, p, _| {
        let count = s.rng.index(3) + 2;
        s.candle_cluster(p, count, 0.1);
    });
    s.scatter(2, 0.5, ring(0.65, 0.6), &mut |s, p, _| {
        let yaw = s.rng.range(0.0, TAU);
        let h = s.rng.range(0.3, 0.45);
        s.prop("rubble_large", p, yaw, h, 0.0);
    });
    // 7. Air: a dim violet wash and drifting dust.
    let top = s.c + Y(s.height - 1.1);
    s.light(top, [0.6, 0.5, 1.0], 1.4, 9.0);
    s.plan.sparkle_tint = Some([0.75, 0.68, 1.3]);
    let h = s.height;
    s.sparkles(22, 0.4, h - 1.2, None, None);
}
