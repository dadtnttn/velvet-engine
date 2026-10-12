use crate::assets::GameAssets;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::graphics::{fit_line, wrap_text, Canvas, FontSystem, Rect, SvgCache};
use crate::model::VisualCommandView;

/// Where and how commands land on the canvas at one point of the list.
#[derive(Clone, Copy, PartialEq)]
struct Pen {
    offset: [f32; 2],
    clip: Option<Rect>,
    opacity: f32,
    /// Canvas pixels per logical unit: the app lays out in its logical
    /// canvas (1280x720) and is drawn at the window's real resolution.
    k: f32,
}
impl Default for Pen {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            clip: None,
            opacity: 1.0,
            k: 1.0,
        }
    }
}
impl Pen {
    /// A logical length in canvas pixels.
    fn len(&self, v: f32) -> i32 {
        (v * self.k).round() as i32
    }
    /// A logical point (offset applied) in canvas pixels.
    fn x(&self, v: f32) -> i32 {
        ((v + self.offset[0]) * self.k).round() as i32
    }
    fn y(&self, v: f32) -> i32 {
        ((v + self.offset[1]) * self.k).round() as i32
    }
}
#[derive(Default)]
struct State {
    pen: Pen,
    offset_stack: Vec<[f32; 2]>,
    clip_stack: Vec<Option<Rect>>,
    opacity_stack: Vec<f32>,
}
/// Rounded at both edges, so shapes that touch in logical units still touch
/// at any scale.
fn rect(c: &VisualCommandView, s: &Pen) -> Rect {
    let (x0, y0) = (s.x(c.x), s.y(c.y));
    Rect::new(x0, y0, s.x(c.x + c.w) - x0, s.y(c.y + c.h) - y0)
}
fn color(v: i64) -> u32 {
    v as u32
}
fn ordered(commands: &[VisualCommandView]) -> Vec<&VisualCommandView> {
    let mut out = commands.iter().collect::<Vec<_>>();
    out.sort_by_key(|c| c.layer);
    out
}
fn control(state: &mut State, c: &VisualCommandView) -> bool {
    let pen = &mut state.pen;
    match c.op.as_str() {
        "translate_push" => {
            state.offset_stack.push(pen.offset);
            pen.offset[0] += c.x;
            pen.offset[1] += c.y;
            true
        }
        "transform_pop" => {
            if let Some(v) = state.offset_stack.pop() {
                pen.offset = v;
            }
            true
        }
        "clip_push" => {
            state.clip_stack.push(pen.clip);
            let next = rect(c, pen);
            pen.clip = match pen.clip {
                Some(old) => old.intersect(next),
                None => Some(next),
            };
            true
        }
        "clip_pop" => {
            pen.clip = state.clip_stack.pop().unwrap_or(None);
            true
        }
        "opacity_push" => {
            state.opacity_stack.push(pen.opacity);
            pen.opacity *= c.ratio.clamp(0.0, 1.0);
            true
        }
        "opacity_pop" => {
            pen.opacity = state.opacity_stack.pop().unwrap_or(1.0);
            true
        }
        _ => false,
    }
}

pub fn action_at(commands: &[VisualCommandView], cursor: [f32; 2]) -> Option<String> {
    let mut state = State::default();
    let mut hit = None;
    for c in ordered(commands) {
        if control(&mut state, c) {
            continue;
        }
        if c.action.is_empty() {
            continue;
        }
        let area = rect(c, &state.pen);
        let inside_clip = match state.pen.clip {
            Some(clip) => clip.contains(cursor[0], cursor[1]),
            None => true,
        };
        if area.contains(cursor[0], cursor[1]) && inside_clip {
            hit = Some(c.action.clone());
        }
    }
    hit
}

/// What the canvas holds from the frame drawn last, so the next frame only
/// redraws what changed (most frames move a few things on a still screen).
#[derive(Default)]
pub struct Damage {
    /// Signature and reach (canvas pixels) of every command drawn last.
    last: Vec<(u64, Option<Rect>)>,
    size: (usize, usize),
    scale: f32,
    /// Redraw everything next frame.
    stale: bool,
    /// What the last frame redrew: `None` is the whole canvas.
    pub redrawn: Option<Vec<Rect>>,
}
impl Damage {
    /// Fonts or anything else outside the commands changed.
    pub fn invalidate(&mut self) {
        self.stale = true;
    }
}

/// Most separate areas redrawn in one frame before they merge into one.
const MAX_REGIONS: usize = 8;

/// A drawing command with the pen it is drawn with.
struct Item<'a> {
    c: &'a VisualCommandView,
    pen: Pen,
    sig: u64,
    reach: Option<Rect>,
}

/// Draw `commands` (logical units) into `canvas`, `scale` canvas pixels per
/// logical unit; `cursor` is in logical units. Only what changed since the
/// last frame is redrawn (`damage` remembers it).
#[allow(clippy::too_many_arguments)]
pub fn render(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    svg: &mut SvgCache,
    assets: &GameAssets,
    commands: &[VisualCommandView],
    time: f32,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
    scale: f32,
    damage: &mut Damage,
) {
    let cursor = cursor.map(|p| [p[0] * scale, p[1] * scale]);
    let mut state = State {
        pen: Pen {
            k: scale,
            ..Pen::default()
        },
        ..State::default()
    };
    let mut items = Vec::with_capacity(commands.len());
    for c in ordered(commands) {
        if !control(&mut state, c) {
            items.push(Item {
                c,
                pen: state.pen,
                sig: signature(c, &state.pen, time, cursor, mouse_down),
                reach: None,
            });
        }
    }
    let whole = canvas.bounds();
    let size = (canvas.width, canvas.height);
    let fresh = damage.stale || damage.size != size || damage.scale != scale;
    let regions = if fresh {
        None
    } else {
        changed_regions(&mut items, &damage.last, fonts, whole)
    };
    match &regions {
        None => {
            for item in &mut items {
                item.reach = reach(item.c, &item.pen, fonts, whole);
                draw(
                    canvas, fonts, svg, assets, item, None, time, cursor, mouse_down,
                );
            }
        }
        Some(regions) => {
            for &region in regions {
                for item in items.iter().filter(|i| touches(i.reach, region)) {
                    draw(
                        canvas,
                        fonts,
                        svg,
                        assets,
                        item,
                        Some(region),
                        time,
                        cursor,
                        mouse_down,
                    );
                }
            }
        }
    }
    canvas.set_clip(None);
    canvas.set_opacity(1.0);
    damage.last = items.iter().map(|i| (i.sig, i.reach)).collect();
    damage.size = size;
    damage.scale = scale;
    damage.stale = false;
    damage.redrawn = regions;
}

fn touches(reach: Option<Rect>, region: Rect) -> bool {
    reach.is_some_and(|r| r.intersect(region).is_some())
}

/// The areas to redraw, or `None` to redraw everything. Fills in the reach
/// of every item (kept from the last frame when it did not change).
fn changed_regions(
    items: &mut [Item],
    last: &[(u64, Option<Rect>)],
    fonts: &FontSystem,
    whole: Rect,
) -> Option<Vec<Rect>> {
    let (old, new) = (last.len(), items.len());
    let mut head = 0;
    while head < old.min(new) && last[head].0 == items[head].sig {
        head += 1;
    }
    let mut tail = 0;
    while tail < (old - head).min(new - head) && last[old - 1 - tail].0 == items[new - 1 - tail].sig
    {
        tail += 1;
    }
    let mut dirty: Vec<Rect> = Vec::new();
    let mut add = |reach: Option<Rect>| {
        if let Some(r) = reach {
            dirty.push(r);
        }
    };
    for (i, item) in items.iter_mut().enumerate() {
        // Same position in both lists, same signature: same pixels.
        let before = if i < head {
            Some(i)
        } else if i >= new - tail {
            Some(old - (new - i))
        } else if old == new {
            Some(i)
        } else {
            None
        };
        match before {
            Some(j) if last[j].0 == item.sig => item.reach = last[j].1,
            _ => {
                item.reach = reach(item.c, &item.pen, fonts, whole);
                add(item.reach);
                if let Some(j) = before {
                    add(last[j].1);
                }
            }
        }
    }
    // Commands gone from the middle of the list.
    if old != new {
        for entry in &last[head..old - tail] {
            add(entry.1);
        }
    }
    merge(dirty, whole)
}

/// Overlapping areas become one; too many or too big means redraw it all.
fn merge(mut dirty: Vec<Rect>, whole: Rect) -> Option<Vec<Rect>> {
    let mut merged: Vec<Rect> = Vec::new();
    while let Some(mut r) = dirty.pop() {
        let mut grown = true;
        while grown {
            grown = false;
            if let Some(at) = merged.iter().position(|m| m.intersect(r).is_some()) {
                r = union(r, merged.swap_remove(at));
                grown = true;
            }
        }
        merged.push(r);
    }
    if merged.len() > MAX_REGIONS {
        let all = merged.iter().copied().reduce(union)?;
        merged = vec![all];
    }
    let area: i64 = merged.iter().map(|r| i64::from(r.w) * i64::from(r.h)).sum();
    let limit = i64::from(whole.w) * i64::from(whole.h) * 3 / 5;
    (area <= limit).then_some(merged)
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// Everything that decides a command's pixels: its fields, where it lands
/// in canvas pixels, its clip and opacity, hover for buttons and the frame
/// of animated sprites.
fn signature(
    c: &VisualCommandView,
    pen: &Pen,
    time: f32,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) -> u64 {
    let mut h = DefaultHasher::new();
    let area = rect(c, pen);
    (c.op.as_str(), area.x, area.y, area.w, area.h).hash(&mut h);
    (pen.len(c.w), pen.len(c.h), pen.x(c.x2), pen.y(c.y2)).hash(&mut h);
    pen.clip.map(|r| (r.x, r.y, r.w, r.h)).hash(&mut h);
    let floats = [
        pen.opacity,
        pen.k,
        c.rx,
        c.ry,
        c.radius,
        c.thickness,
        c.fps,
        c.size,
        c.line_height,
        c.ratio,
        c.max_width,
    ];
    floats.map(f32::to_bits).hash(&mut h);
    (
        c.color,
        c.stroke,
        c.background,
        c.fill,
        c.frame_width,
        c.frame,
    )
        .hash(&mut h);
    (c.source_x, c.source_y, c.source_w, c.source_h).hash(&mut h);
    (c.tile_w, c.tile_h, c.source_border, c.destination_border).hash(&mut h);
    (c.flip_x, c.max_lines).hash(&mut h);
    let texts = [
        &c.asset,
        &c.hover_asset,
        &c.pressed_asset,
        &c.fit,
        &c.text,
        &c.font,
        &c.align,
        &c.icon,
        &c.overflow,
    ];
    texts.hash(&mut h);
    match c.op.as_str() {
        "button" => {
            let hovered = cursor.is_some_and(|p| area.contains(p[0], p[1]));
            (hovered, hovered && mouse_down).hash(&mut h);
        }
        "sprite_frame" if c.fps > 0.0 => ((time * c.fps).floor() as i64).hash(&mut h),
        _ => {}
    }
    h.finish()
}

/// The canvas pixels a command can touch (clip applied), or `None`.
fn reach(c: &VisualCommandView, pen: &Pen, fonts: &FontSystem, whole: Rect) -> Option<Rect> {
    let area = rect(c, pen);
    let around =
        |x: i32, y: i32, rx: i32, ry: i32| Rect::new(x - rx, y - ry, 2 * rx + 1, 2 * ry + 1);
    let r = match c.op.as_str() {
        "clear" => return Some(whole),
        "circle" => {
            let r = pen.len(c.radius).max(1);
            around(pen.x(c.x), pen.y(c.y), r, r)
        }
        "ellipse" | "alpha_ellipse" => around(
            pen.x(c.x),
            pen.y(c.y),
            pen.len(c.rx).max(1),
            pen.len(c.ry).max(1),
        ),
        "line" => {
            let t = pen.len(c.thickness).max(1) / 2 + 1;
            let (x0, y0, x1, y1) = (pen.x(c.x), pen.y(c.y), pen.x(c.x2), pen.y(c.y2));
            let (left, top) = (x0.min(x1) - t, y0.min(y1) - t);
            Rect::new(
                left,
                top,
                x0.max(x1) + t + 1 - left,
                y0.max(y1) + t + 1 - top,
            )
        }
        "text" => {
            let f = fonts.named(&c.font);
            let (line, size) = fit_line(
                &f,
                &c.text,
                c.size.max(8.0) * pen.k,
                c.max_width * pen.k,
                &c.overflow,
            );
            let w = Canvas::measure_text(&f, &line, size).ceil() as i32;
            let x = match c.align.as_str() {
                "center" => pen.x(c.x) - w / 2,
                "right" => pen.x(c.x) - w,
                _ => pen.x(c.x),
            };
            let rim = outline(c, pen).map_or(0, |(_, radius)| radius.ceil() as i32 + 1);
            let m = size.ceil() as i32 + 1 + rim;
            Rect::new(x - m, pen.y(c.y) - 2 * m, w + 2 * m, 3 * m)
        }
        "text_wrap" => {
            let size = c.size.max(8.0) * pen.k;
            let lines = wrap_text(
                &fonts.named(&c.font),
                &c.text,
                size,
                pen.len(c.w) as f32,
                c.max_lines.max(0) as usize,
            );
            let m = size.ceil() as i32 + 1;
            let below = (lines.len().max(1) as i32 - 1) * pen.len(c.line_height.max(10.0));
            Rect::new(
                pen.x(c.x) - m,
                pen.y(c.y) - m,
                pen.len(c.w) + 2 * m,
                below + 3 * m,
            )
        }
        "button" | "keycap" => {
            let m = pen.len(c.size.max(8.0));
            Rect::new(area.x - m, area.y - m, area.w + 2 * m, area.h + 2 * m)
        }
        _ => area,
    };
    let r = whole.intersect(r)?;
    match pen.clip {
        Some(clip) => clip.intersect(r),
        None => Some(r),
    }
}

/// Draw one command, inside `region` when only part of the canvas is
/// redrawn.
#[allow(clippy::too_many_arguments)]
fn draw(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    svg: &mut SvgCache,
    assets: &GameAssets,
    item: &Item,
    region: Option<Rect>,
    time: f32,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) {
    let (c, pen) = (item.c, &item.pen);
    let clip = match (pen.clip, region) {
        (Some(clip), Some(region)) => match clip.intersect(region) {
            Some(both) => Some(both),
            None => return,
        },
        (clip, None) => clip,
        (None, region) => region,
    };
    canvas.set_clip(clip);
    canvas.set_opacity(pen.opacity);
    match c.op.as_str() {
        "clear" => match region {
            Some(region) => canvas.clear_area(region, color(c.color)),
            None => canvas.clear(color(c.color)),
        },
        "svg" => {
            let area = rect(c, pen);
            let (w, h) = (area.w.max(0) as u32, area.h.max(0) as u32);
            match svg.raster_spans(&c.text, w, h) {
                Some((image, Some(spans))) if !c.flip_x => canvas.image_spans(image, spans, area),
                Some((image, _)) => {
                    canvas.image_region(image, 0, 0, image.width, image.height, area, c.flip_x)
                }
                None => {}
            }
        }
        _ => render_command(canvas, fonts, assets, c, pen, time, cursor, mouse_down),
    }
}

fn render_command(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    c: &VisualCommandView,
    s: &Pen,
    time: f32,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) {
    let area = rect(c, s);
    match c.op.as_str() {
        "rect" => canvas.rect(area, color(c.color)),
        "alpha_rect" => canvas.alpha_rect(area, color(c.color)),
        "rounded_rect" => canvas.rounded_rect(area, s.len(c.radius), color(c.color)),
        "border" => canvas.border(area, s.len(c.thickness).max(1), color(c.color)),
        "circle" => canvas.circle(s.x(c.x), s.y(c.y), s.len(c.radius).max(1), color(c.color)),
        "ellipse" => canvas.ellipse(
            s.x(c.x),
            s.y(c.y),
            s.len(c.rx).max(1),
            s.len(c.ry).max(1),
            color(c.color),
        ),
        "alpha_ellipse" => canvas.alpha_ellipse(
            s.x(c.x),
            s.y(c.y),
            s.len(c.rx).max(1),
            s.len(c.ry).max(1),
            color(c.color),
        ),
        "line" => canvas.round_line(
            s.x(c.x),
            s.y(c.y),
            s.x(c.x2),
            s.y(c.y2),
            s.len(c.thickness).max(1),
            color(c.color),
        ),
        "sprite" => sprite(canvas, assets, c, area),
        "sprite_frame" => sprite_frame(canvas, assets, c, area, time),
        "tile_fill" => tile_fill(canvas, assets, c, area, s),
        "nine_slice" => nine_slice(canvas, assets, c, area, s),
        "text" => text(canvas, fonts, c, s),
        "text_wrap" => text_wrap(canvas, fonts, c, s),
        "panel" => {
            canvas.alpha_rect(area, color(c.color));
            canvas.border(area, s.len(c.thickness).max(1), color(c.stroke));
        }
        "bar" => bar(canvas, c, area, s),
        "button" => button(canvas, fonts, assets, c, area, s, cursor, mouse_down),
        "keycap" => keycap(canvas, fonts, assets, c, area, s),
        _ => {}
    }
}
fn sprite(canvas: &mut Canvas, assets: &GameAssets, c: &VisualCommandView, area: Rect) {
    let Some(image) = assets.named(&c.asset) else {
        return;
    };
    if c.fit == "content" {
        canvas.image_fit_content(image, area)
    } else {
        canvas.image_fit(image, area)
    }
}
fn sprite_frame(
    canvas: &mut Canvas,
    assets: &GameAssets,
    c: &VisualCommandView,
    area: Rect,
    time: f32,
) {
    let Some(image) = assets.named(&c.asset) else {
        return;
    };
    let animated = if c.fps > 0.0 {
        (time * c.fps).floor() as i64
    } else {
        0
    };
    canvas.sprite_frame(
        image,
        c.frame_width.max(1) as usize,
        (c.frame + animated).max(0) as usize,
        area,
        c.flip_x,
    )
}
fn tile_fill(canvas: &mut Canvas, assets: &GameAssets, c: &VisualCommandView, area: Rect, s: &Pen) {
    let Some(image) = assets.named(&c.asset) else {
        return;
    };
    let tw = s.len(c.tile_w.max(1) as f32).max(1);
    let th = s.len(c.tile_h.max(1) as f32).max(1);
    let mut y = area.y;
    while y < area.y + area.h {
        let mut x = area.x;
        while x < area.x + area.w {
            canvas.image_region(
                image,
                c.source_x.max(0) as usize,
                c.source_y.max(0) as usize,
                c.source_w.max(1) as usize,
                c.source_h.max(1) as usize,
                Rect::new(
                    x,
                    y,
                    tw.min(area.x + area.w - x),
                    th.min(area.y + area.h - y),
                ),
                false,
            );
            x += tw;
        }
        y += th;
    }
}
fn nine_slice(
    canvas: &mut Canvas,
    assets: &GameAssets,
    c: &VisualCommandView,
    area: Rect,
    s: &Pen,
) {
    if let Some(image) = assets.named(&c.asset) {
        canvas.nine_slice(
            image,
            c.source_border.max(0) as usize,
            area,
            s.len(c.destination_border.max(0) as f32),
        )
    }
}
/// Outline of a `text` command: color and radius in canvas pixels.
fn outline(c: &VisualCommandView, s: &Pen) -> Option<(u32, f32)> {
    let stroke = color(c.stroke);
    (c.thickness > 0.0 && stroke >> 24 != 0).then_some((stroke, c.thickness * s.k))
}
fn text(canvas: &mut Canvas, fonts: &FontSystem, c: &VisualCommandView, s: &Pen) {
    let f = fonts.named(&c.font);
    let (x, y) = (s.x(c.x), s.y(c.y));
    let (line, size) = fit_line(
        &f,
        &c.text,
        c.size.max(8.0) * s.k,
        c.max_width * s.k,
        &c.overflow,
    );
    let x = match c.align.as_str() {
        "center" => x - Canvas::measure_text(&f, &line, size).round() as i32 / 2,
        "right" => x - Canvas::measure_text(&f, &line, size).round() as i32,
        _ => x,
    };
    match outline(c, s) {
        Some((stroke, radius)) => {
            canvas.text_line_outlined(&f, &line, x, y, size, color(c.color), stroke, radius)
        }
        None => canvas.text_line(&f, &line, x, y, size, color(c.color)),
    };
}
fn text_wrap(canvas: &mut Canvas, fonts: &FontSystem, c: &VisualCommandView, s: &Pen) {
    canvas.text_wrapped(
        &fonts.named(&c.font),
        &c.text,
        s.x(c.x),
        s.y(c.y),
        s.len(c.w),
        c.size.max(8.0) * s.k,
        s.len(c.line_height.max(10.0)),
        color(c.color),
        c.max_lines.max(0) as usize,
    );
}
fn bar(canvas: &mut Canvas, c: &VisualCommandView, area: Rect, s: &Pen) {
    canvas.alpha_rect(area, color(c.background));
    let pad = s.len(2.0);
    let inner = Rect::new(
        area.x + pad,
        area.y + pad,
        (area.w - 2 * pad).max(0),
        (area.h - 2 * pad).max(0),
    );
    let width = (inner.w as f32 * c.ratio.clamp(0.0, 1.0)).round() as i32;
    if width > 0 {
        canvas.rect(Rect::new(inner.x, inner.y, width, inner.h), color(c.fill));
    }
    canvas.border(area, s.len(c.thickness).max(1), color(c.stroke));
}
#[allow(clippy::too_many_arguments)]
fn button(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    c: &VisualCommandView,
    area: Rect,
    s: &Pen,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) {
    let hovered = cursor.is_some_and(|p| area.contains(p[0], p[1]));
    let pressed = hovered && mouse_down;
    let id = if pressed && !c.pressed_asset.is_empty() {
        &c.pressed_asset
    } else if hovered && !c.hover_asset.is_empty() {
        &c.hover_asset
    } else {
        &c.asset
    };
    if !id.is_empty() {
        if let Some(image) = assets.named(id) {
            canvas.image_fit(image, area)
        } else {
            fallback_button(canvas, c, area, s, hovered)
        }
    } else {
        fallback_button(canvas, c, area, s, hovered)
    }
    if !c.icon.is_empty() {
        if let Some(icon) = assets.named(&c.icon) {
            let side = (area.h - s.len(24.0)).max(1);
            canvas.image_fit(
                icon,
                Rect::new(area.x + s.len(14.0), area.y + s.len(12.0), side, side),
            );
        }
    }
    // Labels never spill out of the button: shrink first, then ellipsis.
    let font = fonts.named(&c.font);
    let icon_room = if c.icon.is_empty() { 0 } else { area.h };
    let room = if c.max_width > 0.0 {
        c.max_width * s.k
    } else {
        (area.w - s.len(16.0) - 2 * icon_room).max(8) as f32
    };
    let overflow = if c.overflow.is_empty() {
        "shrink"
    } else {
        c.overflow.as_str()
    };
    let (label, size) = fit_line(&font, &c.text, c.size.max(8.0) * s.k, room, overflow);
    let push = if pressed { s.len(1.0).max(1) } else { 0 };
    canvas.centered_text(
        &font,
        &label,
        area.x + area.w / 2 + push,
        area.y + area.h / 2 + (size * 0.4) as i32 + push,
        size,
        color(c.color),
    );
}
fn fallback_button(canvas: &mut Canvas, c: &VisualCommandView, area: Rect, s: &Pen, hovered: bool) {
    let fill = if hovered && c.fill != 0 {
        c.fill
    } else {
        c.background
    };
    canvas.alpha_rect(area, color(fill));
    canvas.border(area, s.len(c.thickness).max(1), color(c.stroke));
}
fn keycap(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    c: &VisualCommandView,
    area: Rect,
    s: &Pen,
) {
    if !c.asset.is_empty() {
        if let Some(image) = assets.named(&c.asset) {
            canvas.image_fit(image, area)
        } else {
            fallback_button(canvas, c, area, s, false)
        }
    } else {
        fallback_button(canvas, c, area, s, false)
    }
    let size = c.size.max(8.0) * s.k;
    canvas.centered_text(
        &fonts.named(&c.font),
        &c.text,
        area.x + area.w / 2,
        area.y + area.h / 2 + (size * 0.4) as i32,
        size,
        color(c.color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(op: &str, layer: i64) -> VisualCommandView {
        VisualCommandView {
            op: op.into(),
            layer,
            ..Default::default()
        }
    }

    #[test]
    fn hit_testing_respects_translate_and_clip() {
        let mut translate = command("translate_push", 0);
        translate.x = 100.0;
        translate.y = 50.0;
        let mut clip = command("clip_push", 1);
        clip.w = 40.0;
        clip.h = 40.0;
        let mut button = command("button", 2);
        button.x = 5.0;
        button.y = 5.0;
        button.w = 20.0;
        button.h = 20.0;
        button.action = "go".into();
        let clip_pop = command("clip_pop", 3);
        let transform_pop = command("transform_pop", 4);
        let commands = vec![translate, clip, button, clip_pop, transform_pop];
        assert_eq!(action_at(&commands, [110.0, 60.0]).as_deref(), Some("go"));
        assert_eq!(action_at(&commands, [130.0, 80.0]), None);
    }
}
