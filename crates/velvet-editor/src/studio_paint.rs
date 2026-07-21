//! Polished dual-mode Studio chrome + canvas paint (softbuffer ARGB).
//!
//! Visual language: OLED dark editor (slate base, violet accent, green CTA).
//! Hit zones for palette, hierarchy, toolbar, and canvas support real pointer UX.

use velvet_document::DesignerWidget;
use velvet_story::{draw_text_line, fill_rect, pack_rgb};

use crate::layers::{DesignSurface, LayerEdge, LayerTreeRow, ScreenLayer};
use crate::vscript::{api_catalog, classify_line, SyntaxKind};

/// Which center surface to paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    /// 1 Visual canvas.
    Visual,
    /// 2 VScript editor.
    Script,
    /// 3 Layer nodes graph.
    Nodes,
}

/// Map design base scale (1 body / 2 title) through user UI zoom (1..=4).
#[inline]
fn txt(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    base: i32,
    zoom: i32,
) {
    let z = zoom.clamp(1, 4);
    let s = if base <= 1 { z } else { (z + 1).min(5) };
    draw_text_line(buf, ww, wh, x, y, text, color, s);
}

// ── Design tokens (ARGB via pack_rgb) ──────────────────────────────────────

// High-contrast OLED tokens (readable body ≥ ~4.5:1 on surfaces)
fn c_bg() -> u32 {
    pack_rgb(12, 14, 20)
}
fn c_surface() -> u32 {
    pack_rgb(24, 28, 40)
}
fn c_surface_2() -> u32 {
    pack_rgb(36, 42, 58)
}
fn c_border() -> u32 {
    pack_rgb(90, 100, 130)
}
fn c_border_soft() -> u32 {
    pack_rgb(55, 62, 85)
}
fn c_text() -> u32 {
    pack_rgb(248, 250, 255)
}
fn c_text_muted() -> u32 {
    pack_rgb(190, 198, 220)
}
fn c_text_dim() -> u32 {
    pack_rgb(160, 168, 190)
}
fn c_accent() -> u32 {
    pack_rgb(124, 92, 220)
}
fn c_accent_hi() -> u32 {
    pack_rgb(160, 130, 255)
}
fn c_cta() -> u32 {
    pack_rgb(34, 180, 100)
}
fn c_cta_hi() -> u32 {
    pack_rgb(80, 220, 140)
}
fn c_sel() -> u32 {
    pack_rgb(70, 100, 200)
}
fn c_sel_ring() -> u32 {
    pack_rgb(255, 200, 90)
}
fn c_canvas() -> u32 {
    pack_rgb(12, 14, 22)
}
fn c_grid() -> u32 {
    pack_rgb(32, 36, 52)
}
fn c_widget() -> u32 {
    pack_rgb(52, 48, 82)
}
fn c_widget_panel() -> u32 {
    pack_rgb(40, 42, 62)
}
fn c_shadow() -> u32 {
    pack_rgb(4, 5, 10)
}

/// Layout metrics for the Studio window (all sizes scale with `zoom`).
#[derive(Debug, Clone, Copy)]
pub struct StudioLayout {
    pub ww: i32,
    pub wh: i32,
    /// UI zoom 1..=4 — same as text scale; layout metrics scale with this.
    pub zoom: i32,
    pub left_w: i32,
    pub right_w: i32,
    pub top_h: i32,
    pub bot_h: i32,
    pub canvas_x: i32,
    pub canvas_y: i32,
    pub canvas_w: i32,
    pub canvas_h: i32,
    /// Y of LAYERS section header.
    pub layers_y: i32,
    /// Y of first layer row.
    pub layers_rows_y: i32,
    /// Max layer rows shown.
    pub max_layers: usize,
    /// Y where palette section starts (for hit tests).
    pub palette_y: i32,
    /// Y of first hierarchy row.
    pub hierarchy_y: i32,
    pub header_h: i32,
    pub row_h: i32,
    pub pal_item_h: i32,
    pub pal_gap: i32,
    pub insp_row_h: i32,
    pub pad: i32,
    pub pill_w: i32,
    pub pill_h: i32,
    pub save_w: i32,
    pub max_hier: usize,
}

impl StudioLayout {
    /// Build layout. `zoom` is 1..=4 and scales chrome, rows, pills, and hit boxes.
    pub fn new(ww: u32, wh: u32, zoom: i32) -> Self {
        let z = zoom.clamp(1, 4);
        let ww = ww as i32;
        let wh = wh as i32;
        // Density scales with zoom so text never overflows chrome.
        let left_w = ((ww as f32 * (0.14 + 0.02 * z as f32)).round() as i32)
            .clamp(150 + 40 * z, 220 + 50 * z);
        let right_w = ((ww as f32 * (0.17 + 0.02 * z as f32)).round() as i32)
            .clamp(170 + 40 * z, 240 + 55 * z);
        let top_h = 36 + 10 * z; // 46..76
        let bot_h = 36 + 10 * z;
        let gap = 6 + 2 * z;
        let header_h = 22 + 6 * z;
        let row_h = 14 + 8 * z; // hierarchy line height
        let pal_item_h = 22 + 10 * z;
        let pal_gap = 4 + 2 * z;
        let insp_row_h = 28 + 14 * z; // label + edit box
        let pad = 6 + 2 * z;
        let pill_w = 72 + 18 * z;
        let pill_h = 18 + 6 * z;
        let save_w = 56 + 14 * z;
        let max_hier = match z {
            1 | 2 => 5,
            3 => 4,
            _ => 3,
        };
        let max_layers = 10usize;

        let canvas_x = left_w + gap;
        let canvas_y = top_h + gap;
        let canvas_w = (ww - left_w - right_w - gap * 2).max(64);
        let canvas_h = (wh - top_h - bot_h - gap * 2).max(64);

        let layers_y = top_h;
        let layers_rows_y = layers_y + header_h + pad / 2;
        let layers_block = header_h + max_layers as i32 * row_h + pad;
        let hierarchy_y = top_h + layers_block + header_h + pad / 2;
        let palette_y = hierarchy_y + max_hier as i32 * row_h + pad + 4;

        Self {
            ww,
            wh,
            zoom: z,
            left_w,
            right_w,
            top_h,
            bot_h,
            canvas_x,
            canvas_y,
            canvas_w,
            canvas_h,
            layers_y,
            layers_rows_y,
            max_layers,
            palette_y,
            hierarchy_y,
            header_h,
            row_h,
            pal_item_h,
            pal_gap,
            insp_row_h,
            pad,
            pill_w,
            pill_h,
            save_w,
            max_hier,
        }
    }

    /// Approx glyph advance for body text at this zoom.
    pub fn char_w(&self) -> i32 {
        6 * self.zoom.max(1)
    }

    pub fn max_chars_in(&self, width_px: i32) -> usize {
        ((width_px / self.char_w()).max(4)) as usize
    }

    /// Window pixel → canvas percent (0..=100).
    pub fn screen_to_canvas_pct(&self, sx: f64, sy: f64) -> (f32, f32) {
        let px =
            ((sx as f32 - self.canvas_x as f32) / self.canvas_w as f32 * 100.0).clamp(0.0, 100.0);
        let py =
            ((sy as f32 - self.canvas_y as f32) / self.canvas_h as f32 * 100.0).clamp(0.0, 100.0);
        (px, py)
    }

    /// Screen pixel delta → canvas percent delta.
    pub fn screen_delta_to_pct(&self, dx: f64, dy: f64) -> (f32, f32) {
        let dpx = dx as f32 / self.canvas_w as f32 * 100.0;
        let dpy = dy as f32 / self.canvas_h as f32 * 100.0;
        (dpx, dpy)
    }

    pub fn contains_canvas(&self, sx: f64, sy: f64) -> bool {
        let x = sx as i32;
        let y = sy as i32;
        x >= self.canvas_x
            && y >= self.canvas_y
            && x < self.canvas_x + self.canvas_w
            && y < self.canvas_y + self.canvas_h
    }

    pub fn contains_left_dock(&self, sx: f64, sy: f64) -> bool {
        let x = sx as i32;
        let y = sy as i32;
        x >= 0 && x < self.left_w && y >= self.top_h && y < self.wh - self.bot_h
    }

    fn pill_w_scaled(&self) -> i32 {
        (self.pill_w as f32 * 0.85).round() as i32
    }

    fn toolbar_pill_x(&self) -> i32 {
        let pw = self.pill_w_scaled();
        self.ww - (pw * 3 + self.save_w + self.pad * 5 + 8)
    }

    /// Hit test toolbar mode pills / save. Returns action id.
    pub fn hit_toolbar(&self, sx: f64, sy: f64) -> Option<&'static str> {
        let x = sx as i32;
        let y = sy as i32;
        let py0 = (self.top_h - self.pill_h) / 2;
        let py1 = py0 + self.pill_h;
        if y < py0 || y > py1 {
            return None;
        }
        let pill_x = self.toolbar_pill_x();
        let pw = self.pill_w_scaled();
        let gap = self.pad;
        if x >= pill_x && x < pill_x + pw {
            return Some("mode_visual");
        }
        let x2 = pill_x + pw + gap;
        if x >= x2 && x < x2 + pw {
            return Some("mode_script");
        }
        let x3 = x2 + pw + gap;
        if x >= x3 && x < x3 + pw {
            return Some("mode_nodes");
        }
        let x4 = x3 + pw + gap;
        if x >= x4 && x < x4 + self.save_w {
            return Some("save");
        }
        None
    }

    /// Graph content area (below top bar, above bottom tools).
    pub fn graph_content_rect(&self) -> (i32, i32, i32, i32) {
        let top_bar = 28 + 8 * self.zoom;
        let bot_h = 34 + 8 * self.zoom;
        let x = self.canvas_x;
        let y = self.canvas_y + top_bar + 4;
        let w = self.canvas_w;
        let h = (self.canvas_h - top_bar - bot_h - 8).max(40);
        (x, y, w, h)
    }

    /// Hit a graph node in Nodes mode.
    pub fn hit_graph_node(
        &self,
        sx: f64,
        sy: f64,
        layout_nodes: &[(String, f32, f32)],
    ) -> Option<String> {
        let (gx, gy, gw, gh) = self.graph_content_rect();
        let x = sx as i32;
        let y = sy as i32;
        for (id, px, py) in layout_nodes {
            let (x0, y0, nw, nh) =
                crate::layers::LayerStack::node_screen_rect(gx, gy, gw, gh, *px, *py, self.zoom);
            if x >= x0 && x < x0 + nw && y >= y0 && y < y0 + nh {
                return Some(id.clone());
            }
        }
        None
    }

    /// Hit layer row → index in sorted layer list.
    pub fn hit_layer_row(&self, sy: f64, layer_count: usize) -> Option<usize> {
        let y = sy as i32;
        if y < self.layers_rows_y || y >= self.hierarchy_y - self.header_h {
            return None;
        }
        let row = ((y - self.layers_rows_y) / self.row_h) as usize;
        if row < layer_count.min(self.max_layers) {
            Some(row)
        } else {
            None
        }
    }

    /// Hit hierarchy row → widget index among canvas widgets (0-based).
    pub fn hit_hierarchy(&self, sy: f64, widget_count: usize) -> Option<usize> {
        let y = sy as i32;
        if y < self.hierarchy_y || y >= self.palette_y - 4 {
            return None;
        }
        let row = ((y - self.hierarchy_y) / self.row_h) as usize;
        if row < widget_count.min(self.max_hier) {
            Some(row)
        } else {
            None
        }
    }

    /// Hit palette item: button / label / panel.
    pub fn hit_palette(&self, sx: f64, sy: f64) -> Option<&'static str> {
        let x = sx as i32;
        let y = sy as i32;
        if x < self.pad || x > self.left_w - self.pad {
            return None;
        }
        let base = self.palette_y + self.header_h + self.pad;
        let h = self.pal_item_h;
        let gap = self.pal_gap;
        for (i, kind) in ["button", "label", "panel"].iter().enumerate() {
            let y0 = base + i as i32 * (h + gap);
            if y >= y0 && y < y0 + h {
                return Some(*kind);
            }
        }
        None
    }

    pub fn contains_inspector(&self, sx: f64, sy: f64) -> bool {
        let x = sx as i32;
        let y = sy as i32;
        x >= self.ww - self.right_w && x < self.ww && y >= self.top_h && y < self.wh - self.bot_h
    }

    /// Hit editable inspector field when a widget is selected.
    /// Row order: 0=id(ro), 1=kind(ro), 2=text, 3=pos, 4=size.
    pub fn hit_inspector_field(&self, sx: f64, sy: f64) -> Option<InspectorField> {
        if !self.contains_inspector(sx, sy) {
            return None;
        }
        let y = sy as i32;
        let base = self.top_h + self.header_h + self.pad;
        let row = (y - base) / self.insp_row_h;
        match row {
            2 => Some(InspectorField::Text),
            3 => Some(InspectorField::Pos),
            4 => Some(InspectorField::Size),
            _ => None,
        }
    }
}

/// Editable inspector property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorField {
    Text,
    Pos,
    Size,
}

impl InspectorField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Pos => "POS",
            Self::Size => "SIZE",
        }
    }
}

/// Whether a visual region should be drawn as a draggable widget (not screen chrome).
pub fn is_canvas_widget(w: &DesignerWidget) -> bool {
    let id = w.id.to_ascii_lowercase();
    if id.starts_with("screen.") || id.starts_with("plugin.") {
        return false;
    }
    matches!(w.kind.as_str(), "button" | "label" | "panel" | "widget")
        && (id.starts_with("button.")
            || id.starts_with("label.")
            || id.starts_with("panel.")
            || w.kind == "button"
            || w.kind == "label"
            || w.kind == "panel")
}

fn parse_pct_pair(s: &str) -> (f32, f32) {
    let t = s.trim().trim_start_matches('(').trim_end_matches(')');
    let mut parts = t.split(',');
    let a = parts
        .next()
        .unwrap_or("50")
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50.0);
    let b = parts
        .next()
        .unwrap_or("50")
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50.0);
    (a, b)
}

fn rect_outline(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    c: u32,
    t: i32,
) {
    fill_rect(buf, ww, wh, x0, y0, x1, y0 + t, c);
    fill_rect(buf, ww, wh, x0, y1 - t, x1, y1, c);
    fill_rect(buf, ww, wh, x0, y0, x0 + t, y1, c);
    fill_rect(buf, ww, wh, x1 - t, y0, x1, y1, c);
}

fn draw_panel_header(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    title: &str,
    header_h: i32,
    zoom: i32,
) {
    fill_rect(buf, ww, wh, x0, y0, x1, y0 + header_h, c_surface_2());
    fill_rect(
        buf,
        ww,
        wh,
        x0,
        y0 + header_h - 1,
        x1,
        y0 + header_h,
        c_border_soft(),
    );
    fill_rect(buf, ww, wh, x0, y0, x0 + 3, y0 + header_h, c_accent());
    let ty = y0 + (header_h - 8 * zoom).max(4) / 2;
    txt(buf, ww, wh, x0 + 10, ty, title, c_text_muted(), 2, zoom);
}

fn draw_pill(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    active: bool,
    label: &str,
    zoom: i32,
) {
    let fill = if active { c_accent() } else { c_surface_2() };
    fill_rect(buf, ww, wh, x0, y0, x1, y1, fill);
    if active {
        rect_outline(buf, ww, wh, x0, y0, x1, y1, c_accent_hi(), 1);
    } else {
        rect_outline(buf, ww, wh, x0, y0, x1, y1, c_border(), 1);
    }
    let tw = (label.len() as i32) * 6 * zoom;
    let tx = x0 + ((x1 - x0) - tw) / 2;
    let ty = y0 + ((y1 - y0) - 8 * zoom).max(2) / 2;
    txt(
        buf,
        ww,
        wh,
        tx.max(x0 + 4),
        ty,
        label,
        if active { c_text() } else { c_text_muted() },
        1,
        zoom,
    );
}

/// Layer stack snapshot for paint (borrowed data).
pub struct LayerPaintView<'a> {
    pub layers: &'a [ScreenLayer],
    pub tree_rows: &'a [LayerTreeRow],
    pub edges: &'a [LayerEdge],
    pub active_id: &'a str,
    pub breadcrumb: &'a str,
    pub res_w: f32,
    pub res_h: f32,
    pub animating: bool,
    pub editable: bool,
    pub pos_px: Option<(i32, i32)>,
    pub connect_from: Option<&'a str>,
    pub script_cursor_line: usize,
    pub script_issues: &'a [String],
    pub node_layout: &'a [(String, f32, f32)],
    /// Nodes tool: select | connect | disconnect | overlay
    pub nodes_tool: &'a str,
    /// Selected edge (from, to)
    pub selected_edge: Option<(&'a str, &'a str)>,
}

/// Paint full Studio chrome + simplified widgets or advanced script.
// This is the final renderer boundary; the flat arguments avoid allocating a transient frame object.
#[allow(clippy::too_many_arguments)]
pub fn paint_studio(
    buf: &mut [u32],
    layout: &StudioLayout,
    mode: PaintMode,
    project_name: &str,
    selected: Option<&str>,
    widgets: &[DesignerWidget],
    advanced_src: &str,
    status: &str,
    dragging: bool,
    edit_field: Option<InspectorField>,
    edit_buf: &str,
    ui_zoom: i32,
    layers: &LayerPaintView<'_>,
) {
    let _mode = mode;
    let ww = layout.ww as u32;
    let wh = layout.wh as u32;
    let lay = *layout;
    // Prefer layout zoom (scaled chrome) over raw param if they diverge.
    let zoom = lay.zoom.clamp(1, 4);
    let _ = ui_zoom;

    // App background
    fill_rect(buf, ww, wh, 0, 0, lay.ww, lay.wh, c_bg());

    // ── Top toolbar ────────────────────────────────────────────────────────
    fill_rect(buf, ww, wh, 0, 0, lay.ww, lay.top_h, c_surface());
    fill_rect(buf, ww, wh, 0, lay.top_h - 1, lay.ww, lay.top_h, c_border());
    // brand mark scales with zoom
    let mark = 8 + 4 * zoom;
    let mark_y = (lay.top_h - mark) / 2;
    fill_rect(
        buf,
        ww,
        wh,
        lay.pad,
        mark_y,
        lay.pad + mark,
        mark_y + mark,
        c_accent(),
    );
    let title_y = (lay.top_h - 10 * zoom).max(4) / 2;
    txt(
        buf,
        ww,
        wh,
        lay.pad + mark + 8,
        title_y,
        "VELVET STUDIO",
        c_text(),
        2,
        zoom,
    );
    let name_x = lay.pad + mark + 8 + (14 * 6 * (zoom + 1));
    txt(
        buf,
        ww,
        wh,
        name_x,
        title_y + 2,
        project_name,
        c_text_dim(),
        1,
        zoom,
    );
    let zlabel = format!("x{zoom}");
    txt(
        buf,
        ww,
        wh,
        name_x + (project_name.len() as i32 + 1) * lay.char_w(),
        title_y + 2,
        &zlabel,
        c_text_dim(),
        1,
        zoom,
    );

    // Mode pills + Save (1 Visual · 2 Script · 3 Nodes)
    let pill_w = (lay.pill_w as f32 * 0.85).round() as i32;
    let pill_x = lay.ww - (pill_w * 3 + lay.save_w + lay.pad * 5 + 8);
    let pill_y = (lay.top_h - lay.pill_h) / 2;
    draw_pill(
        buf,
        ww,
        wh,
        pill_x,
        pill_y,
        pill_x + pill_w,
        pill_y + lay.pill_h,
        mode == PaintMode::Visual,
        "1 Vis",
        zoom,
    );
    let pill2 = pill_x + pill_w + lay.pad;
    draw_pill(
        buf,
        ww,
        wh,
        pill2,
        pill_y,
        pill2 + pill_w,
        pill_y + lay.pill_h,
        mode == PaintMode::Script,
        "2 Scr",
        zoom,
    );
    let pill3 = pill2 + pill_w + lay.pad;
    draw_pill(
        buf,
        ww,
        wh,
        pill3,
        pill_y,
        pill3 + pill_w,
        pill_y + lay.pill_h,
        mode == PaintMode::Nodes,
        "3 Nod",
        zoom,
    );
    let save_x = pill3 + pill_w + lay.pad;
    fill_rect(
        buf,
        ww,
        wh,
        save_x,
        pill_y,
        save_x + lay.save_w,
        pill_y + lay.pill_h,
        c_cta(),
    );
    rect_outline(
        buf,
        ww,
        wh,
        save_x,
        pill_y,
        save_x + lay.save_w,
        pill_y + lay.pill_h,
        c_cta_hi(),
        1,
    );
    let save_ty = pill_y + (lay.pill_h - 8 * zoom).max(2) / 2;
    txt(
        buf,
        ww,
        wh,
        save_x + lay.save_w / 2 - 2 * 6 * zoom,
        save_ty,
        "Save",
        c_text(),
        1,
        zoom,
    );

    // ── Left dock ──────────────────────────────────────────────────────────
    fill_rect(
        buf,
        ww,
        wh,
        0,
        lay.top_h,
        lay.left_w,
        lay.wh - lay.bot_h,
        c_surface(),
    );
    fill_rect(
        buf,
        ww,
        wh,
        lay.left_w - 1,
        lay.top_h,
        lay.left_w,
        lay.wh - lay.bot_h,
        c_border_soft(),
    );

    // ── Layers tree (pantallas + subcapas) ─────────────────────────────────
    draw_panel_header(
        buf,
        ww,
        wh,
        0,
        lay.layers_y,
        lay.left_w,
        "LAYERS",
        lay.header_h,
        zoom,
    );
    let mut ly = lay.layers_rows_y;
    for row in layers.tree_rows.iter().take(lay.max_layers) {
        let row_bot = ly + lay.row_h - 2;
        if row.active {
            fill_rect(
                buf,
                ww,
                wh,
                3,
                ly,
                lay.left_w - 3,
                row_bot,
                pack_rgb(40, 70, 55),
            );
            fill_rect(buf, ww, wh, 3, ly, 6, row_bot, c_cta_hi());
        } else if row.is_root {
            fill_rect(
                buf,
                ww,
                wh,
                3,
                ly,
                lay.left_w - 3,
                row_bot,
                pack_rgb(28, 32, 46),
            );
        }
        let indent = lay.pad + row.depth as i32 * (10 + 2 * zoom);
        let ty = ly + (lay.row_h - 8 * zoom).max(2) / 2;
        // expand marker
        let mark = if row.has_children {
            if row.expanded {
                "-"
            } else {
                "+"
            }
        } else if row.depth > 0 {
            "."
        } else {
            " "
        };
        txt(buf, ww, wh, indent, ty, mark, c_text_muted(), 1, zoom);
        let lock = if row.locked { "#" } else { " " };
        txt(
            buf,
            ww,
            wh,
            indent + 8 * zoom,
            ty,
            lock,
            if row.locked {
                pack_rgb(255, 140, 120)
            } else {
                c_text_dim()
            },
            1,
            zoom,
        );
        // name only (res on second visual line via short suffix)
        let name_x = indent + 14 * zoom;
        let avail = (lay.left_w - name_x - lay.pad).max(24);
        let nchars = lay.max_chars_in(avail);
        let label = if row.is_root {
            format!("{} {}x{}", row.name, row.width_px, row.height_px)
        } else {
            row.name.clone()
        };
        txt(
            buf,
            ww,
            wh,
            name_x,
            ty,
            &label.chars().take(nchars).collect::<String>(),
            if row.active {
                c_text()
            } else if row.is_root {
                c_text_muted()
            } else {
                pack_rgb(200, 210, 230)
            },
            1,
            zoom,
        );
        ly += lay.row_h;
    }

    draw_panel_header(
        buf,
        ww,
        wh,
        0,
        lay.hierarchy_y - lay.header_h - lay.pad / 2,
        lay.left_w,
        "HIERARCHY",
        lay.header_h,
        zoom,
    );
    let canvas_widgets: Vec<&DesignerWidget> =
        widgets.iter().filter(|w| is_canvas_widget(w)).collect();
    let mut hy = lay.hierarchy_y;
    let hier_chars = lay.max_chars_in(lay.left_w - 36);
    for (i, w) in canvas_widgets.iter().take(lay.max_hier).enumerate() {
        let sel = selected == Some(w.id.as_str());
        let row_bot = hy + lay.row_h - 2;
        if sel {
            fill_rect(
                buf,
                ww,
                wh,
                4,
                hy,
                lay.left_w - 4,
                row_bot,
                pack_rgb(45, 55, 100),
            );
            fill_rect(buf, ww, wh, 4, hy, 7, row_bot, c_accent_hi());
        }
        let kind_mark = match w.kind.as_str() {
            "label" => "L",
            "panel" => "P",
            _ => "B",
        };
        let badge = 10 + 4 * zoom;
        let by = hy + (lay.row_h - badge) / 2;
        fill_rect(buf, ww, wh, 12, by, 12 + badge, by + badge, c_surface_2());
        txt(buf, ww, wh, 14, by + 1, kind_mark, c_accent_hi(), 1, zoom);
        let label = w.text.as_deref().unwrap_or(w.id.as_str());
        let line = label.to_string();
        txt(
            buf,
            ww,
            wh,
            16 + badge,
            hy + (lay.row_h - 8 * zoom).max(2) / 2,
            &line.chars().take(hier_chars).collect::<String>(),
            if sel { c_text() } else { c_text_muted() },
            1,
            zoom,
        );
        hy += lay.row_h;
        let _ = i;
    }
    if canvas_widgets.is_empty() {
        txt(
            buf,
            ww,
            wh,
            lay.pad,
            hy,
            "Empty screen",
            c_text_muted(),
            1,
            zoom,
        );
        hy += lay.row_h;
        txt(
            buf,
            ww,
            wh,
            lay.pad,
            hy,
            "B/L/P drop widgets",
            c_text_dim(),
            1,
            zoom,
        );
    }

    // Palette (aligned with hit_palette)
    let pal_y = lay.palette_y;
    draw_panel_header(
        buf,
        ww,
        wh,
        0,
        pal_y,
        lay.left_w,
        "PALETTE",
        lay.header_h,
        zoom,
    );
    let mut py = pal_y + lay.header_h + lay.pad;
    for (label, accent) in [
        ("Button", pack_rgb(90, 80, 160)),
        ("Label", pack_rgb(70, 110, 140)),
        ("Panel", pack_rgb(60, 90, 100)),
    ] {
        fill_rect(
            buf,
            ww,
            wh,
            lay.pad,
            py,
            lay.left_w - lay.pad,
            py + lay.pal_item_h,
            c_surface_2(),
        );
        rect_outline(
            buf,
            ww,
            wh,
            lay.pad,
            py,
            lay.left_w - lay.pad,
            py + lay.pal_item_h,
            c_border(),
            1,
        );
        fill_rect(
            buf,
            ww,
            wh,
            lay.pad,
            py,
            lay.pad + 4,
            py + lay.pal_item_h,
            accent,
        );
        let ty = py + (lay.pal_item_h - 8 * zoom).max(2) / 2;
        txt(buf, ww, wh, lay.pad + 10, ty, label, c_text(), 1, zoom);
        py += lay.pal_item_h + lay.pal_gap;
    }
    txt(
        buf,
        ww,
        wh,
        lay.pad,
        py + 4,
        "Click to place",
        c_text_dim(),
        1,
        zoom,
    );

    // ── Right dock — inspector ─────────────────────────────────────────────
    let rx0 = lay.ww - lay.right_w;
    fill_rect(
        buf,
        ww,
        wh,
        rx0,
        lay.top_h,
        lay.ww,
        lay.wh - lay.bot_h,
        c_surface(),
    );
    fill_rect(
        buf,
        ww,
        wh,
        rx0,
        lay.top_h,
        rx0 + 1,
        lay.wh - lay.bot_h,
        c_border_soft(),
    );
    draw_panel_header(
        buf,
        ww,
        wh,
        rx0,
        lay.top_h,
        lay.ww,
        "INSPECTOR",
        lay.header_h,
        zoom,
    );
    let mut iy = lay.top_h + lay.header_h + lay.pad;
    if let Some(id) = selected {
        if let Some(w) = widgets.iter().find(|w| w.id == id) {
            // Rows aligned with hit_inspector_field (insp_row_h)
            let fields: [(&str, String, Option<InspectorField>, bool); 5] = [
                ("ID", id.to_string(), None, false),
                ("KIND", w.kind.clone(), None, false),
                (
                    "TEXT",
                    w.text.as_deref().unwrap_or("").to_string(),
                    Some(InspectorField::Text),
                    true,
                ),
                (
                    "POS %",
                    w.position.as_deref().unwrap_or("(50%, 50%)").to_string(),
                    Some(InspectorField::Pos),
                    true,
                ),
                (
                    "SIZE",
                    w.size.as_deref().unwrap_or("(18%, 8%)").to_string(),
                    Some(InspectorField::Size),
                    true,
                ),
            ];
            let insp_chars = lay.max_chars_in(lay.right_w - lay.pad * 3);
            let label_h = 8 * zoom + 2;
            let box_h = (lay.insp_row_h - label_h - 4).max(12 + 4 * zoom);
            for (key, val, field, editable) in fields {
                let editing = field.is_some() && edit_field == field;
                let row_top = iy;
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + lay.pad,
                    row_top,
                    key,
                    if editable {
                        c_text_muted()
                    } else {
                        c_text_dim()
                    },
                    1,
                    zoom,
                );
                let box_y = row_top + label_h;
                let box_fill = if editing {
                    pack_rgb(40, 50, 90)
                } else if editable {
                    c_surface_2()
                } else {
                    pack_rgb(24, 26, 38)
                };
                fill_rect(
                    buf,
                    ww,
                    wh,
                    rx0 + lay.pad,
                    box_y,
                    lay.ww - lay.pad,
                    box_y + box_h,
                    box_fill,
                );
                if editing {
                    rect_outline(
                        buf,
                        ww,
                        wh,
                        rx0 + lay.pad,
                        box_y,
                        lay.ww - lay.pad,
                        box_y + box_h,
                        c_accent_hi(),
                        2,
                    );
                } else if editable {
                    rect_outline(
                        buf,
                        ww,
                        wh,
                        rx0 + lay.pad,
                        box_y,
                        lay.ww - lay.pad,
                        box_y + box_h,
                        c_border(),
                        1,
                    );
                }
                let shown = if editing { format!("{edit_buf}|") } else { val };
                let t_y = box_y + (box_h - 8 * zoom).max(2) / 2;
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + lay.pad + 6,
                    t_y,
                    &shown.chars().take(insp_chars).collect::<String>(),
                    if editing { c_cta_hi() } else { c_text() },
                    1,
                    zoom,
                );
                iy = row_top + lay.insp_row_h;
            }
            // Pixel coords for active layer resolution
            if let Some((px, py)) = layers.pos_px {
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + lay.pad,
                    iy,
                    "POS px",
                    c_text_dim(),
                    1,
                    zoom,
                );
                iy += 8 * zoom + 2;
                fill_rect(
                    buf,
                    ww,
                    wh,
                    rx0 + lay.pad,
                    iy,
                    lay.ww - lay.pad,
                    iy + 10 + 4 * zoom,
                    c_surface_2(),
                );
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + lay.pad + 6,
                    iy + 2,
                    &format!(
                        "{px}, {py}  ({}x{})",
                        layers.res_w as i32, layers.res_h as i32
                    ),
                    pack_rgb(180, 220, 255),
                    1,
                    zoom,
                );
                iy += 14 + 6 * zoom;
            }
            txt(
                buf,
                ww,
                wh,
                rx0 + lay.pad,
                iy,
                if layers.editable {
                    "layer editable"
                } else {
                    "layer LOCKED"
                },
                if layers.editable {
                    c_cta_hi()
                } else {
                    pack_rgb(220, 120, 100)
                },
                1,
                zoom,
            );
            iy += lay.row_h;
            iy += lay.pad;
            if edit_field.is_some() {
                fill_rect(
                    buf,
                    ww,
                    wh,
                    rx0 + 12,
                    iy,
                    lay.ww - 12,
                    iy + 36,
                    pack_rgb(30, 40, 55),
                );
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 16,
                    iy + 6,
                    "Enter apply  Esc cancel",
                    c_text_muted(),
                    1,
                    zoom,
                );
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 16,
                    iy + 20,
                    "type to edit selected field",
                    c_text_dim(),
                    1,
                    zoom,
                );
                iy += 44;
            } else {
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    "Click field to edit",
                    c_text_dim(),
                    1,
                    zoom,
                );
                iy += 16;
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    "T text  P pos  Z size",
                    c_text_dim(),
                    1,
                    zoom,
                );
                iy += 16;
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    "Arrows nudge 1%",
                    c_text_dim(),
                    1,
                    zoom,
                );
                iy += 20;
            }
            if dragging {
                fill_rect(
                    buf,
                    ww,
                    wh,
                    rx0 + 12,
                    iy,
                    lay.ww - 12,
                    iy + 24,
                    pack_rgb(40, 60, 50),
                );
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 18,
                    iy + 6,
                    "DRAGGING...",
                    c_cta_hi(),
                    1,
                    zoom,
                );
            }
        } else {
            txt(buf, ww, wh, rx0 + 14, iy, id, c_text_muted(), 1, zoom);
        }
    } else {
        txt(
            buf,
            ww,
            wh,
            rx0 + 14,
            iy,
            "No selection",
            c_text_dim(),
            1,
            zoom,
        );
        iy += 22;
        txt(
            buf,
            ww,
            wh,
            rx0 + 14,
            iy,
            "Click canvas widget",
            c_text_dim(),
            1,
            zoom,
        );
        iy += 18;
        txt(
            buf,
            ww,
            wh,
            rx0 + 14,
            iy,
            "or hierarchy row",
            c_text_dim(),
            1,
            zoom,
        );
        iy += 24;
        txt(
            buf,
            ww,
            wh,
            rx0 + 14,
            iy,
            "Then edit TEXT / POS / SIZE",
            c_text_dim(),
            1,
            zoom,
        );
    }

    // ── Bottom console ─────────────────────────────────────────────────────
    fill_rect(
        buf,
        ww,
        wh,
        0,
        lay.wh - lay.bot_h,
        lay.ww,
        lay.wh,
        pack_rgb(10, 12, 18),
    );
    fill_rect(
        buf,
        ww,
        wh,
        0,
        lay.wh - lay.bot_h,
        lay.ww,
        lay.wh - lay.bot_h + 1,
        c_border(),
    );
    let status_y0 = lay.wh - lay.bot_h + lay.pad;
    let status_y1 = status_y0 + 8 * zoom + 4;
    txt(
        buf,
        ww,
        wh,
        lay.pad,
        status_y0,
        "STATUS",
        c_text_dim(),
        1,
        zoom,
    );
    let status_chars = lay.max_chars_in(lay.ww / 2);
    txt(
        buf,
        ww,
        wh,
        lay.pad,
        status_y1,
        &status.chars().take(status_chars).collect::<String>(),
        c_text_muted(),
        1,
        zoom,
    );
    let help = "[ ] layers  N sublayer  M mobile  U lock  Ctrl+4 phone";
    let help_chars = lay.max_chars_in(lay.ww / 2 - lay.pad);
    txt(
        buf,
        ww,
        wh,
        lay.ww / 2,
        status_y1,
        &help.chars().take(help_chars).collect::<String>(),
        c_text_muted(),
        1,
        zoom,
    );

    // ── Canvas frame + letterboxed design surface (layer resolution) ───────
    fill_rect(
        buf,
        ww,
        wh,
        lay.canvas_x - 3,
        lay.canvas_y - 3,
        lay.canvas_x + lay.canvas_w + 3,
        lay.canvas_y + lay.canvas_h + 3,
        c_border(),
    );
    fill_rect(
        buf,
        ww,
        wh,
        lay.canvas_x,
        lay.canvas_y,
        lay.canvas_x + lay.canvas_w,
        lay.canvas_y + lay.canvas_h,
        pack_rgb(8, 9, 14),
    );

    let surface = DesignSurface::fit(
        lay.canvas_x,
        lay.canvas_y,
        lay.canvas_w,
        lay.canvas_h,
        layers.res_w,
        layers.res_h,
    );

    // Ghost other visible roots/siblings (dim outlines)
    let active_z = layers
        .layers
        .iter()
        .find(|l| l.id == layers.active_id)
        .map(|l| l.z)
        .unwrap_or(0);
    for (gi, gl) in layers
        .layers
        .iter()
        .filter(|l| l.visible && l.id != layers.active_id && l.parent.is_none())
        .enumerate()
    {
        let ghost = DesignSurface::fit(
            lay.canvas_x,
            lay.canvas_y,
            lay.canvas_w,
            lay.canvas_h,
            gl.width_px as f32,
            gl.height_px as f32,
        );
        let inset = (gi as i32 + 1) * 3;
        let col = if gl.z < active_z {
            pack_rgb(50, 55, 75)
        } else {
            pack_rgb(70, 60, 90)
        };
        rect_outline(
            buf,
            ww,
            wh,
            ghost.x + inset,
            ghost.y + inset,
            ghost.x + ghost.w - inset,
            ghost.y + ghost.h - inset,
            col,
            1,
        );
    }

    // Active design surface
    let border_c = if layers.animating {
        pack_rgb(80, 200, 140)
    } else if layers.editable {
        c_border()
    } else {
        pack_rgb(120, 70, 70)
    };
    fill_rect(
        buf,
        ww,
        wh,
        surface.x - 2,
        surface.y - 2,
        surface.x + surface.w + 2,
        surface.y + surface.h + 2,
        border_c,
    );
    fill_rect(
        buf,
        ww,
        wh,
        surface.x,
        surface.y,
        surface.x + surface.w,
        surface.y + surface.h,
        c_canvas(),
    );

    if mode == PaintMode::Visual {
        // Grid on design surface
        let step_x = (surface.w / 10).max(16);
        let mut gx = surface.x + step_x;
        while gx < surface.x + surface.w {
            fill_rect(
                buf,
                ww,
                wh,
                gx,
                surface.y,
                gx + 1,
                surface.y + surface.h,
                c_grid(),
            );
            gx += step_x;
        }
        let step_y = (surface.h / 10).max(16);
        let mut gy = surface.y + step_y;
        while gy < surface.y + surface.h {
            fill_rect(
                buf,
                ww,
                wh,
                surface.x,
                gy,
                surface.x + surface.w,
                gy + 1,
                c_grid(),
            );
            gy += step_y;
        }
        let cx = surface.x + surface.w / 2;
        let cy = surface.y + surface.h / 2;
        fill_rect(
            buf,
            ww,
            wh,
            cx,
            surface.y,
            cx + 1,
            surface.y + surface.h,
            pack_rgb(45, 50, 70),
        );
        fill_rect(
            buf,
            ww,
            wh,
            surface.x,
            cy,
            surface.x + surface.w,
            cy + 1,
            pack_rgb(45, 50, 70),
        );

        // Breadcrumb + resolution (high contrast strip)
        fill_rect(
            buf,
            ww,
            wh,
            surface.x,
            surface.y,
            surface.x + surface.w,
            surface.y + 12 + 8 * zoom,
            pack_rgb(18, 22, 34),
        );
        let res_label = format!(
            "{}  |  {}x{}{}",
            if layers.breadcrumb.is_empty() {
                layers.active_id
            } else {
                layers.breadcrumb
            },
            layers.res_w as i32,
            layers.res_h as i32,
            if layers.animating {
                " anim"
            } else if layers.editable {
                ""
            } else {
                " LOCK"
            }
        );
        txt(
            buf,
            ww,
            wh,
            surface.x + 8,
            surface.y + 4,
            &res_label
                .chars()
                .take(lay.max_chars_in(surface.w - 16))
                .collect::<String>(),
            if layers.animating {
                c_cta_hi()
            } else {
                c_text()
            },
            1,
            zoom,
        );

        for w in canvas_widgets {
            let (x, y) = parse_pct_pair(w.position.as_deref().unwrap_or("(50%,50%)"));
            let (sw, sh) = parse_pct_pair(w.size.as_deref().unwrap_or("(18%,8%)"));
            // Widget chrome scales with UI zoom so labels fit inside buttons.
            let min_w = (56 + 28 * zoom) as f32;
            let min_h = (22 + 14 * zoom) as f32;
            let max_h = (40 + 22 * zoom) as f32;
            let bw = ((sw / 100.0) * surface.w as f32)
                .clamp(min_w.min(surface.w as f32 * 0.5), surface.w as f32 * 0.7)
                as i32;
            let bh = ((sh / 100.0) * surface.h as f32).clamp(
                min_h.min(surface.h as f32 * 0.3),
                max_h.min(surface.h as f32 * 0.4),
            ) as i32;
            let px = surface.x + ((x / 100.0) * surface.w as f32) as i32 - bw / 2;
            let py = surface.y + ((y / 100.0) * surface.h as f32) as i32 - bh / 2;
            let sel = selected == Some(w.id.as_str());
            let is_drag_sel = sel && dragging;

            // drop shadow
            fill_rect(
                buf,
                ww,
                wh,
                px + 4,
                py + 5,
                px + bw + 4,
                py + bh + 5,
                c_shadow(),
            );

            let fill = if is_drag_sel {
                pack_rgb(80, 120, 200)
            } else if sel {
                c_sel()
            } else if w.kind == "panel" {
                c_widget_panel()
            } else if w.kind == "label" {
                pack_rgb(48, 58, 78)
            } else {
                c_widget()
            };
            fill_rect(buf, ww, wh, px, py, px + bw, py + bh, fill);

            // top highlight strip
            fill_rect(
                buf,
                ww,
                wh,
                px,
                py,
                px + bw,
                py + (2 + zoom / 2),
                if sel {
                    c_accent_hi()
                } else {
                    pack_rgb(90, 85, 130)
                },
            );

            if sel {
                rect_outline(
                    buf,
                    ww,
                    wh,
                    px - 2,
                    py - 2,
                    px + bw + 2,
                    py + bh + 2,
                    c_sel_ring(),
                    2,
                );
                // corner handles scale with zoom
                let hs = 6 + 2 * zoom;
                for (hx, hy) in [
                    (px - 3, py - 3),
                    (px + bw - hs + 3, py - 3),
                    (px - 3, py + bh - hs + 3),
                    (px + bw - hs + 3, py + bh - hs + 3),
                ] {
                    fill_rect(buf, ww, wh, hx, hy, hx + hs, hy + hs, c_sel_ring());
                    rect_outline(
                        buf,
                        ww,
                        wh,
                        hx,
                        hy,
                        hx + hs,
                        hy + hs,
                        pack_rgb(40, 35, 20),
                        1,
                    );
                }
            }

            let label = w.text.as_deref().unwrap_or(w.id.as_str());
            let label_chars = lay.max_chars_in(bw - 20);
            let text_x = px + 8 + 2 * zoom;
            let text_y = py + (bh - 8 * zoom).max(2) / 2;
            txt(
                buf,
                ww,
                wh,
                text_x,
                text_y,
                &label.chars().take(label_chars).collect::<String>(),
                c_text(),
                1,
                zoom,
            );

            // kind badge top-right of widget
            let badge = match w.kind.as_str() {
                "label" => "LBL",
                "panel" => "PNL",
                _ => "BTN",
            };
            let badge_w = 3 * lay.char_w();
            txt(
                buf,
                ww,
                wh,
                px + bw - badge_w - 4,
                py + 2 + zoom,
                badge,
                if sel {
                    pack_rgb(200, 210, 255)
                } else {
                    c_text_dim()
                },
                1,
                zoom,
            );
        }
    } else if mode == PaintMode::Script {
        // VScript editor — full document + API strip
        fill_rect(
            buf,
            ww,
            wh,
            lay.canvas_x + 4,
            lay.canvas_y + 4,
            lay.canvas_x + lay.canvas_w - 4,
            lay.canvas_y + lay.canvas_h - 4,
            pack_rgb(10, 12, 18),
        );
        txt(
            buf,
            ww,
            wh,
            lay.canvas_x + 12,
            lay.canvas_y + 8,
            "VSCRIPT  layer/button/game/scene  |  F2 validate  F3-F8 insert API",
            pack_rgb(120, 220, 160),
            1,
            zoom,
        );
        let line_h = 10 + 6 * zoom;
        let gutter_w = 28 + 8 * zoom;
        fill_rect(
            buf,
            ww,
            wh,
            lay.canvas_x + 4,
            lay.canvas_y + 22 + 4 * zoom,
            lay.canvas_x + 4 + gutter_w,
            lay.canvas_y + lay.canvas_h - 4,
            pack_rgb(16, 18, 28),
        );
        let max_lines = ((lay.canvas_h - 40 - 8 * zoom) / line_h).max(4) as usize;
        let api_h = 14 + 10 * zoom;
        let text_bottom = lay.canvas_y + lay.canvas_h - 4 - api_h;
        for (i, line) in advanced_src.lines().take(max_lines).enumerate() {
            let y = lay.canvas_y + 28 + 4 * zoom + i as i32 * line_h;
            if y + line_h > text_bottom {
                break;
            }
            let ln = i + 1;
            let is_cur = i == layers.script_cursor_line;
            if is_cur {
                fill_rect(
                    buf,
                    ww,
                    wh,
                    lay.canvas_x + 4 + gutter_w,
                    y - 1,
                    lay.canvas_x + lay.canvas_w - 4,
                    y + line_h - 2,
                    pack_rgb(30, 40, 60),
                );
            }
            txt(
                buf,
                ww,
                wh,
                lay.canvas_x + 8,
                y,
                &format!("{ln:>3}"),
                if is_cur { c_cta_hi() } else { c_text_dim() },
                1,
                zoom,
            );
            let kind = classify_line(line);
            let col = match kind {
                SyntaxKind::Comment => pack_rgb(120, 150, 200),
                SyntaxKind::Keyword => pack_rgb(200, 160, 255),
                SyntaxKind::Flow => pack_rgb(120, 220, 200),
                SyntaxKind::String => pack_rgb(180, 220, 160),
                SyntaxKind::Normal => pack_rgb(200, 210, 220),
            };
            let max_c = lay.max_chars_in(lay.canvas_w - gutter_w - 24);
            txt(
                buf,
                ww,
                wh,
                lay.canvas_x + 8 + gutter_w,
                y,
                &line.chars().take(max_c).collect::<String>(),
                col,
                1,
                zoom,
            );
        }
        // issues
        if let Some(iss) = layers.script_issues.first() {
            txt(
                buf,
                ww,
                wh,
                lay.canvas_x + 12,
                text_bottom - line_h,
                &format!("! {iss}").chars().take(60).collect::<String>(),
                pack_rgb(255, 140, 120),
                1,
                zoom,
            );
        }
        // API catalog strip
        fill_rect(
            buf,
            ww,
            wh,
            lay.canvas_x + 4,
            text_bottom,
            lay.canvas_x + lay.canvas_w - 4,
            lay.canvas_y + lay.canvas_h - 4,
            pack_rgb(22, 28, 40),
        );
        let mut ax = lay.canvas_x + 10;
        let ay = text_bottom + 4;
        txt(buf, ww, wh, ax, ay, "API", c_text_dim(), 1, zoom);
        ax += 28;
        for (i, (cat, snip, _desc)) in api_catalog().iter().take(8).enumerate() {
            let short: String = snip.chars().take(14).collect();
            let label = format!("F{}:{cat}", i + 3);
            txt(
                buf,
                ww,
                wh,
                ax,
                ay,
                &label,
                pack_rgb(160, 200, 255),
                1,
                zoom,
            );
            ax += (label.len() as i32 + 2) * 6 * zoom;
            let _ = short;
            if ax > lay.canvas_x + lay.canvas_w - 80 {
                break;
            }
        }
    } else {
        // ── Nodes mode — polished layer graph ─────────────────────────────
        let cx0 = lay.canvas_x;
        let cy0 = lay.canvas_y;
        let cw = lay.canvas_w;
        let ch = lay.canvas_h;
        fill_rect(
            buf,
            ww,
            wh,
            cx0,
            cy0,
            cx0 + cw,
            cy0 + ch,
            pack_rgb(12, 14, 22),
        );
        // subtle dotted grid
        let gstep = 32 + 4 * zoom;
        let mut gx = cx0 + gstep;
        while gx < cx0 + cw {
            fill_rect(buf, ww, wh, gx, cy0, gx + 1, cy0 + ch, pack_rgb(22, 26, 36));
            gx += gstep;
        }
        let mut gy = cy0 + gstep;
        while gy < cy0 + ch {
            fill_rect(buf, ww, wh, cx0, gy, cx0 + cw, gy + 1, pack_rgb(22, 26, 36));
            gy += gstep;
        }

        // Top bar
        let top_bar = 28 + 8 * zoom;
        fill_rect(
            buf,
            ww,
            wh,
            cx0,
            cy0,
            cx0 + cw,
            cy0 + top_bar,
            pack_rgb(20, 24, 36),
        );
        fill_rect(
            buf,
            ww,
            wh,
            cx0,
            cy0 + top_bar - 1,
            cx0 + cw,
            cy0 + top_bar,
            c_border_soft(),
        );
        txt(
            buf,
            ww,
            wh,
            cx0 + 10,
            cy0 + 6,
            "NODES  —  pantallas graph",
            c_text(),
            1,
            zoom,
        );
        let tool = layers.nodes_tool;
        let tool_hint = match tool {
            "connect" => "CONNECT: click A then B",
            "disconnect" => "DISCONNECT: click edge or A then B",
            "overlay" => "OVERLAY: click A then B (show)",
            _ => "SELECT: click node, drag move, click edge",
        };
        txt(
            buf,
            ww,
            wh,
            cx0 + 10 + 22 * 6 * zoom,
            cy0 + 6,
            tool_hint,
            c_cta_hi(),
            1,
            zoom,
        );
        if let Some(from) = layers.connect_from {
            txt(
                buf,
                ww,
                wh,
                cx0 + 10,
                cy0 + 14 + 2 * zoom,
                &format!("from: {from}  →  click target"),
                pack_rgb(255, 220, 120),
                1,
                zoom,
            );
        }

        // Bottom tools strip
        let bot_h = 34 + 8 * zoom;
        let by0 = cy0 + ch - bot_h;
        fill_rect(
            buf,
            ww,
            wh,
            cx0,
            by0,
            cx0 + cw,
            cy0 + ch,
            pack_rgb(18, 22, 34),
        );
        fill_rect(buf, ww, wh, cx0, by0, cx0 + cw, by0 + 1, c_border_soft());
        let tools = [
            ("1 Select", "select"),
            ("2 Connect", "connect"),
            ("3 Cut", "disconnect"),
            ("4 Overlay", "overlay"),
            ("N Screen", "new"),
            ("S Sub", "sub"),
            ("Del", "del"),
        ];
        let mut tx = cx0 + 8;
        for (label, id) in tools {
            let active = tool == id;
            let tw = (label.len() as i32) * 6 * zoom + 16;
            let fill = if tool == id {
                c_accent()
            } else {
                c_surface_2()
            };
            fill_rect(buf, ww, wh, tx, by0 + 6, tx + tw, by0 + bot_h - 6, fill);
            rect_outline(
                buf,
                ww,
                wh,
                tx,
                by0 + 6,
                tx + tw,
                by0 + bot_h - 6,
                if tool == id {
                    c_accent_hi()
                } else {
                    c_border()
                },
                1,
            );
            txt(buf, ww, wh, tx + 6, by0 + 10, label, c_text(), 1, zoom);
            let _ = active;
            tx += tw + 8;
        }
        txt(
            buf,
            ww,
            wh,
            cx0 + cw - 200,
            by0 + 10,
            &format!("{} edges", layers.edges.len()),
            c_text_muted(),
            1,
            zoom,
        );

        let graph_top = cy0 + top_bar + 4;
        let graph_bot = by0 - 4;
        let graph_h = (graph_bot - graph_top).max(40);

        // edges
        for e in layers.edges {
            let a = layers.node_layout.iter().find(|(id, _, _)| id == &e.from);
            let b = layers.node_layout.iter().find(|(id, _, _)| id == &e.to);
            if let (Some((_, ax, ay)), Some((_, bx, by))) = (a, b) {
                let (x0, y0, nw, nh) = crate::layers::LayerStack::node_screen_rect(
                    cx0, graph_top, cw, graph_h, *ax, *ay, zoom,
                );
                let (x1, y1, _nw1, nh1) = crate::layers::LayerStack::node_screen_rect(
                    cx0, graph_top, cw, graph_h, *bx, *by, zoom,
                );
                let axp = x0 + nw;
                let ayp = y0 + nh / 2;
                let bxp = x1;
                let byp = y1 + nh1 / 2;
                let mx = (axp + bxp) / 2;
                let sel = layers.selected_edge == Some((e.from.as_str(), e.to.as_str()));
                let col = if sel {
                    pack_rgb(255, 200, 90)
                } else {
                    match e.kind {
                        crate::layers::LayerEdgeKind::Transition => pack_rgb(100, 170, 255),
                        crate::layers::LayerEdgeKind::Overlay => pack_rgb(180, 130, 255),
                        crate::layers::LayerEdgeKind::Back => pack_rgb(120, 220, 160),
                    }
                };
                let thick = if sel { 3 } else { 2 };
                // orthogonal path
                fill_rect(
                    buf,
                    ww,
                    wh,
                    axp.min(mx),
                    ayp - thick / 2,
                    axp.max(mx) + 1,
                    ayp + thick / 2 + 1,
                    col,
                );
                fill_rect(
                    buf,
                    ww,
                    wh,
                    mx - thick / 2,
                    ayp.min(byp),
                    mx + thick / 2 + 1,
                    ayp.max(byp) + 1,
                    col,
                );
                fill_rect(
                    buf,
                    ww,
                    wh,
                    mx.min(bxp),
                    byp - thick / 2,
                    mx.max(bxp) + 1,
                    byp + thick / 2 + 1,
                    col,
                );
                // arrow head
                fill_rect(buf, ww, wh, bxp - 6, byp - 4, bxp, byp + 5, col);
                fill_rect(buf, ww, wh, bxp - 10, byp - 2, bxp - 4, byp + 3, col);
                // kind badge mid
                let kind_l = e.kind.as_str();
                let lab = e
                    .label
                    .as_deref()
                    .map(|s| format!("{kind_l}:{s}"))
                    .unwrap_or_else(|| kind_l.to_string());
                fill_rect(
                    buf,
                    ww,
                    wh,
                    mx - 4,
                    (ayp + byp) / 2 - 8,
                    mx + (lab.len() as i32) * 6 * zoom + 8,
                    (ayp + byp) / 2 + 8 + 2 * zoom,
                    pack_rgb(16, 18, 28),
                );
                txt(
                    buf,
                    ww,
                    wh,
                    mx,
                    (ayp + byp) / 2 - 4,
                    &lab.chars().take(16).collect::<String>(),
                    col,
                    1,
                    zoom,
                );
            }
        }

        // nodes
        for (id, px, py) in layers.node_layout {
            let (x0, y0, nw, nh) = crate::layers::LayerStack::node_screen_rect(
                cx0, graph_top, cw, graph_h, *px, *py, zoom,
            );
            let active = id == layers.active_id;
            let from = layers.connect_from == Some(id.as_str());
            let layer = layers.layers.iter().find(|l| l.id == *id);
            let is_root = layer.map(|l| l.parent.is_none()).unwrap_or(true);
            // shadow
            fill_rect(
                buf,
                ww,
                wh,
                x0 + 3,
                y0 + 4,
                x0 + nw + 3,
                y0 + nh + 4,
                pack_rgb(4, 6, 10),
            );
            let fill = if from {
                pack_rgb(40, 90, 60)
            } else if active {
                pack_rgb(45, 70, 130)
            } else if is_root {
                pack_rgb(40, 46, 64)
            } else {
                pack_rgb(32, 38, 52)
            };
            fill_rect(buf, ww, wh, x0, y0, x0 + nw, y0 + nh, fill);
            // top accent strip
            fill_rect(
                buf,
                ww,
                wh,
                x0,
                y0,
                x0 + nw,
                y0 + 3,
                if is_root {
                    c_accent_hi()
                } else {
                    pack_rgb(80, 160, 200)
                },
            );
            rect_outline(
                buf,
                ww,
                wh,
                x0,
                y0,
                x0 + nw,
                y0 + nh,
                if from {
                    pack_rgb(120, 255, 160)
                } else if active {
                    pack_rgb(140, 190, 255)
                } else {
                    c_border()
                },
                if from || active { 2 } else { 1 },
            );
            // ports
            let port = 6 + zoom;
            fill_rect(
                buf,
                ww,
                wh,
                x0 - port / 2,
                y0 + nh / 2 - port / 2,
                x0 + port / 2,
                y0 + nh / 2 + port / 2,
                pack_rgb(90, 200, 255),
            );
            fill_rect(
                buf,
                ww,
                wh,
                x0 + nw - port / 2,
                y0 + nh / 2 - port / 2,
                x0 + nw + port / 2,
                y0 + nh / 2 + port / 2,
                pack_rgb(120, 255, 160),
            );
            let name = layer.map(|l| l.name.as_str()).unwrap_or(id.as_str());
            let res = layer
                .map(|l| format!("{}x{}", l.width_px, l.height_px))
                .unwrap_or_default();
            let locked = layer.map(|l| l.locked).unwrap_or(false);
            txt(
                buf,
                ww,
                wh,
                x0 + 8,
                y0 + 8,
                &name.chars().take(14).collect::<String>(),
                c_text(),
                1,
                zoom,
            );
            let sub = if locked { format!("{res} #") } else { res };
            txt(
                buf,
                ww,
                wh,
                x0 + 8,
                y0 + 8 + 11 * zoom,
                &sub,
                c_text_dim(),
                1,
                zoom,
            );
        }
    }
}

/// Hit-test bottom nodes toolbar. Returns tool id string.
pub fn hit_nodes_toolbar(
    layout: &StudioLayout,
    sx: f64,
    sy: f64,
    zoom: i32,
) -> Option<&'static str> {
    let bot_h = 34 + 8 * zoom;
    let by0 = layout.canvas_y + layout.canvas_h - bot_h;
    let y = sy as i32;
    let x = sx as i32;
    if y < by0 + 6 || y > by0 + bot_h - 6 {
        return None;
    }
    if x < layout.canvas_x || x >= layout.canvas_x + layout.canvas_w {
        return None;
    }
    let tools = [
        ("1 Select", "select"),
        ("2 Connect", "connect"),
        ("3 Cut", "disconnect"),
        ("4 Overlay", "overlay"),
        ("N Screen", "new"),
        ("S Sub", "sub"),
        ("Del", "del"),
    ];
    let mut tx = layout.canvas_x + 8;
    for (label, id) in tools {
        let tw = (label.len() as i32) * 6 * zoom + 16;
        if x >= tx && x < tx + tw {
            return Some(id);
        }
        tx += tw + 8;
    }
    None
}
