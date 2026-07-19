//! Title menu for **Luz de Estación** — literary visual-novel lobby.
//!
//! Atmospheric background + serif title + soft selection list.
//! Pure paint (no winit) so tests can dump the frame.

use std::path::Path;
use std::sync::OnceLock;

use fontdue::Font;
use velvet_story::{draw_text_line, pack_rgb};

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
    matches!(i, 0 | 4) // Nueva partida + Salir (others stub for now)
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

/// Cover-blit background.
pub fn blit_cover(pixels: &mut [u32], ww: u32, wh: u32, img: &RgbImage) {
    let (sw, sh, src) = img;
    if *sw == 0 || *sh == 0 {
        return;
    }
    for y in 0..wh {
        let sy = y * *sh / wh;
        for x in 0..ww {
            let sx = x * *sw / ww;
            pixels[(y * ww + x) as usize] = src[(sy * *sw + sx) as usize];
        }
    }
}

fn fill(pixels: &mut [u32], ww: u32, wh: u32, rgb: (u8, u8, u8)) {
    let c = pack_rgb(rgb.0, rgb.1, rgb.2);
    for p in pixels.iter_mut().take((ww * wh) as usize) {
        *p = c;
    }
}

fn panel(pixels: &mut [u32], ww: u32, wh: u32, x: i32, y: i32, w: i32, h: i32, rgb: (u8, u8, u8), a: f32) {
    let a = a.clamp(0.0, 1.0);
    let base = pack_rgb(rgb.0, rgb.1, rgb.2);
    for row in y.max(0)..(y + h).min(wh as i32) {
        for col in x.max(0)..(x + w).min(ww as i32) {
            let i = (row as u32 * ww + col as u32) as usize;
            pixels[i] = blend(pixels[i], base, a);
        }
    }
}

fn outline(pixels: &mut [u32], ww: u32, wh: u32, x: i32, y: i32, w: i32, h: i32, rgb: (u8, u8, u8), t: i32) {
    let t = t.max(1);
    panel(pixels, ww, wh, x, y, w, t, rgb, 0.95);
    panel(pixels, ww, wh, x, y + h - t, w, t, rgb, 0.95);
    panel(pixels, ww, wh, x, y, t, h, rgb, 0.95);
    panel(pixels, ww, wh, x + w - t, y, t, h, rgb, 0.95);
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

fn vignette_bottom(pixels: &mut [u32], ww: u32, wh: u32, strength: f32) {
    let band = (wh as f32 * 0.55) as u32;
    for y in 0..band {
        let t = 1.0 - y as f32 / band as f32;
        let a = t * t * strength;
        let py = wh - 1 - y;
        for x in 0..ww {
            let i = (py * ww + x) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(6, 4, 12), a);
        }
    }
}

fn vignette_left(pixels: &mut [u32], ww: u32, wh: u32, width: u32, strength: f32) {
    for x in 0..width.min(ww) {
        let a = (1.0 - x as f32 / width as f32) * strength;
        for y in 0..wh {
            let i = (y * ww + x) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(6, 4, 12), a * 0.85);
        }
    }
}

// ── Title font ────────────────────────────────────────────────────────────

static TITLE_FONT: OnceLock<Option<Font>> = OnceLock::new();

fn title_font() -> Option<&'static Font> {
    TITLE_FONT.get_or_init(load_serif).as_ref()
}

fn load_serif() -> Option<Font> {
    let mut paths = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        let f = Path::new(&windir).join("Fonts");
        for n in ["georgiab.ttf", "georgia.ttf", "timesbd.ttf", "constanb.ttf"] {
            paths.push(f.join(n));
        }
    }
    paths.push(Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf").to_path_buf());
    for p in paths {
        if let Ok(bytes) = std::fs::read(&p) {
            if let Ok(font) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                return Some(font);
            }
        }
    }
    None
}

fn measure(font: &Font, text: &str, px: f32) -> f32 {
    text.chars()
        .map(|c| font.rasterize(c, px).0.advance_width)
        .sum()
}

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
                let cov = bmp[row * m.width + col] as f32 / 255.0 * op;
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
        pen += m.advance_width;
    }
}

/// Full novel title menu paint.
pub fn paint_novel_menu(
    pixels: &mut [u32],
    bg: Option<&RgbImage>,
    sel: usize,
) {
    let ww = WW;
    let wh = WH;
    if let Some(b) = bg {
        blit_cover(pixels, ww, wh, b);
    } else {
        fill(pixels, ww, wh, (12, 8, 22));
    }

    vignette_bottom(pixels, ww, wh, 0.72);
    vignette_left(pixels, ww, wh, 520, 0.38);

    // Top thin gold line
    panel(pixels, ww, wh, 40, 28, ww as i32 - 80, 1, (200, 170, 110), 0.55);

    // Title block — lower-left literary placement
    let title_x = 72.0;
    let title_y0 = 160.0;
    if let Some(font) = title_font() {
        let line1 = "Luz de";
        let line2 = "Estación";
        let px1 = 52.0;
        let px2 = 78.0;
        // soft shadow
        draw_font(pixels, ww, wh, font, title_x + 2.0, title_y0 + 2.0, line1, px1, (20, 12, 30), 0.5);
        draw_font(pixels, ww, wh, font, title_x, title_y0, line1, px1, (230, 210, 190), 1.0);
        let base2 = title_y0 + px2 * 0.95;
        draw_font(pixels, ww, wh, font, title_x + 3.0, base2 + 3.0, line2, px2, (20, 12, 30), 0.55);
        draw_font(pixels, ww, wh, font, title_x, base2, line2, px2, (255, 210, 140), 1.0);

        // tagline
        let tag = "una novela visual";
        let tpx = 22.0;
        let tw = measure(font, tag, tpx);
        draw_font(
            pixels,
            ww,
            wh,
            font,
            title_x,
            base2 + 36.0,
            tag,
            tpx,
            (180, 165, 195),
            0.9,
        );
        let _ = tw;
    } else {
        draw_text_line(pixels, ww, wh, 72, 160, "Luz de Estacion", pack_rgb(255, 210, 140), 4); // ASCII fallback
        draw_text_line(pixels, ww, wh, 72, 220, "una novela visual", pack_rgb(180, 165, 195), 2);
    }

    // Menu column
    let mx = 72i32;
    let my0 = 380i32;
    let mw = 360i32;
    let mh = 42i32;
    let gap = 10i32;

    for (i, label) in MENU_ITEMS.iter().enumerate() {
        let y = my0 + i as i32 * (mh + gap);
        let enabled = menu_enabled(i);
        let selected = i == sel && enabled;
        if selected {
            panel(pixels, ww, wh, mx - 4, y - 2, mw + 8, mh + 4, (255, 120, 160), 0.18);
            panel(pixels, ww, wh, mx, y, mw, mh, (40, 18, 48), 0.82);
            outline(pixels, ww, wh, mx, y, mw, mh, (255, 170, 120), 1);
            // accent bar
            panel(pixels, ww, wh, mx + 4, y + 8, 4, mh - 16, (255, 140, 170), 0.95);
            draw_text_line(
                pixels,
                ww,
                wh,
                mx + 22,
                y + 14,
                label,
                pack_rgb(255, 230, 200),
                2,
            );
        } else if enabled {
            panel(pixels, ww, wh, mx, y, mw, mh, (18, 12, 28), 0.55);
            outline(pixels, ww, wh, mx, y, mw, mh, (120, 100, 140), 1);
            draw_text_line(
                pixels,
                ww,
                wh,
                mx + 22,
                y + 14,
                label,
                pack_rgb(210, 200, 220),
                2,
            );
        } else {
            panel(pixels, ww, wh, mx, y, mw, mh, (14, 10, 20), 0.4);
            outline(pixels, ww, wh, mx, y, mw, mh, (60, 50, 70), 1);
            draw_text_line(
                pixels,
                ww,
                wh,
                mx + 22,
                y + 14,
                &format!("{label}  ·"),
                pack_rgb(110, 100, 120),
                2,
            );
        }
    }

    // Bottom hint
    draw_text_line(
        pixels,
        ww,
        wh,
        72,
        wh as i32 - 36,
        "Arriba/Abajo  Enter  Esc",
        pack_rgb(140, 130, 155),
        1,
    );
    // Version / brand whisper
    draw_text_line(
        pixels,
        ww,
        wh,
        ww as i32 - 200,
        wh as i32 - 36,
        "Velvet Engine",
        pack_rgb(100, 90, 120),
        1,
    );

    // Frame
    outline(pixels, ww, wh, 10, 10, ww as i32 - 20, wh as i32 - 20, (180, 150, 100), 1);
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
    use std::path::PathBuf;

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
        // from Nueva partida (0), down should land on Salir (4) skipping stubs
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
    }
}
