//! Pixel palettes — core → mid → edge, ordered bright to dark.
//! BitFX-like: user picks palette, particles map age → palette index.

use serde::{Deserialize, Serialize};

/// Named palette id (stable for presets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteId {
    /// Fire: white → yellow → orange → red → smoke
    Fire,
    /// Ice frost
    Frost,
    /// Toxic slime acid
    Toxic,
    /// Magic arcane / mana
    Arcane,
    /// Neon electric / lightning
    Electric,
    /// Blood gush
    Blood,
    /// Nature leaf / poison
    Nature,
    /// Ember / lava
    Ember,
    /// Smoke / steam
    Smoke,
    /// Gold / holy sparkle
    Gold,
    /// Void / shadow
    Void,
    /// Water / aqua
    Aqua,
    /// Sunset dusk
    Sunset,
    /// Matrix / digital green
    Digital,
    /// Candy pop
    Candy,
}

impl PaletteId {
    /// All ids for UI dropdown.
    pub fn all() -> &'static [Self] {
        &[
            Self::Fire,
            Self::Frost,
            Self::Toxic,
            Self::Arcane,
            Self::Electric,
            Self::Blood,
            Self::Nature,
            Self::Ember,
            Self::Smoke,
            Self::Gold,
            Self::Void,
            Self::Aqua,
            Self::Sunset,
            Self::Digital,
            Self::Candy,
        ]
    }

    /// Human label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Fire => "Fire",
            Self::Frost => "Frost",
            Self::Toxic => "Toxic",
            Self::Arcane => "Arcane",
            Self::Electric => "Electric",
            Self::Blood => "Blood",
            Self::Nature => "Nature",
            Self::Ember => "Ember",
            Self::Smoke => "Smoke",
            Self::Gold => "Gold",
            Self::Void => "Void",
            Self::Aqua => "Aqua",
            Self::Sunset => "Sunset",
            Self::Digital => "Digital",
            Self::Candy => "Candy",
        }
    }

    /// Parse case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fire" => Some(Self::Fire),
            "frost" | "ice" => Some(Self::Frost),
            "toxic" | "acid" | "slime" => Some(Self::Toxic),
            "arcane" | "magic" | "mana" => Some(Self::Arcane),
            "electric" | "lightning" | "neon" => Some(Self::Electric),
            "blood" => Some(Self::Blood),
            "nature" | "leaf" => Some(Self::Nature),
            "ember" | "lava" => Some(Self::Ember),
            "smoke" | "steam" | "ash" => Some(Self::Smoke),
            "gold" | "holy" | "sparkle" => Some(Self::Gold),
            "void" | "shadow" | "dark" => Some(Self::Void),
            "aqua" | "water" => Some(Self::Aqua),
            "sunset" | "dusk" => Some(Self::Sunset),
            "digital" | "matrix" => Some(Self::Digital),
            "candy" | "pop" => Some(Self::Candy),
            _ => None,
        }
    }
}

/// Palette definition with up to 8 colors (core → edge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// Id.
    pub id: PaletteId,
    /// Label.
    pub name: &'static str,
    /// Colors RGBA (alpha 255, inner → outer).
    pub colors: &'static [[u8; 4]],
}

impl Palette {
    /// Sample t 0..=1 (0 = hottest core, 1 = coldest edge) → RGBA.
    pub fn sample(&self, t: f32) -> [u8; 4] {
        if self.colors.is_empty() {
            return [255, 255, 255, 255];
        }
        if self.colors.len() == 1 {
            return self.colors[0];
        }
        let t = t.clamp(0.0, 1.0);
        let scaled = t * (self.colors.len() - 1) as f32;
        let i = scaled.floor() as usize;
        let f = scaled - i as f32;
        let a = self.colors[i];
        let b = self.colors[(i + 1).min(self.colors.len() - 1)];
        [
            lerp_u8(a[0], b[0], f),
            lerp_u8(a[1], b[1], f),
            lerp_u8(a[2], b[2], f),
            255,
        ]
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// All bundled palettes.
pub fn palettes() -> Vec<Palette> {
    PALETTES.to_vec()
}

/// Get palette by id.
pub fn palette_colors(id: PaletteId) -> &'static [[u8; 4]] {
    for p in PALETTES {
        if p.id == id {
            return p.colors;
        }
    }
    PALETTES[0].colors
}

/// Static palette table — vivid pixel-art friendly.
pub static PALETTES: &[Palette] = &[
    Palette {
        id: PaletteId::Fire,
        name: "Fire",
        colors: &[
            [255, 255, 255, 255],
            [255, 240, 120, 255],
            [255, 180, 40, 255],
            [255, 90, 20, 255],
            [200, 30, 10, 255],
            [80, 20, 20, 255],
            [40, 18, 18, 255],
        ],
    },
    Palette {
        id: PaletteId::Frost,
        name: "Frost",
        colors: &[
            [255, 255, 255, 255],
            [180, 230, 255, 255],
            [110, 190, 255, 255],
            [60, 120, 220, 255],
            [30, 60, 160, 255],
            [20, 30, 90, 255],
        ],
    },
    Palette {
        id: PaletteId::Toxic,
        name: "Toxic",
        colors: &[
            [255, 255, 200, 255],
            [180, 255, 80, 255],
            [90, 220, 40, 255],
            [40, 160, 30, 255],
            [30, 90, 20, 255],
            [20, 40, 15, 255],
        ],
    },
    Palette {
        id: PaletteId::Arcane,
        name: "Arcane",
        colors: &[
            [255, 255, 255, 255],
            [220, 180, 255, 255],
            [160, 90, 255, 255],
            [90, 40, 180, 255],
            [40, 20, 90, 255],
            [18, 12, 40, 255],
        ],
    },
    Palette {
        id: PaletteId::Electric,
        name: "Electric",
        colors: &[
            [255, 255, 255, 255],
            [180, 240, 255, 255],
            [80, 200, 255, 255],
            [20, 140, 255, 255],
            [20, 80, 200, 255],
            [30, 30, 90, 255],
        ],
    },
    Palette {
        id: PaletteId::Blood,
        name: "Blood",
        colors: &[
            [255, 230, 230, 255],
            [255, 80, 80, 255],
            [210, 20, 30, 255],
            [140, 10, 20, 255],
            [70, 10, 15, 255],
            [30, 8, 10, 255],
        ],
    },
    Palette {
        id: PaletteId::Nature,
        name: "Nature",
        colors: &[
            [255, 255, 200, 255],
            [140, 230, 80, 255],
            [60, 180, 40, 255],
            [40, 120, 30, 255],
            [30, 60, 20, 255],
        ],
    },
    Palette {
        id: PaletteId::Ember,
        name: "Ember",
        colors: &[
            [255, 255, 180, 255],
            [255, 220, 60, 255],
            [255, 140, 20, 255],
            [220, 60, 10, 255],
            [120, 20, 10, 255],
        ],
    },
    Palette {
        id: PaletteId::Smoke,
        name: "Smoke",
        colors: &[
            [240, 240, 240, 255],
            [180, 180, 185, 255],
            [120, 120, 130, 255],
            [70, 70, 75, 255],
            [35, 35, 38, 255],
        ],
    },
    Palette {
        id: PaletteId::Gold,
        name: "Gold",
        colors: &[
            [255, 255, 255, 255],
            [255, 240, 150, 255],
            [255, 210, 60, 255],
            [210, 160, 30, 255],
            [120, 90, 20, 255],
        ],
    },
    Palette {
        id: PaletteId::Void,
        name: "Void",
        colors: &[
            [220, 200, 255, 255],
            [120, 80, 180, 255],
            [60, 30, 90, 255],
            [30, 15, 50, 255],
            [14, 10, 22, 255],
        ],
    },
    Palette {
        id: PaletteId::Aqua,
        name: "Aqua",
        colors: &[
            [255, 255, 255, 255],
            [160, 240, 255, 255],
            [40, 180, 220, 255],
            [20, 90, 160, 255],
            [10, 40, 90, 255],
        ],
    },
    Palette {
        id: PaletteId::Sunset,
        name: "Sunset",
        colors: &[
            [255, 255, 255, 255],
            [255, 200, 120, 255],
            [255, 120, 80, 255],
            [200, 40, 100, 255],
            [90, 20, 80, 255],
            [30, 15, 45, 255],
        ],
    },
    Palette {
        id: PaletteId::Digital,
        name: "Digital",
        colors: &[
            [255, 255, 255, 255],
            [140, 255, 140, 255],
            [40, 220, 40, 255],
            [20, 140, 20, 255],
            [10, 60, 10, 255],
        ],
    },
    Palette {
        id: PaletteId::Candy,
        name: "Candy",
        colors: &[
            [255, 255, 255, 255],
            [255, 180, 220, 255],
            [255, 90, 160, 255],
            [180, 40, 120, 255],
            [90, 20, 80, 255],
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_ends() {
        let p = &PALETTES[0];
        assert_eq!(p.sample(0.0), p.colors[0]);
        assert_eq!(p.sample(1.0), *p.colors.last().unwrap());
    }

    #[test]
    fn parse_all() {
        for id in PaletteId::all() {
            assert_eq!(PaletteId::parse(id.label()), Some(*id));
        }
    }
}
