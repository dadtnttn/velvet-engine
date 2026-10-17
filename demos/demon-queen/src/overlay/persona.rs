//! Flourishes in the manner of Persona's menus, for Rosalía's book and the elixir: groups turned as
//! one (a tile with its words and gems, so a tilted card keeps its smooth edges), slashes of colour
//! sweeping across, a halftone of dots, speed lines, a comic burst behind a shout, plates of text
//! in a speech box, and the eased entrances they come in with. All of it the painter's shapes and
//! text; nothing here is a picture. Entrances move things once when a page opens: what the pointer
//! or the keys light up only changes its light and colour, never its place or size.
use std::f32::consts::TAU;

use super::ui_paint::*;
use super::*;

/// `paint`'s shapes and text turned by `angle` (radians; positive turns clockwise on screen)
/// about `pivot`. Each shape is drawn from its own local coordinates, so turning the corners of
/// its quad turns it whole and its edges stay smooth.
pub(super) fn turned(
    v: &mut Vec<MapVertex>,
    pivot: [f32; 2],
    angle: f32,
    paint: impl FnOnce(&mut Vec<MapVertex>),
) {
    let start = v.len();
    paint(v);
    if angle == 0.0 {
        return;
    }
    let (s, c) = angle.sin_cos();
    for vertex in &mut v[start..] {
        let dx = vertex.position[0] - pivot[0];
        let dy = vertex.position[1] - pivot[1];
        vertex.position = [pivot[0] + dx * c - dy * s, pivot[1] + dx * s + dy * c];
    }
}

/// Fades everything `paint` adds by `alpha` (an entrance or an exit).
pub(super) fn faded(v: &mut Vec<MapVertex>, alpha: f32, paint: impl FnOnce(&mut Vec<MapVertex>)) {
    let start = v.len();
    paint(v);
    if alpha >= 0.999 {
        return;
    }
    let a = alpha.clamp(0.0, 1.0);
    for vertex in &mut v[start..] {
        vertex.color[3] *= a;
        vertex.color2[3] *= a;
    }
}

/// Moves everything `paint` adds by `by` (an entrance sliding in).
pub(super) fn shifted(
    v: &mut Vec<MapVertex>,
    by: [f32; 2],
    paint: impl FnOnce(&mut Vec<MapVertex>),
) {
    let start = v.len();
    paint(v);
    if by == [0.0, 0.0] {
        return;
    }
    for vertex in &mut v[start..] {
        vertex.position[0] += by[0];
        vertex.position[1] += by[1];
    }
}

/// 0 → 1 with a quick start and a soft landing.
pub(super) fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// How far in an entrance is: `age` seconds since the page opened, starting after `delay`,
/// lasting `length`.
pub(super) fn entrance(age: f32, delay: f32, length: f32) -> f32 {
    ease_out((age - delay) / length.max(1e-3))
}

/// A band of colour across the screen through `center`, `length` long and `thickness` thick,
/// turned by `angle`; `top` and `bottom` colour its two long edges.
#[allow(clippy::too_many_arguments)]
pub(super) fn slash(
    v: &mut Vec<MapVertex>,
    center: [f32; 2],
    length: f32,
    thickness: f32,
    angle: f32,
    top: [f32; 4],
    bottom: [f32; 4],
) {
    turned(v, center, angle, |v| {
        panel(
            v,
            Rect::new(
                center[0] - length * 0.5,
                center[1] - thickness * 0.5,
                length,
                thickness,
            ),
            0.0,
            Panel {
                top,
                bottom,
                ..Panel::fill(0, 0, 0.0)
            },
        );
    });
}

/// A halftone over `r`: dots on a grid `step` apart, growing toward the side `toward` points to
/// (a unit vector), up to `most` across.
pub(super) fn halftone(
    v: &mut Vec<MapVertex>,
    r: Rect,
    step: f32,
    most: f32,
    toward: [f32; 2],
    color: [f32; 4],
) {
    let cols = (r.w / step).ceil() as i32;
    let rows = (r.h / step).ceil() as i32;
    for j in 0..=rows {
        for i in 0..=cols {
            let x = r.x + i as f32 * step + if j % 2 == 1 { step * 0.5 } else { 0.0 };
            let y = r.y + j as f32 * step;
            let u = ((x - r.x) / r.w.max(1.0)) * toward[0] + ((y - r.y) / r.h.max(1.0)) * toward[1];
            let k = u.clamp(0.0, 1.0);
            let radius = most * 0.5 * k;
            if radius > 0.6 {
                disc(v, [x, y], radius, color);
            }
        }
    }
}

/// Speed lines: thin streaks across `r` turned by `angle`, sliding with `time`.
pub(super) fn speed_lines(
    v: &mut Vec<MapVertex>,
    r: Rect,
    angle: f32,
    count: usize,
    time: f32,
    color: [f32; 4],
) {
    let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
    turned(v, c, angle, |v| {
        for i in 0..count {
            let fi = i as f32;
            let y = r.y + r.h * ((fi * 0.618_034).fract());
            let length = r.w * (0.18 + 0.22 * ((fi * 1.37).sin() * 0.5 + 0.5));
            let x = r.x
                + ((time * (0.35 + 0.2 * (fi * 0.7).cos().abs()) + fi * 0.21).fract())
                    * (r.w + length)
                - length;
            let width = 1.5 + 2.5 * ((fi * 2.1).sin() * 0.5 + 0.5);
            wire(
                v,
                [x, y],
                [x + length, y],
                width,
                [color[0], color[1], color[2], 0.0],
                color,
                0.0,
                false,
            );
        }
    });
}

/// A comic burst: `points` spikes between `inner` and `outer` around `c`, turned by `angle`, with
/// a dark rim `rim` wide.
#[allow(clippy::too_many_arguments)]
pub(super) fn burst(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    inner: f32,
    outer: f32,
    points: usize,
    angle: f32,
    fill: [f32; 4],
    rim: [f32; 4],
    rim_width: f32,
) {
    let star = |v: &mut Vec<MapVertex>, grow: f32, color: [f32; 4]| {
        let n = points.max(5) * 2;
        let at = |k: usize| {
            let a = angle + k as f32 / n as f32 * TAU;
            // Spikes of uneven length, like a hand-drawn shout.
            let jag = 1.0 + 0.16 * ((k as f32 * 2.7).sin());
            let r = if k % 2 == 0 { outer * jag } else { inner } + grow;
            [c[0] + a.cos() * r, c[1] + a.sin() * r]
        };
        for k in 0..n {
            push_triangle(v, [c, at(k), at((k + 1) % n)], color);
        }
        let outline: Vec<[f32; 2]> = (0..n).map(at).collect();
        for k in 0..n {
            let (a, b) = (outline[k], outline[(k + 1) % n]);
            let d = [b[0] - a[0], b[1] - a[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
            // Outward from the centre's side of the edge.
            let mid = [(a[0] + b[0]) * 0.5 - c[0], (a[1] + b[1]) * 0.5 - c[1]];
            let mut out = [-d[1] / len, d[0] / len];
            if out[0] * mid[0] + out[1] * mid[1] < 0.0 {
                out = [-out[0], -out[1]];
            }
            push_fringe(v, a, b, out, 1.1, color);
        }
    };
    if rim_width > 0.0 {
        star(v, rim_width, rim);
    }
    star(v, 0.0, fill);
}

/// A speech box in the manner of the game's comics: a white card with a black edge, its speaker
/// on a tab of their colour and `words` inside in black, turned by `angle` about its centre. Returns
/// its rect (before turning).
#[allow(clippy::too_many_arguments)]
pub(super) fn speech_box(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    at: [f32; 2],
    max_width: f32,
    who: &str,
    words: &str,
    accent: u32,
    angle: f32,
    alpha: f32,
    s: f32,
) -> Rect {
    let size = 27.0 * s;
    let lines = wrap(font, Face::Body, words, size, max_width - 56.0 * s);
    let width = lines
        .iter()
        .map(|l| text_width(font, Face::Body, l, size))
        .fold(0.0, f32::max)
        .max(text_width(font, Face::Title, who, 24.0 * s))
        + 56.0 * s;
    let height = 58.0 * s + lines.len() as f32 * size * 1.38 + 18.0 * s;
    let r = Rect::new(at[0], at[1] - height, width, height);
    let center = [r.x + r.w * 0.5, r.y + r.h * 0.5];
    faded(v, alpha, |v| {
        turned(v, center, angle, |v| {
            soft(
                v,
                Rect::new(r.x + 10.0 * s, r.y + 14.0 * s, r.w, r.h),
                10.0 * s,
                10.0 * s,
                rgba(SHADOW, 0.6),
                false,
            );
            panel(
                v,
                grow(r, 5.0 * s),
                12.0 * s,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            panel(v, r, 9.0 * s, Panel::fill(0xfffaf4, 0xf3e6dc, 1.0));
            // The speaker's name on a tab of their colour.
            let tab = Rect::new(
                r.x + 18.0 * s,
                r.y - 20.0 * s,
                text_width(font, Face::Title, who, 24.0 * s) + 34.0 * s,
                40.0 * s,
            );
            panel(
                v,
                grow(tab, 3.0 * s),
                8.0 * s,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            panel(
                v,
                tab,
                6.0 * s,
                Panel::fill(
                    mix(accent, 0xffffff, 0.15),
                    mix(accent, 0x000000, 0.25),
                    1.0,
                ),
            );
            text(
                font,
                v,
                Face::Title,
                who,
                tab.x + 17.0 * s,
                tab.y + 6.0 * s,
                24.0 * s,
                rgba(0xffffff, 1.0),
            );
            for (i, line) in lines.iter().enumerate() {
                text(
                    font,
                    v,
                    Face::Body,
                    line,
                    r.x + 28.0 * s,
                    r.y + 34.0 * s + i as f32 * size * 1.38,
                    size,
                    rgba(0x1a0c12, 1.0),
                );
            }
        });
    });
    r
}

/// Big words with a black edge and a coloured shadow behind (a title on a slash).
#[allow(clippy::too_many_arguments)]
pub(super) fn shout(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    words: &str,
    x: f32,
    y: f32,
    size: f32,
    ink: u32,
    shadow: u32,
    alpha: f32,
) {
    let d = (size * 0.07).max(2.0);
    text_outline(
        font,
        v,
        face,
        words,
        x + d,
        y + d,
        size,
        rgba(shadow, alpha),
        rgba(shadow, alpha),
    );
    text_outline(
        font,
        v,
        face,
        words,
        x,
        y,
        size,
        rgba(ink, alpha),
        rgba(0x0b0508, alpha),
    );
}

/// A comic stamp's words (¡ZAS!, ¡GRAAAH!, REGALO…) centred on `c`: the letters in their colour
/// with a dark ink outline, a clean white edge round that, and a hard shadow in a deep shade of
/// the colour; `burst` puts a jagged burst of the colour's dark behind them. `slam` (0..1, how far
/// the impact has played) throws impact lines out from behind for an instant. All of it is drawn
/// from the glyphs' distance fields: clean at any size.
#[allow(clippy::too_many_arguments)]
pub(super) fn comic_stamp(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    words: &str,
    c: [f32; 2],
    size: f32,
    color: u32,
    burst_behind: bool,
    slam: f32,
    time: f32,
) {
    let w = text_width(font, face, words, size);
    let (x, y) = (c[0] - w * 0.5, c[1] - size * 0.56);
    let ink = 0x0b0508;
    let deep = mix(color, 0x000000, 0.62);
    let line = (size * 0.045).max(2.5);
    let edge = (size * 0.035).max(2.0);
    if burst_behind {
        // Just round the words: its spikes reach a little past them.
        let r = w * 0.5 + size * 0.2;
        burst(
            v,
            [c[0], c[1] + size * 0.04],
            r * 0.78,
            r,
            18,
            0.3 + time * 0.12,
            rgba(mix(deep, color, 0.25), 0.88),
            rgba(ink, 0.95),
            line * 0.9,
        );
    }
    // Impact lines out from behind it, the moment it lands.
    if slam < 1.0 {
        let k = slam.clamp(0.0, 1.0);
        let n = 14;
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU + 0.21 * (i % 3) as f32;
            let (sn, cs) = a.sin_cos();
            let r0 = w * 0.45 + size * 0.2 + k * size * 0.9;
            let r1 = r0 + size * (0.5 + 0.35 * ((i * 7 % 5) as f32 / 5.0)) * (1.0 - k);
            wire(
                v,
                [c[0] + cs * r0, c[1] + sn * r0 * 0.6],
                [c[0] + cs * r1, c[1] + sn * r1 * 0.6],
                size * 0.05 * (1.0 - k) + 1.0,
                rgba(0xffffff, 0.9 * (1.0 - k)),
                rgba(color, 0.0),
                0.0,
                false,
            );
        }
    }
    // The shadow: the whole stamp, outline and all, in a deep shade.
    let sh = size * 0.06;
    text_rimmed(
        font,
        v,
        face,
        words,
        x + sh,
        y + sh,
        size,
        rgba(deep, 0.9),
        line + edge,
        rgba(deep, 0.9),
    );
    // The white edge, the ink outline, the letters.
    text_rimmed(
        font,
        v,
        face,
        words,
        x,
        y,
        size,
        rgba(ink, 1.0),
        line + edge,
        rgba(0xfff8f0, 1.0),
    );
    text_rimmed(
        font,
        v,
        face,
        words,
        x,
        y,
        size,
        rgba(color, 1.0),
        line,
        rgba(ink, 1.0),
    );
    // A glint along the letters' tops.
    text(
        font,
        v,
        face,
        words,
        x - size * 0.012,
        y - size * 0.018,
        size,
        rgba(mix(color, 0xffffff, 0.55), 0.35),
    );
    text(font, v, face, words, x, y, size, rgba(color, 1.0));
}

/// The cursor that points at the lit tile: a red arrowhead with a white edge, bobbing.
pub(super) fn cursor(v: &mut Vec<MapVertex>, tip: [f32; 2], size: f32, time: f32) {
    let bob = (time * 5.0).sin() * size * 0.12;
    let tip = [tip[0] + bob, tip[1]];
    let back = [tip[0] - size * 1.25, tip[1]];
    let up = [back[0] - size * 0.15, tip[1] - size * 0.62];
    let down = [back[0] - size * 0.15, tip[1] + size * 0.62];
    let edge = size * 0.16;
    push_triangle(
        v,
        [
            [tip[0] + edge, tip[1]],
            [up[0] - edge, up[1] - edge],
            [down[0] - edge, down[1] + edge],
        ],
        rgba(0xffffff, 1.0),
    );
    push_triangle(v, [tip, up, down], rgba(CRIMSON, 1.0));
    push_triangle(
        v,
        [[back[0] + size * 0.2, back[1]], up, down],
        rgba(0x8a1030, 1.0),
    );
}
