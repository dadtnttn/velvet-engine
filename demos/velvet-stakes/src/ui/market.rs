//! Playable cyber-noir Night Market and pointer geometry.
//!
//! The painter consumes a read-only model. Purchases, rerolls, run flow, and
//! currency mutations remain in the game host.

use crate::render::{blit_card, blit_cover, fill, outline, panel, rect, text, ArtBank, RgbImage};
use crate::title_font::{draw_font_text, measure_text, title_font, ui_font};
use crate::ui::theme::{Theme, WH, WW};
use velvet_story::pack_rgb;

const INK: (u8, u8, u8) = (5, 4, 12);
const COPPER: (u8, u8, u8) = (166, 101, 58);
const COPPER_BRIGHT: (u8, u8, u8) = (226, 151, 91);
const COPPER_DIM: (u8, u8, u8) = (83, 48, 66);
const MAGENTA: (u8, u8, u8) = (236, 47, 172);
const VIOLET: (u8, u8, u8) = (167, 72, 244);
const OFFER_X: i32 = 250;
const OFFER_Y: i32 = 176;
const OFFER_W: i32 = 112;
const OFFER_H: i32 = 292;
const OFFER_GAP: i32 = 10;

/// Axis-aligned rectangle in the fixed 1280 x 720 market canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl MarketRect {
    /// Whether a logical point lies inside the rectangle.
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Non-card action available in the market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketAction {
    /// Spend run cash to replace all unsold stock.
    Reroll,
    /// Leave the market and enter the next blind.
    Continue,
    /// Return from browse-only mode to the lobby.
    Back,
}

/// Pointer target returned by market hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketHit {
    /// Market offer index.
    Offer(usize),
    /// Reroll stock button.
    Reroll,
    /// Continue-run button.
    Continue,
    /// Browse-mode back button.
    Back,
}

/// Hover and pressed state used by the painter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarketInteraction {
    /// Offer under the pointer.
    pub hovered_offer: Option<usize>,
    /// Offer currently held down.
    pub pressed_offer: Option<usize>,
    /// Action under the pointer.
    pub hovered_action: Option<MarketAction>,
    /// Action currently held down.
    pub pressed_action: Option<MarketAction>,
}

/// Read-only data for one market card.
#[derive(Debug, Clone, Copy)]
pub struct MarketOfferView<'a> {
    /// Catalog id used to resolve art.
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Compact card class.
    pub kind: &'a str,
    /// Rules copy.
    pub rules: &'a str,
    /// Run-cash price.
    pub price: i64,
    /// Whether this exact offer was already purchased.
    pub bought: bool,
    /// Whether current run cash covers the price.
    pub affordable: bool,
}

/// Complete read-only market frame.
#[derive(Debug, Clone, Copy)]
pub struct MarketView<'a> {
    /// Persistent lobby chips shown while browsing.
    pub vault: i64,
    /// Spendable cash inside the current run.
    pub money: i64,
    /// Score from the blind just cleared.
    pub score: i64,
    /// Target of the next blind.
    pub next_target: i64,
    /// One-based next round number.
    pub next_round: u32,
    /// Number of rounds in the run.
    pub rounds_total: u32,
    /// Current run-deck size.
    pub deck_count: usize,
    /// Cost of the next reroll.
    pub reroll_cost: i64,
    /// Whether buying and continuing are enabled.
    pub in_run: bool,
    /// Current keyboard-selected offer.
    pub selected_offer: usize,
    /// Short actionable status message.
    pub status: &'a str,
    /// Live market stock.
    pub offers: &'a [MarketOfferView<'a>],
}

/// Rectangles for up to five visible offers.
pub fn market_offer_rects(count: usize) -> Vec<MarketRect> {
    (0..count.min(5))
        .map(|index| MarketRect {
            x: OFFER_X + index as i32 * (OFFER_W + OFFER_GAP),
            y: OFFER_Y,
            w: OFFER_W,
            h: OFFER_H,
        })
        .collect()
}

/// Fixed, accessible hit rectangle for a market action.
pub fn market_action_rect(action: MarketAction) -> MarketRect {
    match action {
        MarketAction::Reroll => MarketRect {
            x: 25,
            y: 570,
            w: 196,
            h: 56,
        },
        MarketAction::Continue => MarketRect {
            x: 976,
            y: 620,
            w: 286,
            h: 76,
        },
        MarketAction::Back => MarketRect {
            x: 1202,
            y: 18,
            w: 50,
            h: 48,
        },
    }
}

/// Resolve the market target at logical coordinates.
pub fn hit_test_market(offer_count: usize, in_run: bool, x: i32, y: i32) -> Option<MarketHit> {
    if market_action_rect(MarketAction::Reroll).contains(x, y) {
        return Some(MarketHit::Reroll);
    }
    if in_run && market_action_rect(MarketAction::Continue).contains(x, y) {
        return Some(MarketHit::Continue);
    }
    if !in_run && market_action_rect(MarketAction::Back).contains(x, y) {
        return Some(MarketHit::Back);
    }
    market_offer_rects(offer_count)
        .iter()
        .enumerate()
        .find_map(|(index, bounds)| bounds.contains(x, y).then_some(MarketHit::Offer(index)))
}

/// Paint the complete Night Market screen.
pub fn paint_market(
    pixels: &mut [u32],
    theme: &Theme,
    background: Option<&RgbImage>,
    art: &ArtBank,
    view: &MarketView<'_>,
    interaction: MarketInteraction,
) {
    if let Some(background) = background {
        blit_cover(pixels, WW, WH, background);
        panel(pixels, WW, WH, 0, 0, WW as i32, WH as i32, INK, 0.08);
    } else {
        fill(pixels, WW, WH, INK);
        paint_fallback_market(pixels);
    }

    paint_top_hud(pixels, theme, view, interaction);
    paint_left_rail(pixels, theme, art, view, interaction);
    paint_stock(pixels, theme, art, view, interaction);
    paint_continue(pixels, theme, view, interaction);
    paint_outer_frame(pixels);
}

fn paint_top_hud(
    pixels: &mut [u32],
    theme: &Theme,
    view: &MarketView<'_>,
    interaction: MarketInteraction,
) {
    paint_cut_panel(pixels, 10, 9, 1260, 70, INK, 0.92, COPPER_DIM, 8);
    rect(pixels, WW, WH, 12, 78, 1256, 1, COPPER);

    paint_title_text(pixels, 28, 46, "VELVET ARCANA", 26.0, theme.gold, 1.0, 2);
    paint_ui_text(
        pixels,
        30,
        65,
        "NIGHTFALL CASINO  /  NIGHT MARKET",
        8.5,
        COPPER_BRIGHT,
        0.86,
        1,
    );
    paint_hud_divider(pixels, 302);

    let currency_label = if view.in_run { "RUN CASH" } else { "VAULT" };
    let currency = if view.in_run { view.money } else { view.vault };
    paint_chip(pixels, 337, 44, 17);
    paint_metric(pixels, 365, currency_label, &format_counter(currency), 53);
    paint_hud_divider(pixels, 482);
    paint_metric(
        pixels,
        504,
        "DECK",
        &format!("{} CARDS", view.deck_count),
        53,
    );
    paint_hud_divider(pixels, 634);
    paint_metric(pixels, 657, "LAST SCORE", &format_counter(view.score), 53);
    paint_hud_divider(pixels, 815);
    paint_metric(
        pixels,
        838,
        "NEXT ROUND",
        &format!("{} / {}", view.next_round, view.rounds_total),
        53,
    );
    paint_hud_divider(pixels, 979);
    paint_metric(
        pixels,
        1002,
        "NEXT TARGET",
        &format_counter(view.next_target),
        53,
    );

    if view.in_run {
        paint_ui_text(pixels, 1179, 37, "RUN", 8.0, COPPER_BRIGHT, 0.9, 1);
        paint_ui_text(pixels, 1160, 55, "ACTIVE", 11.0, theme.text, 1.0, 1);
    } else {
        let bounds = market_action_rect(MarketAction::Back);
        let hovered = interaction.hovered_action == Some(MarketAction::Back);
        let pressed = interaction.pressed_action == Some(MarketAction::Back);
        paint_action_plate(pixels, bounds, hovered, pressed, true, "X", 17.0, false);
    }
}

fn paint_left_rail(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    view: &MarketView<'_>,
    interaction: MarketInteraction,
) {
    paint_cut_panel(pixels, 12, 92, 220, 548, INK, 0.91, COPPER_DIM, 8);
    paint_title_text(pixels, 28, 130, "THE NIGHT", 23.0, theme.gold, 1.0, 2);
    paint_title_text(pixels, 28, 154, "MARKET", 23.0, theme.gold_soft, 1.0, 2);
    paint_ui_text(
        pixels,
        29,
        176,
        "RARE GOODS. ONE ROUND ONLY.",
        8.0,
        COPPER_BRIGHT,
        0.85,
        1,
    );
    rect(pixels, WW, WH, 28, 187, 187, 1, COPPER_DIM);

    let focus_index = interaction
        .hovered_offer
        .unwrap_or(view.selected_offer)
        .min(view.offers.len().saturating_sub(1));
    if let Some(offer) = view.offers.get(focus_index) {
        paint_ui_text(pixels, 29, 210, "FEATURED CARD", 8.5, COPPER_BRIGHT, 1.0, 1);
        paint_cut_panel(pixels, 25, 222, 194, 197, (11, 7, 24), 0.95, COPPER_DIM, 6);
        if let Some(image) = art.images.get(offer.id) {
            blit_card(pixels, WW, WH, image, 34, 238, 72, 116, 0.98);
            outline(pixels, WW, WH, 32, 236, 76, 120, COPPER, 1);
        }
        paint_ui_text(
            pixels,
            118,
            252,
            &truncate_chars(&offer.name.to_ascii_uppercase(), 13),
            12.0,
            theme.gold_soft,
            1.0,
            1,
        );
        paint_ui_text(pixels, 118, 273, offer.kind, 8.5, VIOLET, 1.0, 1);
        let rule_lines = split_rule_lines(offer.rules, 15);
        for (line_index, line) in rule_lines.iter().take(3).enumerate() {
            paint_ui_text(
                pixels,
                118,
                295 + line_index as i32 * 17,
                line,
                9.0,
                theme.text,
                0.94,
                1,
            );
        }
        rect(pixels, WW, WH, 117, 342, 89, 1, COPPER_DIM);
        paint_chip(pixels, 128, 374, 8);
        paint_title_text(
            pixels,
            144,
            381,
            &format!("${}", offer.price),
            19.0,
            if offer.affordable || offer.bought {
                theme.gold_soft
            } else {
                (236, 112, 126)
            },
            1.0,
            1,
        );
        paint_ui_text(
            pixels,
            35,
            402,
            if offer.bought {
                "SOLD TO THE COLLECTOR"
            } else if offer.affordable {
                "ENTER / CLICK TO BUY"
            } else {
                "MORE CASH REQUIRED"
            },
            8.0,
            if offer.bought { MAGENTA } else { theme.muted },
            1.0,
            1,
        );
    }

    paint_cut_panel(pixels, 25, 436, 194, 113, (12, 8, 26), 0.94, COPPER_DIM, 6);
    paint_ui_text(
        pixels,
        38,
        457,
        "MARKET MESSAGE",
        8.5,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    for (index, line) in wrap_words(view.status, 27).iter().take(4).enumerate() {
        paint_ui_text(
            pixels,
            38,
            480 + index as i32 * 16,
            line,
            9.0,
            theme.text,
            0.96,
            1,
        );
    }

    let reroll = market_action_rect(MarketAction::Reroll);
    let enabled = view.in_run && view.money >= view.reroll_cost;
    let hovered = interaction.hovered_action == Some(MarketAction::Reroll);
    let pressed = interaction.pressed_action == Some(MarketAction::Reroll);
    paint_action_plate(
        pixels,
        reroll,
        hovered,
        pressed,
        enabled,
        &format!("REROLL STOCK   ${}", view.reroll_cost),
        14.0,
        true,
    );
    paint_ui_text(
        pixels,
        49,
        637,
        if view.in_run {
            "R  /  CLICK"
        } else {
            "START A RUN TO BUY"
        },
        8.0,
        theme.muted,
        0.9,
        1,
    );
}

fn paint_stock(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    view: &MarketView<'_>,
    interaction: MarketInteraction,
) {
    paint_cut_panel(pixels, 240, 92, 638, 548, INK, 0.83, COPPER_DIM, 8);
    paint_cut_panel(pixels, 251, 103, 616, 54, (19, 8, 34), 0.92, COPPER_DIM, 6);
    paint_diamond(pixels, 273, 130, 9, MAGENTA, 0.9);
    paint_title_text(pixels, 293, 138, "CARDS", 19.0, theme.gold_soft, 1.0, 1);
    paint_ui_text(
        pixels,
        392,
        134,
        "FRESH STOCK  /  BUYING ADDS A COPY TO THIS RUN",
        9.0,
        theme.muted,
        0.95,
        1,
    );
    rect(pixels, WW, WH, 253, 155, 610, 1, COPPER);

    let focus = interaction.hovered_offer.unwrap_or(view.selected_offer);
    let cards = market_offer_rects(view.offers.len());
    for (index, (offer, bounds)) in view.offers.iter().zip(cards).enumerate() {
        paint_offer_card(
            pixels,
            theme,
            art,
            offer,
            bounds,
            index == focus,
            interaction.pressed_offer == Some(index),
        );
    }

    paint_service_tile(
        pixels,
        MarketRect {
            x: 252,
            y: 486,
            w: 190,
            h: 128,
        },
        "RUN DECK",
        &format!("{} CARDS", view.deck_count),
        "Purchased cards persist until run ends.",
        theme,
    );
    paint_service_tile(
        pixels,
        MarketRect {
            x: 451,
            y: 486,
            w: 190,
            h: 128,
        },
        "NEXT BLIND",
        &format!("ROUND {} / {}", view.next_round, view.rounds_total),
        &format!("Target: {} chips", format_counter(view.next_target)),
        theme,
    );
    paint_service_tile(
        pixels,
        MarketRect {
            x: 650,
            y: 486,
            w: 216,
            h: 128,
        },
        "CASH FLOW",
        &format!("${} AVAILABLE", format_counter(view.money)),
        if view.in_run {
            "Spend now or carry cash forward."
        } else {
            "Browse only. Begin a run to trade."
        },
        theme,
    );
    paint_ui_text(
        pixels,
        255,
        631,
        "ARROWS SELECT  /  ENTER BUY  /  R REROLL",
        8.0,
        theme.muted,
        0.88,
        1,
    );
}

fn paint_offer_card(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    offer: &MarketOfferView<'_>,
    bounds: MarketRect,
    focused: bool,
    pressed: bool,
) {
    let lift = if focused && !pressed { 5 } else { 0 };
    let bounds = MarketRect {
        y: bounds.y - lift + i32::from(pressed) * 2,
        ..bounds
    };
    if focused {
        outline(
            pixels,
            WW,
            WH,
            bounds.x - 3,
            bounds.y - 3,
            bounds.w + 6,
            bounds.h + 6,
            (92, 28, 111),
            3,
        );
    }
    paint_cut_panel(
        pixels,
        bounds.x,
        bounds.y,
        bounds.w,
        bounds.h,
        if focused { (28, 9, 42) } else { (9, 6, 19) },
        if pressed { 0.98 } else { 0.94 },
        if focused { COPPER_BRIGHT } else { COPPER_DIM },
        6,
    );

    paint_ui_text(
        pixels,
        bounds.x + 9,
        bounds.y + 22,
        offer.kind,
        8.0,
        if focused { MAGENTA } else { VIOLET },
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        bounds.x + bounds.w - 23,
        bounds.y + 22,
        &format!("{}", offer.price),
        9.0,
        theme.gold_soft,
        1.0,
        1,
    );
    if let Some(image) = art.images.get(offer.id) {
        blit_card(
            pixels,
            WW,
            WH,
            image,
            bounds.x + 8,
            bounds.y + 31,
            bounds.w - 16,
            150,
            if offer.bought { 0.34 } else { 0.96 },
        );
    }
    outline(
        pixels,
        WW,
        WH,
        bounds.x + 7,
        bounds.y + 30,
        bounds.w - 14,
        152,
        if focused { COPPER } else { COPPER_DIM },
        1,
    );
    rect(
        pixels,
        WW,
        WH,
        bounds.x + 8,
        bounds.y + 181,
        bounds.w - 16,
        1,
        COPPER_DIM,
    );
    paint_centered_text(
        pixels,
        MarketRect {
            x: bounds.x + 4,
            y: bounds.y + 183,
            w: bounds.w - 8,
            h: 33,
        },
        &truncate_chars(&offer.name.to_ascii_uppercase(), 14),
        12.0,
        theme.gold_soft,
        1.0,
        true,
    );
    let lines = split_rule_lines(offer.rules, 17);
    for (index, line) in lines.iter().take(2).enumerate() {
        paint_centered_text(
            pixels,
            MarketRect {
                x: bounds.x + 5,
                y: bounds.y + 218 + index as i32 * 17,
                w: bounds.w - 10,
                h: 17,
            },
            line,
            8.5,
            theme.text,
            0.92,
            false,
        );
    }
    rect(
        pixels,
        WW,
        WH,
        bounds.x + 8,
        bounds.y + bounds.h - 42,
        bounds.w - 16,
        1,
        COPPER_DIM,
    );
    paint_chip(pixels, bounds.x + 32, bounds.y + bounds.h - 22, 7);
    paint_title_text(
        pixels,
        bounds.x + 46,
        bounds.y + bounds.h - 16,
        &format!("${}", offer.price),
        16.0,
        if offer.affordable || offer.bought {
            theme.gold_soft
        } else {
            (236, 112, 126)
        },
        1.0,
        1,
    );
    if offer.bought {
        panel(
            pixels,
            WW,
            WH,
            bounds.x + 7,
            bounds.y + 30,
            bounds.w - 14,
            152,
            INK,
            0.55,
        );
        paint_centered_text(
            pixels,
            MarketRect {
                x: bounds.x + 7,
                y: bounds.y + 82,
                w: bounds.w - 14,
                h: 46,
            },
            "SOLD",
            23.0,
            (244, 116, 196),
            1.0,
            true,
        );
    }
}

fn paint_service_tile(
    pixels: &mut [u32],
    bounds: MarketRect,
    label: &str,
    value: &str,
    detail: &str,
    theme: &Theme,
) {
    paint_cut_panel(
        pixels,
        bounds.x,
        bounds.y,
        bounds.w,
        bounds.h,
        (11, 7, 23),
        0.91,
        COPPER_DIM,
        6,
    );
    paint_diamond(pixels, bounds.x + 18, bounds.y + 19, 6, VIOLET, 0.9);
    paint_ui_text(
        pixels,
        bounds.x + 31,
        bounds.y + 23,
        label,
        8.5,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    paint_title_text(
        pixels,
        bounds.x + 15,
        bounds.y + 58,
        value,
        17.0,
        theme.gold_soft,
        1.0,
        1,
    );
    rect(
        pixels,
        WW,
        WH,
        bounds.x + 15,
        bounds.y + 69,
        bounds.w - 30,
        1,
        COPPER_DIM,
    );
    for (index, line) in wrap_words(detail, 28).iter().take(2).enumerate() {
        paint_ui_text(
            pixels,
            bounds.x + 15,
            bounds.y + 91 + index as i32 * 15,
            line,
            8.5,
            theme.muted,
            0.95,
            1,
        );
    }
}

fn paint_continue(
    pixels: &mut [u32],
    theme: &Theme,
    view: &MarketView<'_>,
    interaction: MarketInteraction,
) {
    if !view.in_run {
        paint_cut_panel(pixels, 982, 620, 280, 76, INK, 0.78, COPPER_DIM, 8);
        paint_centered_text(
            pixels,
            MarketRect {
                x: 982,
                y: 620,
                w: 280,
                h: 45,
            },
            "BROWSE MODE",
            18.0,
            theme.gold_soft,
            1.0,
            true,
        );
        paint_centered_text(
            pixels,
            MarketRect {
                x: 982,
                y: 658,
                w: 280,
                h: 24,
            },
            "X / ESC  BACK TO LOBBY",
            8.5,
            theme.muted,
            1.0,
            false,
        );
        return;
    }
    let bounds = market_action_rect(MarketAction::Continue);
    let hovered = interaction.hovered_action == Some(MarketAction::Continue);
    let pressed = interaction.pressed_action == Some(MarketAction::Continue);
    paint_action_plate(
        pixels,
        bounds,
        hovered,
        pressed,
        true,
        "CONTINUE RUN",
        23.0,
        true,
    );
    paint_ui_text(pixels, 1060, 692, "C / SPACE", 8.5, theme.text, 0.92, 1);
}

#[allow(clippy::too_many_arguments)]
fn paint_action_plate(
    pixels: &mut [u32],
    bounds: MarketRect,
    hovered: bool,
    pressed: bool,
    enabled: bool,
    label: &str,
    size: f32,
    title: bool,
) {
    let fill_rgb = if !enabled {
        (17, 12, 25)
    } else if pressed {
        (71, 19, 82)
    } else if hovered {
        (80, 22, 101)
    } else {
        (43, 13, 59)
    };
    let border = if enabled && (hovered || pressed) {
        COPPER_BRIGHT
    } else if enabled {
        (145, 67, 132)
    } else {
        COPPER_DIM
    };
    if hovered && enabled {
        outline(
            pixels,
            WW,
            WH,
            bounds.x - 3,
            bounds.y - 3,
            bounds.w + 6,
            bounds.h + 6,
            (87, 27, 111),
            3,
        );
    }
    paint_cut_panel(
        pixels, bounds.x, bounds.y, bounds.w, bounds.h, fill_rgb, 0.96, border, 7,
    );
    paint_centered_text(
        pixels,
        bounds,
        label,
        size,
        if enabled {
            (232, 196, 159)
        } else {
            (112, 98, 119)
        },
        1.0,
        title,
    );
}

fn paint_metric(pixels: &mut [u32], x: i32, label: &str, value: &str, value_y: i32) {
    paint_ui_text(pixels, x, 31, label, 8.5, COPPER_BRIGHT, 0.85, 1);
    paint_title_text(pixels, x, value_y, value, 18.0, (235, 219, 224), 1.0, 1);
}

fn paint_hud_divider(pixels: &mut [u32], x: i32) {
    panel(pixels, WW, WH, x, 21, 1, 45, COPPER_DIM, 0.78);
}

fn paint_chip(pixels: &mut [u32], cx: i32, cy: i32, radius: i32) {
    paint_circle(pixels, cx, cy, radius, (30, 10, 43), 1.0);
    paint_circle_outline(pixels, cx, cy, radius, COPPER_BRIGHT, 2);
    paint_circle_outline(pixels, cx, cy, (radius - 5).max(2), MAGENTA, 2);
    for angle in (0..360).step_by(60) {
        let radians = angle as f32 * std::f32::consts::PI / 180.0;
        let x = cx + ((radius - 2) as f32 * radians.cos()).round() as i32;
        let y = cy + ((radius - 2) as f32 * radians.sin()).round() as i32;
        paint_circle(pixels, x, y, 2, (239, 197, 155), 0.9);
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
            x + w - 1 - step,
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
        5,
        5,
        WW as i32 - 10,
        WH as i32 - 10,
        (72, 39, 49),
        1,
    );
    rect(pixels, WW, WH, 13, 13, 120, 1, COPPER_DIM);
    rect(pixels, WW, WH, 1147, 13, 120, 1, COPPER_DIM);
    paint_diamond(pixels, 137, 13, 5, COPPER_BRIGHT, 0.8);
    paint_diamond(pixels, 1143, 13, 5, COPPER_BRIGHT, 0.8);
}

fn paint_fallback_market(pixels: &mut [u32]) {
    for index in 0..12 {
        let x = 760 + index * 43;
        let height = 90 + (index * 37 % 170);
        panel(
            pixels,
            WW,
            WH,
            x,
            500 - height,
            24,
            height,
            (54, 14, 78),
            0.55,
        );
        rect(pixels, WW, WH, x + 5, 515 - height, 3, 5, (215, 43, 171));
    }
    panel(pixels, WW, WH, 0, 480, WW as i32, 240, (8, 4, 16), 0.86);
}

fn paint_diamond(pixels: &mut [u32], cx: i32, cy: i32, size: i32, rgb: (u8, u8, u8), opacity: f32) {
    for y in -size..=size {
        let width = size - y.abs();
        for x in -width..=width {
            set_pixel(pixels, cx + x, cy + y, rgb, opacity);
        }
    }
}

fn paint_circle(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    radius: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let rr = radius * radius;
    for y in -radius..=radius {
        for x in -radius..=radius {
            if x * x + y * y <= rr {
                set_pixel(pixels, cx + x, cy + y, rgb, opacity);
            }
        }
    }
}

fn paint_circle_outline(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    radius: i32,
    rgb: (u8, u8, u8),
    thickness: i32,
) {
    let outer = radius * radius;
    let inner = (radius - thickness.max(1)).max(0).pow(2);
    for y in -radius..=radius {
        for x in -radius..=radius {
            let value = x * x + y * y;
            if value <= outer && value >= inner {
                set_pixel(pixels, cx + x, cy + y, rgb, 1.0);
            }
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
    bounds: MarketRect,
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

fn split_rule_lines(value: &str, max_chars: usize) -> Vec<String> {
    let normalized = value.replace(" · ", "|").replace(" Â· ", "|");
    let mut lines = Vec::new();
    for part in normalized.split('|') {
        lines.extend(wrap_words(part, max_chars));
    }
    lines
}

fn wrap_words(value: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let next_len =
            current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if next_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
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
    fn market_targets_are_large_and_do_not_overlap() {
        let offers = market_offer_rects(5);
        assert_eq!(offers.len(), 5);
        assert!(offers.iter().all(|bounds| bounds.w >= 48 && bounds.h >= 48));
        for pair in offers.windows(2) {
            assert!(pair[1].x - (pair[0].x + pair[0].w) >= 8);
        }
        for action in [
            MarketAction::Reroll,
            MarketAction::Continue,
            MarketAction::Back,
        ] {
            let bounds = market_action_rect(action);
            assert!(bounds.w >= 48 && bounds.h >= 48);
        }
    }

    #[test]
    fn hit_testing_respects_run_only_actions() {
        let continue_bounds = market_action_rect(MarketAction::Continue);
        assert_eq!(
            hit_test_market(5, true, continue_bounds.x + 10, continue_bounds.y + 10),
            Some(MarketHit::Continue)
        );
        assert_eq!(
            hit_test_market(5, false, continue_bounds.x + 10, continue_bounds.y + 10),
            None
        );
        let first = market_offer_rects(5)[0];
        assert_eq!(
            hit_test_market(5, true, first.x + 10, first.y + 10),
            Some(MarketHit::Offer(0))
        );
    }

    #[test]
    #[ignore = "manual visual evidence; run explicitly with --ignored"]
    fn dump_market_png_for_evidence() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let background = load_rgb(&data.join("ui/night_market_bg.png"));
        let art = ArtBank::from_catalog_dir(
            &data.join("art"),
            &["strike", "guard", "fireball", "focus", "bash"],
        );
        let offers = [
            MarketOfferView {
                id: "strike",
                name: "Metropolis",
                kind: "ATK",
                rules: "+18 chips",
                price: 2,
                bought: false,
                affordable: true,
            },
            MarketOfferView {
                id: "guard",
                name: "Velvet Guard",
                kind: "DEF",
                rules: "+12 chips",
                price: 2,
                bought: true,
                affordable: true,
            },
            MarketOfferView {
                id: "fireball",
                name: "Neon Flare",
                kind: "SPL",
                rules: "+35 chips · +1 mult",
                price: 4,
                bought: false,
                affordable: true,
            },
            MarketOfferView {
                id: "focus",
                name: "Focus",
                kind: "SKL",
                rules: "+8 chips · draw 1",
                price: 2,
                bought: false,
                affordable: true,
            },
            MarketOfferView {
                id: "bash",
                name: "Midnight Bash",
                kind: "ATK",
                rules: "+28 chips",
                price: 5,
                bought: false,
                affordable: false,
            },
        ];
        let view = MarketView {
            vault: 12_450,
            money: 4,
            score: 685,
            next_target: 700,
            next_round: 2,
            rounds_total: 3,
            deck_count: 21,
            reroll_cost: 1,
            in_run: true,
            selected_offer: 0,
            status: "Blind cleared. Improve the deck, then continue.",
            offers: &offers,
        };
        let mut pixels = vec![0; (WW * WH) as usize];
        paint_market(
            &mut pixels,
            &Theme::default(),
            background.as_ref(),
            &art,
            &view,
            MarketInteraction {
                hovered_offer: Some(2),
                ..MarketInteraction::default()
            },
        );

        let output = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/night_market_ui_paint.png");
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
        assert!(std::fs::metadata(output).unwrap().len() > 40_000);
    }
}
