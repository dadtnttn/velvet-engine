//! The court's screens in the game's Persona manner, as the wardrobe and the shop are: the throne's
//! audiences (the petitioner's bust, what they ask, the Queen's answers as slanted ribbons with what
//! each costs or gives), the castle's works and the armoury as slanted cards, Nyxara's lessons (her
//! words, then the six runes to repeat), and Nemi's training (her blows coming at three targets, to
//! parry on time); the court's mornings (a row for each of the court: what she did, her four
//! tasks), the balcony's telescope (a compass, then the eyepiece) and the library's book of lore.
//! What the keys or the pointer light changes only light and colour, never a place or a size.
//! Authored at 1920×1080.
use std::f32::consts::TAU;

use super::persona::*;
use super::persona_screens::{backdrop, fill, heading, key_row, pointer_down, rim, slanted};
use super::ui_paint::*;
use super::*;

type Uv = ([f32; 2], [f32; 2]);

const INK_DARK: u32 = 0x0e0816;
const BONE: u32 = 0xf6efe4;
const GOOD: u32 = 0x4fd08a;

/// A chip: a short fact on a plate (`tone` 0 plain, 1 good, 2 bad, 3 gold).
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CourtChip {
    pub text: String,
    pub tone: u8,
}

/// One of the court on the morning's page: who, what she did, and her tasks (cards
/// `first..first + count`).
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CourtRow {
    pub name: String,
    pub role: String,
    pub report: String,
    pub chips: Vec<CourtChip>,
    pub portrait: Option<PortraitRef>,
    pub first: u8,
    pub count: u8,
}

/// A card or a ribbon to choose: an answer, a room, a sword, a lesson.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CourtCard {
    pub title: String,
    pub detail: String,
    pub chips: Vec<CourtChip>,
    pub picture: Option<PortraitRef>,
    /// An item's icon on it (its id in the shop's icons): the armoury's swords.
    pub icon: String,
    /// 0 open, 1 locked, 2 done.
    pub state: u8,
}

/// Nyxara's runes as they stand.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct RuneView {
    /// The rune shown now (while the row is shown).
    pub lit: Option<u8>,
    pub showing: bool,
    pub count: u8,
    pub entered: u8,
    pub round: u8,
    pub rounds: u8,
    pub lives: u8,
    /// The last rune pressed: which, right?, how long ago (ms).
    pub flash: Option<(u8, bool, u32)>,
    pub over: Option<bool>,
}

/// Nemi's training as it stands.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct DuelView {
    /// The blows on their way: from where (0 left, 1 right, 2 above) and how near (‰; 1000 = now).
    pub incoming: Vec<(u8, u32)>,
    /// The last blow decided: parried?, from where, how long ago (ms).
    pub last: Option<(bool, u8, u32)>,
    pub parried: u32,
    pub hit: u32,
    pub total: u32,
    pub good: u32,
    pub over: bool,
    pub start_ms: u32,
}

/// The court's page.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CourtView {
    /// 0 audience, 1 works, 2 armoury, 3 lessons, 4 a lesson's words, 5 runes, 6 a lesson's end,
    /// 7 training, 8 the morning, 9 the telescope, 10 the book of lore.
    pub kind: u8,
    pub title: String,
    pub tag: String,
    /// The page's colour (0: gold).
    pub accent: u32,
    pub stats: Vec<(String, String)>,
    pub portrait: Option<PortraitRef>,
    pub speaker: String,
    pub role: String,
    pub text: String,
    /// A second line (the answer chosen, over its reply).
    pub aside: String,
    pub cards: Vec<CourtCard>,
    pub choice: u8,
    /// What an answer did.
    pub chips: Vec<CourtChip>,
    pub notice: String,
    pub stamp: String,
    pub stamp_ms: u32,
    pub keys: Vec<(String, String)>,
    pub runes: Option<RuneView>,
    pub duel: Option<DuelView>,
    pub page_ms: u32,
    pub open_ms: u32,
    /// The morning's rows.
    pub rows: Vec<CourtRow>,
    /// The telescope's eyepiece is up (a sighting on show; `portrait` is what is seen).
    pub scope: bool,
    /// Since the lit card changed (the book's page turning), ms.
    pub flip_ms: u32,
    /// The vanity's dates: a card for each girl.
    pub dates: Vec<DateView>,
}

/// A girl on the dates' page: her affection against the next date's mark, the dates had, the
/// next one (and why it waits), and her memories (a square cut of each shot; hers yet or not).
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct DateView {
    pub name: String,
    pub role: String,
    pub color: u32,
    pub hearts: i32,
    /// The next date's mark (0: none left).
    pub need: i32,
    pub done: u8,
    pub total: u8,
    pub next: String,
    pub place: String,
    /// Why the next date waits (empty: it can be had now).
    pub why: String,
    pub night: bool,
    /// Her route done opens nights that go further (Lira's, Nyxara's), else it just ends.
    pub nights: bool,
    pub memories: Vec<(PortraitRef, bool)>,
}

fn accent_of(view: &CourtView) -> u32 {
    if view.accent == 0 {
        GOLD
    } else {
        view.accent
    }
}

fn chip_colors(tone: u8) -> (u32, u32) {
    match tone {
        1 => (GOOD, INK_DARK),
        2 => (CRIMSON, 0xffffff),
        3 => (GOLD_HI, INK_DARK),
        _ => (0x2a2032, BONE),
    }
}

/// A row of chips from (x, y), each a slanted plate; returns where the row ends.
fn chip_row(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    chips: &[CourtChip],
    size: f32,
) -> f32 {
    let mut x = x;
    for c in chips {
        let (back, ink) = chip_colors(c.tone);
        let w = text_width(font, Face::Display, &c.text, size) + size * 1.4;
        let h = size * 1.7;
        fill(v, slanted(x, y, w, h, size * 0.4), rgba(back, 1.0));
        text(
            font,
            v,
            Face::Display,
            &c.text,
            x + size * 0.7,
            y + size * 0.18,
            size,
            rgba(ink, 1.0),
        );
        x += w + size * 0.6;
    }
    x
}

/// Her stats on a slanted plate at the top right.
fn stats_plate(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    stats: &[(String, String)],
    accent: u32,
) {
    if stats.is_empty() {
        return;
    }
    let row = 46.0;
    let w = 420.0;
    let h = 26.0 + stats.len() as f32 * row;
    let (x, y) = (width - w - 90.0, 54.0);
    fill(
        v,
        slanted(x + 12.0, y + 12.0, w, h, 12.0),
        rgba(0x000000, 0.55),
    );
    fill(v, slanted(x, y, w, h, 12.0), rgba(0x120a1a, 0.95));
    fill(v, slanted(x, y, 10.0, h, 12.0), rgba(accent, 1.0));
    for (i, (label, value)) in stats.iter().enumerate() {
        let ry = y + 18.0 + i as f32 * row;
        text_shadow(
            font,
            v,
            Face::Ui,
            label,
            x + 34.0,
            ry + 4.0,
            22.0,
            rgba(CREAM, 0.95),
        );
        let vw = text_width(font, Face::Title, value, 28.0);
        text_outline(
            font,
            v,
            Face::Title,
            value,
            x + w - 36.0 - vw,
            ry,
            28.0,
            rgba(GOLD_HI, 1.0),
            rgba(0x000000, 0.8),
        );
    }
}

/// A picture (a square cut of the game's art) in a slanted frame, with a pool of light.
fn framed(v: &mut Vec<MapVertex>, r: Rect, uv: Option<Uv>, accent: u32, lit: bool, time: f32) {
    let lean = r.h * 0.03;
    fill(
        v,
        slanted(r.x + 14.0, r.y + 16.0, r.w, r.h, lean),
        rgba(0x000000, 0.6),
    );
    fill(
        v,
        slanted(r.x, r.y, r.w, r.h, lean),
        rgba(mix(accent, 0x000000, 0.78), 0.97),
    );
    halo(
        v,
        [r.x + r.w * 0.5, r.y + r.h * 0.45],
        r.w * 0.5,
        80.0,
        rgba(
            accent,
            if lit {
                0.3 + 0.08 * wave(time, 0.6)
            } else {
                0.14
            },
        ),
    );
    if let Some(uv) = uv {
        let pad = r.w * 0.04;
        cutout(
            v,
            Rect::new(r.x + pad, r.y + pad, r.w - pad * 2.0, r.h - pad * 2.0),
            uv,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
    rim(
        v,
        slanted(r.x, r.y, r.w, r.h, lean),
        3.0,
        rgba(
            if lit {
                BONE
            } else {
                mix(accent, 0x000000, 0.3)
            },
            1.0,
        ),
    );
}

/// A speaker's words on a slanted dark panel: the name on a bone tab, the lines under it.
#[allow(clippy::too_many_arguments)]
fn words_panel(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    who: &str,
    role: &str,
    words: &str,
    accent: u32,
    size: f32,
    lines_max: usize,
    reveal: usize,
) {
    fill(
        v,
        slanted(r.x + 12.0, r.y + 14.0, r.w, r.h, 12.0),
        rgba(0x000000, 0.55),
    );
    fill(v, slanted(r.x, r.y, r.w, r.h, 12.0), rgba(0x0d0814, 0.94));
    fill(v, slanted(r.x, r.y, r.w, 8.0, 0.5), rgba(accent, 1.0));
    let mut y = r.y + 26.0;
    if !who.is_empty() {
        let tw = text_width(font, Face::Title, who, 32.0) + 40.0;
        fill(v, slanted(r.x + 26.0, y, tw, 48.0, 10.0), rgba(BONE, 1.0));
        text(
            font,
            v,
            Face::Title,
            who,
            r.x + 46.0,
            y + 6.0,
            32.0,
            rgba(INK_DARK, 1.0),
        );
        if !role.is_empty() {
            text_shadow(
                font,
                v,
                Face::Italic,
                role,
                r.x + 46.0 + tw,
                y + 12.0,
                22.0,
                rgba(CREAM, 0.85),
            );
        }
        y += 70.0;
    }
    // The lines are laid out whole first, so the words typing out never jump to the next line.
    let mut left = reveal;
    for (i, line) in wrap(font, Face::Body, words, size, r.w - 80.0)
        .iter()
        .take(lines_max)
        .enumerate()
    {
        if left == 0 {
            break;
        }
        let n = line.chars().count();
        let shown: String = line.chars().take(left).collect();
        left = left.saturating_sub(n);
        text_shadow(
            font,
            v,
            Face::Body,
            &shown,
            r.x + 40.0,
            y + i as f32 * size * 1.36,
            size,
            rgba(INK, 0.97),
        );
    }
}

/// How much of a page's words have typed out `age` seconds in (from `delay`), at a reading pace.
fn typed(age: f32, delay: f32) -> usize {
    ((age - delay).max(0.0) * 75.0) as usize
}

// ------------------------------------------------------------------ where things are

const LEFT_X: f32 = 110.0;
const TOP_Y: f32 = 236.0;
/// The audience: the bust's frame, the words, and the answers anchored at the foot.
const BUST: Rect = Rect::new(110.0, 236.0, 520.0, 520.0);
const SAY_X: f32 = 690.0;
const SAY_W: f32 = 1120.0;
const RIBBON_H: f32 = 92.0;
const RIBBON_GAP: f32 = 16.0;
/// What a petitioner asks (four lines, smaller type when a language runs longer); the answers
/// hang right under it.
const ASK_H: f32 = 220.0;
const RIBBONS_TOP: f32 = TOP_Y + ASK_H + 34.0;

fn ribbon_rect(_n: usize, i: usize) -> Rect {
    let y = RIBBONS_TOP + i as f32 * (RIBBON_H + RIBBON_GAP);
    Rect::new(SAY_X, y, SAY_W, RIBBON_H)
}

/// The cards' grid: columns and each card's rect (`wide`: the whole width; else beside a portrait).
fn grid(n: usize, wide: bool) -> Vec<Rect> {
    let cols = if n <= 4 { 2 } else { 3 };
    let area_w = if wide { 1700.0 } else { 1220.0 };
    let gap = 40.0;
    let w = (area_w - gap * (cols as f32 - 1.0)) / cols as f32;
    let rows = n.div_ceil(cols).max(1);
    let h = ((700.0 - gap * (rows as f32 - 1.0)) / rows as f32).min(330.0);
    (0..n)
        .map(|i| {
            let (c, r) = (i % cols, i / cols);
            Rect::new(
                LEFT_X + c as f32 * (w + gap),
                TOP_Y + r as f32 * (h + gap),
                w,
                h,
            )
        })
        .collect()
}

/// The six runes' buttons.
fn rune_button(i: usize) -> ([f32; 2], f32) {
    ([960.0 + (i as f32 - 2.5) * 200.0, 830.0], 72.0)
}

/// Nemi's three targets (left, right, above).
fn target(from: u8) -> [f32; 2] {
    match from {
        0 => [470.0, 640.0],
        1 => [1070.0, 640.0],
        _ => [770.0, 420.0],
    }
}

const LANE_COLOURS: [u32; 3] = [0x5cc8ff, 0xffb347, 0xff5c9a];
const TARGET_R: f32 = 76.0;

/// What is under the pointer: a card, an answer or a rune.
pub fn hit(width: f32, height: f32, view: &CourtView, x: f32, y: f32) -> Option<usize> {
    let (x, y) = (x * 1920.0 / width.max(1.0), y * 1080.0 / height.max(1.0));
    let inside = |r: Rect| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
    match view.kind {
        0 => {
            let n = view.cards.len();
            (0..n).find(|&i| inside(ribbon_rect(n, i)))
        }
        1 | 2 => grid(view.cards.len(), true).into_iter().position(inside),
        3 => grid(view.cards.len(), false).into_iter().position(inside),
        5 => (0..6).find(|&i| {
            let (c, r) = rune_button(i);
            (x - c[0]).powi(2) + (y - c[1]).powi(2) <= r * r
        }),
        8 => (0..view.cards.len()).find(|&i| morning_slot(view, i).is_some_and(inside)),
        9 if !view.scope => (0..view.cards.len()).find(|&d| inside(compass_card(d))),
        10 => (0..view.cards.len()).find(|&i| inside(codex_row(i, view.cards.len()))),
        11 => (0..view.dates.len()).find(|&i| inside(date_card(i, view.dates.len()))),
        _ => None,
    }
}

// The morning: a row for each of the court, her portrait and what she did, then her tasks.
const ROW_Y: f32 = 222.0;
const ROW_H: f32 = 168.0;
const ROW_GAP: f32 = 12.0;
const TASK_X: f32 = 836.0;
const TASK_GAP: f32 = 14.0;

fn morning_card(row: usize, col: usize) -> Rect {
    let w = (1810.0 - TASK_X - TASK_GAP * 3.0) / 4.0;
    Rect::new(
        TASK_X + col as f32 * (w + TASK_GAP),
        ROW_Y + row as f32 * (ROW_H + ROW_GAP) + 6.0,
        w,
        ROW_H - 12.0,
    )
}

/// Card `i`'s rect on the morning's page.
fn morning_slot(view: &CourtView, i: usize) -> Option<Rect> {
    view.rows.iter().enumerate().find_map(|(r, row)| {
        let first = row.first as usize;
        (i >= first && i < first + row.count as usize).then(|| morning_card(r, i - first))
    })
}

// The telescope: a compass of four directions, then the eyepiece.
const COMPASS: [f32; 2] = [960.0, 600.0];
const EYE: [f32; 2] = [660.0, 610.0];
const EYE_R: f32 = 300.0;

fn compass_card(d: usize) -> Rect {
    let (w, h) = (380.0, 118.0);
    match d {
        0 => Rect::new(COMPASS[0] - w * 0.5, 236.0, w, h),
        1 => Rect::new(1300.0, COMPASS[1] - h * 0.5, w, h),
        2 => Rect::new(COMPASS[0] - w * 0.5, 846.0, w, h),
        _ => Rect::new(240.0, COMPASS[1] - h * 0.5, w, h),
    }
}

// The book: the entries down the left, the open page on the right.
const BOOK: Rect = Rect::new(770.0, 228.0, 1040.0, 724.0);

fn codex_row(i: usize, n: usize) -> Rect {
    let h = (724.0 / n.max(1) as f32).min(48.0);
    Rect::new(110.0, 228.0 + i as f32 * h, 610.0, h - 4.0)
}

// The dates: a card for each girl, side by side.
const DATE_W: f32 = 540.0;
const DATE_H: f32 = 730.0;

/// Where girl `i` of `n` has her tall card and its scale: centred side by side; more than three are made
/// smaller to share the page's width (the card is laid out at full size and scaled about its corner).
fn date_slot(i: usize, n: usize) -> ([f32; 2], f32) {
    let n = n.max(1) as f32;
    let gap = 40.0;
    let room = 1920.0 - 2.0 * LEFT_X;
    let s = ((room - gap * (n - 1.0)) / n / DATE_W).min(1.0);
    let total = n * DATE_W * s + (n - 1.0) * gap;
    let x = (1920.0 - total) * 0.5 + i as f32 * (DATE_W * s + gap);
    let y = TOP_Y - 14.0 + DATE_H * (1.0 - s) * 0.3;
    ([x, y], s)
}

fn date_card(i: usize, n: usize) -> Rect {
    let (o, s) = date_slot(i, n);
    Rect::new(o[0], o[1], DATE_W * s, DATE_H * s)
}

/// A ring from r0 to r1 about c.
fn annulus(v: &mut Vec<MapVertex>, c: [f32; 2], r0: f32, r1: f32, color: [f32; 4]) {
    let n = 96;
    for k in 0..n {
        let (a0, a1) = (k as f32 / n as f32 * TAU, (k + 1) as f32 / n as f32 * TAU);
        let p = |a: f32, r: f32| [c[0] + a.cos() * r, c[1] + a.sin() * r];
        push_triangle(v, [p(a0, r0), p(a0, r1), p(a1, r1)], color);
        push_triangle(v, [p(a0, r0), p(a1, r1), p(a1, r0)], color);
    }
}

/// The rune a card names (`rune:<n>` in its icon), if it names one.
fn card_rune(card: &CourtCard) -> Option<u8> {
    card.icon.strip_prefix("rune:")?.parse().ok()
}

/// A card's emblem on a round plinth: an item's icon, or one of Nyxara's runes drawn in light. A
/// slow glint runs round the rim; being lit only brightens it.
#[allow(clippy::too_many_arguments)]
fn emblem(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    radius: f32,
    icon: Option<IconEntry>,
    rune_no: Option<u8>,
    accent: u32,
    lit: bool,
    locked: bool,
    time: f32,
) {
    let glow = if locked {
        0.06
    } else if lit {
        0.44 + 0.12 * wave(time, 0.7)
    } else {
        0.2
    };
    halo(v, c, radius * 0.9, radius * 0.6, rgba(accent, glow));
    disc(v, c, radius, rgba(INK_DARK, 0.92));
    disc(v, c, radius * 0.86, rgba(mix(accent, 0x000000, 0.72), 0.95));
    annulus(
        v,
        c,
        radius - 4.0,
        radius,
        rgba(
            if lit {
                BONE
            } else {
                mix(accent, 0x000000, 0.25)
            },
            1.0,
        ),
    );
    if !locked {
        let a = time * 0.8;
        let p = |r: f32, a: f32| [c[0] + a.cos() * r, c[1] + a.sin() * r];
        for k in 0..10 {
            let (a0, a1) = (a + k as f32 * 0.05, a + (k + 1) as f32 * 0.05);
            let f = 1.0 - k as f32 / 10.0;
            push_triangle(
                v,
                [
                    p(radius - 4.0, a0),
                    p(radius + 1.0, a0),
                    p(radius + 1.0, a1),
                ],
                rgba(0xffffff, 0.75 * f),
            );
            push_triangle(
                v,
                [
                    p(radius - 4.0, a0),
                    p(radius + 1.0, a1),
                    p(radius - 4.0, a1),
                ],
                rgba(0xffffff, 0.75 * f),
            );
        }
    }
    let tint = if locked {
        [0.35, 0.32, 0.4, 1.0]
    } else if lit {
        [1.0; 4]
    } else {
        [0.84, 0.82, 0.88, 1.0]
    };
    if let Some(icon) = icon {
        let s = radius * 1.36;
        super::push_icon_rect(
            v,
            Rect::new(c[0] - s * 0.5, c[1] - s * 0.5, s, s),
            icon,
            tint,
        );
    } else if let Some(n) = rune_no {
        let breathe = if locked {
            0.0
        } else {
            0.5 + 0.5 * wave(time, 0.45)
        };
        halo(
            v,
            c,
            radius * 0.5,
            radius * 0.4,
            rgba(LILAC, 0.18 + 0.14 * breathe),
        );
        rune(
            v,
            n,
            c,
            radius * 1.05,
            11.0,
            rgba(LILAC, tint[3] * if locked { 0.4 } else { 0.95 }),
        );
        rune(
            v,
            n,
            c,
            radius * 1.05,
            4.0,
            rgba(0xffffff, if locked { 0.3 } else { 0.95 }),
        );
    }
}

// ------------------------------------------------------------------ the runes' glyphs

/// Rune `i` drawn in strokes about `c`, `s` across.
fn rune(v: &mut Vec<MapVertex>, i: u8, c: [f32; 2], s: f32, width: f32, color: [f32; 4]) {
    let p = |x: f32, y: f32| [c[0] + x * s * 0.5, c[1] + y * s * 0.5];
    let line = |v: &mut Vec<MapVertex>, pts: &[[f32; 2]]| {
        for w in pts.windows(2) {
            wire(
                v,
                p(w[0][0], w[0][1]),
                p(w[1][0], w[1][1]),
                width,
                color,
                color,
                0.0,
                false,
            );
        }
    };
    let ring = |v: &mut Vec<MapVertex>, r: f32, cx: f32, cy: f32| {
        let pts: Vec<[f32; 2]> = (0..=24)
            .map(|k| {
                let a = k as f32 / 24.0 * TAU;
                [cx + a.cos() * r, cy + a.sin() * r]
            })
            .collect();
        line(v, &pts);
    };
    match i % 6 {
        // The spark: a bolt.
        0 => line(
            v,
            &[[-0.25, -0.95], [0.25, -0.15], [-0.2, 0.1], [0.3, 0.95]],
        ),
        // The seal: a ring cut through.
        1 => {
            ring(v, 0.72, 0.0, 0.0);
            line(v, &[[0.0, -0.95], [0.0, 0.95]]);
        }
        // The light: a sun.
        2 => {
            ring(v, 0.3, 0.0, 0.0);
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                line(
                    v,
                    &[
                        [a.cos() * 0.5, a.sin() * 0.5],
                        [a.cos() * 0.92, a.sin() * 0.92],
                    ],
                );
            }
        }
        // The blood: a drop.
        3 => {
            line(
                v,
                &[
                    [0.0, -0.95],
                    [-0.55, 0.2],
                    [-0.4, 0.62],
                    [0.0, 0.85],
                    [0.4, 0.62],
                    [0.55, 0.2],
                    [0.0, -0.95],
                ],
            );
        }
        // The name: a stem with three bars.
        4 => {
            line(v, &[[0.0, -0.95], [0.0, 0.95]]);
            line(v, &[[-0.7, -0.6], [0.4, -0.6]]);
            line(v, &[[-0.4, 0.0], [0.7, 0.0]]);
            line(v, &[[-0.6, 0.6], [0.2, 0.6]]);
        }
        // The crown.
        _ => line(
            v,
            &[
                [-0.85, 0.55],
                [-0.85, -0.45],
                [-0.42, 0.05],
                [0.0, -0.7],
                [0.42, 0.05],
                [0.85, -0.45],
                [0.85, 0.55],
                [-0.85, 0.55],
            ],
        ),
    }
}

// ------------------------------------------------------------------ drawing

/// The pictures the page needs, already in the portrait atlas: its portrait and each card's.
pub struct CourtFaces {
    pub portrait: Option<Uv>,
    pub cards: Vec<Option<Uv>>,
    /// The cards' item icons.
    pub(super) icons: Vec<Option<IconEntry>>,
    /// The morning's rows' portraits.
    pub rows: Vec<Option<Uv>>,
    /// The dates' memories, girl by girl (none while one is not hers yet).
    pub dates: Vec<Vec<Option<Uv>>>,
}

pub fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &CourtView,
    faces: &CourtFaces,
    time: f32,
) {
    let accent = accent_of(view);
    let deep = mix(accent, 0x000000, 0.82);
    let age = view.page_ms as f32 / 1000.0;
    backdrop(v, width, height, accent, deep, time);
    heading(font, v, LEFT_X, 46.0, &view.title, &view.tag, accent);
    stats_plate(font, v, width, &view.stats, accent);
    match view.kind {
        0 => draw_audience(font, v, view, faces, accent, age, time),
        1 | 2 => draw_cards(font, v, view, faces, accent, true, time),
        3 => {
            framed(
                v,
                Rect::new(1390.0, 236.0, 420.0, 420.0),
                faces.portrait,
                accent,
                false,
                time,
            );
            draw_cards(font, v, view, faces, accent, false, time);
        }
        4 | 6 => draw_talk(font, v, view, faces, accent, age, time),
        5 => {
            // Nyxara at the circle's side, tracing.
            let k = entrance(age, 0.1, 0.4);
            let bust = Rect::new(1440.0, 250.0, 360.0, 360.0);
            shifted(v, [(1.0 - k) * 80.0, (time * 1.1).sin() * 2.5], |v| {
                faded(v, k, |v| {
                    framed(v, bust, faces.portrait, accent, false, time);
                    let w = text_width(font, Face::Title, &view.speaker, 30.0) + 40.0;
                    fill(
                        v,
                        slanted(bust.x + 10.0, bust.y + bust.h + 16.0, w, 48.0, 10.0),
                        rgba(BONE, 1.0),
                    );
                    text(
                        font,
                        v,
                        Face::Title,
                        &view.speaker,
                        bust.x + 30.0,
                        bust.y + bust.h + 22.0,
                        30.0,
                        rgba(INK_DARK, 1.0),
                    );
                });
            });
            draw_runes(font, v, view, accent, time);
        }
        7 => draw_duel(font, v, view, faces, accent, time),
        8 => draw_morning(font, v, view, faces, accent, time),
        9 => draw_telescope(font, v, view, faces, accent, time),
        10 => draw_codex(font, v, view, accent, time),
        11 => draw_dates(font, v, view, faces, time),
        _ => {}
    }
    // What just happened, at the foot.
    if !view.notice.is_empty() {
        let w = text_width(font, Face::Italic, &view.notice, 24.0).min(1500.0) + 60.0;
        let x = (width - w) * 0.5;
        fill(
            v,
            slanted(x, height - 132.0, w, 44.0, 10.0),
            rgba(0x0b0610, 0.92),
        );
        rim(
            v,
            slanted(x, height - 132.0, w, 44.0, 10.0),
            1.5,
            rgba(accent, 0.9),
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            &view.notice,
            x + 30.0,
            height - 124.0,
            24.0,
            rgba(CREAM, 1.0),
        );
    }
    // A big word thrown on screen.
    if !view.stamp.is_empty() {
        let t = view.stamp_ms as f32 / 1000.0;
        let slam = (t / 0.35).min(1.0);
        let size = 120.0 * (1.35 - 0.35 * ease_out(slam));
        faded(v, (1.0 - (t - 1.6).max(0.0) / 0.5).clamp(0.0, 1.0), |v| {
            comic_stamp(
                font,
                v,
                Face::Display,
                &view.stamp,
                match view.kind {
                    // Under the reply, where the answers were; over the swords; beside the
                    // gift's card; under Nemi's targets.
                    0 => [SAY_X + SAY_W * 0.5, TOP_Y + ASK_H + 300.0],
                    2 => [width * 0.5, height * 0.52],
                    6 => [SAY_X + SAY_W - 260.0, TOP_Y + 690.0],
                    7 => [width * 0.4, height * 0.84],
                    _ => [width * 0.5, 150.0],
                },
                size,
                accent,
                true,
                slam,
                time,
            );
        });
    }
    let keys: Vec<(&str, &str)> = view
        .keys
        .iter()
        .map(|(k, w)| (k.as_str(), w.as_str()))
        .collect();
    key_row(font, v, LEFT_X, height - 66.0, &keys, accent);
    // Coming in: two slanted bands of the page's colour sweep across and off, the way Persona
    // opens a menu.
    let open = view.open_ms as f32 / 1000.0;
    if open < 0.6 {
        for (delay, w, tone, a) in [(0.0, 760.0, accent, 0.92), (0.07, 300.0, 0xffffff, 0.55)] {
            let k = ((open - delay) / 0.45).clamp(0.0, 1.0);
            if k <= 0.0 || k >= 1.0 {
                continue;
            }
            let x = -w - 300.0 + (width + w + 600.0) * (k * k * (3.0 - 2.0 * k));
            fill(
                v,
                slanted(x, -20.0, w, height + 40.0, 300.0),
                rgba(tone, a * (1.0 - k * k)),
            );
        }
    }
}

fn draw_audience(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    accent: u32,
    age: f32,
    time: f32,
) {
    // The petitioner, coming forward, then shifting from foot to foot while they wait.
    let k = entrance(age, 0.0, 0.45);
    let sway = (time * 1.3).sin() * 3.0 * k;
    shifted(v, [(1.0 - k) * -80.0, sway], |v| {
        faded(v, k, |v| {
            framed(v, BUST, faces.portrait, accent, true, time);
            let name_w =
                text_width(font, Face::Title, &view.speaker, 34.0).min(BUST.w - 40.0) + 44.0;
            fill(
                v,
                slanted(BUST.x + 10.0, BUST.y + BUST.h + 18.0, name_w, 54.0, 10.0),
                rgba(BONE, 1.0),
            );
            text(
                font,
                v,
                Face::Title,
                &view.speaker,
                BUST.x + 32.0,
                BUST.y + BUST.h + 25.0,
                34.0,
                rgba(INK_DARK, 1.0),
            );
            text_shadow(
                font,
                v,
                Face::Italic,
                &view.role,
                BUST.x + 16.0,
                BUST.y + BUST.h + 84.0,
                23.0,
                rgba(CREAM, 0.92),
            );
        });
    });
    let answered = view.cards.is_empty();
    // What they ask (or what they answer back).
    let say = Rect::new(SAY_X, TOP_Y, SAY_W, ASK_H);
    // The largest type at which it all fits the box.
    let (size, lines) = [(30.0, 4), (25.0, 5), (22.0, 6)]
        .into_iter()
        .find(|&(s, n)| wrap(font, Face::Body, &view.text, s, SAY_W - 80.0).len() <= n)
        .unwrap_or((22.0, 6));
    let k2 = entrance(age, 0.15, 0.4);
    shifted(v, [0.0, (1.0 - k2) * -24.0], |v| {
        faded(v, k2, |v| {
            words_panel(
                font,
                v,
                say,
                "",
                "",
                &view.text,
                accent,
                size,
                lines,
                typed(age, 0.25),
            );
        });
    });
    if answered {
        // The answer chosen, the reply, and what it did.
        if !view.aside.is_empty() {
            let w = text_width(font, Face::Italic, &view.aside, 24.0).min(SAY_W - 60.0) + 50.0;
            fill(
                v,
                slanted(SAY_X + 20.0, TOP_Y - 26.0, w, 40.0, 8.0),
                rgba(accent, 1.0),
            );
            text(
                font,
                v,
                Face::Italic,
                &view.aside,
                SAY_X + 44.0,
                TOP_Y - 20.0,
                24.0,
                rgba(INK_DARK, 1.0),
            );
        }
        // What it did lands once the reply has been said, with a flash behind it.
        let said = (0.25 + view.text.chars().count() as f32 / 75.0).min(3.0);
        let k3 = entrance(age, said, 0.4);
        if k3 > 0.0 && k3 < 1.0 {
            halo(
                v,
                [SAY_X + 160.0, TOP_Y + ASK_H + 62.0],
                120.0,
                90.0,
                rgba(accent, 0.55 * (1.0 - k3)),
            );
        }
        shifted(v, [0.0, (1.0 - k3) * 30.0], |v| {
            faded(v, k3, |v| {
                chip_row(
                    font,
                    v,
                    SAY_X + 10.0,
                    TOP_Y + ASK_H + 40.0,
                    &view.chips,
                    28.0,
                );
            });
        });
        return;
    }
    let n = view.cards.len();
    for (i, card) in view.cards.iter().enumerate() {
        let r = ribbon_rect(n, i);
        let lit = view.choice as usize == i;
        let locked = card.state == 1;
        let k = entrance(age, 0.25 + i as f32 * 0.08, 0.35);
        shifted(v, [(1.0 - k) * 120.0, 0.0], |v| {
            faded(v, k, |v| {
                let q = slanted(r.x, r.y, r.w, r.h, 12.0);
                fill(
                    v,
                    slanted(r.x + 10.0, r.y + 12.0, r.w, r.h, 12.0),
                    rgba(0x000000, 0.5),
                );
                if lit {
                    soft(
                        v,
                        r,
                        10.0,
                        26.0,
                        rgba(accent, 0.4 + 0.1 * wave(time, 0.9)),
                        true,
                    );
                }
                let back = if lit && !locked {
                    mix(accent, 0x000000, 0.25)
                } else {
                    0x140c1c
                };
                fill(v, q, rgba(back, 0.97));
                keycap(
                    font,
                    v,
                    Rect::new(r.x + 22.0, r.y + 24.0, 44.0, 44.0),
                    &format!("{}", i + 1),
                    20.0,
                    lit,
                    locked,
                    accent,
                    1.0,
                );
                let ink = if lit && !locked {
                    INK_DARK
                } else if locked {
                    0x8a7a92
                } else {
                    BONE
                };
                let title_size = 28.0f32
                    .min(28.0 * 760.0 / text_width(font, Face::Body, &card.title, 28.0).max(1.0));
                text(
                    font,
                    v,
                    Face::Body,
                    &card.title,
                    r.x + 88.0,
                    r.y + 14.0,
                    title_size,
                    rgba(ink, 1.0),
                );
                if !card.detail.is_empty() {
                    text(
                        font,
                        v,
                        Face::Italic,
                        &card.detail,
                        r.x + 90.0,
                        r.y + 54.0,
                        20.0,
                        rgba(if lit && !locked { INK_DARK } else { CREAM }, 0.9),
                    );
                } else {
                    chip_row(font, v, r.x + 90.0, r.y + 52.0, &card.chips, 18.0);
                }
                if locked {
                    padlock(v, [r.x + r.w - 46.0, r.y + r.h * 0.5], 28.0, GOLD, 0.9);
                }
                rim(
                    v,
                    q,
                    if lit { 3.5 } else { 1.2 },
                    rgba(
                        if lit {
                            BONE
                        } else {
                            mix(accent, 0x000000, 0.45)
                        },
                        1.0,
                    ),
                );
                if lit {
                    cursor(v, [r.x - 14.0, r.y + r.h * 0.5], 30.0, time);
                }
            });
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_cards(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    accent: u32,
    wide: bool,
    time: f32,
) {
    let age = view.page_ms as f32 / 1000.0;
    let rects = grid(view.cards.len(), wide);
    for (i, (card, r)) in view.cards.iter().zip(rects).enumerate() {
        let lit = view.choice as usize == i;
        let locked = card.state == 1;
        let done = card.state == 2;
        let k = entrance(age, i as f32 * 0.06, 0.35);
        shifted(v, [0.0, (1.0 - k) * 40.0], |v| {
            faded(v, k, |v| {
                let lean = 14.0;
                let q = slanted(r.x, r.y, r.w, r.h, lean);
                fill(
                    v,
                    slanted(r.x + 12.0, r.y + 14.0, r.w, r.h, lean),
                    rgba(0x000000, 0.55),
                );
                if lit {
                    soft(
                        v,
                        r,
                        12.0,
                        30.0,
                        rgba(accent, 0.42 + 0.1 * wave(time, 0.8)),
                        true,
                    );
                }
                let top = if locked {
                    0x16101e
                } else if lit {
                    mix(accent, 0x000000, 0.55)
                } else {
                    mix(accent, 0x000000, 0.8)
                };
                push_triangle(v, [q[0], q[1], q[2]], rgba(top, 0.97));
                push_triangle(v, [q[0], q[2], q[3]], rgba(INK_DARK, 0.97));
                push_outline_fringe(v, &q, rgba(INK_DARK, 0.97));
                // Its picture, if it has one.
                let mut tx = r.x + 30.0;
                if let Some(uv) = faces.cards.get(i).copied().flatten() {
                    let side = (r.h - 50.0).min(r.w * 0.38);
                    let tint = if locked {
                        [0.35, 0.32, 0.4, 1.0]
                    } else if lit {
                        [1.0; 4]
                    } else {
                        [0.8, 0.78, 0.84, 1.0]
                    };
                    cutout(v, Rect::new(r.x + 26.0, r.y + 25.0, side, side), uv, tint);
                    rim(
                        v,
                        slanted(r.x + 26.0, r.y + 25.0, side, side, 0.0),
                        1.5,
                        rgba(mix(accent, 0x000000, 0.3), 1.0),
                    );
                    tx = r.x + 26.0 + side + 26.0;
                } else {
                    let icon = faces.icons.get(i).copied().flatten();
                    let rune_no = card_rune(card);
                    if icon.is_some() || rune_no.is_some() {
                        let radius = ((r.h - 70.0) * 0.5).min(r.w * 0.17);
                        emblem(
                            v,
                            [r.x + 30.0 + radius, r.y + r.h * 0.5],
                            radius,
                            icon,
                            rune_no,
                            accent,
                            lit,
                            locked,
                            time,
                        );
                        tx = r.x + 30.0 + radius * 2.0 + 30.0;
                    }
                }
                let text_w = r.x + r.w - tx - 30.0;
                let title_size = 30.0f32
                    .min(30.0 * text_w / text_width(font, Face::Title, &card.title, 30.0).max(1.0));
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &card.title,
                    tx,
                    r.y + 26.0,
                    title_size,
                    rgba(if locked { 0x9a8aa2 } else { BONE }, 1.0),
                    rgba(0x000000, 0.8),
                );
                // The smallest size that still fits it all between the title and the chips (a
                // longer language), cut with «…» if even that does not.
                let fit = |size: f32| {
                    let lines = wrap(font, Face::Italic, &card.detail, size, text_w);
                    let room = ((r.h - 140.0) / (size * 1.25)).max(1.0) as usize;
                    (lines, room, size)
                };
                let (mut lines, room, size) = [20.0, 18.0, 16.5, 15.0]
                    .into_iter()
                    .map(fit)
                    .find(|(lines, room, _)| lines.len() <= *room)
                    .unwrap_or_else(|| fit(15.0));
                if lines.len() > room {
                    lines.truncate(room);
                    if let Some(last) = lines.last_mut() {
                        last.push('…');
                    }
                }
                for (j, l) in lines.iter().enumerate() {
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        l,
                        tx,
                        r.y + 74.0 + j as f32 * size * 1.25,
                        size,
                        rgba(CREAM, if locked { 0.6 } else { 0.92 }),
                    );
                }
                chip_row(font, v, tx, r.y + r.h - 58.0, &card.chips, 19.0);
                if done {
                    let sc = [r.x + r.w - 46.0, r.y + 40.0];
                    disc(v, sc, 24.0, rgba(INK_DARK, 0.9));
                    disc(v, sc, 20.0, rgba(GOOD, 1.0));
                    wire(
                        v,
                        [sc[0] - 9.0, sc[1]],
                        [sc[0] - 2.0, sc[1] + 7.0],
                        4.5,
                        rgba(INK_DARK, 1.0),
                        rgba(INK_DARK, 1.0),
                        0.0,
                        false,
                    );
                    wire(
                        v,
                        [sc[0] - 2.0, sc[1] + 7.0],
                        [sc[0] + 10.0, sc[1] - 7.0],
                        4.5,
                        rgba(INK_DARK, 1.0),
                        rgba(INK_DARK, 1.0),
                        0.0,
                        false,
                    );
                } else if locked {
                    padlock(v, [r.x + r.w - 46.0, r.y + 42.0], 26.0, GOLD, 0.9);
                }
                keycap(
                    font,
                    v,
                    Rect::new(r.x + r.w - 76.0, r.y + r.h - 62.0, 40.0, 40.0),
                    &format!("{}", i + 1),
                    18.0,
                    lit,
                    locked,
                    accent,
                    1.0,
                );
                rim(
                    v,
                    q,
                    if lit { 4.0 } else { 1.5 },
                    rgba(
                        if lit {
                            BONE
                        } else {
                            mix(accent, 0x000000, 0.4)
                        },
                        1.0,
                    ),
                );
                if lit {
                    pointer_down(v, [r.x + r.w * 0.5, r.y - 12.0], time);
                }
            });
        });
    }
}

/// A small task card on the morning's page.
#[allow(clippy::too_many_arguments)]
fn task_card(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    card: &CourtCard,
    lit: bool,
    accent: u32,
    k: f32,
    time: f32,
) {
    let locked = card.state == 1;
    let given = card.state == 2;
    shifted(v, [(1.0 - k) * 60.0, 0.0], |v| {
        faded(v, k, |v| {
            let lean = 10.0;
            let q = slanted(r.x, r.y, r.w, r.h, lean);
            fill(
                v,
                slanted(r.x + 8.0, r.y + 10.0, r.w, r.h, lean),
                rgba(0x000000, 0.5),
            );
            if lit {
                soft(
                    v,
                    r,
                    10.0,
                    22.0,
                    rgba(accent, 0.4 + 0.1 * wave(time, 0.8)),
                    true,
                );
            }
            let back = if locked {
                0x16101e
            } else if given {
                mix(accent, 0x000000, 0.5)
            } else if lit {
                mix(accent, 0x000000, 0.62)
            } else {
                0x150d1c
            };
            fill(v, q, rgba(back, 0.97));
            if given {
                fill(v, slanted(r.x, r.y, 8.0, r.h, lean), rgba(GOLD_HI, 1.0));
            }
            let tx = r.x + 20.0;
            let tw = r.w - 40.0;
            let title_w = if locked { tw - 30.0 } else { tw };
            let size = 22.0f32
                .min(22.0 * title_w / text_width(font, Face::Title, &card.title, 22.0).max(1.0));
            text_outline(
                font,
                v,
                Face::Title,
                &card.title,
                tx,
                r.y + 14.0,
                size.max(15.0),
                rgba(if locked { 0x9a8aa2 } else { BONE }, 1.0),
                rgba(0x000000, 0.8),
            );
            let (lines, dsize) = [16.0f32, 14.5, 13.0]
                .into_iter()
                .map(|s| (wrap(font, Face::Italic, &card.detail, s, tw), s))
                .find(|(l, _)| l.len() <= 3)
                .unwrap_or_else(|| (wrap(font, Face::Italic, &card.detail, 13.0, tw), 13.0));
            for (j, l) in lines.iter().take(3).enumerate() {
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    l,
                    tx,
                    r.y + 48.0 + j as f32 * dsize * 1.25,
                    dsize,
                    rgba(CREAM, if locked { 0.55 } else { 0.9 }),
                );
            }
            // the chips on as many rows as they need, from the foot up
            let mut rows: Vec<Vec<CourtChip>> = vec![Vec::new()];
            let mut used = 0.0;
            for c in &card.chips {
                let w = text_width(font, Face::Display, &c.text, 16.0) + 16.0 * 1.4 + 8.0;
                if used + w > tw && !rows.last().is_some_and(|r| r.is_empty()) {
                    rows.push(Vec::new());
                    used = 0.0;
                }
                used += w;
                rows.last_mut().expect("a row").push(c.clone());
            }
            let n = rows.len();
            for (j, row) in rows.iter().enumerate() {
                chip_row(
                    font,
                    v,
                    tx,
                    r.y + r.h - 36.0 - (n - 1 - j) as f32 * 30.0,
                    row,
                    16.0,
                );
            }
            if locked {
                padlock(v, [r.x + r.w - 30.0, r.y + 26.0], 20.0, GOLD, 0.9);
            }
            rim(
                v,
                q,
                if lit { 3.5 } else { 1.2 },
                rgba(
                    if lit {
                        BONE
                    } else if given {
                        GOLD_HI
                    } else {
                        mix(accent, 0x000000, 0.45)
                    },
                    1.0,
                ),
            );
        });
    });
}

fn draw_morning(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    accent: u32,
    time: f32,
) {
    let age = view.page_ms as f32 / 1000.0;
    for (r, row) in view.rows.iter().enumerate() {
        let y = ROW_Y + r as f32 * (ROW_H + ROW_GAP);
        let k = entrance(age, r as f32 * 0.09, 0.4);
        shifted(v, [(1.0 - k) * -70.0, 0.0], |v| {
            faded(v, k, |v| {
                // the band behind her
                fill(
                    v,
                    slanted(LEFT_X - 6.0, y, TASK_X - LEFT_X - 18.0, ROW_H, 12.0),
                    rgba(0x0b0610, 0.82),
                );
                fill(
                    v,
                    slanted(LEFT_X - 6.0, y, 8.0, ROW_H, 12.0),
                    rgba(accent, 1.0),
                );
                framed(
                    v,
                    Rect::new(LEFT_X + 16.0, y + 12.0, 150.0, 150.0),
                    faces.rows.get(r).copied().flatten(),
                    accent,
                    false,
                    time,
                );
                let tx = LEFT_X + 192.0;
                let tw = TASK_X - tx - 40.0;
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &row.name,
                    tx,
                    y + 10.0,
                    32.0,
                    rgba(BONE, 1.0),
                    rgba(0x000000, 0.8),
                );
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    &row.role,
                    tx,
                    y + 50.0,
                    17.0,
                    rgba(accent, 0.95),
                );
                let said = if row.report.is_empty() {
                    "—Aún no le he contado nada, Majestad."
                } else {
                    row.report.as_str()
                };
                let (lines, size) = [19.0f32, 17.5, 16.0]
                    .into_iter()
                    .map(|s| (wrap(font, Face::Italic, said, s, tw), s))
                    .find(|(l, _)| l.len() <= 3)
                    .unwrap_or_else(|| (wrap(font, Face::Italic, said, 16.0, tw), 16.0));
                for (j, l) in lines.iter().take(3).enumerate() {
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        l,
                        tx,
                        y + 78.0 + j as f32 * size * 1.28,
                        size,
                        rgba(CREAM, if row.report.is_empty() { 0.55 } else { 0.95 }),
                    );
                }
                chip_row(font, v, tx, y + ROW_H - 38.0, &row.chips, 17.0);
            });
        });
        for c in 0..row.count as usize {
            let i = row.first as usize + c;
            if let Some(card) = view.cards.get(i) {
                let kk = entrance(age, 0.15 + r as f32 * 0.09 + c as f32 * 0.04, 0.32);
                task_card(
                    font,
                    v,
                    morning_card(r, c),
                    card,
                    view.choice as usize == i,
                    accent,
                    kk,
                    time,
                );
            }
        }
    }
}

fn draw_telescope(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    accent: u32,
    time: f32,
) {
    let age = view.page_ms as f32 / 1000.0;
    if !view.scope {
        // The compass: the brass ring and its rose, the four ways round it.
        let c = COMPASS;
        let k = entrance(age, 0.0, 0.45);
        faded(v, k, |v| {
            disc(v, c, 190.0, rgba(0x0b0610, 0.9));
            annulus(v, c, 176.0, 190.0, rgba(GOLD, 0.95));
            annulus(v, c, 150.0, 154.0, rgba(GOLD, 0.5));
            let spin = time * 0.15;
            for i in 0..16 {
                let a = spin + i as f32 / 16.0 * TAU;
                let long = if i % 4 == 0 {
                    140.0
                } else if i % 2 == 0 {
                    96.0
                } else {
                    70.0
                };
                let tip = [c[0] + a.cos() * long, c[1] + a.sin() * long];
                let spread = if i % 4 == 0 { 0.38 } else { 0.22 };
                let l = [
                    c[0] + (a + spread).cos() * 30.0,
                    c[1] + (a + spread).sin() * 30.0,
                ];
                let r = [
                    c[0] + (a - spread).cos() * 30.0,
                    c[1] + (a - spread).sin() * 30.0,
                ];
                let lit = i % 4 == 0 && (i / 4) as u8 == (view.choice + 3) % 4;
                push_triangle(
                    v,
                    [l, tip, c],
                    rgba(
                        if lit {
                            GOLD_HI
                        } else {
                            mix(GOLD, 0x000000, 0.35)
                        },
                        1.0,
                    ),
                );
                push_triangle(
                    v,
                    [c, tip, r],
                    rgba(if lit { BONE } else { mix(GOLD, 0x000000, 0.6) }, 1.0),
                );
            }
            disc(v, c, 16.0, rgba(GOLD_HI, 1.0));
        });
        for (d, card) in view.cards.iter().enumerate() {
            let r = compass_card(d);
            let lit = view.choice as usize == d;
            let kk = entrance(age, 0.12 + d as f32 * 0.07, 0.35);
            shifted(v, [0.0, (1.0 - kk) * 30.0], |v| {
                faded(v, kk, |v| {
                    let q = slanted(r.x, r.y, r.w, r.h, 12.0);
                    fill(
                        v,
                        slanted(r.x + 10.0, r.y + 12.0, r.w, r.h, 12.0),
                        rgba(0x000000, 0.5),
                    );
                    if lit {
                        soft(
                            v,
                            r,
                            10.0,
                            26.0,
                            rgba(accent, 0.42 + 0.1 * wave(time, 0.8)),
                            true,
                        );
                    }
                    fill(
                        v,
                        q,
                        rgba(
                            if lit {
                                mix(accent, 0x000000, 0.35)
                            } else {
                                0x140c1c
                            },
                            0.97,
                        ),
                    );
                    let ink = if lit { INK_DARK } else { BONE };
                    text(
                        font,
                        v,
                        Face::Title,
                        &card.title,
                        r.x + 28.0,
                        r.y + 14.0,
                        36.0,
                        rgba(ink, 1.0),
                    );
                    text(
                        font,
                        v,
                        Face::Italic,
                        &card.detail,
                        r.x + 28.0,
                        r.y + 62.0,
                        20.0,
                        rgba(if lit { INK_DARK } else { CREAM }, 0.92),
                    );
                    if !card.chips.is_empty() {
                        let w = text_width(font, Face::Display, &card.chips[0].text, 15.0) + 30.0;
                        chip_row(font, v, r.x + r.w - w - 10.0, r.y - 14.0, &card.chips, 15.0);
                    }
                    rim(
                        v,
                        q,
                        if lit { 3.5 } else { 1.4 },
                        rgba(
                            if lit {
                                BONE
                            } else {
                                mix(accent, 0x000000, 0.4)
                            },
                            1.0,
                        ),
                    );
                });
            });
        }
        return;
    }
    // The eyepiece: what is seen in a ring of brass, the dark of the tube round it.
    let k = entrance(age, 0.0, 0.5);
    let open = ease_out(k);
    let r = EYE_R * (0.86 + 0.14 * open);
    faded(v, k, |v| {
        // the tube's dark round the glass
        halo(v, EYE, r + 40.0, 90.0, rgba(0x000000, 0.75));
        disc(v, EYE, r + 30.0, rgba(0x06030a, 1.0));
        if let Some(uv) = faces.portrait {
            cutout_round(
                v,
                Rect::new(EYE[0] - r, EYE[1] - r, r * 2.0, r * 2.0),
                uv,
                [1.0, 1.0, 1.0, 1.0],
                r,
            );
        } else {
            disc(v, EYE, r, rgba(0x1a1230, 1.0));
        }
        // a soft dark edge inside the glass, the brass rim, a glint
        for i in 0..6 {
            let t = i as f32 / 6.0;
            annulus(
                v,
                EYE,
                r - 34.0 + t * 34.0,
                r - 28.0 + t * 34.0,
                rgba(0x000000, 0.08 + t * 0.14),
            );
        }
        annulus(v, EYE, r, r + 16.0, rgba(GOLD, 1.0));
        annulus(
            v,
            EYE,
            r + 16.0,
            r + 22.0,
            rgba(mix(GOLD, 0x000000, 0.55), 1.0),
        );
        arc(
            v,
            EYE,
            r - 40.0,
            5.0,
            -2.6,
            0.1,
            rgba(0xffffff, 0.16),
            rgba(0xffffff, 0.0),
        );
        // the cross hairs, faint
        wire(
            v,
            [EYE[0] - 24.0, EYE[1]],
            [EYE[0] + 24.0, EYE[1]],
            1.5,
            rgba(0xffffff, 0.35),
            rgba(0xffffff, 0.35),
            0.0,
            false,
        );
        wire(
            v,
            [EYE[0], EYE[1] - 24.0],
            [EYE[0], EYE[1] + 24.0],
            1.5,
            rgba(0xffffff, 0.35),
            rgba(0xffffff, 0.35),
            0.0,
            false,
        );
    });
    let k2 = entrance(age, 0.25, 0.4);
    shifted(v, [(1.0 - k2) * 60.0, 0.0], |v| {
        faded(v, k2, |v| {
            // As tall as what is seen takes to tell, centred on the glass.
            let lines = wrap(font, Face::Body, &view.text, 26.0, 740.0 - 80.0)
                .len()
                .min(9);
            let h = (96.0 + lines as f32 * 26.0 * 1.36 + 34.0).max(220.0);
            let panel = Rect::new(1070.0, (EYE[1] - h * 0.5 - 20.0).max(300.0), 740.0, h);
            words_panel(
                font,
                v,
                panel,
                &view.speaker,
                "",
                &view.text,
                accent,
                26.0,
                9,
                typed(age, 0.35),
            );
            text_shadow(
                font,
                v,
                Face::Italic,
                &view.role,
                panel.x + 30.0,
                panel.y - 40.0,
                22.0,
                rgba(accent, 1.0),
            );
            chip_row(
                font,
                v,
                panel.x + 20.0,
                panel.y + panel.h + 26.0,
                &view.chips,
                22.0,
            );
        });
    });
}

fn draw_codex(font: &FontAtlas, v: &mut Vec<MapVertex>, view: &CourtView, accent: u32, time: f32) {
    let age = view.page_ms as f32 / 1000.0;
    let n = view.cards.len();
    for (i, card) in view.cards.iter().enumerate() {
        let r = codex_row(i, n);
        let lit = view.choice as usize == i;
        let locked = card.state == 1;
        let diary = i + 1 == n;
        let k = entrance(age, i as f32 * 0.025, 0.3);
        shifted(v, [(1.0 - k) * -50.0, 0.0], |v| {
            faded(v, k, |v| {
                let q = slanted(r.x, r.y, r.w, r.h, 8.0);
                if lit {
                    soft(
                        v,
                        r,
                        6.0,
                        16.0,
                        rgba(accent, 0.35 + 0.08 * wave(time, 0.8)),
                        true,
                    );
                }
                fill(
                    v,
                    q,
                    rgba(
                        if lit {
                            mix(accent, 0x000000, 0.35)
                        } else if diary {
                            0x24122a
                        } else {
                            0x110a18
                        },
                        0.95,
                    ),
                );
                let ink = if lit {
                    INK_DARK
                } else if locked {
                    0x7a6a82
                } else {
                    BONE
                };
                let size = (r.h * 0.46).min(21.0);
                let tag_w = text_width(font, Face::Ui, &card.detail, size * 0.66) + 18.0;
                let room = r.w - 40.0 - tag_w - 70.0;
                let title_size = size
                    .min(size * room / text_width(font, Face::Body, &card.title, size).max(1.0));
                text(
                    font,
                    v,
                    if diary { Face::Italic } else { Face::Body },
                    &card.title,
                    r.x + 22.0,
                    r.y + (r.h - title_size) * 0.42,
                    title_size,
                    rgba(ink, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Ui,
                    &card.detail,
                    r.x + r.w - tag_w - 10.0,
                    r.y + (r.h - size * 0.66) * 0.5,
                    size * 0.66,
                    rgba(if lit { INK_DARK } else { accent }, 0.85),
                );
                if locked {
                    padlock(
                        v,
                        [r.x + r.w - tag_w - 34.0, r.y + r.h * 0.5],
                        size * 0.85,
                        GOLD,
                        0.7,
                    );
                } else if !card.chips.is_empty() {
                    let w = text_width(font, Face::Display, &card.chips[0].text, 12.0) + 22.0;
                    chip_row(
                        font,
                        v,
                        r.x + r.w - tag_w - w - 24.0,
                        r.y + (r.h - 20.0) * 0.5,
                        &card.chips,
                        12.0,
                    );
                }
            });
        });
    }
    // The page open: paper in a dark binding; it turns when the lit entry changes.
    let b = BOOK;
    let k = entrance(age, 0.1, 0.45);
    let flip = (view.flip_ms as f32 / 1000.0 / 0.34).min(1.0);
    let turned = ease_out(flip);
    faded(v, k, |v| {
        fill(
            v,
            slanted(b.x + 14.0, b.y + 16.0, b.w, b.h, 14.0),
            rgba(0x000000, 0.6),
        );
        fill(
            v,
            slanted(b.x - 14.0, b.y - 14.0, b.w + 28.0, b.h + 28.0, 14.0),
            rgba(0x2a1424, 1.0),
        );
        fill(v, slanted(b.x, b.y, b.w, b.h, 12.0), rgba(0xefe4cc, 1.0));
        fill(v, slanted(b.x, b.y, 26.0, b.h, 12.0), rgba(0xd8c8a8, 1.0));
        rim(
            v,
            slanted(b.x - 14.0, b.y - 14.0, b.w + 28.0, b.h + 28.0, 14.0),
            2.0,
            rgba(accent, 0.9),
        );
        shifted(v, [(1.0 - turned) * 40.0, 0.0], |v| {
            faded(v, turned, |v| {
                let tx = b.x + 70.0;
                let tw = b.w - 130.0;
                let size = 40.0f32
                    .min(40.0 * tw / text_width(font, Face::Title, &view.speaker, 40.0).max(1.0));
                text(
                    font,
                    v,
                    Face::Title,
                    &view.speaker,
                    tx,
                    b.y + 40.0,
                    size,
                    rgba(0x2a1c14, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Italic,
                    &view.role,
                    tx,
                    b.y + 96.0,
                    22.0,
                    rgba(0x7a4a3a, 1.0),
                );
                fill(
                    v,
                    slanted(tx, b.y + 134.0, tw * 0.6, 3.0, 0.0),
                    rgba(0xb08a5a, 0.8),
                );
                // The largest type the page holds (a short entry reads big), the fleuron's room kept.
                let (lines, s) = [30.0f32, 27.0, 24.0, 22.0, 20.0, 18.5]
                    .into_iter()
                    .map(|s| (wrap(font, Face::Body, &view.text, s, tw), s))
                    .find(|(l, s)| (l.len() as f32) * s * 1.42 <= b.h - 270.0)
                    .unwrap_or_else(|| (wrap(font, Face::Body, &view.text, 18.5, tw), 18.5));
                let mut end = b.y + 160.0;
                for (j, l) in lines.iter().enumerate() {
                    let y = b.y + 160.0 + j as f32 * s * 1.42;
                    if y > b.y + b.h - 110.0 {
                        break;
                    }
                    text(font, v, Face::Body, l, tx, y, s, rgba(0x2a1c14, 0.95));
                    end = y + s * 1.42;
                }
                // A fleuron under the last line, and the folio at the foot.
                if !view.text.is_empty() {
                    let fy = (end + 46.0).min(b.y + b.h - 90.0);
                    let fx = tx + tw * 0.5;
                    let ink = rgba(0xb08a5a, 0.85);
                    fill(v, slanted(fx - 150.0, fy, 110.0, 2.0, 0.0), ink);
                    fill(v, slanted(fx + 40.0, fy, 110.0, 2.0, 0.0), ink);
                    for (dx, r) in [(-24.0, 5.0), (0.0, 9.0), (24.0, 5.0)] {
                        gem_turned(
                            v,
                            [fx + dx, fy + 1.0],
                            r * 0.7,
                            r,
                            0.0,
                            0xb08a5a,
                            0xe8d2a8,
                            0.9,
                            0.0,
                        );
                    }
                }
                let folio = format!("{}", view.choice as usize + 1);
                let fw = text_width(font, Face::Italic, &folio, 22.0);
                text(
                    font,
                    v,
                    Face::Italic,
                    &folio,
                    b.x + b.w - 50.0 - fw,
                    b.y + b.h - 50.0,
                    22.0,
                    rgba(0x7a4a3a, 0.8),
                );
            });
        });
        // the leaf turning over, from the right edge to the spine
        if flip < 1.0 {
            let w = b.w * (1.0 - turned);
            let x = b.x + b.w - w;
            fill(
                v,
                slanted(x, b.y, w, b.h, 12.0),
                rgba(0xf6eedc, 0.85 * (1.0 - flip)),
            );
            fill(
                v,
                slanted(x - 18.0, b.y, 18.0, b.h, 12.0),
                rgba(0x000000, 0.12 * (1.0 - flip)),
            );
        }
    });
}

/// A heart about `c`, `r` from its middle to its sides.
fn heart(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, color: [f32; 4]) {
    disc(v, [c[0] - r * 0.48, c[1] - r * 0.2], r * 0.56, color);
    disc(v, [c[0] + r * 0.48, c[1] - r * 0.2], r * 0.56, color);
    push_triangle(
        v,
        [
            [c[0] - r * 1.02, c[1] - r * 0.06],
            [c[0] + r * 1.02, c[1] - r * 0.06],
            [c[0], c[1] + r * 1.02],
        ],
        color,
    );
}

/// The size at which `s` fits `max` wide (no smaller than half of `size`).
fn fitted_size(font: &FontAtlas, face: Face, s: &str, size: f32, max: f32) -> f32 {
    let w = text_width(font, face, s, size);
    if w > max {
        (size * max / w).max(size * 0.5)
    } else {
        size
    }
}

/// The vanity's dates: a tall card for each girl side by side (centred; more than three share the width,
/// smaller): her bust from her sprite with her name on a plate of her colour, her affection as hearts
/// against the next date's mark, the route's chips, the next date (or why it waits) and her memories
/// along the foot (the ones not had yet, dark with a question mark). The lit card rises a little.
fn draw_dates(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    time: f32,
) {
    let age = view.page_ms as f32 / 1000.0;
    let n = view.dates.len();
    for (i, d) in view.dates.iter().enumerate() {
        let (o, scale) = date_slot(i, n);
        let lit = view.choice as usize == i;
        let state = view.cards.get(i).map_or(0, |c| c.state);
        let (waiting, complete) = (state == 1, state == 2);
        let color = if d.color == 0 { GOLD } else { d.color };
        let k = entrance(age, 0.08 * i as f32, 0.38);
        let lift = if lit { -14.0 } else { 0.0 };
        let r = Rect::new(o[0], o[1] + lift, DATE_W, DATE_H);
        scaled(v, o, scale, |v| {
            shifted(v, [0.0, (1.0 - k) * 46.0], |v| {
                faded(v, k, |v| {
                    date_card_tall(font, v, r, d, faces, i, color, lit, waiting, complete, time)
                });
            })
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn date_card_tall(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    d: &DateView,
    faces: &CourtFaces,
    i: usize,
    color: u32,
    lit: bool,
    waiting: bool,
    complete: bool,
    time: f32,
) {
    let lean = 14.0;
    // the card: its shadow, its glow when lit, the plate, a band of her colour at the top
    fill(
        v,
        slanted(r.x + 14.0, r.y + 18.0, r.w, r.h, lean),
        rgba(0x000000, 0.55),
    );
    if lit {
        soft(
            v,
            r,
            14.0,
            30.0,
            rgba(color, 0.32 + 0.08 * wave(time, 0.8)),
            true,
        );
    }
    fill(v, slanted(r.x, r.y, r.w, r.h, lean), rgba(0x0d0814, 0.97));
    halo(
        v,
        [r.x + 140.0, r.y + 190.0],
        220.0,
        120.0,
        rgba(color, if lit { 0.15 } else { 0.07 }),
    );
    fill(v, slanted(r.x, r.y, r.w, 10.0, 0.6), rgba(color, 1.0));

    // her bust (tall), and her name on a plate of her colour across its foot
    let bust = Rect::new(r.x + 24.0, r.y + 30.0, 228.0, 304.0);
    framed(
        v,
        bust,
        faces.cards.get(i).copied().flatten(),
        color,
        lit,
        time,
    );
    let name_size = fitted_size(font, Face::Title, &d.name, 54.0, r.w - 300.0);
    text_outline(
        font,
        v,
        Face::Title,
        &d.name,
        r.x + 272.0,
        r.y + 34.0,
        name_size,
        rgba(BONE, 1.0),
        rgba(0x000000, 0.85),
    );
    let role_lines = wrap(font, Face::Italic, &d.role, 21.0, r.w - 296.0);
    for (k, line) in role_lines.iter().take(2).enumerate() {
        text_shadow(
            font,
            v,
            Face::Italic,
            line,
            r.x + 274.0,
            r.y + 100.0 + k as f32 * 26.0,
            21.0,
            rgba(CREAM, 0.9),
        );
    }

    // her affection: ten hearts, the next date's mark ringed in gold
    let full = if complete { 10 } else { d.hearts.clamp(0, 10) };
    let mark = if complete || d.need <= 0 {
        None
    } else {
        Some(d.need.min(10))
    };
    let hy = r.y + 200.0;
    text_shadow(
        font,
        v,
        Face::Ui,
        "AFECTO",
        r.x + 274.0,
        hy - 32.0,
        17.0,
        rgba(mix(color, 0xffffff, 0.35), 1.0),
    );
    for h in 0..10 {
        let c = [r.x + 288.0 + h as f32 * 23.0, hy];
        let ringed = mark == Some(h + 1);
        heart(
            v,
            [c[0], c[1] + 1.0],
            if ringed { 13.0 } else { 11.5 },
            rgba(if ringed { GOLD_HI } else { INK_DARK }, 1.0),
        );
        let colour = if h < full {
            rgba(mix(color, 0xffffff, 0.25), if lit { 1.0 } else { 0.9 })
        } else {
            rgba(0x2a2032, 1.0)
        };
        heart(v, c, 9.5, colour);
        if h < full {
            disc(v, [c[0] - 4.0, c[1] - 4.0], 2.0, rgba(0xffffff, 0.7));
        }
    }
    if let Some(need) = mark {
        text_shadow(
            font,
            v,
            Face::Display,
            &format!("{} / {need}", d.hearts.max(0)),
            r.x + 274.0,
            hy + 20.0,
            21.0,
            rgba(if d.hearts >= need { GOLD_HI } else { CREAM }, 1.0),
        );
    }
    // the route
    let mut chips = vec![if complete {
        CourtChip {
            text: "RUTA COMPLETA".into(),
            tone: 1,
        }
    } else {
        CourtChip {
            text: format!("CITA {} / {}", d.done + 1, d.total),
            tone: 3,
        }
    }];
    if d.night && !complete {
        chips.push(CourtChip {
            text: "DE NOCHE".into(),
            tone: 0,
        });
    }
    chip_row(font, v, r.x + 274.0, r.y + 276.0, &chips, 19.0);

    // the next date
    fill(
        v,
        slanted(r.x + 24.0, r.y + 356.0, r.w - 48.0, 2.0, 0.0),
        rgba(mix(color, 0x000000, 0.4), 1.0),
    );
    let label = if complete {
        "TODAS LAS CITAS"
    } else {
        "SIGUIENTE CITA"
    };
    text_shadow(
        font,
        v,
        Face::Ui,
        label,
        r.x + 30.0,
        r.y + 372.0,
        18.0,
        rgba(mix(color, 0xffffff, 0.35), 1.0),
    );
    let size = fitted_size(font, Face::Title, &d.next, 42.0, r.w - 60.0);
    text_outline(
        font,
        v,
        Face::Title,
        &d.next,
        r.x + 30.0,
        r.y + 398.0,
        size,
        rgba(BONE, 1.0),
        rgba(0x000000, 0.85),
    );
    let size = fitted_size(font, Face::Italic, &d.place, 23.0, r.w - 60.0);
    text_shadow(
        font,
        v,
        Face::Italic,
        &d.place,
        r.x + 32.0,
        r.y + 456.0,
        size,
        rgba(CREAM, 0.9),
    );

    // can it be had now? why not?
    let plate = slanted(r.x + 26.0, r.y + 500.0, r.w - 52.0, 64.0, 10.0);
    if waiting {
        fill(v, plate, rgba(0x2a1020, 0.95));
        rim(v, plate, 1.5, rgba(CRIMSON, 0.9));
        let mut size = 21.0;
        let mut lines = wrap(font, Face::Body, &d.why, size, r.w - 110.0);
        if lines.len() > 2 {
            size = 17.0;
            lines = wrap(font, Face::Body, &d.why, size, r.w - 110.0);
        }
        let top = r.y + 532.0 - lines.len().min(2) as f32 * size * 0.68;
        for (k, line) in lines.iter().take(2).enumerate() {
            text_shadow(
                font,
                v,
                Face::Body,
                line,
                r.x + 50.0,
                top + k as f32 * size * 1.3,
                size,
                rgba(CREAM, 1.0),
            );
        }
    } else {
        let (back, words) = if complete && d.nights {
            (GOOD, "LAS NOCHES PUEDEN IR A MÁS")
        } else if complete {
            (GOOD, "RUTA COMPLETA")
        } else {
            (color, "ENTER · ¡A LA CITA!")
        };
        fill(v, plate, rgba(back, if lit { 1.0 } else { 0.82 }));
        let size = fitted_size(font, Face::Display, words, 28.0, r.w - 110.0);
        text(
            font,
            v,
            Face::Display,
            words,
            r.x + 50.0,
            r.y + 516.0,
            size,
            rgba(INK_DARK, 1.0),
        );
    }

    // her memories
    let got = d.memories.iter().filter(|(_, open)| *open).count();
    text_shadow(
        font,
        v,
        Face::Display,
        &format!("RECUERDOS · {got} / {}", d.memories.len()),
        r.x + 30.0,
        r.y + 584.0,
        20.0,
        rgba(mix(color, 0xffffff, 0.35), 1.0),
    );
    let count = d.memories.len().max(1) as f32;
    let gap = 8.0;
    let side = ((r.w - 56.0 - gap * (count - 1.0)) / count).min(96.0);
    for (m, (_, open)) in d.memories.iter().enumerate() {
        let q = Rect::new(
            r.x + 28.0 + m as f32 * (side + gap),
            r.y + 614.0,
            side,
            side,
        );
        fill(
            v,
            slanted(q.x + 5.0, q.y + 6.0, q.w, q.h, 5.0),
            rgba(0x000000, 0.5),
        );
        let uv = faces
            .dates
            .get(i)
            .and_then(|row| row.get(m))
            .copied()
            .flatten();
        match (open, uv) {
            (true, Some(uv)) => {
                let tint = if lit {
                    [1.0; 4]
                } else {
                    [0.86, 0.84, 0.9, 1.0]
                };
                cutout(v, q, uv, tint);
                rim(v, slanted(q.x, q.y, q.w, q.h, 0.0), 2.0, rgba(BONE, 1.0));
            }
            _ => {
                fill(v, slanted(q.x, q.y, q.w, q.h, 0.0), rgba(0x150d1c, 1.0));
                halo(
                    v,
                    [q.x + q.w * 0.5, q.y + q.h * 0.5],
                    side * 0.33,
                    26.0,
                    rgba(color, 0.12),
                );
                let qs = side * 0.5;
                let w = text_width(font, Face::Display, "?", qs);
                text(
                    font,
                    v,
                    Face::Display,
                    "?",
                    q.x + (q.w - w) * 0.5,
                    q.y + side * 0.2,
                    qs,
                    rgba(mix(color, 0x000000, 0.45), 1.0),
                );
                rim(
                    v,
                    slanted(q.x, q.y, q.w, q.h, 0.0),
                    1.5,
                    rgba(mix(color, 0x000000, 0.4), 1.0),
                );
            }
        }
    }

    // the card's edge: bright while lit
    rim(
        v,
        slanted(r.x, r.y, r.w, r.h, lean),
        if lit { 3.0 } else { 1.5 },
        rgba(if lit { BONE } else { mix(color, 0x000000, 0.3) }, 1.0),
    );
    if waiting && !lit {
        fill(v, slanted(r.x, r.y, r.w, r.h, lean), rgba(0x000000, 0.18));
    }
}

fn draw_talk(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    accent: u32,
    age: f32,
    time: f32,
) {
    let k = entrance(age, 0.0, 0.35);
    faded(v, k, |v| {
        framed(v, BUST, faces.portrait, accent, true, time);
    });
    let has_card = !view.cards.is_empty();
    let say = Rect::new(SAY_X, TOP_Y, SAY_W, if has_card { 320.0 } else { 420.0 });
    faded(v, entrance(age, 0.1, 0.35), |v| {
        words_panel(
            font,
            v,
            say,
            &view.speaker,
            &view.role,
            &view.text,
            accent,
            31.0,
            if has_card { 4 } else { 7 },
            typed(age, 0.2),
        );
    });
    // A gift learned: its card.
    if let Some(card) = view.cards.first() {
        let r = Rect::new(SAY_X + 120.0, TOP_Y + 370.0, SAY_W - 240.0, 250.0);
        let k = entrance(age, 0.4, 0.5);
        shifted(v, [0.0, (1.0 - k) * 50.0], |v| {
            faded(v, k, |v| {
                let q = slanted(r.x, r.y, r.w, r.h, 14.0);
                soft(
                    v,
                    r,
                    14.0,
                    40.0,
                    rgba(accent, 0.45 + 0.12 * wave(time, 0.7)),
                    true,
                );
                fill(v, q, rgba(mix(accent, 0x000000, 0.55), 0.98));
                burst(
                    v,
                    [r.x + 110.0, r.y + r.h * 0.5],
                    52.0,
                    84.0,
                    12,
                    time * 0.3,
                    rgba(GOLD_HI, 0.95),
                    rgba(INK_DARK, 0.9),
                    4.0,
                );
                rune(
                    v,
                    card_rune(card).unwrap_or(1),
                    [r.x + 110.0, r.y + r.h * 0.5],
                    70.0,
                    6.0,
                    rgba(INK_DARK, 1.0),
                );
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &card.title,
                    r.x + 220.0,
                    r.y + 34.0,
                    40.0,
                    rgba(BONE, 1.0),
                    rgba(0x000000, 0.8),
                );
                for (j, l) in wrap(font, Face::Italic, &card.detail, 24.0, r.w - 260.0)
                    .iter()
                    .take(3)
                    .enumerate()
                {
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        l,
                        r.x + 222.0,
                        r.y + 96.0 + j as f32 * 30.0,
                        24.0,
                        rgba(CREAM, 0.95),
                    );
                }
                chip_row(font, v, r.x + 222.0, r.y + r.h - 56.0, &card.chips, 20.0);
                rim(v, q, 3.0, rgba(BONE, 1.0));
            });
        });
    }
}

fn draw_runes(font: &FontAtlas, v: &mut Vec<MapVertex>, view: &CourtView, accent: u32, time: f32) {
    let Some(r) = view.runes.as_ref() else {
        return;
    };
    // The circle where Nyxara traces them.
    let c = [960.0, 470.0];
    halo(
        v,
        c,
        230.0,
        90.0,
        rgba(accent, 0.18 + 0.06 * wave(time, 0.5)),
    );
    disc(v, c, 182.0, rgba(INK_DARK, 0.9));
    let ring_pts: Vec<[f32; 2]> = (0..=48)
        .map(|k| {
            let a = k as f32 / 48.0 * TAU + time * 0.15;
            [c[0] + a.cos() * 176.0, c[1] + a.sin() * 176.0]
        })
        .collect();
    for w in ring_pts.windows(2) {
        wire(
            v,
            w[0],
            w[1],
            3.0,
            rgba(accent, 0.8),
            rgba(accent, 0.8),
            0.0,
            false,
        );
    }
    match (r.lit, r.over) {
        (_, Some(won)) => {
            let color = if won { GOOD } else { CRIMSON };
            disc(v, c, 120.0, rgba(color, 0.25));
            let w = if won { "✓" } else { "✗" };
            let tw = text_width(font, Face::Title, w, 160.0);
            text_outline(
                font,
                v,
                Face::Title,
                w,
                c[0] - tw * 0.5,
                c[1] - 100.0,
                160.0,
                rgba(color, 1.0),
                rgba(0x000000, 0.9),
            );
        }
        (Some(lit), None) => {
            halo(v, c, 150.0, 60.0, rgba(LILAC, 0.45));
            rune(v, lit, c, 240.0, 16.0, rgba(LILAC, 1.0));
            rune(v, lit, c, 240.0, 6.0, rgba(0xffffff, 1.0));
        }
        (None, None) => {
            // Her turn: how far she is in the row.
            let n = r.count.max(1) as usize;
            for i in 0..n {
                let x = c[0] + (i as f32 - (n as f32 - 1.0) * 0.5) * 46.0;
                let done = (i as u8) < r.entered;
                disc(
                    v,
                    [x, c[1]],
                    15.0,
                    rgba(if done { LILAC } else { 0x3a2c48 }, 1.0),
                );
            }
            if r.showing {
                let tw = text_width(font, Face::Italic, "…", 60.0);
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    "…",
                    c[0] - tw * 0.5,
                    c[1] - 90.0,
                    60.0,
                    rgba(CREAM, 0.8),
                );
            }
        }
    }
    // What to do, under the circle.
    if !view.text.is_empty() {
        let tw = text_width(font, Face::Italic, &view.text, 26.0);
        text_shadow(
            font,
            v,
            Face::Italic,
            &view.text,
            960.0 - tw * 0.5,
            680.0,
            26.0,
            rgba(CREAM, 0.95),
        );
    }
    // Rounds and lives.
    let mut x = LEFT_X;
    for i in 0..r.rounds {
        let done = i < r.round;
        gem(
            v,
            [x + 20.0, 290.0],
            16.0,
            16.0,
            if done { GOLD_HI } else { 0x3a2c48 },
            1.0,
            if done { 0.6 } else { 0.0 },
        );
        x += 44.0;
    }
    text_shadow(
        font,
        v,
        Face::Ui,
        "Ronda",
        LEFT_X,
        312.0,
        22.0,
        rgba(CREAM, 0.9),
    );
    for i in 0..3u8 {
        let alive = i < r.lives;
        disc(
            v,
            [LEFT_X + 20.0 + i as f32 * 44.0, 380.0],
            15.0,
            rgba(if alive { CRIMSON } else { 0x3a2c48 }, 1.0),
        );
    }
    text_shadow(
        font,
        v,
        Face::Ui,
        "Fallos que te quedan",
        LEFT_X,
        402.0,
        22.0,
        rgba(CREAM, 0.9),
    );
    // The six runes to press.
    for i in 0..6usize {
        let (bc, br) = rune_button(i);
        let flash = r.flash.filter(|(f, _, _)| *f as usize == i);
        let ring = match flash {
            Some((_, true, _)) => GOOD,
            Some((_, false, _)) => CRIMSON,
            None => mix(accent, 0x000000, 0.2),
        };
        let her_turn = !r.showing && r.over.is_none();
        disc(v, [bc[0] + 6.0, bc[1] + 8.0], br, rgba(0x000000, 0.5));
        disc(
            v,
            bc,
            br,
            rgba(
                if flash.is_some() {
                    mix(ring, 0x000000, 0.5)
                } else {
                    0x150d1e
                },
                0.97,
            ),
        );
        let pts: Vec<[f32; 2]> = (0..=32)
            .map(|k| {
                let a = k as f32 / 32.0 * TAU;
                [bc[0] + a.cos() * br, bc[1] + a.sin() * br]
            })
            .collect();
        for w in pts.windows(2) {
            wire(
                v,
                w[0],
                w[1],
                if flash.is_some() { 6.0 } else { 3.0 },
                rgba(ring, 1.0),
                rgba(ring, 1.0),
                0.0,
                false,
            );
        }
        rune(
            v,
            i as u8,
            bc,
            78.0,
            6.0,
            rgba(if her_turn { LILAC } else { 0x7a6a8a }, 1.0),
        );
        keycap(
            font,
            v,
            Rect::new(bc[0] - 20.0, bc[1] + br + 14.0, 40.0, 40.0),
            &format!("{}", i + 1),
            18.0,
            flash.is_some(),
            !her_turn,
            accent,
            1.0,
        );
    }
}

fn draw_duel(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    accent: u32,
    time: f32,
) {
    let Some(d) = view.duel.as_ref() else {
        return;
    };
    // Nemi at the right, her words under her.
    framed(
        v,
        Rect::new(1380.0, 236.0, 430.0, 430.0),
        faces.portrait,
        accent,
        true,
        time,
    );
    if !view.text.is_empty() {
        speech_box(
            font,
            v,
            [1330.0, 900.0],
            520.0,
            &view.speaker,
            &view.text,
            accent,
            -0.02,
            1.0,
            1.0,
        );
    }
    // The score: parried out of all, and the mark a good session needs.
    let bar = Rect::new(LEFT_X, 250.0, 1160.0, 22.0);
    fill(
        v,
        slanted(bar.x, bar.y, bar.w, bar.h, 6.0),
        rgba(0x1a1022, 0.95),
    );
    let total = d.total.max(1) as f32;
    let k = d.parried as f32 / total;
    if k > 0.0 {
        fill(
            v,
            slanted(bar.x, bar.y, bar.w * k, bar.h, 6.0),
            rgba(GOOD, 1.0),
        );
    }
    let hk = d.hit as f32 / total;
    if hk > 0.0 {
        fill(
            v,
            slanted(bar.x + bar.w * (1.0 - hk), bar.y, bar.w * hk, bar.h, 6.0),
            rgba(CRIMSON, 0.9),
        );
    }
    let mark = bar.x + bar.w * d.good as f32 / total;
    wire(
        v,
        [mark, bar.y - 10.0],
        [mark, bar.y + bar.h + 10.0],
        4.0,
        rgba(GOLD_HI, 1.0),
        rgba(GOLD_HI, 1.0),
        0.0,
        false,
    );
    // The three targets, the blows coming at them.
    let keys = ["A", "D", "W"];
    for lane in 0..3u8 {
        let c = target(lane);
        let col = LANE_COLOURS[lane as usize];
        halo(v, c, TARGET_R + 30.0, 40.0, rgba(col, 0.16));
        disc(v, c, TARGET_R, rgba(0x120a1a, 0.92));
        let pts: Vec<[f32; 2]> = (0..=40)
            .map(|k| {
                let a = k as f32 / 40.0 * TAU;
                [c[0] + a.cos() * TARGET_R, c[1] + a.sin() * TARGET_R]
            })
            .collect();
        for w in pts.windows(2) {
            wire(
                v,
                w[0],
                w[1],
                5.0,
                rgba(col, 1.0),
                rgba(col, 1.0),
                0.0,
                false,
            );
        }
        let kw = text_width(font, Face::Title, keys[lane as usize], 30.0);
        text_outline(
            font,
            v,
            Face::Title,
            keys[lane as usize],
            c[0] - kw * 0.5,
            c[1] - 20.0,
            30.0,
            rgba(BONE, 1.0),
            rgba(0x000000, 0.9),
        );
    }
    for &(lane, permille) in &d.incoming {
        let c = target(lane);
        let k = permille as f32 / 1000.0;
        let r = TARGET_R + (1.0 - k).max(0.0) * 260.0;
        let col = LANE_COLOURS[lane as usize % 3];
        let near = (k - 0.8).max(0.0) / 0.2;
        let pts: Vec<[f32; 2]> = (0..=48)
            .map(|j| {
                let a = j as f32 / 48.0 * TAU;
                [c[0] + a.cos() * r, c[1] + a.sin() * r]
            })
            .collect();
        for w in pts.windows(2) {
            wire(
                v,
                w[0],
                w[1],
                4.0 + near * 6.0,
                rgba(mix(0xffffff, col, near.min(1.0)), 0.95),
                rgba(mix(0xffffff, col, near.min(1.0)), 0.95),
                0.0,
                false,
            );
        }
        // Nemi's wooden sword, coming.
        let from = match lane {
            0 => [c[0] - r, c[1]],
            1 => [c[0] + r, c[1]],
            _ => [c[0], c[1] - r],
        };
        disc(v, from, 12.0, rgba(0xd9b07a, 1.0));
    }
    if let Some((parried, lane, ms)) = d.last {
        let c = target(lane);
        let t = ms as f32 / 1000.0;
        faded(v, (1.0 - (t - 0.4).max(0.0) / 0.3).clamp(0.0, 1.0), |v| {
            comic_stamp(
                font,
                v,
                Face::Display,
                if parried { "¡PARADA!" } else { "¡AY!" },
                [c[0], c[1] - TARGET_R - 50.0],
                54.0,
                if parried { GOOD } else { CRIMSON },
                false,
                (t / 0.25).min(1.0),
                time,
            );
        });
    }
}
