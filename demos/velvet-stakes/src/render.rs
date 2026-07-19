//! Softbuffer helpers: blit ARGB art + text.

use std::collections::HashMap;
use std::path::Path;

use velvet_story::{draw_text_line, pack_rgb};

/// Image buffer: width, height, packed RGB.
pub type RgbImage = (u32, u32, Vec<u32>);

/// Image with per-pixel alpha 0..=255 (for keyed logos).
pub type RgbaImage = (u32, u32, Vec<u32>, Vec<u8>);

/// Preloaded card art as ARGB u32 rows.
pub struct ArtBank {
    pub images: HashMap<String, RgbImage>,
}

impl ArtBank {
    pub fn load(stats_art: &[(String, std::path::PathBuf)]) -> Self {
        let mut images = HashMap::new();
        for (id, path) in stats_art {
            if let Some(img) = load_rgb(path) {
                images.insert(id.clone(), img);
            }
        }
        Self { images }
    }

    pub fn from_catalog_dir(art_dir: &Path, ids: &[&str]) -> Self {
        let pairs: Vec<_> = ids
            .iter()
            .map(|id| (id.to_string(), art_dir.join(format!("{id}.jpg"))))
            .collect();
        Self::load(&pairs)
    }
}

/// Load any image path to packed RGB.
pub fn load_rgb(path: &Path) -> Option<RgbImage> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut px = Vec::with_capacity((w * h) as usize);
    for p in rgba.pixels() {
        let [r, g, b, _a] = p.0;
        px.push(pack_rgb(r, g, b));
    }
    Some((w, h, px))
}

/// Load RGBA (preserves alpha for transparent logos).
pub fn load_rgba(path: &Path) -> Option<RgbaImage> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((w * h) as usize);
    let mut a = Vec::with_capacity((w * h) as usize);
    for p in rgba.pixels() {
        let [r, g, b, alpha] = p.0;
        rgb.push(pack_rgb(r, g, b));
        a.push(alpha);
    }
    Some((w, h, rgb, a))
}

/// Load RGB and **burn pure/near-black to alpha** (for black-bg wordmarks).
///
/// Dark pixels become transparent; copper/gold lettering stays opaque.
pub fn load_rgb_key_black(path: &Path, black_cut: u8) -> Option<RgbaImage> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((w * h) as usize);
    let mut a = Vec::with_capacity((w * h) as usize);
    let cut = black_cut as u32;
    for p in rgba.pixels() {
        let [r, g, b, src_a] = p.0;
        let lum = r.max(g).max(b) as u32;
        let alpha = if lum <= cut {
            0u8
        } else if lum < cut + 36 {
            // soft edge so glow keys cleanly
            let t = (lum - cut) as f32 / 36.0;
            ((t * t) * 255.0) as u8
        } else {
            src_a.max(240)
        };
        rgb.push(pack_rgb(r, g, b));
        a.push(alpha);
    }
    Some((w, h, rgb, a))
}

/// Stretch image to full frame (menu background).
pub fn blit_cover(pixels: &mut [u32], ww: u32, wh: u32, img: &RgbImage) {
    blit_card(pixels, ww, wh, img, 0, 0, ww as i32, wh as i32, 1.0);
}

/// Dark translucent panel for menu buttons.
pub fn panel(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgb: (u8, u8, u8),
    alpha: f32,
) {
    let a = alpha.clamp(0.0, 1.0);
    let ww_i = ww as i32;
    let wh_i = wh as i32;
    let base = pack_rgb(rgb.0, rgb.1, rgb.2);
    for row in y.max(0)..(y + h).min(wh_i) {
        for col in x.max(0)..(x + w).min(ww_i) {
            let i = (row as u32 * ww + col as u32) as usize;
            pixels[i] = blend(pixels[i], base, a);
        }
    }
}

/// Thin gold outline around a rect.
pub fn outline(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgb: (u8, u8, u8),
    thickness: i32,
) {
    let t = thickness.max(1);
    rect(pixels, ww, wh, x, y, w, t, rgb);
    rect(pixels, ww, wh, x, y + h - t, w, t, rgb);
    rect(pixels, ww, wh, x, y, t, h, rgb);
    rect(pixels, ww, wh, x + w - t, y, t, h, rgb);
}

pub fn fill(pixels: &mut [u32], ww: u32, wh: u32, rgb: (u8, u8, u8)) {
    let c = pack_rgb(rgb.0, rgb.1, rgb.2);
    let n = (ww * wh) as usize;
    for p in pixels.iter_mut().take(n) {
        *p = c;
    }
}

pub fn rect(pixels: &mut [u32], ww: u32, wh: u32, x: i32, y: i32, w: i32, h: i32, rgb: (u8, u8, u8)) {
    let c = pack_rgb(rgb.0, rgb.1, rgb.2);
    let ww = ww as i32;
    let wh = wh as i32;
    for row in y.max(0)..(y + h).min(wh) {
        for col in x.max(0)..(x + w).min(ww) {
            pixels[(row * ww + col) as usize] = c;
        }
    }
}

pub fn text(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    x: i32,
    y: i32,
    s: &str,
    rgb: (u8, u8, u8),
    scale: i32,
) {
    draw_text_line(
        pixels,
        ww,
        wh,
        x,
        y,
        s,
        pack_rgb(rgb.0, rgb.1, rgb.2),
        scale,
    );
}

/// Nearest-neighbor blit of art into dest rect with opacity 0..1 and optional scale punch.
pub fn blit_card(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    art: &(u32, u32, Vec<u32>),
    dx: i32,
    dy: i32,
    dw: i32,
    dh: i32,
    opacity: f32,
) {
    if dw <= 2 || dh <= 2 || opacity <= 0.01 {
        return;
    }
    let (sw, sh, src) = art;
    let op = opacity.clamp(0.0, 1.0);
    for row in 0..dh {
        let sy = (row as u32 * sh) / dh as u32;
        let dy = dy + row;
        if dy < 0 || dy >= wh as i32 {
            continue;
        }
        for col in 0..dw {
            let sx = (col as u32 * sw) / dw as u32;
            let dx = dx + col;
            if dx < 0 || dx >= ww as i32 {
                continue;
            }
            let sc = src[(sy * sw + sx) as usize];
            let di = (dy as u32 * ww + dx as u32) as usize;
            if op >= 0.99 {
                pixels[di] = sc;
            } else {
                pixels[di] = blend(pixels[di], sc, op);
            }
        }
    }
}

/// Blit RGBA image (alpha 0..=255) with nearest filter — for keyed title logos.
pub fn blit_rgba(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    art: &RgbaImage,
    dx: i32,
    dy: i32,
    dw: i32,
    dh: i32,
    opacity: f32,
) {
    if dw <= 2 || dh <= 2 || opacity <= 0.01 {
        return;
    }
    let (sw, sh, src, alpha) = art;
    let op = opacity.clamp(0.0, 1.0);
    for row in 0..dh {
        let sy = (row as u32 * sh) / dh as u32;
        let py = dy + row;
        if py < 0 || py >= wh as i32 {
            continue;
        }
        for col in 0..dw {
            let sx = (col as u32 * sw) / dw as u32;
            let px = dx + col;
            if px < 0 || px >= ww as i32 {
                continue;
            }
            let si = (sy * sw + sx) as usize;
            let a = (alpha[si] as f32 / 255.0) * op;
            if a < 0.02 {
                continue;
            }
            let di = (py as u32 * ww + px as u32) as usize;
            pixels[di] = blend(pixels[di], src[si], a);
        }
    }
}

fn blend(dst: u32, src: u32, t: f32) -> u32 {
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((src >> 16) & 0xFF) as f32;
    let sg = ((src >> 8) & 0xFF) as f32;
    let sb = (src & 0xFF) as f32;
    let r = (dr + (sr - dr) * t) as u8;
    let g = (dg + (sg - dg) * t) as u8;
    let b = (db + (sb - db) * t) as u8;
    pack_rgb(r, g, b)
}
