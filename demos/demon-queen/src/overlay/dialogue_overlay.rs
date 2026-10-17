//! The world's conversation box, drawn in code: a pane of velvet glass in a gold frame, the
//! Queen's horns rising from its upper corners, gold curls along its edges and gems of the
//! speaker's colour; the speaker's name on a cartouche crowned by the house crest; their face
//! framed inside on the left; the line typing itself out; and the choices above it as ribbons of
//! glass with a numbered gem, the chosen one lit and swept by a gloss.
use std::f32::consts::PI;

use super::ui_paint::*;
use super::*;

/// The box at 1080p: 1500 × 236, centred, 40 px over the foot of the screen.
pub(super) fn dialogue_rect(width: f32, height: f32) -> Rect {
    let s = height / 1080.0;
    let w = (1500.0 * s).min(width - 120.0 * s);
    Rect::new(
        (width - w) * 0.5,
        height - 40.0 * s - 236.0 * s,
        w,
        236.0 * s,
    )
}

/// The speaker's colour (0xRRGGBB).
fn accent_of(view: &DialogueView) -> u32 {
    ((view.accent[0] as u32) << 16) | ((view.accent[1] as u32) << 8) | view.accent[2] as u32
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_dialogue(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &DialogueView,
    portrait: Option<([f32; 2], [f32; 2])>,
    time: f32,
) {
    let s = height / 1080.0;
    let r = dialogue_rect(width, height);
    let narrator = view.kind == 1 || view.kind == 2;
    let accent = if narrator { 0xd9bd8a } else { accent_of(view) };
    let cx = r.x + r.w * 0.5;

    if !view.choices.is_empty() {
        draw_choices(font, v, width, height, view, time);
    }

    // ── The frame: a deep shadow, an aura of the speaker's colour, the velvet pane.
    soft(
        v,
        Rect::new(r.x + 6.0 * s, r.y + 16.0 * s, r.w - 12.0 * s, r.h),
        30.0 * s,
        30.0 * s,
        rgba(SHADOW, 0.72),
        false,
    );
    soft(
        v,
        grow(r, 2.0 * s),
        30.0 * s,
        24.0 * s,
        rgba(accent, if narrator { 0.10 } else { 0.22 }),
        true,
    );
    panel(
        v,
        r,
        30.0 * s,
        Panel {
            top: rgba(mix(VELVET_TOP, accent, 0.07), 0.96),
            bottom: rgba(VELVET_BOTTOM, 0.97),
            ..Panel::velvet(0.96, 3.2 * s)
        }
        .hairline(11.0 * s, 0.5),
    );
    // A breath of the speaker's colour under the top edge.
    soft(
        v,
        Rect::new(r.x + 60.0 * s, r.y + 18.0 * s, r.w - 120.0 * s, 8.0 * s),
        4.0 * s,
        26.0 * s,
        rgba(accent, 0.07),
        false,
    );

    // ── Ornaments: horns and curls on the corners, gems on the sides, a rule at the foot.
    for (at, sx, sy, horn) in [
        ([r.x, r.y], 1.0, 1.0, true),
        ([r.x + r.w, r.y], -1.0, 1.0, true),
        ([r.x, r.y + r.h], 1.0, -1.0, false),
        ([r.x + r.w, r.y + r.h], -1.0, -1.0, false),
    ] {
        corner(v, at, sx, sy, s * 1.05, accent, horn, 1.0);
    }
    for side in [r.x, r.x + r.w] {
        let c = [side, r.y + r.h * 0.5];
        gem(v, c, 8.0 * s, 14.0 * s, accent, 1.0, 7.0 * s);
        for dir in [-1.0f32, 1.0] {
            let start = [side, c[1] + dir * 20.0 * s];
            let out = if side > cx { 1.0 } else { -1.0 };
            curl(
                v,
                start,
                dir * PI * 0.5,
                34.0 * s,
                1.0,
                -out * dir,
                2.0 * s,
                0.9 * s,
                rgba(GOLD, 1.0),
            );
        }
    }
    rule(v, [cx, r.y + r.h], 150.0 * s, accent, s, false, 1.0);

    // ── The speaker's face, framed inside on the left.
    let mut text_left = r.x + 66.0 * s;
    if let Some(uv) = portrait {
        let side = r.h - 52.0 * s;
        let frame = Rect::new(r.x + 42.0 * s, r.y + 26.0 * s, side, side);
        soft(v, frame, 20.0 * s, 20.0 * s, rgba(accent, 0.32), true);
        panel(
            v,
            grow(frame, 4.0 * s),
            22.0 * s,
            Panel::fill(
                mix(0x1c0f28, accent, 0.30),
                mix(0x0c0612, accent, 0.12),
                1.0,
            )
            .rim(GOLD, 1.0, 3.0 * s)
            .hairline(7.0 * s, 0.4),
        );
        face(v, grow(frame, -3.0 * s), 15.0 * s, uv, 1.0);
        gem(
            v,
            [frame.x + frame.w * 0.5, frame.y + frame.h + 4.0 * s],
            7.0 * s,
            10.0 * s,
            accent,
            1.0,
            5.0 * s,
        );
        text_left = frame.x + frame.w + 40.0 * s;
    }

    // ── The cartouche with the speaker's name, the crest on top (narration: a rule of gold).
    if view.name.is_empty() {
        rule(v, [cx, r.y], 200.0 * s, accent, s * 1.1, true, 1.0);
    } else {
        let size = 31.0 * s;
        let name_w = text_width(font, Face::Title, &view.name, size);
        let plate = Rect::new(
            cx - (name_w * 0.5 + 74.0 * s).max(170.0 * s),
            r.y - 29.0 * s,
            (name_w + 148.0 * s).max(340.0 * s),
            58.0 * s,
        );
        // Threads from the cartouche along the top edge.
        for side in [-1.0f32, 1.0] {
            let a = [cx + side * (plate.w * 0.5 + 6.0 * s), r.y];
            let b = [cx + side * (plate.w * 0.5 + 230.0 * s), r.y];
            wire(
                v,
                a,
                b,
                2.6 * s,
                rgba(GOLD, 1.0),
                rgba(GOLD, 0.0),
                0.0,
                true,
            );
            curl(
                v,
                [cx + side * (plate.w * 0.5 + 40.0 * s), r.y],
                if side > 0.0 { 0.0 } else { PI },
                54.0 * s,
                1.15,
                -side,
                2.2 * s,
                0.9 * s,
                rgba(GOLD, 1.0),
            );
        }
        soft(
            v,
            Rect::new(plate.x, plate.y + 8.0 * s, plate.w, plate.h),
            plate.h * 0.5,
            14.0 * s,
            rgba(SHADOW, 0.7),
            false,
        );
        panel(
            v,
            plate,
            plate.h * 0.5,
            Panel::fill(
                mix(0x3c1c52, accent, 0.16),
                mix(0x170a22, accent, 0.06),
                1.0,
            )
            .rim(GOLD, 1.0, 2.6 * s)
            .hairline(6.0 * s, 0.55)
            .bevel(1.0),
        );
        for side in [-1.0f32, 1.0] {
            gem_turned(
                v,
                [cx + side * plate.w * 0.5, plate.y + plate.h * 0.5],
                12.0 * s,
                15.0 * s,
                0.0,
                GOLD,
                GOLD,
                1.0,
                0.0,
            );
            gem(
                v,
                [cx + side * plate.w * 0.5, plate.y + plate.h * 0.5],
                5.0 * s,
                7.0 * s,
                accent,
                1.0,
                4.0 * s,
            );
        }
        crest(v, [cx, plate.y - 2.0 * s], 40.0 * s, accent, 1.0, 7.0 * s);
        text_outline(
            font,
            v,
            Face::Title,
            &view.name,
            cx - name_w * 0.5,
            plate.y + (plate.h - size) * 0.5 - 2.0 * s,
            size,
            rgba(INK, 1.0),
            rgba(0x12060e, 0.85),
        );
    }

    // ── The line (or the choice's question), typing itself out.
    let body = if view.choices.is_empty() {
        view.text.as_str()
    } else {
        view.prompt.as_deref().unwrap_or("")
    };
    let typed: String = if view.choices.is_empty() {
        body.chars().take(view.shown).collect()
    } else {
        body.to_owned()
    };
    let done = typed.chars().count() >= body.chars().count();
    let (face_kind, ink) = match view.kind {
        1 => (Face::Italic, CREAM),
        2 => (Face::Italic, 0xd8c8b0),
        3 => (Face::Italic, LILAC),
        _ => (Face::Body, INK),
    };
    let (full, typed) = match view.kind {
        2 => (
            format!("— {body} —"),
            format!("— {typed}{}", if done { " —" } else { "" }),
        ),
        3 => (
            format!("({body})"),
            format!("({typed}{}", if done { ")" } else { "" }),
        ),
        _ => (body.to_owned(), typed),
    };
    let text_w = r.x + r.w - 74.0 * s - text_left;
    let mut size = 30.0 * s;
    while size > 22.0 * s && wrap(font, face_kind, &full, size, text_w).len() > 3 {
        size -= 0.5 * s;
    }
    let lines = wrap(font, face_kind, &full, size, text_w);
    let line_h = size * 1.48;
    let area_top = r.y + 30.0 * s;
    let area_h = r.h - 64.0 * s;
    let top = area_top + ((area_h - lines.len() as f32 * line_h) * 0.5).max(0.0);
    let mut left = typed.chars().count();
    let mut end = [text_left, top];
    for (i, line) in lines.iter().enumerate() {
        let n = line.chars().count();
        let piece: String = line.chars().take(left).collect();
        left = left.saturating_sub(n + 1);
        let y = top + i as f32 * line_h;
        let x = if view.kind == 2 {
            text_left + (text_w - text_width(font, face_kind, line, size)) * 0.5
        } else {
            text_left
        };
        text_shadow(font, v, face_kind, &piece, x, y, size, rgba(ink, 1.0));
        if !piece.is_empty() {
            end = [x + text_width(font, face_kind, &piece, size), y];
        }
    }

    // ── The line is out: a gold chevron breathing at its end, the key in the corner.
    if view.continue_key && view.choices.is_empty() && view.shown >= view.text.chars().count() {
        let bob = wave(time, 1.4) * 3.0 * s;
        chevron(
            v,
            [end[0] + 20.0 * s, end[1] + size * 0.62 + bob],
            13.0 * s,
            rgba(GOLD_HI, 0.95),
        );
        keycap(
            font,
            v,
            Rect::new(
                r.x + r.w - 92.0 * s,
                r.y + r.h - 62.0 * s,
                44.0 * s,
                38.0 * s,
            ),
            "F",
            20.0 * s,
            false,
            false,
            accent,
            0.9,
        );
    }
}

/// The choices, above the box: one glass ribbon each, a gem with its number, the chosen one lit.
fn draw_choices(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &DialogueView,
    time: f32,
) {
    let s = height / 1080.0;
    let r = dialogue_rect(width, height);
    let row_h = 60.0 * s;
    let gap = 14.0 * s;
    let row_w = 820.0 * s;
    let n = view.choices.len() as f32;
    let bottom = r.y - 74.0 * s;
    let top = bottom - n * row_h - (n - 1.0) * gap;
    for (i, (label, open)) in view.choices.iter().enumerate() {
        let selected = i == view.selected;
        let row = Rect::new(
            (width - row_w) * 0.5,
            top + i as f32 * (row_h + gap),
            row_w,
            row_h,
        );
        let cy = row.y + row.h * 0.5;
        let lit = selected && *open;
        soft(
            v,
            Rect::new(row.x, row.y + 8.0 * s, row.w, row.h),
            row.h * 0.5,
            16.0 * s,
            rgba(SHADOW, 0.65),
            false,
        );
        if lit {
            let pulse = 0.30 + 0.12 * wave(time, 0.8);
            soft(v, row, row.h * 0.5, 18.0 * s, rgba(0xb46cff, pulse), true);
        }
        let (top_c, bottom_c, rim, ink, stone) = if lit {
            (0x58287a, 0x2a1040, GOLD_HI, 0xfff4e4, 0xb466ff)
        } else if *open {
            (0x2a163e, 0x130a1e, GOLD, 0xeee2d6, 0x7a3cc0)
        } else {
            (0x1e1824, 0x110d16, 0x6a6074, 0x8a8290, 0x4a4452)
        };
        let mut style = Panel::fill(top_c, bottom_c, 0.97)
            .rim(rim, 1.0, 2.0 * s)
            .hairline(5.0 * s, if lit { 0.55 } else { 0.3 })
            .bevel(1.0);
        if lit {
            style = style.sheen(time * 0.35);
        }
        panel(v, row, row.h * 0.5, style);
        // Pointed gold tips on both ends.
        for side in [-1.0f32, 1.0] {
            let tip = [row.x + row.w * 0.5 + side * (row.w * 0.5 + 2.0 * s), cy];
            gem_turned(v, tip, 11.0 * s, 9.0 * s, 0.0, rim, rim, 1.0, 0.0);
        }
        // The number on a gem.
        let gc = [row.x + 42.0 * s, cy];
        gem(
            v,
            gc,
            17.0 * s,
            21.0 * s,
            stone,
            1.0,
            if lit { 9.0 * s } else { 0.0 },
        );
        let number = format!("{}", i + 1);
        let nw = text_width(font, Face::Ui, &number, 22.0 * s);
        text_outline(
            font,
            v,
            Face::Ui,
            &number,
            gc[0] - nw * 0.5,
            cy - 15.0 * s,
            22.0 * s,
            rgba(0xfff6ea, 1.0),
            rgba(0x1a0826, 0.9),
        );
        let size = 25.0 * s;
        let mut fitted = size;
        while fitted > 16.0 * s && text_width(font, Face::Body, label, fitted) > row.w - 150.0 * s {
            fitted -= 0.5 * s;
        }
        text_shadow(
            font,
            v,
            Face::Body,
            label,
            row.x + 80.0 * s,
            cy - fitted * 0.62,
            fitted,
            rgba(ink, 1.0),
        );
        if lit {
            sparkle(
                v,
                [row.x + row.w - 44.0 * s, cy],
                9.0 * s,
                rgba(GOLD_HI, 0.85 + 0.15 * wave(time, 1.3)),
                0.6,
            );
        }
        if !*open {
            padlock(
                v,
                [row.x + row.w - 44.0 * s, cy + 2.0 * s],
                26.0 * s,
                0x8a8290,
                1.0,
            );
        }
    }
}
