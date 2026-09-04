//! Emitter paths — how spawn positions move over time.
//! BitFX has tweakable path width/height sliders; we expose `scale` + `phase`.

use serde::{Deserialize, Serialize};

/// Kind of spawn path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    /// Single point (center).
    Point,
    /// Horizontal line
    LineH,
    /// Vertical line
    LineV,
    /// Circle ring
    Circle,
    /// Arc (half circle)
    Arc,
    /// Sine wave horizontally
    Wave,
    /// Figure-8 / lemniscate
    Figure8,
    /// Spiral
    Spiral,
    /// DNA double helix (two interleaved sines) — BitFX favourite
    Dna,
    /// Grid scattered
    Grid,
    /// Bezier curve (quad)
    Bezier,
}

impl PathKind {
    /// All kinds.
    pub fn all() -> &'static [Self] {
        &[
            Self::Point,
            Self::LineH,
            Self::LineV,
            Self::Circle,
            Self::Arc,
            Self::Wave,
            Self::Figure8,
            Self::Spiral,
            Self::Dna,
            Self::Grid,
            Self::Bezier,
        ]
    }

    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Point => "Point",
            Self::LineH => "Line H",
            Self::LineV => "Line V",
            Self::Circle => "Circle",
            Self::Arc => "Arc",
            Self::Wave => "Wave",
            Self::Figure8 => "Figure-8",
            Self::Spiral => "Spiral",
            Self::Dna => "DNA",
            Self::Grid => "Grid",
            Self::Bezier => "Bezier",
        }
    }

    /// Parse.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "point" => Some(Self::Point),
            "line_h" | "lineh" | "hline" | "line" => Some(Self::LineH),
            "line_v" | "linev" | "vline" => Some(Self::LineV),
            "circle" | "ring" => Some(Self::Circle),
            "arc" => Some(Self::Arc),
            "wave" | "sine" => Some(Self::Wave),
            "figure8" | "figure_8" | "8" | "lemniscate" => Some(Self::Figure8),
            "spiral" => Some(Self::Spiral),
            "dna" | "helix" | "double" => Some(Self::Dna),
            "grid" | "scatter" => Some(Self::Grid),
            "bezier" | "curve" => Some(Self::Bezier),
            _ => None,
        }
    }
}

/// Path configuration (size sliders = scale_x/scale_y).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathConfig {
    /// Kind.
    pub kind: PathKind,
    /// Width scale (0.1..=2.0) — requested BitFX feature
    pub scale_x: f32,
    /// Height scale
    pub scale_y: f32,
    /// Phase offset (for animation)
    pub phase: f32,
    /// Frequency (how tight waves/spiral)
    pub frequency: f32,
    /// Twist for DNA (helix twist)
    pub twist: f32,
    /// Segments for sampling (resolution)
    pub segments: u32,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            kind: PathKind::Point,
            scale_x: 1.0,
            scale_y: 1.0,
            phase: 0.0,
            frequency: 1.0,
            twist: 1.0,
            segments: 24,
        }
    }
}

impl PathConfig {
    /// Sample t 0..=1 (along path) + global time for animated paths.
    pub fn sample(&self, t: f32, time: f32) -> PathSample {
        let t = t.clamp(0.0, 1.0);
        let sx = self.scale_x.clamp(0.1, 3.0);
        let sy = self.scale_y.clamp(0.1, 3.0);
        let phase = self.phase + time * 0.8;
        let freq = self.frequency.clamp(0.2, 4.0);
        let (x, y, nx, ny) = match self.kind {
            PathKind::Point => (0.0, 0.0, 0.0, 1.0),
            PathKind::LineH => {
                let x = (t - 0.5) * 2.0 * sx;
                (x, 0.0, 0.0, 1.0)
            }
            PathKind::LineV => {
                let y = (t - 0.5) * 2.0 * sy;
                (0.0, y, 1.0, 0.0)
            }
            PathKind::Circle => {
                let a = t * std::f32::consts::TAU + phase;
                (a.cos() * sx, a.sin() * sy, a.cos(), a.sin())
            }
            PathKind::Arc => {
                let a = t * std::f32::consts::PI + phase;
                (a.cos() * sx, a.sin() * sy * 0.5, a.cos(), a.sin())
            }
            PathKind::Wave => {
                let x = (t - 0.5) * 2.0 * sx;
                let y = (x * freq * 3.0 + phase).sin() * sy * 0.5;
                // normal approx
                let dy = (freq * 3.0 * (x * freq * 3.0 + phase).cos()) * sy * 0.5;
                let len = (1.0 + dy * dy).sqrt();
                (x, y, -dy / len, 1.0 / len)
            }
            PathKind::Figure8 => {
                let a = t * std::f32::consts::TAU + phase;
                let x = (a).sin() * sx;
                let y = (a * 2.0).sin() * sy * 0.5;
                (x, y, 0.0, 1.0)
            }
            PathKind::Spiral => {
                let a = t * std::f32::consts::TAU * 2.5 * freq + phase;
                let r = t * sx;
                (
                    a.cos() * r,
                    a.sin() * r * sy / sx.max(0.1),
                    a.cos(),
                    a.sin(),
                )
            }
            PathKind::Dna => {
                // Two strands interleaved: even t on strand A, odd on B — offset phase pi
                let strand = if (t * self.segments as f32) as i32 % 2 == 0 {
                    0.0
                } else {
                    std::f32::consts::PI
                };
                let a = t * std::f32::consts::TAU * freq * 1.2 + phase + strand;
                let x = a.sin() * sx * 0.5;
                let y = (t - 0.5) * 2.0 * sy;
                let backbone = (y * 0.5).sin() * 0.1;
                (x + backbone, y, x.signum(), 1.0)
            }
            PathKind::Grid => {
                // jittered grid
                let cols = 4;
                let rows = 4;
                let i = (t * (cols * rows) as f32).floor() as i32;
                let cx = (i % cols) as f32 / cols as f32 - 0.5;
                let cy = (i / cols) as f32 / rows as f32 - 0.5;
                (cx * 2.0 * sx, cy * 2.0 * sy, 0.0, 1.0)
            }
            PathKind::Bezier => {
                // quadratic bezier: p0(-sx, -sy) -> p1(0, sy*1.2) -> p2(sx, -sy*0.5)
                let p0 = (-sx, -sy * 0.5);
                let p1 = (0.0, sy * 1.1);
                let p2 = (sx, -sy * 0.3);
                let u = 1.0 - t;
                let x = u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0;
                let y = u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1;
                // derivative
                let dx = 2.0 * u * (p1.0 - p0.0) + 2.0 * t * (p2.0 - p1.0);
                let dy = 2.0 * u * (p1.1 - p0.1) + 2.0 * t * (p2.1 - p1.1);
                let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                let nx = -dy / len;
                let ny = dx / len;
                return PathSample { x, y, nx, ny };
            }
        };
        // for non-bezier, normal is (nx, ny) computed above or fallback
        PathSample { x, y, nx, ny }
    }
}

/// Sampled point on path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathSample {
    /// X in normalized -1..=1 range (scaled)
    pub x: f32,
    /// Y
    pub y: f32,
    /// Normal X (for initial vel direction)
    pub nx: f32,
    /// Normal Y
    pub ny: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sample_in_range() {
        let cfg = PathConfig {
            kind: PathKind::Circle,
            scale_x: 1.2,
            scale_y: 0.8,
            ..Default::default()
        };
        let s = cfg.sample(0.5, 0.0);
        assert!(s.x.abs() <= 1.3);
        assert!(s.y.abs() <= 1.0);
    }

    #[test]
    fn parse_kinds() {
        assert_eq!(PathKind::parse("dna"), Some(PathKind::Dna));
        assert_eq!(PathKind::parse("circle"), Some(PathKind::Circle));
    }
}
