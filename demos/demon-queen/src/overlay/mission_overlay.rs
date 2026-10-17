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
    let accent = if view.main { GOLD_HI } else { LILAC };
    // A slanted dark plate (Persona's), a wedge of the mission's colour at its left.
    let lean = 8.0;
    let plate = |x: f32, y: f32, w: f32, h: f32| -> [[f32; 2]; 4] {
        [
            [x + lean, y],
            [x + w + lean, y],
            [x + w - lean, y + h],
            [x - lean, y + h],
        ]
    };
    let fill = |v: &mut Vec<MapVertex>, q: [[f32; 2]; 4], color: [f32; 4]| {
        push_triangle(v, [q[0], q[1], q[2]], color);
        push_triangle(v, [q[0], q[2], q[3]], color);
        push_outline_fringe(v, &q, color);
    };
    fill(
        v,
        plate(r.x + 8.0, r.y + 8.0, r.w, r.h),
        rgba(0x000000, 0.5),
    );
    fill(v, plate(r.x, r.y, r.w, r.h), rgba(0x0e0816, 0.92));
    fill(v, plate(r.x, r.y, 8.0, r.h), rgba(accent, 1.0));
    // The heading on a tag of its colour, J on the right.
    let mut y = r.y + 14.0;
    let heading = if view.main {
        "MISIÓN PRINCIPAL"
    } else {
        "MISIÓN SECUNDARIA"
    };
    let hw = text_width(font, Face::Display, heading, 16.0) + 22.0;
    fill(v, plate(r.x + pad, y - 4.0, hw, 26.0), rgba(accent, 1.0));
    text(
        font,
        v,
        Face::Display,
        heading,
        r.x + pad + 11.0,
        y - 1.0,
        16.0,
        rgba(0x0e0816, 1.0),
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
    text_outline(
        font,
        v,
        Face::Title,
        &view.title,
        r.x + pad,
        y,
        26.0,
        rgba(0xfff6ee, 1.0),
        rgba(0x000000, 0.85),
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
    let accent = if view.main { 0xffcf5a } else { 0xb48cff };
    let fill4 = |v: &mut Vec<MapVertex>, q: [[f32; 2]; 4], color: [f32; 4]| {
        push_triangle(v, [q[0], q[1], q[2]], color);
        push_triangle(v, [q[0], q[2], q[3]], color);
        push_outline_fringe(v, &q, color);
    };
    let slant = |x: f32, y: f32, w: f32, h: f32, lean: f32| -> [[f32; 2]; 4] {
        [
            [x + lean, y],
            [x + w + lean, y],
            [x + w - lean, y + h],
            [x - lean, y + h],
        ]
    };
    // The name on a slanted dark plate, the distance on a chip of the mission's colour.
    let plate = |v: &mut Vec<MapVertex>, cx: f32, top: f32| {
        let ns = 21.0;
        let nw = text_width(font, Face::Title, &view.marker_label, ns);
        let ds = 19.0;
        let dw = if view.marker_distance.is_empty() {
            0.0
        } else {
            text_width(font, Face::Display, &view.marker_distance, ds) + 22.0
        };
        let w = nw + 34.0 + if dw > 0.0 { dw + 10.0 } else { 0.0 };
        let x = cx - w * 0.5;
        fill4(
            v,
            slant(x + 5.0, top + 5.0, w, 34.0, 7.0),
            rgba(0x000000, 0.5),
        );
        fill4(v, slant(x, top, w, 34.0, 7.0), rgba(0x0e0816, 0.92));
        fill4(v, slant(x, top, 6.0, 34.0, 7.0), rgba(accent, 1.0));
        text_outline(
            font,
            v,
            Face::Title,
            &view.marker_label,
            x + 17.0,
            top + 4.0,
            ns,
            rgba(0xfff6ee, 1.0),
            rgba(0x000000, 0.8),
        );
        if dw > 0.0 {
            let dx = x + w - dw - 8.0;
            fill4(v, slant(dx, top + 5.0, dw, 24.0, 5.0), rgba(accent, 1.0));
            text(
                font,
                v,
                Face::Display,
                &view.marker_distance,
                dx + 11.0,
                top + 6.0,
                ds,
                rgba(0x0e0816, 1.0),
            );
        }
    };
    if inside {
        // A bobbing arrow pointing down at it, a turning ring, the gem, its light.
        let bob = (time * 4.0).sin() * 6.0;
        halo(
            v,
            p,
            30.0 + 6.0 * pulse,
            18.0,
            rgba(accent, 0.28 + 0.15 * pulse),
        );
        arc(
            v,
            p,
            24.0,
            3.0,
            time * 0.4,
            0.7,
            rgba(accent, 0.9),
            rgba(accent, 0.15),
        );
        gem(v, p, 13.0, 18.0, accent, 1.0, 8.0);
        let tip = [p[0], p[1] - 34.0 + bob];
        push_triangle(
            v,
            [
                [tip[0], tip[1] + 3.0],
                [tip[0] - 15.0, tip[1] - 22.0],
                [tip[0] + 15.0, tip[1] - 22.0],
            ],
            rgba(0x0e0816, 0.9),
        );
        push_triangle(
            v,
            [
                tip,
                [tip[0] - 11.0, tip[1] - 19.0],
                [tip[0] + 11.0, tip[1] - 19.0],
            ],
            rgba(accent, 1.0),
        );
        plate(v, p[0], p[1] + 30.0);
    } else {
        // The edge arrow, pointing away from the middle of the screen, and the name by it.
        let c = [WINDOW_W * 0.5, WINDOW_H * 0.5];
        let d = [p[0] - c[0], p[1] - c[1]];
        let len = d[0].hypot(d[1]).max(1e-3);
        let (ux, uy) = (d[0] / len, d[1] / len);
        let s = 30.0;
        let push = (time * 5.0).sin() * 4.0;
        let p = [p[0] + ux * push, p[1] + uy * push];
        let tip = [p[0] + ux * s, p[1] + uy * s];
        let left = [
            p[0] - uy * s * 0.8 - ux * s * 0.5,
            p[1] + ux * s * 0.8 - uy * s * 0.5,
        ];
        let right = [
            p[0] + uy * s * 0.8 - ux * s * 0.5,
            p[1] - ux * s * 0.8 - uy * s * 0.5,
        ];
        halo(v, p, 34.0, 18.0, rgba(accent, 0.25 + 0.15 * pulse));
        push_triangle(
            v,
            [
                [tip[0] + ux * 4.0, tip[1] + uy * 4.0],
                [left[0] - ux * 4.0 - uy * 3.0, left[1] - uy * 4.0 + ux * 3.0],
                [
                    right[0] - ux * 4.0 + uy * 3.0,
                    right[1] - uy * 4.0 - ux * 3.0,
                ],
            ],
            rgba(0x0e0816, 0.92),
        );
        push_triangle(v, [tip, left, right], rgba(accent, 1.0));
        // The plate inward of the arrow, kept on screen.
        let at = [p[0] - ux * 70.0, p[1] - uy * 60.0];
        let cx = at[0].clamp(180.0, WINDOW_W - 180.0);
        let mut top = (at[1] - 17.0).clamp(150.0, WINDOW_H - 220.0);
        // Never under the corner map and the mission's panel (top right).
        if cx > WINDOW_W - 640.0 && top < 640.0 {
            top = 640.0;
        }
        plate(v, cx, top);
    }
}

const WINDOW_W: f32 = 1920.0;
const WINDOW_H: f32 = 1080.0;

/// The notice at the top, in the manner of the game's Persona menus: a slanted dark plate slams in
/// from the left with a white streak running over it, its colour wedge at the left holding a gem
/// (a new mission, violet) or a seal (done, gold); its heading on a small tag, the mission's name
/// big with an ink rim; a comic «¡HECHO!» stamped at its end when it is done. It slides off to the
/// right at the end.
pub(super) fn draw_notice(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    view: &MissionView,
    time: f32,
) {
    let Some((heading, title, age)) = view.notice.as_ref() else {
        return;
    };
    let age = *age;
    let come = entrance(age, 0.0, 0.32);
    let go = ((age - (NOTICE_SECS - 0.45)) / 0.45).clamp(0.0, 1.0);
    let alpha = entrance(age, 0.0, 0.12) * (1.0 - go);
    if alpha <= 0.01 {
        return;
    }
    let done = heading == "Misión completada";
    // A place still being made: orange, a warning sign, and a line saying so under it.
    // A note on what to do comes the same way, its line under it.
    let note_line = (heading == "Nota")
        .then(|| title.split_once('\n').map(|(_, l)| l.to_owned()))
        .flatten();
    let title = &title.split('\n').next().unwrap_or(title).to_owned();
    let building = heading == "En desarrollo" || heading == "Nota";
    let main = view.main || done;
    let accent = if building {
        0xff7a2e
    } else if done {
        GOLD
    } else if main {
        0x9a5cff
    } else {
        0x3fc7b0
    };
    let accent_hi = if building {
        0xffc08a
    } else if done {
        GOLD_HI
    } else if main {
        0xd6b8ff
    } else {
        0x9ff0e0
    };
    // Its size: the name decides it.
    let ts = 50.0;
    let name = fit(font, Face::Title, title, ts, 900.0);
    let tw = text_width(font, Face::Title, &name, ts);
    let w = (tw + 230.0).max(560.0);
    let h = 104.0;
    let cx = width * 0.5 - 40.0;
    let x0 = cx - w * 0.5;
    let y0 = 112.0;
    let lean = 26.0;
    let slide = (1.0 - come) * -(w + 300.0) + go * 260.0;
    let plate = |dx: f32, dy: f32, ww: f32| -> [[f32; 2]; 4] {
        [
            [x0 + dx + lean, y0 + dy],
            [x0 + dx + ww + lean, y0 + dy],
            [x0 + dx + ww - lean, y0 + dy + h],
            [x0 + dx - lean, y0 + dy + h],
        ]
    };
    faded(v, alpha, |v| {
        shifted(v, [slide, 0.0], |v| {
            // Its shadow, the plate, its rims.
            let sh = plate(12.0, 12.0, w);
            push_triangle(v, [sh[0], sh[1], sh[2]], rgba(0x000000, 0.55));
            push_triangle(v, [sh[0], sh[2], sh[3]], rgba(0x000000, 0.55));
            let p = plate(0.0, 0.0, w);
            push_triangle(v, [p[0], p[1], p[2]], rgba(0x0e0816, 0.96));
            push_triangle(v, [p[0], p[2], p[3]], rgba(0x0e0816, 0.96));
            push_outline_fringe(v, &p, rgba(0x0e0816, 0.96));
            // The colour wedge at its left with its sign.
            let wedge = plate(0.0, 0.0, 120.0);
            push_triangle(v, [wedge[0], wedge[1], wedge[2]], rgba(accent, 1.0));
            push_triangle(
                v,
                [wedge[0], wedge[2], wedge[3]],
                rgba(mix(accent, 0x000000, 0.25), 1.0),
            );
            push_outline_fringe(v, &wedge, rgba(accent, 1.0));
            let sign = [x0 + 58.0, y0 + h * 0.5];
            if building {
                // A warning sign: a dark triangle with a light one in it and a «!».
                let tri = |r: f32| {
                    [
                        [sign[0], sign[1] - r],
                        [sign[0] + r * 0.95, sign[1] + r * 0.72],
                        [sign[0] - r * 0.95, sign[1] + r * 0.72],
                    ]
                };
                push_triangle(v, tri(34.0), rgba(0x0e0816, 0.95));
                push_triangle(v, tri(27.0), rgba(0xffe2c4, 1.0));
                text(
                    font,
                    v,
                    Face::Display,
                    "!",
                    sign[0] - 5.0,
                    sign[1] - 15.0,
                    30.0,
                    rgba(0x0e0816, 1.0),
                );
            } else if done {
                // A seal: a gold disc, its rim and a tick.
                disc(v, sign, 30.0, rgba(0x0e0816, 0.9));
                disc(v, sign, 26.0, rgba(GOLD_HI, 1.0));
                arc(v, sign, 21.0, 2.5, 0.0, 1.0, rgba(GOLD_DEEP, 1.0), [0.0; 4]);
                wire(
                    v,
                    [sign[0] - 11.0, sign[1] + 1.0],
                    [sign[0] - 3.0, sign[1] + 9.0],
                    6.0,
                    rgba(0x0e0816, 1.0),
                    rgba(0x0e0816, 1.0),
                    0.0,
                    false,
                );
                wire(
                    v,
                    [sign[0] - 3.0, sign[1] + 9.0],
                    [sign[0] + 12.0, sign[1] - 9.0],
                    6.0,
                    rgba(0x0e0816, 1.0),
                    rgba(0x0e0816, 1.0),
                    0.0,
                    false,
                );
            } else {
                let pulse = 0.5 + 0.5 * (time * 3.0).sin();
                halo(v, sign, 30.0, 14.0, rgba(accent_hi, 0.25 + 0.2 * pulse));
                gem(
                    v,
                    sign,
                    20.0,
                    27.0,
                    mix(accent, 0xffffff, 0.2),
                    1.0,
                    0.6 + 0.4 * pulse,
                );
            }
            // Gold lines along its top and foot.
            let line = |v: &mut Vec<MapVertex>, a: [f32; 2], b: [f32; 2]| {
                wire(
                    v,
                    a,
                    b,
                    3.0,
                    rgba(accent_hi, 0.0),
                    rgba(accent_hi, 0.95),
                    0.0,
                    false,
                );
            };
            line(
                v,
                [p[0][0] + 120.0, p[0][1] + 6.0],
                [p[1][0] - 10.0, p[1][1] + 6.0],
            );
            line(
                v,
                [p[3][0] + 120.0, p[3][1] - 6.0],
                [p[2][0] - 10.0, p[2][1] - 6.0],
            );
            // The heading on its tag.
            let caps = heading.to_uppercase();
            let hs = 19.0;
            let hw = text_width(font, Face::Display, &caps, hs) + 30.0;
            let tag = [x0 + 150.0, y0 - 16.0];
            let tq = [
                [tag[0] + 8.0, tag[1]],
                [tag[0] + hw + 8.0, tag[1]],
                [tag[0] + hw - 8.0, tag[1] + 30.0],
                [tag[0] - 8.0, tag[1] + 30.0],
            ];
            push_triangle(v, [tq[0], tq[1], tq[2]], rgba(accent_hi, 1.0));
            push_triangle(v, [tq[0], tq[2], tq[3]], rgba(accent_hi, 1.0));
            push_outline_fringe(v, &tq, rgba(accent_hi, 1.0));
            text(
                font,
                v,
                Face::Display,
                &caps,
                tag[0] + 15.0,
                tag[1] + 3.0,
                hs,
                rgba(0x0e0816, 1.0),
            );
            // The name.
            text_outline(
                font,
                v,
                Face::Title,
                &name,
                x0 + 150.0,
                y0 + h * 0.5 - ts * 0.52,
                ts,
                rgba(0xfff6ee, 1.0),
                rgba(0x000000, 0.9),
            );
            // A white streak running over it as it lands.
            let k = ((age - 0.18) / 0.5).clamp(0.0, 1.0);
            if k > 0.0 && k < 1.0 {
                let sx = x0 - 60.0 + (w + 120.0) * k;
                let q = [
                    [sx + lean + 40.0, y0],
                    [sx + lean + 90.0, y0],
                    [sx - lean + 50.0, y0 + h],
                    [sx - lean, y0 + h],
                ];
                let a = 0.35 * (1.0 - (k * 2.0 - 1.0).abs());
                push_triangle(v, [q[0], q[1], q[2]], rgba(0xffffff, a));
                push_triangle(v, [q[0], q[2], q[3]], rgba(0xffffff, a));
            }
        });
    });
    // Being made: when it comes, under the plate.
    if building {
        let line = note_line
            .as_deref()
            .unwrap_or("Llegará en una próxima versión.");
        let ls = {
            let w = text_width(font, Face::Body, line, 21.0);
            if w > 1100.0 {
                21.0 * 1100.0 / w
            } else {
                21.0
            }
        };
        let lw = text_width(font, Face::Body, line, ls);
        let at = [x0 + 150.0 + slide, y0 + h + 14.0];
        faded(v, alpha, |v| {
            let q = [
                [at[0] - 12.0, at[1]],
                [at[0] + lw + 24.0, at[1]],
                [at[0] + lw + 14.0, at[1] + 34.0],
                [at[0] - 22.0, at[1] + 34.0],
            ];
            push_triangle(v, [q[0], q[1], q[2]], rgba(0x0e0816, 0.9));
            push_triangle(v, [q[0], q[2], q[3]], rgba(0x0e0816, 0.9));
            text(
                font,
                v,
                Face::Body,
                line,
                at[0],
                at[1] + 5.0,
                ls,
                rgba(0xffe2c4, 1.0),
            );
        });
    }
    // Done: «¡HECHO!» stamped at its end, a beat after it lands.
    if done {
        let t = age - 0.35;
        if t > 0.0 {
            let pop = if t < 0.16 {
                1.0 + 1.2 * (1.0 - t / 0.16)
            } else {
                1.0
            };
            let c = [x0 + w + 10.0 + slide, y0 + h * 0.5 + 6.0];
            faded(v, alpha * (t / 0.08).min(1.0), |v| {
                turned(v, c, -0.16, |v| {
                    comic_stamp(
                        font,
                        v,
                        Face::Display,
                        "¡HECHO!",
                        c,
                        46.0 * pop,
                        GOLD_HI,
                        false,
                        (t / 0.3).min(1.0),
                        time,
                    );
                });
            });
        }
    }
}
