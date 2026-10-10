//! The game's water: the villa's brook (it runs, and foams where the bamboo spout pours into it
//! and where the mill's paddles beat it), its hot pools, its well, and the forest's pool, each
//! with its look and its depth. Where the water is comes from the exported scenes; the solvers
//! and what disturbs the water (her steps, drops, bubbles, the spout, the mill) are the
//! renderer's (`velvet_render::WaterBody3D` and `WaterLife3D`).
use super::*;
use velvet_math::{srgb_to_linear, Color};
use velvet_render::{WaterBody3D, WaterLook3D, WaterStyle3D};

/// The water's groups for `WaterLife3D`: the wood's and the villa's (one shows at a time).
pub(super) const FOREST: u8 = 0;
pub(super) const VILLA: u8 = 1;

/// Linear colours (from sRGB hex).
fn srgb(hex: u32) -> [f32; 3] {
    let c = srgb_to_linear(Color::from_hex(hex));
    [c.r, c.g, c.b]
}

/// The kinds of water in the game, each with its look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Brook,
    HotPool,
    Well,
    ForestPool,
}

fn style(kind: Kind) -> WaterStyle3D {
    match kind {
        Kind::Brook => WaterStyle3D {
            look: WaterLook3D::Brook,
            shallow: srgb(0x5fa8a0),
            deep: srgb(0x1d5a66),
            deep_at: 0.45,
            clarity: 0.5,
            foam: 1.0,
        },
        Kind::HotPool => WaterStyle3D {
            look: WaterLook3D::Spring,
            shallow: srgb(0x8fd0c4),
            deep: srgb(0x3a9a9a),
            deep_at: 0.8,
            clarity: 0.3,
            foam: 0.5,
        },
        Kind::Well => WaterStyle3D {
            look: WaterLook3D::Well,
            shallow: srgb(0x2a4a50),
            deep: srgb(0x0a181c),
            deep_at: 0.6,
            clarity: 0.15,
            foam: 0.3,
        },
        Kind::ForestPool => WaterStyle3D {
            look: WaterLook3D::Pond,
            shallow: srgb(0x2a5246),
            deep: srgb(0x071818),
            deep_at: 0.7,
            clarity: 0.2,
            foam: 0.6,
        },
    }
}

/// A body of water for these triangles.
pub(super) fn body(
    kind: Kind,
    triangles: &[[Vec3; 3]],
    course: Option<&[[f32; 2]]>,
) -> Option<WaterBody3D> {
    let (cell, depth) = match kind {
        Kind::Brook => (0.1, 0.45),
        Kind::HotPool => (0.06, 0.9),
        Kind::Well => (0.04, 1.4),
        Kind::ForestPool => (0.06, 1.0),
    };
    let mut body = WaterBody3D::from_triangles(triangles, cell, depth)?;
    body.style = style(kind);
    if kind == Kind::HotPool {
        body.damping = 0.7;
    }
    if let Some(course) = course {
        body = body.with_flow_along(course, 0.75);
    }
    Some(body)
}
