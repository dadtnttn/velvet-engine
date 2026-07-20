//! Title menu for **Luz de Estación** — high-quality literary VN lobby.
//!
//! - All text: **fontdue** TrueType with 2× supersample (no softbuffer bitmap font)
//! - Soft rounded chrome (distance-field AA), not square 8-bit boxes
//! - Layout scales to any framebuffer size for DPI-sharp presentation

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use fontdue::Font;
use velvet_story::pack_rgb;

/// Design reference width (layout units — UI is authored against this).
pub const DESIGN_W: u32 = 1280;
/// Design reference height.
pub const DESIGN_H: u32 = 720;

/// Default / headless compose width (Full HD — sharp enough, softbuffer-friendly).
pub const WW: u32 = 1920;
/// Default / headless compose height.
pub const WH: u32 = 1080;

/// Softbuffer CPU budget: longest edge of the compose buffer (window may be larger).
pub const MAX_COMPOSE_EDGE: u32 = 1920;

/// Compose size for a physical window: 1:1 when ≤ [`MAX_COMPOSE_EDGE`], else scaled down.
pub fn compose_size_for_window(dw: u32, dh: u32) -> (u32, u32) {
    let dw = dw.max(1);
    let dh = dh.max(1);
    let max_dim = dw.max(dh);
    if max_dim <= MAX_COMPOSE_EDGE {
        return (dw, dh);
    }
    let scale = MAX_COMPOSE_EDGE as f32 / max_dim as f32;
    (
        ((dw as f32) * scale).round().max(1.0) as u32,
        ((dh as f32) * scale).round().max(1.0) as u32,
    )
}

/// Menu entries (index = selection).
pub const MENU_ITEMS: &[&str] = &[
    "Nueva partida",
    "Continuar",
    "Galería",
    "Opciones",
    "Salir",
];

/// Which indices are currently interactive.
pub fn menu_enabled(i: usize) -> bool {
    matches!(i, 0 | 4)
}

/// RGB image buffer.
pub type RgbImage = (u32, u32, Vec<u32>);

/// Load packed RGB from disk.
pub fn load_rgb(path: &Path) -> Option<RgbImage> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut px = Vec::with_capacity((w * h) as usize);
    for p in rgba.pixels() {
        let [r, g, b, _] = p.0;
        px.push(pack_rgb(r, g, b));
    }
    Some((w, h, px))
}

/// Bilinear cover-blit.
pub fn blit_cover(pixels: &mut [u32], ww: u32, wh: u32, img: &RgbImage) {
    let (sw, sh, src) = img;
    if *sw == 0 || *sh == 0 {
        return;
    }
    for y in 0..wh {
        let v = (y as f32 + 0.5) * (*sh as f32) / wh as f32 - 0.5;
        for x in 0..ww {
            let u = (x as f32 + 0.5) * (*sw as f32) / ww as f32 - 0.5;
            pixels[(y * ww + x) as usize] = sample_rgb_bilinear(src, *sw, *sh, u, v);
        }
    }
}

fn sample_rgb_bilinear(src: &[u32], sw: u32, sh: u32, x: f32, y: f32) -> u32 {
    let w = sw as i32;
    let h = sh as i32;
    if w <= 0 || h <= 0 {
        return 0;
    }
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let at = |xx: i32, yy: i32| src[(yy as u32 * sw + xx as u32) as usize];
    let ch = |c: u32, s: u32| ((c >> s) & 0xFF) as f32;
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c00 = at(x0, y0);
    let c10 = at(x1, y0);
    let c01 = at(x0, y1);
    let c11 = at(x1, y1);
    let bl = |shift: u32| {
        lerp(
            lerp(ch(c00, shift), ch(c10, shift), fx),
            lerp(ch(c01, shift), ch(c11, shift), fx),
            fy,
        ) as u8
    };
    pack_rgb(bl(16), bl(8), bl(0))
}

fn fill(pixels: &mut [u32], ww: u32, wh: u32, rgb: (u8, u8, u8)) {
    let c = pack_rgb(rgb.0, rgb.1, rgb.2);
    for p in pixels.iter_mut().take((ww * wh) as usize) {
        *p = c;
    }
}

fn blend(dst: u32, src: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((src >> 16) & 0xFF) as f32;
    let sg = ((src >> 8) & 0xFF) as f32;
    let sb = (src & 0xFF) as f32;
    pack_rgb(
        (dr + (sr - dr) * t) as u8,
        (dg + (sg - dg) * t) as u8,
        (db + (sb - db) * t) as u8,
    )
}

fn put(pixels: &mut [u32], ww: u32, wh: u32, x: i32, y: i32, rgb: (u8, u8, u8), a: f32) {
    if x < 0 || y < 0 || x >= ww as i32 || y >= wh as i32 || a <= 0.0 {
        return;
    }
    let i = (y as u32 * ww + x as u32) as usize;
    pixels[i] = blend(pixels[i], pack_rgb(rgb.0, rgb.1, rgb.2), a.clamp(0.0, 1.0));
}

/// Soft rounded rect with continuous AA (no square pixel corners).
fn rounded_panel(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    rgb: (u8, u8, u8),
    alpha: f32,
) {
    let a0 = alpha.clamp(0.0, 1.0);
    let r = radius.max(0.5);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;
    for row in y0.max(0)..y1.min(wh as i32) {
        for col in x0.max(0)..x1.min(ww as i32) {
            let px = col as f32 + 0.5;
            let py = row as f32 + 0.5;
            let cx0 = x + r;
            let cy0 = y + r;
            let cx1 = x + w - r;
            let cy1 = y + h - r;
            let dx = if px < cx0 {
                cx0 - px
            } else if px > cx1 {
                px - cx1
            } else {
                0.0
            };
            let dy = if py < cy0 {
                cy0 - py
            } else if py > cy1 {
                py - cy1
            } else {
                0.0
            };
            let dist = (dx * dx + dy * dy).sqrt();
            // smooth coverage: 1 inside, 0 outside, soft band ~1px
            let edge = ((dist - r) + 0.75).clamp(0.0, 1.5) / 1.5;
            let cov = (1.0 - edge) * (1.0 - edge) * a0;
            if cov > 0.01 {
                put(pixels, ww, wh, col, row, rgb, cov);
            }
        }
    }
}

fn soft_hline(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    x0: f32,
    x1: f32,
    y: f32,
    rgb: (u8, u8, u8),
    a: f32,
) {
    let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let yi = y.floor() as i32;
    let fy = y - yi as f32;
    for x in x0.floor() as i32..=x1.ceil() as i32 {
        put(pixels, ww, wh, x, yi, rgb, a * (1.0 - fy));
        put(pixels, ww, wh, x, yi + 1, rgb, a * fy * 0.85);
    }
}

fn vignette_bottom(pixels: &mut [u32], ww: u32, wh: u32, strength: f32) {
    let band = (wh as f32 * 0.58) as u32;
    for y in 0..band {
        let t = 1.0 - y as f32 / band as f32;
        let a = t * t * strength;
        let py = wh - 1 - y;
        for x in 0..ww {
            let i = (py * ww + x) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(6, 4, 14), a);
        }
    }
}

fn vignette_left(pixels: &mut [u32], ww: u32, wh: u32, width: u32, strength: f32) {
    for x in 0..width.min(ww) {
        let a = (1.0 - x as f32 / width as f32).powf(1.25) * strength;
        for y in 0..wh {
            let i = (y * ww + x) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(6, 4, 14), a);
        }
    }
}

// ── Fonts ─────────────────────────────────────────────────────────────────

struct Fonts {
    title: Font,
    ui: Font,
    /// Which files loaded (debug / tests).
    title_name: String,
    ui_name: String,
}

static FONTS: OnceLock<Option<Fonts>> = OnceLock::new();

/// Loaded font names for diagnostics.
pub fn font_status() -> Option<(String, String)> {
    fonts().map(|f| (f.title_name.clone(), f.ui_name.clone()))
}

fn fonts() -> Option<&'static Fonts> {
    FONTS.get_or_init(load_fonts).as_ref()
}

fn try_font_named(paths: &[(PathBuf, &str)]) -> Option<(Font, String)> {
    for (p, name) in paths {
        if let Ok(bytes) = std::fs::read(p) {
            if let Ok(f) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                return Some((f, (*name).into()));
            }
        }
    }
    None
}

fn win(name: &str) -> (PathBuf, &str) {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    (PathBuf::from(windir).join("Fonts").join(name), name)
}

fn load_fonts() -> Option<Fonts> {
    // Prefer clean literary faces (avoid anything that looks "pixel")
    let title_cands = [
        win("constanb.ttf"),
        win("BOOKOSB.TTF"),
        win("georgiab.ttf"),
        win("cambriab.ttf"),
        win("timesbd.ttf"),
        win("georgia.ttf"),
        win("constan.ttf"),
        (
            PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"),
            "DejaVuSerif-Bold",
        ),
    ];
    // Sharp UI: Segoe UI is the cleanest Windows UI face
    let ui_cands = [
        win("segoeui.ttf"),
        win("calibri.ttf"),
        win("constan.ttf"),
        win("georgia.ttf"),
        win("arial.ttf"),
        win("times.ttf"),
        (
            PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            "DejaVuSans",
        ),
    ];

    let (title, title_name) = try_font_named(&title_cands)?;
    let (ui, ui_name) = try_font_named(&ui_cands).unwrap_or_else(|| {
        let (f, n) = try_font_named(&title_cands).expect("title ok");
        (f, n)
    });
    eprintln!("novel menu fonts: title={title_name}  ui={ui_name}");
    Some(Fonts {
        title,
        ui,
        title_name,
        ui_name,
    })
}

fn measure(font: &Font, text: &str, px: f32) -> f32 {
    text.chars()
        .map(|c| font.rasterize(c, px).0.advance_width)
        .sum()
}

/// Rasterize at 2× and box-filter down — much sharper than single-pass small sizes.
fn draw_font_hq(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    font: &Font,
    x: f32,
    baseline: f32,
    text: &str,
    px: f32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let op = opacity.clamp(0.0, 1.0);
    let src = pack_rgb(rgb.0, rgb.1, rgb.2);
    let scale = 2.0f32;
    let px_hi = px * scale;
    let mut pen = 0.0f32;

    for ch in text.chars() {
        let (m, bmp) = font.rasterize(ch, px_hi);
        if m.width == 0 || m.height == 0 {
            pen += m.advance_width / scale;
            continue;
        }
        // Destination origin (float)
        let ox = x + pen + m.xmin as f32 / scale;
        let oy = baseline - (m.height as f32 + m.ymin as f32) / scale;
        let dw = (m.width as f32 / scale).ceil() as i32 + 1;
        let dh = (m.height as f32 / scale).ceil() as i32 + 1;

        for row in 0..dh {
            for col in 0..dw {
                // 2×2 sample from hi-res coverage
                let mut acc = 0.0f32;
                let mut n = 0.0f32;
                for sy in 0..2 {
                    for sx in 0..2 {
                        let hx = (col as f32 * scale + sx as f32 + 0.5) as i32;
                        let hy = (row as f32 * scale + sy as f32 + 0.5) as i32;
                        if hx >= 0 && hy >= 0 && hx < m.width as i32 && hy < m.height as i32 {
                            acc += bmp[hy as usize * m.width + hx as usize] as f32 / 255.0;
                            n += 1.0;
                        }
                    }
                }
                if n < 1.0 {
                    continue;
                }
                let raw = acc / n;
                // light smoothstep keeps AA but avoids chalky edges
                let t = raw * raw * (3.0 - 2.0 * raw);
                let cov = t * op;
                if cov < 0.02 {
                    continue;
                }
                let px_ = (ox + col as f32).round() as i32;
                let py_ = (oy + row as f32).round() as i32;
                if px_ < 0 || py_ < 0 || px_ >= ww as i32 || py_ >= wh as i32 {
                    continue;
                }
                let i = (py_ as u32 * ww + px_ as u32) as usize;
                pixels[i] = blend(pixels[i], src, cov);
            }
        }
        pen += m.advance_width / scale;
    }
}

/// Paint novel menu at default [`WW`]×[`WH`].
pub fn paint_novel_menu(pixels: &mut [u32], bg: Option<&RgbImage>, sel: usize) {
    paint_novel_menu_size(pixels, WW, WH, bg, sel);
}

/// Paint at arbitrary resolution (scales layout from 1280×720 design space).
pub fn paint_novel_menu_size(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    bg: Option<&RgbImage>,
    sel: usize,
) {
    assert!(pixels.len() >= (ww * wh) as usize);
    // Map design 1280×720 → framebuffer (scale by width; letterbox if aspect differs)
    let s = ww as f32 / DESIGN_W as f32;
    let ox = 0.0f32;
    let oy = ((wh as f32 - DESIGN_H as f32 * s) * 0.5).max(0.0);

    if let Some(b) = bg {
        blit_cover(pixels, ww, wh, b);
    } else {
        fill(pixels, ww, wh, (12, 8, 22));
    }

    vignette_bottom(pixels, ww, wh, 0.78);
    vignette_left(pixels, ww, wh, (520.0 * s) as u32, 0.45);

    soft_hline(
        pixels,
        ww,
        wh,
        ox + 48.0 * s,
        ox + (DESIGN_W as f32 - 48.0) * s,
        oy + 32.0 * s,
        (210, 180, 130),
        0.4,
    );

    let Some(f) = fonts() else {
        return;
    };

    // Title
    let title_x = ox + 80.0 * s;
    let base1 = oy + 175.0 * s;
    let px1 = 48.0 * s;
    let px2 = 86.0 * s;

    draw_font_hq(
        pixels, ww, wh, &f.title, title_x + 2.0 * s, base1 + 2.0 * s, "Luz de", px1, (12, 8, 20),
        0.5,
    );
    draw_font_hq(
        pixels, ww, wh, &f.title, title_x, base1, "Luz de", px1, (236, 224, 210), 1.0,
    );

    let base2 = base1 + px2 * 0.92;
    draw_font_hq(
        pixels, ww, wh, &f.title, title_x + 2.5 * s, base2 + 2.5 * s, "Estación", px2, (16, 10, 22),
        0.55,
    );
    draw_font_hq(
        pixels, ww, wh, &f.title, title_x, base2, "Estación", px2, (255, 214, 150), 1.0,
    );
    draw_font_hq(
        pixels,
        ww,
        wh,
        &f.title,
        title_x - 0.3 * s,
        base2 - 0.4 * s,
        "Estación",
        px2,
        (255, 240, 200),
        0.2,
    );

    draw_font_hq(
        pixels,
        ww,
        wh,
        &f.ui,
        title_x,
        base2 + 40.0 * s,
        "una novela visual",
        20.0 * s,
        (175, 160, 190),
        0.92,
    );

    soft_hline(
        pixels,
        ww,
        wh,
        title_x,
        title_x + 200.0 * s,
        base2 + 54.0 * s,
        (200, 160, 110),
        0.45,
    );

    // Menu items — novel style: soft pill only when selected, text always HQ
    let mx = ox + 80.0 * s;
    let my0 = oy + 400.0 * s;
    let row_h = 52.0 * s;
    let btn_w = 380.0 * s;
    let label_px = 28.0 * s;

    for (i, label) in MENU_ITEMS.iter().enumerate() {
        let y = my0 + i as f32 * row_h;
        let enabled = menu_enabled(i);
        let selected = i == sel && enabled;
        let baseline = y + 32.0 * s;

        if selected {
            // Layered soft glass (no hard square corners)
            rounded_panel(
                pixels,
                ww,
                wh,
                mx - 14.0 * s,
                y - 2.0 * s,
                btn_w + 28.0 * s,
                44.0 * s,
                14.0 * s,
                (30, 14, 40),
                0.78,
            );
            rounded_panel(
                pixels,
                ww,
                wh,
                mx - 14.0 * s,
                y - 2.0 * s,
                btn_w + 28.0 * s,
                44.0 * s,
                14.0 * s,
                (255, 150, 175),
                0.12,
            );
            // Soft gold edge ring
            rounded_panel(
                pixels,
                ww,
                wh,
                mx - 14.0 * s,
                y - 2.0 * s,
                btn_w + 28.0 * s,
                44.0 * s,
                14.0 * s,
                (255, 200, 150),
                0.08,
            );
            draw_font_hq(
                pixels,
                ww,
                wh,
                &f.ui,
                mx,
                baseline,
                "›",
                label_px,
                (255, 210, 170),
                1.0,
            );
            draw_font_hq(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 26.0 * s,
                baseline + 1.0 * s,
                label,
                label_px,
                (30, 16, 36),
                0.4,
            );
            draw_font_hq(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 26.0 * s,
                baseline,
                label,
                label_px,
                (255, 240, 220),
                1.0,
            );
        } else if enabled {
            draw_font_hq(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 10.0 * s,
                baseline,
                "·",
                label_px * 0.9,
                (170, 150, 185),
                0.75,
            );
            draw_font_hq(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 30.0 * s,
                baseline,
                label,
                label_px,
                (220, 210, 230),
                0.95,
            );
        } else {
            draw_font_hq(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 30.0 * s,
                baseline,
                label,
                label_px,
                (125, 115, 135),
                0.5,
            );
        }
    }

    // Footer
    draw_font_hq(
        pixels,
        ww,
        wh,
        &f.ui,
        ox + 80.0 * s,
        oy + (DESIGN_H as f32 - 34.0) * s,
        "↑ ↓  seleccionar      Enter  confirmar      Esc  salir",
        16.0 * s,
        (155, 145, 170),
        0.88,
    );
    let brand = "Velvet Engine";
    let bw = measure(&f.ui, brand, 15.0 * s);
    draw_font_hq(
        pixels,
        ww,
        wh,
        &f.ui,
        ww as f32 - 48.0 * s - bw,
        oy + (DESIGN_H as f32 - 34.0) * s,
        brand,
        15.0 * s,
        (115, 105, 130),
        0.8,
    );

    soft_hline(
        pixels,
        ww,
        wh,
        ox + 18.0 * s,
        ox + (DESIGN_W as f32 - 18.0) * s,
        oy + 14.0 * s,
        (190, 160, 110),
        0.35,
    );
    soft_hline(
        pixels,
        ww,
        wh,
        ox + 18.0 * s,
        ox + (DESIGN_W as f32 - 18.0) * s,
        oy + (DESIGN_H as f32 - 16.0) * s,
        (190, 160, 110),
        0.35,
    );
}

/// Move selection to next enabled entry (dir = ±1).
pub fn move_sel(sel: usize, dir: i32) -> usize {
    let n = MENU_ITEMS.len() as i32;
    let mut s = sel as i32;
    for _ in 0..n {
        s = (s + dir).rem_euclid(n);
        if menu_enabled(s as usize) {
            return s as usize;
        }
    }
    sel
}

/// Bilinear letterbox (for any leftover scale — never nearest-neighbor).
pub fn letterbox_bilinear(src: &[u32], sw: u32, sh: u32, dw: u32, dh: u32, void: u32) -> Vec<u32> {
    let mut out = vec![void; (dw * dh) as usize];
    if sw == 0 || sh == 0 {
        return out;
    }
    let scale = (dw as f32 / sw as f32).min(dh as f32 / sh as f32);
    let tw = ((sw as f32 * scale).round() as u32).max(1).min(dw);
    let th = ((sh as f32 * scale).round() as u32).max(1).min(dh);
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    for y in 0..th {
        let v = (y as f32 + 0.5) * sh as f32 / th as f32 - 0.5;
        for x in 0..tw {
            let u = (x as f32 + 0.5) * sw as f32 / tw as f32 - 0.5;
            out[((oy + y) * dw + (ox + x)) as usize] = sample_rgb_bilinear(src, sw, sh, u, v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_ui() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui")
    }

    #[test]
    fn fonts_load_real_ttf() {
        let (t, u) = font_status().expect("fonts");
        assert!(!t.is_empty() && !u.is_empty());
        // Must not be empty / fake
        assert!(t.ends_with(".ttf") || t.ends_with(".TTF") || t.contains("DejaVu"));
    }

    #[test]
    fn novel_menu_paints_filled_frame() {
        let bg = load_rgb(&data_ui().join("menu_bg.jpg"));
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_novel_menu(&mut pixels, bg.as_ref(), 0);
        let non_black = pixels
            .iter()
            .filter(|&&p| {
                let r = ((p >> 16) & 0xFF) as u32;
                let g = ((p >> 8) & 0xFF) as u32;
                let b = (p & 0xFF) as u32;
                r + g + b > 40
            })
            .count();
        assert!(non_black as f32 / pixels.len() as f32 > 0.4);
    }

    #[test]
    fn novel_menu_selection_changes_pixels() {
        let bg = load_rgb(&data_ui().join("menu_bg.jpg"));
        let mut a = vec![0u32; (WW * WH) as usize];
        let mut b = vec![0u32; (WW * WH) as usize];
        paint_novel_menu(&mut a, bg.as_ref(), 0);
        paint_novel_menu(&mut b, bg.as_ref(), 4);
        assert_ne!(a, b);
    }

    #[test]
    fn move_sel_skips_disabled() {
        assert_eq!(move_sel(0, 1), 4);
        assert_eq!(move_sel(4, -1), 0);
    }

    #[test]
    fn dump_novel_menu_png() {
        let bg = load_rgb(&data_ui().join("menu_bg.jpg"));
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_novel_menu(&mut pixels, bg.as_ref(), 0);
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/novel_menu.png");
        let mut rgba = Vec::with_capacity(pixels.len() * 4);
        for &p in &pixels {
            rgba.push(((p >> 16) & 0xFF) as u8);
            rgba.push(((p >> 8) & 0xFF) as u8);
            rgba.push((p & 0xFF) as u8);
            rgba.push(255);
        }
        image::save_buffer(&out, &rgba, WW, WH, image::ColorType::Rgba8).expect("png");
        assert!(out.exists());
        assert_eq!((WW, WH), (1920, 1080));
    }

    #[test]
    fn default_buffer_is_full_hd() {
        assert_eq!(WW, 1920);
        assert_eq!(WH, 1080);
        assert_eq!(DESIGN_W, 1280);
        assert_eq!(DESIGN_H, 720);
    }

    #[test]
    fn compose_size_matches_window_until_cap() {
        assert_eq!(compose_size_for_window(1280, 720), (1280, 720));
        assert_eq!(compose_size_for_window(1920, 1080), (1920, 1080));
        let (w, h) = compose_size_for_window(3840, 2160);
        assert_eq!(w.max(h), MAX_COMPOSE_EDGE);
        assert!((w as f32 / h as f32 - 16.0 / 9.0).abs() < 0.02);
    }

    #[test]
    fn paint_at_window_size_works() {
        let (w, h) = (960u32, 540u32);
        let mut pixels = vec![0u32; (w * h) as usize];
        paint_novel_menu_size(&mut pixels, w, h, None, 0);
        assert!(pixels.iter().any(|&p| p != 0));
    }
}
