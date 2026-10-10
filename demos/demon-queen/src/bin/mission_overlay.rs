//! Las misiones en pantalla: el panel de la misión seguida bajo el minimapa (su título, el paso, la
//! distancia y las otras abiertas), el camino dorado en los mapas, el rombo sobre el destino (o una
//! flecha en el borde si queda fuera de la vista) y el aviso de «Nueva misión» / «Misión completada».
//! Lo que dice y dónde está lo calcula el mundo (`explore/guide.rs`).
use super::persona::*;
use super::ui_paint::*;
use super::*;

/// How long a notice stays up (as `missions::NOTICE_SECS`).
const NOTICE_SECS: f32 = 4.2;

/// What the missions show this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MissionView {
    pub title: String,
    pub step: String,
    pub detail: String,
    /// Part of the story (else a side errand).
    pub main: bool,
    /// "34 m", "Aquí", or the zone it is in ("En el bosque").
    pub distance: String,
    /// The other open missions.
    pub others: Vec<String>,
    /// J changes the followed one.
    pub can_cycle: bool,
    /// The way (world x, z) and where it ends.
    pub route: Vec<[f32; 2]>,
    pub target: Option<[f32; 2]>,
    /// The target on the screen, and whether it is in view (else the point is on the edge).
    pub marker: Option<([f32; 2], bool)>,
    pub marker_label: String,
    pub marker_distance: String,
    /// A notice: its heading, the mission's title, seconds since it came.
    pub notice: Option<(String, String, f32)>,
}

impl Hash for MissionView {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let q = |x: f32| (x * 4.0).round() as i32;
        self.title.hash(h);
        self.step.hash(h);
        self.detail.hash(h);
        self.main.hash(h);
        self.distance.hash(h);
        self.others.hash(h);
        self.can_cycle.hash(h);
        for p in &self.route {
            (q(p[0]), q(p[1])).hash(h);
        }
        self.target.map(|t| (q(t[0]), q(t[1]))).hash(h);
        self.marker
            .map(|(p, inside)| (p[0].round() as i32, p[1].round() as i32, inside))
            .hash(h);
        self.marker_label.hash(h);
        self.marker_distance.hash(h);
        self.notice
            .as_ref()
            .map(|(a, b, t)| (a, b, (t * 30.0) as i32))
            .hash(h);
    }
}

impl MissionView {
    /// Whether anything moves on it (a notice coming and going).
    pub fn animating(&self) -> bool {
        self.notice.is_some()
    }
}

/// A world point (x, z) on a map drawn north up (the maps take -z as their z), not clamped.
fn on_map(map: Rect, b: MapBounds, x: f32, z: f32, flip: bool) -> [f32; 2] {
    let z = if flip { -z } else { z };
    let tx = (x - b.min_x) / (b.max_x - b.min_x).max(1e-3);
    let tz = (z - b.min_z) / (b.max_z - b.min_z).max(1e-3);
    [map.x + tx * map.w, map.y + (1.0 - tz) * map.h]
}

/// The part of a → b inside `r` (Liang–Barsky), if any.
fn clip(a: [f32; 2], b: [f32; 2], r: Rect) -> Option<([f32; 2], [f32; 2])> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [
        (-dx, a[0] - r.x),
        (dx, r.x + r.w - a[0]),
        (-dy, a[1] - r.y),
        (dy, r.y + r.h - a[1]),
    ] {
        if p.abs() < 1e-6 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((
        [a[0] + dx * t0, a[1] + dy * t0],
        [a[0] + dx * t1, a[1] + dy * t1],
    ))
}

/// The way on a map: a gold line with a dark edge, and the target's diamond (on the map's edge,
/// pointing out, when it lies beyond what the corner map shows).
pub(super) fn draw_route(
    v: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    view: &MissionView,
    expanded: bool,
    flip: bool,
) {
    let inner = Rect::new(map.x + 3.0, map.y + 3.0, map.w - 6.0, map.h - 6.0);
    let points: Vec<[f32; 2]> = view
        .route
        .iter()
        .map(|p| on_map(map, bounds, p[0], p[1], flip))
        .collect();
    let (edge, line) = if expanded { (9.0, 5.0) } else { (7.0, 3.6) };
    for pass in 0..2 {
        for w in points.windows(2) {
            let Some((a, b)) = clip(w[0], w[1], inner) else {
                continue;
            };
            if pass == 0 {
                wire(
                    v,
                    a,
                    b,
                    edge,
                    rgba(SHADOW, 0.85),
                    rgba(SHADOW, 0.85),
                    0.0,
                    false,
                );
            } else {
                wire(
                    v,
                    a,
                    b,
                    line,
                    rgba(GOLD_HI, 1.0),
                    rgba(GOLD_HI, 1.0),
                    4.0,
                    false,
                );
            }
        }
    }
    let Some(t) = view.target else {
        return;
    };
    let at = on_map(map, bounds, t[0], t[1], flip);
    let pad = 14.0;
    let inside = at[0] >= inner.x + pad
        && at[0] <= inner.x + inner.w - pad
        && at[1] >= inner.y + pad
        && at[1] <= inner.y + inner.h - pad;
    let size = if expanded { 13.0 } else { 10.0 };
    if inside {
        halo(v, at, size * 1.6, size, rgba(GOLD_HI, 0.45));
        gem(v, at, size * 0.8, size, 0xffcf5a, 1.0, 6.0);
    } else {
        // On the edge, an arrowhead pointing the way out.
        let c = [inner.x + inner.w * 0.5, inner.y + inner.h * 0.5];
        let d = [at[0] - c[0], at[1] - c[1]];
        let k = ((d[0].abs() / (inner.w * 0.5 - pad)).max(d[1].abs() / (inner.h * 0.5 - pad)))
            .max(1e-3);
        let p = [c[0] + d[0] / k, c[1] + d[1] / k];
        let len = d[0].hypot(d[1]).max(1e-3);
        let (ux, uy) = (d[0] / len, d[1] / len);
        let s = size * 1.25;
        let tip = [p[0] + ux * s, p[1] + uy * s];
        let left = [
            p[0] - uy * s * 0.75 - ux * s * 0.4,
            p[1] + ux * s * 0.75 - uy * s * 0.4,
        ];
        let right = [
            p[0] + uy * s * 0.75 - ux * s * 0.4,
            p[1] - ux * s * 0.75 - uy * s * 0.4,
        ];
        halo(v, p, s * 1.4, s, rgba(GOLD_HI, 0.35));
        push_triangle(
            v,
            [
                [tip[0] + ux * 2.5, tip[1] + uy * 2.5],
                [left[0] - ux * 2.0, left[1] - uy * 2.0],
                [right[0] - ux * 2.0, right[1] - uy * 2.0],
            ],
            rgba(SHADOW, 0.9),
        );
        push_triangle(v, [tip, left, right], rgba(0xffcf5a, 1.0));
    }
}

/// The panel under the corner map: the followed mission, its step, how far, the others.
pub(super) fn draw_panel(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &MissionView,
) {
    if view.title.is_empty() {
        return;
    }
    let map = map_kit::compact_rect(width, height);
    let (pad, w) = (18.0, map.w);
    let others = view.others.len().min(2) as f32;
    let detail = !view.detail.is_empty();
    let h = 118.0
        + if detail { 24.0 } else { 0.0 }
        + if others > 0.0 {
            12.0 + others * 25.0
        } else {
            0.0
        };
    let r = Rect::new(map.x, map.y + map.h + 12.0, w, h);
    soft(v, r, 14.0, 10.0, rgba(SHADOW, 0.55), false);
    panel(v, r, 14.0, Panel::velvet(0.88, 1.6).hairline(5.0, 0.35));
    let accent = if view.main { GOLD_HI } else { LILAC };
    // The heading: a gem, its kind, and J on the right.
    let mut y = r.y + 14.0;
    gem(
        v,
        [r.x + pad + 7.0, y + 9.0],
        6.0,
        8.5,
        if view.main { 0xffcf5a } else { 0xb48cff },
        1.0,
        4.0,
    );
    let heading = if view.main {
        "MISIÓN PRINCIPAL"
    } else {
        "MISIÓN SECUNDARIA"
    };
    text(
        font,
        v,
        Face::Display,
        heading,
        r.x + pad + 22.0,
        y,
        16.0,
        rgba(accent, 0.95),
    );
    if view.can_cycle {
        let key = Rect::new(r.x + w - pad - 34.0, y - 4.0, 34.0, 28.0);
        keycap(font, v, key, "J", 16.0, false, false, GOLD, 1.0);
        let label = "cambiar";
        let lw = text_width(font, Face::Ui, label, 15.0);
        text(
            font,
            v,
            Face::Ui,
            label,
            key.x - lw - 8.0,
            y + 1.0,
            15.0,
            rgba(DIM, 0.95),
        );
    }
    y += 28.0;
    text_shadow(
        font,
        v,
        Face::Title,
        &view.title,
        r.x + pad,
        y,
        25.0,
        rgba(CREAM, 1.0),
    );
    y += 38.0;
    // The step, and how far (or where) on the right.
    let dist_w = if view.distance.is_empty() {
        0.0
    } else {
        text_width(font, Face::Title, &view.distance, 20.0)
    };
    chevron_right(v, [r.x + pad + 5.0, y + 11.0], 9.0, rgba(accent, 1.0));
    let step_room = w - pad * 2.0 - 20.0 - dist_w - 14.0;
    let step = fit(font, Face::Body, &view.step, 20.0, step_room);
    text_shadow(
        font,
        v,
        Face::Body,
        &step,
        r.x + pad + 20.0,
        y,
        20.0,
        rgba(INK, 1.0),
    );
    if dist_w > 0.0 {
        text_shadow(
            font,
            v,
            Face::Title,
            &view.distance,
            r.x + w - pad - dist_w,
            y,
            20.0,
            rgba(accent, 1.0),
        );
    }
    y += 28.0;
    if detail {
        let d = fit(font, Face::Italic, &view.detail, 17.0, w - pad * 2.0 - 20.0);
        text(
            font,
            v,
            Face::Italic,
            &d,
            r.x + pad + 20.0,
            y,
            17.0,
            rgba(DIM, 1.0),
        );
        y += 24.0;
    }
    if others > 0.0 {
        y += 4.0;
        wire(
            v,
            [r.x + pad, y],
            [r.x + w - pad, y],
            1.2,
            rgba(GOLD_DEEP, 0.0),
            rgba(GOLD_DEEP, 0.8),
            0.0,
            false,
        );
        y += 9.0;
        for title in view.others.iter().take(2) {
            disc(v, [r.x + pad + 6.0, y + 9.0], 3.0, rgba(DIM, 0.9));
            let t = fit(font, Face::Ui, title, 16.0, w - pad * 2.0 - 20.0);
            text(
                font,
                v,
                Face::Ui,
                &t,
                r.x + pad + 18.0,
                y,
                16.0,
                rgba(DIM, 0.95),
            );
            y += 25.0;
        }
    }
}

/// A small arrowhead pointing right.
fn chevron_right(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, color: [f32; 4]) {
    let (w, h) = (size * 0.5, size * 0.62);
    wire(
        v,
        [c[0] - w, c[1] - h],
        [c[0] + w, c[1]],
        size * 0.32,
        color,
        color,
        0.0,
        false,
    );
    wire(
        v,
        [c[0] - w, c[1] + h],
        [c[0] + w, c[1]],
        size * 0.32,
        color,
        color,
        0.0,
        false,
    );
}

/// `s` cut with an ellipsis to fit `max` pixels.
fn fit(font: &FontAtlas, face: Face, s: &str, size: f32, max: f32) -> String {
    if text_width(font, face, s, size) <= max {
        return s.to_owned();
    }
    let mut out: String = s.to_owned();
    while !out.is_empty() && text_width(font, face, &format!("{out}…"), size) > max {
        out.pop();
    }
    format!("{}…", out.trim_end())
}

/// The mark over the target in the world: a diamond with its name and distance under it, or, out
/// of view, an arrow on the screen's edge pointing to it.
pub(super) fn draw_marker(font: &FontAtlas, v: &mut Vec<MapVertex>, view: &MissionView, time: f32) {
    let Some((p, inside)) = view.marker else {
        return;
    };
    let pulse = 0.5 + 0.5 * (time * 3.0).sin();
    if inside {
        halo(
            v,
            p,
            22.0 + 4.0 * pulse,
            14.0,
            rgba(GOLD_HI, 0.30 + 0.15 * pulse),
        );
        gem(v, p, 11.0, 15.0, 0xffcf5a, 1.0, 8.0);
        let label = if view.marker_distance.is_empty() {
            view.marker_label.clone()
        } else {
            format!("{} · {}", view.marker_label, view.marker_distance)
        };
        let size = 17.0;
        let lw = text_width(font, Face::Ui, &label, size);
        let plate = Rect::new(p[0] - lw * 0.5 - 12.0, p[1] + 22.0, lw + 24.0, size + 12.0);
        panel(
            v,
            plate,
            plate.h * 0.5,
            Panel::fill(0x1a0c22, 0x0a0410, 0.82).rim(GOLD, 0.8, 1.2),
        );
        text(
            font,
            v,
            Face::Ui,
            &label,
            plate.x + 12.0,
            plate.y + 5.0,
            size,
            rgba(CREAM, 1.0),
        );
    } else {
        // The edge arrow, pointing away from the middle of the screen.
        let c = [WINDOW_W * 0.5, WINDOW_H * 0.5];
        let d = [p[0] - c[0], p[1] - c[1]];
        let len = d[0].hypot(d[1]).max(1e-3);
        let (ux, uy) = (d[0] / len, d[1] / len);
        let s = 20.0;
        let tip = [p[0] + ux * s, p[1] + uy * s];
        let left = [
            p[0] - uy * s * 0.8 - ux * s * 0.5,
            p[1] + ux * s * 0.8 - uy * s * 0.5,
        ];
        let right = [
            p[0] + uy * s * 0.8 - ux * s * 0.5,
            p[1] - ux * s * 0.8 - uy * s * 0.5,
        ];
        halo(v, p, 26.0, 16.0, rgba(GOLD_HI, 0.25 + 0.15 * pulse));
        push_triangle(
            v,
            [
                [tip[0] + ux * 3.0, tip[1] + uy * 3.0],
                [left[0] - ux * 3.0, left[1] - uy * 3.0],
                [right[0] - ux * 3.0, right[1] - uy * 3.0],
            ],
            rgba(SHADOW, 0.9),
        );
        push_triangle(v, [tip, left, right], rgba(0xffcf5a, 1.0));
        if !view.marker_distance.is_empty() {
            let size = 16.0;
            let lw = text_width(font, Face::Title, &view.marker_distance, size);
            let at = [p[0] - ux * 34.0, p[1] - uy * 34.0];
            text_outline(
                font,
                v,
                Face::Title,
                &view.marker_distance,
                at[0] - lw * 0.5,
                at[1] - size * 0.6,
                size,
                rgba(CREAM, 1.0),
                rgba(SHADOW, 0.9),
            );
        }
    }
}

const WINDOW_W: f32 = 1920.0;
const WINDOW_H: f32 = 1080.0;

/// The notice at the top: a band across with its heading and the mission's name, sliding in and
/// fading out.
pub(super) fn draw_notice(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    view: &MissionView,
) {
    let Some((heading, title, age)) = view.notice.as_ref() else {
        return;
    };
    let fade_in = entrance(*age, 0.0, 0.35);
    let fade_out = 1.0 - ((age - (NOTICE_SECS - 0.6)) / 0.6).clamp(0.0, 1.0);
    let alpha = fade_in * fade_out;
    if alpha <= 0.01 {
        return;
    }
    let done = heading == "Misión completada";
    let ink = if done { GOLD_HI } else { LILAC };
    let cy = 168.0;
    faded(v, alpha, |v| {
        shifted(v, [(1.0 - fade_in) * -60.0, 0.0], |v| {
            // Clear of the corner map (it starts some 535 px from the right edge).
            slash(
                v,
                [width * 0.5, cy + 6.0],
                780.0,
                112.0,
                -0.02,
                rgba(0x2a0f38, 0.92),
                rgba(0x0e0616, 0.92),
            );
            slash(
                v,
                [width * 0.5, cy + 62.0],
                800.0,
                4.0,
                -0.02,
                rgba(GOLD, 0.9),
                rgba(GOLD, 0.9),
            );
            let caps = heading.to_uppercase();
            let hs = 22.0;
            let hw = text_width(font, Face::Display, &caps, hs);
            text(
                font,
                v,
                Face::Display,
                &caps,
                width * 0.5 - hw * 0.5,
                cy - 38.0,
                hs,
                rgba(ink, 1.0),
            );
            let ts = 46.0;
            let tw = text_width(font, Face::Title, title, ts);
            shout(
                font,
                v,
                Face::Title,
                title,
                width * 0.5 - tw * 0.5,
                cy - 8.0,
                ts,
                0xfff6ee,
                if done { 0x8a6436 } else { 0x5a2a8a },
                1.0,
            );
            for side in [-1.0f32, 1.0] {
                sparkle(
                    v,
                    [width * 0.5 + side * (tw * 0.5 + 34.0), cy + 16.0],
                    9.0,
                    rgba(ink, 1.0),
                    6.0,
                );
            }
        });
    });
}
