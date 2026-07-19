//! Visual theme constants for Velvet Arcana — Nightfall Casino.

/// Logical render width.
pub const WW: u32 = 1280;
/// Logical render height.
pub const WH: u32 = 720;

/// Brand palette (RGB).
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Deep void behind panels.
    pub void: (u8, u8, u8),
    /// Panel fill.
    pub panel: (u8, u8, u8),
    /// Selected panel fill (fallback plates).
    #[allow(dead_code)]
    pub panel_sel: (u8, u8, u8),
    /// Gold accent (titles, selection).
    pub gold: (u8, u8, u8),
    /// Soft gold body text.
    pub gold_soft: (u8, u8, u8),
    /// Neon purple edge.
    pub neon: (u8, u8, u8),
    /// Soft body text.
    pub text: (u8, u8, u8),
    /// Muted caption.
    pub muted: (u8, u8, u8),
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            void: (10, 6, 20),
            panel: (18, 12, 34),
            panel_sel: (72, 32, 118),
            gold: (212, 160, 106),
            gold_soft: (201, 184, 150),
            neon: (170, 100, 255),
            text: (220, 210, 235),
            muted: (160, 145, 185),
        }
    }
}

/// Main menu entries (order = selection index).
pub const TITLE_ITEMS: &[MenuItem] = &[
    MenuItem {
        id: "start",
        label: "START RUN",
    },
    MenuItem {
        id: "collection",
        label: "COLLECTION",
    },
    MenuItem {
        id: "shop",
        label: "SHOP",
    },
    MenuItem {
        id: "options",
        label: "OPTIONS",
    },
    MenuItem {
        id: "quit",
        label: "QUIT",
    },
];

/// One menu row.
#[derive(Debug, Clone, Copy)]
pub struct MenuItem {
    /// Stable id (for future routing / analytics).
    #[allow(dead_code)]
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
}
