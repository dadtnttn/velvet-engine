//! sRGB / linear color-space conversion helpers.

use crate::{clamp, Color, Color8};

/// Convert a single channel from sRGB (gamma-encoded) to linear.
#[inline]
pub fn srgb_to_linear_channel(c: f32) -> f32 {
    let c = clamp(c, 0.0, 1.0);
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert a single channel from linear to sRGB (gamma-encoded).
#[inline]
pub fn linear_to_srgb_channel(c: f32) -> f32 {
    let c = clamp(c, 0.0, 1.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert RGBA color from sRGB to linear (alpha unchanged).
pub fn srgb_to_linear(c: Color) -> Color {
    Color {
        r: srgb_to_linear_channel(c.r),
        g: srgb_to_linear_channel(c.g),
        b: srgb_to_linear_channel(c.b),
        a: c.a,
    }
}

/// Convert RGBA color from linear to sRGB (alpha unchanged).
pub fn linear_to_srgb(c: Color) -> Color {
    Color {
        r: linear_to_srgb_channel(c.r),
        g: linear_to_srgb_channel(c.g),
        b: linear_to_srgb_channel(c.b),
        a: c.a,
    }
}

/// Convert 8-bit sRGB bytes to linear float color.
pub fn srgb8_to_linear(c: Color8) -> Color {
    srgb_to_linear(Color::from_color8(c))
}

/// Convert linear float color to 8-bit sRGB storage.
pub fn linear_to_srgb8(c: Color) -> Color8 {
    linear_to_srgb(c).to_color8()
}

/// Relative luminance (Rec. 709) for a **linear** RGB color.
pub fn linear_luminance(c: Color) -> f32 {
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
}

/// Relative luminance treating input as sRGB-encoded.
pub fn srgb_luminance(c: Color) -> f32 {
    linear_luminance(srgb_to_linear(c))
}

/// WCAG contrast ratio between two linear colors (alpha ignored).
pub fn contrast_ratio_linear(a: Color, b: Color) -> f32 {
    let l1 = linear_luminance(a);
    let l2 = linear_luminance(b);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

/// Linear interpolate in linear space (preferred for lighting).
pub fn lerp_linear(a: Color, b: Color, t: f32) -> Color {
    let al = srgb_to_linear(a);
    let bl = srgb_to_linear(b);
    linear_to_srgb(al.lerp(bl, t))
}

/// Premultiply alpha in linear space then return sRGB.
pub fn premultiply_srgb(c: Color) -> Color {
    let lin = srgb_to_linear(c);
    linear_to_srgb(lin.premultiply())
}

/// Approximate gamma encode with fixed exponent (faster, less accurate than sRGB).
#[inline]
pub fn approx_gamma_encode(c: f32, gamma: f32) -> f32 {
    clamp(c, 0.0, 1.0).powf(1.0 / gamma.max(1e-3))
}

/// Approximate gamma decode.
#[inline]
pub fn approx_gamma_decode(c: f32, gamma: f32) -> f32 {
    clamp(c, 0.0, 1.0).powf(gamma.max(1e-3))
}

/// Convert HSV (h in degrees 0..360, s/v in 0..1) to sRGB-ish [`Color`].
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let s = clamp(s, 0.0, 1.0);
    let v = clamp(v, 0.0, 1.0);
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Color::rgb(r + m, g + m, b + m)
}

/// Convert RGB to HSV; hue in degrees.
pub fn rgb_to_hsv(c: Color) -> (f32, f32, f32) {
    let r = clamp(c.r, 0.0, 1.0);
    let g = clamp(c.g, 0.0, 1.0);
    let b = clamp(c.b, 0.0, 1.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max <= 1e-8 { 0.0 } else { delta / max };
    let h = if delta <= 1e-8 {
        0.0
    } else if (max - r).abs() < 1e-8 {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < 1e-8 {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Simple tonemap (Reinhard) for HDR linear RGB → display linear.
pub fn reinhard_tonemap(c: Color, exposure: f32) -> Color {
    let e = exposure.max(0.0);
    Color {
        r: (c.r * e) / (1.0 + c.r * e),
        g: (c.g * e) / (1.0 + c.g * e),
        b: (c.b * e) / (1.0 + c.b * e),
        a: c.a,
    }
}

/// Apply exposure then sRGB encode for display.
pub fn linear_to_display_srgb(c: Color, exposure: f32) -> Color {
    linear_to_srgb(reinhard_tonemap(c, exposure))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_mid_gray() {
        let s = 0.5;
        let lin = srgb_to_linear_channel(s);
        let back = linear_to_srgb_channel(lin);
        assert!((back - s).abs() < 1e-5);
        // Mid sRGB gray is darker in linear.
        assert!(lin < 0.3);
    }

    #[test]
    fn black_white_endpoints() {
        assert!(srgb_to_linear_channel(0.0).abs() < 1e-6);
        assert!((srgb_to_linear_channel(1.0) - 1.0).abs() < 1e-5);
        assert!((linear_to_srgb_channel(1.0) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn color_roundtrip() {
        let c = Color::rgb(0.2, 0.5, 0.8);
        let back = linear_to_srgb(srgb_to_linear(c));
        assert!((back.r - c.r).abs() < 1e-4);
        assert!((back.g - c.g).abs() < 1e-4);
        assert!((back.b - c.b).abs() < 1e-4);
    }

    #[test]
    fn luminance_white() {
        let l = linear_luminance(Color::WHITE);
        assert!((l - 1.0).abs() < 1e-5);
    }

    #[test]
    fn contrast_black_white() {
        let ratio = contrast_ratio_linear(Color::BLACK, Color::WHITE);
        assert!((ratio - 21.0).abs() < 0.1);
    }

    #[test]
    fn hsv_roundtrip_red() {
        let c = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((c.r - 1.0).abs() < 1e-4);
        assert!(c.g.abs() < 1e-4);
        let (h, s, v) = rgb_to_hsv(c);
        assert!(h.abs() < 1e-2 || (h - 360.0).abs() < 1e-2);
        assert!((s - 1.0).abs() < 1e-4);
        assert!((v - 1.0).abs() < 1e-4);
    }

    #[test]
    fn tonemap_reduces_hdr() {
        let c = Color::rgb(4.0, 4.0, 4.0);
        let t = reinhard_tonemap(c, 1.0);
        assert!(t.r < 1.0);
        assert!(t.r > 0.7);
    }

    #[test]
    fn srgb8_path() {
        let c8 = Color8::rgba(128, 64, 32, 255);
        let lin = srgb8_to_linear(c8);
        let back = linear_to_srgb8(lin);
        assert!((back.r as i16 - 128).abs() <= 1);
    }

    #[test]
    fn property_channel_roundtrip_grid() {
        for i in 0..=32 {
            let s = i as f32 / 32.0;
            let lin = srgb_to_linear_channel(s);
            let back = linear_to_srgb_channel(lin);
            assert!((back - s).abs() < 1e-4, "s={s} lin={lin} back={back}");
            // sRGB encode is above linear for mid tones (gamma).
            if s > 0.05 && s < 0.95 {
                assert!(lin < s + 1e-4, "expected linear darker for mid gray");
            }
        }
    }

    #[test]
    fn property_color_roundtrip_samples() {
        let samples = [
            Color::rgb(0.0, 0.0, 0.0),
            Color::rgb(1.0, 1.0, 1.0),
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.0, 1.0, 0.0),
            Color::rgb(0.0, 0.0, 1.0),
            Color::rgb(0.2, 0.4, 0.6),
            Color::rgba(0.1, 0.2, 0.3, 0.5),
            Color::rgb(0.04045, 0.5, 0.8),
        ];
        for c in samples {
            let back = linear_to_srgb(srgb_to_linear(c));
            assert!((back.r - c.r).abs() < 1e-4);
            assert!((back.g - c.g).abs() < 1e-4);
            assert!((back.b - c.b).abs() < 1e-4);
            assert!((back.a - c.a).abs() < 1e-6);
        }
    }

    #[test]
    fn property_luminance_monotonic_gray() {
        let mut prev = -1.0f32;
        for i in 0..=20 {
            let g = i as f32 / 20.0;
            let l = srgb_luminance(Color::rgb(g, g, g));
            assert!(l + 1e-5 >= prev, "luminance should rise with gray");
            prev = l;
        }
        assert!(linear_luminance(Color::rgb(1.0, 0.0, 0.0)) < 0.3);
        assert!(linear_luminance(Color::rgb(0.0, 1.0, 0.0)) > 0.5);
    }

    #[test]
    fn property_contrast_ratio_order() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        let gray = Color::rgb(0.5, 0.5, 0.5);
        let c_bw = contrast_ratio_linear(srgb_to_linear(black), srgb_to_linear(white));
        let c_bg = contrast_ratio_linear(srgb_to_linear(black), srgb_to_linear(gray));
        let c_gw = contrast_ratio_linear(srgb_to_linear(gray), srgb_to_linear(white));
        assert!(c_bw > c_bg);
        assert!(c_bw > c_gw);
        assert!(c_bw > 20.0);
        // Symmetric.
        assert!(
            (contrast_ratio_linear(Color::WHITE, Color::BLACK) - c_bw).abs() < 1e-4
                || (contrast_ratio_linear(srgb_to_linear(white), srgb_to_linear(black))
                    - contrast_ratio_linear(srgb_to_linear(black), srgb_to_linear(white)))
                .abs()
                    < 1e-4
        );
    }

    #[test]
    fn property_hsv_primary_and_gray() {
        // Primaries.
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((red.r - 1.0).abs() < 1e-3 && red.g.abs() < 1e-3 && red.b.abs() < 1e-3);
        let green = hsv_to_rgb(120.0, 1.0, 1.0);
        assert!(green.g > 0.99 && green.r.abs() < 1e-2);
        let blue = hsv_to_rgb(240.0, 1.0, 1.0);
        assert!(blue.b > 0.99 && blue.r.abs() < 1e-2);
        // Gray: s=0.
        for v in [0.0_f32, 0.5, 1.0] {
            let c = hsv_to_rgb(180.0, 0.0, v);
            assert!((c.r - v).abs() < 1e-3);
            assert!((c.g - v).abs() < 1e-3);
            assert!((c.b - v).abs() < 1e-3);
        }
        // Roundtrip several hues.
        for h in [0.0_f32, 30.0, 90.0, 150.0, 210.0, 300.0] {
            let c = hsv_to_rgb(h, 0.8, 0.9);
            let (hh, ss, vv) = rgb_to_hsv(c);
            assert!((ss - 0.8).abs() < 0.05, "s={ss}");
            assert!((vv - 0.9).abs() < 0.05, "v={vv}");
            let dh = (hh - h)
                .abs()
                .min((hh - h + 360.0).abs())
                .min((hh - h - 360.0).abs());
            assert!(dh < 2.0, "h in={h} out={hh}");
        }
    }

    #[test]
    fn property_tonemap_and_display() {
        for e in [0.5_f32, 1.0, 2.0] {
            for intensity in [0.0_f32, 0.5, 1.0, 2.0, 8.0, 50.0] {
                let c = Color::rgb(intensity, intensity * 0.5, intensity * 0.25);
                let t = reinhard_tonemap(c, e);
                assert!(t.r <= 1.0 + 1e-4);
                assert!(t.g <= 1.0 + 1e-4);
                assert!(t.b <= 1.0 + 1e-4);
                assert!(t.r >= 0.0 && t.a == c.a);
                if intensity > 0.0 {
                    assert!(t.r > 0.0);
                }
                let d = linear_to_display_srgb(c, e);
                assert!((0.0..=1.0).contains(&d.r) || d.r < 1.0 + 1e-3);
            }
        }
        // Higher exposure brightens mid tones before tonemap saturation.
        let c = Color::rgb(0.5, 0.5, 0.5);
        let lo = reinhard_tonemap(c, 0.5);
        let hi = reinhard_tonemap(c, 2.0);
        assert!(hi.r > lo.r);
    }

    #[test]
    fn property_srgb8_quantization_stable() {
        for r in [0u8, 1, 64, 127, 128, 200, 254, 255] {
            for g in [0u8, 128, 255] {
                for b in [0u8, 64, 255] {
                    let c8 = Color8::rgba(r, g, b, 255);
                    let back = linear_to_srgb8(srgb8_to_linear(c8));
                    assert!((back.r as i16 - r as i16).abs() <= 1);
                    assert!((back.g as i16 - g as i16).abs() <= 1);
                    assert!((back.b as i16 - b as i16).abs() <= 1);
                }
            }
        }
    }

    #[test]
    fn property_lerp_linear_midpoint() {
        let a = Color::rgb(0.0, 0.0, 0.0);
        let b = Color::rgb(1.0, 1.0, 1.0);
        let mid = lerp_linear(a, b, 0.5);
        // Mid in linear is not 0.5 sRGB.
        assert!(mid.r > 0.0 && mid.r < 1.0);
        // Alpha path via color lerp identity endpoints.
        let ends0 = lerp_linear(a, b, 0.0);
        let ends1 = lerp_linear(a, b, 1.0);
        assert!(ends0.r.abs() < 1e-3);
        assert!((ends1.r - 1.0).abs() < 1e-3);
    }
}
