//! Title wordmark load/key/blit — shipped path used by paint + live-dev.
//!
//! Black-background art is burned to soft alpha; scale uses bilinear filter
//! so serifs are not square stairs.

use std::path::Path;

use velvet_story::pack_rgb;

/// RGBA buffer: (w, h, packed_rgb, alpha 0..=255).
pub type RgbaBuf = (u32, u32, Vec<u32>, Vec<u8>);

/// Load title wordmark from disk.
///
/// - PNG/WebP with alpha: keep alpha, then soft-feather.
/// - JPG / opaque: burn near-black to soft alpha.
pub fn load_title_wordmark(path: &Path) -> Option<RgbaBuf> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut has_real_alpha = false;
    let mut rgb = Vec::with_capacity((w * h) as usize);
    let mut a = Vec::with_capacity((w * h) as usize);
    for p in rgba.pixels() {
        let [r, g, b, alpha] = p.0;
        if alpha < 250 {
            has_real_alpha = true;
        }
        rgb.push(pack_rgb(r, g, b));
        a.push(alpha);
    }
    if !has_real_alpha {
        // Opaque file (typical JPG black plate) — key by luminance
        a = key_black_soft(&rgb, w, h, 14, 48);
    }
    feather_alpha(&mut a, w as usize, h as usize, 1);
    Some((w, h, rgb, a))
}

/// Soft black-key: pure black → 0, copper glow → soft ramp (not hard 0/255).
pub fn key_black_soft(rgb: &[u32], _w: u32, _h: u32, cut: u8, soft: u8) -> Vec<u8> {
    let cut = cut as f32;
    let soft = soft.max(1) as f32;
    rgb.iter()
        .map(|c| {
            let r = ((c >> 16) & 0xFF) as f32;
            let g = ((c >> 8) & 0xFF) as f32;
            let b = (c & 0xFF) as f32;
            let lum = r.max(g).max(b);
            if lum <= cut {
                0
            } else if lum < cut + soft {
                // smoothstep for softer letter edges
                let t = ((lum - cut) / soft).clamp(0.0, 1.0);
                let s = t * t * (3.0 - 2.0 * t);
                (s * 255.0) as u8
            } else {
                255
            }
        })
        .collect()
}

/// Slight alpha feather (max of neighborhood) to kill hard pixel corners.
pub fn feather_alpha(a: &mut [u8], w: usize, h: usize, radius: usize) {
    if radius == 0 || w == 0 || h == 0 {
        return;
    }
    let src = a.to_vec();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut maxv = src[i] as i32;
            let mut sum = 0i32;
            let mut n = 0i32;
            for dy in -(radius as i32)..=radius as i32 {
                for dx in -(radius as i32)..=radius as i32 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let v = src[ny as usize * w + nx as usize] as i32;
                    maxv = maxv.max(v);
                    sum += v;
                    n += 1;
                }
            }
            let avg = if n > 0 { sum / n } else { src[i] as i32 };
            // blend toward neighborhood so hard corners soften without bloating
            let blended = (src[i] as i32 * 2 + avg + maxv) / 4;
            a[i] = blended.clamp(0, 255) as u8;
        }
    }
}

/// Count alpha samples that are soft (1..=254) — used by tests / diagnostics.
pub fn count_soft_alpha(a: &[u8]) -> usize {
    a.iter().filter(|&&v| (1..=254).contains(&v)).count()
}

/// Bilinear sample of RGBA buffer at continuous source coords.
fn sample_bilinear(rgb: &[u32], a: &[u8], w: u32, h: u32, x: f32, y: f32) -> (u32, f32) {
    let w = w as i32;
    let h = h as i32;
    if w <= 0 || h <= 0 {
        return (0, 0.0);
    }
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let idx = |xx: i32, yy: i32| -> usize { (yy * w + xx) as usize };
    let c00 = rgb[idx(x0, y0)];
    let c10 = rgb[idx(x1, y0)];
    let c01 = rgb[idx(x0, y1)];
    let c11 = rgb[idx(x1, y1)];
    let a00 = a[idx(x0, y0)] as f32;
    let a10 = a[idx(x1, y0)] as f32;
    let a01 = a[idx(x0, y1)] as f32;
    let a11 = a[idx(x1, y1)] as f32;

    let ch = |c: u32, shift: u32| -> f32 { ((c >> shift) & 0xFF) as f32 };
    let lerp = |p: f32, q: f32, t: f32| p + (q - p) * t;
    let bilerp = |v00: f32, v10: f32, v01: f32, v11: f32| {
        lerp(lerp(v00, v10, fx), lerp(v01, v11, fx), fy)
    };

    let r = bilerp(ch(c00, 16), ch(c10, 16), ch(c01, 16), ch(c11, 16));
    let g = bilerp(ch(c00, 8), ch(c10, 8), ch(c01, 8), ch(c11, 8));
    let b = bilerp(ch(c00, 0), ch(c10, 0), ch(c01, 0), ch(c11, 0));
    let alpha = bilerp(a00, a10, a01, a11) / 255.0;
    (
        pack_rgb(r.round() as u8, g.round() as u8, b.round() as u8),
        alpha.clamp(0.0, 1.0),
    )
}

/// Blit wordmark with **bilinear** filtering (smooth letter edges when scaled).
pub fn blit_rgba_bilinear(
    pixels: &mut [u32],
    ww: u32,
    wh: u32,
    art: &RgbaBuf,
    dx: i32,
    dy: i32,
    dw: i32,
    dh: i32,
    opacity: f32,
) {
    if dw <= 1 || dh <= 1 || opacity <= 0.01 {
        return;
    }
    let (sw, sh, rgb, alpha) = art;
    if *sw == 0 || *sh == 0 {
        return;
    }
    let op = opacity.clamp(0.0, 1.0);
    for row in 0..dh {
        let py = dy + row;
        if py < 0 || py >= wh as i32 {
            continue;
        }
        // sample at pixel centers
        let sy = (row as f32 + 0.5) * (*sh as f32) / dh as f32 - 0.5;
        for col in 0..dw {
            let px = dx + col;
            if px < 0 || px >= ww as i32 {
                continue;
            }
            let sx = (col as f32 + 0.5) * (*sw as f32) / dw as f32 - 0.5;
            let (sc, sa) = sample_bilinear(rgb, alpha, *sw, *sh, sx, sy);
            let a = sa * op;
            if a < 0.02 {
                continue;
            }
            let di = (py as u32 * ww + px as u32) as usize;
            pixels[di] = blend(pixels[di], sc, a);
        }
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

/// After bilinear scale into a small buffer, count soft alphas (edge quality probe).
pub fn probe_scaled_soft_alpha(art: &RgbaBuf, dw: i32, dh: i32) -> usize {
    let (sw, sh, rgb, alpha) = art;
    let mut soft = 0usize;
    for row in 0..dh {
        let sy = (row as f32 + 0.5) * (*sh as f32) / dh as f32 - 0.5;
        for col in 0..dw {
            let sx = (col as f32 + 0.5) * (*sw as f32) / dw as f32 - 0.5;
            let (_c, a) = sample_bilinear(rgb, alpha, *sw, *sh, sx, sy);
            let u = (a * 255.0) as u8;
            if (1..=254).contains(&u) {
                soft += 1;
            }
        }
    }
    soft
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn temp_png_black_plate(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "velvet_logo_{}_{}_{}",
            std::process::id(),
            tag,
            n
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("word.png");
        // 32x16 black plate with a soft copper disc (letter-like blob)
        let mut img = image::RgbaImage::new(32, 16);
        for y in 0..16u32 {
            for x in 0..32u32 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        let cx = 16.0f32;
        let cy = 8.0f32;
        for y in 0..16u32 {
            for x in 0..32u32 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let d = (dx * dx + dy * dy).sqrt();
                if d < 6.0 {
                    let t = (1.0 - d / 6.0).clamp(0.0, 1.0);
                    let r = (80.0 + 140.0 * t) as u8;
                    let g = (40.0 + 80.0 * t) as u8;
                    let b = (20.0 + 40.0 * t) as u8;
                    img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
        }
        img.save(&path).expect("write temp wordmark png");
        path
    }

    #[test]
    fn key_produces_soft_alpha_not_only_binary() {
        let path = temp_png_black_plate("key");
        let logo = load_title_wordmark(&path).expect("load");
        let soft = count_soft_alpha(&logo.3);
        assert!(
            soft > 8,
            "expected soft alpha fringe from burn, got soft={soft}"
        );
        // pure corners of plate should be transparent
        assert_eq!(logo.3[0], 0);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn bilinear_scale_keeps_soft_edges() {
        let path = temp_png_black_plate("bilerp");
        let logo = load_title_wordmark(&path).expect("load");
        let soft = probe_scaled_soft_alpha(&logo, 96, 48);
        assert!(
            soft > 20,
            "bilinear scaled wordmark should have soft edge samples, soft={soft}"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_real_logo_title_if_present() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui/logo_title.png");
        if !p.exists() {
            return;
        }
        let logo = load_title_wordmark(&p).expect("png wordmark");
        assert!(logo.0 > 10 && logo.1 > 10);
        assert!(count_soft_alpha(&logo.3) > 50, "real logo should have soft alpha");
    }

    #[test]
    fn write_helper_compiles() {
        let f = std::env::temp_dir().join("velvet_logo_touch.txt");
        let mut file = std::fs::File::create(&f).unwrap();
        writeln!(file, "ok").unwrap();
        let _ = std::fs::remove_file(&f);
    }
}
