//! FxPreset — JSON-serializable effect recipe (BitFX-like).

use serde::{Deserialize, Serialize};

use crate::emitter::EmitterConfig;
use crate::palette::PaletteId;
use crate::path::PathConfig;

/// Canvas dimensions (pixel art frame size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    /// Width px (e.g. 32..=128)
    pub width: u32,
    /// Height px
    pub height: u32,
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            width: 64,
            height: 64,
        }
    }
}

impl Dimensions {
    /// Clamp to valid range.
    pub fn clamped(self) -> Self {
        Self {
            width: self.width.clamp(16, 256),
            height: self.height.clamp(16, 256),
        }
    }
    /// Count pixels.
    pub fn len(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Empty dims? (zero area)
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Loop mode (how to handle seamless looping).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopMode {
    /// Seamless wrap (particles respawn, loop perfect)
    #[default]
    Seamless,
    /// Ping-pong (bounce time)
    PingPong,
}

/// Particle behaviour config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleConfig {
    /// Max particles alive at once
    pub max_particles: usize,
    /// Lifetime seconds min/max
    pub lifetime_min: f32,
    /// Lifetime max
    pub lifetime_max: f32,
    /// Speed min/max (px/s, canvas space)
    pub speed_min: f32,
    /// Speed max
    pub speed_max: f32,
    /// Gravity Y (px/s², negative = up like smoke)
    pub gravity: f32,
    /// Drag 0..=1 (air friction)
    pub drag: f32,
    /// Jitter (random offset per frame, px)
    pub jitter: f32,
    /// Size px (1 or 2 typical for pixel art)
    pub size: u32,
    /// Flicker (random alpha/size variation)
    pub flicker: f32,
    /// Fade curve exponent (1 = linear)
    pub fade_pow: f32,
    /// Shrink over life (0..=1)
    pub shrink: f32,
    /// Spin (radians/s, for future sprite rotation; visual wobble for squares)
    pub spin: f32,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            max_particles: 600,
            lifetime_min: 0.4,
            lifetime_max: 1.1,
            speed_min: 8.0,
            speed_max: 28.0,
            gravity: -12.0,
            drag: 0.6,
            jitter: 0.4,
            size: 1,
            flicker: 0.15,
            fade_pow: 1.2,
            shrink: 0.0,
            spin: 0.0,
        }
    }
}

/// Metadata (name, author, tags).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FxMeta {
    /// Display name
    pub name: String,
    /// Optional author
    pub author: String,
    /// Tags
    pub tags: Vec<String>,
    /// Version (semver-ish)
    pub version: String,
}

impl Default for FxMeta {
    fn default() -> Self {
        Self {
            name: "Untitled FX".into(),
            author: String::new(),
            tags: Vec::new(),
            version: "1.0.0".into(),
        }
    }
}

/// Full preset (save/load JSON, drive sim).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxPreset {
    /// Meta
    pub meta: FxMeta,
    /// Canvas dimensions
    pub dimensions: Dimensions,
    /// Total frames for spritesheet loop
    pub frames: u32,
    /// FPS virtual (for time scaling)
    pub fps: u32,
    /// Loop mode
    pub loop_mode: LoopMode,
    /// Palette id
    pub palette: PaletteId,
    /// Optional palette override colors (if non-empty, use these instead of builtin)
    pub palette_override: Vec<[u8; 4]>,
    /// Path
    pub path: PathConfig,
    /// Emitter
    pub emitter: EmitterConfig,
    /// Particles
    pub particles: ParticleConfig,
    /// Background color (RGB, alpha 255) — usually transparent or dark
    pub background: [u8; 4],
    /// Seed (deterministic)
    pub seed: u64,
    /// Bloom / additive flag (for preview; particles drawn additive-ish)
    pub additive: bool,
}

impl Default for FxPreset {
    fn default() -> Self {
        Self {
            meta: FxMeta::default(),
            dimensions: Dimensions::default(),
            frames: 16,
            fps: 12,
            loop_mode: LoopMode::Seamless,
            palette: PaletteId::Fire,
            palette_override: Vec::new(),
            path: PathConfig::default(),
            emitter: EmitterConfig::default(),
            particles: ParticleConfig::default(),
            background: [0, 0, 0, 0],
            seed: 0xC0FFEE,
            additive: false,
        }
    }
}

impl FxPreset {
    /// New with seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed: seed | 1,
            ..Default::default()
        }
    }

    /// Validate + clamp ranges (call after deserialization or mutation).
    pub fn validated(mut self) -> Self {
        self.dimensions = self.dimensions.clamped();
        self.frames = self.frames.clamp(4, 64);
        self.fps = self.fps.clamp(6, 30);
        self.particles.max_particles = self.particles.max_particles.clamp(32, 4096);
        self.particles.lifetime_min = self.particles.lifetime_min.clamp(0.05, 5.0);
        self.particles.lifetime_max = self
            .particles
            .lifetime_max
            .clamp(self.particles.lifetime_min, 5.0);
        self.particles.speed_min = self.particles.speed_min.clamp(0.0, 300.0);
        self.particles.speed_max = self
            .particles
            .speed_max
            .clamp(self.particles.speed_min, 400.0);
        self.particles.drag = self.particles.drag.clamp(0.0, 1.0);
        self.particles.flicker = self.particles.flicker.clamp(0.0, 1.0);
        self.particles.size = self.particles.size.clamp(1, 4);
        self.particles.fade_pow = self.particles.fade_pow.clamp(0.2, 4.0);
        self.particles.shrink = self.particles.shrink.clamp(0.0, 1.0);
        self.path.scale_x = self.path.scale_x.clamp(0.1, 3.0);
        self.path.scale_y = self.path.scale_y.clamp(0.1, 3.0);
        self.path.frequency = self.path.frequency.clamp(0.2, 4.0);
        self.emitter.rate = self.emitter.rate.clamp(4.0, 800.0);
        self.emitter.spread = self.emitter.spread.clamp(0.0, std::f32::consts::PI);
        if self.palette_override.len() > 16 {
            self.palette_override.truncate(16);
        }
        if self.seed == 0 {
            self.seed = 0xC0FFEE;
        }
        self
    }

    /// Short preset id (for filenames): name lowercased, alnum -> dash.
    pub fn file_stem(&self) -> String {
        let mut s = String::new();
        for c in self.meta.name.chars() {
            if c.is_ascii_alphanumeric() {
                s.push(c.to_ascii_lowercase());
            } else if c.is_ascii_whitespace() || c == '-' || c == '_' {
                s.push('-');
            }
        }
        let t = s.trim_matches('-').to_string();
        if t.is_empty() {
            "fx".into()
        } else {
            t
        }
    }

    /// To JSON string pretty.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// From JSON string.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Random factory (alias).
    pub fn random(seed: u64) -> Self {
        crate::randomize::random_preset(seed)
    }

    /// Duration seconds (frames / fps)
    pub fn duration(&self) -> f32 {
        self.frames as f32 / self.fps as f32
    }

    /// Effective palette colors (override or builtin).
    pub fn effective_palette(&self) -> Vec<[u8; 4]> {
        if !self.palette_override.is_empty() {
            self.palette_override.clone()
        } else {
            crate::palette::palette_colors(self.palette).to_vec()
        }
    }

    /// Built-in example presets (for UI picker fallback).
    pub fn examples() -> Vec<Self> {
        vec![
            Self::fire(),
            Self::smoke(),
            Self::magic(),
            Self::lightning(),
            Self::portal(),
            Self::dna(),
            Self::embers(),
            Self::heal(),
        ]
    }

    /// Fire preset.
    pub fn fire() -> Self {
        Self {
            meta: FxMeta {
                name: "Fire".into(),
                tags: vec!["fire".into()],
                ..Default::default()
            },
            dimensions: Dimensions {
                width: 64,
                height: 64,
            },
            frames: 16,
            fps: 12,
            palette: PaletteId::Fire,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::LineH,
                scale_x: 0.35,
                scale_y: 1.0,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                shape: crate::emitter::EmitterShape::Cone,
                rate: 110.0,
                spread: 0.65,
                angle: -std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: -18.0,
                drag: 0.7,
                speed_min: 10.0,
                speed_max: 30.0,
                lifetime_min: 0.5,
                lifetime_max: 1.0,
                jitter: 0.5,
                flicker: 0.18,
                ..Default::default()
            },
            seed: 0xF1A2,
            ..Default::default()
        }
        .validated()
    }

    /// Smoke.
    pub fn smoke() -> Self {
        Self {
            meta: FxMeta {
                name: "Smoke".into(),
                tags: vec!["smoke".into()],
                ..Default::default()
            },
            palette: PaletteId::Smoke,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::Point,
                scale_x: 1.0,
                scale_y: 1.0,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                rate: 45.0,
                spread: 0.9,
                angle: -std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: -6.0,
                drag: 0.35,
                speed_min: 6.0,
                speed_max: 16.0,
                lifetime_min: 1.0,
                lifetime_max: 2.0,
                size: 2,
                jitter: 0.8,
                flicker: 0.12,
                fade_pow: 1.6,
                ..Default::default()
            },
            seed: 0x5A06,
            ..Default::default()
        }
        .validated()
    }

    /// Magic arcane swirl.
    pub fn magic() -> Self {
        Self {
            meta: FxMeta {
                name: "Magic".into(),
                tags: vec!["magic".into()],
                ..Default::default()
            },
            palette: PaletteId::Arcane,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::Circle,
                scale_x: 0.55,
                scale_y: 0.55,
                frequency: 1.0,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                shape: crate::emitter::EmitterShape::Ring,
                rate: 80.0,
                spread: std::f32::consts::PI,
                angle: 0.0,
                align_to_path: false,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: -2.0,
                drag: 0.55,
                speed_min: 4.0,
                speed_max: 14.0,
                lifetime_min: 0.7,
                lifetime_max: 1.4,
                jitter: 0.6,
                flicker: 0.22,
                ..Default::default()
            },
            seed: 0xAAC0,
            additive: true,
            ..Default::default()
        }
        .validated()
    }

    /// Lightning / electric zig.
    pub fn lightning() -> Self {
        Self {
            meta: FxMeta {
                name: "Lightning".into(),
                tags: vec!["electric".into()],
                ..Default::default()
            },
            palette: PaletteId::Electric,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::LineV,
                scale_x: 1.0,
                scale_y: 0.9,
                frequency: 2.2,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                rate: 140.0,
                spread: 0.45,
                angle: 0.0,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: 0.0,
                drag: 0.9,
                speed_min: 30.0,
                speed_max: 70.0,
                lifetime_min: 0.15,
                lifetime_max: 0.35,
                size: 1,
                jitter: 1.2,
                flicker: 0.4,
                ..Default::default()
            },
            seed: 0xE1EC,
            additive: true,
            ..Default::default()
        }
        .validated()
    }

    /// Portal (double ring).
    pub fn portal() -> Self {
        Self {
            meta: FxMeta {
                name: "Portal".into(),
                tags: vec!["portal".into()],
                ..Default::default()
            },
            palette: PaletteId::Void,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::Circle,
                scale_x: 0.62,
                scale_y: 0.62,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                shape: crate::emitter::EmitterShape::Ring,
                rate: 100.0,
                spread: 0.3,
                angle: 0.0,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: 0.0,
                drag: 0.45,
                speed_min: 6.0,
                speed_max: 18.0,
                lifetime_min: 0.6,
                lifetime_max: 1.2,
                jitter: 0.3,
                flicker: 0.18,
                ..Default::default()
            },
            seed: 0xB07A,
            additive: true,
            ..Default::default()
        }
        .validated()
    }

    /// DNA helix.
    pub fn dna() -> Self {
        Self {
            meta: FxMeta {
                name: "DNA".into(),
                tags: vec!["dna".into()],
                ..Default::default()
            },
            dimensions: Dimensions {
                width: 48,
                height: 96,
            },
            palette: PaletteId::Toxic,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::Dna,
                scale_x: 0.55,
                scale_y: 0.95,
                frequency: 1.8,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                rate: 95.0,
                spread: 0.25,
                angle: 0.0,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: 0.0,
                drag: 0.75,
                speed_min: 2.0,
                speed_max: 8.0,
                lifetime_min: 0.8,
                lifetime_max: 1.5,
                size: 1,
                jitter: 0.25,
                flicker: 0.1,
                ..Default::default()
            },
            seed: 0xD1A0,
            ..Default::default()
        }
        .validated()
    }

    /// Embers.
    pub fn embers() -> Self {
        Self {
            meta: FxMeta {
                name: "Embers".into(),
                tags: vec!["ember".into()],
                ..Default::default()
            },
            palette: PaletteId::Ember,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::Wave,
                scale_x: 0.7,
                scale_y: 0.3,
                frequency: 1.4,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                rate: 60.0,
                spread: 1.0,
                angle: -std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: -14.0,
                drag: 0.5,
                speed_min: 8.0,
                speed_max: 22.0,
                lifetime_min: 0.6,
                lifetime_max: 1.3,
                jitter: 0.7,
                flicker: 0.25,
                ..Default::default()
            },
            seed: 0xE22A,
            additive: true,
            ..Default::default()
        }
        .validated()
    }

    /// Heal sparkle.
    pub fn heal() -> Self {
        Self {
            meta: FxMeta {
                name: "Heal".into(),
                tags: vec!["heal".into()],
                ..Default::default()
            },
            palette: PaletteId::Gold,
            path: crate::path::PathConfig {
                kind: crate::path::PathKind::Spiral,
                scale_x: 0.5,
                scale_y: 0.5,
                frequency: 1.6,
                ..Default::default()
            },
            emitter: crate::emitter::EmitterConfig {
                rate: 70.0,
                spread: 0.6,
                angle: -std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            particles: ParticleConfig {
                gravity: -8.0,
                drag: 0.6,
                speed_min: 6.0,
                speed_max: 16.0,
                lifetime_min: 0.8,
                lifetime_max: 1.6,
                size: 1,
                jitter: 0.5,
                flicker: 0.2,
                ..Default::default()
            },
            seed: 0x1EA7,
            additive: true,
            ..Default::default()
        }
        .validated()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let p = FxPreset::fire();
        let s = p.to_json().unwrap();
        let back = FxPreset::from_json(&s).unwrap();
        assert_eq!(p.meta.name, back.meta.name);
        assert_eq!(p.palette, back.palette);
    }

    #[test]
    fn validated_clamps() {
        let p = FxPreset {
            frames: 999,
            fps: 0,
            ..Default::default()
        }
        .validated();
        assert!(p.frames <= 64);
        assert!(p.fps >= 6);
    }
}
