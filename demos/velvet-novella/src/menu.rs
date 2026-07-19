//! Title menu for **Luz de Estación** — high-quality literary VN lobby.
//!
//! All text is **fontdue** (TrueType coverage AA) — no softbuffer 8-bit glyphs.
//! Buttons are soft rounded novel chrome, not chunky game rectangles.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use fontdue::Font;
use velvet_story::pack_rgb;

/// Logical frame size for the novel menu.
pub const WW: u32 = 1280;
/// Logical frame height.
pub const WH: u32 = 720;

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

/// Bilinear cover-blit (smooth background scale).
pub fn blit_cover(pixels: &mut [u32], ww: u32, wh: u32, img: &RgbImage) {
    let (sw, sh, src) = img;
    if *sw == 0 || *sh == 0 {
        return;
    }
    for y in 0..wh {
        let v = (y as f32 + 0.5) * (*sh as f32) / wh as f32 - 0.5;
        for x in 0..ww {
            let u = (x as f32 + 0.5) * (*sw as f32) / ww as f32 - 0.5;
            pixels[(y * ww + x) as usize] = sample_bilinear(src, *sw, *sh, u, v);
        }
    }
}

fn sample_bilinear(src: &[u32], sw: u32, sh: u32, x: f32, y: f32) -> u32 {
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
    let c = |xx: i32, yy: i32| src[(yy as u32 * sw + xx as u32) as usize];
    let ch = |c: u32, s: u32| ((c >> s) & 0xFF) as f32;
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let bilerp = |c00: u32, c10: u32, c01: u32, c11: u32, shift: u32| {
        lerp(
            lerp(ch(c00, shift), ch(c10, shift), fx),
            lerp(ch(c01, shift), ch(c11, shift), fx),
            fy,
        )
    };
    let c00 = c(x0, y0);
    let c10 = c(x1, y0);
    let c01 = c(x0, y1);
    let c11 = c(x1, y1);
    pack_rgb(
        bilerp(c00, c10, c01, c11, 16) as u8,
        bilerp(c00, c10, c01, c11, 8) as u8,
        bilerp(c00, c10, c01, c11, 0) as u8,
    )
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
    if x < 0 || y < 0 || x >= ww as i32 || y >= wh as i32 {
        return;
    }
    let i = (y as u32 * ww + x as u32) as usize;
    pixels[i] = blend(pixels[i], pack_rgb(rgb.0, rgb.1, rgb.2), a);
}

/// Soft rounded rectangle (anti-aliased corners — novel chrome, not square pixels).
fn rounded_panel(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    radius: f32,
    rgb: (u8, u8, u8),
    alpha: f32,
) {
    let a = alpha.clamp(0.0, 1.0);
    let r = radius.max(0.0);
    for row in y.max(0)..(y + h).min(wh as i32) {
        for col in x.max(0)..(x + w).min(ww as i32) {
            let px = col as f32 + 0.5;
            let py = row as f32 + 0.5;
            let cx0 = x as f32 + r;
            let cy0 = y as f32 + r;
            let cx1 = (x + w) as f32 - r;
            let cy1 = (y + h) as f32 - r;
            // distance outside rounded rect (0 = inside)
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
            let edge = if r < 0.5 {
                0.0
            } else {
                (dist - r + 0.5).clamp(0.0, 1.0)
            };
            let cov = (1.0 - edge) * a;
            if cov > 0.01 {
                put(pixels, ww, wh, col, row, rgb, cov);
            }
        }
    }
}

fn soft_line_h(pixels: &mut [u32], ww: u32, wh: u32, x0: i32, x1: i32, y: i32, rgb: (u8, u8, u8), a: f32) {
    let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    for x in x0..=x1 {
        put(pixels, ww, wh, x, y, rgb, a);
        put(pixels, ww, wh, x, y + 1, rgb, a * 0.35);
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
        let a = (1.0 - x as f32 / width as f32).powf(1.2) * strength;
        for y in 0..wh {
            let i = (y * ww + x) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(6, 4, 14), a);
        }
    }
}

// ── Fonts (TrueType via fontdue) ───────────────────────────────────────────

struct Fonts {
    /// Display / title (serif).
    title: Font,
    /// Menu & UI (clean sans or second serif).
    ui: Font,
}

static FONTS: OnceLock<Option<Fonts>> = OnceLock::new();

fn fonts() -> Option<&'static Fonts> {
    FONTS.get_or_init(load_fonts).as_ref()
}

fn try_font(paths: &[PathBuf]) -> Option<Font> {
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            if let Ok(f) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                return Some(f);
            }
        }
    }
    None
}

fn win_fonts(names: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        let dir = Path::new(&windir).join("Fonts");
        for n in names {
            out.push(dir.join(n));
        }
    }
    out
}

fn load_fonts() -> Option<Fonts> {
    // Literary display for the novel title
    let mut title_paths = win_fonts(&[
        "constanb.ttf", // Constantia Bold — clean literary
        "BOOKOSB.TTF",  // Book Antiqua Bold
        "georgiab.ttf",
        "cambriab.ttf",
        "timesbd.ttf",
        "georgia.ttf",
        "constan.ttf",
    ]);
    title_paths.push(PathBuf::from(
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    ));

    // Sharp UI labels (novel menus often use a refined sans for items)
    let mut ui_paths = win_fonts(&[
        "constan.ttf",   // Constantia — elegant body
        "georgia.ttf",
        "calibri.ttf",
        "segoeui.ttf",
        "arial.ttf",
        "times.ttf",
    ]);
    ui_paths.push(PathBuf::from(
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    ));

    let title = try_font(&title_paths)?;
    let ui = try_font(&ui_paths).unwrap_or_else(|| {
        // same as title if no separate UI face
        try_font(&title_paths).expect("title already loaded")
    });
    Some(Fonts { title, ui })
}

fn measure(font: &Font, text: &str, px: f32) -> f32 {
    text.chars()
        .map(|c| font.rasterize(c, px).0.advance_width)
        .sum()
}

/// High-quality glyph blit with coverage AA (no nearest-neighbor block glyphs).
fn draw_font(
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
    let mut pen = x;
    for ch in text.chars() {
        let (m, bmp) = font.rasterize(ch, px);
        let ox = pen + m.xmin as f32;
        let oy = baseline - m.height as f32 - m.ymin as f32;
        for row in 0..m.height {
            for col in 0..m.width {
                // smoothstep coverage for slightly softer (less “hard pixel”) edges
                let raw = bmp[row * m.width + col] as f32 / 255.0;
                let t = raw * raw * (3.0 - 2.0 * raw);
                let cov = t * op;
                if cov < 0.015 {
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
        pen += m.advance_width;
    }
}

/// Full novel title menu paint — quality first.
pub fn paint_novel_menu(pixels: &mut [u32], bg: Option<&RgbImage>, sel: usize) {
    let ww = WW;
    let wh = WH;
    if let Some(b) = bg {
        blit_cover(pixels, ww, wh, b);
    } else {
        fill(pixels, ww, wh, (12, 8, 22));
    }

    vignette_bottom(pixels, ww, wh, 0.75);
    vignette_left(pixels, ww, wh, 560, 0.42);

    // Subtle top rule
    soft_line_h(pixels, ww, wh, 48, ww as i32 - 48, 32, (210, 180, 130), 0.4);

    let Some(f) = fonts() else {
        // Extremely rare: no TTF at all — solid fallback without 8-bit font spam
        rounded_panel(pixels, ww, wh, 60, 140, 400, 80, 8.0, (20, 12, 30), 0.7);
        return;
    };

    // ── Title ────────────────────────────────────────────────────────────
    let title_x = 80.0;
    let base1 = 175.0;
    let px1 = 48.0;
    let px2 = 84.0;
    let line1 = "Luz de";
    let line2 = "Estación";

    draw_font(
        pixels, ww, wh, &f.title, title_x + 2.5, base1 + 2.5, line1, px1, (12, 8, 20), 0.55,
    );
    draw_font(
        pixels, ww, wh, &f.title, title_x, base1, line1, px1, (236, 224, 210), 1.0,
    );

    let base2 = base1 + px2 * 0.92;
    draw_font(
        pixels, ww, wh, &f.title, title_x + 3.0, base2 + 3.0, line2, px2, (18, 10, 24), 0.6,
    );
    // warm gold face
    draw_font(
        pixels, ww, wh, &f.title, title_x, base2, line2, px2, (255, 214, 150), 1.0,
    );
    // soft highlight
    draw_font(
        pixels,
        ww,
        wh,
        &f.title,
        title_x - 0.4,
        base2 - 0.5,
        line2,
        px2,
        (255, 240, 200),
        0.22,
    );

    let tag = "una novela visual";
    let tpx = 20.0;
    draw_font(
        pixels,
        ww,
        wh,
        &f.ui,
        title_x + 2.0,
        base2 + 38.0,
        tag,
        tpx,
        (175, 160, 190),
        0.92,
    );

    // Decorative rule under tagline
    let rule_y = (base2 + 52.0) as i32;
    soft_line_h(pixels, ww, wh, title_x as i32, title_x as i32 + 180, rule_y, (200, 160, 110), 0.45);
    // tiny diamond
    put(pixels, ww, wh, title_x as i32 + 190, rule_y, (230, 190, 130), 0.9);
    put(pixels, ww, wh, title_x as i32 + 189, rule_y + 1, (230, 190, 130), 0.5);
    put(pixels, ww, wh, title_x as i32 + 191, rule_y + 1, (230, 190, 130), 0.5);

    // ── Novel buttons (text-forward, soft glass) ──────────────────────────
    let mx = 80.0;
    let my0 = 400.0;
    let row_h = 48.0;
    let btn_w = 340.0;
    let label_px = 26.0;

    for (i, label) in MENU_ITEMS.iter().enumerate() {
        let y = my0 + i as f32 * row_h;
        let enabled = menu_enabled(i);
        let selected = i == sel && enabled;
        let baseline = y + 30.0;

        if selected {
            // Soft rose-gold glass plate
            rounded_panel(
                pixels,
                ww,
                wh,
                mx as i32 - 10,
                y as i32 - 4,
                btn_w as i32 + 20,
                40,
                12.0,
                (48, 22, 52),
                0.72,
            );
            rounded_panel(
                pixels,
                ww,
                wh,
                mx as i32 - 10,
                y as i32 - 4,
                btn_w as i32 + 20,
                40,
                12.0,
                (255, 140, 170),
                0.10,
            );
            // Left accent stroke (soft)
            for dy in 6..34 {
                put(
                    pixels,
                    ww,
                    wh,
                    mx as i32 - 4,
                    y as i32 + dy,
                    (255, 180, 140),
                    0.85,
                );
                put(
                    pixels,
                    ww,
                    wh,
                    mx as i32 - 3,
                    y as i32 + dy,
                    (255, 200, 160),
                    0.35,
                );
            }
            // Marker
            draw_font(
                pixels,
                ww,
                wh,
                &f.ui,
                mx,
                baseline,
                "›",
                label_px,
                (255, 200, 160),
                1.0,
            );
            draw_font(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 22.0,
                baseline + 1.0,
                label,
                label_px,
                (40, 20, 40),
                0.35,
            );
            draw_font(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 22.0,
                baseline,
                label,
                label_px,
                (255, 236, 210),
                1.0,
            );
        } else if enabled {
            // Bare literary row — no heavy box
            draw_font(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 8.0,
                baseline,
                "·",
                label_px * 0.85,
                (160, 140, 170),
                0.7,
            );
            draw_font(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 28.0,
                baseline,
                label,
                label_px,
                (215, 205, 225),
                0.95,
            );
        } else {
            // Disabled: softer, dim
            let dim = format!("{label}");
            draw_font(
                pixels,
                ww,
                wh,
                &f.ui,
                mx + 28.0,
                baseline,
                &dim,
                label_px,
                (120, 110, 130),
                0.55,
            );
        }
    }

    // Footer — UI font, small
    let foot = "↑ ↓  seleccionar     Enter  confirmar     Esc  salir";
    draw_font(
        pixels,
        ww,
        wh,
        &f.ui,
        80.0,
        wh as f32 - 32.0,
        foot,
        15.0,
        (150, 140, 165),
        0.85,
    );
    let brand = "Velvet Engine";
    let bw = measure(&f.ui, brand, 14.0);
    draw_font(
        pixels,
        ww,
        wh,
        &f.ui,
        ww as f32 - 48.0 - bw,
        wh as f32 - 32.0,
        brand,
        14.0,
        (110, 100, 125),
        0.75,
    );

    // Thin outer frame (soft corners via short inset lines)
    soft_line_h(pixels, ww, wh, 18, ww as i32 - 18, 14, (190, 160, 110), 0.35);
    soft_line_h(
        pixels,
        ww,
        wh,
        18,
        ww as i32 - 18,
        wh as i32 - 16,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn data_ui() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui")
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
        let frac = non_black as f32 / pixels.len() as f32;
        assert!(frac > 0.4, "menu should be substantially filled, frac={frac}");
    }

    #[test]
    fn novel_menu_uses_real_fonts_not_bitmap() {
        assert!(fonts().is_some(), "system TTF fonts required for novel menu quality");
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_novel_menu(&mut pixels, None, 0);
        // Gold / cream title pixels present
        let mut warm = 0usize;
        for y in 120..280 {
            for x in 60..500 {
                let p = pixels[(y * WW + x) as usize];
                let r = ((p >> 16) & 0xFF) as u32;
                let g = ((p >> 8) & 0xFF) as u32;
                let b = (p & 0xFF) as u32;
                if r > 180 && g > 140 && r > b {
                    warm += 1;
                }
            }
        }
        assert!(warm > 300, "serif title should paint warm AA pixels, got {warm}");
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
        assert!(std::fs::metadata(&out).unwrap().len() > 20_000);
    }
}
