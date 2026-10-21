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
use super::stage_overlay::{PictureInfo, PictureRun};
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
    /// The dates (the great hall's sofas): a card for each girl.
    pub dates: Vec<DateView>,
    /// One of the dates' memories shown big.
    pub show: Option<MemoryShow>,
    /// Pictures to load ahead (the lit girl's memories).
    pub preload: Vec<String>,
}

/// A memory of the dates shown big: its shot, name and line, whose it is (her name and colour) and
/// which of hers (`index` of `count`), and since when (ms).
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct MemoryShow {
    pub image: String,
    pub title: String,
    pub line: String,
    pub girl: String,
    pub color: u32,
    pub index: u8,
    pub count: u8,
    pub age_ms: u32,
}

/// What the pointer test says over a memory shown big.
pub const MEMORY_SHOWN: usize = 999;

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
    /// Chiquita's word on her card: why the date waits and how her affection rises, or what to
    /// do now.
    pub say: String,
    /// Her sprite (under `data/assets`), cut out at the left while she is lit.
    pub sprite: String,
    /// The memories her next date gives (0: none left).
    pub reward: u8,
    /// Her dates in order: whether each is a night's.
    pub route: Vec<bool>,
    pub memories: Vec<(PortraitRef, bool)>,
    /// The memory lit when the keys are on her row, and its name.
    pub focus: Option<u8>,
    pub focus_title: String,
    /// Back from a date with her: her affection, dates had and memories had before it.
    pub fresh: Option<(i32, u8, u8)>,
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
        // the value shrinks to keep clear of its label (a longer language)
        let room = w - 70.0 - text_width(font, Face::Ui, label, 22.0) - 18.0;
        let size = fitted_size(font, Face::Title, value, 28.0, room);
        let vw = text_width(font, Face::Title, value, size);
        text_outline(
            font,
            v,
            Face::Title,
            value,
            x + w - 36.0 - vw,
            ry + (28.0 - size) * 0.5,
            size,
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
        11 => date_hit(view, x, y),
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
    // A memory of the dates shown big has the screen to itself.
    if let Some(show) = view.show.as_ref().filter(|_| view.kind == 11) {
        draw_memory_show(font, v, width, height, view, show);
        return;
    }
    backdrop(v, width, height, accent, deep, time);
    // (the dates' page: the lit girl stands at the left, the title over her rows)
    let title_x = if view.kind == 11 { DATE_X } else { LEFT_X };
    heading(font, v, title_x, 46.0, &view.title, &view.tag, accent);
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
        // (on the dates' page, over the rows: its foot is her card's buttons)
        let (x, y) = if view.kind == 11 {
            (DATE_X + 10.0, 192.0)
        } else {
            ((width - w) * 0.5, height - 132.0)
        };
        fill(v, slanted(x, y, w, 44.0, 10.0), rgba(0x0b0610, 0.92));
        rim(v, slanted(x, y, w, 44.0, 10.0), 1.5, rgba(accent, 0.9));
        text_shadow(
            font,
            v,
            Face::Italic,
            &view.notice,
            x + 30.0,
            y + 8.0,
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
    // (the dates' page puts its keys over the lit girl's figure, in `draw_top`)
    if view.kind != 11 {
        key_row(font, v, LEFT_X, height - 66.0, &keys, accent);
    }
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

// The dates' page, laid out as the shop is (the game's Persona pages): the lit girl cut out at the
// left with her name thrown across her (her sprite drawn sharp by `pictures`, her name in `draw_top`),
// the girls on tilted rows, her memories on tiles under them, and her card at the right.
const DATE_TILT: f32 = -0.025;
const DATE_X: f32 = 650.0;
const DATE_ROW_W: f32 = 540.0;
const DATE_CARD: Rect = Rect::new(1206.0, 246.0, 680.0, 736.0);
const DATE_GO: Rect = Rect::new(1228.0, 914.0, 330.0, 54.0);
const DATE_MEMORIES: Rect = Rect::new(1574.0, 914.0, 290.0, 54.0);
/// What the pointer test says over the card's buttons: the date, her memories.
pub const DATE_GO_HIT: usize = 200;
pub const DATE_MEMORIES_HIT: usize = 201;
/// The lit girl's sprite: its height on the page and where its middle stands.
const DATE_SPRITE_H: f32 = 1180.0;
const DATE_SPRITE_X: f32 = 300.0;
const DATE_SPRITE_Y: f32 = 150.0;

/// Girl `i` of `n`'s row.
fn date_row(i: usize, n: usize) -> Rect {
    let step = (336.0 / n.max(1) as f32).min(112.0);
    Rect::new(DATE_X, 250.0 + i as f32 * step, DATE_ROW_W, step - 12.0)
}

/// Memory `m` of `count` on its tile, three to a row under the girls.
fn memory_tile(m: usize, count: usize) -> Rect {
    let rows = count.div_ceil(3).max(1) as f32;
    let side = ((316.0 - 14.0 * (rows - 1.0)) / rows).min(150.0);
    let (c, r) = ((m % 3) as f32, (m / 3) as f32);
    let x0 = DATE_X + (DATE_ROW_W - (side * 3.0 + 20.0 * 2.0)) * 0.5;
    Rect::new(x0 + c * (side + 20.0), 662.0 + r * (side + 14.0), side, side)
}

fn date_colour(d: &DateView) -> u32 {
    if d.color == 0 {
        GOLD
    } else {
        d.color
    }
}

/// What is under the pointer on the dates' page (in 1920 × 1080): a girl's row, one of the lit
/// girl's memories (100 + 16 × the girl + the memory), the card's buttons, or the memory shown big
/// ([`MEMORY_SHOWN`]).
fn date_hit(view: &CourtView, x: f32, y: f32) -> Option<usize> {
    if view.show.is_some() {
        return Some(MEMORY_SHOWN);
    }
    let inside = |r: Rect| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
    let n = view.dates.len();
    let lit = view.choice as usize;
    if let Some(d) = view.dates.get(lit) {
        let count = d.memories.len().min(16);
        if let Some(m) = (0..count).find(|&m| inside(memory_tile(m, count))) {
            return Some(100 + lit * 16 + m);
        }
    }
    if inside(DATE_GO) {
        return Some(DATE_GO_HIT);
    }
    if inside(DATE_MEMORIES) {
        return Some(DATE_MEMORIES_HIT);
    }
    (0..n).find(|&i| inside(date_row(i, n)))
}

/// The red cursor at the left end of a row turned by `angle`.
fn row_cursor(v: &mut Vec<MapVertex>, r: Rect, angle: f32, time: f32) {
    let (sn, cs) = angle.sin_cos();
    let half = r.w * 0.5;
    let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
    cursor(v, [c[0] - half * cs - 14.0, c[1] - half * sn], 30.0, time);
}

/// A pill of words on a colour.
fn pill(font: &FontAtlas, v: &mut Vec<MapVertex>, x: f32, y: f32, words: &str, back: u32, ink: u32) -> f32 {
    // (signs read better in the titles' face than in the shouting one)
    let face = if words.starts_with('+') {
        Face::Title
    } else {
        Face::Display
    };
    let w = text_width(font, face, words, 18.0) + 28.0;
    panel(
        v,
        Rect::new(x - 2.0, y - 2.0, w + 4.0, 34.0),
        17.0,
        Panel::fill(INK_DARK, INK_DARK, 1.0),
    );
    panel(
        v,
        Rect::new(x, y, w, 30.0),
        15.0,
        Panel::fill(back, mix(back, 0x000000, 0.25), 1.0),
    );
    text(font, v, face, words, x + 14.0, y + 3.0, 18.0, rgba(ink, 1.0));
    w
}

/// A button of the card: black-edged, her colour when it can be used and `lit`, greyed when not.
#[allow(clippy::too_many_arguments)]
fn date_button(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    label: &str,
    lit: bool,
    enabled: bool,
    color: u32,
) {
    turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], DATE_TILT, |v| {
        panel(
            v,
            Rect::new(r.x + 8.0, r.y + 10.0, r.w, r.h),
            8.0,
            Panel::fill(SHADOW, SHADOW, 0.7),
        );
        panel(v, grow(r, 3.0), 9.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
        let (top, bottom, rim) = if !enabled {
            (0x241c2a, 0x120e16, 0x5a4e62)
        } else if lit {
            (mix(color, 0xffffff, 0.1), mix(color, 0x000000, 0.45), GOLD_HI)
        } else {
            (0x2a1a36, 0x100a16, GOLD_DEEP)
        };
        panel(v, r, 7.0, Panel::fill(top, bottom, 0.98).rim(rim, 1.0, 2.0).bevel(0.6));
        let size = fitted_size(font, Face::Title, label, 28.0, r.w - 24.0);
        let w = text_width(font, Face::Title, label, size);
        text_outline(
            font,
            v,
            Face::Title,
            label,
            r.x + (r.w - w) * 0.5,
            r.y + (r.h - size) * 0.5 - 2.0,
            size,
            rgba(if enabled { 0xfff6ee } else { 0x9a8ea2 }, 1.0),
            rgba(INK_DARK, 0.8),
        );
    });
}

/// The dates' page: a band of the lit girl's colour where her name lands, the girls' rows, her
/// memories and her card. Her figure and her name are drawn over all this (`pictures`, `draw_top`).
fn draw_dates(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &CourtView,
    faces: &CourtFaces,
    time: f32,
) {
    let age = view.page_ms as f32 / 1000.0;
    let flip = view.flip_ms as f32 / 1000.0;
    let n = view.dates.len();
    let lit = (view.choice as usize).min(n.saturating_sub(1));
    let Some(d) = view.dates.get(lit) else {
        return;
    };
    let color = date_colour(d);
    // her band, low across the left, under her name
    let k = entrance(flip.min(age), 0.0, 0.3);
    shifted(v, [-(1.0 - k) * 900.0, 0.0], |v| {
        slash(
            v,
            [260.0, 905.0],
            1500.0,
            170.0,
            -0.09,
            rgba(mix(color, 0x000000, 0.25), 0.95),
            rgba(mix(color, 0x000000, 0.7), 0.97),
        );
        slash(
            v,
            [260.0, 990.0],
            1500.0,
            14.0,
            -0.09,
            rgba(INK_DARK, 1.0),
            rgba(INK_DARK, 1.0),
        );
    });

    // the girls
    for (i, g) in view.dates.iter().enumerate() {
        let r = date_row(i, n);
        let on = i == lit;
        let gc = date_colour(g);
        let state = view.cards.get(i).map_or(0, |c| c.state);
        let a = entrance(age, 0.06 + 0.04 * i as f32, 0.26);
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 260.0, 0.0], |v| {
                turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], DATE_TILT, |v| {
                    panel(
                        v,
                        Rect::new(r.x + 10.0, r.y + 12.0, r.w, r.h),
                        9.0,
                        Panel::fill(SHADOW, SHADOW, 0.7),
                    );
                    panel(v, grow(r, 4.0), 10.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
                    let (top, bottom, rim) = if on {
                        (mix(gc, 0x000000, 0.3), mix(gc, 0x000000, 0.78), GOLD_HI)
                    } else {
                        (0x24182e, 0x0c0712, GOLD_DEEP)
                    };
                    panel(
                        v,
                        r,
                        8.0,
                        Panel::fill(top, bottom, 0.98).rim(rim, 1.0, 2.2).bevel(0.6),
                    );
                    panel(
                        v,
                        Rect::new(r.x, r.y, 10.0, r.h),
                        3.0,
                        Panel::fill(mix(gc, 0xffffff, 0.25), gc, 1.0),
                    );
                    // her face
                    let side = r.h - 20.0;
                    let face = Rect::new(r.x + 24.0, r.y + 10.0, side, side);
                    panel(v, grow(face, 3.0), 5.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
                    match faces.cards.get(i).copied().flatten() {
                        Some(uv) => cutout(v, face, uv, [1.0; 4]),
                        None => panel(v, face, 4.0, Panel::fill(gc, mix(gc, 0x000000, 0.6), 1.0)),
                    }
                    // her name, and her next date
                    let tx = face.x + face.w + 18.0;
                    let size = fitted_size(font, Face::Title, &g.name, 34.0, 220.0);
                    text_outline(
                        font,
                        v,
                        Face::Title,
                        &g.name,
                        tx,
                        r.y + 10.0,
                        size,
                        rgba(0xfff6ee, 1.0),
                        rgba(INK_DARK, 0.85),
                    );
                    let size = fitted_size(font, Face::Italic, &g.next, 20.0, 250.0);
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        &g.next,
                        tx + 2.0,
                        r.y + 52.0,
                        size,
                        rgba(CREAM, 0.92),
                    );
                    // whether she can be asked out now
                    let (words, back, ink) = match state {
                        0 => ("¡LISTA!", GOOD, INK_DARK),
                        2 => ("COMPLETA", GOLD_HI, INK_DARK),
                        _ => ("ESPERA", CRIMSON, 0xffffff),
                    };
                    let w = text_width(font, Face::Display, words, 18.0) + 28.0;
                    pill(font, v, r.x + r.w - w - 20.0, r.y + 12.0, words, back, ink);
                    if state != 2 {
                        let count = format!("CITA {} / {}", g.done + 1, g.total);
                        let cw = text_width(font, Face::Display, &count, 18.0);
                        text_shadow(
                            font,
                            v,
                            Face::Display,
                            &count,
                            r.x + r.w - cw - 22.0,
                            r.y + 54.0,
                            18.0,
                            rgba(GOLD_HI, 1.0),
                        );
                    }
                });
            });
        });
        if on && a > 0.95 && d.focus.is_none() {
            row_cursor(v, r, DATE_TILT, time);
        }
    }

    // her memories, on tiles
    let got = d.memories.iter().filter(|(_, open)| *open).count();
    let label = match d.focus {
        Some(_) => format!("RECUERDO · {}", d.focus_title),
        None => format!("RECUERDOS · {got} / {}", d.memories.len()),
    };
    let size = fitted_size(font, Face::Display, &label, 26.0, DATE_ROW_W - 40.0);
    text_outline(
        font,
        v,
        Face::Display,
        &label,
        DATE_X + 4.0,
        614.0,
        size,
        rgba(if d.focus.is_some() { GOLD_HI } else { 0xfff6ee }, 1.0),
        rgba(INK_DARK, 0.85),
    );
    let count = d.memories.len().min(16);
    let new_from = d.fresh.map_or(usize::MAX, |(_, _, m)| m as usize);
    let k = entrance(flip.min(age), 0.05, 0.28);
    for (m, (_, open)) in d.memories.iter().take(count).enumerate() {
        let q = memory_tile(m, count);
        let focused = d.focus == Some(m as u8);
        let uv = faces.dates.get(lit).and_then(|row| row.get(m)).copied().flatten();
        faded(v, k, |v| {
            shifted(v, [0.0, (1.0 - k) * 40.0], |v| {
                turned(v, [q.x + q.w * 0.5, q.y + q.h * 0.5], DATE_TILT, |v| {
                    panel(
                        v,
                        Rect::new(q.x + 8.0, q.y + 10.0, q.w, q.h),
                        6.0,
                        Panel::fill(SHADOW, SHADOW, 0.7),
                    );
                    panel(v, grow(q, 4.0), 7.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
                    match (open, uv) {
                        (true, Some(uv)) => cutout(v, q, uv, [1.0; 4]),
                        _ => {
                            panel(v, q, 5.0, Panel::fill(0x24182e, 0x0c0712, 1.0));
                            let qs = q.w * 0.42;
                            let w = text_width(font, Face::Title, "?", qs);
                            text(
                                font,
                                v,
                                Face::Title,
                                "?",
                                q.x + (q.w - w) * 0.5,
                                q.y + (q.h - qs) * 0.5 - qs * 0.1,
                                qs,
                                rgba(mix(color, 0x000000, 0.35), 1.0),
                            );
                        }
                    }
                    panel(
                        v,
                        q,
                        5.0,
                        Panel::fill(0, 0, 0.0).rim(
                            if focused { GOLD_HI } else { GOLD_DEEP },
                            1.0,
                            if focused { 4.0 } else { 2.0 },
                        ),
                    );
                    // a memory the date just had brought
                    if *open && m >= new_from {
                        pill(font, v, q.x - 6.0, q.y - 12.0, "NUEVO", CRIMSON, 0xffffff);
                    }
                });
            });
        });
        if focused {
            row_cursor(v, q, DATE_TILT, time);
        }
    }

    // her card
    let k = entrance(flip.min(age), 0.0, 0.25).min(entrance(age, 0.12, 0.3));
    let state = view.cards.get(lit).map_or(0, |c| c.state);
    faded(v, k, |v| {
        shifted(v, [(1.0 - k) * 120.0, 0.0], |v| {
            date_card(font, v, d, state, age);
        });
    });
}

/// The lit girl's card: her next date's name on a white plate, when and where, her route, her
/// affection against its mark, Chiquita's word on it, and the buttons.
fn date_card(font: &FontAtlas, v: &mut Vec<MapVertex>, d: &DateView, state: u8, age: f32) {
    let r = DATE_CARD;
    let color = date_colour(d);
    let light = mix(color, 0xffffff, 0.4);
    let complete = state == 2;
    panel(
        v,
        Rect::new(r.x + 12.0, r.y + 16.0, r.w, r.h),
        14.0,
        Panel::fill(SHADOW, SHADOW, 0.8),
    );
    panel(v, grow(r, 5.0), 16.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
    panel(
        v,
        r,
        12.0,
        Panel::fill(mix(color, 0x0d0814, 0.86), 0x07040b, 0.97)
            .rim(GOLD, 1.0, 2.2)
            .hairline(7.0, 0.28)
            .bevel(0.8),
    );
    panel(
        v,
        Rect::new(r.x, r.y, r.w, 8.0),
        4.0,
        Panel::fill(color, mix(color, 0x000000, 0.4), 1.0),
    );
    let x = r.x + 30.0;
    text_shadow(
        font,
        v,
        Face::Ui,
        if complete {
            "TODAS LAS CITAS"
        } else {
            "SIGUIENTE CITA"
        },
        x,
        r.y + 24.0,
        19.0,
        rgba(light, 1.0),
    );
    // the date's name on a white plate
    let size = fitted_size(font, Face::Title, &d.next, 40.0, 580.0);
    let w = (text_width(font, Face::Title, &d.next, size) + 44.0).min(620.0);
    let plate = Rect::new(x - 6.0, r.y + 54.0, w, 62.0);
    turned(v, [plate.x + plate.w * 0.5, plate.y + plate.h * 0.5], -0.03, |v| {
        panel(v, grow(plate, 3.0), 8.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
        panel(v, plate, 6.0, Panel::fill(0xfffaf4, 0xe8e0d2, 1.0));
        text(
            font,
            v,
            Face::Title,
            &d.next,
            plate.x + 22.0,
            plate.y + (plate.h - size) * 0.5 - 2.0,
            size,
            rgba(0x1a0c16, 1.0),
        );
    });
    // where and when
    let size = fitted_size(font, Face::Italic, &d.place, 24.0, 610.0);
    text_shadow(
        font,
        v,
        Face::Italic,
        &d.place,
        x,
        r.y + 136.0,
        size,
        rgba(CREAM, 0.95),
    );
    // her route: a block for each date (had, the next, still to come; a night's with a moon)
    text_shadow(font, v, Face::Title, "La ruta", x, r.y + 186.0, 24.0, rgba(GOLD_HI, 1.0));
    let total = d.total.max(1) as usize;
    let fresh_at = d.fresh.map(|(_, before, _)| before as usize);
    for k in 0..total {
        let b = Rect::new(x + 120.0 + k as f32 * 98.0, r.y + 182.0, 86.0, 36.0);
        let done = k < d.done as usize;
        let next = k == d.done as usize && !complete;
        let (back, ink) = if done {
            (mix(color, 0xffffff, 0.2), INK_DARK)
        } else if next {
            (GOLD_HI, INK_DARK)
        } else {
            (0x24182e, mix(color, 0xffffff, 0.15))
        };
        // the date just had lands on its block
        let pop = match fresh_at {
            Some(f) if f == k => 1.0 + 0.25 * (1.0 - ease_out((age - 0.3) / 0.3)).max(0.0),
            _ => 1.0,
        };
        let b = Rect::new(
            b.x - b.w * (pop - 1.0) * 0.5,
            b.y - b.h * (pop - 1.0) * 0.5,
            b.w * pop,
            b.h * pop,
        );
        turned(v, [b.x + b.w * 0.5, b.y + b.h * 0.5], DATE_TILT * 2.0, |v| {
            panel(v, grow(b, 3.0), 5.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
            panel(v, b, 4.0, Panel::fill(back, mix(back, 0x000000, 0.3), 1.0));
            let night = d.route.get(k).copied().unwrap_or(false);
            let words = format!("{}", k + 1);
            let tw = text_width(font, Face::Display, &words, 22.0);
            // a night's date: its number beside a moon
            let tx = if night {
                b.x + b.w * 0.5 - tw - 4.0
            } else {
                b.x + (b.w - tw) * 0.5
            };
            text(
                font,
                v,
                Face::Display,
                &words,
                tx,
                b.y + (b.h - 22.0) * 0.5 - 2.0,
                22.0,
                rgba(ink, 1.0),
            );
            if night {
                let c = [b.x + b.w * 0.5 + 12.0, b.y + b.h * 0.5];
                disc(v, c, 9.0, rgba(ink, 1.0));
                disc(v, [c[0] + 4.0, c[1] - 3.5], 8.0, rgba(back, 1.0));
            }
        });
    }
    if d.night && !complete {
        pill(font, v, x, r.y + 236.0, "DE NOCHE", 0x2a1a44, 0xe8d8ff);
    }

    // her affection against the next date's mark (back from a date, the new hearts pop in)
    let ay = r.y + 270.0;
    wire(
        v,
        [x, ay - 14.0],
        [r.x + r.w - 30.0, ay - 14.0],
        2.0,
        rgba(GOLD_DEEP, 0.9),
        rgba(GOLD_DEEP, 0.0),
        0.0,
        false,
    );
    text_shadow(font, v, Face::Title, "Afecto", x, ay, 24.0, rgba(GOLD_HI, 1.0));
    let gained = d.fresh.map_or(0, |(before, _, _)| d.hearts - before.max(0));
    if gained > 0 && age > 0.5 {
        let lw = text_width(font, Face::Title, "Afecto", 24.0);
        pill(font, v, x + lw + 14.0, ay - 2.0, &format!("+{gained}"), GOLD_HI, INK_DARK);
    }
    let mark = if complete || d.need <= 0 {
        None
    } else {
        Some(d.need.min(10))
    };
    if let Some(need) = mark {
        let (words, ink) = if d.hearts >= need {
            ("LISTA PARA LA SIGUIENTE".to_owned(), GOOD)
        } else {
            (format!("FALTAN {}", need - d.hearts.max(0)), 0xff9aa8)
        };
        let size = fitted_size(font, Face::Display, &words, 20.0, 300.0);
        let w = text_width(font, Face::Display, &words, size);
        text_shadow(
            font,
            v,
            Face::Display,
            &words,
            r.x + r.w - 32.0 - w,
            ay + 2.0,
            size,
            rgba(ink, 1.0),
        );
    }
    let full = if complete { 10 } else { d.hearts.clamp(0, 10) };
    let from = d.fresh.map_or(i32::MAX, |(before, _, _)| before.max(0));
    for h in 0..10 {
        let c = [x + 26.0 + h as f32 * 50.0, ay + 70.0];
        let ringed = mark == Some(h + 1);
        let new = h >= from && h < full;
        let at = 0.55 + 0.14 * (h - from.min(h)) as f32;
        let filled = h < full && (!new || age >= at);
        let pop = if new && filled {
            1.0 + 0.5 * (1.0 - ease_out((age - at) / 0.25))
        } else {
            1.0
        };
        heart(
            v,
            c,
            (if ringed { 23.0 } else { 20.0 }) * pop,
            rgba(if ringed { GOLD_HI } else { INK_DARK }, 1.0),
        );
        heart(
            v,
            c,
            16.5 * pop,
            if filled {
                rgba(mix(color, 0xffffff, 0.2), 1.0)
            } else {
                rgba(0x2e2238, 1.0)
            },
        );
    }

    // what the next date gives: its memories, still dark
    if d.reward > 0 {
        let ry = r.y + 400.0;
        wire(
            v,
            [x, ry - 14.0],
            [r.x + r.w - 30.0, ry - 14.0],
            2.0,
            rgba(GOLD_DEEP, 0.9),
            rgba(GOLD_DEEP, 0.0),
            0.0,
            false,
        );
        text_shadow(font, v, Face::Title, "Recompensa", x, ry, 24.0, rgba(GOLD_HI, 1.0));
        for k in 0..d.reward as usize {
            let q = Rect::new(x + 4.0 + k as f32 * 96.0, ry + 40.0, 82.0, 82.0);
            turned(v, [q.x + q.w * 0.5, q.y + q.h * 0.5], DATE_TILT * 2.0, |v| {
                panel(v, grow(q, 3.0), 6.0, Panel::fill(INK_DARK, INK_DARK, 1.0));
                panel(
                    v,
                    q,
                    5.0,
                    Panel::fill(0x24182e, 0x0c0712, 1.0).rim(GOLD_DEEP, 1.0, 2.0),
                );
                let w = text_width(font, Face::Title, "?", 40.0);
                text(
                    font,
                    v,
                    Face::Title,
                    "?",
                    q.x + (q.w - w) * 0.5,
                    q.y + 16.0,
                    40.0,
                    rgba(mix(color, 0x000000, 0.2), 1.0),
                );
            });
        }
        let words = if d.reward == 1 {
            "1 recuerdo para tu galería".to_owned()
        } else {
            format!("{} recuerdos para tu galería", d.reward)
        };
        text_shadow(
            font,
            v,
            Face::Italic,
            &words,
            x + 20.0 + d.reward as f32 * 96.0,
            ry + 66.0,
            22.0,
            rgba(CREAM, 0.95),
        );
    }

    // Chiquita's word on it (why it waits and how her affection rises, or what to do now)
    let words = if d.say.is_empty() { d.why.clone() } else { d.say.clone() };
    if !words.is_empty() {
        speech_box(
            font,
            v,
            [x - 4.0, 884.0],
            620.0,
            "Chiquita",
            &words,
            0xb07ae0,
            -0.012,
            1.0,
            0.84,
        );
    }

    // the buttons
    let (label, enabled) = match state {
        0 => ("¡A la cita! · Enter", true),
        2 if d.nights => ("Invítala de noche", false),
        2 => ("Ruta completa", false),
        _ => ("¡A la cita!", false),
    };
    date_button(font, v, DATE_GO, label, true, enabled, color);
    let got = d.memories.iter().any(|(_, open)| *open);
    date_button(font, v, DATE_MEMORIES, "Recuerdos · S", false, got, color);
}

/// Over the lit girl's figure (drawn after the page's shapes): her name thrown across her and what
/// she is, and the keys at the foot.
pub(crate) fn draw_top(font: &FontAtlas, v: &mut Vec<MapVertex>, view: &CourtView, time: f32) {
    let _ = time;
    if view.kind != 11 || view.show.is_some() {
        return;
    }
    let lit = view.choice as usize;
    let Some(d) = view.dates.get(lit) else {
        return;
    };
    let color = date_colour(d);
    let age = view.page_ms as f32 / 1000.0;
    let na = entrance((view.flip_ms as f32 / 1000.0).min(age), 0.12, 0.3);
    let base = [40.0, 846.0];
    faded(v, na, |v| {
        shifted(v, [(1.0 - na) * -80.0, 0.0], |v| {
            turned(v, base, -0.09, |v| {
                let size = fitted_size(font, Face::Title, &d.name, 110.0, 600.0);
                shout(
                    font,
                    v,
                    Face::Title,
                    &d.name,
                    base[0],
                    base[1],
                    size,
                    0xffffff,
                    mix(color, 0x000000, 0.45),
                    1.0,
                );
                let size = fitted_size(font, Face::Italic, &d.role, 28.0, 560.0);
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    &d.role,
                    base[0] + 14.0,
                    base[1] + 120.0,
                    size,
                    rgba(mix(color, 0xffffff, 0.7), 1.0),
                    rgba(INK_DARK, 0.9),
                );
            });
        });
    });
    let keys: Vec<(&str, &str)> = view
        .keys
        .iter()
        .map(|(k, w)| (k.as_str(), w.as_str()))
        .collect();
    key_row(font, v, DATE_X, 1014.0, &keys, color);
}

/// A memory shown big (its shot is drawn under this, sharp, by [`pictures`]): the dark it comes
/// out of, a fall of shadow at the foot, whose it is and which of hers, its name and its line, and
/// the keys.
fn draw_memory_show(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &CourtView,
    s: &MemoryShow,
) {
    let t = s.age_ms as f32 / 1000.0;
    let color = if s.color == 0 { GOLD } else { s.color };
    // the dark at the foot, for the words: a few bands, each blending into the next
    let top = height * 0.58;
    let stops = [0.0, 0.18, 0.5, 0.8, 0.92];
    let band = (height - top) / (stops.len() - 1) as f32;
    for (j, pair) in stops.windows(2).enumerate() {
        let y = top + j as f32 * band;
        gradient(
            v,
            Rect::new(0.0, y, width, band),
            rgba(0x05030a, pair[0]),
            rgba(0x05030a, pair[1]),
        );
    }
    // out of the dark
    let k = entrance(t, 0.0, 0.45);
    if k < 1.0 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(0x000000, 1.0 - k),
        );
    }
    let k2 = entrance(t, 0.25, 0.45);
    shifted(v, [(1.0 - k2) * -50.0, 0.0], |v| {
        faded(v, k2, |v| {
            // whose: her name on a plate of her colour, and which of hers
            let name_w = text_width(font, Face::Display, &s.girl, 26.0) + 44.0;
            fill(
                v,
                slanted(LEFT_X, 752.0, name_w, 44.0, 10.0),
                rgba(color, 1.0),
            );
            text(
                font,
                v,
                Face::Display,
                &s.girl,
                LEFT_X + 22.0,
                757.0,
                26.0,
                rgba(INK_DARK, 1.0),
            );
            text_shadow(
                font,
                v,
                Face::Display,
                &format!("RECUERDO {} / {}", s.index, s.count),
                LEFT_X + name_w + 18.0,
                760.0,
                24.0,
                rgba(GOLD_HI, 1.0),
            );
            let size = fitted_size(font, Face::Title, &s.title, 64.0, 1600.0);
            text_outline(
                font,
                v,
                Face::Title,
                &s.title,
                LEFT_X,
                808.0,
                size,
                rgba(BONE, 1.0),
                rgba(0x000000, 0.9),
            );
            for (n, line) in wrap(font, Face::Italic, &s.line, 30.0, 1500.0)
                .iter()
                .take(2)
                .enumerate()
            {
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    line,
                    LEFT_X + 4.0,
                    892.0 + n as f32 * 38.0,
                    30.0,
                    rgba(CREAM, 1.0),
                );
            }
        })
    });
    let keys: Vec<(&str, &str)> = view
        .keys
        .iter()
        .map(|(k, w)| (k.as_str(), w.as_str()))
        .collect();
    key_row(font, v, LEFT_X, height - 66.0, &keys, color);
}

/// A rectangle shading from `top` down to `bottom`.
fn gradient(v: &mut Vec<MapVertex>, r: Rect, top: [f32; 4], bottom: [f32; 4]) {
    let at = |x: f32, y: f32, color: [f32; 4]| MapVertex {
        position: [x, y],
        color,
        ..MapVertex::BLANK
    };
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    v.extend([
        at(x0, y0, top),
        at(x1, y0, top),
        at(x1, y1, bottom),
        at(x0, y0, top),
        at(x1, y1, bottom),
        at(x0, y1, bottom),
    ]);
}

/// The pictures the court's page shows this frame (to load before drawing them).
pub(crate) fn shown(view: &CourtView) -> Vec<&str> {
    match view.show.as_ref() {
        Some(s) => vec![s.image.as_str()],
        None if view.kind == 11 => view
            .dates
            .get(view.choice as usize)
            .filter(|d| !d.sprite.is_empty())
            .map(|d| vec![d.sprite.as_str()])
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

/// The lit girl of the dates' page cut out at the left, sharp, over the page's shapes (her name
/// goes over her in `draw_top`); she slides in when she is lit.
fn sprite(
    view: &CourtView,
    info: &dyn Fn(&str) -> Option<PictureInfo>,
    width: f32,
    height: f32,
) -> Option<PictureRun> {
    if view.kind != 11 {
        return None;
    }
    let d = view.dates.get(view.choice as usize)?;
    let size = info(&d.sprite)?.size;
    let s = DATE_SPRITE_H / size[1].max(1.0);
    let age = view.page_ms as f32 / 1000.0;
    let k = entrance((view.flip_ms as f32 / 1000.0).min(age), 0.0, 0.3);
    let (w, h) = (size[0] * s, DATE_SPRITE_H);
    let x = DATE_SPRITE_X - w * 0.5 - (1.0 - k) * 160.0;
    let y = DATE_SPRITE_Y;
    let (sx, sy) = (width / 1920.0, height / 1080.0);
    let p = [
        [x * sx, y * sy],
        [(x + w) * sx, y * sy],
        [(x + w) * sx, (y + h) * sy],
        [x * sx, (y + h) * sy],
    ];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let vertices = [0usize, 1, 2, 0, 2, 3]
        .iter()
        .map(|&i| MapVertex {
            position: p[i],
            uv: uv[i],
            color: [1.0, 1.0, 1.0, k],
            text: 15.0,
            ..MapVertex::BLANK
        })
        .collect();
    Some(PictureRun {
        image: d.sprite.clone(),
        over: true,
        vertices,
    })
}

/// The court's pictures drawn sharp under its shapes: a memory of the dates' page shown big, over
/// the whole screen, pushing in slowly.
pub(crate) fn pictures(
    view: &CourtView,
    info: &dyn Fn(&str) -> Option<PictureInfo>,
    width: f32,
    height: f32,
) -> Vec<PictureRun> {
    let Some(s) = view.show.as_ref() else {
        return sprite(view, info, width, height).into_iter().collect();
    };
    let t = s.age_ms as f32 / 1000.0;
    let z = 1.0 + 0.035 * ease_out(t / 9.0);
    let (u0, u1) = (0.5 - 0.5 / z, 0.5 + 0.5 / z);
    let p = [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]];
    let uv = [[u0, u0], [u1, u0], [u1, u1], [u0, u1]];
    let vertices = [0usize, 1, 2, 0, 2, 3]
        .iter()
        .map(|&i| MapVertex {
            position: p[i],
            uv: uv[i],
            color: [1.0; 4],
            text: 15.0,
            ..MapVertex::BLANK
        })
        .collect();
    vec![PictureRun {
        image: s.image.clone(),
        over: false,
        vertices,
    }]
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
