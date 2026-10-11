use crate::assets::GameAssets;
use crate::graphics::{fit_line, Canvas, FontSystem, Rect, SvgCache};
use crate::model::VisualCommandView;

#[derive(Clone)]
struct State {
    offset: [f32; 2],
    clip: Option<Rect>,
    opacity: f32,
    offset_stack: Vec<[f32; 2]>,
    clip_stack: Vec<Option<Rect>>,
    opacity_stack: Vec<f32>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            clip: None,
            opacity: 1.0,
            offset_stack: Vec::new(),
            clip_stack: Vec::new(),
            opacity_stack: Vec::new(),
        }
    }
}
fn rect(c: &VisualCommandView, s: &State) -> Rect {
    Rect::new(
        (c.x + s.offset[0]).round() as i32,
        (c.y + s.offset[1]).round() as i32,
        c.w.round() as i32,
        c.h.round() as i32,
    )
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
    match c.op.as_str() {
        "translate_push" => {
            state.offset_stack.push(state.offset);
            state.offset[0] += c.x;
            state.offset[1] += c.y;
            true
        }
        "transform_pop" => {
            if let Some(v) = state.offset_stack.pop() {
                state.offset = v;
            }
            true
        }
        "clip_push" => {
            state.clip_stack.push(state.clip);
            let next = rect(c, state);
            state.clip = match state.clip {
                Some(old) => old.intersect(next),
                None => Some(next),
            };
            true
        }
        "clip_pop" => {
            state.clip = state.clip_stack.pop().unwrap_or(None);
            true
        }
        "opacity_push" => {
            state.opacity_stack.push(state.opacity);
            state.opacity *= c.ratio.clamp(0.0, 1.0);
            true
        }
        "opacity_pop" => {
            state.opacity = state.opacity_stack.pop().unwrap_or(1.0);
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
        let area = rect(c, &state);
        let inside_clip = match state.clip {
            Some(clip) => clip.contains(cursor[0], cursor[1]),
            None => true,
        };
        if area.contains(cursor[0], cursor[1]) && inside_clip {
            hit = Some(c.action.clone());
        }
    }
    hit
}

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
) {
    let mut state = State::default();
    for c in ordered(commands) {
        if control(&mut state, c) {
            canvas.set_clip(state.clip);
            canvas.set_opacity(state.opacity);
            continue;
        }
        canvas.set_clip(state.clip);
        canvas.set_opacity(state.opacity);
        if c.op == "svg" {
            let area = rect(c, &state);
            if let Some(image) = svg.raster(&c.text, area.w.max(0) as u32, area.h.max(0) as u32) {
                canvas.image_region(image, 0, 0, image.width, image.height, area, c.flip_x);
            }
            continue;
        }
        render_command(canvas, fonts, assets, c, &state, time, cursor, mouse_down);
    }
    canvas.set_clip(None);
    canvas.set_opacity(1.0);
}

fn render_command(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    c: &VisualCommandView,
    s: &State,
    time: f32,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) {
    let area = rect(c, s);
    match c.op.as_str() {
        "clear" => canvas.clear(color(c.color)),
        "rect" => canvas.rect(area, color(c.color)),
        "alpha_rect" => canvas.alpha_rect(area, color(c.color)),
        "rounded_rect" => canvas.rounded_rect(area, c.radius.round() as i32, color(c.color)),
        "border" => canvas.border(area, c.thickness.round().max(1.0) as i32, color(c.color)),
        "circle" => canvas.circle(
            (c.x + s.offset[0]).round() as i32,
            (c.y + s.offset[1]).round() as i32,
            c.radius.round().max(1.0) as i32,
            color(c.color),
        ),
        "ellipse" => canvas.ellipse(
            (c.x + s.offset[0]).round() as i32,
            (c.y + s.offset[1]).round() as i32,
            c.rx.round().max(1.0) as i32,
            c.ry.round().max(1.0) as i32,
            color(c.color),
        ),
        "alpha_ellipse" => canvas.alpha_ellipse(
            (c.x + s.offset[0]).round() as i32,
            (c.y + s.offset[1]).round() as i32,
            c.rx.round().max(1.0) as i32,
            c.ry.round().max(1.0) as i32,
            color(c.color),
        ),
        "line" => canvas.round_line(
            (c.x + s.offset[0]).round() as i32,
            (c.y + s.offset[1]).round() as i32,
            (c.x2 + s.offset[0]).round() as i32,
            (c.y2 + s.offset[1]).round() as i32,
            c.thickness.round().max(1.0) as i32,
            color(c.color),
        ),
        "sprite" => sprite(canvas, assets, c, area),
        "sprite_frame" => sprite_frame(canvas, assets, c, area, time),
        "tile_fill" => tile_fill(canvas, assets, c, area),
        "nine_slice" => nine_slice(canvas, assets, c, area),
        "text" => text(canvas, fonts, c, s),
        "text_wrap" => text_wrap(canvas, fonts, c, s),
        "panel" => {
            canvas.alpha_rect(area, color(c.color));
            canvas.border(area, c.thickness.round().max(1.0) as i32, color(c.stroke));
        }
        "bar" => bar(canvas, c, area),
        "button" => button(canvas, fonts, assets, c, area, cursor, mouse_down),
        "keycap" => keycap(canvas, fonts, assets, c, area),
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
fn tile_fill(canvas: &mut Canvas, assets: &GameAssets, c: &VisualCommandView, area: Rect) {
    let Some(image) = assets.named(&c.asset) else {
        return;
    };
    let tw = c.tile_w.max(1) as i32;
    let th = c.tile_h.max(1) as i32;
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
fn nine_slice(canvas: &mut Canvas, assets: &GameAssets, c: &VisualCommandView, area: Rect) {
    if let Some(image) = assets.named(&c.asset) {
        canvas.nine_slice(
            image,
            c.source_border.max(0) as usize,
            area,
            c.destination_border.max(0) as i32,
        )
    }
}
fn text(canvas: &mut Canvas, fonts: &FontSystem, c: &VisualCommandView, s: &State) {
    let f = fonts.named(&c.font);
    let x = (c.x + s.offset[0]).round() as i32;
    let y = (c.y + s.offset[1]).round() as i32;
    let (line, size) = fit_line(&f, &c.text, c.size.max(8.0), c.max_width, &c.overflow);
    match c.align.as_str() {
        "center" => canvas.centered_text(&f, &line, x, y, size, color(c.color)),
        "right" => {
            let width = Canvas::measure_text(&f, &line, size).round() as i32;
            canvas.text_line(&f, &line, x - width, y, size, color(c.color));
        }
        _ => {
            canvas.text_line(&f, &line, x, y, size, color(c.color));
        }
    }
}
fn text_wrap(canvas: &mut Canvas, fonts: &FontSystem, c: &VisualCommandView, s: &State) {
    canvas.text_wrapped(
        &fonts.named(&c.font),
        &c.text,
        (c.x + s.offset[0]).round() as i32,
        (c.y + s.offset[1]).round() as i32,
        c.w.round() as i32,
        c.size.max(8.0),
        c.line_height.round().max(10.0) as i32,
        color(c.color),
        c.max_lines.max(0) as usize,
    );
}
fn bar(canvas: &mut Canvas, c: &VisualCommandView, area: Rect) {
    canvas.alpha_rect(area, color(c.background));
    let inner = Rect::new(
        area.x + 2,
        area.y + 2,
        (area.w - 4).max(0),
        (area.h - 4).max(0),
    );
    let width = (inner.w as f32 * c.ratio.clamp(0.0, 1.0)).round() as i32;
    if width > 0 {
        canvas.rect(Rect::new(inner.x, inner.y, width, inner.h), color(c.fill));
    }
    canvas.border(area, c.thickness.round().max(1.0) as i32, color(c.stroke));
}
fn button(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    c: &VisualCommandView,
    area: Rect,
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
            fallback_button(canvas, c, area, hovered)
        }
    } else {
        fallback_button(canvas, c, area, hovered)
    }
    if !c.icon.is_empty() {
        if let Some(icon) = assets.named(&c.icon) {
            canvas.image_fit(
                icon,
                Rect::new(
                    area.x + 14,
                    area.y + 12,
                    (area.h - 24).max(1),
                    (area.h - 24).max(1),
                ),
            );
        }
    }
    // Labels never spill out of the button: shrink first, then ellipsis.
    let font = fonts.named(&c.font);
    let icon_room = if c.icon.is_empty() { 0 } else { area.h };
    let room = if c.max_width > 0.0 {
        c.max_width
    } else {
        (area.w - 16 - 2 * icon_room).max(8) as f32
    };
    let overflow = if c.overflow.is_empty() {
        "shrink"
    } else {
        c.overflow.as_str()
    };
    let (label, size) = fit_line(&font, &c.text, c.size.max(8.0), room, overflow);
    canvas.centered_text(
        &font,
        &label,
        area.x + area.w / 2 + i32::from(pressed),
        area.y + area.h / 2 + (size * 0.4) as i32 + i32::from(pressed),
        size,
        color(c.color),
    );
}
fn fallback_button(canvas: &mut Canvas, c: &VisualCommandView, area: Rect, hovered: bool) {
    let fill = if hovered && c.fill != 0 {
        c.fill
    } else {
        c.background
    };
    canvas.alpha_rect(area, color(fill));
    canvas.border(area, c.thickness.round().max(1.0) as i32, color(c.stroke));
}
fn keycap(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    c: &VisualCommandView,
    area: Rect,
) {
    if !c.asset.is_empty() {
        if let Some(image) = assets.named(&c.asset) {
            canvas.image_fit(image, area)
        } else {
            fallback_button(canvas, c, area, false)
        }
    } else {
        fallback_button(canvas, c, area, false)
    }
    canvas.centered_text(
        &fonts.named(&c.font),
        &c.text,
        area.x + area.w / 2,
        area.y + area.h / 2 + (c.size.max(8.0) * 0.4) as i32,
        c.size.max(8.0),
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
