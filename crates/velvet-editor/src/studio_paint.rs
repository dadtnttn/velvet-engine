//! Polished dual-mode Studio chrome + canvas paint (softbuffer ARGB).
//!
//! Visual language: OLED dark editor (slate base, violet accent, green CTA).
//! Hit zones for palette, hierarchy, toolbar, and canvas support real pointer UX.

use velvet_document::DesignerWidget;
use velvet_story::{draw_text_line, fill_rect, pack_rgb};

// ── Design tokens (ARGB via pack_rgb) ──────────────────────────────────────

fn c_bg() -> u32 {
    pack_rgb(15, 17, 26)
}
fn c_surface() -> u32 {
    pack_rgb(22, 25, 38)
}
fn c_surface_2() -> u32 {
    pack_rgb(30, 34, 52)
}
fn c_border() -> u32 {
    pack_rgb(55, 62, 90)
}
fn c_border_soft() -> u32 {
    pack_rgb(40, 45, 68)
}
fn c_text() -> u32 {
    pack_rgb(240, 242, 250)
}
fn c_text_muted() -> u32 {
    pack_rgb(140, 148, 175)
}
fn c_text_dim() -> u32 {
    pack_rgb(100, 108, 135)
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

/// Layout metrics for the Studio window.
#[derive(Debug, Clone, Copy)]
pub struct StudioLayout {
    pub ww: i32,
    pub wh: i32,
    pub left_w: i32,
    pub right_w: i32,
    pub top_h: i32,
    pub bot_h: i32,
    pub canvas_x: i32,
    pub canvas_y: i32,
    pub canvas_w: i32,
    pub canvas_h: i32,
    /// Y where palette section starts (for hit tests).
    pub palette_y: i32,
    /// Y of first hierarchy row.
    pub hierarchy_y: i32,
}

impl StudioLayout {
    pub fn new(ww: u32, wh: u32) -> Self {
        let ww = ww as i32;
        let wh = wh as i32;
        let left_w = ((ww as f32 * 0.16).round() as i32).clamp(180, 260);
        let right_w = ((ww as f32 * 0.19).round() as i32).clamp(200, 300);
        let top_h = 48;
        let bot_h = 52;
        let gap = 10;
        let canvas_x = left_w + gap;
        let canvas_y = top_h + gap;
        let canvas_w = (ww - left_w - right_w - gap * 2).max(80);
        let canvas_h = (wh - top_h - bot_h - gap * 2).max(80);
        // Hierarchy header 28 + ~5 rows → palette below; fixed offset for hit-test
        let hierarchy_y = top_h + 36;
        // After ~6 hierarchy rows + header for palette
        let palette_y = hierarchy_y + 6 * 24 + 36;
        Self {
            ww,
            wh,
            left_w,
            right_w,
            top_h,
            bot_h,
            canvas_x,
            canvas_y,
            canvas_w,
            canvas_h,
            palette_y,
            hierarchy_y,
        }
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

    /// Hit test toolbar mode pills / save. Returns action id.
    pub fn hit_toolbar(&self, sx: f64, sy: f64) -> Option<&'static str> {
        let x = sx as i32;
        let y = sy as i32;
        if y < 8 || y > 40 {
            return None;
        }
        let pill_x = self.ww - 380;
        if x >= pill_x && x < pill_x + 100 {
            return Some("mode_visual");
        }
        if x >= pill_x + 108 && x < pill_x + 216 {
            return Some("mode_script");
        }
        if x >= pill_x + 230 && x < pill_x + 320 {
            return Some("save");
        }
        None
    }

    /// Hit hierarchy row → widget index among canvas widgets (0-based).
    pub fn hit_hierarchy(&self, sy: f64, widget_count: usize) -> Option<usize> {
        let y = sy as i32;
        if y < self.hierarchy_y || y >= self.palette_y - 28 {
            return None;
        }
        let row = ((y - self.hierarchy_y) / 24) as usize;
        if row < widget_count.min(6) {
            Some(row)
        } else {
            None
        }
    }

    /// Hit palette item: 0=button, 1=label, 2=panel.
    pub fn hit_palette(&self, sx: f64, sy: f64) -> Option<&'static str> {
        let x = sx as i32;
        let y = sy as i32;
        if x < 10 || x > self.left_w - 10 {
            return None;
        }
        let base = self.palette_y + 34;
        let h = 34;
        let gap = 8;
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
        let base = self.top_h + 42;
        // each field: label 16 + box 20 + gap ~12 → 48
        let row_h = 48;
        let row = ((y - base) / row_h) as i32;
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

fn draw_panel_header(buf: &mut [u32], ww: u32, wh: u32, x0: i32, y0: i32, x1: i32, title: &str) {
    fill_rect(buf, ww, wh, x0, y0, x1, y0 + 30, c_surface_2());
    fill_rect(buf, ww, wh, x0, y0 + 29, x1, y0 + 30, c_border_soft());
    // accent bar on left of header
    fill_rect(buf, ww, wh, x0, y0, x0 + 3, y0 + 30, c_accent());
    draw_text_line(buf, ww, wh, x0 + 12, y0 + 9, title, c_text_muted(), 2);
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
) {
    let fill = if active { c_accent() } else { c_surface_2() };
    fill_rect(buf, ww, wh, x0, y0, x1, y1, fill);
    if active {
        rect_outline(buf, ww, wh, x0, y0, x1, y1, c_accent_hi(), 1);
    } else {
        rect_outline(buf, ww, wh, x0, y0, x1, y1, c_border(), 1);
    }
    let tw = (label.len() as i32) * 7;
    let tx = x0 + ((x1 - x0) - tw) / 2;
    let ty = y0 + ((y1 - y0) - 10) / 2;
    draw_text_line(
        buf,
        ww,
        wh,
        tx.max(x0 + 4),
        ty,
        label,
        if active { c_text() } else { c_text_muted() },
        1,
    );
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
) {
    let ww = layout.ww as u32;
    let wh = layout.wh as u32;
    let lay = *layout;

    // App background
    fill_rect(buf, ww, wh, 0, 0, lay.ww, lay.wh, c_bg());

    // ── Top toolbar ────────────────────────────────────────────────────────
    fill_rect(buf, ww, wh, 0, 0, lay.ww, lay.top_h, c_surface());
    fill_rect(buf, ww, wh, 0, lay.top_h - 1, lay.ww, lay.top_h, c_border());
    // brand mark
    fill_rect(buf, ww, wh, 12, 14, 28, 34, c_accent());
    draw_text_line(buf, ww, wh, 36, 16, "VELVET STUDIO", c_text(), 2);
    draw_text_line(buf, ww, wh, 210, 18, project_name, c_text_dim(), 2);

    // Mode pills + Save
    let pill_x = lay.ww - 380;
    draw_pill(
        buf,
        ww,
        wh,
        pill_x,
        10,
        pill_x + 100,
        38,
        mode_simplified,
        "1 Visual",
    );
    draw_pill(
        buf,
        ww,
        wh,
        pill_x + 108,
        10,
        pill_x + 216,
        38,
        !mode_simplified,
        "2 Script",
    );
    // Save CTA
    fill_rect(buf, ww, wh, pill_x + 230, 10, pill_x + 320, 38, c_cta());
    rect_outline(buf, ww, wh, pill_x + 230, 10, pill_x + 320, 38, c_cta_hi(), 1);
    draw_text_line(buf, ww, wh, pill_x + 250, 18, "Save", c_text(), 2);

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

    draw_panel_header(buf, ww, wh, 0, lay.top_h, lay.left_w, "HIERARCHY");
    let canvas_widgets: Vec<&DesignerWidget> = widgets.iter().filter(|w| is_canvas_widget(w)).collect();
    let mut hy = lay.hierarchy_y;
    for (i, w) in canvas_widgets.iter().take(6).enumerate() {
        let sel = selected == Some(w.id.as_str());
        if sel {
            fill_rect(buf, ww, wh, 4, hy - 2, lay.left_w - 4, hy + 18, pack_rgb(45, 55, 100));
            fill_rect(buf, ww, wh, 4, hy - 2, 7, hy + 18, c_accent_hi());
        }
        let kind_mark = match w.kind.as_str() {
            "label" => "L",
            "panel" => "P",
            _ => "B",
        };
        fill_rect(buf, ww, wh, 12, hy, 26, hy + 14, c_surface_2());
        draw_text_line(buf, ww, wh, 15, hy + 2, kind_mark, c_accent_hi(), 1);
        let label = w.text.as_deref().unwrap_or(w.id.as_str());
        let line = format!("{}", label);
        draw_text_line(
            buf,
            ww,
            wh,
            32,
            hy + 2,
            &line.chars().take(16).collect::<String>(),
            if sel { c_text() } else { c_text_muted() },
            1,
        );
        hy += 24;
        let _ = i;
    }
    if canvas_widgets.is_empty() {
        draw_text_line(buf, ww, wh, 14, hy, "No widgets yet", c_text_dim(), 1);
        hy += 20;
        draw_text_line(buf, ww, wh, 14, hy, "Use palette below", c_text_dim(), 1);
    }

    // Palette (aligned with hit_palette)
    let pal_y = lay.palette_y;
    draw_panel_header(buf, ww, wh, 0, pal_y, lay.left_w, "PALETTE");
    let mut py = pal_y + 34;
    for (label, accent) in [
        ("Button", pack_rgb(90, 80, 160)),
        ("Label", pack_rgb(70, 110, 140)),
        ("Panel", pack_rgb(60, 90, 100)),
    ] {
        // card-like palette item
        fill_rect(buf, ww, wh, 10, py, lay.left_w - 10, py + 34, c_surface_2());
        rect_outline(buf, ww, wh, 10, py, lay.left_w - 10, py + 34, c_border(), 1);
        fill_rect(buf, ww, wh, 10, py, 14, py + 34, accent);
        draw_text_line(buf, ww, wh, 24, py + 10, label, c_text(), 2);
        draw_text_line(buf, ww, wh, lay.left_w - 48, py + 12, "drag", c_text_dim(), 1);
        py += 42;
    }
    draw_text_line(
        buf,
        ww,
        wh,
        12,
        py + 6,
        "Click to place  B/L/P keys",
        c_text_dim(),
        1,
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
    draw_panel_header(buf, ww, wh, rx0, lay.top_h, lay.ww, "INSPECTOR");
    let mut iy = lay.top_h + 42;
    if let Some(id) = selected {
        if let Some(w) = widgets.iter().find(|w| w.id == id) {
            // Rows aligned with hit_inspector_field (row_h = 48)
            let fields: [( &str, String, Option<InspectorField>, bool); 5] = [
                ("ID", id.to_string(), None, false),
                ("KIND", w.kind.clone(), None, false),
                (
                    "TEXT  (name)",
                    w.text.as_deref().unwrap_or("").to_string(),
                    Some(InspectorField::Text),
                    true,
                ),
                (
                    "POS  x%,y%",
                    w.position.as_deref().unwrap_or("(50%, 50%)").to_string(),
                    Some(InspectorField::Pos),
                    true,
                ),
                (
                    "SIZE  w%,h%",
                    w.size.as_deref().unwrap_or("(18%, 8%)").to_string(),
                    Some(InspectorField::Size),
                    true,
                ),
            ];
            for (key, val, field, editable) in fields {
                let editing = field.is_some() && edit_field == field;
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    key,
                    if editable { c_text_muted() } else { c_text_dim() },
                    1,
                );
                if editable {
                    draw_text_line(
                        buf,
                        ww,
                        wh,
                        lay.ww - 52,
                        iy,
                        "edit",
                        c_text_dim(),
                        1,
                    );
                }
                iy += 16;
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
                    rx0 + 12,
                    iy - 2,
                    lay.ww - 12,
                    iy + 20,
                    box_fill,
                );
                if editing {
                    rect_outline(
                        buf,
                        ww,
                        wh,
                        rx0 + 12,
                        iy - 2,
                        lay.ww - 12,
                        iy + 20,
                        c_accent_hi(),
                        2,
                    );
                } else if editable {
                    rect_outline(
                        buf,
                        ww,
                        wh,
                        rx0 + 12,
                        iy - 2,
                        lay.ww - 12,
                        iy + 20,
                        c_border(),
                        1,
                    );
                }
                let shown = if editing {
                    format!("{edit_buf}|")
                } else {
                    val
                };
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 16,
                    iy + 3,
                    &shown.chars().take(26).collect::<String>(),
                    if editing { c_cta_hi() } else { c_text() },
                    1,
                );
                iy += 32; // total row ~48
            }
            iy += 4;
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
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 16,
                    iy + 6,
                    "Enter apply  Esc cancel",
                    c_text_muted(),
                    1,
                );
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 16,
                    iy + 20,
                    "type to edit selected field",
                    c_text_dim(),
                    1,
                );
                iy += 44;
            } else {
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    "Click field to edit",
                    c_text_dim(),
                    1,
                );
                iy += 16;
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    "T text  P pos  Z size",
                    c_text_dim(),
                    1,
                );
                iy += 16;
                draw_text_line(
                    buf,
                    ww,
                    wh,
                    rx0 + 14,
                    iy,
                    "Arrows nudge 1%",
                    c_text_dim(),
                    1,
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
                draw_text_line(buf, ww, wh, rx0 + 18, iy + 6, "DRAGGING...", c_cta_hi(), 1);
            }
        } else {
            draw_text_line(buf, ww, wh, rx0 + 14, iy, id, c_text_muted(), 1);
        }
    } else {
        draw_text_line(buf, ww, wh, rx0 + 14, iy, "No selection", c_text_dim(), 1);
        iy += 22;
        draw_text_line(buf, ww, wh, rx0 + 14, iy, "Click canvas widget", c_text_dim(), 1);
        iy += 18;
        draw_text_line(buf, ww, wh, rx0 + 14, iy, "or hierarchy row", c_text_dim(), 1);
        iy += 24;
        draw_text_line(
            buf,
            ww,
            wh,
            rx0 + 14,
            iy,
            "Then edit TEXT / POS / SIZE",
            c_text_dim(),
            1,
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
    draw_text_line(
        buf,
        ww,
        wh,
        14,
        lay.wh - lay.bot_h + 8,
        "STATUS",
        c_text_dim(),
        1,
    );
    draw_text_line(
        buf,
        ww,
        wh,
        14,
        lay.wh - lay.bot_h + 26,
        &status.chars().take(80).collect::<String>(),
        c_text_muted(),
        1,
    );
    draw_text_line(
        buf,
        ww,
        wh,
        lay.ww - 480,
        lay.wh - lay.bot_h + 26,
        "Tab mode  |  palette  |  drag  |  edit TEXT/POS/SIZE  |  Ctrl+S  |  Esc",
        c_text_dim(),
        1,
    );

    // ── Canvas frame ───────────────────────────────────────────────────────
    // Outer chrome with soft border
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
        lay.canvas_x - 1,
        lay.canvas_y - 1,
        lay.canvas_x + lay.canvas_w + 1,
        lay.canvas_y + lay.canvas_h + 1,
        c_border_soft(),
    );
    fill_rect(
        buf,
        ww,
        wh,
        lay.canvas_x,
        lay.canvas_y,
        lay.canvas_x + lay.canvas_w,
        lay.canvas_y + lay.canvas_h,
        c_canvas(),
    );

    if mode_simplified {
        // Subtle grid (10% steps)
        let step_x = (lay.canvas_w / 10).max(20);
        let mut gx = lay.canvas_x + step_x;
        while gx < lay.canvas_x + lay.canvas_w {
            fill_rect(
                buf,
                ww,
                wh,
                gx,
                lay.canvas_y,
                gx + 1,
                lay.canvas_y + lay.canvas_h,
                c_grid(),
            );
            gx += step_x;
        }
        let step_y = (lay.canvas_h / 10).max(20);
        let mut gy = lay.canvas_y + step_y;
        while gy < lay.canvas_y + lay.canvas_h {
            fill_rect(
                buf,
                ww,
                wh,
                lay.canvas_x,
                gy,
                lay.canvas_x + lay.canvas_w,
                gy + 1,
                c_grid(),
            );
            gy += step_y;
        }
        // center crosshair guides (light)
        let cx = lay.canvas_x + lay.canvas_w / 2;
        let cy = lay.canvas_y + lay.canvas_h / 2;
        fill_rect(
            buf,
            ww,
            wh,
            cx,
            lay.canvas_y,
            cx + 1,
            lay.canvas_y + lay.canvas_h,
            pack_rgb(45, 50, 70),
        );
        fill_rect(
            buf,
            ww,
            wh,
            lay.canvas_x,
            cy,
            lay.canvas_x + lay.canvas_w,
            cy + 1,
            pack_rgb(45, 50, 70),
        );

        draw_text_line(
            buf,
            ww,
            wh,
            lay.canvas_x + 12,
            lay.canvas_y + 10,
            "CANVAS  —  drag to move, snap 1%",
            c_text_dim(),
            1,
        );

        for w in canvas_widgets {
            let (x, y) = parse_pct_pair(w.position.as_deref().unwrap_or("(50%,50%)"));
            let (sw, sh) = parse_pct_pair(w.size.as_deref().unwrap_or("(18%,8%)"));
            let bw = ((sw / 100.0) * lay.canvas_w as f32)
                .clamp(80.0, lay.canvas_w as f32 * 0.55) as i32;
            let bh = ((sh / 100.0) * lay.canvas_h as f32).clamp(40.0, 80.0) as i32;
            let px = lay.canvas_x + ((x / 100.0) * lay.canvas_w as f32) as i32 - bw / 2;
            let py = lay.canvas_y + ((y / 100.0) * lay.canvas_h as f32) as i32 - bh / 2;
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
                py + 3,
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
                // corner handles
                let hs = 8;
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
            let text_x = px + 14;
            let text_y = py + bh / 2 - 6;
            draw_text_line(
                buf,
                ww,
                wh,
                text_x,
                text_y,
                &label.chars().take(18).collect::<String>(),
                c_text(),
                2,
            );

            // kind badge top-right of widget
            let badge = match w.kind.as_str() {
                "label" => "LBL",
                "panel" => "PNL",
                _ => "BTN",
            };
            draw_text_line(
                buf,
                ww,
                wh,
                px + bw - 36,
                py + 6,
                badge,
                if sel {
                    pack_rgb(200, 210, 255)
                } else {
                    c_text_dim()
                },
                1,
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
        draw_text_line(
            buf,
            ww,
            wh,
            lay.canvas_x + 14,
            lay.canvas_y + 14,
            "ADVANCED SCRIPT  —  same file, visual regions protected",
            pack_rgb(100, 190, 140),
            1,
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
            draw_text_line(
                buf,
                ww,
                wh,
                lay.canvas_x + 14,
                lay.canvas_y + 42 + i as i32 * 16,
                &format!("{ln:>3}"),
                c_text_dim(),
                1,
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
            draw_text_line(
                buf,
                ww,
                wh,
                lay.canvas_x + 56,
                lay.canvas_y + 42 + i as i32 * 16,
                &clipped,
                col,
                1,
            );
        }
    }
}
