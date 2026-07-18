//! # velvet-math
//!
//! Lightweight 2D math types used across Velvet Engine.
//!
//! This crate intentionally has no engine dependencies so it can be used from
//! tools, scripts tooling, and runtime code alike.

#![deny(missing_docs)]
#![warn(clippy::all)]

mod aabb;
mod color;
mod curve;
mod easing;
mod mat3;
mod mat4;
mod random;
mod rect;
mod srgb;
mod transform;
mod vec2;
mod vec3;

pub use aabb::{aabb_sweep, aabb_union, Aabb2};
pub use color::{Color, Color8};
pub use curve::{
    catmull_rom, cubic_bezier, cubic_bezier_curvature, cubic_bezier_deriv, cubic_bezier_eased,
    cubic_bezier_length, hermite, line_sample, mix_f32, quadratic_bezier, quadratic_bezier_deriv,
    split_cubic_bezier, BezierPath2, CubicBezier2, Polyline2,
};
pub use easing::{
    back_in, back_in_out, back_out, bounce_in, bounce_in_out, bounce_out, circ_in, circ_in_out,
    circ_out, cubic_in, cubic_in_out, cubic_out, elastic_in, elastic_in_out, elastic_out, expo_in,
    expo_in_out, expo_out, linear, ping_pong, quad_in, quad_in_out, quad_out, quart_in,
    quart_in_out, quart_out, quint_in, quint_in_out, quint_out, repeat, sine_in, sine_in_out,
    sine_out, smootherstep, smoothstep as ease_smoothstep, Ease,
};
pub use mat3::Mat3;
pub use mat4::Mat4;
pub use random::{hash_u64, mix_seed, Pcg32, SplitMix64, XorShift32};
pub use rect::Rect;
pub use srgb::{
    approx_gamma_decode, approx_gamma_encode, contrast_ratio_linear, hsv_to_rgb, lerp_linear,
    linear_luminance, linear_to_display_srgb, linear_to_srgb, linear_to_srgb8,
    linear_to_srgb_channel, premultiply_srgb, reinhard_tonemap, rgb_to_hsv, srgb8_to_linear,
    srgb_luminance, srgb_to_linear, srgb_to_linear_channel,
};
pub use transform::Transform2D;
pub use vec2::Vec2;
pub use vec3::Vec3;

/// Approximate equality for floating-point comparisons in tests and tolerances.
pub fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() <= epsilon
}

/// Clamp `value` into `[min, max]`.
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

/// Linear interpolation from `a` to `b` by `t` (not clamped).
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep interpolation in `[edge0, edge1]`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Degrees to radians.
#[inline]
pub fn deg_to_rad(degrees: f32) -> f32 {
    degrees * std::f32::consts::PI / 180.0
}

/// Radians to degrees.
#[inline]
pub fn rad_to_deg(radians: f32) -> f32 {
    radians * 180.0 / std::f32::consts::PI
}

/// Inverse lerp: how far `v` is between `a` and `b` (unclamped).
#[inline]
pub fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 {
    if (b - a).abs() < 1e-12 {
        0.0
    } else {
        (v - a) / (b - a)
    }
}

/// Remap `v` from `[in_min, in_max]` to `[out_min, out_max]`.
#[inline]
pub fn remap(v: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    lerp(out_min, out_max, inverse_lerp(in_min, in_max, v))
}

/// Remap with clamped input.
#[inline]
pub fn remap_clamped(v: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let t = clamp(inverse_lerp(in_min, in_max, v), 0.0, 1.0);
    lerp(out_min, out_max, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_and_clamp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < f32::EPSILON);
        assert!((clamp(15.0, 0.0, 10.0) - 10.0).abs() < f32::EPSILON);
        assert!((clamp(-1.0, 0.0, 10.0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn angle_conversion_roundtrip() {
        let deg = 90.0;
        let back = rad_to_deg(deg_to_rad(deg));
        assert!((back - deg).abs() < 1e-4);
    }

    #[test]
    fn remap_works() {
        assert!((remap(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < 1e-5);
        assert!((remap_clamped(15.0, 0.0, 10.0, 0.0, 1.0) - 1.0).abs() < 1e-5);
    }
}
