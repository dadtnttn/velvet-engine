//! Statistical wave spectrum for visual water (Tessendorf-style).
//!
//! Three fixed sines look dead: real wind-seas are a *spectrum* of many
//! directional components whose frequencies obey the deep-water dispersion
//! relation `ω² = g·k` with independent random phases. Incommensurate
//! frequencies interfere into wave groups that grow and dissolve, so the
//! surface never visibly repeats.
//!
//! This is a discrete CPU-friendly version of Tessendorf's "Simulating
//! Ocean Water" (1999): Gerstner components, amplitudes from a
//! Phillips-like falloff (`A ∝ λ^1.5`, long waves carry the energy),
//! directions scattered around the wind, phases from a seeded PRNG.
//! Pure `f32` math, no engine dependencies; determinism is bitwise for a
//! given seed (visuals must not depend on run order).
//!
//! The physics sim underneath ([`crate::shallow`]) is untouched: the
//! spectrum only displaces the presentation mesh.

use std::f32::consts::TAU;

/// Gravity in the deep-water dispersion relation `ω = sqrt(g·k)`.
pub const GRAVITY: f32 = 9.81;

/// One directional Gerstner component with its own random phase.
#[derive(Debug, Clone, Copy)]
pub struct SpectralWave {
    /// XZ travel direction (unit length).
    pub dir: [f32; 2],
    /// Crest height in world units.
    pub amplitude: f32,
    /// Wavelength in world units.
    pub wavelength: f32,
    /// Angular frequency in rad/s (from dispersion, not chosen).
    pub speed: f32,
    /// Crest sharpness weight (energy-normalized, sums to < 1).
    pub sharpness: f32,
    /// Random initial phase in radians (what kills static patterns).
    pub phase: f32,
}

/// Deterministic SplitMix64 PRNG (visual phases must be seed-stable).
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        // Top 24 bits -> [0, 1).
        ((self.next_u64() >> 40) as f32) / 16_777_216.0
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// A wind-sea: many [`SpectralWave`] components acting as one surface.
#[derive(Debug, Clone)]
pub struct WaveSpectrum {
    waves: Vec<SpectralWave>,
}

impl WaveSpectrum {
    /// Build a fetch-limited wind-sea.
    ///
    /// * `wind_dir` — prevailing wind (XZ, normalized internally).
    /// * `wind_speed` — m/s; sets directional spread (light wind wanders,
    ///   strong wind aligns) and crest sharpness.
    /// * `max_wavelength` — longest component (fit it to the water body).
    /// * `chop_height` — summed crest height; the surface stays within
    ///   roughly `±chop_height` of still level.
    /// * `modes` — component count (48 is plenty on CPU).
    pub fn wind_sea(
        seed: u64,
        wind_dir: [f32; 2],
        wind_speed: f32,
        max_wavelength: f32,
        chop_height: f32,
        modes: usize,
    ) -> Self {
        let modes = modes.clamp(4, 256);
        let wlen = (wind_dir[0] * wind_dir[0] + wind_dir[1] * wind_dir[1])
            .sqrt()
            .max(1e-6);
        let (wx, wz) = (wind_dir[0] / wlen, wind_dir[1] / wlen);
        let base_ang = wz.atan2(wx);
        // Light wind meanders, strong wind locks direction.
        let spread = (14.0 / wind_speed.max(2.0)).clamp(0.35, 1.2);
        let q_total = (0.35 + wind_speed * 0.045).clamp(0.4, 0.85);
        let mut rng = Rng(seed ^ 0x0CEA_001C);
        // Geometric ladder: a few long carriers + a tail of chop.
        let ratio = 1.0 / 18.0f32.powf(1.0 / (modes - 1) as f32);
        let mut raw: Vec<(f32, f32, f32)> = Vec::with_capacity(modes);
        let mut energy = 0.0f32;
        for m in 0..modes {
            let lambda = max_wavelength * ratio.powi(m as i32);
            // Phillips-like falloff: long waves carry the energy.
            let shape = (lambda / max_wavelength).powf(1.5);
            // Short waves scatter away from the wind, long ones align.
            let jitter = spread * (0.3 + 0.7 * (1.0 - lambda / max_wavelength));
            let ang = base_ang + rng.range(-jitter, jitter);
            // Directional alignment: downwind modes keep their energy.
            let align = ((ang - base_ang).cos() * 0.5 + 0.5).powi(2);
            let e = shape * (0.25 + 0.75 * align) * rng.range(0.55, 1.0);
            energy += e;
            raw.push((lambda, ang, e));
        }
        let scale = chop_height.max(0.0) / energy.max(1e-9);
        let waves = raw
            .into_iter()
            .map(|(lambda, ang, e)| {
                let amp = e * scale;
                let k = TAU / lambda.max(0.01);
                WaveSpectrum::one(
                    ang,
                    amp,
                    lambda,
                    k,
                    q_total * e / energy.max(1e-9),
                    &mut rng,
                )
            })
            .collect();
        Self { waves }
    }

    fn one(
        ang: f32,
        amplitude: f32,
        wavelength: f32,
        k: f32,
        sharpness: f32,
        rng: &mut Rng,
    ) -> SpectralWave {
        SpectralWave {
            dir: [ang.cos(), ang.sin()],
            amplitude,
            wavelength,
            // Deep-water dispersion: each wavelength travels at its own
            // physical speed, so interference patterns evolve over time.
            speed: (GRAVITY * k).sqrt(),
            sharpness: sharpness.clamp(0.0, 1.0),
            phase: rng.next_f32() * TAU,
        }
    }

    /// Component list (read-only, e.g. for stats or baking).
    pub fn waves(&self) -> &[SpectralWave] {
        &self.waves
    }

    /// Summed crest height (surface stays within roughly ± this).
    pub fn crest_height(&self) -> f32 {
        self.waves.iter().map(|w| w.amplitude).sum()
    }

    /// Still-water height at `(x, z, t)` (vertical part of the Gerstner sum).
    pub fn height(&self, x: f32, z: f32, t: f32) -> f32 {
        let mut h = 0.0f32;
        for w in &self.waves {
            let k = TAU / w.wavelength.max(0.01);
            let phase = k * (w.dir[0] * x + w.dir[1] * z) - w.speed * t + w.phase;
            h += w.amplitude * phase.sin();
        }
        h
    }

    /// Full Gerstner displacement `[dx, dy, dz]` (visual only).
    pub fn displace(&self, x: f32, z: f32, t: f32) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for w in &self.waves {
            let k = TAU / w.wavelength.max(0.01);
            let q = w.sharpness / (k * w.amplitude.max(1e-6));
            let phase = k * (w.dir[0] * x + w.dir[1] * z) - w.speed * t + w.phase;
            let (s, c) = phase.sin_cos();
            out[0] += q * w.amplitude * w.dir[0] * c;
            out[1] += w.amplitude * s;
            out[2] += q * w.amplitude * w.dir[1] * c;
        }
        out
    }

    /// Analytic surface gradient `[dh/dx, dh/dz]` of [`Self::height`]
    /// (exact shading normals, no finite-difference shimmer).
    pub fn gradient(&self, x: f32, z: f32, t: f32) -> [f32; 2] {
        let mut g = [0.0f32; 2];
        for w in &self.waves {
            let k = TAU / w.wavelength.max(0.01);
            let phase = k * (w.dir[0] * x + w.dir[1] * z) - w.speed * t + w.phase;
            let slope = w.amplitude * k * phase.cos();
            g[0] += slope * w.dir[0];
            g[1] += slope * w.dir[1];
        }
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lake() -> WaveSpectrum {
        WaveSpectrum::wind_sea(7, [1.0, 0.35], 7.0, 34.0, 0.9, 48)
    }

    #[test]
    fn spectrum_is_deterministic_per_seed() {
        let (a, b) = (lake(), lake());
        assert_eq!(a.waves.len(), b.waves.len());
        for (x, y) in a.waves.iter().zip(b.waves.iter()) {
            assert_eq!(x.dir, y.dir);
            assert_eq!(x.amplitude, y.amplitude);
            assert_eq!(x.wavelength, y.wavelength);
            assert_eq!(x.speed, y.speed);
            assert_eq!(x.phase, y.phase);
        }
        // Different seeds give different phases.
        let c = WaveSpectrum::wind_sea(8, [1.0, 0.35], 7.0, 34.0, 0.9, 48);
        assert!(c
            .waves
            .iter()
            .zip(a.waves.iter())
            .any(|(x, y)| (x.phase - y.phase).abs() > 1e-3));
    }

    #[test]
    fn modes_obey_deep_water_dispersion() {
        for w in lake().waves() {
            let k = TAU / w.wavelength;
            let expect = (GRAVITY * k).sqrt();
            assert!(
                (w.speed - expect).abs() < 1e-5,
                "mode λ={} ω={} expected {expect}",
                w.wavelength,
                w.speed
            );
        }
    }

    #[test]
    fn crest_height_matches_target() {
        let s = lake();
        assert!((s.crest_height() - 0.9).abs() < 0.01);
        // Sharpness budget keeps crests loop-free.
        let q: f32 = s.waves.iter().map(|w| w.sharpness).sum();
        assert!(q <= 1.0, "sharpness budget blown: {q}");
    }

    #[test]
    fn surface_evolves_without_repeating() {
        let s = lake();
        // Mean absolute change over a probe grid (single points are noisy).
        let mean_change = |t: f32| {
            let mut acc = 0.0f32;
            for j in 0..8 {
                for i in 0..8 {
                    let x = -40.0 + i as f32 * 11.4;
                    let z = -40.0 + j as f32 * 11.4;
                    acc += (s.height(x, z, t) - s.height(x, z, 0.0)).abs();
                }
            }
            acc / 64.0
        };
        let drift = mean_change(37.7);
        assert!(drift > 0.04, "frozen surface, mean change {drift}");
        // One full period of the LONGEST carrier must not restore the
        // surface (incommensurate modes => no visible loop).
        let slow = s
            .waves()
            .iter()
            .min_by(|a, b| a.speed.partial_cmp(&b.speed).unwrap())
            .unwrap();
        let period = TAU / slow.speed;
        let loop_err = mean_change(period);
        assert!(
            loop_err > 0.02,
            "surface repeats at carrier period {period:.1}s (err {loop_err})"
        );
    }

    #[test]
    fn analytic_gradient_matches_height() {
        let s = WaveSpectrum::wind_sea(3, [1.0, 0.0], 6.0, 20.0, 0.5, 8);
        let (x, z, t) = (12.5, -7.25, 4.2);
        let e = 0.02;
        let fx = (s.height(x + e, z, t) - s.height(x - e, z, t)) / (2.0 * e);
        let fz = (s.height(x, z + e, t) - s.height(x, z - e, t)) / (2.0 * e);
        let g = s.gradient(x, z, t);
        assert!((fx - g[0]).abs() < 1e-3, "{fx} vs {}", g[0]);
        assert!((fz - g[1]).abs() < 1e-3, "{fz} vs {}", g[1]);
    }
}
