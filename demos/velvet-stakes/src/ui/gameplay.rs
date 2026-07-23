//! Cyber-noir gameplay composition and pointer geometry.
//!
//! This module intentionally consumes a small, read-only view model. Game rules,
//! story flow, and input dispatch remain in the host; painting and hit-testing
//! stay deterministic at the 1280 x 720 logical resolution.

use crate::render::{blit_card, blit_cover, fill, panel, text, ArtBank, RgbImage};
use crate::title_font::{draw_font_text, measure_text, title_font, ui_font};
use crate::ui::theme::{Theme, WH, WW};
use velvet_story::pack_rgb;

const COPPER: (u8, u8, u8) = (166, 101, 58);
const COPPER_BRIGHT: (u8, u8, u8) = (226, 151, 91);
const COPPER_DIM: (u8, u8, u8) = (83, 48, 66);
const MAGENTA: (u8, u8, u8) = (236, 47, 172);
const VIOLET: (u8, u8, u8) = (167, 72, 244);
const INK: (u8, u8, u8) = (6, 5, 14);
const CARD_W: i32 = 110;
const CARD_H: i32 = 164;
const CARD_Y: i32 = 536;
const HAND_LEFT: i32 = 286;
const HAND_RIGHT: i32 = 1044;

/// Axis-aligned rectangle in logical gameplay pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl GameplayRect {
    /// Return whether a logical point falls inside the rectangle.
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Visible action in the right gameplay rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayAction {
    /// Score the currently selected cards.
    Play,
    /// Discard the currently selected cards.
    Discard,
    /// Open the pause menu.
    Pause,
}

/// Pointer target returned by gameplay hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayHit {
    /// Hand-card index.
    Card(usize),
    /// Primary play-hand control.
    Play,
    /// Discard control.
    Discard,
    /// Pause control.
    Pause,
}

/// Pointer state used for hover and pressed feedback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameplayInteraction {
    /// Card currently under the pointer.
    pub hovered_card: Option<usize>,
    /// Card currently held down.
    pub pressed_card: Option<usize>,
    /// Action currently under the pointer.
    pub hovered_action: Option<GameplayAction>,
    /// Action currently held down.
    pub pressed_action: Option<GameplayAction>,
}

/// Read-only card data needed by the painter.
#[derive(Debug, Clone, Copy)]
pub struct GameplayCardView<'a> {
    /// Stable catalog identifier used to find card art.
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Short type label, such as `ATK` or `SPL`.
    pub kind: &'a str,
    /// Chip contribution.
    pub chips: i64,
    /// Multiplier contribution.
    pub mult: i64,
    /// Selection state.
    pub selected: bool,
    /// Deal-animation opacity in the `0..=1` range.
    pub opacity: f32,
    /// Deal-animation scale where `1` is resting size.
    pub scale: f32,
}

/// Complete read-only model for one gameplay frame.
#[derive(Debug, Clone, Copy)]
pub struct GameplayView<'a> {
    /// Current chips shown in the top HUD.
    pub chips: i64,
    /// Current or preview multiplier.
    pub multiplier: i64,
    /// Accumulated blind score.
    pub score: i64,
    /// Score required to clear the blind.
    pub target: i64,
    /// Clamped progress through the current blind.
    pub progress: f32,
    /// Current round number.
    pub round: u32,
    /// Total rounds in the displayed run arc.
    pub rounds_total: u32,
    /// Ante number.
    pub ante: u32,
    /// Filled risk pips.
    pub risk: u8,
    /// Total risk pips.
    pub risk_total: u8,
    /// Player display name.
    pub player_name: &'a str,
    /// Player archetype or rank.
    pub player_rank: &'a str,
    /// Hands remaining.
    pub hands_left: u32,
    /// Discards remaining.
    pub discards_left: u32,
    /// Run currency.
    pub money: i64,
    /// Cards left in draw pile.
    pub draw_count: usize,
    /// Cards in discard pile.
    pub discard_count: usize,
    /// Opponent or blind name.
    pub opponent_name: &'a str,
    /// Opponent role line.
    pub opponent_rank: &'a str,
    /// Selected-hand classification.
    pub preview_label: &'a str,
    /// Preview chip count.
    pub preview_chips: i64,
    /// Preview multiplier.
    pub preview_mult: i64,
    /// Preview total score.
    pub preview_total: i64,
    /// Latest gameplay feedback line.
    pub last_message: &'a str,
    /// Cards currently in hand.
    pub hand: &'a [GameplayCardView<'a>],
}

/// Resolve hand-card rectangles for the current hand and selection state.
pub fn hand_card_rects(card_count: usize, selected: &[bool]) -> Vec<GameplayRect> {
    if card_count == 0 {
        return Vec::new();
    }
    let usable = HAND_RIGHT - HAND_LEFT - CARD_W;
    let step = if card_count == 1 {
        0.0
    } else {
        usable as f32 / (card_count - 1) as f32
    };
    let single_offset = if card_count == 1 { usable / 2 } else { 0 };
    (0..card_count)
        .map(|index| GameplayRect {
            x: HAND_LEFT + single_offset + (index as f32 * step).round() as i32,
            y: CARD_Y
                - if selected.get(index).copied().unwrap_or(false) {
                    12
                } else {
                    0
                },
            w: CARD_W,
            h: CARD_H,
        })
        .collect()
}

/// Return the fixed 48px-or-larger hit rectangle for a gameplay action.
pub fn gameplay_action_rect(action: GameplayAction) -> GameplayRect {
    let (y, h) = match action {
        GameplayAction::Play => (342, 58),
        GameplayAction::Discard => (412, 50),
        GameplayAction::Pause => (474, 48),
    };
    GameplayRect {
        x: 1082,
        y,
        w: 178,
        h,
    }
}

/// Hit-test cards and right-rail controls at logical coordinates.
///
/// Cards are tested back-to-front, so the visually topmost card wins when a
/// large hand causes overlap.
pub fn hit_test_gameplay(
    card_count: usize,
    selected: &[bool],
    x: i32,
    y: i32,
) -> Option<GameplayHit> {
    for action in [
        GameplayAction::Play,
        GameplayAction::Discard,
        GameplayAction::Pause,
    ] {
        if gameplay_action_rect(action).contains(x, y) {
            return Some(match action {
                GameplayAction::Play => GameplayHit::Play,
                GameplayAction::Discard => GameplayHit::Discard,
                GameplayAction::Pause => GameplayHit::Pause,
            });
        }
    }
    let rects = hand_card_rects(card_count, selected);
    // Selected cards are painted last and therefore own overlapping pixels.
    for index in (0..card_count).rev() {
        if selected.get(index).copied().unwrap_or(false) && rects[index].contains(x, y) {
            return Some(GameplayHit::Card(index));
        }
    }
    for index in (0..card_count).rev() {
        if !selected.get(index).copied().unwrap_or(false) && rects[index].contains(x, y) {
            return Some(GameplayHit::Card(index));
        }
    }
    None
}

/// Paint the complete cyber-noir gameplay screen.
#[allow(clippy::too_many_arguments)]
pub fn paint_gameplay(
    pixels: &mut [u32],
    theme: &Theme,
    background: Option<&RgbImage>,
    portrait: Option<&RgbImage>,
    art: &ArtBank,
    view: &GameplayView<'_>,
    interaction: GameplayInteraction,
) {
    let has_background = background.is_some();
    if let Some(background) = background {
        blit_cover(pixels, WW, WH, background);
        panel(pixels, WW, WH, 0, 0, WW as i32, WH as i32, INK, 0.18);
    } else {
        fill(pixels, WW, WH, INK);
        paint_fallback_skyline(pixels);
    }

    paint_scene_lighting(pixels);
    paint_table(pixels, has_background);
    paint_top_hud(pixels, theme, view);
    paint_left_rail(pixels, theme, portrait, view);
    paint_opponent(pixels, theme, view);
    paint_action_rail(pixels, theme, view, interaction);
    paint_hand(pixels, theme, art, view, interaction);
    paint_outer_frame(pixels);
}

fn paint_scene_lighting(pixels: &mut [u32]) {
    // A restrained central spotlight separates the playable table from the rails.
    paint_ellipse(pixels, 665, 350, 455, 235, (38, 12, 55), 0.09);
    paint_ellipse(pixels, 665, 394, 356, 158, (97, 22, 110), 0.045);
    panel(pixels, WW, WH, 238, 80, 2, 628, COPPER_DIM, 0.2);
    panel(pixels, WW, WH, 1068, 80, 2, 628, COPPER_DIM, 0.2);
    paint_line_alpha(pixels, 250, 86, 1056, 86, (117, 54, 122), 0.2);
}

fn paint_top_hud(pixels: &mut [u32], theme: &Theme, view: &GameplayView<'_>) {
    paint_cut_panel(pixels, 10, 9, 1260, 70, (5, 4, 12), 0.91, COPPER_DIM, 8);
    paint_line_alpha(pixels, 12, 78, 1268, 78, COPPER, 0.74);

    paint_title_text(pixels, 30, 39, "VELVET ARCANA", 25.0, theme.gold, 1.0, 2);
    paint_centered_text(
        pixels,
        GameplayRect {
            x: 28,
            y: 45,
            w: 237,
            h: 22,
        },
        "NIGHTFALL CASINO",
        9.0,
        COPPER_BRIGHT,
        0.95,
        false,
    );
    paint_line_alpha(pixels, 22, 60, 70, 60, COPPER_DIM, 0.8);
    paint_diamond(pixels, 78, 60, 3, COPPER_BRIGHT, 0.95);
    paint_line_alpha(pixels, 86, 60, 274, 60, COPPER_DIM, 0.8);

    paint_hud_divider(pixels, 286);
    paint_chip(pixels, 314, 43, 13);
    paint_metric(pixels, 337, "CHIPS", &format_counter(view.chips), 42);
    paint_hud_divider(pixels, 432);
    paint_metric(
        pixels,
        453,
        "MULTIPLIER",
        &format!("x{}", view.multiplier.max(1)),
        43,
    );
    paint_hud_divider(pixels, 552);
    paint_metric(pixels, 576, "SCORE", &format_counter(view.score), 44);

    paint_crystal(pixels, 684, 18, 27, 48);
    paint_line_alpha(pixels, 673, 77, 697, 91, COPPER, 0.72);
    paint_line_alpha(pixels, 697, 91, 721, 77, COPPER, 0.72);

    paint_hud_divider(pixels, 728);
    paint_metric(
        pixels,
        752,
        "ROUND",
        &format!("{} / {}", view.round, view.rounds_total.max(1)),
        44,
    );
    paint_hud_divider(pixels, 848);
    paint_metric(pixels, 872, "TARGET", &format_counter(view.target), 44);
    paint_hud_divider(pixels, 972);
    paint_ui_text(pixels, 994, 31, "RISK", 9.5, COPPER_BRIGHT, 0.95, 1);
    for index in 0..view.risk_total.min(6) {
        let filled = index < view.risk;
        let x = 998 + index as i32 * 19;
        paint_diamond(
            pixels,
            x,
            51,
            5,
            if filled { MAGENTA } else { COPPER_DIM },
            if filled { 1.0 } else { 0.72 },
        );
        if filled {
            set_pixel(pixels, x, 49, (255, 214, 236), 1.0);
        }
    }

    paint_cut_panel(pixels, 1214, 20, 42, 44, (15, 10, 24), 0.98, COPPER, 5);
    for y in [33, 42, 51] {
        paint_line_alpha(pixels, 1225, y, 1245, y, COPPER_BRIGHT, 0.95);
    }
}

fn paint_metric(pixels: &mut [u32], x: i32, label: &str, value: &str, value_y: i32) {
    paint_ui_text(pixels, x, 28, label, 8.5, COPPER_BRIGHT, 0.95, 1);
    paint_ui_text(
        pixels,
        x,
        value_y + 13,
        value,
        16.0,
        (229, 218, 233),
        1.0,
        1,
    );
}

fn paint_hud_divider(pixels: &mut [u32], x: i32) {
    paint_line_alpha(pixels, x, 18, x, 69, COPPER_DIM, 0.52);
    paint_diamond(pixels, x, 43, 2, COPPER_BRIGHT, 0.82);
}

fn paint_left_rail(
    pixels: &mut [u32],
    theme: &Theme,
    portrait: Option<&RgbImage>,
    view: &GameplayView<'_>,
) {
    paint_cut_panel(pixels, 12, 91, 224, 617, (5, 4, 13), 0.84, COPPER_DIM, 8);

    paint_cut_panel(pixels, 25, 105, 198, 101, (13, 8, 25), 0.92, COPPER_DIM, 6);
    panel(pixels, WW, WH, 33, 113, 68, 68, (28, 12, 45), 1.0);
    if let Some(portrait) = portrait {
        blit_image_cover(pixels, portrait, 35, 115, 64, 64);
    } else {
        paint_profile_silhouette(pixels, 67, 122);
    }
    paint_rect_frame(pixels, 32, 112, 70, 70, COPPER_BRIGHT, 0.9);
    paint_ui_text(
        pixels,
        111,
        132,
        view.player_name,
        13.5,
        theme.gold_soft,
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        111,
        151,
        view.player_rank,
        9.5,
        (198, 138, 134),
        0.95,
        1,
    );
    paint_diamond(pixels, 41, 190, 7, MAGENTA, 0.96);
    paint_progress_bar(
        pixels,
        55,
        186,
        151,
        8,
        view.hands_left as f32 / 4.0,
        MAGENTA,
    );
    paint_ui_text(
        pixels,
        162,
        199,
        &format!("{} HANDS", view.hands_left),
        8.5,
        theme.text,
        1.0,
        1,
    );

    paint_section_title(pixels, 28, 229, "RUN RULES");
    for (index, (title, detail)) in [
        ("HAND LIMIT", "SELECT UP TO 5 CARDS"),
        ("FOCUS", "DRAW +1 AFTER PLAY"),
    ]
    .iter()
    .enumerate()
    {
        let y = 242 + index as i32 * 57;
        paint_cut_panel(pixels, 26, y, 196, 50, (13, 8, 25), 0.91, COPPER_DIM, 5);
        paint_diamond(
            pixels,
            43,
            y + 16,
            5,
            if index == 0 { COPPER_BRIGHT } else { VIOLET },
            1.0,
        );
        paint_ui_text(pixels, 57, y + 19, title, 10.0, theme.gold_soft, 1.0, 1);
        paint_ui_text(pixels, 57, y + 37, detail, 8.5, theme.muted, 0.95, 1);
    }

    paint_section_title(pixels, 28, 374, "RUN RESOURCES");
    for (index, (label, value)) in [
        ("HANDS", view.hands_left.to_string()),
        ("DISCARD", view.discards_left.to_string()),
        ("CREDITS", format_counter(view.money)),
    ]
    .iter()
    .enumerate()
    {
        let x = 28 + index as i32 * 65;
        paint_cut_panel(pixels, x, 388, 55, 66, (13, 8, 25), 0.93, COPPER_DIM, 5);
        paint_diamond(
            pixels,
            x + 27,
            407,
            7,
            if index == 2 { COPPER_BRIGHT } else { MAGENTA },
            0.95,
        );
        paint_centered_text(
            pixels,
            GameplayRect {
                x,
                y: 416,
                w: 55,
                h: 17,
            },
            value,
            9.5,
            theme.text,
            1.0,
            false,
        );
        paint_centered_text(
            pixels,
            GameplayRect {
                x,
                y: 438,
                w: 55,
                h: 14,
            },
            label,
            6.5,
            theme.muted,
            0.88,
            false,
        );
    }

    paint_section_title(pixels, 28, 488, "CARD ZONES");
    paint_card_pile(pixels, 35, 513, false, view.draw_count);
    paint_card_pile(pixels, 133, 513, true, view.discard_count);
    paint_centered_text(
        pixels,
        GameplayRect {
            x: 24,
            y: 661,
            w: 95,
            h: 18,
        },
        "DRAW DECK",
        8.5,
        COPPER_BRIGHT,
        0.92,
        false,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x: 122,
            y: 661,
            w: 95,
            h: 18,
        },
        "DISCARD",
        8.5,
        COPPER_BRIGHT,
        0.92,
        false,
    );
    paint_ui_text(
        pixels,
        30,
        697,
        &format!("CREDITS  {}", format_counter(view.money)),
        9.0,
        theme.gold_soft,
        1.0,
        1,
    );
}

fn paint_section_title(pixels: &mut [u32], x: i32, y: i32, label: &str) {
    paint_diamond(pixels, x + 2, y - 3, 3, COPPER_BRIGHT, 0.9);
    paint_ui_text(pixels, x + 14, y, label, 8.5, COPPER_BRIGHT, 0.82, 1);
    paint_line_alpha(pixels, x + 14, y + 7, 218, y + 7, COPPER_DIM, 0.5);
}

fn paint_opponent(pixels: &mut [u32], theme: &Theme, view: &GameplayView<'_>) {
    const X: i32 = 276;
    const W: i32 = 778;

    // Blind contract: one strong visual anchor instead of several disconnected labels.
    paint_cut_panel(pixels, X, 94, W, 104, (7, 5, 16), 0.86, COPPER_DIM, 9);
    panel(pixels, WW, WH, X + 1, 95, 4, 102, MAGENTA, 0.72);
    paint_ui_text(
        pixels,
        X + 24,
        118,
        "CURRENT BLIND",
        8.5,
        COPPER_BRIGHT,
        0.92,
        1,
    );
    paint_title_text(
        pixels,
        X + 24,
        151,
        view.opponent_name,
        22.0,
        theme.gold_soft,
        1.0,
        2,
    );
    paint_ui_text(
        pixels,
        X + 25,
        174,
        view.opponent_rank,
        9.0,
        theme.muted,
        0.94,
        1,
    );

    paint_contract_metric(pixels, X + 450, 109, "ANTE", &view.ante.to_string(), theme);
    paint_contract_metric(
        pixels,
        X + 538,
        109,
        "ROUND",
        &format!("{}/{}", view.round, view.rounds_total.max(1)),
        theme,
    );
    paint_contract_metric(
        pixels,
        X + 638,
        109,
        "TARGET",
        &format_counter(view.target),
        theme,
    );

    paint_progress_bar(pixels, X + 450, 166, 298, 10, view.progress, MAGENTA);
    paint_ui_text(
        pixels,
        X + 450,
        191,
        &format!(
            "{} SCORED  /  {} REQUIRED",
            format_counter(view.score),
            format_counter(view.target)
        ),
        9.0,
        theme.text,
        0.94,
        1,
    );

    // Dealer cards remain visible, but are treated as atmospheric opposition.
    for index in 0..3 {
        let x = 561 + index * 73;
        paint_card_back(pixels, x, 214 + (index % 2) * 3, 62, 88, 0.78);
    }
    paint_diamond(pixels, 665, 256, 17, (95, 35, 116), 0.62);
    paint_diamond(pixels, 665, 256, 7, COPPER_BRIGHT, 0.92);

    paint_ui_text(
        pixels,
        292,
        326,
        "BUILD YOUR HAND",
        9.0,
        COPPER_BRIGHT,
        0.94,
        1,
    );
    paint_line_alpha(pixels, 411, 322, 1024, 322, COPPER_DIM, 0.5);

    let selected = view.hand.iter().filter(|card| card.selected).count();
    for index in 0..5 {
        let x = 437 + index * 94;
        let filled = index < selected as i32;
        paint_selection_slot(pixels, x, 342, index + 1, filled, theme);
    }

    // Central scoring equation: the most important decision feedback on the table.
    paint_cut_panel(pixels, 348, 420, 634, 74, (8, 5, 18), 0.91, COPPER_DIM, 8);
    panel(
        pixels,
        WW,
        WH,
        350,
        422,
        4,
        70,
        MAGENTA,
        if selected > 0 { 0.92 } else { 0.28 },
    );
    paint_ui_text(
        pixels,
        369,
        443,
        if selected > 0 {
            "HAND PREVIEW"
        } else {
            "SELECT CARDS"
        },
        8.0,
        COPPER_BRIGHT,
        0.9,
        1,
    );
    paint_title_text(
        pixels,
        369,
        469,
        &view.preview_label.to_uppercase(),
        16.0,
        if selected > 0 {
            theme.gold_soft
        } else {
            theme.muted
        },
        if selected > 0 { 1.0 } else { 0.68 },
        1,
    );
    paint_score_equation(pixels, 648, 432, view, selected > 0, theme);

    paint_centered_text(
        pixels,
        GameplayRect {
            x: 360,
            y: 497,
            w: 610,
            h: 15,
        },
        if view.last_message.is_empty() {
            "CLICK CARDS OR USE KEYS 1-8"
        } else {
            view.last_message
        },
        8.5,
        theme.text,
        0.84,
        false,
    );
}

fn paint_contract_metric(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    label: &str,
    value: &str,
    theme: &Theme,
) {
    paint_cut_panel(pixels, x, y, 86, 48, (13, 8, 25), 0.9, COPPER_DIM, 5);
    paint_centered_text(
        pixels,
        GameplayRect {
            x,
            y: y + 5,
            w: 86,
            h: 15,
        },
        label,
        7.5,
        COPPER_BRIGHT,
        0.88,
        false,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x,
            y: y + 20,
            w: 86,
            h: 23,
        },
        value,
        12.5,
        theme.text,
        1.0,
        false,
    );
}

fn paint_selection_slot(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    number: i32,
    filled: bool,
    theme: &Theme,
) {
    let border = if filled { MAGENTA } else { COPPER_DIM };
    if filled {
        paint_cut_panel(pixels, x - 3, y - 3, 74, 68, MAGENTA, 0.11, MAGENTA, 7);
    }
    paint_cut_panel(
        pixels,
        x,
        y,
        68,
        62,
        if filled { (35, 10, 47) } else { (10, 7, 20) },
        if filled { 0.96 } else { 0.72 },
        border,
        6,
    );
    paint_diamond(
        pixels,
        x + 34,
        y + 29,
        if filled { 11 } else { 8 },
        if filled { MAGENTA } else { COPPER_DIM },
        if filled { 0.9 } else { 0.48 },
    );
    paint_diamond(
        pixels,
        x + 34,
        y + 29,
        3,
        theme.gold_soft,
        if filled { 1.0 } else { 0.5 },
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x,
            y: y + 44,
            w: 68,
            h: 13,
        },
        &number.to_string(),
        7.0,
        if filled { theme.text } else { theme.muted },
        0.82,
        false,
    );
}

fn paint_score_equation(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    view: &GameplayView<'_>,
    active: bool,
    theme: &Theme,
) {
    let alpha = if active { 1.0 } else { 0.52 };
    paint_score_token(
        pixels,
        x,
        y,
        82,
        "CHIPS",
        &format_counter(view.preview_chips),
        COPPER_BRIGHT,
        alpha,
        theme,
    );
    paint_ui_text(pixels, x + 92, y + 31, "x", 16.0, theme.muted, alpha, 1);
    paint_score_token(
        pixels,
        x + 114,
        y,
        70,
        "MULT",
        &view.preview_mult.max(1).to_string(),
        VIOLET,
        alpha,
        theme,
    );
    paint_ui_text(pixels, x + 194, y + 31, "=", 16.0, theme.muted, alpha, 1);
    paint_score_token(
        pixels,
        x + 219,
        y,
        100,
        "TOTAL",
        &format_counter(view.preview_total),
        MAGENTA,
        alpha,
        theme,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_score_token(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    label: &str,
    value: &str,
    accent: (u8, u8, u8),
    alpha: f32,
    theme: &Theme,
) {
    paint_cut_panel(pixels, x, y, w, 42, (13, 8, 25), 0.9 * alpha, accent, 5);
    paint_centered_text(
        pixels,
        GameplayRect {
            x,
            y: y + 3,
            w,
            h: 13,
        },
        label,
        6.8,
        accent,
        0.9 * alpha,
        false,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x,
            y: y + 15,
            w,
            h: 24,
        },
        value,
        12.0,
        theme.text,
        alpha,
        false,
    );
}

fn paint_table(pixels: &mut [u32], has_background: bool) {
    if !has_background {
        paint_ellipse(pixels, 665, 395, 424, 132, (8, 5, 22), 0.76);
    }
    paint_ellipse(
        pixels,
        665,
        405,
        418,
        120,
        (19, 7, 31),
        if has_background { 0.16 } else { 0.26 },
    );
    paint_ellipse_outline(
        pixels,
        665,
        405,
        418,
        120,
        (88, 35, 112),
        if has_background { 1 } else { 3 },
        if has_background { 0.34 } else { 0.68 },
    );
    paint_ellipse_outline(pixels, 665, 405, 404, 108, MAGENTA, 1, 0.28);
    paint_ellipse_outline(pixels, 665, 405, 382, 94, COPPER_DIM, 1, 0.22);
    paint_line_alpha(pixels, 286, 522, 1044, 522, COPPER_DIM, 0.32);
    paint_diamond(pixels, 665, 522, 4, COPPER_BRIGHT, 0.68);
}

fn paint_action_rail(
    pixels: &mut [u32],
    theme: &Theme,
    view: &GameplayView<'_>,
    interaction: GameplayInteraction,
) {
    panel(pixels, WW, WH, 1070, 80, 210, 640, (3, 3, 10), 0.48);
    paint_line_alpha(pixels, 1069, 91, 1069, 704, COPPER_DIM, 0.62);

    let selected = view.hand.iter().filter(|card| card.selected).count();
    paint_cut_panel(pixels, 1082, 101, 178, 174, (9, 6, 19), 0.9, COPPER_DIM, 7);
    panel(
        pixels,
        WW,
        WH,
        1084,
        103,
        3,
        170,
        MAGENTA,
        if selected > 0 { 0.82 } else { 0.26 },
    );
    paint_ui_text(
        pixels,
        1098,
        126,
        "HAND PREVIEW",
        8.5,
        COPPER_BRIGHT,
        0.9,
        1,
    );
    paint_title_text(
        pixels,
        1098,
        154,
        &view.preview_label.to_uppercase(),
        14.0,
        if selected > 0 {
            theme.gold_soft
        } else {
            theme.muted
        },
        if selected > 0 { 1.0 } else { 0.62 },
        1,
    );
    paint_ui_text(pixels, 1098, 182, "SELECTED", 7.2, theme.muted, 0.82, 1);
    paint_ui_text(
        pixels,
        1217,
        182,
        &format!("{} / 5", selected),
        9.5,
        theme.text,
        1.0,
        1,
    );
    paint_progress_bar(pixels, 1098, 193, 146, 7, selected as f32 / 5.0, MAGENTA);
    paint_ui_text(
        pixels,
        1098,
        224,
        "PROJECTED SCORE",
        7.2,
        theme.muted,
        0.82,
        1,
    );
    paint_ui_text(
        pixels,
        1098,
        252,
        &format_counter(view.preview_total),
        21.0,
        if selected > 0 {
            (244, 218, 238)
        } else {
            theme.muted
        },
        if selected > 0 { 1.0 } else { 0.55 },
        2,
    );

    paint_ui_text(pixels, 1092, 310, "ACTIONS", 8.5, COPPER_BRIGHT, 0.82, 1);
    paint_line_alpha(pixels, 1092, 319, 1257, 319, COPPER_DIM, 0.5);

    let can_play = selected > 0 && view.hands_left > 0;
    let can_discard = selected > 0 && view.discards_left > 0;
    paint_action_button(
        pixels,
        theme,
        GameplayAction::Play,
        "PLAY HAND",
        &format!("P  /  {} CARDS", selected),
        can_play,
        true,
        interaction,
    );
    paint_action_button(
        pixels,
        theme,
        GameplayAction::Discard,
        "DISCARD",
        &format!("D  /  {} LEFT", view.discards_left),
        can_discard,
        false,
        interaction,
    );
    paint_action_button(
        pixels,
        theme,
        GameplayAction::Pause,
        "PAUSE",
        "ESC",
        true,
        false,
        interaction,
    );

    paint_cut_panel(
        pixels,
        1082,
        550,
        178,
        106,
        (11, 7, 22),
        0.86,
        COPPER_DIM,
        6,
    );
    paint_ui_text(pixels, 1096, 574, "RUN STATUS", 8.5, COPPER_BRIGHT, 0.88, 1);
    paint_ui_text(
        pixels,
        1096,
        600,
        &format!("HANDS     {}", view.hands_left),
        9.5,
        theme.text,
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        1096,
        624,
        &format!("DISCARDS  {}", view.discards_left),
        9.5,
        theme.text,
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        1096,
        648,
        &format!("ANTE      {}", view.ante),
        9.5,
        theme.muted,
        0.9,
        1,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_action_button(
    pixels: &mut [u32],
    theme: &Theme,
    action: GameplayAction,
    label: &str,
    detail: &str,
    enabled: bool,
    primary: bool,
    interaction: GameplayInteraction,
) {
    let rect = gameplay_action_rect(action);
    let hovered = interaction.hovered_action == Some(action);
    let pressed = interaction.pressed_action == Some(action);
    let inset = if pressed { 2 } else { 0 };
    let draw = GameplayRect {
        x: rect.x + inset,
        y: rect.y + inset,
        w: rect.w - inset * 2,
        h: rect.h - inset * 2,
    };
    let border = if !enabled {
        COPPER_DIM
    } else if primary || hovered {
        COPPER_BRIGHT
    } else {
        COPPER
    };
    let fill_rgb = if primary && enabled {
        (55, 14, 66)
    } else if hovered && enabled {
        (34, 15, 45)
    } else {
        (14, 9, 25)
    };
    if primary && enabled {
        paint_cut_panel(
            pixels,
            draw.x - 3,
            draw.y - 3,
            draw.w + 6,
            draw.h + 6,
            MAGENTA,
            if hovered { 0.16 } else { 0.09 },
            MAGENTA,
            7,
        );
    }
    paint_cut_panel(
        pixels,
        draw.x,
        draw.y,
        draw.w,
        draw.h,
        fill_rgb,
        if enabled { 0.97 } else { 0.68 },
        border,
        7,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x: draw.x,
            y: draw.y + 7,
            w: draw.w,
            h: 23,
        },
        label,
        if primary { 15.0 } else { 12.5 },
        if enabled {
            theme.gold_soft
        } else {
            theme.muted
        },
        if enabled { 1.0 } else { 0.62 },
        primary,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x: draw.x,
            y: draw.y + 33,
            w: draw.w,
            h: 13,
        },
        detail,
        7.5,
        if enabled { theme.text } else { theme.muted },
        if enabled { 0.86 } else { 0.5 },
        false,
    );
}

fn paint_hand(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    view: &GameplayView<'_>,
    interaction: GameplayInteraction,
) {
    let selected: Vec<bool> = view.hand.iter().map(|card| card.selected).collect();
    let rects = hand_card_rects(view.hand.len(), &selected);
    // Resting cards first, selected cards last. This keeps chosen cards readable
    // instead of allowing later unselected cards to cut through their frames.
    for selected_pass in [false, true] {
        for (index, (card, base_rect)) in view.hand.iter().zip(rects.iter().copied()).enumerate() {
            if card.selected != selected_pass {
                continue;
            }
            let hovered = interaction.hovered_card == Some(index);
            let pressed = interaction.pressed_card == Some(index);
            let scale = card.scale.clamp(0.7, 1.05) * if pressed { 0.985 } else { 1.0 };
            let w = (base_rect.w as f32 * scale).round() as i32;
            let h = (base_rect.h as f32 * scale).round() as i32;
            let rect = GameplayRect {
                x: base_rect.x + (base_rect.w - w) / 2,
                y: base_rect.y + (base_rect.h - h) - if hovered && !card.selected { 4 } else { 0 },
                w,
                h,
            };
            paint_card(pixels, theme, art, card, rect, index, hovered);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_card(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    card: &GameplayCardView<'_>,
    rect: GameplayRect,
    index: usize,
    hovered: bool,
) {
    let opacity = card.opacity.clamp(0.0, 1.0);
    if opacity <= 0.01 {
        return;
    }
    let accent = card_kind_color(card.kind);

    panel(
        pixels,
        WW,
        WH,
        rect.x + 5,
        rect.y + 8,
        rect.w,
        rect.h,
        (0, 0, 4),
        0.7 * opacity,
    );
    if card.selected || hovered {
        let glow = if card.selected { MAGENTA } else { accent };
        for spread in (2..=4).rev() {
            paint_cut_outline(
                pixels,
                rect.x - spread,
                rect.y - spread,
                rect.w + spread * 2,
                rect.h + spread * 2,
                glow,
                8,
                (5 - spread) as f32 * if card.selected { 0.13 } else { 0.075 } * opacity,
            );
        }
    }
    paint_cut_panel(
        pixels,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        (8, 6, 17),
        0.995,
        if card.selected { MAGENTA } else { COPPER },
        7,
    );
    panel(
        pixels,
        WW,
        WH,
        rect.x + 4,
        rect.y + 8,
        4,
        rect.h - 16,
        accent,
        0.9 * opacity,
    );

    let art_rect = GameplayRect {
        x: rect.x + 9,
        y: rect.y + 7,
        w: rect.w - 16,
        h: rect.h - 59,
    };
    if let Some(image) = art.images.get(card.id) {
        blit_card(
            pixels, WW, WH, image, art_rect.x, art_rect.y, art_rect.w, art_rect.h, opacity,
        );
    } else {
        paint_card_back(
            pixels, art_rect.x, art_rect.y, art_rect.w, art_rect.h, opacity,
        );
    }
    paint_rect_frame(
        pixels,
        art_rect.x,
        art_rect.y,
        art_rect.w,
        art_rect.h,
        if card.selected { MAGENTA } else { COPPER_DIM },
        0.72 * opacity,
    );

    let footer_y = rect.y + rect.h - 51;
    panel(
        pixels,
        WW,
        WH,
        rect.x + 5,
        footer_y,
        rect.w - 10,
        46,
        (5, 4, 13),
        0.97 * opacity,
    );
    paint_line_alpha(
        pixels,
        rect.x + 8,
        footer_y,
        rect.x + rect.w - 9,
        footer_y,
        if card.selected { MAGENTA } else { accent },
        0.9 * opacity,
    );

    let name = truncate_chars(&card.name.to_uppercase(), 14);
    paint_centered_text(
        pixels,
        GameplayRect {
            x: rect.x + 6,
            y: footer_y + 3,
            w: rect.w - 12,
            h: 18,
        },
        &name,
        9.0,
        theme.gold_soft,
        opacity,
        true,
    );

    paint_stat_badge(
        pixels,
        rect.x + 11,
        footer_y + 24,
        45,
        "CHIP",
        &format!("+{}", card.chips),
        COPPER_BRIGHT,
        opacity,
        theme,
    );
    paint_stat_badge(
        pixels,
        rect.x + rect.w - 57,
        footer_y + 24,
        46,
        "MULT",
        &format!("x{}", card.mult.max(1)),
        accent,
        opacity,
        theme,
    );

    paint_cut_panel(
        pixels,
        rect.x + 10,
        rect.y + 10,
        24,
        22,
        (8, 5, 18),
        0.92 * opacity,
        accent,
        4,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x: rect.x + 10,
            y: rect.y + 11,
            w: 24,
            h: 19,
        },
        &(index + 1).to_string(),
        10.0,
        COPPER_BRIGHT,
        opacity,
        false,
    );

    let kind_w = 36;
    paint_cut_panel(
        pixels,
        rect.x + rect.w - kind_w - 10,
        rect.y + 10,
        kind_w,
        20,
        (8, 5, 18),
        0.9 * opacity,
        accent,
        4,
    );
    paint_centered_text(
        pixels,
        GameplayRect {
            x: rect.x + rect.w - kind_w - 10,
            y: rect.y + 11,
            w: kind_w,
            h: 17,
        },
        card.kind,
        7.5,
        accent,
        opacity,
        false,
    );

    if card.selected {
        paint_diamond(pixels, rect.x + rect.w / 2, rect.y - 4, 5, MAGENTA, opacity);
        paint_diamond(
            pixels,
            rect.x + rect.w / 2,
            rect.y - 4,
            2,
            (255, 221, 240),
            opacity,
        );
    }
}

fn card_kind_color(kind: &str) -> (u8, u8, u8) {
    match kind.to_ascii_uppercase().as_str() {
        "ATK" => (229, 83, 112),
        "DEF" => (87, 157, 232),
        "SPL" => (181, 91, 244),
        "SKL" => (84, 205, 178),
        _ => COPPER_BRIGHT,
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_stat_badge(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    label: &str,
    value: &str,
    accent: (u8, u8, u8),
    opacity: f32,
    theme: &Theme,
) {
    paint_ui_text(pixels, x, y + 8, label, 5.5, accent, 0.82 * opacity, 1);
    paint_ui_text(pixels, x, y + 20, value, 8.0, theme.text, opacity, 1);
    paint_line_alpha(
        pixels,
        x + w - 1,
        y + 2,
        x + w - 1,
        y + 19,
        COPPER_DIM,
        0.4 * opacity,
    );
}

fn paint_card_pile(pixels: &mut [u32], x: i32, y: i32, discarded: bool, count: usize) {
    for offset in (0..4).rev() {
        paint_card_back(
            pixels,
            x + offset * 2,
            y - offset * 2,
            68,
            112,
            if discarded { 0.42 } else { 0.92 },
        );
    }
    paint_cut_panel(pixels, x + 17, y + 102, 38, 27, (8, 5, 17), 0.97, COPPER, 6);
    paint_centered_text(
        pixels,
        GameplayRect {
            x: x + 17,
            y: y + 106,
            w: 38,
            h: 17,
        },
        &count.to_string(),
        11.0,
        COPPER_BRIGHT,
        1.0,
        false,
    );
}

fn paint_card_back(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, opacity: f32) {
    paint_cut_panel(pixels, x, y, w, h, (15, 7, 27), 0.96 * opacity, COPPER, 5);
    paint_cut_outline(
        pixels,
        x + 4,
        y + 4,
        w - 8,
        h - 8,
        (89, 38, 93),
        4,
        0.85 * opacity,
    );
    paint_cut_outline(
        pixels,
        x + 8,
        y + 8,
        w - 16,
        h - 16,
        COPPER_DIM,
        3,
        0.62 * opacity,
    );
    let cx = x + w / 2;
    let cy = y + h / 2;
    paint_diamond(
        pixels,
        cx,
        cy,
        (w / 7).max(5),
        (126, 45, 135),
        0.88 * opacity,
    );
    paint_diamond(
        pixels,
        cx,
        cy,
        (w / 15).max(2),
        COPPER_BRIGHT,
        0.94 * opacity,
    );
    paint_line_alpha(
        pixels,
        x + 12,
        y + 12,
        x + w - 13,
        y + h - 13,
        COPPER_DIM,
        0.46 * opacity,
    );
    paint_line_alpha(
        pixels,
        x + w - 13,
        y + 12,
        x + 12,
        y + h - 13,
        COPPER_DIM,
        0.46 * opacity,
    );
}

fn paint_progress_bar(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fraction: f32,
    fill_rgb: (u8, u8, u8),
) {
    panel(pixels, WW, WH, x, y, w, h, (20, 10, 29), 0.98);
    paint_rect_frame(pixels, x, y, w, h, COPPER_DIM, 0.75);
    let fill_w = ((w - 2) as f32 * fraction.clamp(0.0, 1.0)).round() as i32;
    if fill_w > 0 {
        panel(pixels, WW, WH, x + 1, y + 1, fill_w, h - 2, fill_rgb, 0.95);
        panel(
            pixels,
            WW,
            WH,
            x + 2,
            y + 1,
            (fill_w - 2).max(0),
            1,
            (255, 170, 225),
            0.82,
        );
    }
}

fn paint_outer_frame(pixels: &mut [u32]) {
    paint_cut_outline(pixels, 9, 8, 1262, 702, COPPER, 8, 0.78);
    paint_cut_outline(pixels, 13, 12, 1254, 694, COPPER_DIM, 6, 0.38);
}

fn paint_fallback_skyline(pixels: &mut [u32]) {
    for (index, x) in (250..1110).step_by(38).enumerate() {
        let height = 90 + ((index * 47) % 170) as i32;
        panel(
            pixels,
            WW,
            WH,
            x,
            314 - height,
            24,
            height,
            (17, 8, 37),
            0.92,
        );
        for y in (324 - height..306).step_by(13) {
            panel(
                pixels,
                WW,
                WH,
                x + 5 + (index as i32 % 2) * 7,
                y,
                3,
                5,
                if index % 3 == 0 { MAGENTA } else { VIOLET },
                0.68,
            );
        }
    }
}

fn paint_profile_silhouette(pixels: &mut [u32], cx: i32, top: i32) {
    paint_circle(pixels, cx, top + 17, 11, (84, 39, 111), 1.0);
    for row in 0..29 {
        let span = 12 + row / 2;
        panel(
            pixels,
            WW,
            WH,
            cx - span,
            top + 29 + row,
            span * 2 + 1,
            1,
            (57, 25, 87),
            1.0,
        );
    }
}

fn blit_image_cover(pixels: &mut [u32], image: &RgbImage, x: i32, y: i32, width: i32, height: i32) {
    if width <= 0 || height <= 0 {
        return;
    }
    let (source_width, source_height, source) = image;
    if *source_width == 0 || *source_height == 0 {
        return;
    }
    let target_ratio = width as f32 / height as f32;
    let source_ratio = *source_width as f32 / *source_height as f32;
    let (crop_x, crop_y, crop_width, crop_height) = if source_ratio > target_ratio {
        let crop_width = (*source_height as f32 * target_ratio).round() as u32;
        (
            (*source_width - crop_width) / 2,
            0,
            crop_width,
            *source_height,
        )
    } else {
        let crop_height = (*source_width as f32 / target_ratio).round() as u32;
        (
            0,
            (*source_height - crop_height) / 2,
            *source_width,
            crop_height,
        )
    };
    for row in 0..height {
        let source_y = crop_y + (row as u32 * crop_height) / height as u32;
        for col in 0..width {
            let source_x = crop_x + (col as u32 * crop_width) / width as u32;
            let Some(&color) = source.get((source_y * *source_width + source_x) as usize) else {
                continue;
            };
            let target_x = x + col;
            let target_y = y + row;
            if target_x >= 0 && target_y >= 0 && target_x < WW as i32 && target_y < WH as i32 {
                pixels[(target_y as u32 * WW + target_x as u32) as usize] = color;
            }
        }
    }
}

fn paint_chip(pixels: &mut [u32], cx: i32, cy: i32, radius: i32) {
    paint_circle(pixels, cx, cy, radius, (90, 34, 99), 1.0);
    paint_circle_outline(pixels, cx, cy, radius, COPPER_BRIGHT, 2, 0.95);
    paint_circle_outline(pixels, cx, cy, radius - 5, (217, 61, 184), 2, 0.9);
    paint_diamond(pixels, cx, cy, 3, COPPER_BRIGHT, 1.0);
}

fn paint_crystal(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32) {
    let cx = x + w / 2;
    for row in 0..h {
        let half = if row <= h / 2 {
            ((row + 1) * w / 2 / (h / 2).max(1)).max(1)
        } else {
            ((h - row) * w / 2 / (h / 2).max(1)).max(1)
        };
        for dx in -half..=half {
            let light = 1.0 - dx.abs() as f32 / (half + 1) as f32;
            set_pixel(
                pixels,
                cx + dx,
                y + row,
                (
                    (139.0 + 96.0 * light) as u8,
                    (37.0 + 72.0 * light) as u8,
                    (194.0 + 61.0 * light) as u8,
                ),
                0.98,
            );
        }
    }
    paint_line_alpha(pixels, cx, y + 1, cx, y + h - 2, (255, 202, 244), 0.92);
}

fn paint_circle(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    radius: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= r2 {
                set_pixel(pixels, cx + dx, cy + dy, rgb, opacity);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_circle_outline(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    radius: i32,
    rgb: (u8, u8, u8),
    thickness: i32,
    opacity: f32,
) {
    let outer = radius * radius;
    let inner = (radius - thickness).max(0).pow(2);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d = dx * dx + dy * dy;
            if d <= outer && d >= inner {
                set_pixel(pixels, cx + dx, cy + dy, rgb, opacity);
            }
        }
    }
}

fn paint_ellipse(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    for dy in -ry..=ry {
        let ratio = 1.0 - (dy as f32 * dy as f32) / (ry as f32 * ry as f32);
        let span = (ratio.max(0.0).sqrt() * rx as f32).round() as i32;
        panel(
            pixels,
            WW,
            WH,
            cx - span,
            cy + dy,
            span * 2 + 1,
            1,
            rgb,
            opacity,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_ellipse_outline(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    rgb: (u8, u8, u8),
    thickness: i32,
    opacity: f32,
) {
    for degree in 0..360 {
        let angle = degree as f32 * std::f32::consts::PI / 180.0;
        for inset in 0..thickness.max(1) {
            let x = cx + ((rx - inset) as f32 * angle.cos()).round() as i32;
            let y = cy + ((ry - inset) as f32 * angle.sin()).round() as i32;
            set_pixel(pixels, x, y, rgb, opacity);
        }
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
    if w <= cut * 2 || h <= cut * 2 {
        return;
    }
    for row in 0..h {
        let inset = cut_inset(row, h, cut);
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
    paint_cut_outline(pixels, x, y, w, h, border, cut, opacity.max(0.72));
}

#[allow(clippy::too_many_arguments)]
fn paint_cut_outline(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgb: (u8, u8, u8),
    cut: i32,
    opacity: f32,
) {
    if w <= cut * 2 || h <= cut * 2 {
        return;
    }
    let right = x + w - 1;
    let bottom = y + h - 1;
    paint_line_alpha(pixels, x + cut, y, right - cut, y, rgb, opacity);
    paint_line_alpha(pixels, right - cut, y, right, y + cut, rgb, opacity);
    paint_line_alpha(pixels, right, y + cut, right, bottom - cut, rgb, opacity);
    paint_line_alpha(
        pixels,
        right,
        bottom - cut,
        right - cut,
        bottom,
        rgb,
        opacity,
    );
    paint_line_alpha(pixels, right - cut, bottom, x + cut, bottom, rgb, opacity);
    paint_line_alpha(pixels, x + cut, bottom, x, bottom - cut, rgb, opacity);
    paint_line_alpha(pixels, x, bottom - cut, x, y + cut, rgb, opacity);
    paint_line_alpha(pixels, x, y + cut, x + cut, y, rgb, opacity);
}

fn cut_inset(row: i32, h: i32, cut: i32) -> i32 {
    if row < cut {
        cut - row
    } else if row >= h - cut {
        row - (h - cut - 1)
    } else {
        0
    }
}

fn paint_rect_frame(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    paint_line_alpha(pixels, x, y, x + w - 1, y, rgb, opacity);
    paint_line_alpha(pixels, x + w - 1, y, x + w - 1, y + h - 1, rgb, opacity);
    paint_line_alpha(pixels, x + w - 1, y + h - 1, x, y + h - 1, rgb, opacity);
    paint_line_alpha(pixels, x, y + h - 1, x, y, rgb, opacity);
}

fn paint_diamond(pixels: &mut [u32], cx: i32, cy: i32, size: i32, rgb: (u8, u8, u8), opacity: f32) {
    for dy in -size..=size {
        let span = size - dy.abs();
        panel(
            pixels,
            WW,
            WH,
            cx - span,
            cy + dy,
            span * 2 + 1,
            1,
            rgb,
            opacity,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_line_alpha(
    pixels: &mut [u32],
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        set_pixel(pixels, x0, y0, rgb, opacity);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = error * 2;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
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
fn paint_centered_text(
    pixels: &mut [u32],
    rect: GameplayRect,
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
    let x = rect.x + ((rect.w as f32 - width) * 0.5).round() as i32;
    let baseline = rect.y + ((rect.h as f32 + size * 0.66) * 0.5).round() as i32;
    if title {
        paint_title_text(pixels, x, baseline, value, size, rgb, opacity, 1);
    } else {
        paint_ui_text(pixels, x, baseline, value, size, rgb, opacity, 1);
    }
}

fn truncate_chars(value: &str, max: usize) -> String {
    let count = value.chars().count();
    if count <= max {
        return value.to_string();
    }
    value
        .chars()
        .take(max.saturating_sub(1))
        .chain(['\u{2026}'])
        .collect()
}

fn format_counter(value: i64) -> String {
    let negative = value.is_negative();
    let digits = value.unsigned_abs().to_string();
    let mut grouped =
        String::with_capacity(digits.len() + digits.len() / 3 + usize::from(negative));
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
    fn hand_geometry_stays_between_side_rails() {
        let selected = [false, true, false, false, false, false, false, true];
        let rects = hand_card_rects(8, &selected);
        assert_eq!(rects.len(), 8);
        assert!(rects.iter().all(|rect| rect.x >= 240));
        assert!(rects.iter().all(|rect| rect.x + rect.w <= 1070));
        assert_eq!(rects[1].y, CARD_Y - 12);
        assert_eq!(rects[0].y, CARD_Y);
        assert!(rects.iter().all(|rect| rect.y + rect.h <= WH as i32));
    }

    #[test]
    fn action_targets_are_large_and_separated() {
        let play = gameplay_action_rect(GameplayAction::Play);
        let discard = gameplay_action_rect(GameplayAction::Discard);
        let pause = gameplay_action_rect(GameplayAction::Pause);
        for rect in [play, discard, pause] {
            assert!(rect.w >= 48 && rect.h >= 48);
            assert!(rect.x + rect.w <= WW as i32);
        }
        assert!(discard.y - (play.y + play.h) >= 8);
        assert!(pause.y - (discard.y + discard.h) >= 8);
    }

    #[test]
    fn hit_test_prefers_frontmost_overlapping_card() {
        let selected = vec![false; 12];
        let rects = hand_card_rects(12, &selected);
        let overlap_x = rects[0].x + rects[1].x - rects[0].x + 2;
        let hit = hit_test_gameplay(12, &selected, overlap_x, CARD_Y + 20);
        assert_eq!(hit, Some(GameplayHit::Card(1)));
    }

    #[test]
    fn actions_win_only_inside_their_own_rail() {
        let selected = [false; 8];
        let play = gameplay_action_rect(GameplayAction::Play);
        assert_eq!(
            hit_test_gameplay(8, &selected, play.x + 10, play.y + 10),
            Some(GameplayHit::Play)
        );
        assert_eq!(hit_test_gameplay(8, &selected, 1075, play.y + 10), None);
    }

    #[test]
    fn empty_and_single_hand_geometry_is_stable() {
        assert!(hand_card_rects(0, &[]).is_empty());
        let one = hand_card_rects(1, &[true]);
        assert_eq!(one[0].x + one[0].w / 2, (HAND_LEFT + HAND_RIGHT) / 2);
        assert_eq!(one[0].y, CARD_Y - 12);
    }

    #[test]
    fn counters_are_grouped_for_dense_hud_readability() {
        assert_eq!(format_counter(12_450), "12,450");
        assert_eq!(format_counter(-1_250_000), "-1,250,000");
    }

    #[test]
    #[ignore = "manual visual evidence; run explicitly with --ignored"]
    fn dump_gameplay_png_for_evidence() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let background = load_rgb(&data.join("ui/gameplay_bg_night_broker.png"));
        let portrait = load_rgb(&data.join("ui/portrait_collector.jpg"));
        let art = ArtBank::from_catalog_dir(
            &data.join("art"),
            &["strike", "guard", "fireball", "focus", "bash"],
        );
        let hand = [
            GameplayCardView {
                id: "strike",
                name: "Metropolis",
                kind: "ATK",
                chips: 18,
                mult: 1,
                selected: false,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "guard",
                name: "Velvet Guard",
                kind: "DEF",
                chips: 12,
                mult: 1,
                selected: true,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "fireball",
                name: "Neon Flare",
                kind: "SPL",
                chips: 35,
                mult: 2,
                selected: false,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "focus",
                name: "Focus",
                kind: "SKL",
                chips: 8,
                mult: 1,
                selected: true,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "bash",
                name: "Midnight Bash",
                kind: "ATK",
                chips: 28,
                mult: 1,
                selected: false,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "strike",
                name: "Strike",
                kind: "ATK",
                chips: 18,
                mult: 1,
                selected: false,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "guard",
                name: "Guard",
                kind: "DEF",
                chips: 12,
                mult: 1,
                selected: false,
                opacity: 1.0,
                scale: 1.0,
            },
            GameplayCardView {
                id: "focus",
                name: "Focus",
                kind: "SKL",
                chips: 8,
                mult: 1,
                selected: false,
                opacity: 1.0,
                scale: 1.0,
            },
        ];
        let view = GameplayView {
            chips: 84,
            multiplier: 3,
            score: 134,
            target: 700,
            progress: 134.0 / 700.0,
            round: 2,
            rounds_total: 3,
            ante: 2,
            risk: 3,
            risk_total: 6,
            player_name: "THE COLLECTOR",
            player_rank: "CHAOS DEALER",
            hands_left: 3,
            discards_left: 2,
            money: 1_500,
            draw_count: 12,
            discard_count: 2,
            opponent_name: "NIGHT SYNDICATE BROKER",
            opponent_rank: "BIG BLIND",
            preview_label: "Mixed Hand",
            preview_chips: 84,
            preview_mult: 3,
            preview_total: 252,
            last_message: "Two cards selected",
            hand: &hand,
        };
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_gameplay(
            &mut pixels,
            &Theme::default(),
            background.as_ref(),
            portrait.as_ref(),
            &art,
            &view,
            GameplayInteraction::default(),
        );

        let output =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/gameplay_ui_paint.png");
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
        assert!(std::fs::metadata(output).unwrap().len() > 20_000);
    }
}
