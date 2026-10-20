//! The canvas: a logical size drawn at its own pixels, a stack of states (transform, clip, opacity) and the drawing
//! calls.

use std::sync::Arc;

use tiny_skia::{
    BlendMode, FillRule, FilterQuality, GradientStop, LineCap, LineJoin, LinearGradient, Mask, Paint, Pixmap, PixmapPaint,
    RadialGradient, SpreadMode, Stroke, StrokeDash, Transform,
};

use std::collections::HashMap;

use crate::blur::blur_mask;
use crate::color::Color;
use crate::font::{Align, TextStyle};
use crate::geometry::{Point, Rect};
use crate::image::{Fit, Image};
use crate::path::Path;

/// How what is drawn next mixes with what is already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blend {
    /// Over, by its opacity.
    #[default]
    Normal,
    /// Darkens like ink soaking into paper.
    Multiply,
    /// Lightens like light on a surface.
    Screen,
    Overlay,
    Darken,
    Lighten,
    /// Adds (glows).
    Add,
    /// Takes away what is under it, by its opacity (rubs out, tears edges).
    Erase,
}

impl Blend {
    fn mode(self) -> BlendMode {
        match self {
            Blend::Normal => BlendMode::SourceOver,
            Blend::Multiply => BlendMode::Multiply,
            Blend::Screen => BlendMode::Screen,
            Blend::Overlay => BlendMode::Overlay,
            Blend::Darken => BlendMode::Darken,
            Blend::Lighten => BlendMode::Lighten,
            Blend::Add => BlendMode::Plus,
            Blend::Erase => BlendMode::DestinationOut,
        }
    }
}

/// How a shape is filled: one colour, or a gradient between colour stops (positions from 0 to 1).
#[derive(Debug, Clone)]
pub enum Fill {
    Solid(Color),
    Linear { from: Point, to: Point, stops: Vec<(f32, Color)> },
    Radial { center: Point, radius: f32, stops: Vec<(f32, Color)> },
}

impl From<Color> for Fill {
    fn from(color: Color) -> Self {
        Fill::Solid(color)
    }
}

/// How a line is drawn: its width (in logical pixels), colour, round or square ends and corners, and an optional
/// dash pattern (lengths of dash and gap, in logical pixels).
#[derive(Debug, Clone)]
pub struct StrokeStyle {
    pub width: f32,
    pub color: Color,
    pub round: bool,
    pub dash: Option<Vec<f32>>,
}

impl StrokeStyle {
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color, round: true, dash: None }
    }

    /// Square ends and sharp corners.
    pub fn square(mut self) -> Self {
        self.round = false;
        self
    }

    pub fn dashed(mut self, pattern: &[f32]) -> Self {
        self.dash = Some(pattern.to_vec());
        self
    }
}

/// What `save` keeps and `restore` brings back.
#[derive(Clone)]
struct State {
    transform: Transform,
    clip: Option<Arc<Mask>>,
    opacity: f32,
    blend: Blend,
}

/// A blurred shadow already made: its shape and blur in pixels, and its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ShadowKey {
    w: u32,
    h: u32,
    radius: u32,
    blur: u32,
    color: Color,
}

/// A canvas of a logical size, drawn at its own pixels.
pub struct Canvas {
    width: f32,
    height: f32,
    k: f32,
    pixmap: Pixmap,
    state: State,
    saved: Vec<State>,
    /// Shadows already blurred: a shadow of the same shape is only placed again.
    shadows: HashMap<ShadowKey, Pixmap>,
}

impl std::fmt::Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas")
            .field("logical", &(self.width, self.height))
            .field("pixels", &(self.pixmap.width(), self.pixmap.height()))
            .finish()
    }
}

impl Canvas {
    /// A canvas whose pixels are its logical size (one pixel a logical pixel).
    pub fn new(width: f32, height: f32) -> Canvas {
        Canvas::with_pixels(width, height, width.ceil() as u32, height.ceil() as u32)
    }

    /// A canvas of a logical size drawn at `pw`×`ph` pixels: `k` = the pixels a logical pixel takes (the smaller of the
    /// two ratios, so a slightly different aspect never stretches anything).
    pub fn with_pixels(width: f32, height: f32, pw: u32, ph: u32) -> Canvas {
        let (pw, ph) = (pw.max(1), ph.max(1));
        let k = (pw as f32 / width.max(1.0)).min(ph as f32 / height.max(1.0));
        let pixmap = Pixmap::new(pw, ph).expect("a canvas of at least one pixel");
        Canvas {
            width,
            height,
            k,
            pixmap,
            state: State { transform: Transform::identity(), clip: None, opacity: 1.0, blend: Blend::Normal },
            saved: Vec::new(),
            shadows: HashMap::new(),
        }
    }

    /// A layer for this canvas: a canvas of its own of a logical size, at this canvas's pixels per logical pixel. What
    /// is drawn into it once can be drawn into this canvas many times with `draw_layer`.
    pub fn layer(&self, width: f32, height: f32) -> Canvas {
        // The layer keeps this canvas's exact k (its pixels are rounded up around it): drawn back at scale 1 it is a
        // plain copy, not a resampling.
        let mut layer = Canvas::with_pixels(width, height, (width * self.k).ceil() as u32, (height * self.k).ceil() as u32);
        layer.k = self.k;
        layer
    }

    /// Changes the pixels the canvas is drawn at (the window was resized). The drawing is lost; the state is reset.
    pub fn resize_pixels(&mut self, pw: u32, ph: u32) {
        if pw.max(1) == self.pixmap.width() && ph.max(1) == self.pixmap.height() {
            return;
        }
        *self = Canvas::with_pixels(self.width, self.height, pw, ph);
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    /// The pixels a logical pixel takes.
    pub fn k(&self) -> f32 {
        self.k
    }

    pub fn pixel_width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn pixel_height(&self) -> u32 {
        self.pixmap.height()
    }

    /// The pixels: premultiplied RGBA, row by row.
    pub fn pixels(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// One pixel, premultiplied RGBA, or None outside the canvas.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        self.pixmap.pixel(x, y).map(|c| [c.red(), c.green(), c.blue(), c.alpha()])
    }

    /// Writes the pixels to a PNG file (for tests and captures).
    pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        self.pixmap.save_png(path).map_err(|e| e.to_string())
    }

    /// Fills every pixel with a colour (the clip and the transform are ignored).
    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color.to_skia());
    }

    // ---- state ------------------------------------------------------------------------------------------------------

    /// Keeps the transform, the clip, the opacity and the blend, to bring them back with `restore`.
    pub fn save(&mut self) {
        self.saved.push(self.state.clone());
    }

    /// Brings back the state of the matching `save`.
    pub fn restore(&mut self) {
        if let Some(state) = self.saved.pop() {
            self.state = state;
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.state.transform = self.state.transform.pre_translate(dx, dy);
    }

    pub fn scale(&mut self, s: f32) {
        self.state.transform = self.state.transform.pre_scale(s, s);
    }

    /// Turns what is drawn next around the current origin, in degrees (clockwise on screen).
    pub fn rotate(&mut self, degrees: f32) {
        self.state.transform = self.state.transform.pre_concat(Transform::from_rotate(degrees));
    }

    /// Multiplies the opacity of what is drawn next.
    pub fn multiply_opacity(&mut self, factor: f32) {
        self.state.opacity *= factor.clamp(0.0, 1.0);
    }

    /// How what is drawn next (shapes, text, images, layers) mixes with what is there, until `restore`.
    pub fn set_blend(&mut self, blend: Blend) {
        self.state.blend = blend;
    }

    /// Draws only inside a rectangle (inside the current clip too) until `restore`.
    pub fn clip_rect(&mut self, r: Rect) {
        if let Some(p) = Path::round_rect(r, 0.0) {
            self.clip_path(&p);
        }
    }

    /// Draws only inside a path (inside the current clip too) until `restore`.
    pub fn clip_path(&mut self, path: &Path) {
        let t = self.to_pixels();
        let mask = match &self.state.clip {
            Some(current) => {
                let mut mask = (**current).clone();
                mask.intersect_path(&path.0, FillRule::Winding, true, t);
                mask
            }
            None => {
                let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height()).expect("a mask the canvas's size");
                mask.fill_path(&path.0, FillRule::Winding, true, t);
                mask
            }
        };
        self.state.clip = Some(Arc::new(mask));
    }

    /// Where a logical point lands on this canvas's logical surface under the current transform.
    pub fn map_point(&self, p: Point) -> Point {
        let mut points = [tiny_skia::Point::from_xy(p.x, p.y)];
        self.state.transform.map_points(&mut points);
        Point::new(points[0].x, points[0].y)
    }

    fn to_pixels(&self) -> Transform {
        Transform::from_scale(self.k, self.k).pre_concat(self.state.transform)
    }

    // ---- shapes -----------------------------------------------------------------------------------------------------

    pub fn fill_rect(&mut self, r: Rect, fill: &Fill) {
        if let Some(p) = Path::round_rect(r, 0.0) {
            self.fill_path(&p, fill);
        }
    }

    pub fn fill_round_rect(&mut self, r: Rect, radius: f32, fill: &Fill) {
        if let Some(p) = Path::round_rect(r, radius) {
            self.fill_path(&p, fill);
        }
    }

    pub fn stroke_round_rect(&mut self, r: Rect, radius: f32, stroke: &StrokeStyle) {
        if let Some(p) = Path::round_rect(r, radius) {
            self.stroke_path(&p, stroke);
        }
    }

    pub fn fill_circle(&mut self, center: Point, radius: f32, fill: &Fill) {
        if let Some(p) = Path::circle(center, radius) {
            self.fill_path(&p, fill);
        }
    }

    pub fn stroke_circle(&mut self, center: Point, radius: f32, stroke: &StrokeStyle) {
        if let Some(p) = Path::circle(center, radius) {
            self.stroke_path(&p, stroke);
        }
    }

    pub fn line(&mut self, a: Point, b: Point, stroke: &StrokeStyle) {
        if let Some(p) = Path::polyline(&[a, b]) {
            self.stroke_path(&p, stroke);
        }
    }

    pub fn fill_path(&mut self, path: &Path, fill: &Fill) {
        let Some(paint) = self.paint(fill) else { return };
        let t = self.to_pixels();
        self.pixmap.fill_path(&path.0, &paint, FillRule::Winding, t, self.state.clip.as_deref());
    }

    pub fn stroke_path(&mut self, path: &Path, stroke: &StrokeStyle) {
        let Some(paint) = self.paint(&Fill::Solid(stroke.color)) else { return };
        let t = self.to_pixels();
        let skia = Stroke {
            width: stroke.width,
            line_cap: if stroke.round { LineCap::Round } else { LineCap::Butt },
            line_join: if stroke.round { LineJoin::Round } else { LineJoin::Miter },
            dash: stroke.dash.as_ref().and_then(|d| StrokeDash::new(d.clone(), 0.0)),
            ..Stroke::default()
        };
        self.pixmap.stroke_path(&path.0, &paint, &skia, t, self.state.clip.as_deref());
    }

    /// A soft shadow under a rounded rectangle: the shape moved by `offset` and blurred by about `blur` logical pixels.
    /// The blurred shadow is made once for a shape, a blur and a colour, and placed again (turned with the transform)
    /// the next times.
    pub fn shadow(&mut self, r: Rect, radius: f32, blur: f32, offset: Point, color: Color) {
        let t = self.to_pixels();
        let s = scale_of(&t);
        let color = color.fade(self.state.opacity);
        if color.a == 0 || r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        let pad = (blur * 2.0 + 1.0) * s;
        let key = ShadowKey {
            w: (r.w * s).round() as u32,
            h: (r.h * s).round() as u32,
            radius: (radius * s).round() as u32,
            blur: (blur * s).round() as u32,
            color,
        };
        if !self.shadows.contains_key(&key) {
            let Some(pixmap) = blurred_shadow(key, pad) else { return };
            if self.shadows.len() >= 48 {
                self.shadows.clear();
            }
            self.shadows.insert(key, pixmap);
        }
        let pixmap = &self.shadows[&key];
        let place = t.pre_translate(r.x + offset.x, r.y + offset.y).pre_scale(1.0 / s, 1.0 / s).pre_translate(-pad, -pad);
        let paint = PixmapPaint { opacity: 1.0, blend_mode: BlendMode::SourceOver, quality: FilterQuality::Bilinear };
        draw_pixels(&mut self.pixmap, pixmap, &paint, place, self.state.clip.as_deref());
    }

    // ---- pictures ---------------------------------------------------------------------------------------------------

    /// Draws an image into a rectangle (stretched to it: use `draw_image_fit` to keep its proportions).
    pub fn draw_image(&mut self, image: &Image, dst: Rect, opacity: f32) {
        let base = self.to_pixels();
        let s = scale_of(&base);
        let (pw, ph) = ((dst.w * s).round().max(1.0) as u32, (dst.h * s).round().max(1.0) as u32);
        let paint = self.pixmap_paint(opacity);
        let mask = self.state.clip.clone();
        let target = &mut self.pixmap;
        image.pixels_for(pw, ph, |src| {
            let t = base.pre_translate(dst.x, dst.y).pre_scale(dst.w / src.width() as f32, dst.h / src.height() as f32);
            draw_pixels(target, src, &paint, t, mask.as_deref());
        });
    }

    /// Draws an image inside an area without deforming it.
    pub fn draw_image_fit(&mut self, image: &Image, area: Rect, fit: Fit, opacity: f32) {
        let dst = image.fit(area, fit);
        if fit == Fit::Cover {
            self.save();
            self.clip_rect(area);
            self.draw_image(image, dst, opacity);
            self.restore();
        } else {
            self.draw_image(image, dst, opacity);
        }
    }

    /// Draws a layer (see `layer`) with its top-left corner at `at`, scaled and turned (in degrees) around that corner.
    pub fn draw_layer(&mut self, layer: &Canvas, at: Point, scale: f32, rotation: f32, opacity: f32) {
        let t = self
            .to_pixels()
            .pre_translate(at.x, at.y)
            .pre_concat(Transform::from_rotate(rotation))
            .pre_scale(scale / layer.k, scale / layer.k);
        let paint = self.pixmap_paint(opacity);
        draw_pixels(&mut self.pixmap, &layer.pixmap, &paint, t, self.state.clip.as_deref());
    }

    /// Draws an SVG document into a rectangle, rendered at the pixels it takes there.
    #[cfg(feature = "svg")]
    pub fn draw_svg(&mut self, svg: &crate::svg::Svg, dst: Rect, opacity: f32) {
        let base = self.to_pixels();
        let s = scale_of(&base);
        let (pw, ph) = ((dst.w * s).round().max(1.0) as u32, (dst.h * s).round().max(1.0) as u32);
        let paint = self.pixmap_paint(opacity);
        let mask = self.state.clip.clone();
        let target = &mut self.pixmap;
        svg.pixels_for(pw, ph, |src| {
            let t = base.pre_translate(dst.x, dst.y).pre_scale(dst.w / src.width() as f32, dst.h / src.height() as f32);
            draw_pixels(target, src, &paint, t, mask.as_deref());
        });
    }

    // ---- text -------------------------------------------------------------------------------------------------------

    /// Draws a line of text whose line box starts at `y` (its top), placed on `x` by the style's alignment.
    /// Returns its width.
    pub fn text(&mut self, text: &str, x: f32, y: f32, style: &TextStyle) -> f32 {
        let width = style.measure(text);
        let left = match style.align {
            Align::Left => x,
            Align::Center => x - width / 2.0,
            Align::Right => x - width,
        };
        let baseline = y + style.baseline_offset();
        let glyphs = style.font.layout(text, style.size, style.letter_spacing);
        let scale = style.font.scale(style.size);
        if let Some((offset, color)) = style.shadow {
            if let Some((w, _)) = style.outline {
                self.glyphs(&glyphs, left + offset.x, baseline + offset.y, scale, color, Some(w));
            }
            self.glyphs(&glyphs, left + offset.x, baseline + offset.y, scale, color, None);
        }
        if let Some((w, color)) = style.outline {
            self.glyphs(&glyphs, left, baseline, scale, color, Some(w));
        }
        self.glyphs(&glyphs, left, baseline, scale, style.color, None);
        width
    }

    /// Draws a line of text along a circle, centred at `angle` (degrees, 0 at the top, clockwise), its baseline on the
    /// circle. Over the top (`clockwise`) the letters stand outwards and read left to right; along the bottom (not
    /// `clockwise`) they hang inwards and read left to right too: the two halves of a seal's ring.
    pub fn text_on_arc(&mut self, text: &str, center: Point, radius: f32, angle: f32, clockwise: bool, style: &TextStyle) {
        let radius = radius.max(1.0);
        let letters: Vec<(String, f32)> = text.chars().map(|c| {
            let s = c.to_string();
            let w = style.measure(&s) + style.letter_spacing;
            (s, w)
        }).collect();
        let total: f32 = letters.iter().map(|(_, w)| *w).sum::<f32>() - style.letter_spacing;
        let span = total / radius;
        let single = TextStyle { align: Align::Center, ..style.clone() };
        let mut along = 0.0;
        for (letter, w) in &letters {
            let middle = (along + (w - style.letter_spacing) / 2.0) / radius;
            along += w;
            let theta = if clockwise { angle.to_radians() - span / 2.0 + middle } else { angle.to_radians() + span / 2.0 - middle };
            let at = Point::new(center.x + radius * theta.sin(), center.y - radius * theta.cos());
            self.save();
            self.translate(at.x, at.y);
            self.rotate(if clockwise { theta.to_degrees() } else { theta.to_degrees() - 180.0 });
            let y = if clockwise { -single.baseline_offset() } else { -single.baseline_offset() + single.font.ascent(single.size) * 0.72 };
            self.text(letter, 0.0, y, &single);
            self.restore();
        }
    }

    /// Draws text wrapped to an area's width, line under line from its top, aligned inside it. Returns the height the
    /// lines took (they are not cut at the area's bottom).
    pub fn paragraph(&mut self, text: &str, area: Rect, style: &TextStyle) -> f32 {
        let lines = style.wrap(text, area.w);
        let x = match style.align {
            Align::Left => area.x,
            Align::Center => area.center().x,
            Align::Right => area.right(),
        };
        for (i, line) in lines.iter().enumerate() {
            self.text(line, x, area.y + i as f32 * style.line_px(), style);
        }
        lines.len() as f32 * style.line_px()
    }

    /// Fills (or, with `outline`, strokes by that many logical pixels outside) the glyphs of a line.
    fn glyphs(&mut self, glyphs: &[(Option<tiny_skia::Path>, f32)], left: f32, baseline: f32, scale: f32, color: Color, outline: Option<f32>) {
        let Some(paint) = self.paint(&Fill::Solid(color)) else { return };
        let base = self.to_pixels();
        let stroke = outline.map(|w| Stroke { width: 2.0 * w / scale, line_join: LineJoin::Round, line_cap: LineCap::Round, ..Stroke::default() });
        for (path, pen) in glyphs {
            let Some(path) = path else { continue };
            let t = base.pre_translate(left + pen, baseline).pre_scale(scale, scale);
            match &stroke {
                Some(stroke) => self.pixmap.stroke_path(path, &paint, stroke, t, self.state.clip.as_deref()),
                None => self.pixmap.fill_path(path, &paint, FillRule::Winding, t, self.state.clip.as_deref()),
            }
        }
    }

    // ---- paints -----------------------------------------------------------------------------------------------------

    fn paint(&self, fill: &Fill) -> Option<Paint<'static>> {
        let opacity = self.state.opacity;
        let mut paint = Paint { anti_alias: true, blend_mode: self.state.blend.mode(), ..Paint::default() };
        let stops = |stops: &[(f32, Color)]| -> Vec<GradientStop> {
            stops.iter().map(|(at, c)| GradientStop::new(*at, c.fade(opacity).to_skia())).collect()
        };
        match fill {
            Fill::Solid(c) => {
                if c.a == 0 || opacity <= 0.0 {
                    return None;
                }
                paint.set_color(c.fade(opacity).to_skia());
            }
            Fill::Linear { from, to, stops: list } => {
                let shader = LinearGradient::new(
                    tiny_skia::Point::from_xy(from.x, from.y),
                    tiny_skia::Point::from_xy(to.x, to.y),
                    stops(list),
                    SpreadMode::Pad,
                    Transform::identity(),
                );
                match shader {
                    Some(shader) => paint.shader = shader,
                    None => paint.set_color(list.first()?.1.fade(opacity).to_skia()),
                }
            }
            Fill::Radial { center, radius, stops: list } => {
                let c = tiny_skia::Point::from_xy(center.x, center.y);
                match RadialGradient::new(c, c, *radius, stops(list), SpreadMode::Pad, Transform::identity()) {
                    Some(shader) => paint.shader = shader,
                    None => paint.set_color(list.first()?.1.fade(opacity).to_skia()),
                }
            }
        }
        Some(paint)
    }

    fn pixmap_paint(&self, opacity: f32) -> PixmapPaint {
        PixmapPaint { opacity: (opacity * self.state.opacity).clamp(0.0, 1.0), blend_mode: self.state.blend.mode(), quality: FilterQuality::Bilinear }
    }
}

/// Draws pixels with a transform. A copy that only moves (no scale, no turn) is put on whole pixels and copied as it
/// is: a layer drawn at its own size stays as sharp as when it was drawn.
fn draw_pixels(target: &mut Pixmap, src: &Pixmap, paint: &PixmapPaint, t: Transform, mask: Option<&Mask>) {
    let only_moves = t.kx.abs() < 1e-5 && t.ky.abs() < 1e-5 && (t.sx - 1.0).abs() < 1e-4 && (t.sy - 1.0).abs() < 1e-4;
    if only_moves && mask.is_none() && paint.opacity >= 1.0 && paint.blend_mode == BlendMode::SourceOver {
        blend_over(target, src, t.tx.round() as i32, t.ty.round() as i32);
        return;
    }
    if only_moves {
        let paint = PixmapPaint { quality: FilterQuality::Nearest, ..*paint };
        target.draw_pixmap(t.tx.round() as i32, t.ty.round() as i32, src.as_ref(), &paint, Transform::identity(), mask);
    } else {
        target.draw_pixmap(0, 0, src.as_ref(), paint, t, mask);
    }
}

/// A rounded rectangle of a shadow's size (in pixels) blurred into a pixmap, with `pad` pixels around it for the blur.
fn blurred_shadow(key: ShadowKey, pad: f32) -> Option<Pixmap> {
    let pad_px = pad.ceil() as u32;
    let (w, h) = (key.w + 2 * pad_px, key.h + 2 * pad_px);
    let mut mask = Mask::new(w, h)?;
    let path = Path::round_rect(Rect::new(pad, pad, key.w as f32, key.h as f32), key.radius as f32)?;
    mask.fill_path(&path.0, FillRule::Winding, true, Transform::identity());
    let mut alpha = mask.data().to_vec();
    blur_mask(&mut alpha, w as usize, h as usize, (key.blur as f32 / 2.0).round().max(1.0) as usize);
    let mut pixmap = Pixmap::new(w, h)?;
    let c = key.color;
    let data = pixmap.data_mut();
    for (i, a) in alpha.into_iter().enumerate() {
        let a = a as u32 * c.a as u32 / 255;
        data[i * 4] = (c.r as u32 * a / 255) as u8;
        data[i * 4 + 1] = (c.g as u32 * a / 255) as u8;
        data[i * 4 + 2] = (c.b as u32 * a / 255) as u8;
        data[i * 4 + 3] = a as u8;
    }
    Some(pixmap)
}

/// Lays premultiplied pixels over others at a whole-pixel place: the plain "source over" of a copy that only moves, done
/// row by row (a few times faster than the general path, which samples through a shader).
fn blend_over(target: &mut Pixmap, src: &Pixmap, x: i32, y: i32) {
    let (tw, th) = (target.width() as i32, target.height() as i32);
    let (sw, sh) = (src.width() as i32, src.height() as i32);
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + sw).min(tw), (y + sh).min(th));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let pixels = (x1 - x0) as usize;
    let source = src.data();
    let dest = target.data_mut();
    for row in y0..y1 {
        let s_start = (((row - y) * sw + (x0 - x)) * 4) as usize;
        let d_start = ((row * tw + x0) * 4) as usize;
        let s = &source[s_start..s_start + pixels * 4];
        let d = &mut dest[d_start..d_start + pixels * 4];
        let mut i = 0;
        while i < pixels {
            let a = s[i * 4 + 3];
            if a == 255 {
                // A run of opaque pixels is copied whole.
                let start = i;
                while i < pixels && s[i * 4 + 3] == 255 {
                    i += 1;
                }
                d[start * 4..i * 4].copy_from_slice(&s[start * 4..i * 4]);
            } else if a == 0 {
                // A run of clear ones leaves the target as it is.
                while i < pixels && s[i * 4 + 3] == 0 {
                    i += 1;
                }
            } else {
                let keep = 255 - a as u32;
                for c in 0..4 {
                    let x = d[i * 4 + c] as u32 * keep + 128;
                    // x / 255, rounded, without a division.
                    d[i * 4 + c] = s[i * 4 + c].saturating_add(((x + (x >> 8)) >> 8) as u8);
                }
                i += 1;
            }
        }
    }
}

/// How many pixels a logical pixel takes under a transform (its average scale).
fn scale_of(t: &Transform) -> f32 {
    (t.sx * t.sy - t.kx * t.ky).abs().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::tests::test_font;

    const RED: Color = Color::rgb(255, 0, 0);
    const BLUE: Color = Color::rgb(0, 0, 255);

    #[test]
    fn logical_coordinates_are_drawn_at_the_canvas_pixels() {
        let mut c = Canvas::with_pixels(100.0, 50.0, 200, 100);
        assert_eq!(c.k(), 2.0);
        c.fill_rect(Rect::new(10.0, 10.0, 20.0, 10.0), &RED.into());
        assert_eq!(c.pixel(25, 25), Some([255, 0, 0, 255]), "logical (12.5, 12.5)");
        assert_eq!(c.pixel(61, 25), Some([0, 0, 0, 0]), "past the right edge at logical 30");
    }

    #[test]
    fn save_restore_brings_back_transform_clip_and_opacity() {
        let mut c = Canvas::new(40.0, 40.0);
        c.save();
        c.translate(20.0, 0.0);
        c.clip_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        c.multiply_opacity(0.5);
        c.fill_rect(Rect::new(0.0, 0.0, 20.0, 20.0), &RED.into());
        c.restore();
        assert_eq!(c.pixel(25, 5).map(|p| p[3]), Some(128), "translated, clipped, half opaque");
        assert_eq!(c.pixel(25, 15), Some([0, 0, 0, 0]), "outside the clip");
        c.fill_rect(Rect::new(0.0, 0.0, 5.0, 5.0), &BLUE.into());
        assert_eq!(c.pixel(2, 2), Some([0, 0, 255, 255]), "back to the start");
    }

    #[test]
    fn layers_are_drawn_where_asked_and_stay_sharp_at_their_size() {
        let base = Canvas::with_pixels(100.0, 100.0, 200, 200);
        let mut layer = base.layer(10.0, 10.0);
        assert_eq!((layer.pixel_width(), layer.pixel_height()), (20, 20));
        layer.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), &RED.into());
        let mut frame = base;
        frame.draw_layer(&layer, Point::new(30.0, 40.0), 1.0, 0.0, 1.0);
        assert_eq!(frame.pixel(60, 80), Some([255, 0, 0, 255]));
        assert_eq!(frame.pixel(79, 99), Some([255, 0, 0, 255]));
        assert_eq!(frame.pixel(80, 100), Some([0, 0, 0, 0]));
    }

    #[test]
    fn a_layer_keeps_the_exact_scale_of_its_canvas() {
        let frame = Canvas::with_pixels(1920.0, 1080.0, 2560, 1440);
        let layer = frame.layer(584.0, 333.0);
        assert_eq!(layer.k(), frame.k());
        assert!(layer.pixel_width() as f32 >= 584.0 * frame.k());
    }

    #[test]
    fn a_moved_layer_blends_like_the_general_path() {
        let base = Canvas::new(40.0, 40.0);
        let mut layer = base.layer(20.0, 20.0);
        layer.fill_circle(Point::new(10.0, 10.0), 9.0, &Color::rgba(200, 40, 90, 160).into());
        let mut fast = Canvas::new(40.0, 40.0);
        fast.clear(Color::hex(0x336699));
        let mut general = Canvas::new(40.0, 40.0);
        general.clear(Color::hex(0x336699));
        fast.draw_layer(&layer, Point::new(7.0, 9.0), 1.0, 0.0, 1.0);
        general.clip_rect(Rect::new(0.0, 0.0, 40.0, 40.0));
        general.draw_layer(&layer, Point::new(7.0, 9.0), 1.0, 0.0, 1.0);
        for (a, b) in fast.pixels().iter().zip(general.pixels()) {
            assert!((*a as i32 - *b as i32).abs() <= 2, "{a} vs {b}");
        }
    }

    #[test]
    fn gradients_go_from_one_stop_to_the_other() {
        let mut c = Canvas::new(100.0, 10.0);
        let fill = Fill::Linear { from: Point::new(0.0, 0.0), to: Point::new(100.0, 0.0), stops: vec![(0.0, RED), (1.0, BLUE)] };
        c.fill_rect(Rect::new(0.0, 0.0, 100.0, 10.0), &fill);
        let left = c.pixel(1, 5).unwrap();
        let right = c.pixel(98, 5).unwrap();
        assert!(left[0] > 240 && left[2] < 15);
        assert!(right[2] > 240 && right[0] < 15);
    }

    #[test]
    fn shadows_are_soft_and_offset() {
        let mut c = Canvas::new(100.0, 100.0);
        c.shadow(Rect::new(30.0, 30.0, 40.0, 40.0), 6.0, 8.0, Point::new(0.0, 6.0), Color::BLACK.fade(0.5));
        let inside = c.pixel(50, 56).unwrap()[3];
        let edge = c.pixel(50, 76).unwrap()[3];
        let far = c.pixel(50, 99).unwrap()[3];
        assert!(inside > 100, "dark under the middle: {inside}");
        assert!(edge > 0 && edge < inside, "soft at the edge: {edge}");
        assert_eq!(far, 0);
    }

    #[test]
    fn text_draws_inside_its_line_and_outlines_reach_further() {
        let font = test_font();
        let style = TextStyle::new(&font, 32.0, Color::BLACK);
        let mut plain = Canvas::new(300.0, 60.0);
        let width = plain.text("Sello", 10.0, 10.0, &style);
        assert!((width - style.measure("Sello")).abs() < 0.01);
        let inked = |c: &Canvas| (0..c.pixel_height()).flat_map(|y| (0..c.pixel_width()).map(move |x| (x, y))).filter(|&(x, y)| c.pixel(x, y).unwrap()[3] > 0).count();
        let plain_ink = inked(&plain);
        assert!(plain_ink > 200, "{plain_ink}");
        let mut outlined = Canvas::new(300.0, 60.0);
        outlined.text("Sello", 10.0, 10.0, &style.clone().outline(3.0, Color::WHITE));
        assert!(inked(&outlined) > plain_ink);
        // Nothing above the line box's top, nothing left of x.
        assert!((0..300).all(|x| plain.pixel(x, 5).unwrap()[3] == 0));
        assert!((0..60).all(|y| plain.pixel(5, y).unwrap()[3] == 0));
    }

    #[test]
    fn centred_and_right_aligned_text_end_where_asked() {
        let font = test_font();
        let mut c = Canvas::new(400.0, 60.0);
        let style = TextStyle::new(&font, 24.0, Color::BLACK).align(Align::Right);
        c.text("Velmira", 300.0, 10.0, &style);
        let rightmost = (0..400u32).rev().find(|&x| (0..60).any(|y| c.pixel(x, y).unwrap()[3] > 0)).unwrap();
        assert!((290..=301).contains(&rightmost), "{rightmost}");
    }

    #[test]
    fn images_scale_into_their_rectangle() {
        let img = Image::from_rgba(4, 4, [255u8, 0, 0, 255].repeat(16)).unwrap();
        let mut c = Canvas::new(50.0, 50.0);
        c.draw_image(&img, Rect::new(10.0, 10.0, 20.0, 20.0), 1.0);
        assert_eq!(c.pixel(20, 20), Some([255, 0, 0, 255]));
        assert_eq!(c.pixel(35, 20), Some([0, 0, 0, 0]));
        let big = Image::from_rgba(400, 400, [0u8, 0, 255, 255].repeat(160_000)).unwrap();
        c.draw_image(&big, Rect::new(0.0, 40.0, 10.0, 10.0), 1.0);
        assert_eq!(c.pixel(5, 45), Some([0, 0, 255, 255]), "a big image drawn small");
    }

    #[test]
    fn blends_multiply_erase_and_come_back_with_restore() {
        let mut c = Canvas::new(40.0, 10.0);
        c.fill_rect(Rect::new(0.0, 0.0, 40.0, 10.0), &Color::rgb(200, 100, 50).into());
        c.save();
        c.set_blend(Blend::Multiply);
        c.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), &Color::rgb(128, 128, 128).into());
        c.set_blend(Blend::Erase);
        c.fill_rect(Rect::new(10.0, 0.0, 10.0, 10.0), &Color::BLACK.into());
        c.restore();
        c.fill_rect(Rect::new(20.0, 0.0, 10.0, 10.0), &Color::rgb(0, 0, 255).into());
        let multiplied = c.pixel(5, 5).unwrap();
        assert!((multiplied[0] as i32 - 100).abs() <= 2 && (multiplied[1] as i32 - 50).abs() <= 2, "{multiplied:?}");
        assert_eq!(c.pixel(15, 5).unwrap()[3], 0, "erased");
        assert_eq!(c.pixel(25, 5), Some([0, 0, 255, 255]), "normal again after restore");
    }

    #[test]
    fn text_on_an_arc_stays_on_its_half_of_the_ring() {
        let font = test_font();
        let mut c = Canvas::new(200.0, 200.0);
        let style = TextStyle::new(&font, 16.0, Color::BLACK);
        c.text_on_arc("ARRIBA", Point::new(100.0, 100.0), 70.0, 0.0, true, &style);
        c.text_on_arc("ABAJO", Point::new(100.0, 100.0), 70.0, 180.0, false, &style);
        let inked = |y0: u32, y1: u32| (y0..y1).any(|y| (0..200).any(|x| c.pixel(x, y).unwrap()[3] > 0));
        assert!(inked(10, 45), "the top text sits round the top");
        assert!(inked(150, 190), "the bottom text sits round the bottom");
        assert!(!inked(60, 140), "nothing in the middle of the ring");
    }

    #[cfg(feature = "svg")]
    #[test]
    fn svg_is_drawn_at_the_canvas_pixels() {
        let svg = crate::svg::Svg::parse(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="#00ff00"/></svg>"##).unwrap();
        let mut c = Canvas::with_pixels(100.0, 100.0, 300, 300);
        c.draw_svg(&svg, Rect::new(10.0, 10.0, 20.0, 20.0), 1.0);
        assert_eq!(c.pixel(60, 60), Some([0, 255, 0, 255]), "the middle of the circle, at 3 pixels a logical pixel");
        assert_eq!(c.pixel(31, 31).map(|p| p[3]), Some(0), "the corner outside the circle");
    }
}
