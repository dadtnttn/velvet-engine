//! GPU product text path: fontdue → glyph atlas → positioned quads.
//!
//! This is the **product** sharp-text path for wgpu hosts (not softbuffer bitmap
//! `draw_text_line`). Glyphon/cosmic-text can wrap the same [`GpuTextRun`] /
//! [`GpuGlyphQuad`] IR later; fontdue keeps the crate free of wgpu coupling.

use std::collections::HashMap;

use fontdue::Font;

/// One glyph instance ready for a textured quad on the GPU.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuGlyphQuad {
    /// Left edge (pixels, same space as layout).
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width of the glyph bitmap.
    pub w: f32,
    /// Height of the glyph bitmap.
    pub h: f32,
    /// Atlas U0 (0..=1).
    pub u0: f32,
    /// Atlas V0.
    pub v0: f32,
    /// Atlas U1.
    pub u1: f32,
    /// Atlas V1.
    pub v1: f32,
    /// Multiplicative RGBA.
    pub color: [f32; 4],
    /// Draw order.
    pub z: f32,
}

/// A text run prepared for GPU submission.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuTextRun {
    /// Original UTF-8.
    pub text: String,
    /// Font size px.
    pub size: f32,
    /// Baseline-ish origin x.
    pub x: f32,
    /// Top / baseline origin y.
    pub y: f32,
    /// Glyph quads.
    pub glyphs: Vec<GpuGlyphQuad>,
    /// Total advance width.
    pub width: f32,
    /// Line height approx.
    pub height: f32,
}

/// CPU glyph atlas (RGBA8) for upload via `GpuContext::create_texture_rgba8`.
#[derive(Debug, Clone)]
pub struct GlyphAtlas {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// RGBA8 packed rows.
    pub rgba: Vec<u8>,
    /// Packed glyph count (codepoint × size keys).
    pub glyph_keys: usize,
}

#[derive(Debug, Clone, Copy)]
struct AtlasRegion {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    /// Bearing x from fontdue metrics.
    xmin: i32,
    /// Bearing y (ymin — typically negative).
    ymin: i32,
    /// Advance width.
    advance: f32,
}

/// Build and pack glyph bitmaps from a TTF/OTF via fontdue.
pub struct GpuTextRasterizer {
    font: Font,
    /// Packing cursor.
    pack_x: u32,
    pack_y: u32,
    row_h: u32,
    atlas_w: u32,
    atlas_h: u32,
    rgba: Vec<u8>,
    regions: HashMap<(char, u32), AtlasRegion>,
}

impl GpuTextRasterizer {
    /// Create from font file bytes.
    pub fn from_font_bytes(bytes: &[u8]) -> Result<Self, String> {
        let font = Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("fontdue: {e}"))?;
        let atlas_w = 1024u32;
        let atlas_h = 1024u32;
        Ok(Self {
            font,
            pack_x: 1,
            pack_y: 1,
            row_h: 0,
            atlas_w,
            atlas_h,
            rgba: vec![0u8; (atlas_w * atlas_h * 4) as usize],
            regions: HashMap::new(),
        })
    }

    /// Try common Windows / Linux system UI fonts.
    pub fn from_system_ui() -> Result<Self, String> {
        let candidates = [
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\calibri.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ];
        for p in candidates {
            if let Ok(bytes) = std::fs::read(p) {
                if let Ok(r) = Self::from_font_bytes(&bytes) {
                    return Ok(r);
                }
            }
        }
        Err("no system UI font found for GpuTextRasterizer".into())
    }

    /// Ensure glyph is in the atlas; return region key size (rounded px).
    fn ensure_glyph(&mut self, ch: char, size_px: f32) -> Option<AtlasRegion> {
        let key_size = size_px.round().max(1.0) as u32;
        let key = (ch, key_size);
        if let Some(r) = self.regions.get(&key) {
            return Some(*r);
        }
        let (metrics, bitmap) = self.font.rasterize(ch, size_px);
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;
        // Space / empty glyphs: store advance-only region.
        if gw == 0 || gh == 0 {
            let reg = AtlasRegion {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
                advance: metrics.advance_width,
            };
            self.regions.insert(key, reg);
            return Some(reg);
        }
        // Pack with 1px padding
        if self.pack_x + gw + 1 > self.atlas_w {
            self.pack_x = 1;
            self.pack_y += self.row_h + 1;
            self.row_h = 0;
        }
        if self.pack_y + gh + 1 > self.atlas_h {
            return None; // atlas full
        }
        let x0 = self.pack_x;
        let y0 = self.pack_y;
        for row in 0..gh {
            for col in 0..gw {
                let cover = bitmap[(row * gw + col) as usize];
                let i = (((y0 + row) * self.atlas_w + (x0 + col)) * 4) as usize;
                // Premultiplied-ish white with alpha = coverage
                self.rgba[i] = 255;
                self.rgba[i + 1] = 255;
                self.rgba[i + 2] = 255;
                self.rgba[i + 3] = cover;
            }
        }
        self.pack_x += gw + 1;
        self.row_h = self.row_h.max(gh);
        let reg = AtlasRegion {
            x: x0,
            y: y0,
            w: gw,
            h: gh,
            xmin: metrics.xmin,
            ymin: metrics.ymin,
            advance: metrics.advance_width,
        };
        self.regions.insert(key, reg);
        Some(reg)
    }

    /// Layout a single-line text run into GPU glyph quads.
    pub fn layout_line(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
        z: f32,
    ) -> GpuTextRun {
        let mut glyphs = Vec::new();
        let mut pen_x = x;
        let mut max_h = size;
        for ch in text.chars() {
            let Some(reg) = self.ensure_glyph(ch, size) else {
                continue;
            };
            if reg.w > 0 && reg.h > 0 {
                let gx = pen_x + reg.xmin as f32;
                // fontdue ymin is typically negative; baseline at y + size
                let baseline = y + size;
                let gy = baseline + reg.ymin as f32;
                let u0 = reg.x as f32 / self.atlas_w as f32;
                let v0 = reg.y as f32 / self.atlas_h as f32;
                let u1 = (reg.x + reg.w) as f32 / self.atlas_w as f32;
                let v1 = (reg.y + reg.h) as f32 / self.atlas_h as f32;
                glyphs.push(GpuGlyphQuad {
                    x: gx,
                    y: gy,
                    w: reg.w as f32,
                    h: reg.h as f32,
                    u0,
                    v0,
                    u1,
                    v1,
                    color,
                    z,
                });
                max_h = max_h.max(reg.h as f32);
            }
            pen_x += reg.advance;
        }
        GpuTextRun {
            text: text.to_string(),
            size,
            x,
            y,
            glyphs,
            width: pen_x - x,
            height: max_h,
        }
    }

    /// Measure advance width without requiring a full run clone of glyphs.
    pub fn measure_width(&mut self, text: &str, size: f32) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            if let Some(reg) = self.ensure_glyph(ch, size) {
                w += reg.advance;
            }
        }
        w
    }

    /// Snapshot atlas for GPU upload.
    pub fn atlas(&self) -> GlyphAtlas {
        GlyphAtlas {
            width: self.atlas_w,
            height: self.atlas_h,
            rgba: self.rgba.clone(),
            glyph_keys: self.regions.len(),
        }
    }

    /// Number of distinct glyphs packed.
    pub fn glyph_count(&self) -> usize {
        self.regions.len()
    }
}

/// Product text descriptor: text, x, y, size, RGBA color, and draw order.
pub type ProductTextItem = (String, f32, f32, f32, [f32; 4], f32);

/// Convert product-style text descriptors into GPU runs.
///
/// `items` are (text, x, y, size, color, z).
pub fn layout_product_text_items(
    raster: &mut GpuTextRasterizer,
    items: &[ProductTextItem],
) -> Vec<GpuTextRun> {
    items
        .iter()
        .map(|(text, x, y, size, color, z)| raster.layout_line(text, *x, *y, *size, *color, *z))
        .collect()
}

/// Flatten runs to a single glyph list (draw order preserved).
pub fn flatten_glyph_quads(runs: &[GpuTextRun]) -> Vec<GpuGlyphQuad> {
    runs.iter().flat_map(|r| r.glyphs.iter().cloned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn try_raster() -> Option<GpuTextRasterizer> {
        GpuTextRasterizer::from_system_ui().ok()
    }

    #[test]
    fn layout_hello_has_glyphs_and_positive_width() {
        let Some(mut r) = try_raster() else {
            eprintln!("phase2_gpu_text: skip — no system font");
            return;
        };
        let run = r.layout_line(
            "Hello Velvet",
            24.0,
            100.0,
            28.0,
            [0.95, 0.95, 0.97, 1.0],
            12.0,
        );
        assert!(run.width > 40.0, "width={}", run.width);
        assert!(
            run.glyphs.len() >= 10,
            "expected many glyphs, got {}",
            run.glyphs.len()
        );
        // Sharp path: every glyph has positive size and UV span
        for g in &run.glyphs {
            assert!(g.w > 0.0 && g.h > 0.0);
            assert!(g.u1 > g.u0 && g.v1 > g.v0);
        }
        let atlas = r.atlas();
        assert_eq!(atlas.rgba.len(), (atlas.width * atlas.height * 4) as usize);
        // Atlas must contain non-zero alpha (real coverage)
        assert!(
            atlas.rgba.iter().skip(3).step_by(4).any(|&a| a > 0),
            "atlas alpha empty"
        );
        eprintln!(
            "phase2_gpu_text: ok glyphs={} atlas={}x{} packed={}",
            run.glyphs.len(),
            atlas.width,
            atlas.height,
            r.glyph_count()
        );
    }

    #[test]
    fn product_items_flatten() {
        let Some(mut r) = try_raster() else {
            return;
        };
        let items = vec![
            (
                "Nora".into(),
                100.0,
                500.0,
                20.0,
                [0.9, 1.0, 0.9, 1.0],
                12.0,
            ),
            (
                "Train lights flicker.".into(),
                100.0,
                540.0,
                28.0,
                [0.95, 0.95, 0.97, 1.0],
                12.0,
            ),
        ];
        let runs = layout_product_text_items(&mut r, &items);
        assert_eq!(runs.len(), 2);
        let flat = flatten_glyph_quads(&runs);
        assert!(flat.len() > 10);
        assert!(runs[1].width > runs[0].width);
    }

    #[test]
    fn measure_matches_layout_width() {
        let Some(mut r) = try_raster() else {
            return;
        };
        let t = "station";
        let m = r.measure_width(t, 24.0);
        let run = r.layout_line(t, 0.0, 0.0, 24.0, [1.0, 1.0, 1.0, 1.0], 0.0);
        assert!(
            (m - run.width).abs() < 0.5,
            "measure={m} layout={}",
            run.width
        );
    }
}
