//! The 2D half of the elixir going down, over the 3D close shot: a slash of crimson wipes across
//! while the shot cuts to her, black bars close in like a film, a plate names the elixir, and as
//! she drinks the words of a comic pop up beside her: «¡POP!» at the cork, «glup» with each gulp,
//! «¡Ah~!» when the flask comes away, pink sparks rising from it. Then the seal wakes: a flash, its
//! ring of runes turning on her belly, a banner across the screen, and the key to go on.
use super::persona::*;
use super::ui_paint::*;
use super::*;

/// What the layer shows this frame (positions on the 1080p screen, from the 3D shot).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct DrinkView {
    /// Since it began (ms).
    pub ms: u32,
    /// Into her clip, once it plays (ms).
    pub clip_ms: Option<u32>,
    /// Since the clip ended: the seal waking (ms).
    pub awake_ms: Option<u32>,
    /// A key closes it now.
    pub ready: bool,
    /// Her lips, the flask and her belly on screen (when in view).
    pub lips: Option<[i32; 2]>,
    pub flask: Option<[i32; 2]>,
    pub belly: Option<[i32; 2]>,
    /// The welcome one (the first she buys).
    pub first: bool,
}

const PINK: u32 = 0xff4f9a;

fn point(p: Option<[i32; 2]>, fallback: [f32; 2]) -> [f32; 2] {
    p.map_or(fallback, |p| [p[0] as f32, p[1] as f32])
}

/// A word popping out beside her: it springs to size, shakes a little, and fades.
#[allow(clippy::too_many_arguments)]
fn pop_word(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    words: &str,
    at: [f32; 2],
    age: f32,
    life: f32,
    size: f32,
    ink: u32,
    edge: u32,
    angle: f32,
    burst_fill: Option<u32>,
    s: f32,
) {
    if age < 0.0 || age > life {
        return;
    }
    let grow_in = ease_out(age / 0.16);
    let spring = 1.0 + 0.18 * (1.0 - grow_in) * (age * 40.0).sin().abs();
    let alpha = (1.0 - ((age - (life - 0.25)) / 0.25).clamp(0.0, 1.0)) * grow_in.min(1.0);
    let size = size * (0.6 + 0.4 * grow_in) * spring;
    let width = text_width(font, Face::Title, words, size);
    faded(v, alpha, |v| {
        turned(v, at, angle, |v| {
            if let Some(fill) = burst_fill {
                burst(
                    v,
                    at,
                    width * 0.42,
                    width * 0.72,
                    11,
                    age * 0.8,
                    rgba(fill, 1.0),
                    rgba(0x0b0508, 1.0),
                    5.0 * s,
                );
            }
            shout(
                font,
                v,
                Face::Title,
                words,
                at[0] - width * 0.5,
                at[1] - size * 0.62,
                size,
                ink,
                edge,
                1.0,
            );
        });
    });
}

pub(super) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &DrinkView,
    time: f32,
) {
    let s = h / 1080.0;
    let c = view.ms as f32 / 1000.0;
    let lead = super::super::drink::LEAD;

    // The film's bars close in.
    let bars = ease_out(c / 0.45);
    let bar_h = 86.0 * s * bars;
    push_rect(v, Rect::new(0.0, 0.0, w, bar_h), rgba(0x020103, 0.94));
    push_rect(v, Rect::new(0.0, h - bar_h, w, bar_h), rgba(0x020103, 0.94));
    if bars > 0.0 {
        push_rect(
            v,
            Rect::new(0.0, bar_h - 3.0 * s, w, 3.0 * s),
            rgba(CRIMSON, 0.85 * bars),
        );
        push_rect(
            v,
            Rect::new(0.0, h - bar_h, w, 3.0 * s),
            rgba(CRIMSON, 0.85 * bars),
        );
    }

    // The plate naming it, sliding in from the left.
    let plate_a = entrance(c, lead, 0.35);
    if plate_a > 0.0 {
        faded(v, plate_a, |v| {
            shifted(v, [-(1.0 - plate_a) * 420.0 * s, 0.0], |v| {
                let r = Rect::new(-30.0 * s, 120.0 * s, 640.0 * s, 118.0 * s);
                turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], -0.06, |v| {
                    panel(
                        v,
                        grow(r, 5.0 * s),
                        6.0 * s,
                        Panel::fill(0xffffff, 0xffffff, 1.0),
                    );
                    panel(v, r, 4.0 * s, Panel::fill(0x14040c, 0x0b0508, 1.0));
                    panel(
                        v,
                        Rect::new(r.x, r.y + r.h - 14.0 * s, r.w, 14.0 * s),
                        0.0,
                        Panel::fill(PINK, 0xc0205a, 1.0),
                    );
                    shout(
                        font,
                        v,
                        Face::Title,
                        "ELIXIR DE ROSALÍA",
                        r.x + 70.0 * s,
                        r.y + 14.0 * s,
                        48.0 * s,
                        0xffffff,
                        0xc0205a,
                        1.0,
                    );
                    let sub = if view.first {
                        "Oferta de bienvenida · receta secreta de la casa"
                    } else {
                        "Receta secreta de la casa"
                    };
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        sub,
                        r.x + 74.0 * s,
                        r.y + 70.0 * s,
                        23.0 * s,
                        rgba(0xffc8dc, 1.0),
                    );
                });
            });
        });
    }

    // The words of the comic, beside her.
    if let Some(ms) = view.clip_ms {
        let t = ms as f32 / 1000.0;
        let flask = point(view.flask, [w * 0.55, h * 0.5]);
        let lips = point(view.lips, [w * 0.5, h * 0.42]);
        use super::super::elixir::{AH, GULPS, POP};
        // Away from her face: on the flask's side of it, out past it.
        let away = {
            let d = [flask[0] - lips[0], flask[1] - lips[1]];
            let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1.0);
            [d[0] / l, d[1] / l]
        };
        let side = if away[0] < 0.0 { -1.0 } else { 1.0 };
        pop_word(
            font,
            v,
            "¡POP!",
            [
                flask[0] + away[0] * 200.0 * s,
                flask[1] + away[1] * 120.0 * s - 150.0 * s,
            ],
            t - POP,
            0.8,
            64.0 * s,
            0xffffff,
            0xc0103c,
            -0.18 * side,
            Some(0xffe14a),
            s,
        );
        for (k, g) in GULPS.iter().enumerate() {
            let at = [
                lips[0] + side * (380.0 + [0.0, 70.0, -20.0][k]) * s,
                lips[1] - (10.0 + 105.0 * k as f32) * s,
            ];
            pop_word(
                font,
                v,
                "glup",
                at,
                t - g,
                0.45,
                (58.0 + 8.0 * k as f32) * s,
                0xffffff,
                PINK,
                side * 0.14,
                None,
                s,
            );
        }
        pop_word(
            font,
            v,
            "¡Ah~!",
            [lips[0] + side * 420.0 * s, lips[1] - 60.0 * s],
            t - AH - 0.05,
            1.3,
            80.0 * s,
            0xffffff,
            0xd02a52,
            0.12 * side,
            Some(0xff9ac8),
            s,
        );
        // Pink sparks rising from the flask while it empties.
        if t > POP && t < AH + 0.6 {
            for i in 0..10 {
                let fi = i as f32;
                let phase = (time * (0.6 + 0.07 * fi) + fi * 0.137).fract();
                let x = flask[0] + ((fi * 2.39).sin() * 70.0 + (time * 2.0 + fi).sin() * 10.0) * s;
                let y = flask[1] - phase * 260.0 * s;
                sparkle(
                    v,
                    [x, y],
                    (5.0 + 5.0 * (1.0 - phase)) * s,
                    rgba(if i % 3 == 0 { 0xffffff } else { 0xffa8d0 }, 1.0 - phase),
                    0.7,
                );
            }
        }
        // What she thinks of it (until the seal wakes).
        if t > AH + 0.4 && view.awake_ms.is_none() {
            let a = ((t - AH - 0.4) / 0.4).clamp(0.0, 1.0);
            let line = "(Sabe a fresa… y a algo que no debería saber a nada.)";
            text_centered(
                font,
                v,
                Face::Italic,
                line,
                w * 0.5,
                h - bar_h * 0.5 - 16.0 * s,
                28.0 * s,
                rgba(0xe2d4ff, a),
            );
        }
    }

    // The seal wakes.
    if let Some(ms) = view.awake_ms {
        let t = ms as f32 / 1000.0;
        let flash = (1.0 - t / 0.7).clamp(0.0, 1.0);
        if flash > 0.0 {
            push_rect(
                v,
                Rect::new(0.0, 0.0, w, h),
                rgba(0xffe6f0, 0.55 * flash * flash),
            );
        }
        // On her belly, kept above the film's bar.
        let belly = point(view.belly, [w * 0.5, h * 0.6]);
        let belly = [belly[0], belly[1].min(h - 280.0 * s)];
        let ring = ease_out(t / 0.6);
        let r = (60.0 + 70.0 * ring) * s;
        let beat = heartbeat(time, 96.0);
        halo(
            v,
            belly,
            r * 0.8,
            40.0 * s,
            rgba(PINK, (0.35 + 0.25 * beat) * ring),
        );
        arc(
            v,
            belly,
            r,
            4.0 * s,
            time * 0.25,
            0.92,
            rgba(0xffc8dc, ring),
            rgba(PINK, 0.25 * ring),
        );
        arc(
            v,
            belly,
            r * 0.72,
            3.0 * s,
            -time * 0.4,
            0.8,
            rgba(0xffffff, 0.9 * ring),
            [0.0; 4],
        );
        for k in 0..8 {
            let a = time * 0.25 * std::f32::consts::TAU + k as f32 * std::f32::consts::TAU / 8.0;
            let p = [belly[0] + a.cos() * r, belly[1] + a.sin() * r];
            sparkle(v, p, (7.0 + 5.0 * beat) * s, rgba(0xffffff, ring), 0.8);
        }
        sparkle(
            v,
            belly,
            (26.0 + 10.0 * beat) * s * ring,
            rgba(0xffffff, ring),
            1.0,
        );

        // The banner across the screen.
        let in_a = entrance(t, 0.2, 0.35);
        if in_a > 0.0 {
            shifted(v, [(1.0 - in_a) * w, 0.0], |v| {
                let cy = h * 0.5 + 140.0 * s;
                slash(
                    v,
                    [w * 0.5, cy],
                    w * 1.4,
                    150.0 * s,
                    -0.08,
                    rgba(0x14040c, 0.96),
                    rgba(0x0b0508, 0.96),
                );
                slash(
                    v,
                    [w * 0.5, cy - 82.0 * s],
                    w * 1.4,
                    8.0 * s,
                    -0.08,
                    rgba(PINK, 1.0),
                    rgba(PINK, 1.0),
                );
                slash(
                    v,
                    [w * 0.5, cy + 82.0 * s],
                    w * 1.4,
                    8.0 * s,
                    -0.08,
                    rgba(CRIMSON, 1.0),
                    rgba(CRIMSON, 1.0),
                );
                turned(v, [w * 0.5, cy], -0.08, |v| {
                    let size = 88.0 * s;
                    let words = "EL SELLO DESPIERTA";
                    let ww = text_width(font, Face::Title, words, size);
                    shout(
                        font,
                        v,
                        Face::Title,
                        words,
                        w * 0.5 - ww * 0.5,
                        cy - 66.0 * s,
                        size,
                        0xffffff,
                        0xc0205a,
                        1.0,
                    );
                    let sub = "Esta noche puede tomar forma con las chicas de la casa";
                    text_centered(
                        font,
                        v,
                        Face::Italic,
                        sub,
                        w * 0.5,
                        cy + 34.0 * s,
                        28.0 * s,
                        rgba(0xffb8d4, 1.0),
                    );
                });
            });
        }
        if view.ready {
            let key = Rect::new(
                w - 290.0 * s,
                h - bar_h * 0.5 - 20.0 * s,
                46.0 * s,
                40.0 * s,
            );
            let pulse = 0.85 + 0.15 * wave(time, 0.9);
            keycap(font, v, key, "F", 20.0 * s, false, false, PINK, 1.0);
            text_shadow(
                font,
                v,
                Face::Italic,
                "Continuar",
                key.x + key.w + 14.0 * s,
                key.y + 8.0 * s,
                23.0 * s,
                rgba(CREAM, pulse),
            );
        }
    } else if view.clip_ms.is_some() {
        let key = Rect::new(
            w - 290.0 * s,
            h - bar_h * 0.5 - 20.0 * s,
            64.0 * s,
            40.0 * s,
        );
        keycap(font, v, key, "Esc", 18.0 * s, false, false, PINK, 0.8);
        text_shadow(
            font,
            v,
            Face::Italic,
            "saltar",
            key.x + key.w + 12.0 * s,
            key.y + 8.0 * s,
            21.0 * s,
            rgba(DIM, 1.0),
        );
    }

    // The wipe: a crimson slash across the whole screen while the shot cuts under it.
    if c < lead + 0.1 {
        let u = (c / (lead + 0.1)).clamp(0.0, 1.0);
        let x = -w * 0.9 + u * w * 2.8;
        let cy = h * 0.5;
        let (length, tall) = (w * 1.3, h * 2.4);
        turned(v, [x, cy], -0.42, |v| {
            let band = Rect::new(x - length * 0.5, cy - tall * 0.5, length, tall);
            panel(v, band, 0.0, Panel::fill(0x8a1030, 0x2a0612, 1.0));
            panel(
                v,
                Rect::new(band.x - 40.0 * s, band.y, 40.0 * s, tall),
                0.0,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            panel(
                v,
                Rect::new(band.x + length, band.y, 22.0 * s, tall),
                0.0,
                Panel::fill(GOLD_HI, GOLD, 1.0),
            );
        });
    }
}
