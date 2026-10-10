//! Shared look of the world maps (the corner minimap and the full map on M):
//! a rounded, tinted frame with a place badge, a Close button with its Esc
//! key, side panels, a scale bar, a compass and the player's marker. The
//! castle, the forest and the village keep their own maps and share this.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Place {
    Castle,
    Forest,
    Village,
    Brothel,
    Baths,
    /// The goblins' village up among the crags.
    Penascal,
}

/// The colours a place gives its map.
#[derive(Clone, Copy)]
pub(super) struct Tone {
    pub top: [f32; 4],
    pub bottom: [f32; 4],
    pub band_top: [f32; 4],
    pub band_bottom: [f32; 4],
    pub rim: [f32; 4],
    pub rim_soft: [f32; 4],
    pub title: [f32; 4],
    pub subtitle: [f32; 4],
    pub ink: [f32; 4],
    pub soft: [f32; 4],
}

pub(super) fn tone(place: Place) -> Tone {
    let gold = [0.86, 0.66, 0.34, 0.97];
    match place {
        Place::Castle => Tone {
            top: [0.058, 0.034, 0.078, 0.985],
            bottom: [0.030, 0.018, 0.046, 0.985],
            band_top: [0.105, 0.058, 0.120, 0.99],
            band_bottom: [0.062, 0.034, 0.080, 0.99],
            rim: gold,
            rim_soft: [0.60, 0.40, 0.72, 0.40],
            title: [1.0, 0.90, 0.68, 1.0],
            subtitle: [0.80, 0.70, 0.90, 0.96],
            ink: [0.98, 0.94, 0.84, 1.0],
            soft: [0.78, 0.72, 0.86, 0.96],
        },
        Place::Forest => Tone {
            top: [0.024, 0.062, 0.056, 0.985],
            bottom: [0.012, 0.034, 0.032, 0.985],
            band_top: [0.045, 0.108, 0.094, 0.99],
            band_bottom: [0.026, 0.066, 0.058, 0.99],
            rim: gold,
            rim_soft: [0.42, 0.72, 0.56, 0.40],
            title: [1.0, 0.92, 0.70, 1.0],
            subtitle: [0.70, 0.88, 0.80, 0.96],
            ink: [0.96, 0.96, 0.86, 1.0],
            soft: [0.70, 0.86, 0.78, 0.96],
        },
        Place::Brothel => Tone {
            top: [0.070, 0.024, 0.036, 0.985],
            bottom: [0.040, 0.012, 0.020, 0.985],
            band_top: [0.130, 0.040, 0.060, 0.99],
            band_bottom: [0.080, 0.024, 0.036, 0.99],
            rim: gold,
            rim_soft: [0.86, 0.38, 0.48, 0.40],
            title: [1.0, 0.90, 0.74, 1.0],
            subtitle: [0.94, 0.72, 0.76, 0.96],
            ink: [0.98, 0.94, 0.86, 1.0],
            soft: [0.90, 0.74, 0.78, 0.96],
        },
        Place::Baths => Tone {
            top: [0.022, 0.060, 0.062, 0.985],
            bottom: [0.012, 0.034, 0.036, 0.985],
            band_top: [0.040, 0.106, 0.104, 0.99],
            band_bottom: [0.024, 0.066, 0.066, 0.99],
            rim: gold,
            rim_soft: [0.46, 0.80, 0.76, 0.40],
            title: [1.0, 0.92, 0.74, 1.0],
            subtitle: [0.72, 0.90, 0.86, 0.96],
            ink: [0.96, 0.98, 0.92, 1.0],
            soft: [0.72, 0.88, 0.84, 0.96],
        },
        Place::Penascal => Tone {
            top: [0.050, 0.040, 0.060, 0.985],
            bottom: [0.026, 0.022, 0.034, 0.985],
            band_top: [0.096, 0.076, 0.110, 0.99],
            band_bottom: [0.056, 0.044, 0.066, 0.99],
            rim: gold,
            rim_soft: [0.66, 0.56, 0.72, 0.40],
            title: [1.0, 0.90, 0.70, 1.0],
            subtitle: [0.82, 0.76, 0.88, 0.96],
            ink: [0.97, 0.94, 0.86, 1.0],
            soft: [0.80, 0.76, 0.86, 0.96],
        },
        Place::Village => Tone {
            top: [0.050, 0.044, 0.030, 0.985],
            bottom: [0.028, 0.026, 0.018, 0.985],
            band_top: [0.098, 0.082, 0.052, 0.99],
            band_bottom: [0.058, 0.050, 0.032, 0.99],
            rim: gold,
            rim_soft: [0.78, 0.62, 0.36, 0.40],
            title: [1.0, 0.92, 0.72, 1.0],
            subtitle: [0.86, 0.80, 0.64, 0.96],
            ink: [0.98, 0.95, 0.84, 1.0],
            soft: [0.84, 0.80, 0.66, 0.96],
        },
    }
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

pub(super) fn with_alpha(color: [f32; 4], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

// ------------------------------------------------------------------ hit test

/// What the pointer can press on a map.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MapHit {
    /// The Close button of the full map.
    Close,
    /// The corner minimap: pressing it opens the full map.
    Expand,
}

pub(super) fn hit_signature(hit: Option<MapHit>) -> u8 {
    match hit {
        None => 0,
        Some(MapHit::Close) => 1,
        Some(MapHit::Expand) => 2,
    }
}

/// The Close button of the full map, at the panel's top right (clear of the
/// menu button the compositor draws in the middle of the minimap's header).
pub(super) fn close_button_rect(panel: Rect) -> Rect {
    Rect::new(panel.x + panel.w - 238.0, panel.y + 24.0, 214.0, 52.0)
}

/// The compact minimap's "M Ampliar" pill.
pub(super) fn expand_pill_rect(panel: Rect) -> Rect {
    Rect::new(
        panel.x + panel.w - 158.0,
        panel.y + panel.h - 42.0,
        144.0,
        30.0,
    )
}

pub(crate) fn hit_test(width: f32, height: f32, expanded: bool, x: f32, y: f32) -> Option<MapHit> {
    if expanded {
        close_button_rect(panel_rect(width, height, expanded))
            .contains(x, y)
            .then_some(MapHit::Close)
    } else {
        // The whole minimap opens the full map, except the menu button the
        // compositor draws just above it.
        let (mx, my, mw, mh) = crate::window_display::MENU_RECT;
        let on_menu = Rect::new(mx as f32, my as f32, mw as f32, mh as f32).contains(x, y);
        (compact_rect(width, height).contains(x, y) && !on_menu).then_some(MapHit::Expand)
    }
}

/// The corner map's chart, the whole of it now that it has no frame: the size the chart had
/// inside the old panel, under the menu button, flush with the right edge.
pub(super) fn compact_rect(width: f32, height: f32) -> Rect {
    let panel = panel_rect(width, height, false);
    let (w, h) = (panel.w - 40.0, panel.h - 136.0);
    let (_, my, _, mh) = crate::window_display::MENU_RECT;
    Rect::new(panel.x + panel.w - w, (my + mh) as f32 + 10.0, w, h)
}

// ---------------------------------------------------------------- primitives

/// A rounded rectangle with a vertical gradient.
pub(super) fn round_gradient(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    radius: f32,
    top: [f32; 4],
    bottom: [f32; 4],
) {
    let points = rounded_outline(rect, radius);
    let centre = [rect.x + rect.w * 0.5, rect.y + rect.h * 0.5];
    let at = |p: [f32; 2]| lerp4(top, bottom, (p[1] - rect.y) / rect.h.max(1.0));
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        for (position, color) in [(centre, at(centre)), (a, at(a)), (b, at(b))] {
            vertices.push(MapVertex {
                position,
                uv: [0.0; 2],
                color,
                text: 0.0,
                ..MapVertex::BLANK
            });
        }
    }
}

/// A soft shadow under a rounded rectangle: a few growing, fainter copies.
pub(super) fn soft_shadow(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    radius: f32,
    spread: f32,
    alpha: f32,
) {
    for k in 0..5 {
        let grow = spread * (1.0 - k as f32 / 5.0);
        let r = Rect::new(
            rect.x - grow,
            rect.y - grow + spread * 0.5,
            rect.w + grow * 2.0,
            rect.h + grow * 2.0,
        );
        push_rounded_rect(vertices, r, radius + grow, [0.0, 0.0, 0.0, alpha * 0.22]);
    }
}

/// A rounded pill with a coloured rim.
pub(super) fn pill(vertices: &mut Vec<MapVertex>, rect: Rect, fill: [f32; 4], rim: [f32; 4]) {
    push_rounded_rect(vertices, rect, rect.h * 0.5, fill);
    push_rounded_border(vertices, rect, rect.h * 0.5, 1.4, rim);
}

/// A small key cap with a caption after it.
pub(super) fn key_caption(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    x: f32,
    cy: f32,
    key: &str,
    caption: &str,
    size: f32,
    color: [f32; 4],
) -> f32 {
    let key_w = (label_width(font, key, size * 0.86) + 16.0).max(size * 1.5);
    let cap = Rect::new(x, cy - size * 0.72, key_w, size * 1.44);
    push_rounded_rect(
        vertices,
        Rect::new(cap.x, cap.y + 2.5, cap.w, cap.h),
        6.0,
        [0.02, 0.014, 0.03, 0.9],
    );
    push_rounded_rect(vertices, cap, 6.0, [0.20, 0.15, 0.24, 0.98]);
    push_rounded_border(vertices, cap, 6.0, 1.1, [0.86, 0.70, 0.42, 0.85]);
    push_label_centered(
        font,
        vertices,
        key,
        cap.x + cap.w * 0.5,
        cap.y + (cap.h - size * 0.86) * 0.5 - 1.0,
        size * 0.86,
        [1.0, 0.88, 0.58, 1.0],
    );
    let text_x = x + key_w + 8.0;
    push_label(
        font,
        vertices,
        caption,
        text_x,
        cy - size * 0.62,
        size,
        color,
    );
    key_w + 8.0 + label_width(font, caption, size)
}

// -------------------------------------------------------------------- frame

pub(super) struct Frame {
    pub content: Rect,
}

/// Draws the map's frame and header; returns where the map itself goes.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_frame(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    expanded: bool,
    place: Place,
    title: &str,
    subtitle: &str,
    hover: Option<MapHit>,
) -> Frame {
    let tone = tone(place);
    if !expanded {
        // The corner map is only the map: no frame, header or rules around it (they made it
        // twice its size). A soft shadow and a dark base under the chart, and a hairline.
        let content = compact_rect(width, height);
        soft_shadow(vertices, content, 14.0, 8.0, 0.55);
        push_rounded_rect(vertices, content, 14.0, with_alpha(tone.bottom, 0.86));
        push_rounded_border(vertices, content, 14.0, 1.0, with_alpha(tone.rim, 0.38));
        let _ = (font, title, subtitle, hover);
        return Frame { content };
    }
    let panel = panel_rect(width, height, expanded);
    let radius = if expanded { 26.0 } else { 18.0 };
    if expanded {
        push_rect(
            vertices,
            Rect::new(0.0, 0.0, width, height),
            [0.004, 0.008, 0.014, 0.74],
        );
    }
    soft_shadow(
        vertices,
        panel,
        radius,
        if expanded { 22.0 } else { 14.0 },
        0.9,
    );
    round_gradient(vertices, panel, radius, tone.top, tone.bottom);
    push_rounded_border(vertices, panel, radius, 2.0, tone.rim);
    let inner = Rect::new(panel.x + 7.0, panel.y + 7.0, panel.w - 14.0, panel.h - 14.0);
    push_rounded_border(vertices, inner, radius - 6.0, 1.0, tone.rim_soft);

    let header = if expanded { 92.0 } else { 76.0 };
    let band = Rect::new(panel.x + 9.0, panel.y + 9.0, panel.w - 18.0, header - 6.0);
    round_gradient(
        vertices,
        band,
        radius - 9.0,
        tone.band_top,
        tone.band_bottom,
    );

    let badge = match place {
        Place::Castle => prompt_icons::Icon::Castle,
        Place::Forest => prompt_icons::Icon::Forest,
        Place::Village => prompt_icons::Icon::House,
        Place::Brothel => prompt_icons::Icon::Heart,
        Place::Baths => prompt_icons::Icon::Bath,
        Place::Penascal => prompt_icons::Icon::Crown,
    };
    let (badge_x, text_x, title_size, subtitle_size, badge_r) = if expanded {
        (panel.x + 66.0, panel.x + 118.0, 36.0, 17.0, 31.0)
    } else {
        // The compositor keeps the menu button over the header's left part.
        (panel.x + panel.w - 44.0, panel.x + 232.0, 25.0, 14.0, 22.0)
    };
    push_prompt_badge(
        vertices,
        badge,
        badge_x,
        panel.y + 9.0 + (header - 6.0) * 0.5,
        badge_r,
        1.0,
    );
    push_label_shadowed(
        font,
        vertices,
        title,
        text_x,
        panel.y + if expanded { 17.0 } else { 12.0 },
        title_size,
        tone.title,
    );
    push_label(
        font,
        vertices,
        subtitle,
        text_x + 2.0,
        panel.y + if expanded { 60.0 } else { 44.0 },
        subtitle_size,
        tone.subtitle,
    );
    // A rule under the header with a diamond in the middle.
    let rule_y = panel.y + header + 8.0;
    push_segment(
        vertices,
        panel.x + 26.0,
        rule_y,
        panel.x + panel.w - 26.0,
        rule_y,
        1.4,
        with_alpha(tone.rim, 0.62),
    );
    let cx = panel.x + panel.w * 0.5;
    for (dx, r) in [(0.0, 5.0), (-16.0, 3.0), (16.0, 3.0)] {
        push_triangle(
            vertices,
            [
                [cx + dx - r, rule_y],
                [cx + dx, rule_y - r],
                [cx + dx + r, rule_y],
            ],
            tone.rim,
        );
        push_triangle(
            vertices,
            [
                [cx + dx - r, rule_y],
                [cx + dx + r, rule_y],
                [cx + dx, rule_y + r],
            ],
            tone.rim,
        );
    }

    if expanded {
        let close = close_button_rect(panel);
        let hot = hover == Some(MapHit::Close);
        pill(
            vertices,
            close,
            if hot {
                [0.30, 0.13, 0.15, 0.99]
            } else {
                [0.10, 0.06, 0.09, 0.96]
            },
            if hot {
                [1.0, 0.78, 0.42, 1.0]
            } else {
                with_alpha(tone.rim, 0.9)
            },
        );
        let text = "Cerrar";
        let key_w = label_width(font, "ESC", 17.0) + 16.0;
        let total = key_w + 12.0 + label_width(font, text, 24.0);
        let start = close.x + (close.w - total) * 0.5;
        let cy = close.y + close.h * 0.5;
        key_caption(
            font,
            vertices,
            start,
            cy,
            "ESC",
            text,
            24.0,
            if hot {
                [1.0, 0.95, 0.80, 1.0]
            } else {
                tone.ink
            },
        );
    } else {
        let pill_rect = expand_pill_rect(panel);
        let hot = hover == Some(MapHit::Expand);
        pill(
            vertices,
            pill_rect,
            if hot {
                [0.26, 0.15, 0.10, 0.98]
            } else {
                [0.08, 0.05, 0.08, 0.92]
            },
            if hot {
                [1.0, 0.78, 0.42, 1.0]
            } else {
                with_alpha(tone.rim, 0.85)
            },
        );
        let cy = pill_rect.y + pill_rect.h * 0.5;
        let total = (label_width(font, "M", 13.0) + 16.0).max(20.0)
            + 8.0
            + label_width(font, "Ampliar", 14.0);
        key_caption(
            font,
            vertices,
            pill_rect.x + (pill_rect.w - total) * 0.5,
            cy,
            "M",
            "Ampliar",
            14.0,
            tone.ink,
        );
    }

    let inset = if expanded { 34.0 } else { 20.0 };
    let content = Rect::new(
        panel.x + inset,
        panel.y + header + if expanded { 24.0 } else { 18.0 },
        panel.w - inset * 2.0,
        panel.h - header - inset - if expanded { 34.0 } else { 40.0 },
    );
    Frame { content }
}

/// A side panel of the full map: rounded, tinted, with a soft rim.
pub(super) fn side_panel(vertices: &mut Vec<MapVertex>, rect: Rect, place: Place) {
    let tone = tone(place);
    soft_shadow(vertices, rect, 18.0, 10.0, 0.6);
    round_gradient(
        vertices,
        rect,
        18.0,
        lerp4(tone.top, tone.band_top, 0.5),
        tone.bottom,
    );
    push_rounded_border(vertices, rect, 18.0, 1.5, with_alpha(tone.rim, 0.82));
    push_rounded_border(
        vertices,
        Rect::new(rect.x + 5.0, rect.y + 5.0, rect.w - 10.0, rect.h - 10.0),
        14.0,
        1.0,
        tone.rim_soft,
    );
}

/// A side panel's title with an icon badge and a rule.
pub(super) fn side_heading(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    place: Place,
    icon: prompt_icons::Icon,
    title: &str,
    subtitle: &str,
) {
    let tone = tone(place);
    push_prompt_badge(vertices, icon, panel.x + 40.0, panel.y + 42.0, 22.0, 1.0);
    push_label_shadowed(
        font,
        vertices,
        title,
        panel.x + 74.0,
        panel.y + 15.0,
        23.0,
        tone.title,
    );
    push_label(
        font,
        vertices,
        subtitle,
        panel.x + 75.0,
        panel.y + 47.0,
        13.5,
        tone.subtitle,
    );
    push_segment(
        vertices,
        panel.x + 18.0,
        panel.y + 80.0,
        panel.x + panel.w - 18.0,
        panel.y + 80.0,
        1.2,
        with_alpha(tone.rim, 0.6),
    );
}

/// One row of a side panel's list: an icon badge, a name, a note, and
/// whether the player is there.
#[allow(clippy::too_many_arguments)]
pub(super) fn place_row(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    place: Place,
    icon: prompt_icons::Icon,
    name: &str,
    note: &str,
    here: bool,
) {
    let tone = tone(place);
    if here {
        round_gradient(
            vertices,
            rect,
            14.0,
            [0.20, 0.13, 0.08, 0.98],
            [0.12, 0.08, 0.06, 0.98],
        );
        push_rounded_border(vertices, rect, 14.0, 1.6, [1.0, 0.80, 0.44, 0.96]);
    } else {
        push_rounded_rect(vertices, rect, 14.0, [0.0, 0.0, 0.0, 0.26]);
        push_rounded_border(vertices, rect, 14.0, 1.0, with_alpha(tone.rim_soft, 0.5));
    }
    push_prompt_badge(
        vertices,
        icon,
        rect.x + 34.0,
        rect.y + rect.h * 0.5,
        20.0,
        if here { 1.0 } else { 0.9 },
    );
    let size = fit_label_size(font, name, rect.w - 84.0, 17.0, 12.0);
    push_label(
        font,
        vertices,
        name,
        rect.x + 64.0,
        rect.y + rect.h * 0.5 - size * 1.05,
        size,
        if here {
            [1.0, 0.95, 0.80, 1.0]
        } else {
            tone.ink
        },
    );
    let note_size = fit_label_size(font, note, rect.w - 84.0, 13.0, 10.5);
    push_label(
        font,
        vertices,
        note,
        rect.x + 64.0,
        rect.y + rect.h * 0.5 + 3.0,
        note_size,
        if here {
            [1.0, 0.82, 0.48, 1.0]
        } else {
            tone.soft
        },
    );
}

/// A horizontal key: swatch and name (for the map's legend).
pub(super) fn legend_row(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    width: f32,
    swatch: impl FnOnce(&mut Vec<MapVertex>, f32, f32),
    label: &str,
    place: Place,
) {
    let tone = tone(place);
    swatch(vertices, x + 22.0, y + 15.0);
    push_label(font, vertices, label, x + 54.0, y + 5.0, 15.5, tone.ink);
    push_segment(
        vertices,
        x + 8.0,
        y + 33.0,
        x + width - 8.0,
        y + 33.0,
        0.8,
        with_alpha(tone.rim_soft, 0.5),
    );
}

// --------------------------------------------------------------- map surface

/// Darkens the map's edges so it sits inside its frame.
pub(super) fn vignette(vertices: &mut Vec<MapVertex>, rect: Rect, strength: f32) {
    let dark = [0.0, 0.0, 0.0, strength];
    let clear = [0.0, 0.0, 0.0, 0.0];
    let edge = rect.h.min(rect.w) * 0.16;
    let quad = |vertices: &mut Vec<MapVertex>, pts: [[f32; 2]; 4], colors: [[f32; 4]; 4]| {
        for i in [0usize, 1, 2, 0, 2, 3] {
            vertices.push(MapVertex {
                position: pts[i],
                uv: [0.0; 2],
                color: colors[i],
                text: 0.0,
                ..MapVertex::BLANK
            });
        }
    };
    let (x0, y0, x1, y1) = (rect.x, rect.y, rect.x + rect.w, rect.y + rect.h);
    quad(
        vertices,
        [[x0, y0], [x1, y0], [x1, y0 + edge], [x0, y0 + edge]],
        [dark, dark, clear, clear],
    );
    quad(
        vertices,
        [[x0, y1 - edge], [x1, y1 - edge], [x1, y1], [x0, y1]],
        [clear, clear, dark, dark],
    );
    quad(
        vertices,
        [[x0, y0], [x0 + edge, y0], [x0 + edge, y1], [x0, y1]],
        [dark, clear, clear, dark],
    );
    quad(
        vertices,
        [[x1 - edge, y0], [x1, y0], [x1, y1], [x1 - edge, y1]],
        [clear, dark, dark, clear],
    );
}

/// A metre scale bar in the map's lower left.
pub(super) fn scale_bar(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    place: Place,
    expanded: bool,
) {
    let tone = tone(place);
    let pixels_per_metre = map.w / (bounds.max_x - bounds.min_x).max(0.001);
    let metres = [5.0f32, 10.0, 20.0, 50.0]
        .into_iter()
        .find(|m| m * pixels_per_metre >= if expanded { 110.0 } else { 70.0 })
        .unwrap_or(50.0);
    let length = metres * pixels_per_metre;
    let (x, y) = (
        map.x + 18.0,
        map.y + map.h - if expanded { 30.0 } else { 24.0 },
    );
    push_rounded_rect(
        vertices,
        Rect::new(
            x - 8.0,
            y - 20.0,
            length + 16.0 + label_width(font, &format!("{metres:.0} m"), 13.0) + 8.0,
            34.0,
        ),
        10.0,
        [0.0, 0.0, 0.0, 0.44],
    );
    push_segment(
        vertices,
        x,
        y,
        x + length,
        y,
        3.0,
        with_alpha(tone.ink, 0.95),
    );
    for t in [0.0, 0.5, 1.0] {
        push_segment(
            vertices,
            x + length * t,
            y - 5.0,
            x + length * t,
            y + 5.0,
            2.0,
            with_alpha(tone.ink, 0.95),
        );
    }
    push_label(
        font,
        vertices,
        &format!("{metres:.0} m"),
        x + length + 10.0,
        y - 8.5,
        13.0,
        tone.ink,
    );
}

/// The compass: a plate with the north needle.
pub(super) fn compass(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    place: Place,
    expanded: bool,
) {
    let tone = tone(place);
    let r = if expanded { 26.0 } else { 19.0 };
    let (x, y) = (map.x + r + 16.0, map.y + r + 16.0);
    push_circle(
        vertices,
        x + 1.5,
        y + 3.0,
        r + 3.0,
        [0.0, 0.0, 0.0, 0.4],
        36,
    );
    push_circle(vertices, x, y, r + 2.0, with_alpha(tone.rim, 0.95), 36);
    push_circle(vertices, x, y, r, [0.03, 0.03, 0.05, 0.95], 36);
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        let (r0, r1) = (r - if k % 2 == 0 { 7.0 } else { 4.0 }, r - 2.0);
        push_segment(
            vertices,
            x + a.cos() * r0,
            y + a.sin() * r0,
            x + a.cos() * r1,
            y + a.sin() * r1,
            1.3,
            with_alpha(tone.ink, 0.6),
        );
    }
    push_triangle(
        vertices,
        [[x, y - r + 6.0], [x - 5.5, y + 1.0], [x + 5.5, y + 1.0]],
        [0.96, 0.30, 0.30, 1.0],
    );
    push_triangle(
        vertices,
        [[x, y + r - 6.0], [x - 5.0, y - 1.0], [x + 5.0, y - 1.0]],
        [0.70, 0.72, 0.72, 0.9],
    );
    push_circle(vertices, x, y, 2.4, [1.0, 0.9, 0.6, 1.0], 10);
    push_label_centered(
        font,
        vertices,
        "N",
        x,
        y - r - 16.0,
        if expanded { 15.0 } else { 12.0 },
        [1.0, 0.92, 0.72, 1.0],
    );
}

/// The player's marker: a glow, a dot and the way she faces.
pub(super) fn player_pin(
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    yaw: f32,
    expanded: bool,
) {
    let (x, y) = map_point(player.x, player.z, bounds, map);
    let r = if expanded { 8.0 } else { 6.0 };
    push_circle(vertices, x, y, r + 11.0, [1.0, 0.30, 0.60, 0.13], 28);
    push_circle(vertices, x, y, r + 6.0, [1.0, 0.30, 0.60, 0.22], 28);
    // A cone of sight ahead.
    let (fx, fy) = (yaw.sin(), -yaw.cos());
    let (px, py) = (-fy, fx);
    let reach = r + if expanded { 24.0 } else { 17.0 };
    push_triangle(
        vertices,
        [
            [x + fx * (r + 1.0), y + fy * (r + 1.0)],
            [
                x + fx * reach + px * reach * 0.42,
                y + fy * reach + py * reach * 0.42,
            ],
            [
                x + fx * reach - px * reach * 0.42,
                y + fy * reach - py * reach * 0.42,
            ],
        ],
        [1.0, 0.86, 0.42, 0.34],
    );
    push_circle(vertices, x, y, r + 2.5, [0.02, 0.02, 0.04, 0.9], 24);
    push_circle(vertices, x, y, r, [1.0, 0.28, 0.58, 1.0], 24);
    push_circle(
        vertices,
        x - r * 0.25,
        y - r * 0.3,
        r * 0.38,
        [1.0, 0.85, 0.92, 0.9],
        12,
    );
    push_segment(
        vertices,
        x,
        y,
        x + fx * (r + 9.0),
        y + fy * (r + 9.0),
        if expanded { 2.6 } else { 2.0 },
        [1.0, 0.88, 0.45, 1.0],
    );
}

/// A landmark: an icon badge with a name plate under (or over) it.
#[allow(clippy::too_many_arguments)]
pub(super) fn landmark(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    world: (f32, f32),
    icon: prompt_icons::Icon,
    label: &str,
    place: Place,
    expanded: bool,
    label_above: bool,
) {
    let tone = tone(place);
    let (x, y) = map_point(world.0, world.1, bounds, map);
    let radius = if expanded { 25.0 } else { 17.0 };
    push_prompt_badge(vertices, icon, x, y, radius, 1.0);
    let size = if expanded { 18.0 } else { 14.0 };
    let width = label_width(font, label, size) + 18.0;
    let plate_y = if label_above {
        y - radius - size - 14.0
    } else {
        y + radius + 6.0
    };
    let plate = Rect::new(x - width * 0.5, plate_y, width, size + 8.0);
    push_rounded_rect(vertices, plate, 8.0, [0.02, 0.02, 0.03, 0.82]);
    push_rounded_border(vertices, plate, 8.0, 1.0, with_alpha(tone.rim, 0.75));
    push_label_centered(font, vertices, label, x, plate.y + 3.5, size, tone.ink);
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 1920.0;
    const H: f32 = 1080.0;

    fn menu() -> Rect {
        let (x, y, w, h) = crate::window_display::MENU_RECT;
        Rect::new(x as f32, y as f32, w as f32, h as f32)
    }

    #[test]
    fn the_close_button_is_inside_the_full_map_and_clear_of_the_menu_button() {
        let panel = panel_rect(W, H, true);
        let close = close_button_rect(panel);
        assert!(close.x >= panel.x && close.x + close.w <= panel.x + panel.w);
        assert!(close.y >= panel.y && close.y + close.h <= panel.y + panel.h);
        let menu = menu();
        let overlap = close.x < menu.x + menu.w
            && menu.x < close.x + close.w
            && close.y < menu.y + menu.h
            && menu.y < close.y + close.h;
        assert!(
            !overlap,
            "the compositor's menu button must not cover Close"
        );
        assert_eq!(
            hit_test(W, H, true, close.x + close.w * 0.5, close.y + close.h * 0.5),
            Some(MapHit::Close)
        );
        assert_eq!(
            hit_test(W, H, true, 960.0, 540.0),
            None,
            "the chart itself is not a button"
        );
        assert_eq!(hit_test(W, H, true, close.x - 5.0, close.y + 5.0), None);
    }

    #[test]
    fn the_minimap_opens_the_full_map_except_where_the_menu_button_is() {
        let panel = panel_rect(W, H, false);
        assert_eq!(
            hit_test(
                W,
                H,
                false,
                panel.x + panel.w * 0.5,
                panel.y + panel.h * 0.7
            ),
            Some(MapHit::Expand)
        );
        let menu = menu();
        assert_eq!(
            hit_test(W, H, false, menu.x + menu.w * 0.5, menu.y + menu.h * 0.5),
            None
        );
        assert_eq!(
            hit_test(W, H, false, 200.0, 500.0),
            None,
            "the world is not a button"
        );
        // The pill and the Close button are different targets in different states.
        assert_ne!(
            hit_signature(Some(MapHit::Close)),
            hit_signature(Some(MapHit::Expand))
        );
        assert_eq!(hit_signature(None), 0);
    }

    #[test]
    fn hit_tests_hold_at_other_canvas_sizes() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
            let panel = panel_rect(w, h, true);
            let close = close_button_rect(panel);
            assert!(hit_test(w, h, true, close.x + 4.0, close.y + 4.0).is_some());
            assert!(close.x + close.w <= w && close.y >= 0.0);
        }
    }

    #[test]
    fn every_place_has_a_bright_title_and_a_dark_panel() {
        for place in [
            Place::Castle,
            Place::Forest,
            Place::Village,
            Place::Brothel,
            Place::Baths,
        ] {
            let tone = tone(place);
            assert!(tone.title[..3].iter().cloned().fold(0.0, f32::max) > 0.8);
            assert!(
                tone.top[..3].iter().all(|c| *c < 0.1),
                "{place:?} panel is dark"
            );
            assert!(tone.top[3] > 0.9 && tone.rim[3] > 0.9);
        }
    }

    #[test]
    fn gradients_and_shadows_stay_finite_and_inside_their_rectangle() {
        let mut v = Vec::new();
        let rect = Rect::new(100.0, 100.0, 400.0, 200.0);
        round_gradient(&mut v, rect, 20.0, [0.1; 4], [0.0, 0.0, 0.0, 1.0]);
        assert!(v.len() % 3 == 0 && !v.is_empty());
        for vertex in &v {
            assert!(vertex
                .position
                .iter()
                .chain(vertex.color.iter())
                .all(|c| c.is_finite()));
            assert!(
                vertex.position[0] >= rect.x - 0.01 && vertex.position[0] <= rect.x + rect.w + 0.01
            );
            assert!(
                vertex.position[1] >= rect.y - 0.01 && vertex.position[1] <= rect.y + rect.h + 0.01
            );
        }
    }
}
