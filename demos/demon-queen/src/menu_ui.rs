//! Widgets for the pause menu, in the manner of Persona's menus: black panels with cut
//! corners and hard offset shadows, a spine in the menu's accent, halftone dots in a corner,
//! slanted plates for buttons, tags and keycaps, segmented slanted meters. The accent is the
//! scene's own light made vivid (see `accent`), so the menu takes the colour of wherever it
//! opens. Focus only changes a widget's colour and light, never its place or size. Everything
//! is drawn with the canvas' anti-aliased primitives.
use crate::prompt_icons::{self, Icon};
use crate::render::{Canvas, Fonts};

pub type Rect = (i32, i32, i32, i32);

pub const GOLD: u32 = 0xffd9b67b;
pub const INK: u32 = 0xfff5eaf3;
pub const MUTED: u32 = 0xffbaacbf;
pub const DANGER: u32 = 0xffe8737f;
/// The menus' red (0xRRGGBB): the accent when the scene's light has no colour to give.
pub const CRIMSON: u32 = 0xe2304f;
/// Near-black with a touch of violet: the plates' ground.
pub const ABYSS: u32 = 0x0c0811;
/// The white of a plate (0xRRGGBB).
pub const BONE: u32 = 0xf7eef3;

/// Colour halfway between two `0xRRGGBB` values (alpha bits ignored).
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x + (y - x) * t).round() as u32 & 0xff) << shift
    };
    ch(16) | ch(8) | ch(0)
}

pub fn argb(alpha: u32, rgb: u32) -> u32 {
    (alpha.min(255) << 24) | (rgb & 0x00ff_ffff)
}

fn channels(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    ]
}

/// The menu's accent for a scene whose light is `light` (0xRRGGBB): the same hue, saturated
/// and bright as Persona's plates are — red in the castle, blue at night in the baths, green
/// in the forest. A light with no hue to speak of gives crimson.
pub fn accent(light: u32) -> u32 {
    let (h, s, _) = crate::rgb_to_hsv(channels(light));
    if s < 0.08 {
        return CRIMSON;
    }
    crate::rgb_u32(crate::hsv_to_rgb(h, (s * 1.35).clamp(0.66, 0.86), 0.88))
}

/// How much light a colour gives off (0..1, relative luminance).
fn luminance(c: u32) -> f32 {
    let [r, g, b] = channels(c).map(|v| v.powf(2.2));
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The ink that reads on a plate of `ground` (0xRRGGBB), as ARGB: white on deep and saturated
/// grounds (crimson, blue, violet), near-black on bright ones (a yellow, a green, a cyan).
pub fn ink_on(ground: u32) -> u32 {
    if luminance(ground) > 0.36 {
        argb(0xff, ABYSS)
    } else {
        0xffff_ffff
    }
}

/// `c` (0xRRGGBB) darkened to `k` of its brightness, and further if it would still give off
/// more than `most` light: so a yellow sinks to a deep gold as a crimson does to a wine.
pub fn deepen(c: u32, k: f32, most: f32) -> u32 {
    let k = k.min((most / luminance(c).max(1e-4)).powf(1.0 / 2.2));
    crate::rgb_u32(channels(c).map(|v| v * k))
}

// ------------------------------------------------------------------ shapes

/// The corners of a plate: a parallelogram whose top edge leans `skew` px to the right of its
/// bottom edge, inside `x..x + w`.
pub fn plate_points(x: f32, y: f32, w: f32, h: f32, skew: f32) -> [(f32, f32); 4] {
    [(x + skew, y), (x + w, y), (x + w - skew, y + h), (x, y + h)]
}

/// A filled plate.
pub fn plate(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, skew: f32, color: u32) {
    canvas.convex(&plate_points(x, y, w, h, skew), color);
}

/// A rectangle with its top-right and bottom-left corners cut by `cut` px.
pub fn cut_points(x: f32, y: f32, w: f32, h: f32, cut: f32) -> [(f32, f32); 6] {
    [
        (x, y),
        (x + w - cut, y),
        (x + w, y + cut),
        (x + w, y + h),
        (x + cut, y + h),
        (x, y + h - cut),
    ]
}

/// The same shape moved by (dx, dy).
pub fn moved<const N: usize>(points: &[(f32, f32); N], dx: f32, dy: f32) -> [(f32, f32); N] {
    points.map(|(x, y)| (x + dx, y + dy))
}

/// The closed outline of a shape, `width` px.
pub fn outline(canvas: &mut Canvas, points: &[(f32, f32)], width: f32, color: u32) {
    for i in 0..points.len() {
        canvas.line(points[i], points[(i + 1) % points.len()], width, color);
    }
}

/// Halftone dots over `rect` (x, y, w, h in px): largest at `toward`, shrinking to nothing at
/// `reach` px from it.
pub fn halftone(
    canvas: &mut Canvas,
    rect: (f32, f32, f32, f32),
    step: f32,
    largest: f32,
    toward: (f32, f32),
    reach: f32,
    color: u32,
) {
    let (x0, y0, w, h) = rect;
    let mut row = 0;
    let mut y = y0 + step * 0.5;
    while y < y0 + h {
        let shift = if row % 2 == 1 { step * 0.5 } else { 0.0 };
        let mut x = x0 + step * 0.5 + shift;
        while x < x0 + w {
            let d = ((x - toward.0).powi(2) + (y - toward.1).powi(2)).sqrt();
            let r = largest * (1.0 - d / reach.max(1.0));
            if r > 0.5 {
                canvas.smooth_disc(x, y, r, color);
            }
            x += step;
        }
        y += step;
        row += 1;
    }
}

/// A pane: a black panel with cut corners, a hard offset shadow, a lighter band along its top,
/// a spine in the accent on its left edge, halftone in its far corner and a fine rim. `light`
/// (the scene's colour) tints it and gives the accent; `lift` (0..1) lights it for focus.
pub fn glass(canvas: &mut Canvas, rect: Rect, radius: i32, light: u32, lift: f32) {
    let (x, y, w, h) = (rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32);
    if w < 4.0 || h < 4.0 {
        return;
    }
    let cut = (radius as f32 * 0.9)
        .clamp(8.0, 30.0)
        .min(h * 0.3)
        .min(w * 0.3);
    let body = cut_points(x, y, w, h, cut);
    canvas.convex(&moved(&body, 7.0, 8.0), 0x78000000);
    canvas.convex(&body, argb(0xf0, mix(ABYSS, light, 0.05 + 0.08 * lift)));
    // The upper band, two-toned as Persona's panels are.
    let band = (h * 0.16).clamp(8.0, 42.0);
    canvas.convex(
        &[
            (x + 6.0, y + 5.0),
            (x + w - cut - 2.0, y + 5.0),
            (x + w - cut + band * 0.6, y + band),
            (x + 6.0, y + band),
        ],
        argb(
            0x14 + (0x18 as f32 * lift) as u32,
            mix(light, 0xffffff, 0.35),
        ),
    );
    canvas.convex(
        &[
            (x, y),
            (x + 4.0, y),
            (x + 4.0, y + h - cut - 4.0),
            (x, y + h - cut),
        ],
        argb(0xe6, accent(light)),
    );
    if w > 60.0 && h > 60.0 {
        halftone(
            canvas,
            (x + w * 0.5, y + h * 0.5, w * 0.5 - 4.0, h * 0.5 - 4.0),
            14.0,
            2.6,
            (x + w, y + h),
            w.min(h) * 0.6,
            argb(
                0x22 + (0x20 as f32 * lift) as u32,
                mix(light, 0xffffff, 0.4),
            ),
        );
    }
    outline(
        canvas,
        &body,
        1.3,
        argb(
            0x44 + (0x80 as f32 * lift) as u32,
            mix(light, 0xffffff, 0.4),
        ),
    );
}

// ------------------------------------------------------------------- text

/// Text whose vertical centre is `cy`.
pub fn text_mid(
    canvas: &mut Canvas,
    fonts: &Fonts,
    text: &str,
    x: i32,
    cy: i32,
    size: f32,
    color: u32,
    bold: bool,
) -> i32 {
    canvas.text(
        fonts,
        text,
        x,
        cy - size.round() as i32 / 2 - 4,
        size,
        color,
        bold,
    );
    fonts.text_width(text, size, bold)
}

/// Text centred on (cx, cy).
pub fn text_center(
    canvas: &mut Canvas,
    fonts: &Fonts,
    text: &str,
    cx: i32,
    cy: i32,
    size: f32,
    color: u32,
    bold: bool,
) {
    let w = fonts.text_width(text, size, bold);
    text_mid(canvas, fonts, text, cx - w / 2, cy, size, color, bold);
}

/// Text ending at `right`, vertically centred on `cy`.
pub fn text_right(
    canvas: &mut Canvas,
    fonts: &Fonts,
    text: &str,
    right: i32,
    cy: i32,
    size: f32,
    color: u32,
    bold: bool,
) {
    let w = fonts.text_width(text, size, bold);
    text_mid(canvas, fonts, text, right - w, cy, size, color, bold);
}

// ---------------------------------------------------------------- widgets

/// A slanted tag of text (a label, a mode); returns its width.
pub fn chip(canvas: &mut Canvas, fonts: &Fonts, x: i32, y: i32, text: &str, accent: u32) -> i32 {
    let size = 15.0;
    let w = fonts.text_width(text, size, true) + 40;
    let h = 30;
    let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
    plate(canvas, fx + 3.0, fy + 3.0, fw, fh, 9.0, 0x66000000);
    plate(
        canvas,
        fx,
        fy,
        fw,
        fh,
        9.0,
        argb(0xd8, mix(ABYSS, accent, 0.28)),
    );
    outline(
        canvas,
        &plate_points(fx, fy, fw, fh, 9.0),
        1.0,
        argb(0xb0, accent),
    );
    let cy = fy + fh * 0.5;
    canvas.convex(
        &[
            (fx + 12.0, cy),
            (fx + 18.0, cy - 6.0),
            (fx + 24.0, cy),
            (fx + 18.0, cy + 6.0),
        ],
        argb(0xff, mix(accent, 0xffffff, 0.2)),
    );
    text_mid(
        canvas,
        fonts,
        text,
        x + 30,
        y + h / 2,
        size,
        argb(0xff, mix(accent, 0xffffff, 0.45)),
        true,
    );
    w
}

/// A slanted plate of `ground` (0xRRGGBB) with `text` in `ink`; returns its width.
#[allow(clippy::too_many_arguments)]
pub fn tag(
    canvas: &mut Canvas,
    fonts: &Fonts,
    x: i32,
    y: i32,
    text: &str,
    size: f32,
    ground: u32,
    ink: u32,
) -> i32 {
    let w = fonts.text_width(text, size, true) + 36;
    let h = (size * 2.1).round() as i32;
    let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
    plate(canvas, fx + 4.0, fy + 4.0, fw, fh, 10.0, 0x99000000);
    plate(canvas, fx, fy, fw, fh, 10.0, argb(0xff, ground));
    text_mid(
        canvas,
        fonts,
        text,
        x + 18,
        y + h / 2,
        size,
        argb(0xff, ink),
        true,
    );
    w
}

/// A keyboard key cap with its legend; returns its width.
pub fn keycap(canvas: &mut Canvas, fonts: &Fonts, x: i32, y: i32, label: &str) -> i32 {
    let size = 15.0;
    let w = (fonts.text_width(label, size, true) + 24).max(36);
    let h = 30;
    let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
    plate(canvas, fx + 3.0, fy + 3.0, fw, fh - 2.0, 6.0, 0xaa000000);
    plate(canvas, fx, fy, fw, fh - 2.0, 6.0, 0xff1a131f);
    outline(
        canvas,
        &plate_points(fx, fy, fw, fh - 2.0, 6.0),
        1.1,
        0x66ffffff,
    );
    text_center(
        canvas,
        fonts,
        label,
        x + w / 2,
        y + (h - 2) / 2,
        size,
        INK,
        true,
    );
    w
}

/// A vector icon at `size` px centred on (cx, cy).
pub fn icon(canvas: &mut Canvas, ic: Icon, cx: i32, cy: i32, size: i32, ink: u32, face: u32) {
    let parts: Vec<(Vec<[f64; 2]>, bool)> = prompt_icons::shape(ic)
        .into_iter()
        .map(|p| {
            (
                p.points
                    .iter()
                    .map(|q| [q[0] as f64, q[1] as f64])
                    .collect(),
                p.ink > 0.5,
            )
        })
        .collect();
    canvas.vector_icon(ic.index(), &parts, cx, cy, size, ink, face);
}

/// A badge holding an icon: a dark disc ringed in `accent`; `lit` fills it with the accent and
/// rings it in white.
pub fn icon_disc(
    canvas: &mut Canvas,
    ic: Icon,
    cx: i32,
    cy: i32,
    radius: i32,
    accent: u32,
    lit: bool,
) {
    let (fx, fy, r) = (cx as f32, cy as f32, radius as f32);
    let base = mix(ABYSS, accent, if lit { 0.55 } else { 0.16 });
    canvas.smooth_disc(fx + 3.0, fy + 3.0, r + 1.0, 0x66000000);
    canvas.smooth_disc(fx, fy, r, argb(0xf8, base));
    canvas.smooth_ring(
        fx,
        fy,
        r - 1.0,
        1.8,
        argb(
            if lit { 0xff } else { 0xa0 },
            if lit { BONE } else { accent },
        ),
    );
    let bright = if lit {
        BONE
    } else {
        mix(accent, 0xffffff, 0.2)
    };
    icon(
        canvas,
        ic,
        cx,
        cy,
        (radius as f32 * 1.15) as i32,
        argb(0xff, bright),
        argb(0xff, base),
    );
}

/// A slanted button. `focused` turns it the accent of `light` (red-white for `danger`) with
/// words that read on it; its shape never changes.
#[allow(clippy::too_many_arguments)]
pub fn pill(
    canvas: &mut Canvas,
    fonts: &Fonts,
    rect: Rect,
    label: &str,
    focused: bool,
    enabled: bool,
    light: u32,
    danger: bool,
) {
    let (x, y, w, h) = (rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32);
    let skew = (h * 0.3).min(18.0);
    let hot = focused && enabled;
    let accent = if danger {
        DANGER & 0x00ff_ffff
    } else {
        accent(light)
    };
    plate(canvas, x + 6.0, y + 6.0, w, h, skew, 0x99000000);
    plate(
        canvas,
        x,
        y,
        w,
        h,
        skew,
        if hot {
            argb(0xff, accent)
        } else {
            argb(0xf0, mix(0x110c17, light, 0.06))
        },
    );
    if hot {
        // A white sliver along the top: the plate is "lit".
        plate(
            canvas,
            x + skew * 0.9,
            y + 3.0,
            w - skew * 1.3,
            3.0,
            1.0,
            0x66ffffff,
        );
    }
    let border = if !enabled {
        0x1cffffff
    } else if hot {
        0xffffffff
    } else {
        0x44ffffff
    };
    outline(
        canvas,
        &plate_points(x, y, w, h, skew),
        if hot { 2.0 } else { 1.1 },
        border,
    );
    let color = if !enabled {
        argb(0x77, MUTED)
    } else if hot {
        ink_on(accent)
    } else {
        INK
    };
    text_center(
        canvas,
        fonts,
        label,
        rect.0 + rect.2 / 2,
        rect.1 + rect.3 / 2,
        22.0,
        color,
        true,
    );
}

/// A slanted meter cut into ten segments: black track, `color` fill with a lighter top.
pub fn meter(canvas: &mut Canvas, x: i32, y: i32, w: i32, h: i32, fraction: f32, color: u32) {
    let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
    let skew = (fh * 0.8).clamp(4.0, 14.0);
    plate(canvas, fx + 3.0, fy + 3.0, fw, fh, skew, 0x88000000);
    plate(canvas, fx, fy, fw, fh, skew, 0xff120d18);
    let f = fraction.clamp(0.0, 1.0);
    if f > 0.0 {
        let fill = skew + (fw - skew) * f;
        plate(canvas, fx, fy, fill, fh, skew, argb(0xff, color));
        plate(
            canvas,
            fx + skew * 0.5,
            fy,
            (fill - skew * 0.5).max(0.0),
            fh * 0.42,
            skew * 0.42,
            argb(0x55, 0xffffff),
        );
    }
    for i in 1..10 {
        let sx = fx + skew + (fw - skew) * i as f32 / 10.0;
        canvas.line((sx, fy), (sx - skew, fy + fh), 1.6, 0x99000000);
    }
    outline(canvas, &plate_points(fx, fy, fw, fh, skew), 1.0, 0x40ffffff);
}

/// An on/off switch, 64 by 34: a slanted track (the accent of `light` when on) and a slanted
/// knob.
pub fn toggle(canvas: &mut Canvas, x: i32, y: i32, on: bool, light: u32) {
    let (fx, fy, w, h) = (x as f32, y as f32, 64.0, 34.0);
    plate(canvas, fx + 3.0, fy + 3.0, w, h, 9.0, 0x88000000);
    plate(
        canvas,
        fx,
        fy,
        w,
        h,
        9.0,
        if on {
            argb(0xff, accent(light))
        } else {
            0xff1d1524
        },
    );
    outline(
        canvas,
        &plate_points(fx, fy, w, h, 9.0),
        1.2,
        if on { 0xffffffff } else { 0x44ffffff },
    );
    let kx = if on { fx + w - 30.0 } else { fx + 6.0 };
    plate(
        canvas,
        kx,
        fy + 5.0,
        24.0,
        h - 10.0,
        6.0,
        if on { 0xffffffff } else { 0xffb9adc4 },
    );
}

/// Row of keycaps with a caption after each, starting at (x, y).
pub fn key_hints(
    canvas: &mut Canvas,
    fonts: &Fonts,
    x: i32,
    y: i32,
    hints: &[(&str, &str)],
) -> i32 {
    let mut cursor = x;
    for (keys, caption) in hints {
        cursor += keycap(canvas, fonts, cursor, y, keys) + 10;
        cursor += text_mid(canvas, fonts, caption, cursor, y + 15, 17.0, MUTED, false) + 26;
    }
    cursor - x
}

/// The light a picture casts: its dominant colour (see `dominant_light`),
/// as 0xRRGGBB.
pub fn light_from_pixels(pixels: &[u32], w: usize, h: usize) -> u32 {
    let c = crate::dominant_light_with(w, h, |x, y| {
        let p = pixels[y * w + x];
        [(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]
    });
    let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (to(c[0]) << 16) | (to(c[1]) << 8) | to(c[2])
}

/// The largest size up to `size` (down to 60 %) at which `text` fits `max` px.
pub fn fit_size(fonts: &Fonts, text: &str, max: i32, size: f32, bold: bool) -> f32 {
    let mut s = size;
    while s > size * 0.6 && fonts.text_width(text, s, bold) > max {
        s -= 0.5;
    }
    s
}

/// Eases a 0..1 progress out (fast start, soft landing).
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// How far an entrance that starts `delay` s in and lasts `length` s has got, `age` s after
/// the page opened (eased).
pub fn entrance(age: f32, delay: f32, length: f32) -> f32 {
    ease_out((age - delay) / length.max(1e-3))
}
