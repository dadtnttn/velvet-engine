//! GPU / render paint path for product VN dialogue (namebox, body, choices).
//!
//! Builds a list of drawables from a live [`ProductUiFrame`] that hosts push into
//! `velvet-render` (colored quads + text draw commands).

use serde::{Deserialize, Serialize};

use crate::product::VnSession;
use crate::product_ui::{build_product_ui_frame, ProductUiFrame};

/// One paint primitive ready for GPU submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProductPaintCmd {
    /// Filled rectangle (namebox chrome, body panel, choice row).
    Quad {
        /// Semantic id: `say_panel` | `namebox` | `choice_N` | …
        id: String,
        /// Left in virtual resolution pixels.
        x: f32,
        /// Top.
        y: f32,
        /// Width.
        w: f32,
        /// Height.
        h: f32,
        /// RGBA 0..=1.
        color: [f32; 4],
        /// Draw order (higher on top).
        z: f32,
    },
    /// Text run.
    Text {
        /// Semantic id.
        id: String,
        /// Left.
        x: f32,
        /// Baseline-ish top.
        y: f32,
        /// UTF-8 content.
        text: String,
        /// Font size px.
        size: f32,
        /// RGBA.
        color: [f32; 4],
        /// Z order.
        z: f32,
        /// Measured advance width.
        width: f32,
    },
}

/// Full paint list for one product frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductPaintList {
    /// Virtual resolution width.
    pub virtual_w: f32,
    /// Virtual resolution height.
    pub virtual_h: f32,
    /// Commands in submission order.
    pub commands: Vec<ProductPaintCmd>,
    /// Scene name (debug).
    pub scene: String,
}

impl ProductPaintList {
    /// Number of drawables.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// All quads with positive size.
    pub fn positive_quads(&self) -> impl Iterator<Item = &ProductPaintCmd> {
        self.commands.iter().filter(|c| match c {
            ProductPaintCmd::Quad { w, h, .. } => *w > 0.0 && *h > 0.0,
            ProductPaintCmd::Text { width, size, .. } => *width > 0.0 && *size > 0.0,
        })
    }

    /// True when say panel geometry was painted.
    pub fn has_say_geometry(&self) -> bool {
        self.commands.iter().any(|c| match c {
            ProductPaintCmd::Quad { id, w, h, .. } => id == "say_panel" && *w > 0.0 && *h > 0.0,
            _ => false,
        })
    }

    /// True when at least one choice row was painted.
    pub fn has_choice_geometry(&self) -> bool {
        self.commands.iter().any(|c| match c {
            ProductPaintCmd::Quad { id, w, h, .. } => {
                id.starts_with("choice_") && *w > 0.0 && *h > 0.0
            }
            _ => false,
        })
    }
}

/// Layout constants (1280×720 product design space).
pub const PRODUCT_VIRTUAL_W: f32 = 1280.0;
/// Height.
pub const PRODUCT_VIRTUAL_H: f32 = 720.0;

/// Build GPU paint commands from a product UI frame.
pub fn paint_product_frame(frame: &ProductUiFrame) -> ProductPaintList {
    paint_product_frame_at(frame, PRODUCT_VIRTUAL_W, PRODUCT_VIRTUAL_H)
}

/// Mood color from background asset path (no image load required).
pub fn background_mood_color(path: Option<&str>) -> [f32; 4] {
    let p = path.unwrap_or("").to_ascii_lowercase();
    if p.contains("rain") || p.contains("station") {
        [0.10, 0.12, 0.22, 1.0]
    } else if p.contains("platform") {
        [0.12, 0.13, 0.24, 1.0]
    } else if p.contains("tunnel") || p.contains("under") {
        [0.08, 0.07, 0.10, 1.0]
    } else if p.contains("train") || p.contains("car") {
        [0.14, 0.10, 0.16, 1.0]
    } else if p.contains("city") || p.contains("light") {
        [0.08, 0.10, 0.22, 1.0]
    } else if p.contains("street") {
        [0.09, 0.10, 0.15, 1.0]
    } else {
        [0.07, 0.06, 0.12, 1.0]
    }
}

/// Character stand color from sprite id.
pub fn sprite_stand_color(id: &str) -> [f32; 4] {
    match id {
        "nora" | "lea" | "aria" => [0.72, 0.28, 0.45, 1.0],
        "june" | "kai" => [0.22, 0.48, 0.72, 1.0],
        "guard" | "inspector" => [0.62, 0.50, 0.28, 1.0],
        "radio" => [0.40, 0.40, 0.45, 1.0],
        "hero" => [0.35, 0.55, 0.40, 1.0],
        _ => [0.40, 0.35, 0.50, 1.0],
    }
}

/// Build paint commands for a virtual resolution.
pub fn paint_product_frame_at(frame: &ProductUiFrame, vw: f32, vh: f32) -> ProductPaintList {
    let mut commands = Vec::new();

    // Full-screen background (always — product stage)
    let bg = background_mood_color(frame.background.as_deref());
    commands.push(ProductPaintCmd::Quad {
        id: "background".into(),
        x: 0.0,
        y: 0.0,
        w: vw,
        h: vh,
        color: bg,
        z: 0.0,
    });
    // Upper sheen band
    commands.push(ProductPaintCmd::Quad {
        id: "background_sheen".into(),
        x: 0.0,
        y: 0.0,
        w: vw,
        h: vh * 0.28,
        color: [
            (bg[0] + 0.06).min(1.0),
            (bg[1] + 0.05).min(1.0),
            (bg[2] + 0.10).min(1.0),
            1.0,
        ],
        z: 0.5,
    });

    // Character stands from presentation sprite ids (left / center / right by order)
    let n = frame.sprite_ids.len().max(1) as f32;
    for (i, sid) in frame.sprite_ids.iter().enumerate() {
        let t = if frame.sprite_ids.len() == 1 {
            0.5
        } else {
            0.28 + 0.44 * (i as f32 / (n - 1.0).max(1.0))
        };
        let body_w = vw * 0.11;
        let body_h = vh * 0.42;
        let x = vw * t - body_w * 0.5;
        let y = vh * 0.28;
        let c = sprite_stand_color(sid);
        commands.push(ProductPaintCmd::Quad {
            id: format!("sprite_{sid}"),
            x,
            y,
            w: body_w,
            h: body_h,
            color: c,
            z: 5.0 + i as f32,
        });
        // head
        let hr = body_w * 0.45;
        commands.push(ProductPaintCmd::Quad {
            id: format!("sprite_head_{sid}"),
            x: x + body_w * 0.5 - hr * 0.5,
            y: y - hr * 0.85,
            w: hr,
            h: hr,
            color: [
                (c[0] + 0.12).min(1.0),
                (c[1] + 0.10).min(1.0),
                (c[2] + 0.08).min(1.0),
                1.0,
            ],
            z: 5.5 + i as f32,
        });
        commands.push(ProductPaintCmd::Text {
            id: format!("sprite_label_{sid}"),
            x,
            y: y + body_h + 8.0,
            text: sid.clone(),
            size: 16.0,
            color: [0.85, 0.85, 0.92, 1.0],
            z: 6.0 + i as f32,
            width: body_w.max(40.0),
        });
    }

    if frame.say_visible {
        let panel_h = (frame.body_height + 48.0).max(120.0).min(vh * 0.35);
        let panel_y = vh - panel_h - 24.0;
        let panel_x = vw * 0.08;
        let panel_w = vw * 0.84;

        commands.push(ProductPaintCmd::Quad {
            id: "say_panel".into(),
            x: panel_x,
            y: panel_y,
            w: panel_w,
            h: panel_h,
            color: [0.08, 0.09, 0.14, 0.92],
            z: 10.0,
        });

        if !frame.namebox.is_empty() {
            let nb_w = (frame.namebox.chars().count() as f32 * 14.0 + 32.0).min(panel_w * 0.5);
            commands.push(ProductPaintCmd::Quad {
                id: "namebox".into(),
                x: panel_x + 16.0,
                y: panel_y - 28.0,
                w: nb_w.max(80.0),
                h: 32.0,
                color: [0.15, 0.35, 0.22, 0.95],
                z: 11.0,
            });
            commands.push(ProductPaintCmd::Text {
                id: "namebox_text".into(),
                x: panel_x + 24.0,
                y: panel_y - 22.0,
                text: frame.namebox.clone(),
                size: 20.0,
                color: [0.9, 1.0, 0.9, 1.0],
                z: 12.0,
                width: nb_w.max(40.0),
            });
        }

        if !frame.body.is_empty() {
            let body_w = frame.body_width.max(1.0).min(panel_w - 40.0);
            let body_h = frame.body_height.max(1.0);
            commands.push(ProductPaintCmd::Text {
                id: "body_text".into(),
                x: panel_x + 24.0,
                y: panel_y + 20.0,
                text: frame.body.clone(),
                size: 28.0,
                color: [0.95, 0.95, 0.97, 1.0],
                z: 12.0,
                width: body_w,
            });
            // Invisible measure quad proving geometry for body
            commands.push(ProductPaintCmd::Quad {
                id: "body_geom".into(),
                x: panel_x + 24.0,
                y: panel_y + 16.0,
                w: body_w,
                h: body_h.max(28.0),
                color: [0.0, 0.0, 0.0, 0.0],
                z: 11.5,
            });
        }
    }

    if frame.choice_visible {
        let start_y = vh * 0.35;
        for (i, label) in frame.choices.iter().enumerate() {
            let y = start_y + i as f32 * 56.0;
            let selected = i == frame.selected_choice;
            let color = if selected {
                [0.2, 0.35, 0.55, 0.95]
            } else {
                [0.12, 0.14, 0.2, 0.9]
            };
            commands.push(ProductPaintCmd::Quad {
                id: format!("choice_{i}"),
                x: vw * 0.2,
                y,
                w: vw * 0.6,
                h: 48.0,
                color,
                z: 20.0,
            });
            commands.push(ProductPaintCmd::Text {
                id: format!("choice_text_{i}"),
                x: vw * 0.22,
                y: y + 12.0,
                text: label.clone(),
                size: 22.0,
                color: [1.0, 1.0, 1.0, 1.0],
                z: 21.0,
                width: vw * 0.55,
            });
        }
    }

    if frame.language_menu_visible {
        commands.push(ProductPaintCmd::Quad {
            id: "lang_menu".into(),
            x: vw - 160.0,
            y: 16.0,
            w: 140.0,
            h: 28.0 + frame.language_options.len() as f32 * 22.0,
            color: [0.1, 0.1, 0.15, 0.85],
            z: 30.0,
        });
    }

    ProductPaintList {
        virtual_w: vw,
        virtual_h: vh,
        commands,
        scene: frame.scene.clone(),
    }
}

/// Build paint list from a live [`VnSession`].
pub fn paint_product_session(session: &VnSession) -> ProductPaintList {
    let frame = build_product_ui_frame(session);
    paint_product_frame(&frame)
}

/// Convert paint quads into `velvet_render`-compatible draw descriptors
/// (texture-less colored quads). Does not require a GPU device.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderDrawDescriptor {
    /// Semantic id.
    pub id: String,
    /// Center x.
    pub cx: f32,
    /// Center y.
    pub cy: f32,
    /// Size w.
    pub w: f32,
    /// Size h.
    pub h: f32,
    /// RGBA.
    pub color: [f32; 4],
    /// Z.
    pub z: f32,
    /// Kind: quad | text.
    pub kind: &'static str,
}

/// Flatten paint list to render descriptors (GPU batch input).
pub fn paint_to_render_descriptors(list: &ProductPaintList) -> Vec<RenderDrawDescriptor> {
    let mut out = Vec::new();
    for c in &list.commands {
        match c {
            ProductPaintCmd::Quad {
                id,
                x,
                y,
                w,
                h,
                color,
                z,
            } => {
                if *w <= 0.0 || *h <= 0.0 {
                    continue;
                }
                out.push(RenderDrawDescriptor {
                    id: id.clone(),
                    cx: x + w * 0.5,
                    cy: y + h * 0.5,
                    w: *w,
                    h: *h,
                    color: *color,
                    z: *z,
                    kind: "quad",
                });
            }
            ProductPaintCmd::Text {
                id,
                x,
                y,
                width,
                size,
                color,
                z,
                ..
            } => {
                if *width <= 0.0 || *size <= 0.0 {
                    continue;
                }
                out.push(RenderDrawDescriptor {
                    id: id.clone(),
                    cx: x + width * 0.5,
                    cy: y + size * 0.5,
                    w: *width,
                    h: *size,
                    color: *color,
                    z: *z,
                    kind: "text",
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::product::VnSession;
    use crate::runtime::{StoryPlayer, StoryWait};

    #[test]
    fn paint_from_live_session_has_positive_say_geometry() {
        let src = r#"
character hero { name: "Hero" }
scene main {
    hero "Hello GPU say path."
    choice {
        "Yes" { jump end }
        "No" { jump end }
    }
}
scene end { "Ending: Paint" }
"#;
        let program = load_program_from_source(src, Some("paint.vel"), "P").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 20 {
            session.advance();
            g += 1;
        }
        let list = paint_product_session(&session);
        assert!(!list.is_empty());
        assert!(list.has_say_geometry(), "cmds={:?}", list.commands);
        let descs = paint_to_render_descriptors(&list);
        assert!(
            descs
                .iter()
                .any(|d| d.id == "say_panel" && d.w > 0.0 && d.h > 0.0),
            "{descs:?}"
        );
        assert!(
            descs
                .iter()
                .any(|d| d.id == "body_text" || d.id == "body_geom"),
            "body geometry missing: {descs:?}"
        );

        session.say.reveal_all();
        session.advance();
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Choice) && g < 10 {
            session.advance();
            g += 1;
        }
        let list2 = paint_product_session(&session);
        assert!(list2.has_choice_geometry(), "{:?}", list2.commands);
        let descs2 = paint_to_render_descriptors(&list2);
        assert!(
            descs2
                .iter()
                .filter(|d| d.id.starts_with("choice_"))
                .count()
                >= 2
        );
    }
}
