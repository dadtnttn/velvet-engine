//! A small vector painter for the overlay. Every shape is a quad whose pixels the shader shades
//! from the shape's signed distance, so edges stay smooth at any size: rounded panels of velvet
//! glass with a metal rim, soft shadows and glows, wire strokes and filigree curls, cut gems,
//! crescent horns, sparkles, progress arcs, faces and pictures masked to rounded frames. The
//! dialogue box and the intimate scenes are built from these and text, with no images.
use demon_queen_dungeon::i18n::tr;
use std::f32::consts::{PI, TAU};

use super::*;

// The shader's modes (`MapVertex::text`).
const BOX: f32 = 5.0;
const SOFT: f32 = 6.0;
const WIRE: f32 = 7.0;
const GEM: f32 = 8.0;
const FACE: f32 = 9.0;
const PICTURE: f32 = 10.0;
const CRESCENT: f32 = 11.0;
const STAR: f32 = 12.0;
const ARC: f32 = 13.0;

// The palette (sRGB).
pub(super) const GOLD_HI: u32 = 0xf6dc9a;
pub(super) const GOLD: u32 = 0xc9a36a;
pub(super) const GOLD_DEEP: u32 = 0x8a6436;
pub(super) const VELVET_TOP: u32 = 0x2c1640;
pub(super) const VELVET_BOTTOM: u32 = 0x10081a;
pub(super) const INK: u32 = 0xfff6ee;
pub(super) const CREAM: u32 = 0xf1e4cc;
pub(super) const LILAC: u32 = 0xe2d4ff;
pub(super) const DIM: u32 = 0xa999b8;
pub(super) const CRIMSON: u32 = 0xd02a52;
pub(super) const SHADOW: u32 = 0x07030c;

/// A colour of the palette with its opacity, as the shader wants it (linear).
pub(super) fn rgba(hex: u32, alpha: f32) -> [f32; 4] {
    srgb(hex, alpha.clamp(0.0, 1.0))
}

/// Two colours mixed (0: all `a`).
pub(super) fn mix(a: u32, b: u32, t: f32) -> u32 {
    mix_hex(a, b, t.clamp(0.0, 1.0))
}

/// One shape: a quad of `half` size (local pixels, the shape plus room for its edge or glow)
/// around `center`, its local x along `axis`. Colours per corner (tl, tr, br, bl). `tex` maps the
/// shape's own box (`shape_half`) onto a texture region, for faces and pictures.
#[allow(clippy::too_many_arguments)]
fn quad(
    v: &mut Vec<MapVertex>,
    center: [f32; 2],
    axis: [f32; 2],
    half: [f32; 2],
    mode: f32,
    colors: [[f32; 4]; 4],
    shape: [f32; 4],
    color2: [f32; 4],
    extra: [f32; 4],
    tex: Option<([f32; 2], [f32; 2], [f32; 2])>,
) {
    let perp = [-axis[1], axis[0]];
    let mut out = [MapVertex::BLANK; 4];
    for (i, (sx, sy)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .into_iter()
        .enumerate()
    {
        let (lx, ly) = (sx * half[0], sy * half[1]);
        let mut e = extra;
        if let Some((uv0, uv1, sh)) = tex {
            e[0] = uv0[0] + (lx / sh[0] * 0.5 + 0.5) * (uv1[0] - uv0[0]);
            e[1] = uv0[1] + (ly / sh[1] * 0.5 + 0.5) * (uv1[1] - uv0[1]);
        }
        out[i] = MapVertex {
            position: [
                center[0] + axis[0] * lx + perp[0] * ly,
                center[1] + axis[1] * lx + perp[1] * ly,
            ],
            uv: [lx, ly],
            color: colors[i],
            text: mode,
            shape,
            color2,
            extra: e,
        };
    }
    v.extend_from_slice(&[out[0], out[1], out[2], out[0], out[2], out[3]]);
}

const AXIS: [f32; 2] = [1.0, 0.0];

fn vertical(top: [f32; 4], bottom: [f32; 4]) -> [[f32; 4]; 4] {
    [top, top, bottom, bottom]
}

fn center_of(r: Rect) -> [f32; 2] {
    [r.x + r.w * 0.5, r.y + r.h * 0.5]
}

/// A rect grown by `by` on every side.
pub(super) fn grow(r: Rect, by: f32) -> Rect {
    Rect::new(r.x - by, r.y - by, r.w + by * 2.0, r.h + by * 2.0)
}

/// How a panel is dressed.
#[derive(Clone, Copy)]
pub(super) struct Panel {
    pub top: [f32; 4],
    pub bottom: [f32; 4],
    /// The metal rim (its colour is lit from above by the shader) and its width.
    pub rim: [f32; 4],
    pub rim_width: f32,
    /// A fine metal line inside the rim, this far in (0: none), and how strong.
    pub hairline: f32,
    pub hairline_alpha: f32,
    /// Lit top edge, shaded foot, darker toward the sides and a velvet grain (0..1).
    pub bevel: f32,
    /// A gloss sweeping across, at this phase (only its fraction counts; `None`: still).
    pub sheen: Option<f32>,
}

impl Panel {
    /// Velvet glass in a gold rim.
    pub(super) fn velvet(alpha: f32, rim_width: f32) -> Self {
        Self {
            top: rgba(VELVET_TOP, alpha),
            bottom: rgba(VELVET_BOTTOM, alpha),
            rim: rgba(GOLD, 1.0),
            rim_width,
            hairline: 0.0,
            hairline_alpha: 0.0,
            bevel: 1.0,
            sheen: None,
        }
    }

    pub(super) fn fill(top: u32, bottom: u32, alpha: f32) -> Self {
        Self {
            top: rgba(top, alpha),
            bottom: rgba(bottom, alpha),
            rim: [0.0; 4],
            rim_width: 0.0,
            hairline: 0.0,
            hairline_alpha: 0.0,
            bevel: 0.0,
            sheen: None,
        }
    }

    pub(super) fn rim(mut self, hex: u32, alpha: f32, width: f32) -> Self {
        self.rim = rgba(hex, alpha);
        self.rim_width = width;
        self
    }

    pub(super) fn hairline(mut self, inset: f32, alpha: f32) -> Self {
        self.hairline = inset;
        self.hairline_alpha = alpha;
        self
    }

    pub(super) fn bevel(mut self, bevel: f32) -> Self {
        self.bevel = bevel;
        self
    }

    pub(super) fn sheen(mut self, phase: f32) -> Self {
        self.sheen = Some(phase);
        self
    }
}

/// A rounded panel.
pub(super) fn panel(v: &mut Vec<MapVertex>, r: Rect, radius: f32, p: Panel) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        [half[0] + 2.0, half[1] + 2.0],
        BOX,
        vertical(p.top, p.bottom),
        [half[0], half[1], radius, p.rim_width],
        p.rim,
        [
            p.hairline,
            p.hairline_alpha,
            p.bevel,
            p.sheen.map_or(0.0, |s| 1.0 + s.rem_euclid(1.0)),
        ],
        None,
    );
}

/// A soft shadow or glow the shape of a rounded rect (`hollow`: only outside it).
pub(super) fn soft(
    v: &mut Vec<MapVertex>,
    r: Rect,
    radius: f32,
    blur: f32,
    color: [f32; 4],
    hollow: bool,
) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        [half[0] + blur * 2.2, half[1] + blur * 2.2],
        SOFT,
        [color; 4],
        [half[0], half[1], radius, blur],
        [0.0; 4],
        [if hollow { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        None,
    );
}

/// Light hugging the inside of a rounded rect: strongest on its edge, fading `blur` inwards.
pub(super) fn vignette(v: &mut Vec<MapVertex>, r: Rect, radius: f32, blur: f32, color: [f32; 4]) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        half,
        SOFT,
        [color; 4],
        [half[0], half[1], radius, blur],
        [0.0; 4],
        [2.0, 0.0, 0.0, 0.0],
        None,
    );
}

/// A round glow.
pub(super) fn halo(v: &mut Vec<MapVertex>, c: [f32; 2], radius: f32, blur: f32, color: [f32; 4]) {
    soft(
        v,
        Rect::new(c[0] - radius, c[1] - radius, radius * 2.0, radius * 2.0),
        radius,
        blur,
        color,
        false,
    );
}

/// A wire from `a` to `b` with round ends, its colour running from `ca` to `cb`. `metal`: lit
/// along its middle like a gold thread; `glow`: a soft halo this wide.
#[allow(clippy::too_many_arguments)]
pub(super) fn wire(
    v: &mut Vec<MapVertex>,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    ca: [f32; 4],
    cb: [f32; 4],
    glow: f32,
    metal: bool,
) {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let axis = if len > 1e-4 {
        [d[0] / len, d[1] / len]
    } else {
        AXIS
    };
    let r = width * 0.5;
    let pad = 2.0 + glow * 2.2;
    quad(
        v,
        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5],
        axis,
        [len * 0.5 + r + pad, r + pad],
        WIRE,
        [ca, cb, cb, ca],
        [len * 0.5, r, glow, 0.0],
        [0.0; 4],
        [if metal { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        None,
    );
}

/// A filigree curl: a gold thread leaving `from` heading `angle`, winding into a spiral after
/// `length` pixels (`turns` turns; `side` +1 turns clockwise on screen, -1 the other way),
/// thinning from `w0` to `w1`, with a bead at its heart.
#[allow(clippy::too_many_arguments)]
pub(super) fn curl(
    v: &mut Vec<MapVertex>,
    from: [f32; 2],
    angle: f32,
    length: f32,
    turns: f32,
    side: f32,
    w0: f32,
    w1: f32,
    color: [f32; 4],
) {
    let n = ((length / 3.0).ceil() as usize).clamp(10, 200);
    // An Euler spiral: the bend grows with the distance, so the thread winds tighter and tighter.
    let rate = 4.0 * PI * turns / (length * length);
    let ds = length / n as f32;
    let mut p = from;
    let mut w = w0;
    for i in 0..n {
        let s = (i as f32 + 0.5) * ds;
        let heading = angle + side * rate * s * s * 0.5;
        let q = [p[0] + heading.cos() * ds, p[1] + heading.sin() * ds];
        let t = (i + 1) as f32 / n as f32;
        let wq = w0 + (w1 - w0) * t;
        wire(v, p, q, (w + wq) * 0.5, color, color, 0.0, true);
        p = q;
        w = wq;
    }
    disc(v, p, w1 * 1.25, color);
}

/// A round dot (a bead of metal when `color` is gold).
pub(super) fn disc(v: &mut Vec<MapVertex>, c: [f32; 2], radius: f32, color: [f32; 4]) {
    panel(
        v,
        Rect::new(c[0] - radius, c[1] - radius, radius * 2.0, radius * 2.0),
        radius,
        Panel {
            top: color,
            bottom: color,
            rim: [0.0; 4],
            rim_width: 0.0,
            hairline: 0.0,
            hairline_alpha: 0.0,
            bevel: 0.0,
            sheen: None,
        },
    );
}

/// A cut gem in a gold setting: a rhombus `hw` × `hh` (half sizes), lit facets and a glint;
/// `glow`: a halo of its colour this wide.
pub(super) fn gem(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    hw: f32,
    hh: f32,
    stone: u32,
    alpha: f32,
    glow: f32,
) {
    gem_turned(v, c, hw, hh, 0.0, stone, GOLD, alpha, glow);
}

/// A gem turned by `angle` (a leaf of the filigree when its stone is gold too).
#[allow(clippy::too_many_arguments)]
pub(super) fn gem_turned(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    hw: f32,
    hh: f32,
    angle: f32,
    stone: u32,
    setting: u32,
    alpha: f32,
    glow: f32,
) {
    let setting_width = (hw.min(hh) * 0.3).clamp(1.2, 4.0);
    quad(
        v,
        c,
        [angle.cos(), angle.sin()],
        [hw + 2.0 + glow * 2.2, hh + 2.0 + glow * 2.2],
        GEM,
        [rgba(stone, alpha); 4],
        [hw, hh, setting_width, glow],
        rgba(setting, alpha),
        [0.0; 4],
        None,
    );
}

/// A crescent of metal: a disc of `r_out` with a disc of `r_in` cut from it at `cut` (local, in
/// the crescent's frame turned by `angle`). Its horns are where the two circles cross.
#[allow(clippy::too_many_arguments)]
pub(super) fn crescent(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    r_out: f32,
    r_in: f32,
    cut: [f32; 2],
    angle: f32,
    metal: u32,
    alpha: f32,
) {
    quad(
        v,
        c,
        [angle.cos(), angle.sin()],
        [r_out + 2.0, r_out + 2.0],
        CRESCENT,
        [rgba(metal, alpha); 4],
        [r_out, r_in, cut[0], cut[1]],
        rgba(0x2a1608, 0.55 * alpha),
        [0.0; 4],
        None,
    );
}

/// A four-pointed sparkle, `glow` strong.
pub(super) fn sparkle(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    radius: f32,
    color: [f32; 4],
    glow: f32,
) {
    quad(
        v,
        c,
        AXIS,
        [radius * 1.6 + 2.0, radius * 1.6 + 2.0],
        STAR,
        [color; 4],
        [radius, 0.16, glow, 0.0],
        [0.0; 4],
        [0.0; 4],
        None,
    );
}

/// A ring filled clockwise from `start` (turns from the top) for `progress` of a turn (0..1),
/// over a faint `track`.
#[allow(clippy::too_many_arguments)]
pub(super) fn arc(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    radius: f32,
    width: f32,
    start: f32,
    progress: f32,
    color: [f32; 4],
    track: [f32; 4],
) {
    let half = radius + width + 2.0;
    quad(
        v,
        c,
        AXIS,
        [half, half],
        ARC,
        [color; 4],
        [
            radius,
            width,
            progress.clamp(0.0, 1.0),
            start.rem_euclid(1.0),
        ],
        track,
        [0.0; 4],
        None,
    );
}

/// A face from the portrait atlas, masked to a rounded frame, darkened toward its edge.
pub(super) fn face(
    v: &mut Vec<MapVertex>,
    r: Rect,
    radius: f32,
    uv: ([f32; 2], [f32; 2]),
    alpha: f32,
) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        [half[0] + 1.5, half[1] + 1.5],
        FACE,
        [[1.0, 1.0, 1.0, alpha]; 4],
        [half[0], half[1], radius, r.w.min(r.h) * 0.16],
        [0.0; 4],
        [0.0; 4],
        Some((uv.0, uv.1, half)),
    );
}

/// A cut-out from the portrait atlas, its own transparency its shape, tinted by `tint`, with no
/// frame and no shading at its edge: a figure standing on the page.
pub(super) fn cutout(v: &mut Vec<MapVertex>, r: Rect, uv: ([f32; 2], [f32; 2]), tint: [f32; 4]) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        [half[0] + 1.5, half[1] + 1.5],
        FACE,
        [tint; 4],
        [half[0], half[1], 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        Some((uv.0, uv.1, half)),
    );
}

/// A cut of the portrait atlas masked to a rounded frame (`radius` half its side: a circle).
pub(super) fn cutout_round(
    v: &mut Vec<MapVertex>,
    r: Rect,
    uv: ([f32; 2], [f32; 2]),
    tint: [f32; 4],
    radius: f32,
) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        [half[0] + 1.5, half[1] + 1.5],
        FACE,
        [tint; 4],
        [half[0], half[1], radius, 0.0],
        [0.0; 4],
        [0.0; 4],
        Some((uv.0, uv.1, half)),
    );
}

/// The scene's picture (bind group 3), masked to a rounded frame.
pub(super) fn picture(v: &mut Vec<MapVertex>, r: Rect, radius: f32) {
    let half = [r.w * 0.5, r.h * 0.5];
    quad(
        v,
        center_of(r),
        AXIS,
        [half[0] + 1.5, half[1] + 1.5],
        PICTURE,
        [[1.0; 4]; 4],
        [half[0], half[1], radius, 34.0],
        [0.0; 4],
        [0.0; 4],
        Some(([0.0, 0.0], [1.0, 1.0], half)),
    );
}

// ───────────────────────────── ornaments ─────────────────────────────

/// A horn: a stroke from `base` heading `angle`, `length` long, bending by `bend` radians
/// (more toward its tip), thick at its root and sharp at its point, ringed with ridges.
#[allow(clippy::too_many_arguments)]
pub(super) fn horn(
    v: &mut Vec<MapVertex>,
    base: [f32; 2],
    angle: f32,
    length: f32,
    bend: f32,
    width: f32,
    metal: u32,
    alpha: f32,
) {
    let n = ((length / 2.2).ceil() as usize).clamp(10, 140);
    let ds = length / n as f32;
    let mut p = base;
    let mut heading = angle;
    let ridge_gap = (n / 7).max(2);
    for i in 0..n {
        let t = (i as f32 + 0.5) / n as f32;
        // The bend grows toward the tip, like a horn that sweeps out at its end.
        heading += bend * (1.0 + 1.2 * t) / (n as f32 * 1.6);
        let q = [p[0] + heading.cos() * ds, p[1] + heading.sin() * ds];
        let w = width * (1.0 - 0.9 * t.powf(1.2)).max(0.08);
        let shade = mix(GOLD_DEEP, metal, 0.35 + 0.65 * t);
        wire(
            v,
            p,
            q,
            w,
            rgba(shade, alpha),
            rgba(shade, alpha),
            0.0,
            true,
        );
        if i % ridge_gap == ridge_gap / 2 && t > 0.06 && t < 0.78 {
            let across = [-heading.sin() * w * 0.48, heading.cos() * w * 0.48];
            wire(
                v,
                [p[0] - across[0], p[1] - across[1]],
                [p[0] + across[0], p[1] + across[1]],
                (w * 0.16).max(0.8),
                rgba(0x3a2410, 0.55 * alpha),
                rgba(0x3a2410, 0.55 * alpha),
                0.0,
                false,
            );
        }
        p = q;
    }
}

/// The Queen's horns: two horns rising from either side of `c`, `size` tall, sweeping up and
/// out like a lyre — the crest of the house, set over crests and frames.
pub(super) fn horns(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, metal: u32, alpha: f32) {
    for side in [-1.0f32, 1.0] {
        horn(
            v,
            [c[0] + side * size * 0.14, c[1] + size * 0.22],
            -PI * 0.5 + side * 0.32,
            size * 0.95,
            side * 0.95,
            size * 0.17,
            metal,
            alpha,
        );
    }
}

/// A crest: the horns with a gem of `stone` between them.
pub(super) fn crest(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    size: f32,
    stone: u32,
    alpha: f32,
    glow: f32,
) {
    horns(v, [c[0], c[1] - size * 0.08], size, GOLD, alpha);
    gem(v, c, size * 0.24, size * 0.36, stone, alpha, glow);
}

/// A rule of gold across `c`: a thread tapering out each side of a gem, ending in curls that
/// turn `up` (or down).
pub(super) fn rule(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    half_width: f32,
    stone: u32,
    scale: f32,
    up: bool,
    alpha: f32,
) {
    let gold = rgba(GOLD, alpha);
    let faint = rgba(GOLD, 0.0);
    for side in [-1.0f32, 1.0] {
        let a = [c[0] + side * 14.0 * scale, c[1]];
        let b = [c[0] + side * half_width, c[1]];
        wire(v, a, b, 2.4 * scale, gold, faint, 0.0, true);
        // A curl leaving the rule two-thirds of the way out.
        let at = [c[0] + side * half_width * 0.55, c[1]];
        let heading = if side > 0.0 { 0.0 } else { PI };
        let turn = if up { -side } else { side };
        curl(
            v,
            at,
            heading,
            34.0 * scale,
            1.1,
            turn,
            2.0 * scale,
            1.0 * scale,
            gold,
        );
    }
    gem(v, c, 7.0 * scale, 10.0 * scale, stone, alpha, 4.0 * scale);
}

/// A corner flourish on a frame's corner `at`: the frame runs toward `sx` along x and `sy`
/// along y from there. Curls along both edges turning outward, a gem on the corner and, on top
/// corners, a horn sweeping out.
pub(super) fn corner(
    v: &mut Vec<MapVertex>,
    at: [f32; 2],
    sx: f32,
    sy: f32,
    scale: f32,
    stone: u32,
    with_horn: bool,
    alpha: f32,
) {
    let gold = rgba(GOLD, alpha);
    let s = scale;
    // Along the top/bottom edge, curling away from the panel.
    curl(
        v,
        [at[0] + sx * 26.0 * s, at[1]],
        if sx > 0.0 { 0.0 } else { PI },
        64.0 * s,
        1.15,
        -sx * sy,
        2.6 * s,
        1.0 * s,
        gold,
    );
    // Along the side, curling away too.
    curl(
        v,
        [at[0], at[1] + sy * 26.0 * s],
        if sy > 0.0 { PI * 0.5 } else { -PI * 0.5 },
        48.0 * s,
        1.1,
        sx * sy,
        2.4 * s,
        1.0 * s,
        gold,
    );
    // Two leaves at the turn.
    let leaf_angle = (sy).atan2(sx);
    gem_turned(
        v,
        [at[0] + sx * 15.0 * s, at[1] - sy * 9.0 * s],
        9.0 * s,
        3.4 * s,
        leaf_angle - sy * 0.9,
        GOLD,
        GOLD,
        alpha,
        0.0,
    );
    gem_turned(
        v,
        [at[0] - sx * 9.0 * s, at[1] + sy * 15.0 * s],
        9.0 * s,
        3.4 * s,
        leaf_angle + sy * 0.9,
        GOLD,
        GOLD,
        alpha,
        0.0,
    );
    if with_horn {
        // A horn rising out of the corner, sweeping outward.
        horn(
            v,
            [at[0] + sx * 6.0 * s, at[1] - sy * 2.0 * s],
            -sy * PI * 0.5 - sx * 0.42,
            46.0 * s,
            -sx * 1.15,
            9.0 * s,
            GOLD,
            alpha,
        );
    }
    gem(
        v,
        [at[0] + sx * 3.0 * s, at[1] + sy * 3.0 * s],
        6.5 * s,
        9.0 * s,
        stone,
        alpha,
        5.0 * s,
    );
}

/// A key on its cap in the Persona manner: a slanted ink plate with a hard shadow and a rim, its
/// legend in the display face; lit, the plate is bone-white with the accent's rim and a glow.
/// Its shape never changes with the light.
#[allow(clippy::too_many_arguments)]
pub(super) fn keycap(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    label: &str,
    size: f32,
    lit: bool,
    dim: bool,
    accent: u32,
    alpha: f32,
) {
    let lean = (r.h * 0.18).min(8.0);
    let quad = |dx: f32, dy: f32, grow: f32| -> [[f32; 2]; 4] {
        let (x, y, w, h) = (
            r.x + dx - grow,
            r.y + dy - grow,
            r.w + grow * 2.0,
            r.h + grow * 2.0,
        );
        [
            [x + lean, y],
            [x + w + lean, y],
            [x + w - lean, y + h],
            [x - lean, y + h],
        ]
    };
    let fill = |v: &mut Vec<MapVertex>, q: [[f32; 2]; 4], c: [f32; 4]| {
        push_triangle(v, [q[0], q[1], q[2]], c);
        push_triangle(v, [q[0], q[2], q[3]], c);
        push_outline_fringe(v, &q, c);
    };
    if lit {
        soft(v, r, 4.0, r.h * 0.45, rgba(accent, 0.45 * alpha), true);
    }
    fill(v, quad(3.5, 3.5, 0.0), rgba(SHADOW, 0.75 * alpha));
    // The rim (a slightly bigger plate under the face), then the face.
    let rim = if dim {
        0x5a5262
    } else if lit {
        accent
    } else {
        0xf6eef3
    };
    fill(v, quad(0.0, 0.0, 1.6), rgba(rim, alpha));
    let face = if dim {
        0x1c1822
    } else if lit {
        0xf6eef3
    } else {
        0x0e0816
    };
    fill(v, quad(0.0, 0.0, 0.0), rgba(face, alpha));
    let ink = if dim {
        0x9a90a6
    } else if lit {
        0x0e0816
    } else {
        0xffffff
    };
    // A single letter in Anton (H, R, M…) closes up into a block at key size: those take the
    // interface's sans.
    let single = label.chars().count() == 1;
    let face_kind = if single { Face::Ui } else { Face::Display };
    let fitted = {
        let room = r.w - lean - 2.0;
        // Anton is narrow: a size up, so the legend reads as the old face did.
        let size = if single { size * 1.15 } else { size * 1.3 };
        let w = text_width(font, face_kind, label, size);
        if w > room {
            size * room / w
        } else {
            size
        }
    };
    let w = text_width(font, face_kind, label, fitted);
    text(
        font,
        v,
        face_kind,
        label,
        r.x + (r.w - w) * 0.5,
        r.y + (r.h - fitted) * 0.5 - fitted * 0.12,
        fitted,
        rgba(ink, alpha),
    );
}

/// A padlock, `size` tall.
pub(super) fn padlock(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, color: u32, alpha: f32) {
    let s = size;
    // The shackle: the upper half of a ring.
    arc(
        v,
        [c[0], c[1] - s * 0.12],
        s * 0.24,
        s * 0.1,
        0.75,
        0.5,
        rgba(color, alpha),
        [0.0; 4],
    );
    panel(
        v,
        Rect::new(c[0] - s * 0.34, c[1] - s * 0.1, s * 0.68, s * 0.52),
        s * 0.1,
        Panel::fill(color, mix(color, 0x000000, 0.35), alpha).bevel(0.6),
    );
    disc(v, [c[0], c[1] + s * 0.12], s * 0.06, rgba(0x120a16, alpha));
}

/// A chevron pointing down (the line is out: go on).
pub(super) fn chevron(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, color: [f32; 4]) {
    let w = size * 0.55;
    let h = size * 0.32;
    wire(
        v,
        [c[0] - w, c[1] - h],
        [c[0], c[1] + h],
        size * 0.2,
        color,
        color,
        size * 0.25,
        true,
    );
    wire(
        v,
        [c[0] + w, c[1] - h],
        [c[0], c[1] + h],
        size * 0.2,
        color,
        color,
        size * 0.25,
        true,
    );
}

/// An arrow up or down for a change of a value.
pub(super) fn arrow(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, up: bool, color: [f32; 4]) {
    let dir = if up { -1.0 } else { 1.0 };
    let tip = [c[0], c[1] + dir * size * 0.46];
    let tail = [c[0], c[1] - dir * size * 0.46];
    let w = size * 0.4;
    let stroke = size * 0.17;
    wire(v, tail, tip, stroke, color, color, 0.0, false);
    for side in [-1.0f32, 1.0] {
        wire(
            v,
            [c[0] + side * w, tip[1] - dir * size * 0.4],
            tip,
            stroke,
            color,
            color,
            0.0,
            false,
        );
    }
}

/// A slow heartbeat (0..1) at `bpm`, two thumps per beat.
pub(super) fn heartbeat(time: f32, bpm: f32) -> f32 {
    let t = (time * bpm / 60.0).rem_euclid(1.0);
    let thump = |at: f32, k: f32| (-((t - at) * k).powi(2)).exp();
    (thump(0.08, 14.0) + 0.6 * thump(0.3, 16.0)).min(1.0)
}

/// A wave in -1..1.
pub(super) fn wave(time: f32, hz: f32) -> f32 {
    (time * hz * TAU).sin()
}

// ───────────────────────────── text ─────────────────────────────

/// The overlay's typefaces.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Face {
    /// The interface's sans (Segoe UI Semibold).
    Ui,
    /// Lines of dialogue (Segoe UI).
    Body,
    /// Names, titles, keys (Constantia Bold, as the visual novel's display face).
    Title,
    /// Thoughts, narration, captions (Constantia Italic).
    Italic,
    /// The stage's display face: names, stamps, labels (Anton).
    Display,
    /// The stage's lines (Nunito ExtraBold).
    Round,
    /// The stage's thoughts and sounds (Nunito Bold Italic).
    RoundItalic,
    /// The stage's narration (Nunito SemiBold Italic).
    RoundSoft,
}

fn face_glyphs(font: &FontAtlas, face: Face) -> (&HashMap<char, GlyphEntry>, f32) {
    let shop = |i: usize| font.shop_faces.get(i).map(|f| (&f.glyphs, f.ascent));
    match face {
        Face::Ui => None,
        Face::Body => shop(3),
        Face::Title => shop(4),
        Face::Italic => shop(5),
        Face::Display => shop(6),
        Face::Round => shop(7),
        Face::RoundItalic => shop(8),
        Face::RoundSoft => shop(9),
    }
    .unwrap_or((&font.glyphs, font.ascent))
}

pub(super) fn text_width(font: &FontAtlas, face: Face, s: &str, size: f32) -> f32 {
    let s = &*tr(s);
    let (glyphs, _) = face_glyphs(font, face);
    let k = size / FONT_RASTER_SIZE;
    s.split('\n')
        .map(|line| font.run(face as u8 + 1, glyphs, line).advance * k)
        .fold(0.0, f32::max)
}

/// Text from its top-left; `\n` breaks lines (1.42 × size apart).
#[allow(clippy::too_many_arguments)]
/// Text with a rim traced round its letters (`rim` screen pixels wide): one pass, round and even,
/// for stamps and titles.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_rimmed(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    s: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
    rim: f32,
    rim_color: [f32; 4],
) {
    let s = &*tr(s);
    let (glyphs, ascent) = face_glyphs(font, face);
    let k = size / FONT_RASTER_SIZE;
    // The rims of the whole text first, then the letters over them: a letter's rim never cuts into
    // its neighbour (as a sticker's outline would).
    let clear = [color[0], color[1], color[2], 0.0];
    let mut baseline = y + ascent * k;
    for line in s.split('\n') {
        let run = font.run(face as u8 + 1, glyphs, line);
        for (g, pen, dy) in &run.glyphs {
            if g.size[0] > 0.0 {
                push_rimmed_glyph(
                    v,
                    Rect::new(
                        x + (pen + g.bearing[0]) * k,
                        baseline + (dy + g.bearing[1]) * k,
                        g.size[0] * k,
                        g.size[1] * k,
                    ),
                    g.uv_min,
                    g.uv_max,
                    clear,
                    rim,
                    rim_color,
                    k,
                );
            }
        }
        baseline += size * 1.42;
    }
    text(font, v, face, s, x, y, size, color);
}

pub(super) fn text(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    s: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let s = &*tr(s);
    let (glyphs, ascent) = face_glyphs(font, face);
    let k = size / FONT_RASTER_SIZE;
    let mut baseline = y + ascent * k;
    for line in s.split('\n') {
        let run = font.run(face as u8 + 1, glyphs, line);
        for (g, pen, dy) in &run.glyphs {
            if g.size[0] > 0.0 {
                push_textured_rect(
                    v,
                    Rect::new(
                        x + (pen + g.bearing[0]) * k,
                        baseline + (dy + g.bearing[1]) * k,
                        g.size[0] * k,
                        g.size[1] * k,
                    ),
                    g.uv_min,
                    g.uv_max,
                    color,
                );
            }
        }
        baseline += size * 1.42;
    }
}

/// Text with a soft dark shadow under it.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_shadow(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    s: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let d = (size * 0.06).clamp(1.0, 2.5);
    text(
        font,
        v,
        face,
        s,
        x + d * 0.6,
        y + d,
        size,
        [0.0, 0.0, 0.0, color[3] * 0.62],
    );
    text(font, v, face, s, x, y, size, color);
}

/// Text ringed by a dark outline (names on plates, titles over pictures).
#[allow(clippy::too_many_arguments)]
pub(super) fn text_outline(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    s: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
    outline: [f32; 4],
) {
    let w = (size * 0.05).clamp(1.0, 2.4);
    text(
        font,
        v,
        face,
        s,
        x + w * 0.5,
        y + w * 1.2,
        size,
        [outline[0], outline[1], outline[2], outline[3] * 0.6],
    );
    text_rimmed(font, v, face, s, x, y, size, color, w, outline);
}

/// Text centred on `cx`.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_centered(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    s: &str,
    cx: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let w = text_width(font, face, s, size);
    text_shadow(font, v, face, s, cx - w * 0.5, y, size, color);
}

/// `s` broken into lines no wider than `max_width` (words kept whole unless one is too wide).
pub(super) fn wrap(
    font: &FontAtlas,
    face: Face,
    s: &str,
    size: f32,
    max_width: f32,
) -> Vec<String> {
    let s = &*tr(s);
    let mut lines = Vec::new();
    for paragraph in s.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            // A word too wide for a line by itself (a language written without spaces: Chinese,
            // Japanese, Thai) is broken between its letters, by those languages' rules.
            let pieces: Vec<String> = if text_width(font, face, word, size) > max_width {
                demon_queen_dungeon::i18n::wrap_letters(word, max_width, |s| {
                    text_width(font, face, s, size)
                })
            } else {
                vec![word.to_owned()]
            };
            let spaced = pieces.len() == 1;
            for (i, piece) in pieces.into_iter().enumerate() {
                let joint = if spaced || i == 0 { " " } else { "" };
                let candidate = if line.is_empty() {
                    piece.clone()
                } else {
                    format!("{line}{joint}{piece}")
                };
                if text_width(font, face, &candidate, size) <= max_width || line.is_empty() {
                    line = candidate;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = piece;
                }
            }
        }
        lines.push(line);
    }
    lines
}
