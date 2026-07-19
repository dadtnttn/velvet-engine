//! # velvet-image
//!
//! Author/runtime **tools** for image types and compression (not encryption).
//! Intended surface for VS2 `image.*` and for `.vcss` SVG/raster hosts.

#![deny(missing_docs)]

use std::io::Cursor;

use image::ImageFormat;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Detected or requested image kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    /// PNG
    Png,
    /// JPEG
    Jpeg,
    /// WebP (may be unavailable depending on features)
    WebP,
    /// SVG (vector; decode path is rasterize)
    Svg,
    /// Unknown / raw
    Unknown,
}

impl ImageKind {
    /// Extension without dot.
    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Svg => "svg",
            Self::Unknown => "bin",
        }
    }

    /// Parse from format name or extension.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "webp" => Self::WebP,
            "svg" => Self::Svg,
            _ => Self::Unknown,
        }
    }
}

/// Metadata from probing bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Kind
    pub kind: ImageKind,
    /// Width if known
    pub width: u32,
    /// Height if known
    pub height: u32,
    /// Has alpha channel (best-effort)
    pub has_alpha: bool,
    /// Input byte length
    pub bytes: usize,
}

/// RGBA8 image buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Row-major RGBA
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// Empty.
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize).saturating_mul(4);
        Self {
            width,
            height,
            pixels: vec![0; n],
        }
    }

    /// Pixel count.
    pub fn len(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Encode options (compression of **output**, not encryption).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageEncode {
    /// Output kind
    pub kind: ImageKind,
    /// JPEG/WebP quality 1..=100 (ignored for PNG)
    pub quality: u8,
    /// PNG compression level 0..=9
    pub png_level: u8,
}

impl Default for ImageEncode {
    fn default() -> Self {
        Self {
            kind: ImageKind::Png,
            quality: 85,
            png_level: 6,
        }
    }
}

/// Image tool errors.
#[derive(Debug, Error)]
pub enum ImageError {
    /// Decode/encode failure
    #[error("image: {0}")]
    Msg(String),
    /// Unsupported
    #[error("unsupported image kind")]
    Unsupported,
    /// Size limit
    #[error("image too large (max {max} bytes)")]
    TooLarge {
        /// Cap
        max: usize,
    },
}

/// Max bytes accepted for decode (sandbox).
pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

/// Probe image kind and dimensions from raw bytes.
pub fn probe(bytes: &[u8]) -> Result<ImageInfo, ImageError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge {
            max: MAX_IMAGE_BYTES,
        });
    }
    if looks_like_svg(bytes) {
        return Ok(ImageInfo {
            kind: ImageKind::Svg,
            width: 0,
            height: 0,
            has_alpha: true,
            bytes: bytes.len(),
        });
    }
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImageError::Msg(e.to_string()))?;
    let format = reader.format();
    let kind = match format {
        Some(ImageFormat::Png) => ImageKind::Png,
        Some(ImageFormat::Jpeg) => ImageKind::Jpeg,
        Some(ImageFormat::WebP) => ImageKind::WebP,
        _ => ImageKind::Unknown,
    };
    let img = reader
        .decode()
        .map_err(|e| ImageError::Msg(e.to_string()))?;
    let has_alpha = img.color().has_alpha();
    Ok(ImageInfo {
        kind,
        width: img.width(),
        height: img.height(),
        has_alpha,
        bytes: bytes.len(),
    })
}

/// Decode raster image to RGBA8.
pub fn decode_rgba(bytes: &[u8]) -> Result<RgbaImage, ImageError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge {
            max: MAX_IMAGE_BYTES,
        });
    }
    if looks_like_svg(bytes) {
        return Err(ImageError::Msg(
            "SVG requires rasterize_svg(w,h), not decode_rgba".into(),
        ));
    }
    let img = image::load_from_memory(bytes).map_err(|e| ImageError::Msg(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(RgbaImage {
        width: w,
        height: h,
        pixels: rgba.into_raw(),
    })
}

/// Encode RGBA with compression settings.
pub fn encode(rgba: &RgbaImage, opts: ImageEncode) -> Result<Vec<u8>, ImageError> {
    let w = rgba.width;
    let h = rgba.height;
    if w == 0 || h == 0 {
        return Err(ImageError::Msg("empty image".into()));
    }
    let expected = (w as usize) * (h as usize) * 4;
    if rgba.pixels.len() < expected {
        return Err(ImageError::Msg("pixel buffer too short".into()));
    }
    let mut out = Cursor::new(Vec::new());
    match opts.kind {
        ImageKind::Png => {
            use image::ImageEncoder as _;
            let enc = image::codecs::png::PngEncoder::new(&mut out);
            enc.write_image(
                &rgba.pixels[..expected],
                w,
                h,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| ImageError::Msg(e.to_string()))?;
            let _ = opts.png_level; // reserved for future compression knobs
        }
        ImageKind::Jpeg => {
            use image::ImageEncoder as _;
            let q = opts.quality.clamp(1, 100);
            let rgb = flatten_rgb(rgba);
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q);
            enc.write_image(&rgb, w, h, image::ExtendedColorType::Rgb8)
                .map_err(|e| ImageError::Msg(e.to_string()))?;
        }
        ImageKind::WebP | ImageKind::Svg | ImageKind::Unknown => {
            return Err(ImageError::Unsupported);
        }
    }
    Ok(out.into_inner())
}

fn flatten_rgb(rgba: &RgbaImage) -> Vec<u8> {
    let n = (rgba.width as usize) * (rgba.height as usize);
    let mut rgb = Vec::with_capacity(n * 3);
    for i in 0..n {
        let o = i * 4;
        let a = rgba.pixels[o + 3] as f32 / 255.0;
        let r = (rgba.pixels[o] as f32 * a + 255.0 * (1.0 - a)) as u8;
        let g = (rgba.pixels[o + 1] as f32 * a + 255.0 * (1.0 - a)) as u8;
        let b = (rgba.pixels[o + 2] as f32 * a + 255.0 * (1.0 - a)) as u8;
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    rgb
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let s = std::str::from_utf8(bytes).unwrap_or("");
    let t = s.trim_start();
    t.starts_with("<svg") || t.starts_with("<?xml") && t.contains("<svg")
}

/// Very small SVG path rasterizer for `M/L/Z` polygons (game icons).
///
/// Full SVG: prefer file raster via external tooling later; this covers `@svg` badges.
pub fn rasterize_simple_svg(svg: &str, width: u32, height: u32) -> Result<RgbaImage, ImageError> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(ImageError::Msg("invalid svg raster size".into()));
    }
    let mut img = RgbaImage::new(width, height);
    // parse fill color
    let fill = parse_svg_color_attr(svg, "fill").unwrap_or([235, 200, 120, 255]);
    let path = extract_path_d(svg).unwrap_or_default();
    let poly = parse_path_points(&path);
    if poly.len() >= 3 {
        fill_polygon(&mut img, &poly, fill);
    } else {
        // fallback: filled rect inset
        fill_rect(
            &mut img,
            width / 8,
            height / 8,
            width * 3 / 4,
            height * 3 / 4,
            fill,
        );
    }
    Ok(img)
}

fn parse_svg_color_attr(svg: &str, attr: &str) -> Option<[u8; 4]> {
    let key = format!("{attr}=\"");
    let i = svg.find(&key)?;
    let rest = &svg[i + key.len()..];
    let end = rest.find('"')?;
    let c = rest[..end].trim();
    if c.starts_with('#') && c.len() >= 7 {
        let r = u8::from_str_radix(&c[1..3], 16).ok()?;
        let g = u8::from_str_radix(&c[3..5], 16).ok()?;
        let b = u8::from_str_radix(&c[5..7], 16).ok()?;
        return Some([r, g, b, 255]);
    }
    None
}

fn extract_path_d(svg: &str) -> Option<String> {
    let i = svg.find("d=\"")?;
    let rest = &svg[i + 3..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_path_points(d: &str) -> Vec<(f32, f32)> {
    let mut pts = Vec::new();
    let mut cur = (0.0f32, 0.0f32);
    let tokens: Vec<&str> = d
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .collect();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        if t == "M" || t == "L" || t == "m" || t == "l" {
            i += 1;
            if i + 1 < tokens.len() {
                if let (Ok(x), Ok(y)) = (tokens[i].parse::<f32>(), tokens[i + 1].parse::<f32>()) {
                    cur = if t == "m" || t == "l" {
                        (cur.0 + x, cur.1 + y)
                    } else {
                        (x, y)
                    };
                    pts.push(cur);
                    i += 2;
                    continue;
                }
            }
        } else if t == "Z" || t == "z" {
            i += 1;
        } else if let Ok(x) = t.parse::<f32>() {
            if i + 1 < tokens.len() {
                if let Ok(y) = tokens[i + 1].parse::<f32>() {
                    cur = (x, y);
                    pts.push(cur);
                    i += 2;
                    continue;
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    pts
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, rgba: [u8; 4]) {
    for yy in y..y.saturating_add(h).min(img.height) {
        for xx in x..x.saturating_add(w).min(img.width) {
            put(img, xx, yy, rgba);
        }
    }
}

fn fill_polygon(img: &mut RgbaImage, pts: &[(f32, f32)], rgba: [u8; 4]) {
    // scale path from viewBox-ish 0..64 into image
    let max_x = pts.iter().map(|p| p.0).fold(1.0f32, f32::max);
    let max_y = pts.iter().map(|p| p.1).fold(1.0f32, f32::max);
    let sx = (img.width as f32) / max_x.max(1.0);
    let sy = (img.height as f32) / max_y.max(1.0);
    let scaled: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (x * sx, y * sy)).collect();
    let min_y = scaled.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32;
    let max_y = scaled.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32;
    for y in min_y..=max_y {
        let mut nodes = Vec::new();
        let n = scaled.len();
        for i in 0..n {
            let (x1, y1) = scaled[i];
            let (x2, y2) = scaled[(i + 1) % n];
            if (y1 <= y as f32 && y2 > y as f32) || (y2 <= y as f32 && y1 > y as f32) {
                let t = (y as f32 - y1) / (y2 - y1);
                nodes.push(x1 + t * (x2 - x1));
            }
        }
        nodes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut j = 0;
        while j + 1 < nodes.len() {
            let x0 = nodes[j].ceil() as i32;
            let x1 = nodes[j + 1].floor() as i32;
            for x in x0..=x1 {
                if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
                    put(img, x as u32, y as u32, rgba);
                }
            }
            j += 2;
        }
    }
}

fn put(img: &mut RgbaImage, x: u32, y: u32, rgba: [u8; 4]) {
    let i = ((y * img.width + x) * 4) as usize;
    if i + 3 < img.pixels.len() {
        img.pixels[i] = rgba[0];
        img.pixels[i + 1] = rgba[1];
        img.pixels[i + 2] = rgba[2];
        img.pixels[i + 3] = rgba[3];
    }
}

/// Build a minimal SVG document from style-like fields.
pub fn build_svg_document(
    view_w: u32,
    view_h: u32,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    path_d: &str,
) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {view_w} {view_h}"><path d="{path_d}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width}"/></svg>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrip_probe_decode_encode() {
        let mut img = RgbaImage::new(4, 4);
        for i in 0..16 {
            let o = i * 4;
            img.pixels[o] = 200;
            img.pixels[o + 1] = 100;
            img.pixels[o + 2] = 50;
            img.pixels[o + 3] = 255;
        }
        let png = encode(
            &img,
            ImageEncode {
                kind: ImageKind::Png,
                ..Default::default()
            },
        )
        .unwrap();
        let info = probe(&png).unwrap();
        assert_eq!(info.kind, ImageKind::Png);
        assert_eq!(info.width, 4);
        assert_eq!(info.height, 4);
        let back = decode_rgba(&png).unwrap();
        assert_eq!(back.width, 4);
        assert_eq!(back.pixels[0], 200);
    }

    #[test]
    fn jpeg_encode_smaller_or_valid() {
        let mut img = RgbaImage::new(32, 32);
        for p in img.pixels.chunks_mut(4) {
            p[0] = 10;
            p[1] = 20;
            p[2] = 30;
            p[3] = 255;
        }
        let jpg = encode(
            &img,
            ImageEncode {
                kind: ImageKind::Jpeg,
                quality: 50,
                png_level: 6,
            },
        )
        .unwrap();
        assert!(!jpg.is_empty());
        let info = probe(&jpg).unwrap();
        assert_eq!(info.kind, ImageKind::Jpeg);
    }

    #[test]
    fn simple_svg_raster_nonzero() {
        let svg = build_svg_document(
            64,
            64,
            "#ebc878",
            "#000",
            1.0,
            "M0,32 L32,0 L64,32 L32,64 Z",
        );
        let img = rasterize_simple_svg(&svg, 64, 64).unwrap();
        assert!(img.pixels.iter().any(|&b| b != 0));
    }
}
