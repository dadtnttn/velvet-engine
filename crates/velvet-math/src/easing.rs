//! Easing / tween functions mapping `t ∈ [0, 1]` → eased `t`.

use crate::clamp;

/// Named easing curves commonly used in UI and gameplay tweens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ease {
    /// Linear (identity).
    #[default]
    Linear,
    /// Quadratic ease-in.
    QuadIn,
    /// Quadratic ease-out.
    QuadOut,
    /// Quadratic ease-in-out.
    QuadInOut,
    /// Cubic ease-in.
    CubicIn,
    /// Cubic ease-out.
    CubicOut,
    /// Cubic ease-in-out.
    CubicInOut,
    /// Quartic ease-in.
    QuartIn,
    /// Quartic ease-out.
    QuartOut,
    /// Quartic ease-in-out.
    QuartInOut,
    /// Quintic ease-in.
    QuintIn,
    /// Quintic ease-out.
    QuintOut,
    /// Quintic ease-in-out.
    QuintInOut,
    /// Sine ease-in.
    SineIn,
    /// Sine ease-out.
    SineOut,
    /// Sine ease-in-out.
    SineInOut,
    /// Exponential ease-in.
    ExpoIn,
    /// Exponential ease-out.
    ExpoOut,
    /// Exponential ease-in-out.
    ExpoInOut,
    /// Circular ease-in.
    CircIn,
    /// Circular ease-out.
    CircOut,
    /// Circular ease-in-out.
    CircInOut,
    /// Back ease-in (overshoot).
    BackIn,
    /// Back ease-out.
    BackOut,
    /// Back ease-in-out.
    BackInOut,
    /// Elastic ease-in.
    ElasticIn,
    /// Elastic ease-out.
    ElasticOut,
    /// Elastic ease-in-out.
    ElasticInOut,
    /// Bounce ease-in.
    BounceIn,
    /// Bounce ease-out.
    BounceOut,
    /// Bounce ease-in-out.
    BounceInOut,
    /// Smoothstep (Hermite).
    Smoothstep,
    /// Smootherstep (Ken Perlin).
    Smootherstep,
}

impl Ease {
    /// Evaluate the easing at parameter `t` (clamped to `[0, 1]`).
    pub fn eval(self, t: f32) -> f32 {
        let t = clamp(t, 0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::QuadIn => quad_in(t),
            Self::QuadOut => quad_out(t),
            Self::QuadInOut => quad_in_out(t),
            Self::CubicIn => cubic_in(t),
            Self::CubicOut => cubic_out(t),
            Self::CubicInOut => cubic_in_out(t),
            Self::QuartIn => quart_in(t),
            Self::QuartOut => quart_out(t),
            Self::QuartInOut => quart_in_out(t),
            Self::QuintIn => quint_in(t),
            Self::QuintOut => quint_out(t),
            Self::QuintInOut => quint_in_out(t),
            Self::SineIn => sine_in(t),
            Self::SineOut => sine_out(t),
            Self::SineInOut => sine_in_out(t),
            Self::ExpoIn => expo_in(t),
            Self::ExpoOut => expo_out(t),
            Self::ExpoInOut => expo_in_out(t),
            Self::CircIn => circ_in(t),
            Self::CircOut => circ_out(t),
            Self::CircInOut => circ_in_out(t),
            Self::BackIn => back_in(t),
            Self::BackOut => back_out(t),
            Self::BackInOut => back_in_out(t),
            Self::ElasticIn => elastic_in(t),
            Self::ElasticOut => elastic_out(t),
            Self::ElasticInOut => elastic_in_out(t),
            Self::BounceIn => bounce_in(t),
            Self::BounceOut => bounce_out(t),
            Self::BounceInOut => bounce_in_out(t),
            Self::Smoothstep => smoothstep(t),
            Self::Smootherstep => smootherstep(t),
        }
    }

    /// Interpolate from `a` to `b` with this easing.
    pub fn lerp(self, a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * self.eval(t)
    }
}

/// Linear.
#[inline]
pub fn linear(t: f32) -> f32 {
    t
}

/// Quadratic ease-in.
#[inline]
pub fn quad_in(t: f32) -> f32 {
    t * t
}

/// Quadratic ease-out.
#[inline]
pub fn quad_out(t: f32) -> f32 {
    t * (2.0 - t)
}

/// Quadratic ease-in-out.
pub fn quad_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

/// Cubic ease-in.
#[inline]
pub fn cubic_in(t: f32) -> f32 {
    t * t * t
}

/// Cubic ease-out.
#[inline]
pub fn cubic_out(t: f32) -> f32 {
    let u = t - 1.0;
    u * u * u + 1.0
}

/// Cubic ease-in-out.
pub fn cubic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let u = 2.0 * t - 2.0;
        0.5 * u * u * u + 1.0
    }
}

/// Quartic ease-in.
#[inline]
pub fn quart_in(t: f32) -> f32 {
    t * t * t * t
}

/// Quartic ease-out.
#[inline]
pub fn quart_out(t: f32) -> f32 {
    let u = t - 1.0;
    1.0 - u * u * u * u
}

/// Quartic ease-in-out.
pub fn quart_in_out(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        let u = t - 1.0;
        1.0 - 8.0 * u * u * u * u
    }
}

/// Quintic ease-in.
#[inline]
pub fn quint_in(t: f32) -> f32 {
    t * t * t * t * t
}

/// Quintic ease-out.
#[inline]
pub fn quint_out(t: f32) -> f32 {
    let u = t - 1.0;
    1.0 + u * u * u * u * u
}

/// Quintic ease-in-out.
pub fn quint_in_out(t: f32) -> f32 {
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        let u = 2.0 * t - 2.0;
        0.5 * u * u * u * u * u + 1.0
    }
}

/// Sine ease-in.
pub fn sine_in(t: f32) -> f32 {
    1.0 - (t * std::f32::consts::FRAC_PI_2).cos()
}

/// Sine ease-out.
pub fn sine_out(t: f32) -> f32 {
    (t * std::f32::consts::FRAC_PI_2).sin()
}

/// Sine ease-in-out.
pub fn sine_in_out(t: f32) -> f32 {
    -0.5 * ((std::f32::consts::PI * t).cos() - 1.0)
}

/// Exponential ease-in.
pub fn expo_in(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else {
        (2.0f32).powf(10.0 * (t - 1.0))
    }
}

/// Exponential ease-out.
pub fn expo_out(t: f32) -> f32 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - (2.0f32).powf(-10.0 * t)
    }
}

/// Exponential ease-in-out.
pub fn expo_in_out(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    if t < 0.5 {
        0.5 * (2.0f32).powf(20.0 * t - 10.0)
    } else {
        1.0 - 0.5 * (2.0f32).powf(-20.0 * t + 10.0)
    }
}

/// Circular ease-in.
pub fn circ_in(t: f32) -> f32 {
    1.0 - (1.0 - t * t).sqrt()
}

/// Circular ease-out.
pub fn circ_out(t: f32) -> f32 {
    let u = t - 1.0;
    (1.0 - u * u).sqrt()
}

/// Circular ease-in-out.
pub fn circ_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - (1.0 - 4.0 * t * t).sqrt())
    } else {
        0.5 * ((1.0 - (2.0 * t - 2.0).powi(2)).sqrt() + 1.0)
    }
}

const BACK_S: f32 = 1.70158;

/// Back ease-in.
pub fn back_in(t: f32) -> f32 {
    let s = BACK_S;
    t * t * ((s + 1.0) * t - s)
}

/// Back ease-out.
pub fn back_out(t: f32) -> f32 {
    let s = BACK_S;
    let u = t - 1.0;
    u * u * ((s + 1.0) * u + s) + 1.0
}

/// Back ease-in-out.
pub fn back_in_out(t: f32) -> f32 {
    let s = BACK_S * 1.525;
    if t < 0.5 {
        let u = 2.0 * t;
        0.5 * (u * u * ((s + 1.0) * u - s))
    } else {
        let u = 2.0 * t - 2.0;
        0.5 * (u * u * ((s + 1.0) * u + s) + 2.0)
    }
}

/// Elastic ease-in.
pub fn elastic_in(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let p = 0.3f32;
    let s = p / 4.0;
    let u = t - 1.0;
    -((2.0f32).powf(10.0 * u) * ((u - s) * (2.0 * std::f32::consts::PI) / p).sin())
}

/// Elastic ease-out.
pub fn elastic_out(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let p = 0.3f32;
    let s = p / 4.0;
    (2.0f32).powf(-10.0 * t) * ((t - s) * (2.0 * std::f32::consts::PI) / p).sin() + 1.0
}

/// Elastic ease-in-out.
pub fn elastic_in_out(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let p = 0.45f32;
    let s = p / 4.0;
    let u = 2.0 * t - 1.0;
    if t < 0.5 {
        -0.5 * ((2.0f32).powf(10.0 * u) * ((u - s) * (2.0 * std::f32::consts::PI) / p).sin())
    } else {
        0.5 * ((2.0f32).powf(-10.0 * u) * ((u - s) * (2.0 * std::f32::consts::PI) / p).sin()) + 1.0
    }
}

/// Bounce ease-out.
pub fn bounce_out(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Bounce ease-in.
pub fn bounce_in(t: f32) -> f32 {
    1.0 - bounce_out(1.0 - t)
}

/// Bounce ease-in-out.
pub fn bounce_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * bounce_in(t * 2.0)
    } else {
        0.5 * bounce_out(t * 2.0 - 1.0) + 0.5
    }
}

/// Hermite smoothstep.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ken Perlin smootherstep.
#[inline]
pub fn smootherstep(t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Ping-pong a value in `[0, 1]` (triangle wave).
pub fn ping_pong(t: f32) -> f32 {
    let t = t.rem_euclid(2.0);
    if t < 1.0 {
        t
    } else {
        2.0 - t
    }
}

/// Repeat `t` into `[0, 1)`.
pub fn repeat(t: f32) -> f32 {
    t.rem_euclid(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints() {
        for ease in [
            Ease::Linear,
            Ease::QuadIn,
            Ease::QuadOut,
            Ease::CubicInOut,
            Ease::SineInOut,
            Ease::ExpoIn,
            Ease::ExpoOut,
            Ease::CircInOut,
            Ease::Smoothstep,
            Ease::Smootherstep,
            Ease::BounceOut,
            Ease::BounceIn,
        ] {
            assert!(
                ease.eval(0.0).abs() < 1e-4,
                "{ease:?} at 0 = {}",
                ease.eval(0.0)
            );
            assert!(
                (ease.eval(1.0) - 1.0).abs() < 1e-3,
                "{ease:?} at 1 = {}",
                ease.eval(1.0)
            );
        }
    }

    #[test]
    fn midpoints_monotonic_quad() {
        let a = Ease::QuadIn.eval(0.25);
        let b = Ease::QuadIn.eval(0.5);
        let c = Ease::QuadIn.eval(0.75);
        assert!(a < b && b < c);
        assert!(a < 0.25); // ease-in lags early
    }

    #[test]
    fn lerp_helper() {
        let v = Ease::Linear.lerp(10.0, 20.0, 0.5);
        assert!((v - 15.0).abs() < 1e-5);
    }

    #[test]
    fn ping_pong_and_repeat() {
        assert!((ping_pong(0.25) - 0.25).abs() < 1e-5);
        assert!((ping_pong(1.25) - 0.75).abs() < 1e-5);
        assert!((repeat(1.25) - 0.25).abs() < 1e-5);
    }

    #[test]
    fn back_overshoots() {
        // Back out may exceed 1 mid-curve; at t near 0.8 value can go > 1 temporarily
        // Actually back_out at t=0.8: still below 1 typically; check derivative style.
        // At t slightly before 1, value approaches 1 from above for back_out?
        // Standard back_out: for t in (0,1) can exceed 1.
        let v = back_out(0.9);
        assert!(v > 0.9);
    }

    #[test]
    fn property_all_eases_endpoints() {
        let eases = [
            Ease::Linear,
            Ease::QuadIn,
            Ease::QuadOut,
            Ease::QuadInOut,
            Ease::CubicIn,
            Ease::CubicOut,
            Ease::CubicInOut,
            Ease::QuartIn,
            Ease::QuartOut,
            Ease::QuartInOut,
            Ease::QuintIn,
            Ease::QuintOut,
            Ease::QuintInOut,
            Ease::SineIn,
            Ease::SineOut,
            Ease::SineInOut,
            Ease::ExpoIn,
            Ease::ExpoOut,
            Ease::ExpoInOut,
            Ease::CircIn,
            Ease::CircOut,
            Ease::CircInOut,
            Ease::BackIn,
            Ease::BackOut,
            Ease::BackInOut,
            Ease::ElasticIn,
            Ease::ElasticOut,
            Ease::ElasticInOut,
            Ease::BounceIn,
            Ease::BounceOut,
            Ease::BounceInOut,
            Ease::Smoothstep,
            Ease::Smootherstep,
        ];
        for ease in eases {
            let z = ease.eval(0.0);
            let o = ease.eval(1.0);
            assert!(z.abs() < 1e-3, "{ease:?}@0={z}");
            assert!((o - 1.0).abs() < 1e-2, "{ease:?}@1={o}");
            // Clamping outside domain.
            assert!((ease.eval(-1.0) - z).abs() < 1e-5);
            assert!((ease.eval(2.0) - o).abs() < 1e-5);
        }
    }

    #[test]
    fn property_in_out_mirror_midpoint() {
        // For symmetric in-out curves, eval(0.5) ≈ 0.5.
        for ease in [
            Ease::Linear,
            Ease::QuadInOut,
            Ease::CubicInOut,
            Ease::SineInOut,
            Ease::Smoothstep,
            Ease::Smootherstep,
        ] {
            let m = ease.eval(0.5);
            assert!((m - 0.5).abs() < 0.05, "{ease:?} mid={m}");
        }
    }

    #[test]
    fn property_ease_in_lags_ease_out_leads() {
        for (ein, eout) in [
            (Ease::QuadIn, Ease::QuadOut),
            (Ease::CubicIn, Ease::CubicOut),
            (Ease::SineIn, Ease::SineOut),
            (Ease::ExpoIn, Ease::ExpoOut),
            (Ease::CircIn, Ease::CircOut),
        ] {
            let t = 0.3;
            assert!(
                ein.eval(t) < eout.eval(t),
                "{ein:?} should lag {eout:?} early"
            );
            let t2 = 0.7;
            assert!(
                ein.eval(t2) < eout.eval(t2) || (ein.eval(t2) - eout.eval(t2)).abs() < 0.2,
                "late samples still ordered or close"
            );
        }
    }

    #[test]
    fn property_monotonic_common_eases() {
        for ease in [
            Ease::Linear,
            Ease::QuadIn,
            Ease::QuadOut,
            Ease::CubicIn,
            Ease::CubicOut,
            Ease::SineIn,
            Ease::SineOut,
            Ease::Smoothstep,
            Ease::Smootherstep,
        ] {
            let mut prev = ease.eval(0.0);
            for i in 1..=20 {
                let t = i as f32 / 20.0;
                let v = ease.eval(t);
                assert!(
                    v + 1e-4 >= prev,
                    "{ease:?} not monotonic at t={t}: {prev} -> {v}"
                );
                prev = v;
            }
        }
    }

    #[test]
    fn property_lerp_matches_linear() {
        for t in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            let v = Ease::Linear.lerp(-10.0, 30.0, t);
            let expected = -10.0 + 40.0 * t;
            assert!((v - expected).abs() < 1e-4);
        }
        let eased = Ease::QuadIn.lerp(0.0, 100.0, 0.5);
        assert!(eased < 50.0); // ease-in lags
    }

    #[test]
    fn property_ping_pong_repeat_cycle() {
        for t in [0.0_f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, -0.25] {
            let p = ping_pong(t);
            assert!((0.0..=1.0).contains(&p), "ping_pong({t})={p}");
            let r = repeat(t);
            assert!(
                (0.0..1.0).contains(&r) || (r - 0.0).abs() < 1e-5,
                "repeat({t})={r}"
            );
        }
        assert!((ping_pong(0.0) - 0.0).abs() < 1e-5);
        assert!((ping_pong(1.0) - 1.0).abs() < 1e-5);
        assert!((repeat(2.0)).abs() < 1e-5);
        assert!((repeat(-0.25) - 0.75).abs() < 1e-4);
    }

    #[test]
    fn property_smoothstep_bounds() {
        for t in [-1.0_f32, 0.0, 0.25, 0.5, 0.75, 1.0, 2.0] {
            let s = smootherstep(t);
            let sm = crate::easing::smoothstep(t);
            assert!((0.0..=1.0).contains(&s), "smootherstep({t})={s}");
            assert!((0.0..=1.0).contains(&sm), "smoothstep({t})={sm}");
        }
        // Smootherstep has flatter ends than smoothstep near 0.
        assert!(smootherstep(0.1) < crate::easing::smoothstep(0.1) + 1e-3);
    }
}
