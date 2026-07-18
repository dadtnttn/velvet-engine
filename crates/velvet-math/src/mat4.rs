//! 4×4 matrices for orthographic projection and optional 2D pipelines.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Mat3, Vec2, Vec3};

/// 4×4 matrix stored in column-major order (GPU-friendly).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mat4 {
    /// Column 0.
    pub x_axis: [f32; 4],
    /// Column 1.
    pub y_axis: [f32; 4],
    /// Column 2.
    pub z_axis: [f32; 4],
    /// Column 3.
    pub w_axis: [f32; 4],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    /// Identity matrix.
    pub const IDENTITY: Self = Self {
        x_axis: [1.0, 0.0, 0.0, 0.0],
        y_axis: [0.0, 1.0, 0.0, 0.0],
        z_axis: [0.0, 0.0, 1.0, 0.0],
        w_axis: [0.0, 0.0, 0.0, 1.0],
    };

    /// Zero matrix.
    pub const ZERO: Self = Self {
        x_axis: [0.0, 0.0, 0.0, 0.0],
        y_axis: [0.0, 0.0, 0.0, 0.0],
        z_axis: [0.0, 0.0, 0.0, 0.0],
        w_axis: [0.0, 0.0, 0.0, 0.0],
    };

    /// Create from four columns.
    pub const fn from_cols(x: [f32; 4], y: [f32; 4], z: [f32; 4], w: [f32; 4]) -> Self {
        Self {
            x_axis: x,
            y_axis: y,
            z_axis: z,
            w_axis: w,
        }
    }

    /// 2D translation embedded in 4×4 (XY, Z=0).
    pub fn from_translation_2d(t: Vec2) -> Self {
        Self {
            x_axis: [1.0, 0.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0, 0.0],
            z_axis: [0.0, 0.0, 1.0, 0.0],
            w_axis: [t.x, t.y, 0.0, 1.0],
        }
    }

    /// 3D translation.
    pub fn from_translation(t: Vec3) -> Self {
        Self {
            x_axis: [1.0, 0.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0, 0.0],
            z_axis: [0.0, 0.0, 1.0, 0.0],
            w_axis: [t.x, t.y, t.z, 1.0],
        }
    }

    /// Non-uniform scale (XYZ).
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            x_axis: [scale.x, 0.0, 0.0, 0.0],
            y_axis: [0.0, scale.y, 0.0, 0.0],
            z_axis: [0.0, 0.0, scale.z, 0.0],
            w_axis: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Uniform 2D scale (Z = 1).
    pub fn from_scale_2d(scale: Vec2) -> Self {
        Self::from_scale(Vec3::new(scale.x, scale.y, 1.0))
    }

    /// Rotation about Z (2D-relevant, radians, counter-clockwise).
    pub fn from_rotation_z(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            x_axis: [c, s, 0.0, 0.0],
            y_axis: [-s, c, 0.0, 0.0],
            z_axis: [0.0, 0.0, 1.0, 0.0],
            w_axis: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// TRS for 2D: translate * rotate_z * scale.
    pub fn from_scale_angle_translation_2d(scale: Vec2, angle: f32, translation: Vec2) -> Self {
        Self::from_translation_2d(translation)
            * Self::from_rotation_z(angle)
            * Self::from_scale_2d(scale)
    }

    /// Orthographic projection (Y-up), mapping `[left,right]×[bottom,top]×[near,far]` to NDC.
    pub fn orthographic_rh(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let rl = (right - left).max(1e-6);
        let tb = (top - bottom).max(1e-6);
        let fn_ = (far - near).max(1e-6);
        Self {
            x_axis: [2.0 / rl, 0.0, 0.0, 0.0],
            y_axis: [0.0, 2.0 / tb, 0.0, 0.0],
            z_axis: [0.0, 0.0, -2.0 / fn_, 0.0],
            w_axis: [
                -(right + left) / rl,
                -(top + bottom) / tb,
                -(far + near) / fn_,
                1.0,
            ],
        }
    }

    /// 2D orthographic from viewport size centered at origin.
    pub fn orthographic_2d(width: f32, height: f32) -> Self {
        let hw = width.max(1e-6) * 0.5;
        let hh = height.max(1e-6) * 0.5;
        Self::orthographic_rh(-hw, hw, -hh, hh, -1.0, 1.0)
    }

    /// Promote a 2D affine [`Mat3`] into a 4×4 (Z preserved as identity).
    pub fn from_mat3(m: Mat3) -> Self {
        Self {
            x_axis: [m.x_axis[0], m.x_axis[1], 0.0, m.x_axis[2]],
            y_axis: [m.y_axis[0], m.y_axis[1], 0.0, m.y_axis[2]],
            z_axis: [0.0, 0.0, 1.0, 0.0],
            w_axis: [m.z_axis[0], m.z_axis[1], 0.0, m.z_axis[2]],
        }
    }

    /// Extract the upper-left affine 2D part as [`Mat3`].
    pub fn to_mat3(self) -> Mat3 {
        Mat3 {
            x_axis: [self.x_axis[0], self.x_axis[1], self.x_axis[3]],
            y_axis: [self.y_axis[0], self.y_axis[1], self.y_axis[3]],
            z_axis: [self.w_axis[0], self.w_axis[1], self.w_axis[3]],
        }
    }

    /// Matrix multiplication `self * rhs`.
    pub fn mul_mat4(self, rhs: Self) -> Self {
        let cols = [rhs.x_axis, rhs.y_axis, rhs.z_axis, rhs.w_axis];
        let mut out = [[0.0f32; 4]; 4];
        let a = [self.x_axis, self.y_axis, self.z_axis, self.w_axis];
        for (j, col) in cols.iter().enumerate() {
            for i in 0..4 {
                out[j][i] =
                    a[0][i] * col[0] + a[1][i] * col[1] + a[2][i] * col[2] + a[3][i] * col[3];
            }
        }
        Self {
            x_axis: out[0],
            y_axis: out[1],
            z_axis: out[2],
            w_axis: out[3],
        }
    }

    /// Transform a 2D point (z=0, w=1) and perspective-divide if needed.
    pub fn transform_point2(self, p: Vec2) -> Vec2 {
        let x = self.x_axis[0] * p.x + self.y_axis[0] * p.y + self.w_axis[0];
        let y = self.x_axis[1] * p.x + self.y_axis[1] * p.y + self.w_axis[1];
        let w = self.x_axis[3] * p.x + self.y_axis[3] * p.y + self.w_axis[3];
        if w.abs() > 1e-8 {
            Vec2::new(x / w, y / w)
        } else {
            Vec2::new(x, y)
        }
    }

    /// Transform a 3D point (w=1).
    pub fn transform_point3(self, p: Vec3) -> Vec3 {
        let x = self.x_axis[0] * p.x + self.y_axis[0] * p.y + self.z_axis[0] * p.z + self.w_axis[0];
        let y = self.x_axis[1] * p.x + self.y_axis[1] * p.y + self.z_axis[1] * p.z + self.w_axis[1];
        let z = self.x_axis[2] * p.x + self.y_axis[2] * p.y + self.z_axis[2] * p.z + self.w_axis[2];
        let w = self.x_axis[3] * p.x + self.y_axis[3] * p.y + self.z_axis[3] * p.z + self.w_axis[3];
        if w.abs() > 1e-8 {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }

    /// Transform a 2D vector (w=0).
    pub fn transform_vector2(self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self.x_axis[0] * v.x + self.y_axis[0] * v.y,
            y: self.x_axis[1] * v.x + self.y_axis[1] * v.y,
        }
    }

    /// Determinant (full 4×4).
    pub fn determinant(self) -> f32 {
        let m = [
            self.x_axis[0],
            self.x_axis[1],
            self.x_axis[2],
            self.x_axis[3],
            self.y_axis[0],
            self.y_axis[1],
            self.y_axis[2],
            self.y_axis[3],
            self.z_axis[0],
            self.z_axis[1],
            self.z_axis[2],
            self.z_axis[3],
            self.w_axis[0],
            self.w_axis[1],
            self.w_axis[2],
            self.w_axis[3],
        ];
        // Laplace expansion along first row of row-major view of columns.
        fn det3(
            a00: f32,
            a01: f32,
            a02: f32,
            a10: f32,
            a11: f32,
            a12: f32,
            a20: f32,
            a21: f32,
            a22: f32,
        ) -> f32 {
            a00 * (a11 * a22 - a12 * a21) - a01 * (a10 * a22 - a12 * a20)
                + a02 * (a10 * a21 - a11 * a20)
        }
        // Column-major index: col * 4 + row
        let c = |col: usize, row: usize| m[col * 4 + row];
        c(0, 0)
            * det3(
                c(1, 1),
                c(1, 2),
                c(1, 3),
                c(2, 1),
                c(2, 2),
                c(2, 3),
                c(3, 1),
                c(3, 2),
                c(3, 3),
            )
            - c(1, 0)
                * det3(
                    c(0, 1),
                    c(0, 2),
                    c(0, 3),
                    c(2, 1),
                    c(2, 2),
                    c(2, 3),
                    c(3, 1),
                    c(3, 2),
                    c(3, 3),
                )
            + c(2, 0)
                * det3(
                    c(0, 1),
                    c(0, 2),
                    c(0, 3),
                    c(1, 1),
                    c(1, 2),
                    c(1, 3),
                    c(3, 1),
                    c(3, 2),
                    c(3, 3),
                )
            - c(3, 0)
                * det3(
                    c(0, 1),
                    c(0, 2),
                    c(0, 3),
                    c(1, 1),
                    c(1, 2),
                    c(1, 3),
                    c(2, 1),
                    c(2, 2),
                    c(2, 3),
                )
    }

    /// Inverse via adjugate, or `None` if singular.
    #[allow(clippy::needless_range_loop)] // index-heavy matrix cofactor construction
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        let m = [self.x_axis, self.y_axis, self.z_axis, self.w_axis];
        // Cofactor matrix (row/col of column-major).
        let mut inv = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                let mut minor = [[0.0f32; 3]; 3];
                let mut r = 0;
                for ii in 0..4 {
                    if ii == i {
                        continue;
                    }
                    let mut c = 0;
                    for jj in 0..4 {
                        if jj == j {
                            continue;
                        }
                        minor[r][c] = m[jj][ii];
                        c += 1;
                    }
                    r += 1;
                }
                let det_m = minor[0][0] * (minor[1][1] * minor[2][2] - minor[1][2] * minor[2][1])
                    - minor[0][1] * (minor[1][0] * minor[2][2] - minor[1][2] * minor[2][0])
                    + minor[0][2] * (minor[1][0] * minor[2][1] - minor[1][1] * minor[2][0]);
                let sign = if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
                inv[i][j] = sign * det_m * inv_det;
            }
        }
        Some(Self {
            x_axis: inv[0],
            y_axis: inv[1],
            z_axis: inv[2],
            w_axis: inv[3],
        })
    }

    /// Transpose.
    pub fn transpose(self) -> Self {
        Self {
            x_axis: [
                self.x_axis[0],
                self.y_axis[0],
                self.z_axis[0],
                self.w_axis[0],
            ],
            y_axis: [
                self.x_axis[1],
                self.y_axis[1],
                self.z_axis[1],
                self.w_axis[1],
            ],
            z_axis: [
                self.x_axis[2],
                self.y_axis[2],
                self.z_axis[2],
                self.w_axis[2],
            ],
            w_axis: [
                self.x_axis[3],
                self.y_axis[3],
                self.z_axis[3],
                self.w_axis[3],
            ],
        }
    }

    /// Column-major flat array.
    pub fn to_cols_array(self) -> [f32; 16] {
        [
            self.x_axis[0],
            self.x_axis[1],
            self.x_axis[2],
            self.x_axis[3],
            self.y_axis[0],
            self.y_axis[1],
            self.y_axis[2],
            self.y_axis[3],
            self.z_axis[0],
            self.z_axis[1],
            self.z_axis[2],
            self.z_axis[3],
            self.w_axis[0],
            self.w_axis[1],
            self.w_axis[2],
            self.w_axis[3],
        ]
    }

    /// From column-major flat array.
    pub fn from_cols_array(a: [f32; 16]) -> Self {
        Self {
            x_axis: [a[0], a[1], a[2], a[3]],
            y_axis: [a[4], a[5], a[6], a[7]],
            z_axis: [a[8], a[9], a[10], a[11]],
            w_axis: [a[12], a[13], a[14], a[15]],
        }
    }
}

impl core::ops::Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.mul_mat4(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_2d_point() {
        let m = Mat4::from_translation_2d(Vec2::new(3.0, -2.0));
        let p = m.transform_point2(Vec2::new(1.0, 4.0));
        assert!((p.x - 4.0).abs() < 1e-5);
        assert!((p.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn rotation_z_90() {
        let m = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let p = m.transform_point2(Vec2::X);
        assert!(p.x.abs() < 1e-5);
        assert!((p.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn orthographic_maps_center() {
        let m = Mat4::orthographic_2d(100.0, 50.0);
        let p = m.transform_point2(Vec2::ZERO);
        assert!(p.x.abs() < 1e-4);
        assert!(p.y.abs() < 1e-4);
        let edge = m.transform_point2(Vec2::new(50.0, 0.0));
        assert!((edge.x - 1.0).abs() < 1e-4);
    }

    #[test]
    fn mat3_roundtrip_affine() {
        let m3 = Mat3::from_scale_angle_translation(Vec2::new(2.0, 3.0), 0.3, Vec2::new(5.0, -1.0));
        let m4 = Mat4::from_mat3(m3);
        let p = Vec2::new(1.5, 2.5);
        let a = m3.transform_point2(p);
        let b = m4.transform_point2(p);
        assert!((a.x - b.x).abs() < 1e-4);
        assert!((a.y - b.y).abs() < 1e-4);
    }

    #[test]
    fn inverse_translation() {
        let m = Mat4::from_translation_2d(Vec2::new(10.0, -4.0));
        let inv = m.inverse().expect("invertible");
        let p = inv.transform_point2(m.transform_point2(Vec2::new(2.0, 3.0)));
        assert!((p.x - 2.0).abs() < 1e-4);
        assert!((p.y - 3.0).abs() < 1e-4);
    }

    #[test]
    fn mul_identity() {
        let m = Mat4::from_scale_angle_translation_2d(Vec2::splat(2.0), 0.1, Vec2::new(1.0, 2.0));
        let r = m * Mat4::IDENTITY;
        for (a, b) in m.to_cols_array().iter().zip(r.to_cols_array().iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn property_trs_composition_matches_stepwise() {
        // translate * rotate * scale applied to points should match composed matrix.
        let scales = [
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 0.5),
            Vec2::new(0.25, 3.0),
        ];
        let angles = [
            0.0_f32,
            0.3,
            -1.2,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ];
        let translations = [Vec2::ZERO, Vec2::new(10.0, -4.0), Vec2::new(-1.5, 2.25)];
        let points = [
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            Vec2::new(3.0, -2.0),
            Vec2::new(-7.5, 0.25),
        ];
        for scale in scales {
            for angle in angles {
                for t in translations {
                    let m = Mat4::from_scale_angle_translation_2d(scale, angle, t);
                    let step = Mat4::from_translation_2d(t)
                        * Mat4::from_rotation_z(angle)
                        * Mat4::from_scale_2d(scale);
                    for p in points {
                        let a = m.transform_point2(p);
                        let b = step.transform_point2(p);
                        assert!(
                            (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4,
                            "scale={scale:?} angle={angle} t={t:?} p={p:?} a={a:?} b={b:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn property_inverse_roundtrip_affine() {
        let cases = [
            (Vec2::splat(1.0), 0.0, Vec2::ZERO),
            (Vec2::new(2.0, 3.0), 0.4, Vec2::new(5.0, -2.0)),
            (Vec2::new(0.5, 0.5), -0.9, Vec2::new(-3.0, 7.0)),
            (
                Vec2::new(4.0, 0.25),
                std::f32::consts::FRAC_PI_4,
                Vec2::new(1.0, 1.0),
            ),
        ];
        let pts = [
            Vec2::ZERO,
            Vec2::new(1.0, 2.0),
            Vec2::new(-4.0, 0.5),
            Vec2::new(8.0, -3.0),
        ];
        for (scale, angle, t) in cases {
            let m = Mat4::from_scale_angle_translation_2d(scale, angle, t);
            let inv = m.inverse().expect("affine TRS should invert");
            let id = m * inv;
            for p in pts {
                let back = inv.transform_point2(m.transform_point2(p));
                assert!((back.x - p.x).abs() < 1e-3, "back={back:?} p={p:?}");
                assert!((back.y - p.y).abs() < 1e-3);
                let idp = id.transform_point2(p);
                assert!((idp.x - p.x).abs() < 1e-3 && (idp.y - p.y).abs() < 1e-3);
            }
            // det of product ≈ 1
            assert!(
                (id.determinant() - 1.0).abs() < 1e-3,
                "det={}",
                id.determinant()
            );
        }
    }

    #[test]
    fn property_scale_determinant() {
        for sx in [0.5_f32, 1.0, 2.0, 3.5] {
            for sy in [0.25_f32, 1.0, 2.0] {
                for sz in [0.5_f32, 1.0, 4.0] {
                    let m = Mat4::from_scale(Vec3::new(sx, sy, sz));
                    let expected = sx * sy * sz;
                    assert!(
                        (m.determinant() - expected).abs() < 1e-4,
                        "det={} expected={expected}",
                        m.determinant()
                    );
                }
            }
        }
    }

    #[test]
    fn property_rotation_preserves_length() {
        let angles: Vec<f32> = (0..16).map(|i| i as f32 * 0.4).collect();
        let vecs = [
            Vec2::X,
            Vec2::Y,
            Vec2::new(3.0, 4.0),
            Vec2::new(-2.0, 5.0),
            Vec2::new(0.1, -0.2),
        ];
        for a in angles {
            let m = Mat4::from_rotation_z(a);
            for v in vecs {
                let out = m.transform_vector2(v);
                assert!(
                    (out.length() - v.length()).abs() < 1e-4,
                    "angle={a} in={} out={}",
                    v.length(),
                    out.length()
                );
            }
        }
    }

    #[test]
    fn property_orthographic_maps_edges() {
        for (w, h) in [(100.0_f32, 50.0), (1920.0, 1080.0), (64.0, 64.0)] {
            let m = Mat4::orthographic_2d(w, h);
            let right = m.transform_point2(Vec2::new(w * 0.5, 0.0));
            let top = m.transform_point2(Vec2::new(0.0, h * 0.5));
            let left = m.transform_point2(Vec2::new(-w * 0.5, 0.0));
            let bottom = m.transform_point2(Vec2::new(0.0, -h * 0.5));
            assert!((right.x - 1.0).abs() < 1e-3, "right={right:?}");
            assert!((left.x + 1.0).abs() < 1e-3, "left={left:?}");
            assert!((top.y - 1.0).abs() < 1e-3, "top={top:?}");
            assert!((bottom.y + 1.0).abs() < 1e-3, "bottom={bottom:?}");
        }
    }

    #[test]
    fn property_mul_associative_sample() {
        let a = Mat4::from_translation_2d(Vec2::new(1.0, 2.0));
        let b = Mat4::from_rotation_z(0.7);
        let c = Mat4::from_scale_2d(Vec2::new(2.0, 0.5));
        let left = (a * b) * c;
        let right = a * (b * c);
        let p = Vec2::new(3.0, -1.0);
        let lp = left.transform_point2(p);
        let rp = right.transform_point2(p);
        assert!((lp.x - rp.x).abs() < 1e-4 && (lp.y - rp.y).abs() < 1e-4);
    }

    #[test]
    fn transform_point3_and_vector2_ignore_translation() {
        let m = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0))
            * Mat4::from_scale(Vec3::new(2.0, 2.0, 2.0));
        let p = m.transform_point3(Vec3::new(1.0, 2.0, 3.0));
        assert!((p.x - 12.0).abs() < 1e-4);
        assert!((p.y - 24.0).abs() < 1e-4);
        assert!((p.z - 36.0).abs() < 1e-4);
        let v = m.transform_vector2(Vec2::new(1.0, 1.0));
        assert!((v.x - 2.0).abs() < 1e-4);
        assert!((v.y - 2.0).abs() < 1e-4);
    }

    #[test]
    fn singular_scale_has_no_inverse() {
        let m = Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0));
        assert!(m.inverse().is_none());
        assert!(m.determinant().abs() < 1e-6);
    }

    #[test]
    fn zero_and_identity_constants() {
        assert_eq!(Mat4::default(), Mat4::IDENTITY);
        let z = Mat4::ZERO;
        assert_eq!(z.determinant(), 0.0);
        let p = z.transform_point2(Vec2::new(5.0, 5.0));
        assert_eq!(p, Vec2::ZERO);
    }
}
