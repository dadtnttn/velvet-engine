//! The expedition HUD's shapes, in the manner of the game's Persona menus: slanted ink plates
//! with a hard shadow and a wedge of the place's colour, slanted chips, keys and segmented bars,
//! and round icon badges, drawn as anti-aliased vertex batches in native 1920x1080 space.
//! Colours are linear RGB, like the rest of the HUD batch.
use super::prompt_icons::{self, Icon};
use super::{triangle, Vertex};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub(super) type Rgba = [f32; 4];

pub(super) fn alpha(c: Rgba, a: f32) -> Rgba {
    [c[0], c[1], c[2], a]
}

pub(super) fn lift(c: Rgba, k: f32) -> Rgba {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}

pub(super) fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// The colours a screen borrows from the place the Queen is in: `accent` is
/// the highlight (rims, labels, meters), `glass` the tint of the panes.
#[derive(Clone, Copy)]
pub(super) struct Tone {
    pub accent: Rgba,
    pub bright: Rgba,
    pub glass: Rgba,
}

impl Tone {
    pub fn new(biome_accent: [f32; 3]) -> Self {
        let peak = biome_accent.into_iter().fold(0.05, f32::max);
        let warm = [0.93, 0.57, 0.16];
        let accent: Vec<f32> = (0..3)
            .map(|i| warm[i] * 0.55 + biome_accent[i] / peak * 0.45 * 0.95)
            .collect();
        let accent = [accent[0], accent[1], accent[2], 1.0];
        let bright = mix(accent, [1.0, 0.95, 0.82, 1.0], 0.45);
        let glass = [
            0.006 + accent[0] * 0.020,
            0.010 + accent[1] * 0.020,
            0.016 + accent[2] * 0.024,
            1.0,
        ];
        Self {
            accent,
            bright,
            glass,
        }
    }
}

pub(super) const IVORY: Rgba = [0.93, 0.90, 0.78, 1.0];
/// The menus' ink (0e0816), linear.
pub(super) const INK: Rgba = [0.0044, 0.0024, 0.0080, 1.0];

/// How far a plate `h` tall leans.
pub(super) fn lean_for(h: f32) -> f32 {
    (h * 0.12).clamp(4.0, 14.0)
}

/// The corners of `r` leaning forward: its top edge starts `lean` in, its bottom edge ends `lean`
/// short (so the plate stays inside `r`). Clockwise on screen.
pub(super) fn slant_path(r: [f32; 4], lean: f32) -> Vec<[f32; 2]> {
    let [x, y, w, h] = r;
    vec![[x + lean, y], [x + w, y], [x + w - lean, y + h], [x, y + h]]
}

pub(super) fn slant(v: &mut Vec<Vertex>, r: [f32; 4], lean: f32, color: Rgba) {
    fill_flat(v, &slant_path(r, lean), color);
}

pub(super) fn slant_outline(v: &mut Vec<Vertex>, r: [f32; 4], lean: f32, width: f32, color: Rgba) {
    stroke(v, &slant_path(r, lean), true, width, color);
}

/// A slanted plate of ink with its hard shadow, `edge` coloured along its leading edge.
pub(super) fn plate(v: &mut Vec<Vertex>, r: [f32; 4], edge: Rgba, strength: f32) {
    let [x, y, w, h] = r;
    let lean = lean_for(h);
    slant(
        v,
        [x + 8.0, y + 8.0, w, h],
        lean,
        [0.0, 0.0, 0.0, 0.42 * strength + 0.08],
    );
    fill(
        v,
        &slant_path(r, lean),
        alpha(mix(INK, edge, 0.012), 0.9 * strength + 0.08),
        alpha(INK, 0.93 * strength + 0.06),
        y,
        y + h,
    );
    // A faint hatch across it, leaning with it.
    let mut s = 30.0;
    while s < w - lean - 6.0 {
        segment(
            v,
            [x + lean + s, y + 3.0],
            [x + s + lean * 3.0 / h, y + h - 3.0],
            1.0,
            [1.0, 1.0, 1.0, 0.008 * strength],
        );
        s += 26.0;
    }
    let k = 8.0;
    fill_flat(
        v,
        &[[x + lean, y], [x + lean + k, y], [x + k, y + h], [x, y + h]],
        alpha(edge, 0.95),
    );
    segment(
        v,
        [x + lean + k, y + 1.0],
        [x + w, y + 1.0],
        2.0,
        alpha(edge, 0.8 * strength),
    );
    slant_outline(v, r, lean, 1.0, [1.0, 1.0, 1.0, 0.08 * strength]);
}
pub(super) const MUTED: Rgba = [0.55, 0.63, 0.62, 1.0];
pub(super) const CRIMSON: Rgba = [0.78, 0.045, 0.03, 1.0];
pub(super) const GREEN: Rgba = [0.28, 0.78, 0.42, 1.0];

/// Corner points of a rounded rectangle, clockwise on screen.
pub(super) fn rounded_path(r: [f32; 4], radius: f32) -> Vec<[f32; 2]> {
    let [x, y, w, h] = r;
    let radius = radius.min(w * 0.5).min(h * 0.5).max(0.5);
    let n = ((radius / 3.0).ceil() as usize).clamp(3, 9);
    let mut out = Vec::with_capacity(4 * (n + 1));
    for (cx, cy, start) in [
        (x + radius, y + radius, PI),
        (x + w - radius, y + radius, PI + FRAC_PI_2),
        (x + w - radius, y + h - radius, 0.0),
        (x + radius, y + h - radius, FRAC_PI_2),
    ] {
        for i in 0..=n {
            let a = start + FRAC_PI_2 * i as f32 / n as f32;
            out.push([cx + a.cos() * radius, cy + a.sin() * radius]);
        }
    }
    out
}

pub(super) fn circle_path(c: [f32; 2], r: f32) -> Vec<[f32; 2]> {
    let n = ((r * 1.4).ceil() as usize).clamp(14, 64);
    (0..n)
        .map(|i| {
            let a = i as f32 * TAU / n as f32;
            [c[0] + a.cos() * r, c[1] + a.sin() * r]
        })
        .collect()
}

/// Outward unit normals of a closed clockwise-on-screen path.
fn normals(path: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = path.len();
    (0..n)
        .map(|i| {
            let a = path[(i + n - 1) % n];
            let b = path[(i + 1) % n];
            let d = [b[0] - a[0], b[1] - a[1]];
            let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-4);
            [d[1] / l, -d[0] / l]
        })
        .collect()
}

fn vertex(v: &mut Vec<Vertex>, p: [f32; 2], color: Rgba) {
    v.push(Vertex {
        p,
        uv: [0.0015; 2],
        color,
    });
}

pub(super) fn quad(
    v: &mut Vec<Vertex>,
    a: ([f32; 2], Rgba),
    b: ([f32; 2], Rgba),
    c: ([f32; 2], Rgba),
    d: ([f32; 2], Rgba),
) {
    for (p, k) in [a, b, c, a, c, d] {
        vertex(v, p, k);
    }
}

/// Fills a convex clockwise path with a vertical gradient (`y0` -> `y1`)
/// and a one-pixel anti-aliased fringe.
pub(super) fn fill(
    v: &mut Vec<Vertex>,
    path: &[[f32; 2]],
    top: Rgba,
    bottom: Rgba,
    y0: f32,
    y1: f32,
) {
    let n = path.len();
    if n < 3 {
        return;
    }
    let at = |p: [f32; 2]| mix(top, bottom, (p[1] - y0) / (y1 - y0).max(0.001));
    let c = path.iter().fold([0.0, 0.0], |s, p| {
        [s[0] + p[0] / n as f32, s[1] + p[1] / n as f32]
    });
    for i in 0..n {
        let (a, b) = (path[i], path[(i + 1) % n]);
        vertex(v, c, at(c));
        vertex(v, a, at(a));
        vertex(v, b, at(b));
    }
    let ns = normals(path);
    for i in 0..n {
        let j = (i + 1) % n;
        let out = |p: [f32; 2], k: [f32; 2]| [p[0] + k[0], p[1] + k[1]];
        let (ca, cb) = (at(path[i]), at(path[j]));
        quad(
            v,
            (path[i], ca),
            (path[j], cb),
            (out(path[j], ns[j]), alpha(cb, 0.0)),
            (out(path[i], ns[i]), alpha(ca, 0.0)),
        );
    }
}

pub(super) fn fill_flat(v: &mut Vec<Vertex>, path: &[[f32; 2]], color: Rgba) {
    fill(v, path, color, color, 0.0, 1.0);
}

/// A ribbon of `width` centred on a path, soft on both edges. Closed paths
/// must run clockwise on screen.
pub(super) fn stroke(
    v: &mut Vec<Vertex>,
    path: &[[f32; 2]],
    closed: bool,
    width: f32,
    color: Rgba,
) {
    let n = path.len();
    if n < 2 {
        return;
    }
    let ns: Vec<[f32; 2]> = if closed {
        normals(path)
    } else {
        (0..n)
            .map(|i| {
                let a = path[i.saturating_sub(1)];
                let b = path[(i + 1).min(n - 1)];
                let d = [b[0] - a[0], b[1] - a[1]];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-4);
                [d[1] / l, -d[0] / l]
            })
            .collect()
    };
    let half = width * 0.5;
    let layers = [
        (half + 0.75, 0.0),
        (half, 1.0),
        (-half, 1.0),
        (-half - 0.75, 0.0),
    ];
    let at = |i: usize, layer: usize| {
        let (d, a) = layers[layer];
        (
            [path[i][0] + ns[i][0] * d, path[i][1] + ns[i][1] * d],
            alpha(color, color[3] * a),
        )
    };
    let count = if closed { n } else { n - 1 };
    for i in 0..count {
        let j = (i + 1) % n;
        for layer in 0..3 {
            quad(
                v,
                at(i, layer),
                at(j, layer),
                at(j, layer + 1),
                at(i, layer + 1),
            );
        }
    }
}

/// A straight soft line (round joins are not needed at these widths).
pub(super) fn segment(v: &mut Vec<Vertex>, a: [f32; 2], b: [f32; 2], width: f32, color: Rgba) {
    stroke(v, &[a, b], false, width, color);
}

/// Dashed straight line.
pub(super) fn dashes(
    v: &mut Vec<Vertex>,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    dash: f32,
    gap: f32,
    color: Rgba,
) {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
    if len < 0.5 {
        return;
    }
    let u = [d[0] / len, d[1] / len];
    let mut t = 0.0;
    while t < len {
        let e = (t + dash).min(len);
        segment(
            v,
            [a[0] + u[0] * t, a[1] + u[1] * t],
            [a[0] + u[0] * e, a[1] + u[1] * e],
            width,
            color,
        );
        t += dash + gap;
    }
}

pub(super) fn disc(v: &mut Vec<Vertex>, c: [f32; 2], r: f32, color: Rgba) {
    fill_flat(v, &circle_path(c, r), color);
}

pub(super) fn ring(v: &mut Vec<Vertex>, c: [f32; 2], r: f32, width: f32, color: Rgba) {
    stroke(v, &circle_path(c, r), true, width, color);
}

/// A rounded rectangle with a vertical gradient.
pub(super) fn round(v: &mut Vec<Vertex>, r: [f32; 4], radius: f32, top: Rgba, bottom: Rgba) {
    fill(v, &rounded_path(r, radius), top, bottom, r[1], r[1] + r[3]);
}

pub(super) fn round_outline(
    v: &mut Vec<Vertex>,
    r: [f32; 4],
    radius: f32,
    width: f32,
    color: Rgba,
) {
    stroke(v, &rounded_path(r, radius), true, width, color);
}

/// A pane of dark glass: soft shadow, tinted body that lets the world show
/// through, a sheen under the top edge and a fine lit rim. `strength` (0..1)
/// scales how opaque and how bright the rim is.
pub(super) fn pane(v: &mut Vec<Vertex>, r: [f32; 4], _radius: f32, tone: &Tone, strength: f32) {
    plate(v, r, tone.accent, strength);
}

/// A slanted chip (tags, slots): a dark plate tinted with `color`, outlined in it.
pub(super) fn chip(v: &mut Vec<Vertex>, r: [f32; 4], color: Rgba, fill_alpha: f32) {
    let lean = (r[3] * 0.22).clamp(3.0, 9.0);
    slant(
        v,
        [r[0] + 3.0, r[1] + 3.0, r[2], r[3]],
        lean,
        [0.0, 0.0, 0.0, 0.45],
    );
    slant(
        v,
        r,
        lean,
        alpha(mix(INK, color, (fill_alpha * 0.45).min(0.12)), 0.95),
    );
    slant_outline(v, r, lean, 1.2, alpha(color, 0.8));
}

/// A slanted bar: an ink track and the filled part shading from `low` to `high`, cut in slanted
/// segments like the menus' gauges.
pub(super) fn meter(v: &mut Vec<Vertex>, r: [f32; 4], fraction: f32, low: Rgba, high: Rgba) {
    let [x, y, w, h] = r;
    let lean = (h * 0.6).clamp(2.0, 9.0);
    slant(v, [x + 3.0, y + 3.0, w, h], lean, [0.0, 0.0, 0.0, 0.45]);
    slant(v, r, lean, alpha(INK, 0.94));
    let filled = (w * fraction.clamp(0.0, 1.0)).max(0.0);
    if filled > lean + 1.0 {
        let right = x + filled;
        quad(
            v,
            ([x + lean, y], low),
            ([right, y], high),
            ([right - lean, y + h], high),
            ([x, y + h], low),
        );
        quad(
            v,
            ([x + lean, y], [1.0, 1.0, 1.0, 0.22]),
            ([right, y], [1.0, 1.0, 1.0, 0.22]),
            ([right - lean * 0.6, y + h * 0.4], [1.0, 1.0, 1.0, 0.0]),
            ([x + lean * 0.4, y + h * 0.4], [1.0, 1.0, 1.0, 0.0]),
        );
        if h >= 9.0 {
            let mut s = 14.0;
            while s < filled - 3.0 {
                segment(
                    v,
                    [x + s + lean, y + 1.0],
                    [x + s, y + h - 1.0],
                    1.6,
                    [0.0, 0.0, 0.0, 0.32],
                );
                s += 14.0;
            }
        }
    }
    slant_outline(v, r, lean, 1.0, [1.0, 1.0, 1.0, 0.22]);
}

/// The icon at `c`, `size` wide, in `ink` on `face`.
pub(super) fn icon(v: &mut Vec<Vertex>, ic: Icon, c: [f32; 2], size: f32, ink: Rgba, face: Rgba) {
    for part in prompt_icons::shape(ic) {
        let points: Vec<[f32; 2]> = part
            .points
            .iter()
            .map(|p| [c[0] + p[0] * size, c[1] + p[1] * size])
            .collect();
        let color = if part.ink > 0.5 { ink } else { face };
        for [a, b, d] in prompt_icons::triangulate(&points) {
            triangle(v, points[a], points[b], points[d], color);
        }
    }
}

/// A round badge holding an icon in its own colour.
pub(super) fn icon_disc(
    v: &mut Vec<Vertex>,
    ic: Icon,
    c: [f32; 2],
    r: f32,
    color: Rgba,
    lit: bool,
) {
    let base = [color[0] * 0.14, color[1] * 0.14, color[2] * 0.14, 1.0];
    disc(v, [c[0] + 1.0, c[1] + 2.5], r + 1.5, [0.0, 0.0, 0.0, 0.34]);
    disc(
        v,
        c,
        r,
        alpha(
            mix([0.012, 0.012, 0.02, 1.0], base, if lit { 1.6 } else { 1.0 }),
            0.97,
        ),
    );
    ring(
        v,
        c,
        r - 1.0,
        1.7,
        alpha(color, if lit { 1.0 } else { 0.62 }),
    );
    let ink = if lit {
        mix(color, [1.0, 1.0, 1.0, 1.0], 0.3)
    } else {
        color
    };
    icon(v, ic, c, r * 1.16, ink, alpha(base, 1.0));
}

/// Altitude ramp shared by the HUD gauge and the map: deep blue for the
/// lowest floors, teal in the middle, warm amber for the highest.
pub(super) fn altitude_color(t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let (low, mid, high) = ([0.05, 0.14, 0.50], [0.06, 0.50, 0.42], [0.95, 0.60, 0.16]);
    let (low, mid, high) = (
        [low[0], low[1], low[2], 1.0],
        [mid[0], mid[1], mid[2], 1.0],
        [high[0], high[1], high[2], 1.0],
    );
    if t < 0.5 {
        mix(low, mid, t * 2.0)
    } else {
        mix(mid, high, t * 2.0 - 1.0)
    }
}

/// A vertical strip in the altitude ramp, high at the top.
pub(super) fn altitude_gauge(v: &mut Vec<Vertex>, r: [f32; 4]) {
    let [x, y, w, h] = r;
    let slices = 10;
    for i in 0..slices {
        let (a, b) = (i as f32 / slices as f32, (i + 1) as f32 / slices as f32);
        let (ca, cb) = (altitude_color(1.0 - a), altitude_color(1.0 - b));
        let (ya, yb) = (y + h * a, y + h * b);
        quad(
            v,
            ([x, ya], ca),
            ([x + w, ya], ca),
            ([x + w, yb], cb),
            ([x, yb], cb),
        );
    }
    let e = w * 0.5;
    disc(v, [x + e, y], e, altitude_color(1.0));
    disc(v, [x + e, y + h], e, altitude_color(0.0));
}

/// A small filled triangle pointing up (`dir` = -1) or down (`dir` = 1).
pub(super) fn caret(v: &mut Vec<Vertex>, c: [f32; 2], size: f32, dir: f32, color: Rgba) {
    triangle(
        v,
        [c[0], c[1] + dir * size * 0.6],
        [c[0] - size * 0.8, c[1] - dir * size * 0.5],
        [c[0] + size * 0.8, c[1] - dir * size * 0.5],
        color,
    );
}

/// A little four-point diamond.
pub(super) fn diamond(v: &mut Vec<Vertex>, c: [f32; 2], r: f32, color: Rgba) {
    fill_flat(
        v,
        &[
            [c[0], c[1] - r],
            [c[0] + r, c[1]],
            [c[0], c[1] + r],
            [c[0] - r, c[1]],
        ],
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sane(v: &[Vertex]) {
        assert!(v.len() % 3 == 0);
        assert!(v
            .iter()
            .all(|v| v.p.into_iter().chain(v.color).all(f32::is_finite)));
    }

    #[test]
    fn rounded_paths_stay_inside_their_rectangle_and_run_clockwise() {
        let r = [40.0, 30.0, 300.0, 90.0];
        let path = rounded_path(r, 22.0);
        assert!(path.iter().all(|p| p[0] >= r[0] - 0.01
            && p[0] <= r[0] + r[2] + 0.01
            && p[1] >= r[1] - 0.01
            && p[1] <= r[1] + r[3] + 0.01));
        // Clockwise on screen (y down) means a positive shoelace sum.
        let area: f32 = (0..path.len())
            .map(|i| {
                let (a, b) = (path[i], path[(i + 1) % path.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum();
        assert!(area > 0.0);
    }

    #[test]
    fn panes_discs_and_meters_are_finite() {
        let tone = Tone::new([0.5, 0.3, 0.8]);
        let mut v = Vec::new();
        pane(&mut v, [36.0, 36.0, 600.0, 116.0], 22.0, &tone, 1.0);
        disc(&mut v, [100.0, 100.0], 30.0, IVORY);
        ring(&mut v, [100.0, 100.0], 30.0, 2.0, IVORY);
        meter(
            &mut v,
            [60.0, 200.0, 400.0, 18.0],
            0.6,
            CRIMSON,
            [1.0, 0.3, 0.1, 1.0],
        );
        icon_disc(
            &mut v,
            Icon::Heart,
            [80.0, 300.0],
            26.0,
            [1.0, 0.3, 0.4, 1.0],
            true,
        );
        dashes(&mut v, [0.0, 0.0], [300.0, 40.0], 1.0, 8.0, 6.0, IVORY);
        sane(&v);
        assert!(v.len() < 6000);
    }

    #[test]
    fn tone_accent_is_bright_enough_for_any_biome() {
        for accent in [[0.0, 0.0, 0.0], [0.05, 0.02, 0.4], [1.0, 1.0, 1.0]] {
            let tone = Tone::new(accent);
            assert!(tone.accent[..3].iter().cloned().fold(0.0, f32::max) > 0.3);
            assert!(tone.glass[..3].iter().all(|c| *c < 0.1));
        }
    }
}
