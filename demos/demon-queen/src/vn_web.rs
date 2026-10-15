//! The visual novel's guion scenes (the prologue «El Relevo», the world's side scenes) in the web
//! scenes' look, as the termas' and the brothel's stage has it: a slanted dark box with a white rim
//! and a rose shadow, the speaker on a white plate in Anton, the line in Nunito (narration soft and
//! dim, a sound gold), a thought on a white card up top, Chiquita's bubble beside her as she pops up
//! alive in her corner (`vn_chiquita`; in the box when she is on stage), the choices as slanted cards with a big number, a chapter's title
//! card sliding across (`@titulo`) and a lesson's keys up in a corner (`@teclas`). The interactive
//! places are the ones the VN always had (the box, the choice cards, the quick actions): only how
//! they are painted changes.
use unicode_segmentation::UnicodeSegmentation;

use crate::render::{Canvas, Fonts, StageFace};
use crate::{dialogue_layout, game, is_inline_continue, story_choices_layout, vn_chiquita, App};

const INK: u32 = 0xfff6f1ea;
const DIM: u32 = 0xffd6c4cf;
const NIGHT: u32 = 0xff0f070c;
const ROSE: u32 = 0xffff4f9a;
const GOLD: u32 = 0xffe9b949;
const RED: u32 = 0xffe2334a;
const VIOLET: u32 = 0xff7b4fd6;
const BOX: u32 = 0xe010060c;

/// A title card slides in this long, holds until `TITLE_HOLD`, and slides out over `TITLE_OUT`.
const TITLE_IN: f32 = 0.4;
const TITLE_HOLD: f32 = 2.6;
const TITLE_OUT: f32 = 0.5;

type Pt = (f32, f32);

/// A rectangle skewed like CSS `skewX(deg)` about its middle (negative leans its top right).
fn skewed(x: f32, y: f32, w: f32, h: f32, deg: f32) -> [Pt; 4] {
    let k = (-deg).to_radians().tan() * h * 0.5;
    [
        (x + k, y),
        (x + w + k, y),
        (x + w - k, y + h),
        (x - k, y + h),
    ]
}

fn moved(c: [Pt; 4], dx: f32, dy: f32) -> [Pt; 4] {
    c.map(|(x, y)| (x + dx, y + dy))
}

/// A shape with an offset shadow showing only past its right and bottom sides, filled, rimmed.
fn card(canvas: &mut Canvas, c: [Pt; 4], shadow: u32, off: f32, fill: u32, rim: u32) {
    let o = moved(c, off, off);
    canvas.convex(&[c[1], o[1], o[2], c[2]], shadow);
    canvas.convex(&[c[3], c[2], o[2], o[3]], shadow);
    canvas.convex(&c, fill);
    for i in 0..4 {
        canvas.line(c[i], c[(i + 1) % 4], 3.0, rim);
    }
}

/// `VELVET_DIALOGO_CLASICO=1`: the visual novel's old panel and buttons (to compare).
pub(crate) fn classic() -> bool {
    static CLASSIC: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CLASSIC.get_or_init(|| std::env::var_os("VELVET_DIALOGO_CLASICO").is_some())
}

/// A dialogue-footer button in the web look: a slanted plate with its shadow always there (rose,
/// stronger when lit), white with dark ink when lit, rose when pressed. Hover, focus and press
/// change colours only; the place and the size are the button's own (`quick_actions::rects`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_quick_button(
    canvas: &mut Canvas,
    fonts: &Fonts,
    rect: (i32, i32, i32, i32),
    main: &str,
    key: &str,
    enabled: bool,
    lit: bool,
    pressed: bool,
) {
    let (x, y, w, h) = (rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32);
    let c = skewed(x, y, w, h, -10.0);
    let o = moved(c, 4.0, 4.0);
    let shadow = if !enabled {
        0x30ff4f9a
    } else if lit {
        ROSE
    } else {
        0x80ff4f9a
    };
    canvas.convex(&[c[1], o[1], o[2], c[2]], shadow);
    canvas.convex(&[c[3], c[2], o[2], o[3]], shadow);
    let fill = if pressed {
        ROSE
    } else if lit {
        INK
    } else {
        0xd910060c
    };
    canvas.convex(&c, fill);
    let rim = if enabled { 0xccf6f1ea } else { 0x44f6f1ea };
    for i in 0..4 {
        canvas.line(c[i], c[(i + 1) % 4], 1.5, rim);
    }
    let ink = if !enabled {
        0x66d9c6d2
    } else if lit && !pressed {
        NIGHT
    } else {
        INK
    };
    let (key_size, gap) = (14.0, 7);
    // The action's word shrinks when a language's is too long for the button.
    let room = w as i32 - 26
        - if key.is_empty() {
            0
        } else {
            fonts.stage_width(StageFace::RoundSoft, key, key_size) + gap
        };
    let full = fonts.stage_width(StageFace::Round, main, 17.0).max(1);
    let main_size = if full > room {
        (17.0 * room as f32 / full as f32).max(11.0)
    } else {
        17.0
    };
    let main_w = fonts.stage_width(StageFace::Round, main, main_size);
    let key_w = if key.is_empty() {
        0
    } else {
        fonts.stage_width(StageFace::RoundSoft, key, key_size) + gap
    };
    let tx = (x + (w - (main_w + key_w) as f32) * 0.5) as i32;
    let ty = (y + (h - main_size) * 0.5 - 3.0) as i32;
    canvas.stage_text(fonts, StageFace::Round, main, tx, ty, main_size, ink);
    if !key.is_empty() {
        canvas.stage_text(
            fonts,
            StageFace::RoundSoft,
            key,
            tx + main_w + gap,
            ty + 2,
            key_size,
            (ink & 0x00ff_ffff) | 0xa000_0000,
        );
    }
}

/// The Queen speaking or thinking (her plate's shadow is violet).
fn is_queen(id: &str) -> bool {
    matches!(
        id.trim_end_matches("_thought"),
        "gorou" | "reina" | "queen" | "queen_original"
    )
}

/// Chiquita speaks from her corner, unless she is on stage (a sprite of hers is up): then in the
/// box, like everyone on stage.
pub(crate) fn chiquita_in_corner(view: &game::SceneView) -> bool {
    view.speaker_id == "chiquita" && !view.layers.iter().any(|l| l.asset.starts_with("chiquita/"))
}

/// Whether the box is drawn for this node (a thought has its card, Chiquita her corner).
pub(crate) fn has_box(view: &game::SceneView) -> bool {
    !view.speaker_id.ends_with("_thought") && !chiquita_in_corner(view)
}

/// Where a title card's band is `age` seconds into its node: its x offset, `None` once it has gone.
pub(crate) fn title_offset(age: f32, reduced_motion: bool) -> Option<f32> {
    if reduced_motion {
        return (age < TITLE_HOLD).then_some(0.0);
    }
    if age < TITLE_IN {
        let t = age / TITLE_IN;
        Some(-2200.0 * (1.0 - t).powi(3))
    } else if age < TITLE_HOLD {
        Some(0.0)
    } else if age < TITLE_HOLD + TITLE_OUT {
        let t = (age - TITLE_HOLD) / TITLE_OUT;
        Some(2200.0 * t * t * t)
    } else {
        None
    }
}

/// «PRÓLOGO · 1 | La última ronda»: a black band across the picture, the kicker in rose over the
/// title in big white Anton.
fn paint_title_card(canvas: &mut Canvas, fonts: &Fonts, text: &str, off: f32) {
    let (kicker, title) = match text.split_once('|') {
        Some((k, t)) => (k.trim(), t.trim()),
        None => ("", text.trim()),
    };
    let title = title.to_uppercase();
    let (y, h) = (392.0, 236.0);
    card(
        canvas,
        skewed(-200.0 + off, y, 2320.0, h, -8.0),
        ROSE,
        16.0,
        0xf2080408,
        INK,
    );
    let mut size = 132.0;
    while size > 60.0 && fonts.stage_width(StageFace::Display, &title, size) > 1480 {
        size -= 4.0;
    }
    let x = (240.0 + off) as i32;
    if !kicker.is_empty() {
        canvas.stage_text(
            fonts,
            StageFace::Display,
            kicker,
            x,
            (y + 14.0) as i32,
            44.0,
            ROSE,
        );
    }
    canvas.stage_text(
        fonts,
        StageFace::Display,
        &title,
        x + 4,
        (y + 64.0) as i32 + 4,
        size,
        0xcc000000,
    );
    canvas.stage_text(
        fonts,
        StageFace::Display,
        &title,
        x,
        (y + 64.0) as i32,
        size,
        INK,
    );
}

/// «W A S D · caminar | F · usar»: one row per group, its keys as caps and what they do.
fn paint_keys(canvas: &mut Canvas, fonts: &Fonts, keys: &str) {
    let groups: Vec<(Vec<&str>, &str)> = keys
        .split('|')
        .map(|g| {
            let (caps, label) = g.split_once('·').unwrap_or((g, ""));
            (caps.split_whitespace().collect(), label.trim())
        })
        .collect();
    let (cap_h, row_h, cap_size, label_size) = (58.0f32, 74.0f32, 32.0f32, 30.0f32);
    let cap_w =
        |t: &str| (fonts.stage_width(StageFace::Display, t, cap_size) as f32 + 30.0).max(cap_h);
    let row_w = |caps: &[&str], label: &str| {
        caps.iter().map(|c| cap_w(c) + 10.0).sum::<f32>()
            + 16.0
            + fonts.stage_width(StageFace::Round, label, label_size) as f32
    };
    let w = groups.iter().map(|(c, l)| row_w(c, l)).fold(0.0, f32::max) + 84.0;
    let h = 62.0 + groups.len() as f32 * row_h + 16.0;
    let (x0, y0) = (64.0, 64.0);
    let slant = 6.0f32.to_radians().tan();
    card(canvas, skewed(x0, y0, w, h, -6.0), ROSE, 10.0, BOX, INK);
    let lean = |y: f32| (y0 + h * 0.5 - y) * slant;
    canvas.stage_text(
        fonts,
        StageFace::Display,
        "CONTROLES",
        (x0 + 36.0 + lean(y0 + 30.0)) as i32,
        (y0 + 12.0) as i32,
        26.0,
        ROSE,
    );
    for (i, (caps, label)) in groups.iter().enumerate() {
        let cy = y0 + 58.0 + i as f32 * row_h;
        let mut x = x0 + 36.0 + lean(cy + cap_h * 0.5);
        for cap in caps {
            let cw = cap_w(cap);
            canvas.round_rect(
                (x + 5.0) as i32,
                (cy + 5.0) as i32,
                cw as i32,
                cap_h as i32,
                9,
                ROSE,
                ROSE,
            );
            canvas.round_rect(
                x as i32,
                cy as i32,
                cw as i32,
                cap_h as i32,
                9,
                INK,
                0xffd9cfd6,
            );
            let tw = fonts.stage_width(StageFace::Display, cap, cap_size) as f32;
            canvas.stage_text(
                fonts,
                StageFace::Display,
                cap,
                (x + (cw - tw) * 0.5) as i32,
                (cy + 4.0) as i32,
                cap_size,
                NIGHT,
            );
            x += cw + 10.0;
        }
        if !label.is_empty() {
            canvas.stage_text(
                fonts,
                StageFace::Round,
                label,
                (x + 8.0) as i32 + 2,
                (cy + 8.0) as i32 + 2,
                label_size,
                0x99000000,
            );
            canvas.stage_text(
                fonts,
                StageFace::Round,
                label,
                (x + 8.0) as i32,
                (cy + 8.0) as i32,
                label_size,
                INK,
            );
        }
    }
}

impl App {
    /// The box, the line, the plate or the card, and the choices of a guion scene's node.
    pub(crate) fn paint_vn_web(&mut self, view: &game::SceneView) {
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let age = self.reveal_started.elapsed().as_secs_f32();
        let dialogue = dialogue_layout(&self.styles);
        let (px, py, pw, ph) = (
            dialogue.x as f32,
            dialogue.y as f32,
            dialogue.width as f32,
            dialogue.height as f32,
        );
        // The line types itself out (as the VN always did).
        let total = view.text.graphemes(true).count();
        let visible = if self.reveal_complete {
            total
        } else {
            (age * 72.0).floor().max(1.0) as usize
        }
        .min(total);
        if visible >= total {
            self.reveal_complete = true;
        }
        let shown: String = view.text.graphemes(true).take(visible).collect();
        let thought = view.speaker_id.ends_with("_thought");
        let corner = chiquita_in_corner(view);
        let narrator = view.speaker_id == "narrator" || view.speaker.is_empty();
        let canvas = &mut self.canvas;
        let typed = |lines: &[String]| -> Vec<String> {
            let mut left = shown.graphemes(true).count();
            let mut out = Vec::new();
            for l in lines {
                if left == 0 {
                    break;
                }
                let n = l.graphemes(true).count();
                out.push(l.graphemes(true).take(left).collect());
                left = left.saturating_sub(n + 1);
            }
            out
        };
        if let Some(keys) = &view.keys {
            paint_keys(canvas, fonts, keys);
        }
        if thought {
            // A white card up top, the thinker named over the words (in the prologue Gorou, whose
            // soul it is, whatever body it is in).
            let size = 34.0;
            let lines = fonts.stage_wrap(StageFace::RoundItalic, &view.text, size, 1100);
            let who = if view.id.starts_with(game::PROLOGUE_NODE_PREFIX) {
                "GOROU".to_owned()
            } else {
                Some(
                    view.speaker
                        .split(" (")
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_uppercase(),
                )
                .filter(|w| !w.is_empty())
                .unwrap_or_else(|| "REINA".to_owned())
            };
            let label = format!("{who} · PENSANDO");
            let tw = lines
                .iter()
                .map(|l| fonts.stage_width(StageFace::RoundItalic, l, size))
                .max()
                .unwrap_or(0);
            let w = (tw.max(fonts.stage_width(StageFace::Display, &label, 22.0)) + 80) as f32;
            let h = 22.0 * 2.0 + 30.0 + lines.len() as f32 * size * 1.4;
            let (x, y) = ((1920.0 - w) * 0.5, 120.0);
            canvas.round_rect(
                (x + 10.0) as i32,
                (y + 10.0) as i32,
                w as i32,
                h as i32,
                6,
                0xcc7b4fd6,
                0xcc7b4fd6,
            );
            canvas.round_rect(
                x as i32, y as i32, w as i32, h as i32, 6, 0xf2faf6f0, 0xf2faf6f0,
            );
            canvas.stage_text(
                fonts,
                StageFace::Display,
                &label,
                (x + 40.0) as i32,
                (y + 16.0) as i32,
                22.0,
                VIOLET,
            );
            for (i, l) in typed(&lines).iter().enumerate() {
                canvas.stage_text(
                    fonts,
                    StageFace::RoundItalic,
                    l,
                    (x + 40.0) as i32,
                    (y + 50.0 + i as f32 * size * 1.4) as i32,
                    size,
                    0xff2b2335,
                );
            }
        } else if corner {
            // Her bubble: beside her head while she pops up alive at the bottom right (its tail
            // pointing at her), else top right, with her face in a ring unless the picture shows
            // her already. Choices move it up out of their way.
            let drawn = vn_chiquita::in_the_picture(view);
            let alive = self.chiquita.ready() && !drawn;
            let choosing =
                self.reveal_complete && !is_inline_continue(view) && !view.choices.is_empty();
            let size = 28.0;
            let bw = 432.0;
            let lines = fonts.stage_wrap(StageFace::Round, &view.text, size, (bw - 48.0) as i32);
            let bh = 18.0 * 2.0 + 30.0 + lines.len() as f32 * size * 1.35;
            let (bx, by, tail) = if alive && !choosing {
                let [right, bottom] = vn_chiquita::BUBBLE_ANCHOR;
                let [tx, ty] = vn_chiquita::TAIL_TIP;
                (
                    right - bw,
                    bottom - bh,
                    Some([
                        (right - 2.0, bottom - 70.0),
                        (right - 2.0, bottom - 26.0),
                        (tx, ty),
                    ]),
                )
            } else {
                let (x0, y0) = (1920.0 - 70.0 - 620.0, 70.0);
                if !alive && !drawn {
                    let c = (x0 + 85.0, y0 + 85.0);
                    canvas.smooth_disc(c.0, c.1, 94.0, INK);
                    canvas.smooth_disc(c.0, c.1, 90.0, VIOLET);
                    canvas.smooth_disc(c.0, c.1, 85.0, 0xff2b1d44);
                    if let Some(face) = self.art.original("reference/retratos/chiquita3d_normal") {
                        canvas.blit_rgba_scaled(
                            (c.0 - 85.0) as i32,
                            (c.1 - 85.0) as i32,
                            170,
                            170,
                            face,
                            1.0,
                        );
                    }
                }
                (x0 + 188.0, y0, None)
            };
            if let Some(tail) = &tail {
                canvas.convex(&tail.map(|(x, y)| (x + 8.0, y + 8.0)), VIOLET);
            }
            canvas.round_rect(
                (bx + 8.0) as i32,
                (by + 8.0) as i32,
                bw as i32,
                bh as i32,
                22,
                VIOLET,
                VIOLET,
            );
            canvas.round_rect(bx as i32, by as i32, bw as i32, bh as i32, 22, INK, INK);
            if let Some(tail) = &tail {
                canvas.convex(tail, INK);
            }
            canvas.stage_text(
                fonts,
                StageFace::Display,
                "CHIQUITA",
                (bx + 24.0) as i32,
                (by + 12.0) as i32,
                22.0,
                VIOLET,
            );
            for (i, l) in typed(&lines).iter().enumerate() {
                canvas.stage_text(
                    fonts,
                    StageFace::Round,
                    l,
                    (bx + 24.0) as i32,
                    (by + 44.0 + i as f32 * size * 1.35) as i32,
                    size,
                    0xff241a33,
                );
            }
        } else {
            // The box: slanted, dark, a white rim and a rose shadow.
            let c = skewed(px, py, pw, ph, -8.0);
            card(canvas, c, 0xd9ff4f9a, 14.0, BOX, INK);
            if !narrator {
                let name = view.speaker.to_uppercase();
                let size = 44.0;
                let nw = fonts.stage_width(StageFace::Display, &name, size) as f32 + 60.0;
                let plate = skewed(px + 40.0, py - 50.0, nw, 60.0, -3.0);
                let shadow = if is_queen(&view.speaker_id) {
                    VIOLET
                } else {
                    ROSE
                };
                canvas.convex(&moved(plate, 8.0, 8.0), shadow);
                canvas.convex(&plate, INK);
                canvas.stage_text(
                    fonts,
                    StageFace::Display,
                    &name,
                    (px + 66.0) as i32,
                    (py - 50.0) as i32,
                    size,
                    NIGHT,
                );
            }
            let (face, mut size, color) = if view.sound {
                (StageFace::RoundItalic, 34.0, GOLD)
            } else if narrator {
                (StageFace::RoundSoft, 34.0, DIM)
            } else {
                (StageFace::Round, 34.0, INK)
            };
            let slant = 8.0f32.to_radians().tan();
            // Three rows fit the box; a longer line gets smaller letters.
            let mut lines = fonts.stage_wrap(face, &view.text, size, (pw - 170.0) as i32);
            while lines.len() > 3 && size > 24.0 {
                size -= 1.0;
                lines = fonts.stage_wrap(face, &view.text, size, (pw - 170.0) as i32);
            }
            for (i, l) in typed(&lines).iter().enumerate() {
                let y = py + 40.0 + i as f32 * size * 1.45;
                // Along the box's slant.
                let x = px + 80.0 + (py + ph * 0.5 - y) * slant;
                canvas.stage_text(fonts, face, l, x as i32 + 2, y as i32 + 2, size, 0x99000000);
                canvas.stage_text(fonts, face, l, x as i32, y as i32, size, color);
            }
            // The rose triangle that asks to go on, just after the line's last word.
            if self.reveal_complete && is_inline_continue(view) {
                if let Some(last) = lines.last() {
                    let i = lines.len() - 1;
                    let y = py + 40.0 + i as f32 * size * 1.45 + size * 0.45;
                    let x = px
                        + 80.0
                        + (py + ph * 0.5 - y) * slant
                        + fonts.stage_width(face, last, size) as f32
                        + 22.0;
                    canvas.convex(&[(x - 12.0, y), (x + 12.0, y), (x, y + 15.0)], ROSE);
                }
            }
        }
        // A chapter's title card, over everything but the choices.
        self.vn_card_animating = false;
        if let Some(text) = &view.title_card {
            if let Some(off) = title_offset(age, self.reduced_motion) {
                paint_title_card(&mut self.canvas, fonts, text, off);
                self.vn_card_animating = true;
            }
        }
        // The choices: slanted cards with a big number; the one in focus (one only: the pointer's
        // when it is over one, else the keys') white with a red shadow and the red pointer at it.
        if self.reveal_complete && !is_inline_continue(view) && !view.choices.is_empty() {
            let choices = story_choices_layout(&self.styles, view.choices.len(), dialogue.y);
            let hovered = (0..view.choices.len()).find(|&i| {
                view.choices[i].enabled && choices.contains(i, self.mouse.0, self.mouse.1)
            });
            self.choice_hover_animating = false;
            if let Some(h) = hovered {
                self.choice_sel = h;
            }
            let focus = self.choice_sel;
            let canvas = &mut self.canvas;
            canvas.vertical_scrim(0, choices.y - 160, 1920, 160, 0x0a0410, 0.0, 0.45);
            for (i, choice) in view.choices.iter().enumerate() {
                let (x, y, w, h) = (
                    choices.x as f32 + 60.0,
                    choices.item_y(i) as f32 + 8.0,
                    choices.width as f32 - 120.0,
                    choices.height as f32 - 16.0,
                );
                let sel = choice.enabled && focus == i;
                let c = skewed(x, y, w, h, -10.0);
                if sel {
                    card(canvas, c, RED, 14.0, INK, INK);
                } else {
                    card(canvas, c, 0xb3ff4f9a, 10.0, BOX, INK);
                }
                if sel {
                    crate::title_menu::pointer(canvas, (x - 4.0, y + h * 0.5), 22.0);
                }
                let num = format!("{}", i + 1);
                canvas.stage_text(
                    fonts,
                    StageFace::Display,
                    &num,
                    (x + 26.0) as i32,
                    (y + h * 0.5 - 40.0) as i32,
                    56.0,
                    if sel { RED } else { ROSE },
                );
                let ink = if !choice.enabled {
                    0x80d6c4cf
                } else if sel {
                    NIGHT
                } else {
                    INK
                };
                let size = 32.0;
                canvas.stage_text(
                    fonts,
                    StageFace::Round,
                    &choice.label,
                    (x + 110.0) as i32,
                    (y + h * 0.5 - size * 0.75) as i32,
                    size,
                    ink,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_card_slides_in_holds_and_slides_out() {
        assert!(title_offset(0.0, false).unwrap() < -2000.0);
        assert_eq!(title_offset(1.0, false), Some(0.0));
        assert!(title_offset(TITLE_HOLD + TITLE_OUT * 0.5, false).unwrap() > 0.0);
        assert_eq!(title_offset(TITLE_HOLD + TITLE_OUT + 0.01, false), None);
        // Without motion it simply shows, then goes.
        assert_eq!(title_offset(0.0, true), Some(0.0));
        assert_eq!(title_offset(TITLE_HOLD, true), None);
    }
}
