//! Polished dual-mode Studio chrome + canvas paint (softbuffer ARGB).
//!
//! Visual language: OLED dark editor (slate base, violet accent, green CTA).
//! Hit zones for palette, hierarchy, toolbar, and canvas support real pointer UX.

use velvet_document::DesignerWidget;
use velvet_story::{draw_text_line, fill_rect, pack_rgb};

use crate::layers::{DesignSurface, LayerTreeRow, ScreenLayer};

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

    fn toolbar_pill_x(&self) -> i32 {
        self.ww - (self.pill_w * 2 + self.save_w + self.pad * 4 + 16)
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
        let gap = self.pad;
        if x >= pill_x && x < pill_x + self.pill_w {
            return Some("mode_visual");
        }
        let x2 = pill_x + self.pill_w + gap;
        if x >= x2 && x < x2 + self.pill_w {
            return Some("mode_script");
        }
        let x3 = x2 + self.pill_w + gap;
        if x >= x3 && x < x3 + self.save_w {
            return Some("save");
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
        x >= self.ww - self.right_w
            && x < self.ww
            && y >= self.top_h
            && y < self.wh - self.bot_h
    }

    /// Hit editable inspector field when a widget is selected.
    /// Row order: 0=id(ro), 1=kind(ro), 2=text, 3=pos, 4=size.
    pub fn hit_inspector_field(&self, sx: f64, sy: f64) -> Option<InspectorField> {
        if !self.contains_inspector(sx, sy) {
            return None;
        }
        let y = sy as i32;
        let base = self.top_h + self.header_h + self.pad;
        let row = ((y - base) / self.insp_row_h) as i32;
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
    matches!(
        w.kind.as_str(),
        "button" | "label" | "panel" | "widget"
    ) && (id.starts_with("button.")
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

fn rect_outline(buf: &mut [u32], ww: u32, wh: u32, x0: i32, y0: i32, x1: i32, y1: i32, c: u32, t: i32) {
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
    pub active_id: &'a str,
    pub breadcrumb: &'a str,
    pub res_w: f32,
    pub res_h: f32,
    pub animating: bool,
    pub editable: bool,
    pub pos_px: Option<(i32, i32)>,
}

/// Paint full Studio chrome + simplified widgets or advanced script.
pub fn paint_studio(
    buf: &mut [u32],
    layout: &StudioLayout,
    mode_simplified: bool,
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
    fill_rect(buf, ww, wh, lay.pad, mark_y, lay.pad + mark, mark_y + mark, c_accent());
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
    let name_x = lay.pad + mark + 8 + 14 * 6 * (zoom + 1) / 1;
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

    // Mode pills + Save (scaled)
    let pill_x = lay.toolbar_pill_x();
    let pill_y = (lay.top_h - lay.pill_h) / 2;
    draw_pill(
        buf,
        ww,
        wh,
        pill_x,
        pill_y,
        pill_x + lay.pill_w,
        pill_y + lay.pill_h,
        mode_simplified,
        "1 Visual",
        zoom,
    );
    let pill2 = pill_x + lay.pill_w + lay.pad;
    draw_pill(
        buf,
        ww,
        wh,
        pill2,
        pill_y,
        pill2 + lay.pill_w,
        pill_y + lay.pill_h,
        !mode_simplified,
        "2 Script",
        zoom,
    );
    let save_x = pill2 + lay.pill_w + lay.pad;
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
    fill_rect(buf, ww, wh, 0, lay.top_h, lay.left_w, lay.wh - lay.bot_h, c_surface());
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
        txt(
            buf,
            ww,
            wh,
            indent,
            ty,
            mark,
            c_text_muted(),
            1,
            zoom,
        );
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
        txt(
            buf,
            ww,
            wh,
            14,
            by + 1,
            kind_mark,
            c_accent_hi(),
            1,
            zoom,
        );
        let label = w.text.as_deref().unwrap_or(w.id.as_str());
        let line = format!("{}", label);
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
            "No widgets yet",
            c_text_dim(),
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
            "Use palette below",
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
    fill_rect(buf, ww, wh, rx0, lay.top_h, lay.ww, lay.wh - lay.bot_h, c_surface());
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
                let shown = if editing {
                    format!("{edit_buf}|")
                } else {
                    val
                };
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
                    &format!("{px}, {py}  ({}x{})", layers.res_w as i32, layers.res_h as i32),
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
                    1, zoom,
                );
                txt(
                    buf,
                    ww,
                    wh,
                    rx0 + 16,
                    iy + 20,
                    "type to edit selected field",
                    c_text_dim(),
                    1, zoom,
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
                    1, zoom,
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
                    1, zoom,
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
                    1, zoom,
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
                txt(buf, ww, wh, rx0 + 18, iy + 6, "DRAGGING...", c_cta_hi(), 1, zoom);
            }
        } else {
            txt(buf, ww, wh, rx0 + 14, iy, id, c_text_muted(), 1, zoom);
        }
    } else {
        txt(buf, ww, wh, rx0 + 14, iy, "No selection", c_text_dim(), 1, zoom);
        iy += 22;
        txt(buf, ww, wh, rx0 + 14, iy, "Click canvas widget", c_text_dim(), 1, zoom);
        iy += 18;
        txt(buf, ww, wh, rx0 + 14, iy, "or hierarchy row", c_text_dim(), 1, zoom);
        iy += 24;
        txt(
            buf,
            ww,
            wh,
            rx0 + 14,
            iy,
            "Then edit TEXT / POS / SIZE",
            c_text_dim(),
            1, zoom,
        );
    }

    // ── Bottom console ─────────────────────────────────────────────────────
    fill_rect(buf, ww, wh, 0, lay.wh - lay.bot_h, lay.ww, lay.wh, pack_rgb(10, 12, 18));
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

    if mode_simplified {
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
            let bh = ((sh / 100.0) * surface.h as f32)
                .clamp(min_h.min(surface.h as f32 * 0.3), max_h.min(surface.h as f32 * 0.4))
                as i32;
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
    } else {
        // Advanced script view
        fill_rect(
            buf,
            ww,
            wh,
            lay.canvas_x + 6,
            lay.canvas_y + 6,
            lay.canvas_x + lay.canvas_w - 6,
            lay.canvas_y + lay.canvas_h - 6,
            pack_rgb(10, 12, 18),
        );
        txt(
            buf,
            ww,
            wh,
            lay.canvas_x + 14,
            lay.canvas_y + 14,
            "ADVANCED SCRIPT  —  same file, visual regions protected",
            pack_rgb(100, 190, 140),
            1, zoom,
        );
        // gutter
        fill_rect(
            buf,
            ww,
            wh,
            lay.canvas_x + 6,
            lay.canvas_y + 36,
            lay.canvas_x + 48,
            lay.canvas_y + lay.canvas_h - 6,
            pack_rgb(16, 18, 28),
        );
        let max_lines = ((lay.canvas_h - 52) / 16).max(4) as usize;
        for (i, line) in advanced_src.lines().take(max_lines).enumerate() {
            let ln = i + 1;
            txt(
                buf,
                ww,
                wh,
                lay.canvas_x + 14,
                lay.canvas_y + 42 + i as i32 * 16,
                &format!("{ln:>3}"),
                c_text_dim(),
                1, zoom,
            );
            let muted = line.trim_start().starts_with("// @");
            let col = if muted {
                pack_rgb(110, 140, 200)
            } else if line.contains("game.") || line.contains("scene.") {
                pack_rgb(190, 150, 255)
            } else if line.contains("text:") {
                pack_rgb(180, 220, 160)
            } else {
                pack_rgb(170, 200, 175)
            };
            let clipped: String = line.chars().take(70).collect();
            txt(
                buf,
                ww,
                wh,
                lay.canvas_x + 56,
                lay.canvas_y + 42 + i as i32 * 16,
                &clipped,
                col,
                1, zoom,
            );
        }
    }
}
