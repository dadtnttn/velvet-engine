//! The Queen's bed as a menu of the castle's own: over the 3D close shot of her lying under the
//! canopy, the night's palette (indigo, violet, moon silver) instead of the brothel's crimson and
//! gold. A crescent and the title at the top, moon dust falling, and what she can do as a fan of
//! cards shaped like the castle's arched windows, each with its picture drawn in the window. The
//! card lit glows in moonlight with a crescent hovering over it; it never moves. Teasing Chiquita
//! brings her face up large on the right with her answer, the Queen's line on the left, and how
//! fond of her Chiquita is. Sleeping fades the room to night and back.
use super::persona::*;
use super::ui_paint::*;
use super::*;

/// A card of the bed's menu.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct AlcoveCard {
    pub label: String,
    pub hint: String,
    pub icon: String,
    pub locked: bool,
}

/// The last tease: what the Queen did and what Chiquita answered.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct AlcoveTease {
    pub queen: String,
    pub thought: bool,
    pub reply: String,
    pub jealous: bool,
    pub age_ms: u32,
}

/// What the bed's menu shows this frame.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct AlcoveView {
    /// 0 the bed, 1 exploring, 2 Chiquita.
    pub page: u8,
    pub cards: Vec<AlcoveCard>,
    pub choice: u8,
    /// Since the page opened, and since she lay down (ms).
    pub page_ms: u32,
    pub open_ms: u32,
    pub night: bool,
    /// How fond of her Chiquita is.
    pub fondness: i32,
    pub tease: Option<AlcoveTease>,
    /// Asleep for this long (ms).
    pub sleep_ms: Option<u32>,
    pub chiquita: Option<PortraitRef>,
    pub queen: Option<PortraitRef>,
    /// Chiquita is out in 3D (no medallion: her name and hearts at her feet).
    pub mascot: bool,
}

type Uv = ([f32; 2], [f32; 2]);

const NIGHT_TOP: u32 = 0x170d32;
const NIGHT_BOTTOM: u32 = 0x07040f;
const VIOLET: u32 = 0x3a2266;
const MOON: u32 = 0xe8eeff;
const SILVER: u32 = 0xc4cce8;
const SILVER_DEEP: u32 = 0x6a7098;
const LAVENDER: u32 = 0xb9a6ff;
const ROSE: u32 = 0xff7ab0;
const CHQ: u32 = 0xd9a6ff;

/// Where the cards stand: a fan along the foot of the screen, the middle ones a little higher.
fn card_rects(w: f32, h: f32, n: usize) -> Vec<(Rect, f32)> {
    let s = h / 1080.0;
    let n = n.max(1);
    let (cw, ch) = if n > 5 {
        (196.0 * s, 268.0 * s)
    } else {
        (232.0 * s, 300.0 * s)
    };
    let gap = if n > 5 { 18.0 * s } else { 26.0 * s };
    let total = cw * n as f32 + gap * (n as f32 - 1.0);
    let x0 = w * 0.5 - total * 0.5;
    let base = h - ch - 92.0 * s;
    (0..n)
        .map(|i| {
            let u = if n > 1 {
                i as f32 / (n as f32 - 1.0) * 2.0 - 1.0
            } else {
                0.0
            };
            let lift = (1.0 - u * u) * 26.0 * s;
            (
                Rect::new(x0 + i as f32 * (cw + gap), base - lift, cw, ch),
                u * 0.07,
            )
        })
        .collect()
}

/// The card under the pointer.
pub(crate) fn hit(w: f32, h: f32, n: usize, x: f32, y: f32) -> Option<usize> {
    card_rects(w, h, n)
        .iter()
        .position(|(r, _)| r.contains(x, y))
}

/// A crescent moon.
fn moon(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, alpha: f32) {
    halo(v, c, r * 0.9, r * 0.8, rgba(MOON, 0.25 * alpha));
    crescent(v, c, r, r * 0.82, [r * 0.42, -r * 0.2], -0.5, MOON, alpha);
}

/// A heart of two discs and a point.
fn heart(v: &mut Vec<MapVertex>, c: [f32; 2], k: f32, color: [f32; 4]) {
    disc(v, [c[0] - k * 0.48, c[1] - k * 0.25], k * 0.55, color);
    disc(v, [c[0] + k * 0.48, c[1] - k * 0.25], k * 0.55, color);
    push_triangle(
        v,
        [
            [c[0] - k * 1.02, c[1] - k * 0.1],
            [c[0] + k * 1.02, c[1] - k * 0.1],
            [c[0], c[1] + k * 0.95],
        ],
        color,
    );
}

/// A card's picture, in its window.
#[allow(clippy::too_many_arguments)]
fn picture(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    kind: &str,
    c: [f32; 2],
    k: f32,
    lit: bool,
    chiquita: Option<Uv>,
    time: f32,
    s: f32,
) {
    let ink = rgba(if lit { MOON } else { SILVER }, 1.0);
    match kind {
        "moon" => {
            moon(v, c, k, 1.0);
            for (dx, dy, r) in [(0.9, -0.7, 0.16), (1.1, 0.35, 0.11), (-0.95, 0.75, 0.13)] {
                sparkle(
                    v,
                    [c[0] + dx * k, c[1] + dy * k],
                    r * k,
                    rgba(MOON, 0.9),
                    0.6,
                );
            }
        }
        "hand" => {
            // A hand mirror like the vanity's: an ornate silver oval, a heart on its crown, its handle.
            let glass = Rect::new(c[0] - k * 0.6, c[1] - k * 0.98, k * 1.2, k * 1.42);
            wire(
                v,
                [c[0], c[1] + k * 0.42],
                [c[0], c[1] + k * 1.18],
                k * 0.2,
                rgba(SILVER, 1.0),
                rgba(SILVER_DEEP, 1.0),
                0.0,
                true,
            );
            disc(v, [c[0], c[1] + k * 1.2], k * 0.13, rgba(SILVER, 1.0));
            panel(
                v,
                grow(glass, k * 0.12),
                k * 0.72,
                Panel::fill(SILVER, SILVER_DEEP, 1.0).bevel(0.9),
            );
            for side in [-1.0f32, 1.0] {
                curl(
                    v,
                    [c[0] + side * k * 0.55, c[1] + k * 0.3],
                    if side < 0.0 { 2.0 } else { 1.14 },
                    k * 0.5,
                    0.9,
                    side,
                    k * 0.07,
                    k * 0.03,
                    rgba(SILVER, 1.0),
                );
            }
            panel(
                v,
                glass,
                k * 0.6,
                Panel::fill(0xd8e4ff, 0x6a6aa8, 1.0).sheen(time * 0.2),
            );
            heart(v, [c[0], c[1] - k * 1.12], k * 0.2, rgba(ROSE, 1.0));
            wire(
                v,
                [c[0] - k * 0.3, c[1] - k * 0.55],
                [c[0] + k * 0.05, c[1] - k * 0.85],
                k * 0.08,
                rgba(0xffffff, 0.7),
                rgba(0xffffff, 0.0),
                0.0,
                false,
            );
        }
        "seal" => {
            let beat = 0.75 + 0.25 * wave(time, 1.1);
            halo(v, c, k * 0.8, k * 0.6, rgba(ROSE, 0.4 * beat));
            arc(
                v,
                c,
                k * 0.85,
                k * 0.1,
                time * 0.12,
                0.88,
                rgba(0xffc2dc, 1.0),
                rgba(ROSE, 0.25),
            );
            horns(v, [c[0], c[1] - k * 0.55], k * 1.5, SILVER, 1.0);
            sparkle(v, c, k * 0.5, rgba(0xffffff, 1.0), 0.8 * beat);
        }
        "chiquita" => {
            let r = Rect::new(c[0] - k, c[1] - k, k * 2.0, k * 2.0);
            soft(v, r, k, k * 0.4, rgba(CHQ, 0.4), true);
            panel(
                v,
                grow(r, 3.0 * s),
                k + 3.0 * s,
                Panel::fill(0x3a2058, 0x1c0e2e, 1.0).rim(CHQ, 1.0, 2.4 * s),
            );
            // Her face from her 3D self, horns and all (drawn ones only when it is missing).
            match chiquita {
                Some(uv) => face(v, r, k, uv, 1.0),
                None => {
                    sparkle(v, c, k * 0.6, rgba(CHQ, 1.0), 0.8);
                    horns(v, [c[0], c[1] - k * 0.85], k * 1.6, SILVER, 1.0);
                }
            }
        }
        "eye" => {
            for side in [-1.0f32, 1.0] {
                arc(
                    v,
                    [c[0], c[1] + side * k * 0.55],
                    k * 1.05,
                    k * 0.09,
                    if side < 0.0 { 0.38 } else { 0.88 },
                    0.24,
                    ink,
                    [0.0; 4],
                );
            }
            disc(v, c, k * 0.42, rgba(LAVENDER, 1.0));
            disc(v, c, k * 0.2, rgba(0x150a26, 1.0));
            sparkle(
                v,
                [c[0] + k * 0.14, c[1] - k * 0.14],
                k * 0.14,
                rgba(0xffffff, 1.0),
                0.4,
            );
        }
        "heart" => {
            halo(v, c, k * 0.7, k * 0.6, rgba(ROSE, 0.35));
            heart(v, c, k * 0.85, rgba(ROSE, 1.0));
            sparkle(
                v,
                [c[0] + k * 0.75, c[1] - k * 0.6],
                k * 0.22,
                rgba(0xffffff, 0.95),
                0.5,
            );
        }
        "flame" => {
            let f = 1.0 + 0.08 * wave(time, 3.0);
            halo(v, c, k * 0.6, k * 0.8, rgba(0xff6a9a, 0.45));
            gem_turned(
                v,
                [c[0], c[1]],
                k * 0.55,
                k * 1.05 * f,
                0.0,
                0xff6a9a,
                0xffc8a0,
                1.0,
                0.0,
            );
            gem_turned(
                v,
                [c[0], c[1] + k * 0.2],
                k * 0.28,
                k * 0.55 * f,
                0.0,
                0xffe0b8,
                0xffffff,
                1.0,
                0.0,
            );
        }
        "feather" => {
            let angle = 0.72;
            let sway = (time * 1.6).sin() * 0.06;
            gem_turned(
                v,
                [c[0] + k * 0.08, c[1] - k * 0.1],
                k * 0.42,
                k * 1.0,
                angle + sway,
                0xd8c8ff,
                SILVER,
                1.0,
                0.0,
            );
            gem_turned(
                v,
                [c[0] + k * 0.08, c[1] - k * 0.1],
                k * 0.22,
                k * 0.82,
                angle + sway,
                0xf2ecff,
                0xf2ecff,
                0.9,
                0.0,
            );
            let (sn, cs) = (angle + sway).sin_cos();
            let along = [sn, -cs];
            let a = [c[0] - along[0] * k * 1.25, c[1] - along[1] * k * 1.25];
            let b = [c[0] + along[0] * k * 0.95, c[1] + along[1] * k * 0.95];
            wire(v, a, b, k * 0.07, ink, rgba(SILVER, 0.6), 0.0, true);
            for t in [0.35f32, 0.55, 0.72] {
                let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                wire(
                    v,
                    p,
                    [p[0] + cs * k * 0.32, p[1] + sn * k * 0.32],
                    k * 0.05,
                    rgba(0x8a78c8, 0.9),
                    rgba(0x8a78c8, 0.0),
                    0.0,
                    false,
                );
            }
        }
        "heartbreak" => {
            heart(v, c, k * 0.85, rgba(ROSE, 1.0));
            let crack = [
                [0.05, -0.75],
                [-0.15, -0.3],
                [0.12, 0.05],
                [-0.1, 0.45],
                [0.0, 0.8],
            ];
            for p in crack.windows(2) {
                wire(
                    v,
                    [c[0] + p[0][0] * k, c[1] + p[0][1] * k],
                    [c[0] + p[1][0] * k, c[1] + p[1][1] * k],
                    k * 0.1,
                    rgba(0x150a26, 1.0),
                    rgba(0x150a26, 1.0),
                    0.0,
                    false,
                );
            }
        }
        "ribbon" => {
            for side in [-1.0f32, 1.0] {
                arc(
                    v,
                    [c[0] + side * k * 0.5, c[1] - k * 0.15],
                    k * 0.42,
                    k * 0.16,
                    0.0,
                    1.0,
                    rgba(ROSE, 1.0),
                    [0.0; 4],
                );
                wire(
                    v,
                    c,
                    [c[0] + side * k * 0.55, c[1] + k * 0.95],
                    k * 0.18,
                    rgba(ROSE, 1.0),
                    rgba(0xb03a6a, 1.0),
                    0.0,
                    false,
                );
            }
            disc(v, c, k * 0.22, rgba(0xd04a80, 1.0));
        }
        "laugh" => {
            let size = k * 1.3;
            let w = text_width(font, Face::Title, "¡Ja!", size);
            shout(
                font,
                v,
                Face::Title,
                "¡Ja!",
                c[0] - w * 0.5,
                c[1] - size * 0.62,
                size,
                MOON,
                VIOLET,
                1.0,
            );
        }
        "zzz" => {
            for (i, (dx, dy, sz)) in [(-0.6, 0.45, 0.6), (0.0, -0.05, 0.8), (0.6, -0.6, 1.0)]
                .iter()
                .enumerate()
            {
                let bob = (time * 1.4 + i as f32).sin() * k * 0.06;
                let size = k * sz;
                text_outline(
                    font,
                    v,
                    Face::Title,
                    "Z",
                    c[0] + dx * k - size * 0.3,
                    c[1] + dy * k - size * 0.6 + bob,
                    size,
                    rgba(MOON, 1.0),
                    rgba(VIOLET, 1.0),
                );
            }
        }
        "moonheart" => {
            moon(v, [c[0] - k * 0.15, c[1]], k * 0.9, 1.0);
            heart(
                v,
                [c[0] + k * 0.55, c[1] + k * 0.35],
                k * 0.38,
                rgba(ROSE, 1.0),
            );
        }
        _ => {
            for side in [-1.0f32, 1.0] {
                wire(
                    v,
                    [c[0] + k * 0.3, c[1] + side * k * 0.55],
                    [c[0] - k * 0.35, c[1]],
                    k * 0.2,
                    ink,
                    ink,
                    0.0,
                    false,
                );
            }
        }
    }
}

/// One card: a slab of night with a silver rim, an arched window holding its picture, its words
/// under. Lit: moonlight round it, the window brighter, its rim white. Locked: dimmed, a padlock.
#[allow(clippy::too_many_arguments)]
fn card(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    c: &AlcoveCard,
    lit: bool,
    chiquita: Option<Uv>,
    time: f32,
    s: f32,
) {
    let dim = if c.locked { 0.55 } else { 1.0 };
    soft(
        v,
        Rect::new(r.x + 6.0 * s, r.y + 16.0 * s, r.w, r.h),
        18.0 * s,
        18.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    if lit {
        soft(
            v,
            r,
            18.0 * s,
            26.0 * s,
            rgba(
                if c.locked { SILVER_DEEP } else { LAVENDER },
                0.5 + 0.12 * wave(time, 0.7),
            ),
            true,
        );
    }
    panel(
        v,
        r,
        18.0 * s,
        Panel::fill(if lit { 0x2a1a52 } else { NIGHT_TOP }, NIGHT_BOTTOM, 0.97)
            .rim(
                if lit { MOON } else { SILVER_DEEP },
                1.0,
                if lit { 2.6 * s } else { 2.0 * s },
            )
            .hairline(7.0 * s, if lit { 0.55 } else { 0.3 })
            .bevel(0.7),
    );
    // The window: a round-headed arch, night sky inside.
    let ww = r.w - 52.0 * s;
    let wx = r.x + 26.0 * s;
    let spring = r.y + 24.0 * s + ww * 0.5;
    let sill = r.y + r.h * 0.66;
    let cx = wx + ww * 0.5;
    let sky_top = if lit { 0x3a3c8a } else { 0x23245a };
    let sky_bottom = if lit { 0x5a3a8a } else { 0x2a1a4a };
    disc(v, [cx, spring], ww * 0.5, rgba(sky_top, dim));
    panel(
        v,
        Rect::new(wx, spring, ww, sill - spring),
        4.0 * s,
        Panel::fill(sky_top, sky_bottom, dim),
    );
    let frame = rgba(if lit { MOON } else { SILVER }, dim);
    arc(
        v,
        [cx, spring],
        ww * 0.5,
        3.0 * s,
        0.75,
        0.5,
        frame,
        [0.0; 4],
    );
    wire(
        v,
        [wx, spring],
        [wx, sill],
        3.0 * s,
        frame,
        frame,
        0.0,
        true,
    );
    wire(
        v,
        [wx + ww, spring],
        [wx + ww, sill],
        3.0 * s,
        frame,
        frame,
        0.0,
        true,
    );
    wire(
        v,
        [wx - 6.0 * s, sill],
        [wx + ww + 6.0 * s, sill],
        4.0 * s,
        frame,
        frame,
        0.0,
        true,
    );
    // A few stars in the glass.
    for i in 0..4 {
        let fi = i as f32;
        let tw = (time * (1.1 + fi * 0.3) + fi).sin() * 0.5 + 0.5;
        sparkle(
            v,
            [
                wx + ww * (0.2 + 0.2 * fi),
                spring - ww * 0.25 + (fi * 1.7).sin() * ww * 0.12,
            ],
            (2.0 + 2.5 * tw) * s,
            rgba(MOON, 0.7 * tw * dim),
            0.4,
        );
    }
    let pic_c = [cx, (spring - ww * 0.15 + sill) * 0.5];
    faded(v, dim, |v| {
        picture(font, v, &c.icon, pic_c, ww * 0.26, lit, chiquita, time, s)
    });
    if c.locked {
        padlock(
            v,
            [r.x + r.w - 30.0 * s, r.y + 30.0 * s],
            30.0 * s,
            SILVER,
            1.0,
        );
    }
    // Its name under the window.
    let size = if r.w < 210.0 * s { 21.0 * s } else { 24.0 * s };
    let lines = wrap(font, Face::Title, &c.label, size, r.w - 28.0 * s);
    let total = lines.len() as f32 * size * 1.18;
    let mut y = sill + (r.y + r.h - sill - total) * 0.5 - 4.0 * s;
    for line in &lines {
        let lw = text_width(font, Face::Title, line, size);
        text_outline(
            font,
            v,
            Face::Title,
            line,
            r.x + (r.w - lw) * 0.5,
            y,
            size,
            rgba(if lit { MOON } else { SILVER }, dim),
            rgba(NIGHT_BOTTOM, 0.9),
        );
        y += size * 1.18;
    }
}

pub(super) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &AlcoveView,
    chiquita: Option<Uv>,
    queen: Option<Uv>,
    time: f32,
) {
    let s = h / 1080.0;
    let age = view.page_ms as f32 / 1000.0;
    let open = view.open_ms as f32 / 1000.0;
    let intro = ease_out(open / 0.7);

    // The night round her: the edges sink, the foot darkens for the cards.
    vignette(
        v,
        Rect::new(0.0, 0.0, w, h),
        0.0,
        w * 0.22,
        rgba(NIGHT_BOTTOM, 0.75 * intro),
    );
    panel(
        v,
        Rect::new(0.0, h * 0.5, w, h * 0.5),
        0.0,
        Panel {
            top: rgba(NIGHT_BOTTOM, 0.0),
            bottom: rgba(NIGHT_BOTTOM, 0.85 * intro),
            ..Panel::fill(0, 0, 0.0)
        },
    );
    // Moon dust, falling slowly.
    for i in 0..26 {
        let fi = i as f32;
        let speed = 0.025 + 0.02 * ((fi * 1.3).sin() * 0.5 + 0.5);
        let phase = (time * speed + fi * 0.618).fract();
        let x = w * ((fi * 0.7548).fract() + 0.02 * (time * 0.3 + fi).sin());
        let y = h * phase;
        let tw = (time * (1.0 + fi * 0.13) + fi).sin() * 0.5 + 0.5;
        sparkle(
            v,
            [x, y],
            (2.0 + 3.0 * tw) * s,
            rgba(MOON, (0.25 + 0.35 * tw) * intro),
            0.5,
        );
    }

    // While she sleeps the menu fades away; only the night is left.
    let awake = 1.0 - view.sleep_ms.map_or(0.0, |ms| (ms as f32 / 400.0).min(1.0));
    let start = v.len();
    // The title, under a crescent.
    let ta = entrance(open, 0.1, 0.5);
    faded(v, ta, |v| {
        shifted(v, [0.0, -(1.0 - ta) * 30.0 * s], |v| {
            moon(v, [104.0 * s, 104.0 * s], 46.0 * s, 1.0);
            let title = match view.page {
                1 => "Explorar el cuerpo",
                2 => "Molestar a Chiquita",
                _ => "El lecho de la Reina",
            };
            text_outline(
                font,
                v,
                Face::Title,
                title,
                170.0 * s,
                58.0 * s,
                54.0 * s,
                rgba(MOON, 1.0),
                rgba(VIOLET, 0.95),
            );
            let sub = match view.page {
                1 => "Es tu cuerpo. Técnicamente.",
                2 => "Vive en tu sello. No tiene adónde huir.",
                _ => "A solas. Bueno: tú, el sello y Chiquita.",
            };
            text_shadow(
                font,
                v,
                Face::Italic,
                sub,
                174.0 * s,
                124.0 * s,
                25.0 * s,
                rgba(LAVENDER, 1.0),
            );
            wire(
                v,
                [170.0 * s, 166.0 * s],
                [640.0 * s, 166.0 * s],
                2.0 * s,
                rgba(SILVER, 0.9),
                rgba(SILVER, 0.0),
                0.0,
                true,
            );
        });
    });

    // The cards, rising one after another into their fan.
    let rects = card_rects(w, h, view.cards.len());
    for (i, ((r, angle), c)) in rects.iter().zip(&view.cards).enumerate() {
        let a = entrance(age, 0.05 + 0.06 * i as f32, 0.35);
        let lit = i == view.choice as usize;
        faded(v, a, |v| {
            shifted(v, [0.0, (1.0 - a) * 240.0 * s], |v| {
                turned(v, [r.x + r.w * 0.5, r.y + r.h], *angle, |v| {
                    card(font, v, *r, c, lit, chiquita, time, s);
                });
            });
        });
        if lit && a > 0.9 {
            // A crescent hovering over the lit card.
            let bob = (time * 2.4).sin() * 5.0 * s;
            moon(v, [r.x + r.w * 0.5, r.y - 34.0 * s + bob], 16.0 * s, 1.0);
        }
    }
    // What the lit card means, under the fan.
    if let Some(c) = view.cards.get(view.choice as usize) {
        let size = 25.0 * s;
        let y = h - 72.0 * s;
        let tw = text_width(font, Face::Italic, &c.hint, size);
        soft(
            v,
            Rect::new(
                w * 0.5 - tw * 0.5 - 30.0 * s,
                y - 6.0 * s,
                tw + 60.0 * s,
                size + 16.0 * s,
            ),
            18.0 * s,
            16.0 * s,
            rgba(NIGHT_BOTTOM, 0.7),
            false,
        );
        text_centered(
            font,
            v,
            Face::Italic,
            &c.hint,
            w * 0.5,
            y,
            size,
            rgba(if c.locked { 0xc8a8c8 } else { MOON }, 1.0),
        );
    }
    // The keys: ◀ ▶ (drawn), Enter, Esc.
    let mut x = w - 470.0 * s;
    let ky = h - 60.0 * s;
    for (key, what) in [
        ("", "elegir"),
        ("Enter", "tomar"),
        (
            "Esc",
            if view.page == 0 {
                "levantarse"
            } else {
                "volver"
            },
        ),
    ] {
        let cap_w = if key.is_empty() {
            58.0 * s
        } else {
            (text_width(font, Face::Title, key, 17.0 * s) + 22.0 * s).max(40.0 * s)
        };
        let cap = Rect::new(x, ky, cap_w, 36.0 * s);
        keycap(font, v, cap, key, 17.0 * s, false, false, LAVENDER, 1.0);
        if key.is_empty() {
            let cy = cap.y + cap.h * 0.42;
            for (cx, dir) in [(cap.x + cap.w * 0.32, -1.0f32), (cap.x + cap.w * 0.68, 1.0)] {
                let tip = [cx + dir * 6.0 * s, cy];
                for side in [-1.0f32, 1.0] {
                    wire(
                        v,
                        [cx - dir * 3.0 * s, cy + side * 7.0 * s],
                        tip,
                        2.6 * s,
                        rgba(MOON, 1.0),
                        rgba(MOON, 1.0),
                        0.0,
                        false,
                    );
                }
            }
        }
        text_shadow(
            font,
            v,
            Face::Italic,
            what,
            x + cap_w + 8.0 * s,
            ky + 7.0 * s,
            19.0 * s,
            rgba(SILVER, 1.0),
        );
        x += cap_w + 8.0 * s + text_width(font, Face::Italic, what, 19.0 * s) + 26.0 * s;
    }

    if view.page == 2 {
        draw_chiquita(font, v, w, h, view, chiquita, queen, age, time, s);
    }
    if awake < 1.0 {
        for vertex in &mut v[start..] {
            vertex.color[3] *= awake;
            vertex.color2[3] *= awake;
        }
    }
    if let Some(ms) = view.sleep_ms {
        draw_sleep(font, v, w, h, view.night, ms as f32 / 1000.0, time, s);
    }
}

/// Chiquita, large on the right: her face in a silver medallion with horns, how fond of the Queen
/// she is, and her answer to the last tease (shaking when it flusters her), the Queen's line on
/// the left.
#[allow(clippy::too_many_arguments)]
fn draw_chiquita(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    _h: f32,
    view: &AlcoveView,
    chiquita: Option<Uv>,
    queen: Option<Uv>,
    age: f32,
    time: f32,
    s: f32,
) {
    let a = entrance(age, 0.1, 0.4);
    let tease_age = view
        .tease
        .as_ref()
        .map_or(99.0, |t| t.age_ms as f32 / 1000.0);
    let shake = match &view.tease {
        Some(t) if t.jealous => (tease_age * 42.0).sin() * 7.0 * s * (-tease_age * 3.0).exp(),
        _ => 0.0,
    };
    let c = [
        w - 240.0 * s + shake,
        330.0 * s + (time * 1.6).sin() * 6.0 * s,
    ];
    let r = 118.0 * s;
    if view.mascot {
        // Her name and how fond of the Queen she is, at her 3D self's feet.
        let c = [w - 230.0 * s, 628.0 * s];
        faded(v, a, |v| {
            let nw = text_width(font, Face::Title, "Chiquita", 32.0 * s);
            text_outline(
                font,
                v,
                Face::Title,
                "Chiquita",
                c[0] - nw * 0.5,
                c[1],
                32.0 * s,
                rgba(CHQ, 1.0),
                rgba(NIGHT_BOTTOM, 0.9),
            );
            let full = (view.fondness.max(0) as f32 / 2.0).min(5.0);
            for k in 0..5 {
                let hc = [c[0] - 2.0 * 34.0 * s + k as f32 * 34.0 * s, c[1] + 54.0 * s];
                heart(v, hc, 11.0 * s, rgba(0x2a1a4a, 1.0));
                let fill = (full - k as f32).clamp(0.0, 1.0);
                if fill > 0.0 {
                    heart(v, hc, 11.0 * s * (0.4 + 0.6 * fill), rgba(ROSE, 1.0));
                }
            }
        });
    }
    faded(v, if view.mascot { 0.0 } else { a }, |v| {
        shifted(v, [(1.0 - a) * 200.0 * s, 0.0], |v| {
            halo(v, c, r, 50.0 * s, rgba(CHQ, 0.35 + 0.15 * wave(time, 0.6)));
            if chiquita.is_none() {
                horns(v, [c[0], c[1] - r * 0.86], r * 1.9, SILVER, 1.0);
            }
            panel(
                v,
                Rect::new(
                    c[0] - r - 6.0 * s,
                    c[1] - r - 6.0 * s,
                    (r + 6.0 * s) * 2.0,
                    (r + 6.0 * s) * 2.0,
                ),
                r + 6.0 * s,
                Panel::fill(SILVER, SILVER_DEEP, 1.0).bevel(0.8),
            );
            let fr = Rect::new(c[0] - r, c[1] - r, r * 2.0, r * 2.0);
            panel(v, fr, r, Panel::fill(0x3a2058, 0x1c0e2e, 1.0));
            if let Some(uv) = chiquita {
                face(v, fr, r, uv, 1.0);
            }
            // Her name, and how fond of the Queen she is: five hearts.
            let name_y = c[1] + r + 18.0 * s;
            let nw = text_width(font, Face::Title, "Chiquita", 32.0 * s);
            text_outline(
                font,
                v,
                Face::Title,
                "Chiquita",
                c[0] - nw * 0.5,
                name_y,
                32.0 * s,
                rgba(CHQ, 1.0),
                rgba(NIGHT_BOTTOM, 0.9),
            );
            let full = (view.fondness.max(0) as f32 / 2.0).min(5.0);
            for k in 0..5 {
                let hc = [
                    c[0] - 2.0 * 34.0 * s + k as f32 * 34.0 * s,
                    name_y + 62.0 * s,
                ];
                heart(v, hc, 11.0 * s, rgba(0x2a1a4a, 1.0));
                let fill = (full - k as f32).clamp(0.0, 1.0);
                if fill > 0.0 {
                    heart(v, hc, 11.0 * s * (0.4 + 0.6 * fill), rgba(ROSE, 1.0));
                }
            }
            text_centered(
                font,
                v,
                Face::Italic,
                "cariño",
                c[0],
                name_y + 82.0 * s,
                18.0 * s,
                rgba(SILVER, 0.9),
            );
        });
    });
    let Some(t) = &view.tease else {
        return;
    };
    // The Queen's line, on the left.
    let qa = ease_out(tease_age / 0.3);
    faded(v, qa, |v| {
        let plate = Rect::new(80.0 * s, 250.0 * s, 620.0 * s, 150.0 * s);
        panel(
            v,
            plate,
            20.0 * s,
            Panel::fill(0x241640, 0x0e0820, 0.95)
                .rim(SILVER, 0.9, 2.0 * s)
                .hairline(6.0 * s, 0.3),
        );
        let fr = Rect::new(plate.x + 18.0 * s, plate.y + 22.0 * s, 104.0 * s, 104.0 * s);
        panel(
            v,
            grow(fr, 3.0 * s),
            55.0 * s,
            Panel::fill(0x3a1a4a, 0x0e0820, 1.0).rim(ROSE, 1.0, 2.0 * s),
        );
        if let Some(uv) = queen {
            face(v, fr, 52.0 * s, uv, 1.0);
        }
        let tx = fr.x + fr.w + 22.0 * s;
        text_shadow(
            font,
            v,
            Face::Title,
            if t.thought { "Reina (piensa)" } else { "Reina" },
            tx,
            plate.y + 20.0 * s,
            24.0 * s,
            rgba(ROSE, 1.0),
        );
        let words = if t.thought {
            format!("({})", t.queen)
        } else {
            t.queen.clone()
        };
        let face_kind = if t.thought { Face::Italic } else { Face::Body };
        for (i, line) in wrap(
            font,
            face_kind,
            &words,
            23.0 * s,
            plate.x + plate.w - 20.0 * s - tx,
        )
        .iter()
        .take(3)
        .enumerate()
        {
            text_shadow(
                font,
                v,
                face_kind,
                line,
                tx,
                plate.y + 58.0 * s + i as f32 * 31.0 * s,
                23.0 * s,
                rgba(if t.thought { 0xe2d4ff } else { MOON }, 1.0),
            );
        }
    });
    // Chiquita's answer, by her face (her 3D self says it in her own balloon).
    let ra = if view.mascot {
        0.0
    } else {
        ease_out((tease_age - 0.45) / 0.3)
    };
    if ra > 0.0 {
        faded(v, ra, |v| {
            let size = 25.0 * s;
            let max = 520.0 * s;
            let lines = wrap(font, Face::Italic, &t.reply, size, max - 44.0 * s);
            let bh = 30.0 * s + lines.len() as f32 * size * 1.35;
            let bx = w - 380.0 * s - max + shake;
            let by = 470.0 * s;
            let b = Rect::new(bx, by, max, bh);
            soft(
                v,
                Rect::new(b.x, b.y + 10.0 * s, b.w, b.h),
                20.0 * s,
                16.0 * s,
                rgba(SHADOW, 0.6),
                false,
            );
            panel(
                v,
                b,
                20.0 * s,
                Panel::fill(0x3a2058, 0x1c0e2e, 0.97)
                    .rim(CHQ, 1.0, 2.2 * s)
                    .hairline(5.0 * s, 0.3),
            );
            // A tail toward her face.
            push_triangle(
                v,
                [
                    [b.x + b.w - 30.0 * s, b.y + 10.0 * s],
                    [b.x + b.w + 40.0 * s, b.y - 60.0 * s],
                    [b.x + b.w - 4.0 * s, b.y + 40.0 * s],
                ],
                rgba(0x3a2058, 0.97),
            );
            for (i, line) in lines.iter().enumerate() {
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    line,
                    b.x + 22.0 * s,
                    b.y + 16.0 * s + i as f32 * size * 1.35,
                    size,
                    rgba(0xf4ecff, 1.0),
                );
            }
        });
    }
}

/// Sleeping: the room sinks into night, a moon crosses, «Zzz» drifts up, and the light comes back
/// at the other end of the day.
#[allow(clippy::too_many_arguments)]
fn draw_sleep(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    night: bool,
    t: f32,
    time: f32,
    s: f32,
) {
    let dark = (t / 0.8).min(1.0) * (1.0 - ((t - 2.0) / 0.8).clamp(0.0, 1.0));
    push_rect(v, Rect::new(0.0, 0.0, w, h), rgba(0x05030c, 0.96 * dark));
    if dark <= 0.01 {
        return;
    }
    let arc_t = (t / 2.6).clamp(0.0, 1.0);
    let c = [
        w * (0.25 + 0.5 * arc_t),
        h * (0.42 - 0.18 * (arc_t * std::f32::consts::PI).sin()),
    ];
    // The moon goes down and the sun comes up, or the other way round.
    let moon_now = (t < 1.3) == night;
    if moon_now {
        moon(v, c, 60.0 * s, dark);
    } else {
        halo(v, c, 50.0 * s, 70.0 * s, rgba(0xffd08a, 0.5 * dark));
        disc(v, c, 44.0 * s, rgba(0xffe6b0, dark));
    }
    for i in 0..3 {
        let fi = i as f32;
        let p = ((t * 0.6 + fi * 0.33) % 1.0).max(0.0);
        let size = (34.0 + 18.0 * fi) * s;
        text_outline(
            font,
            v,
            Face::Title,
            "Z",
            w * 0.5 + (fi * 60.0 - 40.0) * s,
            h * 0.66 - p * 160.0 * s,
            size,
            rgba(MOON, dark * (1.0 - p)),
            rgba(VIOLET, dark * (1.0 - p)),
        );
    }
    let words = if t < 1.3 {
        "Duermes…"
    } else if night {
        "Amanece."
    } else {
        "Anochece."
    };
    text_centered(
        font,
        v,
        Face::Italic,
        words,
        w * 0.5,
        h * 0.78,
        34.0 * s,
        rgba(LAVENDER, dark),
    );
    let _ = time;
}
