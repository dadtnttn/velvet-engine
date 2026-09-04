//! Randomize / mutate — BitFX's headline feature.

use crate::emitter::EmitterShape;
use crate::palette::PaletteId;
use crate::path::{PathConfig, PathKind};
use crate::preset::{Dimensions, FxPreset, LoopMode, ParticleConfig};

fn rng_next(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

fn rand_f32(state: &mut u64) -> f32 {
    (rng_next(state) >> 32) as f32 / u32::MAX as f32
}

fn rand_range(state: &mut u64, a: f32, b: f32) -> f32 {
    a + (b - a) * rand_f32(state)
}

fn pick<'a, T>(state: &mut u64, slice: &'a [T]) -> &'a T {
    let idx = (rand_f32(state) * slice.len() as f32).floor() as usize % slice.len();
    &slice[idx]
}

/// Create a fully random preset from seed.
pub fn random_preset(seed: u64) -> FxPreset {
    let mut s = seed | 1;
    let palette = *pick(&mut s, PaletteId::all());
    let path_kind = *pick(&mut s, PathKind::all());
    let dims_choice = (rand_f32(&mut s) * 5.0).floor() as i32;
    let dimensions = match dims_choice {
        0 => Dimensions {
            width: 32,
            height: 32,
        },
        1 => Dimensions {
            width: 48,
            height: 48,
        },
        2 => Dimensions {
            width: 64,
            height: 64,
        },
        3 => Dimensions {
            width: 96,
            height: 96,
        },
        _ => Dimensions {
            width: 64,
            height: 96,
        },
    };
    let frames = [8, 12, 16, 24, 32][(rand_f32(&mut s) * 5.0).floor() as usize % 5];
    let fps = [8, 12, 16, 24][(rand_f32(&mut s) * 4.0).floor() as usize % 4];
    let emitter_shape = *pick(
        &mut s,
        &[
            EmitterShape::Point,
            EmitterShape::Cone,
            EmitterShape::Circle,
            EmitterShape::Ring,
            EmitterShape::Directed,
        ],
    );
    let path = PathConfig {
        kind: path_kind,
        scale_x: rand_range(&mut s, 0.25, 1.6),
        scale_y: rand_range(&mut s, 0.25, 1.6),
        phase: rand_range(&mut s, 0.0, std::f32::consts::TAU),
        frequency: rand_range(&mut s, 0.6, 2.8),
        twist: rand_range(&mut s, 0.8, 1.4),
        segments: 24,
    };
    let particles = ParticleConfig {
        max_particles: 600,
        lifetime_min: rand_range(&mut s, 0.25, 0.7),
        lifetime_max: rand_range(&mut s, 0.8, 2.0),
        speed_min: rand_range(&mut s, 4.0, 16.0),
        speed_max: rand_range(&mut s, 18.0, 55.0),
        gravity: rand_range(&mut s, -20.0, 6.0),
        drag: rand_range(&mut s, 0.25, 0.78),
        jitter: rand_range(&mut s, 0.2, 1.1),
        size: if rand_f32(&mut s) > 0.7 { 2 } else { 1 },
        flicker: rand_range(&mut s, 0.05, 0.35),
        fade_pow: rand_range(&mut s, 0.8, 2.2),
        shrink: if rand_f32(&mut s) > 0.6 {
            rand_range(&mut s, 0.0, 0.5)
        } else {
            0.0
        },
        spin: 0.0,
    };
    let gravity_bias = match palette {
        PaletteId::Fire | PaletteId::Ember | PaletteId::Smoke => rand_range(&mut s, -18.0, -4.0),
        PaletteId::Aqua | PaletteId::Blood => rand_range(&mut s, 2.0, 12.0),
        _ => rand_range(&mut s, -8.0, 4.0),
    };
    let mut preset = FxPreset {
        meta: crate::preset::FxMeta {
            name: format!("Random {}", (s & 0xFFFF) as u16),
            tags: vec!["random".into()],
            ..Default::default()
        },
        dimensions,
        frames,
        fps,
        loop_mode: if rand_f32(&mut s) > 0.3 {
            LoopMode::Seamless
        } else {
            LoopMode::PingPong
        },
        palette,
        palette_override: Vec::new(),
        path,
        emitter: crate::emitter::EmitterConfig {
            shape: emitter_shape,
            rate: rand_range(&mut s, 45.0, 220.0),
            burst_count: (rand_range(&mut s, 12.0, 36.0) as u32).max(4),
            continuous: rand_f32(&mut s) > 0.25,
            spread: rand_range(&mut s, 0.25, 2.4),
            angle: rand_range(&mut s, -std::f32::consts::PI, std::f32::consts::PI),
            align_to_path: rand_f32(&mut s) > 0.35,
        },
        particles: ParticleConfig {
            gravity: gravity_bias,
            ..particles
        },
        background: if rand_f32(&mut s) > 0.7 {
            [12, 10, 18, 255]
        } else {
            [0, 0, 0, 0]
        },
        seed: s,
        additive: matches!(
            palette,
            PaletteId::Fire
                | PaletteId::Arcane
                | PaletteId::Electric
                | PaletteId::Ember
                | PaletteId::Gold
        ) && rand_f32(&mut s) > 0.4,
    };
    preset = preset.validated();
    preset
}

/// Mutate an existing preset slightly (small tweaks, keep identity).
pub fn mutate_preset(base: &FxPreset, seed: u64) -> FxPreset {
    let mut s = seed ^ base.seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut p = base.clone();
    // mutate one or two fields
    let choice = (rand_f32(&mut s) * 6.0).floor() as i32;
    match choice {
        0 => {
            // palette
            p.palette = *pick(&mut s, PaletteId::all());
            p.palette_override.clear();
        }
        1 => {
            // path scale
            p.path.scale_x = (p.path.scale_x + rand_range(&mut s, -0.25, 0.25)).clamp(0.1, 3.0);
            p.path.scale_y = (p.path.scale_y + rand_range(&mut s, -0.25, 0.25)).clamp(0.1, 3.0);
            p.path.frequency = (p.path.frequency + rand_range(&mut s, -0.4, 0.4)).clamp(0.2, 4.0);
        }
        2 => {
            // path kind
            p.path.kind = *pick(&mut s, PathKind::all());
        }
        3 => {
            // particles gravity/speed
            p.particles.gravity =
                (p.particles.gravity + rand_range(&mut s, -6.0, 6.0)).clamp(-30.0, 20.0);
            p.particles.speed_max = (p.particles.speed_max + rand_range(&mut s, -8.0, 8.0))
                .clamp(p.particles.speed_min, 120.0);
            p.particles.jitter =
                (p.particles.jitter + rand_range(&mut s, -0.2, 0.2)).clamp(0.0, 2.0);
        }
        4 => {
            // emitter
            p.emitter.spread =
                (p.emitter.spread + rand_range(&mut s, -0.3, 0.3)).clamp(0.0, std::f32::consts::PI);
            p.emitter.rate = (p.emitter.rate + rand_range(&mut s, -30.0, 30.0)).clamp(8.0, 700.0);
            p.emitter.angle += rand_range(&mut s, -0.5, 0.5);
        }
        _ => {
            // dimensions
            p.dimensions.width = ((p.dimensions.width as f32 + rand_range(&mut s, -16.0, 16.0))
                .round() as u32)
                .clamp(16, 128);
            p.dimensions.height = ((p.dimensions.height as f32 + rand_range(&mut s, -16.0, 16.0))
                .round() as u32)
                .clamp(16, 128);
        }
    }
    p.seed = s;
    p.meta.name = format!("{}*", base.meta.name);
    p.validated()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_deterministic() {
        let a = random_preset(123);
        let b = random_preset(123);
        assert_eq!(a.seed, b.seed);
        assert_eq!(a.palette, b.palette);
    }

    #[test]
    fn mutate_changes_something() {
        let base = crate::preset::FxPreset::fire();
        let m = mutate_preset(&base, 999);
        // at least one field differs (probabilistic but deterministic seed ensures it)
        assert!(
            m.palette != base.palette
                || m.path.kind != base.path.kind
                || (m.path.scale_x - base.path.scale_x).abs() > 1e-3
                || m.emitter.rate != base.emitter.rate
        );
    }
}
