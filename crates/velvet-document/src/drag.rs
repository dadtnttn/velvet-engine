//! Visual region drag / resize — the pure geometry path Studio GUI and CLI share.

use crate::model::{Document, DocumentError, PropertyValue, RegionId, RegionKind};
use crate::mutate::{apply_visual_patch, VisualPatch, VisualPatchOp};

/// Parsed 2D position for a visual widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetPos {
    /// X coordinate (percent 0–100 or pixels).
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
    /// When true, coordinates are percentages of the parent canvas.
    pub percent: bool,
}

/// Parsed size for a visual widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetSize {
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// When true, values are percentages.
    pub percent: bool,
}

/// Axis-aligned rect used by the canvas (logical percent or pixel space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetRect {
    /// Position.
    pub pos: WidgetPos,
    /// Size (defaults when missing from the document).
    pub size: WidgetSize,
}

impl Default for WidgetSize {
    fn default() -> Self {
        Self {
            w: 20.0,
            h: 8.0,
            percent: true,
        }
    }
}

/// Parse `(x, y)`, `(50%, 62%)`, `50%, 62%`, or `x y` into a position.
pub fn parse_position(raw: &str) -> Option<WidgetPos> {
    let s = raw
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();
    let parts: Vec<&str> = if s.contains(',') {
        s.split(',').map(str::trim).collect()
    } else {
        s.split_whitespace().collect()
    };
    if parts.len() < 2 {
        return None;
    }
    let (x, px) = parse_coord(parts[0])?;
    let (y, py) = parse_coord(parts[1])?;
    Some(WidgetPos {
        x,
        y,
        percent: px || py,
    })
}

/// Parse `(w, h)` size strings.
pub fn parse_size(raw: &str) -> Option<WidgetSize> {
    let pos = parse_position(raw)?;
    Some(WidgetSize {
        w: pos.x,
        h: pos.y,
        percent: pos.percent,
    })
}

fn parse_coord(tok: &str) -> Option<(f32, bool)> {
    let t = tok.trim();
    if let Some(num) = t.strip_suffix('%') {
        let v: f32 = num.trim().parse().ok()?;
        Some((v, true))
    } else {
        let v: f32 = t.parse().ok()?;
        Some((v, false))
    }
}

/// Format a position back to source form.
pub fn format_position(pos: WidgetPos) -> String {
    if pos.percent {
        format!("({:.0}%, {:.0}%)", pos.x, pos.y)
    } else {
        format!("({:.1}, {:.1})", pos.x, pos.y)
    }
}

/// Format a size back to source form.
pub fn format_size(size: WidgetSize) -> String {
    if size.percent {
        format!("({:.0}%, {:.0}%)", size.w, size.h)
    } else {
        format!("({:.1}, {:.1})", size.w, size.h)
    }
}

/// Read position/size for a visual region (defaults when absent).
pub fn region_rect(doc: &Document, region_id: &str) -> Result<WidgetRect, DocumentError> {
    let region = doc
        .find(RegionKind::Visual, region_id)
        .ok_or_else(|| DocumentError::RegionNotFound(region_id.into()))?;
    let get = |k: &str| {
        region
            .properties
            .iter()
            .find(|p| p.key == k)
            .map(|p| match &p.value {
                PropertyValue::String(s) | PropertyValue::Raw(s) => s.as_str(),
            })
    };
    let pos = get("position")
        .and_then(parse_position)
        .unwrap_or(WidgetPos {
            x: 50.0,
            y: 50.0,
            percent: true,
        });
    let size = get("size").and_then(parse_size).unwrap_or_default();
    Ok(WidgetRect { pos, size })
}

/// Apply a drag delta to a visual region's position. Advanced/protected regions
/// with the same id are left untouched (shared patch path).
///
/// `dx`/`dy` are in the same unit as the stored position (percent points or pixels).
pub fn drag_visual_region(
    doc: &mut Document,
    region_id: &str,
    dx: f32,
    dy: f32,
) -> Result<WidgetRect, DocumentError> {
    let mut rect = region_rect(doc, region_id)?;
    rect.pos.x += dx;
    rect.pos.y += dy;
    // Soft clamp for percent space so widgets stay on canvas.
    if rect.pos.percent {
        rect.pos.x = rect.pos.x.clamp(0.0, 100.0);
        rect.pos.y = rect.pos.y.clamp(0.0, 100.0);
    }
    apply_visual_patch(
        doc,
        &VisualPatch {
            region_id: RegionId::new(region_id),
            ops: vec![VisualPatchOp::SetProperty {
                key: "position".into(),
                value: PropertyValue::Raw(format_position(rect.pos)),
            }],
        },
    )?;
    Ok(rect)
}

/// Resize a visual region by delta (width/height).
pub fn resize_visual_region(
    doc: &mut Document,
    region_id: &str,
    dw: f32,
    dh: f32,
) -> Result<WidgetRect, DocumentError> {
    let mut rect = region_rect(doc, region_id)?;
    rect.size.w = (rect.size.w + dw).max(1.0);
    rect.size.h = (rect.size.h + dh).max(1.0);
    apply_visual_patch(
        doc,
        &VisualPatch {
            region_id: RegionId::new(region_id),
            ops: vec![VisualPatchOp::SetProperty {
                key: "size".into(),
                value: PropertyValue::Raw(format_size(rect.size)),
            }],
        },
    )?;
    Ok(rect)
}

/// Hit-test: return region id whose rect contains canvas point (percent space).
pub fn hit_test_visual(doc: &Document, canvas_x: f32, canvas_y: f32) -> Option<String> {
    let mut hits: Vec<(String, f32)> = Vec::new();
    for r in doc.regions.iter().filter(|r| r.kind == RegionKind::Visual) {
        let id = r.id.as_str();
        if id.is_empty() {
            continue;
        }
        if let Ok(rect) = region_rect(doc, id) {
            let (x, y, w, h) = if rect.pos.percent {
                (rect.pos.x, rect.pos.y, rect.size.w, rect.size.h)
            } else {
                // Treat pixel coords as percent of a 1280x720 design canvas.
                (
                    rect.pos.x / 12.80,
                    rect.pos.y / 7.20,
                    rect.size.w / 12.80,
                    rect.size.h / 7.20,
                )
            };
            // Position is center-ish in designer UX: expand half size.
            let left = x - w * 0.5;
            let top = y - h * 0.5;
            if canvas_x >= left && canvas_x <= left + w && canvas_y >= top && canvas_y <= top + h {
                hits.push((id.to_string(), w * h));
            }
        }
    }
    // Smallest area on top.
    hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.into_iter().next().map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;
    use crate::render::render_document;

    const MENU: &str = r#"
// @visual id=button.start
button start {
    text: "Iniciar"
    position: (50%, 62%)
    size: (18%, 7%)
// @advanced id=button.start
    on_pressed {
        game.new()
    }
// @end
}
// @visual id=button.quit
button quit {
    text: "Salir"
    position: (50%, 74%)
// @end
}
"#;

    #[test]
    fn parse_percent_position() {
        let p = parse_position("(50%, 62%)").unwrap();
        assert!((p.x - 50.0).abs() < 0.01);
        assert!((p.y - 62.0).abs() < 0.01);
        assert!(p.percent);
    }

    #[test]
    fn drag_moves_position_preserves_advanced() {
        let mut doc = parse_document(MENU).unwrap();
        let before = region_rect(&doc, "button.start").unwrap();
        assert!((before.pos.x - 50.0).abs() < 0.01);
        let after = drag_visual_region(&mut doc, "button.start", -5.0, 3.0).unwrap();
        assert!((after.pos.x - 45.0).abs() < 0.01);
        assert!((after.pos.y - 65.0).abs() < 0.01);
        let src = render_document(&doc);
        assert!(src.contains("position: (45%, 65%)"), "{src}");
        assert!(src.contains("game.new()"), "advanced must survive: {src}");
        // quit untouched
        let q = region_rect(&doc, "button.quit").unwrap();
        assert!((q.pos.y - 74.0).abs() < 0.01);
    }

    #[test]
    fn hit_test_finds_widget() {
        let doc = parse_document(MENU).unwrap();
        let id = hit_test_visual(&doc, 50.0, 62.0).expect("hit start");
        assert_eq!(id, "button.start");
    }

    #[test]
    fn resize_updates_size_prop() {
        let mut doc = parse_document(MENU).unwrap();
        let r = resize_visual_region(&mut doc, "button.start", 2.0, 1.0).unwrap();
        assert!((r.size.w - 20.0).abs() < 0.01);
        assert!((r.size.h - 8.0).abs() < 0.01);
    }
}
