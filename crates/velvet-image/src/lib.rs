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
        let n = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
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

/// SVG path rasterizer for game UI (`@svg` badges, title wordmarks).
///
/// Supports: `viewBox`, multiple `path d="…"` elements, multi-subpath `M…Z`,
/// `M/L/H/V/C/Q/Z` (cubics/quads tessellated), fill + optional stroke.
/// Transparent background (alpha). Not a full SVG 2 engine.
pub fn rasterize_simple_svg(svg: &str, width: u32, height: u32) -> Result<RgbaImage, ImageError> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(ImageError::Msg("invalid svg raster size".into()));
    }
    let mut img = RgbaImage::new(width, height);
    let (vb_w, vb_h) = parse_viewbox(svg).unwrap_or((64.0, 64.0));
    let fill = parse_svg_color_attr(svg, "fill").unwrap_or([235, 200, 120, 255]);
    let stroke = parse_svg_color_attr(svg, "stroke");
    let stroke_w = parse_stroke_width(svg).unwrap_or(0.0);

    let paths = extract_all_path_d(svg);
    let mut any = false;
    for d in &paths {
        // All subpaths in one `d` use even-odd together (letter holes)
        let mut polys_vb: Vec<Vec<(f32, f32)>> = Vec::new();
        for sub in split_subpaths(d) {
            let poly = parse_path_points(&sub);
            if poly.len() >= 3 {
                polys_vb.push(poly);
            }
        }
        if !polys_vb.is_empty() {
            fill_polygons_vb_evenodd(&mut img, &polys_vb, vb_w, vb_h, fill);
            any = true;
            if stroke_w > 0.05 {
                if let Some(sc) = stroke {
                    if sc[3] > 0 {
                        for poly in &polys_vb {
                            stroke_polyline_vb(&mut img, poly, vb_w, vb_h, sc, stroke_w, true);
                        }
                    }
                }
            }
        }
    }
    if !any {
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

fn parse_viewbox(svg: &str) -> Option<(f32, f32)> {
    let key = "viewBox=\"";
    let i = svg.find(key).or_else(|| svg.find("viewbox=\""))?;
    let rest = &svg[i + key.len()..];
    let end = rest.find('"')?;
    let nums: Vec<f32> = rest[..end]
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|t| t.parse().ok())
        .collect();
    if nums.len() >= 4 {
        Some((nums[2].max(1.0), nums[3].max(1.0)))
    } else {
        None
    }
}

fn parse_stroke_width(svg: &str) -> Option<f32> {
    for key in ["stroke-width=\"", "stroke_width=\""] {
        if let Some(i) = svg.find(key) {
            let rest = &svg[i + key.len()..];
            let end = rest.find('"')?;
            return rest[..end].parse().ok();
        }
    }
    None
}

fn parse_svg_color_attr(svg: &str, attr: &str) -> Option<[u8; 4]> {
    let key = format!("{attr}=\"");
    // last occurrence wins (path can override root); scan all
    let mut last = None;
    let mut search = svg;
    while let Some(i) = search.find(&key) {
        let rest = &search[i + key.len()..];
        let end = rest.find('"')?;
        let c = rest[..end].trim();
        if c.eq_ignore_ascii_case("none") || c.eq_ignore_ascii_case("transparent") {
            last = Some([0, 0, 0, 0]);
        } else if c.starts_with('#') && c.len() >= 7 {
            let r = u8::from_str_radix(&c[1..3], 16).ok()?;
            let g = u8::from_str_radix(&c[3..5], 16).ok()?;
            let b = u8::from_str_radix(&c[5..7], 16).ok()?;
            last = Some([r, g, b, 255]);
        }
        search = &rest[end + 1..];
    }
    last
}

fn extract_all_path_d(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut search = svg;
    while let Some(i) = search.find("d=\"") {
        let rest = &search[i + 3..];
        if let Some(end) = rest.find('"') {
            out.push(rest[..end].to_string());
            search = &rest[end + 1..];
        } else {
            break;
        }
    }
    out
}

/// Split a path `d` into subpaths starting at each absolute/relative moveto.
fn split_subpaths(d: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    for c in d.chars() {
        if (c == 'M' || c == 'm') && !cur.is_empty() {
            parts.push(cur);
            cur = String::new();
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    if parts.is_empty() {
        parts.push(d.to_string());
    }
    parts
}

fn parse_path_points(d: &str) -> Vec<(f32, f32)> {
    let mut pts = Vec::new();
    let mut cur = (0.0f32, 0.0f32);
    let mut start = (0.0f32, 0.0f32);
    // Tokenize: commands and numbers (including negatives / decimals)
    let tokens = tokenize_path(d);
    let mut i = 0;
    let mut cmd = 'M';
    while i < tokens.len() {
        let t = &tokens[i];
        if t.len() == 1
            && t.chars()
                .next()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
        {
            cmd = t.chars().next().unwrap();
            i += 1;
        }
        match cmd {
            'M' | 'm' | 'L' | 'l' => {
                if i + 1 >= tokens.len() {
                    break;
                }
                let Ok(x) = tokens[i].parse::<f32>() else {
                    i += 1;
                    continue;
                };
                let Ok(y) = tokens[i + 1].parse::<f32>() else {
                    i += 1;
                    continue;
                };
                cur = if cmd == 'm' || cmd == 'l' {
                    (cur.0 + x, cur.1 + y)
                } else {
                    (x, y)
                };
                if cmd == 'M' || cmd == 'm' {
                    start = cur;
                }
                pts.push(cur);
                i += 2;
                // subsequent pairs after M are implicit L
                if cmd == 'M' {
                    cmd = 'L';
                }
                if cmd == 'm' {
                    cmd = 'l';
                }
            }
            'H' | 'h' => {
                if i >= tokens.len() {
                    break;
                }
                let Ok(x) = tokens[i].parse::<f32>() else {
                    i += 1;
                    continue;
                };
                cur.0 = if cmd == 'h' { cur.0 + x } else { x };
                pts.push(cur);
                i += 1;
            }
            'V' | 'v' => {
                if i >= tokens.len() {
                    break;
                }
                let Ok(y) = tokens[i].parse::<f32>() else {
                    i += 1;
                    continue;
                };
                cur.1 = if cmd == 'v' { cur.1 + y } else { y };
                pts.push(cur);
                i += 1;
            }
            'C' | 'c' => {
                if i + 5 >= tokens.len() {
                    break;
                }
                let nums: Result<Vec<f32>, _> =
                    tokens[i..i + 6].iter().map(|s| s.parse()).collect();
                let Ok(n) = nums else {
                    i += 1;
                    continue;
                };
                let (x1, y1, x2, y2, x, y) = if cmd == 'c' {
                    (
                        cur.0 + n[0],
                        cur.1 + n[1],
                        cur.0 + n[2],
                        cur.1 + n[3],
                        cur.0 + n[4],
                        cur.1 + n[5],
                    )
                } else {
                    (n[0], n[1], n[2], n[3], n[4], n[5])
                };
                tessellate_cubic(&mut pts, cur, (x1, y1), (x2, y2), (x, y), 12);
                cur = (x, y);
                i += 6;
            }
            'Q' | 'q' => {
                if i + 3 >= tokens.len() {
                    break;
                }
                let nums: Result<Vec<f32>, _> =
                    tokens[i..i + 4].iter().map(|s| s.parse()).collect();
                let Ok(n) = nums else {
                    i += 1;
                    continue;
                };
                let (x1, y1, x, y) = if cmd == 'q' {
                    (cur.0 + n[0], cur.1 + n[1], cur.0 + n[2], cur.1 + n[3])
                } else {
                    (n[0], n[1], n[2], n[3])
                };
                tessellate_quad(&mut pts, cur, (x1, y1), (x, y), 10);
                cur = (x, y);
                i += 4;
            }
            'Z' | 'z' => {
                pts.push(start);
                cur = start;
                // Z consumes no coords
            }
            _ => {
                i += 1;
            }
        }
    }
    pts
}

fn tokenize_path(d: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut num = String::new();
    let flush = |num: &mut String, out: &mut Vec<String>| {
        if !num.is_empty() {
            out.push(std::mem::take(num));
        }
    };
    let bytes: Vec<char> = d.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() {
            flush(&mut num, &mut out);
            out.push(c.to_string());
            i += 1;
        } else if c == ',' || c.is_whitespace() {
            flush(&mut num, &mut out);
            i += 1;
        } else if c == '-' || c == '+' || c == '.' || c.is_ascii_digit() {
            // number; '-' starts new number unless mid-exponent
            if (c == '-' || c == '+')
                && !num.is_empty()
                && !num.ends_with('e')
                && !num.ends_with('E')
            {
                flush(&mut num, &mut out);
            }
            num.push(c);
            i += 1;
        } else {
            flush(&mut num, &mut out);
            i += 1;
        }
    }
    flush(&mut num, &mut out);
    out
}

fn tessellate_cubic(
    pts: &mut Vec<(f32, f32)>,
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    steps: usize,
) {
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let u = 1.0 - t;
        let x =
            u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0;
        let y =
            u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1;
        pts.push((x, y));
    }
}

fn tessellate_quad(
    pts: &mut Vec<(f32, f32)>,
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    steps: usize,
) {
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let u = 1.0 - t;
        let x = u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0;
        let y = u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1;
        pts.push((x, y));
    }
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, rgba: [u8; 4]) {
    for yy in y..y.saturating_add(h).min(img.height) {
        for xx in x..x.saturating_add(w).min(img.width) {
            put(img, xx, yy, rgba);
        }
    }
}

fn map_vb(x: f32, y: f32, vb_w: f32, vb_h: f32, img: &RgbaImage) -> (f32, f32) {
    let sx = img.width as f32 / vb_w.max(1.0);
    let sy = img.height as f32 / vb_h.max(1.0);
    (x * sx, y * sy)
}

fn fill_polygons_vb_evenodd(
    img: &mut RgbaImage,
    polys: &[Vec<(f32, f32)>],
    vb_w: f32,
    vb_h: f32,
    rgba: [u8; 4],
) {
    if rgba[3] == 0 {
        return;
    }
    let scaled: Vec<Vec<(f32, f32)>> = polys
        .iter()
        .filter(|p| p.len() >= 3)
        .map(|pts| {
            pts.iter()
                .map(|(x, y)| map_vb(*x, *y, vb_w, vb_h, img))
                .collect()
        })
        .collect();
    fill_polygons_evenodd(img, &scaled, rgba);
}

/// Even-odd fill across one or more closed contours (supports letter counters / holes).
fn fill_polygons_evenodd(img: &mut RgbaImage, polys: &[Vec<(f32, f32)>], rgba: [u8; 4]) {
    if polys.is_empty() {
        return;
    }
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for scaled in polys {
        for p in scaled {
            min_y = min_y.min(p.1);
            max_y = max_y.max(p.1);
        }
    }
    let min_y = min_y.floor() as i32;
    let max_y = max_y.ceil() as i32;
    for y in min_y..=max_y {
        let mut nodes = Vec::new();
        for scaled in polys {
            let n = scaled.len();
            if n < 2 {
                continue;
            }
            for i in 0..n {
                let (x1, y1) = scaled[i];
                let (x2, y2) = scaled[(i + 1) % n];
                if (y1 <= y as f32 && y2 > y as f32) || (y2 <= y as f32 && y1 > y as f32) {
                    let t = (y as f32 - y1) / (y2 - y1 + 1e-6);
                    nodes.push(x1 + t * (x2 - x1));
                }
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

fn stroke_polyline_vb(
    img: &mut RgbaImage,
    pts: &[(f32, f32)],
    vb_w: f32,
    vb_h: f32,
    rgba: [u8; 4],
    width: f32,
    closed: bool,
) {
    if pts.len() < 2 || rgba[3] == 0 {
        return;
    }
    let scaled: Vec<(f32, f32)> = pts
        .iter()
        .map(|(x, y)| map_vb(*x, *y, vb_w, vb_h, img))
        .collect();
    let n = scaled.len();
    let segs = if closed { n } else { n - 1 };
    let half = (width * img.width as f32 / vb_w.max(1.0) * 0.5).max(0.6);
    for i in 0..segs {
        let a = scaled[i];
        let b = scaled[(i + 1) % n];
        stroke_segment(img, a, b, half, rgba);
    }
}

fn stroke_segment(img: &mut RgbaImage, a: (f32, f32), b: (f32, f32), half: f32, rgba: [u8; 4]) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let steps = (len * 1.5).ceil() as i32;
    let nx = -dy / len * half;
    let ny = dx / len * half;
    for s in 0..=steps {
        let t = s as f32 / steps.max(1) as f32;
        let cx = a.0 + dx * t;
        let cy = a.1 + dy * t;
        // thick stroke as small disk
        let r = half.ceil() as i32;
        for oy in -r..=r {
            for ox in -r..=r {
                if (ox * ox + oy * oy) as f32 <= half * half + 0.5 {
                    let x = (cx + ox as f32).round() as i32;
                    let y = (cy + oy as f32).round() as i32;
                    if x >= 0 && y >= 0 && (x as u32) < img.width && (y as u32) < img.height {
                        put(img, x as u32, y as u32, rgba);
                    }
                }
            }
        }
        let _ = (nx, ny);
    }
}

fn put(img: &mut RgbaImage, x: u32, y: u32, rgba: [u8; 4]) {
    let i = ((y * img.width + x) * 4) as usize;
    if i + 3 < img.pixels.len() {
        // alpha over
        let sa = rgba[3] as f32 / 255.0;
        if sa >= 0.99 {
            img.pixels[i] = rgba[0];
            img.pixels[i + 1] = rgba[1];
            img.pixels[i + 2] = rgba[2];
            img.pixels[i + 3] = rgba[3];
        } else if sa > 0.01 {
            let da = img.pixels[i + 3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a > 0.001 {
                for (channel, &source) in rgba.iter().take(3).enumerate() {
                    let s = source as f32;
                    let d = img.pixels[i + channel] as f32;
                    img.pixels[i + channel] = ((s * sa + d * da * (1.0 - sa)) / out_a)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
                img.pixels[i + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
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

/// Build a multi-path SVG (each path is a full `d` subpath string).
pub fn build_svg_multipath(
    view_w: u32,
    view_h: u32,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    paths: &[&str],
) -> String {
    let mut body = String::new();
    for d in paths {
        body.push_str(&format!(
            r#"<path d="{d}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width}"/>"#
        ));
    }
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {view_w} {view_h}">{body}</svg>"#
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
