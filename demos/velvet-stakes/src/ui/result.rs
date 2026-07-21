//! Premium run-settlement screen and pointer geometry.

use crate::render::{blit_cover, fill, outline, panel, rect, text, RgbImage};
use crate::title_font::{draw_font_text, measure_text, title_font, ui_font};
use crate::ui::theme::{Theme, WH, WW};
use velvet_story::pack_rgb;

const INK: (u8, u8, u8) = (5, 4, 12);
const COPPER_BRIGHT: (u8, u8, u8) = (226, 151, 91);
const COPPER_DIM: (u8, u8, u8) = (83, 48, 66);
const MAGENTA: (u8, u8, u8) = (236, 47, 172);

/// Axis-aligned result-screen rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl ResultRect {
    /// Whether a logical point falls inside the rectangle.
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Visual outcome shown by the settlement screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    /// Intermediate blind victory. Normally routes directly to the market.
    BlindClear,
    /// Blind failed because no hands remained.
    Defeat,
    /// Final blind cleared.
    RunClear,
    /// Flow ended without a scored outcome.
    Aborted,
}

/// Read-only run summary.
#[derive(Debug, Clone, Copy)]
pub struct ResultView<'a> {
    /// Outcome styling and copy.
    pub kind: ResultKind,
    /// Final accumulated score.
    pub score: i64,
    /// Required blind score.
    pub target: i64,
    /// Cash carried at settlement.
    pub cash: i64,
    /// Final deck size.
    pub deck_count: usize,
    /// One-based round reached.
    pub round: u32,
    /// Number of rounds in the run.
    pub rounds_total: u32,
    /// Latest scored combo line.
    pub last_combo: &'a str,
}

/// Keyboard and pointer state for two result actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResultInteraction {
    /// Keyboard-selected action index.
    pub selected: usize,
    /// Action under pointer.
    pub hovered: Option<usize>,
    /// Action currently held down.
    pub pressed: Option<usize>,
}

/// Return the fixed rectangle of a result action.
pub fn result_action_rect(index: usize) -> Option<ResultRect> {
    (index < 2).then_some(ResultRect {
        x: 378,
        y: 520 + index as i32 * 64,
        w: 524,
        h: 54,
    })
}

/// Hit-test the two settlement actions.
pub fn hit_test_result(x: i32, y: i32) -> Option<usize> {
    (0..2).find(|index| result_action_rect(*index).is_some_and(|bounds| bounds.contains(x, y)))
}

/// Paint a complete high-fidelity settlement screen.
pub fn paint_result_screen(
    pixels: &mut [u32],
    theme: &Theme,
    background: Option<&RgbImage>,
    view: &ResultView<'_>,
    interaction: ResultInteraction,
) {
    if let Some(background) = background {
        blit_cover(pixels, WW, WH, background);
    } else {
        fill(pixels, WW, WH, INK);
    }
    panel(pixels, WW, WH, 0, 0, WW as i32, WH as i32, INK, 0.72);
    paint_outer_frame(pixels);
    paint_cut_panel(pixels, 245, 70, 790, 590, INK, 0.93, COPPER_DIM, 10);
    outline(pixels, WW, WH, 254, 79, 772, 572, (54, 22, 70), 1);

    paint_centered(
        pixels,
        ResultRect {
            x: 260,
            y: 90,
            w: 760,
            h: 24,
        },
        "VELVET ARCANA  /  HOUSE SETTLEMENT",
        9.0,
        COPPER_BRIGHT,
        0.92,
        false,
    );
    paint_diamond(pixels, 640, 127, 8, MAGENTA, 0.9);
    rect(pixels, WW, WH, 325, 127, 294, 1, COPPER_DIM);
    rect(pixels, WW, WH, 661, 127, 294, 1, COPPER_DIM);

    let (eyebrow, title, subtitle, title_color) = match view.kind {
        ResultKind::BlindClear => (
            "THE TABLE YIELDS",
            "BLIND CLEARED",
            "The Night Market awaits.",
            (239, 194, 150),
        ),
        ResultKind::Defeat => (
            "THE HOUSE COLLECTS",
            "RUN ENDED",
            "The blind survived your final hand.",
            (238, 115, 133),
        ),
        ResultKind::RunClear => (
            "FORTUNE FAVORS THE BOLD",
            "THE NIGHT IS YOURS",
            "Every blind cleared. The house remembers.",
            (242, 203, 137),
        ),
        ResultKind::Aborted => (
            "TABLE CLOSED",
            "RUN ENDED",
            "Return when the cards call again.",
            theme.gold_soft,
        ),
    };
    paint_centered(
        pixels,
        ResultRect {
            x: 300,
            y: 142,
            w: 680,
            h: 26,
        },
        eyebrow,
        10.0,
        COPPER_BRIGHT,
        1.0,
        false,
    );
    paint_centered(
        pixels,
        ResultRect {
            x: 280,
            y: 168,
            w: 720,
            h: 60,
        },
        title,
        39.0,
        title_color,
        1.0,
        true,
    );
    paint_centered(
        pixels,
        ResultRect {
            x: 320,
            y: 221,
            w: 640,
            h: 28,
        },
        subtitle,
        11.0,
        theme.text,
        0.94,
        false,
    );

    paint_cut_panel(pixels, 320, 258, 640, 108, (12, 7, 25), 0.96, COPPER_DIM, 7);
    paint_ui_text(pixels, 348, 282, "FINAL SCORE", 9.0, COPPER_BRIGHT, 1.0, 1);
    paint_title_text(
        pixels,
        348,
        329,
        &format_counter(view.score),
        38.0,
        (241, 222, 226),
        1.0,
        3,
    );
    paint_ui_text(pixels, 770, 282, "BLIND TARGET", 9.0, COPPER_BRIGHT, 1.0, 1);
    paint_title_text(
        pixels,
        770,
        327,
        &format_counter(view.target),
        28.0,
        theme.gold_soft,
        1.0,
        2,
    );
    paint_progress(
        pixels,
        348,
        344,
        584,
        8,
        if view.target <= 0 {
            0.0
        } else {
            view.score.max(0) as f32 / view.target as f32
        },
        matches!(view.kind, ResultKind::BlindClear | ResultKind::RunClear),
    );

    let stats = [
        (
            "ROUND REACHED",
            format!("{} / {}", view.round, view.rounds_total),
        ),
        ("CASH HELD", format!("${}", format_counter(view.cash))),
        ("FINAL DECK", format!("{} CARDS", view.deck_count)),
    ];
    for (index, (label, value)) in stats.into_iter().enumerate() {
        let x = 320 + index as i32 * 217;
        paint_cut_panel(pixels, x, 380, 206, 72, (10, 6, 21), 0.94, COPPER_DIM, 6);
        paint_centered(
            pixels,
            ResultRect {
                x,
                y: 388,
                w: 206,
                h: 20,
            },
            label,
            8.0,
            COPPER_BRIGHT,
            1.0,
            false,
        );
        paint_centered(
            pixels,
            ResultRect {
                x,
                y: 410,
                w: 206,
                h: 32,
            },
            &value,
            17.0,
            theme.gold_soft,
            1.0,
            true,
        );
    }

    paint_cut_panel(pixels, 320, 465, 640, 40, (19, 8, 31), 0.93, COPPER_DIM, 5);
    paint_ui_text(pixels, 339, 489, "LAST HAND", 8.0, COPPER_BRIGHT, 1.0, 1);
    paint_ui_text(
        pixels,
        429,
        489,
        if view.last_combo.is_empty() {
            "No scored hand recorded"
        } else {
            view.last_combo
        },
        10.0,
        theme.text,
        0.96,
        1,
    );

    let primary = match view.kind {
        ResultKind::RunClear => "PLAY ANOTHER RUN",
        _ => "START A NEW RUN",
    };
    for (index, label) in [primary, "RETURN TO LOBBY"].into_iter().enumerate() {
        let bounds = result_action_rect(index).expect("result action bounds");
        let focused = interaction.hovered == Some(index)
            || (interaction.hovered.is_none() && interaction.selected == index);
        let pressed = interaction.pressed == Some(index);
        paint_action(pixels, bounds, label, index == 0, focused, pressed, theme);
    }
    paint_centered(
        pixels,
        ResultRect {
            x: 300,
            y: 638,
            w: 680,
            h: 17,
        },
        "ARROWS SELECT  /  ENTER CONFIRM  /  ESC LOBBY",
        8.0,
        theme.muted,
        0.9,
        false,
    );
}

fn paint_action(
    pixels: &mut [u32],
    bounds: ResultRect,
    label: &str,
    primary: bool,
    focused: bool,
    pressed: bool,
    theme: &Theme,
) {
    if focused {
        outline(
            pixels,
            WW,
            WH,
            bounds.x - 3,
            bounds.y - 3,
            bounds.w + 6,
            bounds.h + 6,
            (79, 24, 106),
            3,
        );
    }
    let fill_rgb = if pressed {
        (66, 17, 81)
    } else if primary {
        (55, 15, 75)
    } else {
        (16, 9, 28)
    };
    paint_cut_panel(
        pixels,
        bounds.x,
        bounds.y,
        bounds.w,
        bounds.h,
        fill_rgb,
        0.98,
        if focused { COPPER_BRIGHT } else { COPPER_DIM },
        7,
    );
    paint_centered(
        pixels,
        bounds,
        label,
        if primary { 19.0 } else { 15.0 },
        if primary { theme.gold_soft } else { theme.text },
        1.0,
        true,
    );
}

fn paint_progress(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, ratio: f32, success: bool) {
    rect(pixels, WW, WH, x, y, w, h, (26, 13, 36));
    outline(pixels, WW, WH, x, y, w, h, COPPER_DIM, 1);
    let fill_w = ((w - 4) as f32 * ratio.clamp(0.0, 1.0)).round() as i32;
    if fill_w > 0 {
        rect(
            pixels,
            WW,
            WH,
            x + 2,
            y + 2,
            fill_w,
            h - 4,
            if success { MAGENTA } else { (179, 57, 83) },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_cut_panel(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill_rgb: (u8, u8, u8),
    opacity: f32,
    border: (u8, u8, u8),
    cut: i32,
) {
    for row in 0..h {
        let edge = row.min(h - row - 1).max(0);
        let inset = (cut - edge).max(0);
        panel(
            pixels,
            WW,
            WH,
            x + inset,
            y + row,
            w - inset * 2,
            1,
            fill_rgb,
            opacity,
        );
    }
    rect(pixels, WW, WH, x + cut, y, w - cut * 2, 1, border);
    rect(pixels, WW, WH, x + cut, y + h - 1, w - cut * 2, 1, border);
    rect(pixels, WW, WH, x, y + cut, 1, h - cut * 2, border);
    rect(pixels, WW, WH, x + w - 1, y + cut, 1, h - cut * 2, border);
    for step in 0..cut {
        set_pixel(pixels, x + step, y + cut - step, border, 1.0);
        set_pixel(pixels, x + w - 1 - step, y + cut - step, border, 1.0);
        set_pixel(pixels, x + step, y + h - cut + step - 1, border, 1.0);
        set_pixel(
            pixels,
            x + w - step - 1,
            y + h - cut + step - 1,
            border,
            1.0,
        );
    }
}

fn paint_outer_frame(pixels: &mut [u32]) {
    outline(
        pixels,
        WW,
        WH,
        6,
        6,
        WW as i32 - 12,
        WH as i32 - 12,
        (70, 37, 48),
        1,
    );
    rect(pixels, WW, WH, 14, 14, 220, 1, COPPER_DIM);
    rect(pixels, WW, WH, 1046, 14, 220, 1, COPPER_DIM);
    paint_diamond(pixels, 239, 14, 5, COPPER_BRIGHT, 0.85);
    paint_diamond(pixels, 1041, 14, 5, COPPER_BRIGHT, 0.85);
}

fn paint_diamond(pixels: &mut [u32], cx: i32, cy: i32, size: i32, rgb: (u8, u8, u8), opacity: f32) {
    for y in -size..=size {
        let width = size - y.abs();
        for x in -width..=width {
            set_pixel(pixels, cx + x, cy + y, rgb, opacity);
        }
    }
}

fn set_pixel(pixels: &mut [u32], x: i32, y: i32, rgb: (u8, u8, u8), opacity: f32) {
    if x < 0 || y < 0 || x >= WW as i32 || y >= WH as i32 {
        return;
    }
    let index = (y as u32 * WW + x as u32) as usize;
    let Some(dst) = pixels.get_mut(index) else {
        return;
    };
    *dst = blend(*dst, pack_rgb(rgb.0, rgb.1, rgb.2), opacity);
}

fn blend(dst: u32, src: u32, opacity: f32) -> u32 {
    let t = opacity.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xff) as f32;
    let dg = ((dst >> 8) & 0xff) as f32;
    let db = (dst & 0xff) as f32;
    let sr = ((src >> 16) & 0xff) as f32;
    let sg = ((src >> 8) & 0xff) as f32;
    let sb = (src & 0xff) as f32;
    pack_rgb(
        (dr + (sr - dr) * t) as u8,
        (dg + (sg - dg) * t) as u8,
        (db + (sb - db) * t) as u8,
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_ui_text(
    pixels: &mut [u32],
    x: i32,
    baseline: i32,
    value: &str,
    size: f32,
    rgb: (u8, u8, u8),
    opacity: f32,
    fallback_scale: i32,
) {
    if let Some(font) = ui_font() {
        draw_font_text(
            pixels,
            font,
            x as f32,
            baseline as f32,
            value,
            size,
            rgb,
            opacity,
        );
    } else {
        text(
            pixels,
            WW,
            WH,
            x,
            baseline - fallback_scale * 7,
            value,
            rgb,
            fallback_scale,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_title_text(
    pixels: &mut [u32],
    x: i32,
    baseline: i32,
    value: &str,
    size: f32,
    rgb: (u8, u8, u8),
    opacity: f32,
    fallback_scale: i32,
) {
    if let Some(font) = title_font() {
        draw_font_text(
            pixels,
            font,
            x as f32,
            baseline as f32,
            value,
            size,
            rgb,
            opacity,
        );
    } else {
        text(
            pixels,
            WW,
            WH,
            x,
            baseline - fallback_scale * 7,
            value,
            rgb,
            fallback_scale,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_centered(
    pixels: &mut [u32],
    bounds: ResultRect,
    value: &str,
    size: f32,
    rgb: (u8, u8, u8),
    opacity: f32,
    title: bool,
) {
    let width = if title {
        title_font().map(|font| measure_text(font, value, size))
    } else {
        ui_font().map(|font| measure_text(font, value, size))
    }
    .unwrap_or(value.chars().count() as f32 * size * 0.58);
    let x = bounds.x + ((bounds.w as f32 - width) * 0.5).round() as i32;
    let baseline = bounds.y + ((bounds.h as f32 + size * 0.66) * 0.5).round() as i32;
    if title {
        paint_title_text(pixels, x, baseline, value, size, rgb, opacity, 1);
    } else {
        paint_ui_text(pixels, x, baseline, value, size, rgb, opacity, 1);
    }
}

fn format_counter(value: i64) -> String {
    let negative = value.is_negative();
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::new();
    if negative {
        grouped.push('-');
    }
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::load_rgb;
    use std::path::PathBuf;

    #[test]
    fn result_actions_are_large_and_separated() {
        let first = result_action_rect(0).unwrap();
        let second = result_action_rect(1).unwrap();
        assert!(first.w >= 48 && first.h >= 48);
        assert!(second.w >= 48 && second.h >= 48);
        assert!(second.y - (first.y + first.h) >= 8);
        assert_eq!(hit_test_result(first.x + 5, first.y + 5), Some(0));
        assert_eq!(hit_test_result(10, 10), None);
    }

    #[test]
    fn dump_result_png_for_evidence() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let background = load_rgb(&data.join("ui/gameplay_bg_night_broker.png"));
        let view = ResultView {
            kind: ResultKind::RunClear,
            score: 1_684,
            target: 1_400,
            cash: 18,
            deck_count: 23,
            round: 3,
            rounds_total: 3,
            last_combo: "Twin Strike  120x4 = +480",
        };
        let mut pixels = vec![0; (WW * WH) as usize];
        paint_result_screen(
            &mut pixels,
            &Theme::default(),
            background.as_ref(),
            &view,
            ResultInteraction {
                selected: 0,
                ..ResultInteraction::default()
            },
        );
        let output =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/result_ui_paint.png");
        let _ = std::fs::create_dir_all(output.parent().unwrap());
        let mut rgba = Vec::with_capacity((WW * WH * 4) as usize);
        for pixel in pixels {
            rgba.extend_from_slice(&[
                ((pixel >> 16) & 0xff) as u8,
                ((pixel >> 8) & 0xff) as u8,
                (pixel & 0xff) as u8,
                255,
            ]);
        }
        image::save_buffer(&output, &rgba, WW, WH, image::ColorType::Rgba8).unwrap();
        assert!(std::fs::metadata(output).unwrap().len() > 30_000);
    }
}
