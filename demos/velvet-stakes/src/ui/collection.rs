//! Premium cyber-noir card collection, deck editor, and pointer geometry.
//!
//! The painter is intentionally read-only. The game host owns the persistent
//! starter deck and applies add/remove actions selected by this interface.

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
const CARD_X: i32 = 252;
const CARD_Y: i32 = 194;
const CARD_W: i32 = 116;
const CARD_H: i32 = 262;
const CARD_GAP: i32 = 9;

/// Axis-aligned rectangle in the fixed 1280 x 720 collection canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl CollectionRect {
    /// Whether a logical point lies inside the rectangle.
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Catalog category shown in the filter bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CollectionFilter {
    /// Show every illustrated card.
    #[default]
    All,
    /// Show attack cards.
    Attack,
    /// Show defense cards.
    Defense,
    /// Show spell cards.
    Spell,
    /// Show skill cards.
    Skill,
}

impl CollectionFilter {
    /// Stable filter order used by the host and keyboard navigation.
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Attack,
        Self::Defense,
        Self::Spell,
        Self::Skill,
    ];

    /// Convert a stored zero-based index to a filter.
    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::All)
    }

    /// Stable zero-based index.
    pub fn index(self) -> usize {
        match self {
            Self::All => 0,
            Self::Attack => 1,
            Self::Defense => 2,
            Self::Spell => 3,
            Self::Skill => 4,
        }
    }

    /// Compact UI label.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "ALL CARDS",
            Self::Attack => "ATTACK",
            Self::Defense => "DEFENSE",
            Self::Spell => "SPELLS",
            Self::Skill => "SKILLS",
        }
    }
}

/// Deck-editing action shown in the collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionAction {
    /// Add one copy of the focused card.
    Add,
    /// Remove one copy of the focused card.
    Remove,
    /// Return to the lobby.
    Back,
}

/// Pointer target returned by collection hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionHit {
    /// Visible card index.
    Card(usize),
    /// Filter tab.
    Filter(CollectionFilter),
    /// Add-card action.
    Add,
    /// Remove-card action.
    Remove,
    /// Return action.
    Back,
}

/// Hover and pressed state consumed by the painter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollectionInteraction {
    /// Card under the pointer.
    pub hovered_card: Option<usize>,
    /// Card currently held down.
    pub pressed_card: Option<usize>,
    /// Filter under the pointer.
    pub hovered_filter: Option<CollectionFilter>,
    /// Filter currently held down.
    pub pressed_filter: Option<CollectionFilter>,
    /// Action under the pointer.
    pub hovered_action: Option<CollectionAction>,
    /// Action currently held down.
    pub pressed_action: Option<CollectionAction>,
}

/// Read-only presentation data for one real catalog card.
#[derive(Debug, Clone, Copy)]
pub struct CollectionCardView<'a> {
    /// Catalog id used to resolve illustration art.
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Compact card kind label.
    pub kind: &'a str,
    /// Gameplay rules summary.
    pub rules: &'a str,
    /// Copies currently in the starter deck.
    pub owned: usize,
}

/// Complete read-only collection frame.
#[derive(Debug, Clone, Copy)]
pub struct CollectionView<'a> {
    /// Persistent lobby chips.
    pub vault: i64,
    /// Persistent crystals.
    pub crystals: i64,
    /// Persistent profile multiplier.
    pub multiplier: f32,
    /// Number of cards in the editable starter deck.
    pub deck_count: usize,
    /// Maximum starter deck size.
    pub deck_limit: usize,
    /// Minimum safe starter deck size.
    pub min_deck: usize,
    /// Aggregate base chips in the starter deck.
    pub total_chips: i64,
    /// Aggregate extra multiplier in the starter deck.
    pub extra_mult: i64,
    /// Counts for attack, defense, spell, and skill cards.
    pub kind_counts: [usize; 4],
    /// Active filter tab.
    pub filter: CollectionFilter,
    /// Keyboard-selected visible card.
    pub selected_card: usize,
    /// Short actionable host status.
    pub status: &'a str,
    /// Cards visible under the active filter.
    pub cards: &'a [CollectionCardView<'a>],
    /// All catalog cards, used for the deck composition strip.
    pub composition: &'a [CollectionCardView<'a>],
}

/// Rectangles for up to five visible cards.
pub fn collection_card_rects(count: usize) -> Vec<CollectionRect> {
    (0..count.min(5))
        .map(|index| CollectionRect {
            x: CARD_X + index as i32 * (CARD_W + CARD_GAP),
            y: CARD_Y,
            w: CARD_W,
            h: CARD_H,
        })
        .collect()
}

/// Fixed rectangle for one filter tab.
pub fn collection_filter_rect(filter: CollectionFilter) -> CollectionRect {
    CollectionRect {
        x: 252 + filter.index() as i32 * 125,
        y: 137,
        w: 116,
        h: 44,
    }
}

/// Fixed rectangle for one collection action.
pub fn collection_action_rect(action: CollectionAction) -> CollectionRect {
    match action {
        CollectionAction::Add => CollectionRect {
            x: 949,
            y: 574,
            w: 292,
            h: 54,
        },
        CollectionAction::Remove => CollectionRect {
            x: 949,
            y: 636,
            w: 292,
            h: 48,
        },
        CollectionAction::Back => CollectionRect {
            x: 28,
            y: 636,
            w: 178,
            h: 50,
        },
    }
}

fn top_back_rect() -> CollectionRect {
    CollectionRect {
        x: 1208,
        y: 20,
        w: 48,
        h: 48,
    }
}

/// Resolve the collection target at logical coordinates.
pub fn hit_test_collection(card_count: usize, x: i32, y: i32) -> Option<CollectionHit> {
    if collection_action_rect(CollectionAction::Add).contains(x, y) {
        return Some(CollectionHit::Add);
    }
    if collection_action_rect(CollectionAction::Remove).contains(x, y) {
        return Some(CollectionHit::Remove);
    }
    if collection_action_rect(CollectionAction::Back).contains(x, y)
        || top_back_rect().contains(x, y)
    {
        return Some(CollectionHit::Back);
    }
    for filter in CollectionFilter::ALL {
        if collection_filter_rect(filter).contains(x, y) {
            return Some(CollectionHit::Filter(filter));
        }
    }
    collection_card_rects(card_count)
        .iter()
        .enumerate()
        .find_map(|(index, bounds)| bounds.contains(x, y).then_some(CollectionHit::Card(index)))
}

/// Paint the complete collection and starter-deck editor.
pub fn paint_collection_screen(
    pixels: &mut [u32],
    theme: &Theme,
    background: Option<&RgbImage>,
    art: &ArtBank,
    view: &CollectionView<'_>,
    interaction: CollectionInteraction,
) {
    if let Some(background) = background {
        blit_cover(pixels, WW, WH, background);
        panel(pixels, WW, WH, 0, 0, WW as i32, WH as i32, INK, 0.2);
    } else {
        fill(pixels, WW, WH, INK);
        paint_fallback_city(pixels);
    }

    paint_top_hud(pixels, theme, view, interaction);
    paint_deck_rail(pixels, theme, view, interaction);
    paint_library(pixels, theme, art, view, interaction);
    paint_detail(pixels, theme, art, view, interaction);
    paint_outer_frame(pixels);
}

fn paint_top_hud(
    pixels: &mut [u32],
    theme: &Theme,
    view: &CollectionView<'_>,
    interaction: CollectionInteraction,
) {
    paint_cut_panel(pixels, 10, 8, 1260, 72, INK, 0.93, COPPER_DIM, 8);
    rect(pixels, WW, WH, 12, 79, 1256, 1, COPPER);
    paint_title_text(pixels, 28, 44, "VELVET ARCANA", 26.0, theme.gold, 1.0, 2);
    paint_ui_text(
        pixels,
        30,
        64,
        "NIGHTFALL CASINO  /  CARD ARCHIVE",
        8.2,
        COPPER_BRIGHT,
        0.88,
        1,
    );
    paint_hud_divider(pixels, 302);
    paint_chip(pixels, 334, 44, 16);
    paint_metric(pixels, 360, "CHIPS", &format_counter(view.vault), 54);
    paint_hud_divider(pixels, 467);
    paint_diamond(pixels, 493, 44, 15, VIOLET, 0.9);
    paint_metric(pixels, 518, "CRYSTALS", &format_counter(view.crystals), 54);
    paint_hud_divider(pixels, 629);
    paint_metric(
        pixels,
        654,
        "MULTIPLIER",
        &format!("x{:.1}", view.multiplier),
        54,
    );
    paint_hud_divider(pixels, 778);

    paint_ui_text(pixels, 815, 48, "PLAY", 10.0, theme.muted, 0.8, 1);
    paint_cut_panel(pixels, 867, 20, 154, 45, (34, 10, 48), 0.96, COPPER_DIM, 7);
    paint_centered_text(
        pixels,
        CollectionRect {
            x: 867,
            y: 20,
            w: 154,
            h: 45,
        },
        "COLLECTION",
        12.0,
        theme.gold_soft,
        1.0,
        true,
    );
    rect(pixels, WW, WH, 913, 63, 62, 2, MAGENTA);
    paint_ui_text(pixels, 1045, 48, "MARKET", 10.0, theme.muted, 0.8, 1);
    paint_ui_text(
        pixels,
        1116,
        48,
        &format!("{} / {}", view.deck_count, view.deck_limit),
        10.0,
        COPPER_BRIGHT,
        0.94,
        1,
    );

    let back = top_back_rect();
    let hovered = interaction.hovered_action == Some(CollectionAction::Back);
    let pressed = interaction.pressed_action == Some(CollectionAction::Back);
    paint_action_plate(pixels, back, hovered, pressed, true, "X", 16.0, false);
}

fn paint_deck_rail(
    pixels: &mut [u32],
    theme: &Theme,
    view: &CollectionView<'_>,
    interaction: CollectionInteraction,
) {
    paint_cut_panel(pixels, 12, 90, 222, 610, INK, 0.93, COPPER_DIM, 8);
    paint_title_text(pixels, 29, 126, "MY DECK", 22.0, theme.gold, 1.0, 2);
    paint_ui_text(
        pixels,
        171,
        124,
        &format!("{} / {}", view.deck_count, view.deck_limit),
        9.0,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    paint_cut_panel(
        pixels,
        25,
        140,
        196,
        103,
        (21, 8, 34),
        0.96,
        COPPER_BRIGHT,
        6,
    );
    paint_diamond(pixels, 51, 173, 17, VIOLET, 0.96);
    paint_diamond(pixels, 51, 173, 7, MAGENTA, 0.92);
    paint_title_text(
        pixels,
        80,
        171,
        "NIGHT SYNDICATE",
        15.0,
        theme.gold_soft,
        1.0,
        1,
    );
    paint_ui_text(pixels, 80, 191, "ACTIVE STARTER DECK", 8.0, VIOLET, 1.0, 1);
    paint_deck_meter(
        pixels,
        80,
        207,
        123,
        view.deck_count,
        view.deck_limit,
        theme,
    );
    paint_ui_text(
        pixels,
        28,
        267,
        "DECK STATISTICS",
        8.5,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    rect(pixels, WW, WH, 27, 277, 190, 1, COPPER_DIM);

    paint_stat_row(
        pixels,
        29,
        300,
        "TOTAL CARDS",
        &view.deck_count.to_string(),
        theme,
    );
    paint_stat_row(
        pixels,
        29,
        329,
        "BASE CHIPS",
        &format!("+{}", format_counter(view.total_chips)),
        theme,
    );
    paint_stat_row(
        pixels,
        29,
        358,
        "EXTRA MULT",
        &format!("+{}", view.extra_mult),
        theme,
    );
    paint_stat_row(
        pixels,
        29,
        387,
        "SAFE RANGE",
        &format!("{}-{}", view.min_deck, view.deck_limit),
        theme,
    );

    paint_ui_text(
        pixels,
        28,
        423,
        "TYPE BREAKDOWN",
        8.5,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    rect(pixels, WW, WH, 27, 433, 190, 1, COPPER_DIM);
    let labels = ["ATTACK", "DEFENSE", "SPELL", "SKILL"];
    let colors = [(232, 82, 128), (75, 182, 225), VIOLET, (226, 151, 91)];
    for index in 0..4 {
        let y = 455 + index as i32 * 30;
        paint_diamond(pixels, 38, y - 4, 5, colors[index], 0.95);
        paint_ui_text(pixels, 51, y, labels[index], 8.5, theme.text, 0.92, 1);
        paint_ui_text(
            pixels,
            191,
            y,
            &view.kind_counts[index].to_string(),
            10.0,
            colors[index],
            1.0,
            1,
        );
    }

    paint_cut_panel(pixels, 25, 570, 196, 52, (12, 8, 26), 0.94, COPPER_DIM, 6);
    paint_ui_text(
        pixels,
        37,
        589,
        "ARCHIVE STATUS",
        7.5,
        COPPER_BRIGHT,
        0.95,
        1,
    );
    paint_ui_text(
        pixels,
        37,
        608,
        if view.deck_count >= view.min_deck {
            "READY FOR THE TABLE"
        } else {
            "MORE CARDS REQUIRED"
        },
        8.2,
        if view.deck_count >= view.min_deck {
            VIOLET
        } else {
            MAGENTA
        },
        1.0,
        1,
    );

    let bounds = collection_action_rect(CollectionAction::Back);
    let hovered = interaction.hovered_action == Some(CollectionAction::Back);
    let pressed = interaction.pressed_action == Some(CollectionAction::Back);
    paint_action_plate(
        pixels,
        bounds,
        hovered,
        pressed,
        true,
        "<  BACK TO LOBBY",
        14.0,
        true,
    );
}

fn paint_library(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    view: &CollectionView<'_>,
    interaction: CollectionInteraction,
) {
    paint_cut_panel(pixels, 242, 90, 670, 610, INK, 0.88, COPPER_DIM, 8);
    paint_title_text(pixels, 260, 124, "COLLECTION", 24.0, theme.gold, 1.0, 2);
    paint_ui_text(
        pixels,
        735,
        121,
        "5 UNIQUE CARDS",
        8.5,
        theme.muted,
        0.92,
        1,
    );

    for filter in CollectionFilter::ALL {
        let bounds = collection_filter_rect(filter);
        let active = view.filter == filter;
        let hovered = interaction.hovered_filter == Some(filter);
        let pressed = interaction.pressed_filter == Some(filter);
        paint_filter_tab(
            pixels,
            bounds,
            filter.label(),
            active,
            hovered,
            pressed,
            theme,
        );
    }

    let focus = interaction
        .hovered_card
        .unwrap_or(view.selected_card)
        .min(view.cards.len().saturating_sub(1));
    if view.cards.is_empty() {
        paint_centered_text(
            pixels,
            CollectionRect {
                x: 252,
                y: 230,
                w: 616,
                h: 180,
            },
            "NO CARDS IN THIS CATEGORY",
            18.0,
            theme.muted,
            1.0,
            true,
        );
    } else {
        for (index, (card, bounds)) in view
            .cards
            .iter()
            .zip(collection_card_rects(view.cards.len()))
            .enumerate()
        {
            paint_library_card(
                pixels,
                theme,
                art,
                card,
                bounds,
                index == focus,
                interaction.pressed_card == Some(index),
            );
        }
    }

    rect(pixels, WW, WH, 257, 470, 640, 1, COPPER_DIM);
    paint_ui_text(
        pixels,
        260,
        491,
        "DECK COMPOSITION",
        8.5,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        723,
        491,
        "COPIES IN ACTIVE DECK",
        7.8,
        theme.muted,
        0.9,
        1,
    );
    for (index, card) in view.composition.iter().take(5).enumerate() {
        paint_composition_tile(
            pixels,
            theme,
            card,
            CollectionRect {
                x: 253 + index as i32 * 127,
                y: 503,
                w: 118,
                h: 105,
            },
        );
    }

    paint_cut_panel(pixels, 253, 620, 646, 58, (10, 7, 22), 0.94, COPPER_DIM, 6);
    paint_ui_text(
        pixels,
        267,
        639,
        "ARCHIVE MESSAGE",
        7.5,
        COPPER_BRIGHT,
        0.95,
        1,
    );
    for (index, line) in wrap_words(view.status, 76).iter().take(2).enumerate() {
        paint_ui_text(
            pixels,
            267,
            659 + index as i32 * 14,
            line,
            8.5,
            theme.text,
            0.95,
            1,
        );
    }
}

fn paint_detail(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    view: &CollectionView<'_>,
    interaction: CollectionInteraction,
) {
    paint_cut_panel(pixels, 920, 90, 348, 610, INK, 0.83, COPPER_DIM, 8);
    let focus = interaction
        .hovered_card
        .unwrap_or(view.selected_card)
        .min(view.cards.len().saturating_sub(1));
    let Some(card) = view.cards.get(focus) else {
        paint_centered_text(
            pixels,
            CollectionRect {
                x: 940,
                y: 270,
                w: 308,
                h: 80,
            },
            "SELECT A CATEGORY",
            18.0,
            theme.gold_soft,
            1.0,
            true,
        );
        return;
    };

    paint_ui_text(
        pixels,
        949,
        118,
        "SELECTED CARD",
        8.0,
        COPPER_BRIGHT,
        0.95,
        1,
    );
    paint_title_text(
        pixels,
        949,
        143,
        &truncate_chars(&card.name.to_ascii_uppercase(), 22),
        20.0,
        theme.gold_soft,
        1.0,
        1,
    );
    paint_ui_text(pixels, 1179, 137, card.kind, 8.0, VIOLET, 1.0, 1);
    paint_cut_panel(
        pixels,
        974,
        157,
        240,
        304,
        (10, 6, 21),
        0.97,
        COPPER_BRIGHT,
        8,
    );
    if let Some(image) = art.images.get(card.id) {
        blit_card(pixels, WW, WH, image, 985, 169, 218, 214, 0.99);
    }
    outline(pixels, WW, WH, 983, 167, 222, 218, COPPER, 1);
    paint_centered_text(
        pixels,
        CollectionRect {
            x: 982,
            y: 389,
            w: 224,
            h: 31,
        },
        &card.name.to_ascii_uppercase(),
        14.0,
        theme.gold_soft,
        1.0,
        true,
    );
    paint_centered_text(
        pixels,
        CollectionRect {
            x: 982,
            y: 419,
            w: 224,
            h: 22,
        },
        card.rules,
        9.0,
        theme.text,
        0.95,
        false,
    );
    for index in 0..card.owned.min(6) {
        paint_diamond(pixels, 1055 + index as i32 * 17, 450, 5, MAGENTA, 0.95);
    }

    let lore = card_lore(card.id);
    for (index, line) in wrap_words(lore, 43).iter().take(3).enumerate() {
        paint_centered_text(
            pixels,
            CollectionRect {
                x: 944,
                y: 482 + index as i32 * 17,
                w: 300,
                h: 18,
            },
            line,
            9.0,
            theme.muted,
            0.96,
            false,
        );
    }
    paint_ui_text(
        pixels,
        949,
        555,
        &format!("{} COPIES IN DECK", card.owned),
        8.5,
        COPPER_BRIGHT,
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        1179,
        555,
        &format!("{}/{}", view.deck_count, view.deck_limit),
        8.5,
        theme.text,
        1.0,
        1,
    );

    let add = collection_action_rect(CollectionAction::Add);
    let can_add = view.deck_count < view.deck_limit;
    paint_action_plate(
        pixels,
        add,
        interaction.hovered_action == Some(CollectionAction::Add),
        interaction.pressed_action == Some(CollectionAction::Add),
        can_add,
        "+  ADD COPY TO DECK",
        16.0,
        true,
    );
    let remove = collection_action_rect(CollectionAction::Remove);
    let can_remove = card.owned > 0 && view.deck_count > view.min_deck;
    paint_action_plate(
        pixels,
        remove,
        interaction.hovered_action == Some(CollectionAction::Remove),
        interaction.pressed_action == Some(CollectionAction::Remove),
        can_remove,
        "-  REMOVE COPY",
        13.0,
        true,
    );
    paint_ui_text(
        pixels,
        1024,
        697,
        "ENTER ADD  /  X REMOVE",
        7.7,
        theme.muted,
        0.9,
        1,
    );
}

fn paint_library_card(
    pixels: &mut [u32],
    theme: &Theme,
    art: &ArtBank,
    card: &CollectionCardView<'_>,
    bounds: CollectionRect,
    focused: bool,
    pressed: bool,
) {
    let lift = if focused && !pressed { 4 } else { 0 };
    let bounds = CollectionRect {
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
        0.96,
        if focused { COPPER_BRIGHT } else { COPPER_DIM },
        6,
    );
    paint_ui_text(
        pixels,
        bounds.x + 9,
        bounds.y + 21,
        card.kind,
        8.0,
        if focused { MAGENTA } else { VIOLET },
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        bounds.x + 82,
        bounds.y + 21,
        &format!("x{}", card.owned),
        8.0,
        theme.gold_soft,
        1.0,
        1,
    );
    if let Some(image) = art.images.get(card.id) {
        blit_card(
            pixels,
            WW,
            WH,
            image,
            bounds.x + 8,
            bounds.y + 30,
            bounds.w - 16,
            148,
            0.98,
        );
    }
    outline(
        pixels,
        WW,
        WH,
        bounds.x + 7,
        bounds.y + 29,
        bounds.w - 14,
        150,
        if focused { COPPER } else { COPPER_DIM },
        1,
    );
    paint_centered_text(
        pixels,
        CollectionRect {
            x: bounds.x + 5,
            y: bounds.y + 184,
            w: bounds.w - 10,
            h: 31,
        },
        &truncate_chars(&card.name.to_ascii_uppercase(), 14),
        12.0,
        theme.gold_soft,
        1.0,
        true,
    );
    let lines = split_rule_lines(card.rules, 17);
    for (index, line) in lines.iter().take(2).enumerate() {
        paint_centered_text(
            pixels,
            CollectionRect {
                x: bounds.x + 5,
                y: bounds.y + 218 + index as i32 * 16,
                w: bounds.w - 10,
                h: 16,
            },
            line,
            8.2,
            theme.text,
            0.92,
            false,
        );
    }
    rect(
        pixels,
        WW,
        WH,
        bounds.x + 9,
        bounds.y + bounds.h - 13,
        bounds.w - 18,
        1,
        COPPER_DIM,
    );
    for index in 0..card.owned.min(5) {
        paint_diamond(
            pixels,
            bounds.x + 37 + index as i32 * 12,
            bounds.y + bounds.h - 7,
            3,
            if focused { MAGENTA } else { VIOLET },
            0.95,
        );
    }
}

fn paint_composition_tile(
    pixels: &mut [u32],
    theme: &Theme,
    card: &CollectionCardView<'_>,
    bounds: CollectionRect,
) {
    paint_cut_panel(
        pixels,
        bounds.x,
        bounds.y,
        bounds.w,
        bounds.h,
        (11, 7, 23),
        0.93,
        COPPER_DIM,
        6,
    );
    paint_ui_text(
        pixels,
        bounds.x + 10,
        bounds.y + 21,
        &truncate_chars(&card.name.to_ascii_uppercase(), 13),
        8.0,
        theme.gold_soft,
        0.98,
        1,
    );
    paint_title_text(
        pixels,
        bounds.x + 10,
        bounds.y + 51,
        &format!("x{}", card.owned),
        20.0,
        if card.owned > 0 {
            COPPER_BRIGHT
        } else {
            theme.muted
        },
        1.0,
        1,
    );
    paint_ui_text(
        pixels,
        bounds.x + 73,
        bounds.y + 47,
        card.kind,
        7.5,
        VIOLET,
        1.0,
        1,
    );
    rect(
        pixels,
        WW,
        WH,
        bounds.x + 10,
        bounds.y + 66,
        bounds.w - 20,
        4,
        (31, 17, 40),
    );
    let fill_w = ((bounds.w - 20) as f32 * (card.owned.min(8) as f32 / 8.0)).round() as i32;
    rect(
        pixels,
        WW,
        WH,
        bounds.x + 10,
        bounds.y + 66,
        fill_w,
        4,
        MAGENTA,
    );
    paint_ui_text(
        pixels,
        bounds.x + 10,
        bounds.y + 91,
        if card.owned == 1 { "1 COPY" } else { "COPIES" },
        7.4,
        theme.muted,
        0.9,
        1,
    );
}

fn paint_filter_tab(
    pixels: &mut [u32],
    bounds: CollectionRect,
    label: &str,
    active: bool,
    hovered: bool,
    pressed: bool,
    theme: &Theme,
) {
    let fill_rgb = if pressed {
        (70, 18, 82)
    } else if active || hovered {
        (42, 12, 55)
    } else {
        (10, 7, 20)
    };
    paint_cut_panel(
        pixels,
        bounds.x,
        bounds.y,
        bounds.w,
        bounds.h,
        fill_rgb,
        0.96,
        if active || hovered {
            COPPER_BRIGHT
        } else {
            COPPER_DIM
        },
        6,
    );
    paint_centered_text(
        pixels,
        bounds,
        label,
        9.0,
        if active { theme.gold_soft } else { theme.muted },
        1.0,
        true,
    );
    if active {
        rect(
            pixels,
            WW,
            WH,
            bounds.x + 31,
            bounds.y + bounds.h - 2,
            bounds.w - 62,
            2,
            MAGENTA,
        );
    }
}

fn paint_deck_meter(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    width: i32,
    count: usize,
    limit: usize,
    theme: &Theme,
) {
    rect(pixels, WW, WH, x, y, width, 6, (34, 18, 43));
    let filled = ((width as f32 * count as f32 / limit.max(1) as f32).round() as i32).min(width);
    rect(pixels, WW, WH, x, y, filled, 6, MAGENTA);
    paint_ui_text(
        pixels,
        x,
        y + 20,
        &format!("{} / {} CARDS", count, limit),
        7.5,
        theme.muted,
        0.95,
        1,
    );
}

fn paint_stat_row(pixels: &mut [u32], x: i32, y: i32, label: &str, value: &str, theme: &Theme) {
    paint_ui_text(pixels, x, y, label, 8.5, theme.muted, 0.93, 1);
    let width = ui_font()
        .map(|font| measure_text(font, value, 10.0))
        .unwrap_or(value.len() as f32 * 5.8);
    paint_ui_text(
        pixels,
        211 - width.round() as i32,
        y,
        value,
        10.0,
        theme.gold_soft,
        1.0,
        1,
    );
    rect(pixels, WW, WH, x, y + 9, 184, 1, (45, 28, 48));
}

#[allow(clippy::too_many_arguments)]
fn paint_action_plate(
    pixels: &mut [u32],
    bounds: CollectionRect,
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
        pixels, bounds.x, bounds.y, bounds.w, bounds.h, fill_rgb, 0.97, border, 7,
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
    paint_ui_text(pixels, x, 31, label, 8.0, COPPER_BRIGHT, 0.85, 1);
    paint_title_text(pixels, x, value_y, value, 17.0, (235, 219, 224), 1.0, 1);
}

fn paint_hud_divider(pixels: &mut [u32], x: i32) {
    panel(pixels, WW, WH, x, 20, 1, 46, COPPER_DIM, 0.78);
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

fn paint_fallback_city(pixels: &mut [u32]) {
    for index in 0..12 {
        let x = 785 + index * 38;
        let height = 80 + (index * 31 % 190);
        panel(
            pixels,
            WW,
            WH,
            x,
            520 - height,
            21,
            height,
            (54, 14, 78),
            0.52,
        );
        rect(pixels, WW, WH, x + 4, 534 - height, 3, 5, (215, 43, 171));
    }
    panel(pixels, WW, WH, 0, 486, WW as i32, 234, (8, 4, 16), 0.84);
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
    bounds: CollectionRect,
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
        .chain(['…'])
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

fn card_lore(id: &str) -> &'static str {
    match id {
        "strike" => "A neon opening that turns pressure into clean chips.",
        "guard" => "A patient defense drawn from the quiet side of the table.",
        "fireball" => "Volatile arcana. Expensive, luminous, and built to multiply.",
        "focus" => "A measured breath that brings the next possibility into view.",
        "bash" => "Heavy force from the Night Syndicate's private collection.",
        _ => "An archived card from the tables beneath Velvet Arcana.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::load_rgb;
    use std::path::PathBuf;

    #[test]
    fn collection_targets_are_accessible_and_separate() {
        let cards = collection_card_rects(5);
        assert_eq!(cards.len(), 5);
        assert!(cards.iter().all(|bounds| bounds.w >= 48 && bounds.h >= 48));
        for pair in cards.windows(2) {
            assert!(pair[1].x - (pair[0].x + pair[0].w) >= 8);
        }
        for filter in CollectionFilter::ALL {
            let bounds = collection_filter_rect(filter);
            assert!(bounds.w >= 48 && bounds.h >= 44);
        }
        for action in [
            CollectionAction::Add,
            CollectionAction::Remove,
            CollectionAction::Back,
        ] {
            let bounds = collection_action_rect(action);
            assert!(bounds.w >= 48 && bounds.h >= 48);
        }
    }

    #[test]
    fn hit_testing_resolves_cards_filters_and_actions() {
        let first = collection_card_rects(5)[0];
        assert_eq!(
            hit_test_collection(5, first.x + 10, first.y + 10),
            Some(CollectionHit::Card(0))
        );
        let spell = collection_filter_rect(CollectionFilter::Spell);
        assert_eq!(
            hit_test_collection(5, spell.x + 10, spell.y + 10),
            Some(CollectionHit::Filter(CollectionFilter::Spell))
        );
        let add = collection_action_rect(CollectionAction::Add);
        assert_eq!(
            hit_test_collection(5, add.x + 10, add.y + 10),
            Some(CollectionHit::Add)
        );
    }

    #[test]
    #[ignore = "manual visual evidence; run explicitly with --ignored"]
    fn dump_collection_png_for_evidence() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let background = load_rgb(&data.join("ui/menu_bg_city.png"));
        let art = ArtBank::from_catalog_dir(
            &data.join("art"),
            &["strike", "guard", "fireball", "focus", "bash"],
        );
        let cards = [
            CollectionCardView {
                id: "strike",
                name: "Strike",
                kind: "ATK",
                rules: "+18 chips",
                owned: 5,
            },
            CollectionCardView {
                id: "guard",
                name: "Guard",
                kind: "DEF",
                rules: "+12 chips",
                owned: 4,
            },
            CollectionCardView {
                id: "fireball",
                name: "Fireball",
                kind: "SPL",
                rules: "+35 chips · +1 mult",
                owned: 3,
            },
            CollectionCardView {
                id: "focus",
                name: "Focus",
                kind: "SKL",
                rules: "+8 chips · draw 1",
                owned: 4,
            },
            CollectionCardView {
                id: "bash",
                name: "Bash",
                kind: "ATK",
                rules: "+28 chips",
                owned: 4,
            },
        ];
        let view = CollectionView {
            vault: 12_450,
            crystals: 870,
            multiplier: 3.2,
            deck_count: 20,
            deck_limit: 40,
            min_deck: 8,
            total_chips: 375,
            extra_mult: 3,
            kind_counts: [9, 4, 3, 4],
            filter: CollectionFilter::All,
            selected_card: 2,
            status: "Select a card, then add or remove a copy from the starter deck.",
            cards: &cards,
            composition: &cards,
        };
        let mut pixels = vec![0; (WW * WH) as usize];
        paint_collection_screen(
            &mut pixels,
            &Theme::default(),
            background.as_ref(),
            &art,
            &view,
            CollectionInteraction {
                hovered_card: Some(2),
                ..CollectionInteraction::default()
            },
        );

        let output =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/collection_ui_paint.png");
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
