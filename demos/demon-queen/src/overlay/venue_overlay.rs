//! Rosalía's book, in the manner of Persona's menus. Its first page deals the girls like a hand of
//! cards over slashes of the chosen one's colour: each card leaning its own way, her face large
//! with her name thrown over it and her number on a diamond, what she is like and gives, her
//! price, what can be asked of her and her room; the chosen one lit in her colour with the cursor
//! at her side, the house's name across the top, Rosalía and the purse beside it. Then what to ask
//! of the one chosen: her portrait cutting in, her name over it, what can be asked on tilted tiles
//! (the seal's shape locked until the elixir is drunk). Rosalía's offer: her portrait, her word in
//! a speech box, the elixir turning on its stage with its price, and at the card's foot a content
//! warning (the elixir is futa content) with a button to its leaflet. The leaflet is a medicine's,
//! side effects and all, on a folded sheet with Rosalía's stamp. Drawn in code with the overlay's
//! painter; only the faces are pictures. Rosalía's desk (a button on the first page) has her own
//! page: her portrait, the Queen's account with the house on a paper card, and tilted rows to pay,
//! work a debt off, see her ledger or borrow, and the house's jobs. A page's pieces slide in when it
//! opens; the keys and the pointer only change their light, never their place or size.
use std::cell::RefCell;
use std::rc::Rc;

use super::super::venue::{Asking, ClientCard, LoanAction, Menu, Page, OFFER_LEAFLET, ROSTER};
use super::persona::*;
use super::ui_paint::*;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Item(usize),
    Action,
    Close,
    /// A tile of what to ask.
    Ask(usize),
    /// The offer: buy and drink, or not now.
    Buy,
    Decline,
    /// The offer's «Efectos secundarios»: the elixir's leaflet.
    Leaflet,
    /// The elixir's chip on the girls' page.
    Elixir,
    /// A button of Rosalía's loan page, taken.
    Loan(usize),
    /// One of her promissory notes, picked (a second click signs it).
    LoanPick(usize),
    /// Rosalía's desk, from the first page.
    Desk,
    /// A row of her desk, taken.
    DeskRow(usize),
    /// A client's card: picked (a second click serves him).
    Client(usize),
}

type Uv = ([f32; 2], [f32; 2]);

/// The model the offer turns on its stage: the house's crystal heart.
pub(crate) const ELIXIR_MODEL: &str = "data/assets/3d/items/elixir_rosalia.glb";

/// How far each girl's card leans and how far down it sits: a hand of cards dealt on the table.
const CARD_TILT: [f32; 4] = [-0.04, 0.026, -0.03, 0.036];
const CARD_DROP: [f32; 4] = [0.0, 22.0, 6.0, 26.0];

/// Where everything is on the girls' page (1080p, scaled to the screen's height).
struct Layout {
    s: f32,
    cards: Vec<Rect>,
    choose: Rect,
    close: Rect,
    elixir: Rect,
    purse: Rect,
    host: Rect,
    /// Rosalía's desk.
    desk: Rect,
}

fn layout(w: f32, h: f32, count: usize) -> Layout {
    let s = h / 1080.0;
    let count = count.clamp(1, 4);
    let gap = 58.0 * s;
    let card_w = ((w - 320.0 * s - gap * (count as f32 - 1.0)) / count as f32).min(430.0 * s);
    let total = card_w * count as f32 + gap * (count as f32 - 1.0);
    let x0 = w * 0.5 - total * 0.5;
    let cards = (0..count)
        .map(|i| {
            Rect::new(
                x0 + i as f32 * (card_w + gap),
                (226.0 + CARD_DROP[i]) * s,
                card_w,
                584.0 * s,
            )
        })
        .collect();
    // The keys' row stops short of the screen's right, where Chiquita stands when she floats a line.
    let row = h - 112.0 * s;
    let close = Rect::new(w - 440.0 * s, row, 200.0 * s, 66.0 * s);
    let desk = Rect::new(close.x - 326.0 * s, row, 304.0 * s, 66.0 * s);
    let choose = Rect::new(desk.x - 466.0 * s, row, 440.0 * s, 66.0 * s);
    let purse = Rect::new(w - 300.0 * s, 46.0 * s, 196.0 * s, 60.0 * s);
    let elixir = Rect::new(
        purse.x + purse.w - 500.0 * s,
        purse.y + purse.h + 16.0 * s,
        500.0 * s,
        50.0 * s,
    );
    let host = Rect::new(w * 0.5 - 10.0 * s, 42.0 * s, 104.0 * s, 104.0 * s);
    Layout {
        s,
        cards,
        choose,
        close,
        elixir,
        purse,
        host,
        desk,
    }
}

/// The ask page: the portrait on the left, the tiles on the right.
struct AskLayout {
    s: f32,
    portrait: Rect,
    tiles: Vec<Rect>,
    head: [f32; 2],
    back: Rect,
}

/// How far the ask page's tiles lean.
const TILE_ANGLE: f32 = -0.05;

fn ask_layout(w: f32, h: f32, count: usize) -> AskLayout {
    let s = h / 1080.0;
    let portrait = Rect::new(96.0 * s, 92.0 * s, 610.0 * s, 800.0 * s);
    let x = (w * 0.5 - 140.0 * s).max(portrait.x + portrait.w + 90.0 * s);
    let tile_w = (w - x - 110.0 * s).min(940.0 * s);
    let tile_h = 118.0 * s;
    let gap = 30.0 * s;
    let top = 270.0 * s;
    let tiles = (0..count)
        .map(|i| {
            // Each a little further right as they go down, along the slash.
            Rect::new(
                x + i as f32 * 26.0 * s,
                top + i as f32 * (tile_h + gap),
                tile_w,
                tile_h,
            )
        })
        .collect();
    AskLayout {
        s,
        portrait,
        tiles,
        head: [x, 150.0 * s],
        back: Rect::new(w - 330.0 * s, h - 104.0 * s, 230.0 * s, 62.0 * s),
    }
}

/// How far the offer's card leans.
const OFFER_TILT: f32 = -0.035;

/// The offer page: Rosalía on the left, the elixir's card on the right (the leaflet's button
/// under its stage, the content warning across its foot, both turned with the card), its two
/// buttons. Nothing that matters sits at the screen's right, where Chiquita floats her lines.
struct OfferLayout {
    s: f32,
    portrait: Rect,
    card: Rect,
    stage: Rect,
    warning: Rect,
    leaflet: Rect,
    buy: Rect,
    decline: Rect,
}

fn offer_layout(w: f32, h: f32) -> OfferLayout {
    let s = h / 1080.0;
    let portrait = Rect::new(96.0 * s, 92.0 * s, 560.0 * s, 740.0 * s);
    let card_x = (w * 0.5 - 150.0 * s).max(portrait.x + portrait.w + 80.0 * s);
    let card = Rect::new(
        card_x,
        96.0 * s,
        (w - card_x - 110.0 * s).min(900.0 * s),
        704.0 * s,
    );
    let stage = Rect::new(card.x + 40.0 * s, card.y + 120.0 * s, 380.0 * s, 360.0 * s);
    let leaflet = Rect::new(stage.x, stage.y + stage.h + 16.0 * s, stage.w, 56.0 * s);
    let warning = Rect::new(
        card.x + 30.0 * s,
        card.y + card.h - 138.0 * s,
        card.w - 60.0 * s,
        110.0 * s,
    );
    let buy = Rect::new(
        card.x + 40.0 * s,
        card.y + card.h + 40.0 * s,
        520.0 * s,
        70.0 * s,
    );
    let decline = Rect::new(buy.x + buy.w + 30.0 * s, buy.y, 250.0 * s, 70.0 * s);
    OfferLayout {
        s,
        portrait,
        card,
        stage,
        warning,
        leaflet,
        buy,
        decline,
    }
}

/// How far the leaflet's sheet leans.
const LEAFLET_TILT: f32 = -0.008;

/// The leaflet page: the unfolded sheet and the way back to the offer.
struct LeafletLayout {
    s: f32,
    paper: Rect,
    back: Rect,
}

fn leaflet_layout(w: f32, h: f32) -> LeafletLayout {
    let s = h / 1080.0;
    LeafletLayout {
        s,
        paper: Rect::new(140.0 * s, 54.0 * s, w - 280.0 * s, 872.0 * s),
        // Left of where Chiquita stands when she floats a line.
        back: Rect::new(w - 590.0 * s, h - 110.0 * s, 340.0 * s, 64.0 * s),
    }
}

fn center(r: Rect) -> [f32; 2] {
    [r.x + r.w * 0.5, r.y + r.h * 0.5]
}

/// `p` turned by `angle` about `pivot`, as [`turned`] turns what it paints.
fn turn_point(p: [f32; 2], pivot: [f32; 2], angle: f32) -> [f32; 2] {
    let (sn, cs) = angle.sin_cos();
    let (dx, dy) = (p[0] - pivot[0], p[1] - pivot[1]);
    [pivot[0] + dx * cs - dy * sn, pivot[1] + dx * sn + dy * cs]
}

/// `(x, y)` inside `r` once `r` is turned by `angle` about `pivot`.
fn inside_turned_about(r: Rect, pivot: [f32; 2], angle: f32, x: f32, y: f32) -> bool {
    let [lx, ly] = turn_point([x, y], pivot, -angle);
    r.contains(lx, ly)
}

/// `(x, y)` inside `r` turned by `angle` about its centre.
fn inside_turned(r: Rect, angle: f32, x: f32, y: f32) -> bool {
    inside_turned_about(r, center(r), angle, x, y)
}

pub(crate) fn hit(w: f32, h: f32, x: f32, y: f32, menu: &Menu) -> Option<Hit> {
    match menu.page {
        Page::Girls => {
            let count = super::super::venue::catalog().company.len();
            let l = layout(w, h, count);
            if let Some(i) =
                (0..l.cards.len()).find(|&i| inside_turned(l.cards[i], CARD_TILT[i], x, y))
            {
                return Some(Hit::Item(i));
            }
            if l.choose.contains(x, y) {
                return Some(Hit::Action);
            }
            if l.close.contains(x, y) {
                return Some(Hit::Close);
            }
            if menu.progress.elixir_offered && l.elixir.contains(x, y) {
                return Some(Hit::Elixir);
            }
            if l.desk.contains(x, y) {
                return Some(Hit::Desk);
            }
            None
        }
        Page::Desk => {
            let l = desk_layout(w, h, desk_count(menu));
            l.rows
                .iter()
                .position(|r| inside_turned(*r, TILE_ANGLE, x, y))
                .map(Hit::DeskRow)
        }
        Page::Clients => {
            let l = clients_layout(w, h, ROSTER.len());
            if let Some(i) =
                (0..l.cards.len()).find(|&i| inside_turned(l.cards[i], CLIENT_TILT[i], x, y))
            {
                return Some(Hit::Client(i));
            }
            if l.serve.contains(x, y) {
                return Some(Hit::Client(menu.choice));
            }
            l.back.contains(x, y).then_some(Hit::Close)
        }
        Page::Ask => {
            let l = ask_layout(w, h, menu.asking().len());
            if let Some(i) = l
                .tiles
                .iter()
                .position(|r| inside_turned(*r, TILE_ANGLE, x, y))
            {
                return Some(Hit::Ask(i));
            }
            if l.back.contains(x, y) {
                return Some(Hit::Close);
            }
            None
        }
        Page::Offer => {
            let l = offer_layout(w, h);
            if l.buy.contains(x, y) {
                return Some(Hit::Buy);
            }
            if l.decline.contains(x, y) {
                return Some(Hit::Decline);
            }
            if inside_turned_about(l.leaflet, center(l.card), OFFER_TILT, x, y) {
                return Some(Hit::Leaflet);
            }
            None
        }
        Page::Leaflet => {
            let l = leaflet_layout(w, h);
            l.back.contains(x, y).then_some(Hit::Close)
        }
        Page::Loan => {
            let l = loan_layout(w, h, menu);
            if l.sign.contains(x, y) && menu.progress.debt == 0 {
                return matches!(
                    menu.loan_actions().get(menu.choice),
                    Some(LoanAction::Borrow(_))
                )
                .then_some(Hit::Loan(menu.choice));
            }
            let i = l
                .targets
                .iter()
                .position(|(r, angle)| inside_turned(*r, *angle, x, y))?;
            // A note not yet chosen is only picked; the chosen one (or any row) is taken.
            let note = matches!(menu.loan_actions().get(i), Some(LoanAction::Borrow(_)));
            Some(if note && i != menu.choice {
                Hit::LoanPick(i)
            } else {
                Hit::Loan(i)
            })
        }
    }
}

/// The loan page: Rosalía on the left as on the offer (her portrait shorter, so her word fits under
/// it). On the right, before any loan, her two promissory notes side by side and «Firmar» under
/// them; with a debt, her ledger's page and, under it, a tilted row for each thing that can be
/// done about it. Below the top third nothing that matters sits at the screen's right, where
/// Chiquita floats her lines.
struct LoanLayout {
    s: f32,
    portrait: Rect,
    /// With a debt: her ledger's page.
    ledger: Rect,
    /// One per action, as it is drawn and clicked: a note, a row or the back button, and its tilt.
    targets: Vec<(Rect, f32)>,
    /// Before a loan: sign the chosen note.
    sign: Rect,
}

/// How far each promissory note leans.
const NOTE_TILT: [f32; 2] = [-0.035, 0.028];

fn loan_layout(w: f32, h: f32, menu: &Menu) -> LoanLayout {
    let o = offer_layout(w, h);
    let s = o.s;
    let portrait = Rect::new(o.portrait.x, o.portrait.y, o.portrait.w, 640.0 * s);
    let x0 = o.card.x;
    let right = (w - 470.0 * s).min(x0 + 690.0 * s);
    let actions = menu.loan_actions();
    if menu.progress.debt == 0 {
        let gap = 36.0 * s;
        let nw = ((right - x0 - gap) * 0.5).min(340.0 * s);
        let nh = 590.0 * s;
        let mut targets: Vec<(Rect, f32)> = (0..2)
            .map(|i| {
                (
                    Rect::new(
                        x0 + i as f32 * (nw + gap),
                        118.0 * s + i as f32 * 18.0 * s,
                        nw,
                        nh,
                    ),
                    NOTE_TILT[i],
                )
            })
            .collect();
        let foot = 118.0 * s + nh + 64.0 * s;
        let sign = Rect::new(x0, foot, 420.0 * s, 72.0 * s);
        targets.push((
            Rect::new(sign.x + sign.w + 28.0 * s, foot, 230.0 * s, 72.0 * s),
            0.0,
        ));
        targets.truncate(actions.len());
        return LoanLayout {
            s,
            portrait,
            ledger: Rect::new(x0, 0.0, 0.0, 0.0),
            targets,
            sign,
        };
    }
    let choosing = menu.overdue() && menu.progress.debt_way.is_empty();
    let ledger = Rect::new(
        x0,
        92.0 * s,
        right - x0,
        if choosing { 410.0 } else { 480.0 } * s,
    );
    let row_h = if choosing { 112.0 } else { 104.0 } * s;
    let gap = 22.0 * s;
    let top = ledger.y + ledger.h + 46.0 * s;
    let targets = (0..actions.len())
        .map(|i| {
            (
                Rect::new(
                    x0 + 10.0 * s + i as f32 * 22.0 * s,
                    top + i as f32 * (row_h + gap),
                    right - x0 - 70.0 * s,
                    row_h,
                ),
                TILE_ANGLE,
            )
        })
        .collect();
    LoanLayout {
        s,
        portrait,
        ledger,
        targets,
        sign: Rect::new(0.0, 0.0, 0.0, 0.0),
    }
}

fn hex(s: &str, fallback: u32) -> u32 {
    u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(fallback)
}

/// A gold coin: a rimmed disc, a ring stamped on it, a glint.
fn coin(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, alpha: f32) {
    soft(
        v,
        Rect::new(c[0] - r, c[1] - r + r * 0.25, r * 2.0, r * 2.0),
        r,
        r * 0.5,
        rgba(SHADOW, 0.6 * alpha),
        false,
    );
    panel(
        v,
        Rect::new(c[0] - r, c[1] - r, r * 2.0, r * 2.0),
        r,
        Panel::fill(0xffe9a8, 0xb07a2c, alpha)
            .rim(GOLD_DEEP, alpha, (r * 0.14).max(1.0))
            .bevel(0.6),
    );
    arc(
        v,
        c,
        r * 0.62,
        (r * 0.1).max(1.0),
        0.0,
        1.0,
        rgba(0x9a6a28, 0.8 * alpha),
        [0.0; 4],
    );
    sparkle(
        v,
        [c[0] - r * 0.32, c[1] - r * 0.34],
        r * 0.42,
        rgba(0xfffbe8, 0.95 * alpha),
        0.4,
    );
}

/// A face in a window of any shape: the square portrait cropped to the window's proportions,
/// kept toward its top (where the face is).
fn face_window(v: &mut Vec<MapVertex>, r: Rect, radius: f32, uv: Uv, alpha: f32) {
    let (u0, u1) = uv;
    let span = [u1[0] - u0[0], u1[1] - u0[1]];
    let aspect = r.h / r.w.max(1.0);
    let (uv0, uv1) = if aspect < 1.0 {
        let keep = span[1] * aspect;
        let top = u0[1] + (span[1] - keep) * 0.3;
        ([u0[0], top], [u1[0], top + keep])
    } else {
        let keep = span[0] / aspect;
        let left = u0[0] + (span[0] - keep) * 0.5;
        ([left, u0[1]], [left + keep, u1[1]])
    };
    face(v, r, radius, (uv0, uv1), alpha);
}

/// A button: a ribbon of glass with gold tips, its key on a cap at the left, its words.
#[allow(clippy::too_many_arguments)]
fn button(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    key: &str,
    label: &str,
    primary: bool,
    hover: bool,
    enabled: bool,
    accent: u32,
    time: f32,
    s: f32,
) {
    let lit = primary && enabled;
    soft(
        v,
        Rect::new(r.x, r.y + 8.0 * s, r.w, r.h),
        r.h * 0.5,
        14.0 * s,
        rgba(SHADOW, 0.7),
        false,
    );
    if lit {
        soft(
            v,
            r,
            r.h * 0.5,
            18.0 * s,
            rgba(accent, 0.30 + 0.10 * wave(time, 0.8)),
            true,
        );
    }
    let (top, bottom, rim) = if !enabled {
        (0x221c28, 0x14101a, 0x6a6074)
    } else if lit {
        (
            mix(0x5a2272, accent, 0.30),
            mix(0x2a0f3a, accent, 0.12),
            GOLD_HI,
        )
    } else if hover {
        (0x3a1f50, 0x1e0e2a, GOLD_HI)
    } else {
        (0x2c1640, 0x150a20, GOLD)
    };
    let mut style = Panel::fill(top, bottom, 0.97)
        .rim(rim, 1.0, 2.2 * s)
        .hairline(5.0 * s, if lit { 0.55 } else { 0.3 })
        .bevel(1.0);
    if lit {
        style = style.sheen(time * 0.3);
    }
    panel(v, r, r.h * 0.5, style);
    for side in [-1.0f32, 1.0] {
        let tip = [
            r.x + r.w * 0.5 + side * (r.w * 0.5 + 2.0 * s),
            r.y + r.h * 0.5,
        ];
        gem_turned(v, tip, 11.0 * s, 9.0 * s, 0.0, rim, rim, 1.0, 0.0);
    }
    let cap_w = (text_width(font, Face::Title, key, 18.0 * s) + 26.0 * s).max(42.0 * s);
    let cap = Rect::new(
        r.x + 18.0 * s,
        r.y + (r.h - 40.0 * s) * 0.5,
        cap_w,
        40.0 * s,
    );
    keycap(font, v, cap, key, 18.0 * s, false, !enabled, accent, 1.0);
    let size = 24.0 * s;
    let ink = if enabled { INK } else { 0x8a8290 };
    let x = cap.x + cap.w + 16.0 * s;
    let room = r.x + r.w - 26.0 * s - x;
    let mut fitted = size;
    while fitted > 16.0 * s && text_width(font, Face::Body, label, fitted) > room {
        fitted -= 0.5 * s;
    }
    text_shadow(
        font,
        v,
        Face::Body,
        label,
        x,
        r.y + (r.h - fitted) * 0.5 - 3.0 * s,
        fitted,
        rgba(ink, 1.0),
    );
}

/// One girl's card, drawn upright (the caller leans it). Being chosen lights it: a plate of her
/// colour behind it, her glow, a gold rim; the others sit back in the dark. It never moves.
#[allow(clippy::too_many_arguments)]
fn card(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    index: usize,
    e: &super::super::venue::Entry,
    face_uv: Option<Uv>,
    menu: &Menu,
    selected: bool,
    hover: bool,
    gold: u32,
    time: f32,
    s: f32,
) {
    let accent = hex(&e.accent, 0xd9bd8a);
    let price = if e.company_price > 0 {
        e.company_price
    } else {
        e.price
    };
    let can_pay = gold >= price;
    // Persona's hard shadow: a black copy of the card, down and to the right.
    panel(
        v,
        Rect::new(r.x + 16.0 * s, r.y + 18.0 * s, r.w, r.h),
        8.0 * s,
        Panel::fill(0x000000, 0x000000, 0.62),
    );
    if selected {
        let pulse = 0.85 + 0.15 * wave(time, 0.7);
        soft(v, r, 8.0 * s, 30.0 * s, rgba(accent, 0.4 * pulse), true);
    } else if hover {
        soft(v, r, 8.0 * s, 20.0 * s, rgba(accent, 0.18), true);
    }
    // A plate of her colour behind it, out to the upper left: deep, and bright when she is chosen
    // (only its colour changes).
    panel(
        v,
        Rect::new(r.x - 14.0 * s, r.y - 14.0 * s, r.w, r.h),
        8.0 * s,
        if selected {
            Panel::fill(mix(accent, 0xffffff, 0.2), mix(accent, 0x000000, 0.3), 1.0)
        } else {
            Panel::fill(mix(accent, 0x000000, 0.55), mix(accent, 0x000000, 0.8), 0.9)
        },
    );
    panel(
        v,
        grow(r, 6.0 * s),
        11.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    let mut style = Panel::fill(
        mix(0x22102a, accent, if selected { 0.2 } else { 0.08 }),
        0x0b0508,
        0.99,
    )
    .rim(
        if selected {
            GOLD_HI
        } else if hover {
            GOLD
        } else {
            GOLD_DEEP
        },
        1.0,
        2.4 * s,
    )
    .hairline(7.0 * s, if selected { 0.5 } else { 0.22 })
    .bevel(0.6);
    if selected {
        style = style.sheen(time * 0.18);
    }
    panel(v, r, 8.0 * s, style);
    // Her colour down the card's left edge.
    panel(
        v,
        Rect::new(r.x, r.y, 10.0 * s, r.h),
        3.0 * s,
        Panel::fill(
            mix(accent, 0xffffff, 0.3),
            accent,
            if selected { 1.0 } else { 0.65 },
        ),
    );

    // Her face, large, in a frame of her colour: a halftone in its corner, the dark rising at its
    // foot under her name.
    let window = Rect::new(r.x + 20.0 * s, r.y + 16.0 * s, r.w - 36.0 * s, 318.0 * s);
    panel(
        v,
        grow(window, 3.0 * s),
        6.0 * s,
        Panel::fill(
            mix(0x2a1238, accent, 0.45),
            mix(0x0c0612, accent, 0.15),
            1.0,
        ),
    );
    match face_uv {
        Some(uv) => face_window(
            v,
            window,
            4.0 * s,
            uv,
            if selected || hover { 1.0 } else { 0.84 },
        ),
        None => gem(v, center(window), 40.0 * s, 56.0 * s, accent, 1.0, 12.0 * s),
    }
    halftone(
        v,
        Rect::new(
            window.x + window.w * 0.62,
            window.y + 4.0 * s,
            window.w * 0.38 - 4.0 * s,
            window.h * 0.32,
        ),
        13.0 * s,
        7.0 * s,
        [0.9, -0.5],
        rgba(accent, 0.42),
    );
    panel(
        v,
        Rect::new(
            window.x,
            window.y + window.h * 0.52,
            window.w,
            window.h * 0.48,
        ),
        4.0 * s,
        Panel {
            top: rgba(0x0b0508, 0.0),
            bottom: rgba(0x0b0508, 0.9),
            ..Panel::fill(0, 0, 0.0)
        },
    );
    shout(
        font,
        v,
        Face::Title,
        &e.name,
        window.x + 16.0 * s,
        window.y + window.h - 76.0 * s,
        62.0 * s,
        0xffffff,
        accent,
        1.0,
    );

    // Her number on a black diamond over the frame's corner.
    let c = [r.x + 8.0 * s, r.y + 10.0 * s];
    turned(v, c, std::f32::consts::FRAC_PI_4, |v| {
        panel(
            v,
            Rect::new(c[0] - 27.0 * s, c[1] - 27.0 * s, 54.0 * s, 54.0 * s),
            5.0 * s,
            Panel::fill(0x0b0508, 0x0b0508, 1.0),
        );
        panel(
            v,
            Rect::new(c[0] - 21.0 * s, c[1] - 21.0 * s, 42.0 * s, 42.0 * s),
            4.0 * s,
            if selected {
                Panel::fill(mix(accent, 0xffffff, 0.25), accent, 1.0)
            } else {
                Panel::fill(0x2a1834, 0x160c1c, 1.0)
            },
        );
    });
    let number = format!("{:02}", index + 1);
    let nw = text_width(font, Face::Title, &number, 24.0 * s);
    text_outline(
        font,
        v,
        Face::Title,
        &number,
        c[0] - nw * 0.5,
        c[1] - 15.0 * s,
        24.0 * s,
        rgba(0xffffff, 1.0),
        rgba(0x0b0508, 0.9),
    );

    // Her standing in the house on a black tag, her price at the right.
    let mut y = window.y + window.h + 14.0 * s;
    if !e.tier.is_empty() {
        let tw = text_width(font, Face::Italic, &e.tier, 22.0 * s);
        let tag = Rect::new(r.x + 20.0 * s, y, tw + 30.0 * s, 36.0 * s);
        panel(
            v,
            tag,
            4.0 * s,
            Panel::fill(0x0b0508, 0x0b0508, 0.95).rim(mix(accent, 0x000000, 0.2), 0.9, 1.4 * s),
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            &e.tier,
            tag.x + 15.0 * s,
            tag.y + 5.0 * s,
            22.0 * s,
            rgba(mix(accent, 0xffffff, 0.4), 1.0),
        );
    }
    let price_text = format!("{price}");
    let pw = text_width(font, Face::Ui, &price_text, 28.0 * s);
    let px = r.x + r.w - 22.0 * s - pw;
    coin(v, [px - 24.0 * s, y + 18.0 * s], 15.0 * s, 1.0);
    text_shadow(
        font,
        v,
        Face::Ui,
        &price_text,
        px,
        y + 1.0 * s,
        28.0 * s,
        rgba(if can_pay { GOLD_HI } else { 0xe0566b }, 1.0),
    );
    y += 50.0 * s;
    // Translated whole (the catalogs hold both lines as one), then split.
    let summary = demon_queen_dungeon::i18n::tr(&e.summary);
    for line in summary.lines().take(2) {
        text_shadow(
            font,
            v,
            Face::Body,
            line,
            r.x + 24.0 * s,
            y,
            19.0 * s,
            rgba(CREAM, 0.95),
        );
        y += 19.0 * s * 1.4;
    }

    // What she gives, on chips (in smaller type when they would not fit).
    y += 8.0 * s;
    let room = r.w - 44.0 * s;
    let widths = |size: f32| -> Vec<f32> {
        e.benefits
            .iter()
            .take(2)
            .map(|b| text_width(font, Face::Body, b, size) + 40.0 * s)
            .collect()
    };
    let mut size = 17.0 * s;
    let mut ws = widths(size);
    while size > 13.0 * s && ws.iter().sum::<f32>() + 8.0 * s > room {
        size -= 0.5 * s;
        ws = widths(size);
    }
    let chip_h = 32.0 * s;
    let mut x = r.x + 22.0 * s;
    for (label, cw) in e.benefits.iter().zip(ws) {
        let chip = Rect::new(x, y, cw, chip_h);
        panel(
            v,
            chip,
            chip_h * 0.5,
            Panel::fill(mix(0x2c1640, accent, 0.25), 0x150a20, 0.95)
                .rim(mix(GOLD, accent, 0.35), 1.0, 1.4 * s)
                .bevel(1.0),
        );
        gem(
            v,
            [chip.x + 16.0 * s, chip.y + chip_h * 0.5],
            5.0 * s,
            8.0 * s,
            accent,
            1.0,
            3.0 * s,
        );
        text_shadow(
            font,
            v,
            Face::Body,
            label,
            chip.x + 28.0 * s,
            chip.y + (chip_h - size) * 0.5 - 2.0 * s,
            size,
            rgba(INK, 1.0),
        );
        x += cw + 8.0 * s;
    }

    // At its foot: what can be asked of her (the seal's shape once Rosalía has offered it), her room.
    let foot = r.y + r.h - 40.0 * s;
    wire(
        v,
        [r.x + 24.0 * s, foot - 28.0 * s],
        [r.x + r.w - 24.0 * s, foot - 28.0 * s],
        1.2 * s,
        rgba(GOLD, 0.55),
        rgba(GOLD, 0.0),
        0.0,
        false,
    );
    let mut ex = r.x + 42.0 * s;
    for o in e
        .options
        .iter()
        .filter(|o| !o.elixir || menu.progress.elixir_offered)
    {
        let awake = o.elixir && menu.progress.elixir_active;
        emblem(
            v,
            &o.icon,
            [ex, foot],
            17.0 * s,
            false,
            o.elixir && !awake,
            awake,
            accent,
            time,
            s,
        );
        ex += 44.0 * s;
    }
    let room_text = format!("Habitación {}", e.room + 1);
    let rw = text_width(font, Face::Italic, &room_text, 20.0 * s);
    text_shadow(
        font,
        v,
        Face::Italic,
        &room_text,
        r.x + r.w - 24.0 * s - rw,
        foot - 13.0 * s,
        20.0 * s,
        rgba(DIM, 1.0),
    );

    if !selected {
        // The others sit back in the dark.
        panel(
            v,
            grow(r, 6.0 * s),
            11.0 * s,
            Panel::fill(0x05020a, 0x05020a, if hover { 0.08 } else { 0.26 }),
        );
    }
}

/// A warning label: black words on hazard yellow inside a black edge.
fn warn_tag(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, words: &str, s: f32) {
    panel(
        v,
        grow(r, 2.0 * s),
        5.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    panel(
        v,
        r,
        4.0 * s,
        Panel::fill(0xffe25a, 0xf2b705, 1.0).bevel(0.5),
    );
    let size = r.h * 0.56;
    let tw = text_width(font, Face::Title, words, size);
    text(
        font,
        v,
        Face::Title,
        words,
        r.x + (r.w - tw) * 0.5,
        r.y + (r.h - size) * 0.5 - size * 0.12,
        size,
        rgba(0x120808, 1.0),
    );
}

/// How wide [`warn_tag`] makes `words` on a tag `h` tall.
fn warn_tag_width(font: &FontAtlas, words: &str, h: f32) -> f32 {
    text_width(font, Face::Title, words, h * 0.56) + h * 0.9
}

/// The elixir on the girls' page, once Rosalía has offered it: its seal, what it costs (or that
/// it is drunk) and the content warning that goes with it.
fn elixir_sticker(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    menu: &Menu,
    hover: bool,
    time: f32,
    s: f32,
) {
    let awake = menu.progress.elixir_active;
    let pink = 0xff4f9a;
    if awake {
        soft(
            v,
            r,
            r.h * 0.5,
            16.0 * s,
            rgba(pink, 0.35 + 0.12 * wave(time, 0.9)),
            true,
        );
    } else if hover {
        soft(v, r, r.h * 0.5, 12.0 * s, rgba(pink, 0.22), true);
    }
    panel(
        v,
        r,
        r.h * 0.5,
        Panel::fill(if awake { 0x5a1440 } else { 0x2c1640 }, 0x150a20, 0.97)
            .rim(if awake { 0xffb0d0 } else { GOLD }, 1.0, 2.0 * s)
            .bevel(1.0),
    );
    seal_icon(
        v,
        [r.x + 26.0 * s, r.y + r.h * 0.5],
        13.0 * s,
        awake,
        time,
        s,
    );
    let words = "+18 · FUTA";
    let tag_h = 30.0 * s;
    let tag_w = warn_tag_width(font, words, tag_h);
    let tag = Rect::new(
        r.x + r.w - tag_w - 12.0 * s,
        r.y + (r.h - tag_h) * 0.5,
        tag_w,
        tag_h,
    );
    warn_tag(font, v, tag, words, s);
    let (label, ink) = if awake {
        ("El sello está despierto".to_owned(), 0xffd0e4)
    } else {
        (
            format!("E · Elixir de Rosalía · {} oro", menu.elixir_price()),
            INK,
        )
    };
    let room = tag.x - 12.0 * s - (r.x + 48.0 * s);
    let mut size = 19.0 * s;
    while size > 14.0 * s && text_width(font, Face::Body, &label, size) > room {
        size -= 0.5 * s;
    }
    text_shadow(
        font,
        v,
        Face::Body,
        &label,
        r.x + 48.0 * s,
        r.y + (r.h - size) * 0.5 - 2.0 * s,
        size,
        rgba(ink, 1.0),
    );
}

/// The purse: a coin and how much gold she carries.
fn purse(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, gold: u32, s: f32) {
    panel(
        v,
        r,
        r.h * 0.5,
        Panel::fill(0x2c1640, 0x120818, 0.97)
            .rim(GOLD, 1.0, 2.2 * s)
            .hairline(5.0 * s, 0.35)
            .bevel(1.0),
    );
    coin(v, [r.x + 34.0 * s, r.y + r.h * 0.5], 17.0 * s, 1.0);
    let amount = format!("{gold}");
    text_shadow(
        font,
        v,
        Face::Ui,
        &amount,
        r.x + 62.0 * s,
        r.y + r.h * 0.5 - 17.0 * s,
        28.0 * s,
        rgba(GOLD_HI, 1.0),
    );
    let aw = text_width(font, Face::Ui, &amount, 28.0 * s);
    text_shadow(
        font,
        v,
        Face::Italic,
        "oro",
        r.x + 70.0 * s + aw,
        r.y + r.h * 0.5 - 10.0 * s,
        20.0 * s,
        rgba(DIM, 1.0),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    faces: &[Option<Uv>],
    host_face: Option<Uv>,
    time: f32,
    liquid: super::shop_preview::LiquidMotion,
) {
    match menu.page {
        Page::Girls => draw_girls(font, v, w, h, menu, gold, pointer, faces, host_face, time),
        Page::Ask => draw_ask(font, v, w, h, menu, gold, pointer, faces, time),
        Page::Offer => draw_offer(font, v, w, h, menu, gold, pointer, host_face, time, liquid),
        Page::Leaflet => draw_leaflet(font, v, w, h, menu, pointer, time),
        Page::Loan => draw_loan(font, v, w, h, menu, gold, pointer, host_face, time),
        Page::Desk => draw_desk(font, v, w, h, menu, gold, pointer, host_face, time),
        Page::Clients => draw_clients(font, v, w, h, menu, gold, pointer, faces, host_face, time),
    }
}

/// Paper of Rosalía's: warm cream with a fine brown edge, on the page's black outline and shadow.
fn paper(v: &mut Vec<MapVertex>, r: Rect, lift: f32, lit: bool, s: f32) {
    soft(
        v,
        Rect::new(r.x + 14.0 * s, r.y + (20.0 + lift) * s, r.w, r.h),
        10.0 * s,
        (22.0 + lift) * s,
        rgba(SHADOW, 0.75),
        false,
    );
    panel(
        v,
        grow(r, 5.0 * s),
        12.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    if lit {
        panel(
            v,
            grow(r, 6.0 * s),
            13.0 * s,
            Panel::fill(GOLD_HI, GOLD_DEEP, 1.0),
        );
    }
    panel(
        v,
        r,
        8.0 * s,
        Panel::fill(0xfdf6e8, 0xeedcbe, 1.0).rim(0x9a7a56, 0.9, 1.6 * s),
    );
}

/// A paper not chosen, dimmed under a veil of the page's dark.
fn veil(v: &mut Vec<MapVertex>, r: Rect, s: f32) {
    panel(v, r, 8.0 * s, Panel::fill(0x1a0a18, 0x1a0a18, 0.32));
}

/// A band across the top of a paper: her red, with its title.
fn paper_band(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, title: &str, tag: &str, s: f32) {
    let band = Rect::new(r.x, r.y, r.w, 70.0 * s);
    panel(v, band, 8.0 * s, Panel::fill(0xb81a44, 0x7a0a2a, 1.0));
    panel(
        v,
        Rect::new(r.x, band.y + band.h - 10.0 * s, r.w, 10.0 * s),
        0.0,
        Panel::fill(0x7a0a2a, 0x7a0a2a, 1.0),
    );
    text_outline(
        font,
        v,
        Face::Title,
        title,
        r.x + 24.0 * s,
        r.y + 13.0 * s,
        40.0 * s,
        rgba(0xfff4ea, 1.0),
        rgba(0x3a0618, 1.0),
    );
    if !tag.is_empty() {
        let tw = text_width(font, Face::Ui, tag, 18.0 * s);
        text(
            font,
            v,
            Face::Ui,
            tag,
            r.x + r.w - tw - 20.0 * s,
            r.y + 26.0 * s,
            18.0 * s,
            rgba(0xffd6e0, 0.95),
        );
    }
}

/// A dotted leader from `x0` to `x1` on the line `y`, as in an account book.
fn leader(v: &mut Vec<MapVertex>, x0: f32, x1: f32, y: f32, s: f32) {
    let step = 11.0 * s;
    let mut x = x0;
    while x < x1 {
        disc(v, [x, y], 1.8 * s, rgba(LEAF_INK, 0.45));
        x += step;
    }
}

/// Rosalía's wax seal: a red blot with her initial.
fn wax_seal(font: &FontAtlas, v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, s: f32) {
    burst(
        v,
        c,
        r * 0.9,
        r * 1.08,
        14,
        0.3,
        rgba(0xa0102e, 1.0),
        rgba(0x5a0418, 1.0),
        2.0 * s,
    );
    disc(v, c, r * 0.78, rgba(0xc2183c, 1.0));
    disc(v, c, r * 0.62, rgba(0x9a0c2c, 1.0));
    let tw = text_width(font, Face::Title, "R", r * 0.95);
    text(
        font,
        v,
        Face::Title,
        "R",
        c[0] - tw * 0.5,
        c[1] - r * 0.52,
        r * 0.95,
        rgba(0xffc4d0, 0.9),
    );
}

/// The days to pay, as boxes on the paper: those already gone crossed out in red.
#[allow(clippy::too_many_arguments)]
fn day_boxes(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    gone: i32,
    late: bool,
    size: f32,
    s: f32,
) {
    use super::super::venue::LOAN_DAYS;
    for d in 0..LOAN_DAYS {
        let r = Rect::new(x + d as f32 * (size + 10.0 * s), y, size, size);
        panel(
            v,
            r,
            4.0 * s,
            Panel::fill(0xfffaf0, 0xf2e4c8, 1.0).rim(LEAF_INK, 0.7, 1.6 * s),
        );
        let n = format!("{}", d + 1);
        let tw = text_width(font, Face::Ui, &n, size * 0.42);
        text(
            font,
            v,
            Face::Ui,
            &n,
            r.x + (r.w - tw) * 0.5,
            r.y + size * 0.26,
            size * 0.42,
            rgba(LEAF_INK, 0.75),
        );
        if d < gone || late {
            let red = rgba(LEAF_RED, 0.9);
            let m = 7.0 * s;
            wire(
                v,
                [r.x + m, r.y + m],
                [r.x + r.w - m, r.y + r.h - m],
                4.0 * s,
                red,
                red,
                0.0,
                false,
            );
            wire(
                v,
                [r.x + r.w - m, r.y + m],
                [r.x + m, r.y + r.h - m],
                4.0 * s,
                red,
                red,
                0.0,
                false,
            );
        }
    }
}

/// A pile of gold: `count` coins in stacks of five, side by side, their foot at `base`.
fn coin_pile(v: &mut Vec<MapVertex>, base: [f32; 2], count: u32, r: f32) {
    let stacks = count.div_ceil(5);
    for k in 0..stacks {
        let in_stack = (count - k * 5).min(5);
        let x = base[0] + k as f32 * r * 1.7;
        for i in 0..in_stack {
            let wobble = if i % 2 == 0 { 0.0 } else { r * 0.12 };
            coin(v, [x + wobble, base[1] - r - i as f32 * r * 0.42], r, 1.0);
        }
    }
}

/// One of her promissory notes: what she lends now, what goes back to her, in how many days, the
/// small print and her seal.
#[allow(clippy::too_many_arguments)]
fn promissory_note(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    amount: u32,
    number: u32,
    lit: bool,
    hover: bool,
    s: f32,
) {
    use super::super::venue::{owed_for, LOAN_DAYS};
    paper(v, r, if lit { 10.0 } else { 0.0 }, lit, s);
    paper_band(font, v, r, "PAGARÉ", &format!("Nº {number:02}"), s);
    let x = r.x + 24.0 * s;
    let ink = rgba(LEAF_INK, 1.0);
    text(
        font,
        v,
        Face::Italic,
        "La casa le presta",
        x,
        r.y + 88.0 * s,
        24.0 * s,
        rgba(LEAF_INK, 0.8),
    );
    // The gold, and how much.
    coin_pile(v, [x + 30.0 * s, r.y + 250.0 * s], amount / 25, 22.0 * s);
    let big = 78.0 * s;
    let amount_text = format!("{amount}");
    let ax = x + if amount > 100 { 120.0 } else { 86.0 } * s;
    text(
        font,
        v,
        Face::Title,
        &amount_text,
        ax,
        r.y + 138.0 * s,
        big,
        ink,
    );
    text(
        font,
        v,
        Face::Italic,
        "oro, al momento",
        ax + 4.0 * s,
        r.y + 224.0 * s,
        21.0 * s,
        rgba(LEAF_INK, 0.75),
    );
    // A torn line, then what goes back to her.
    let mut dx = r.x + 18.0 * s;
    while dx < r.x + r.w - 18.0 * s {
        wire(
            v,
            [dx, r.y + 282.0 * s],
            [dx + 9.0 * s, r.y + 282.0 * s],
            2.0 * s,
            rgba(LEAF_INK, 0.35),
            rgba(LEAF_INK, 0.35),
            0.0,
            false,
        );
        dx += 16.0 * s;
    }
    text(
        font,
        v,
        Face::Ui,
        "SE DEVUELVE",
        x,
        r.y + 300.0 * s,
        18.0 * s,
        rgba(LEAF_INK, 0.7),
    );
    let owed = format!("{}", owed_for(amount));
    text(
        font,
        v,
        Face::Title,
        &owed,
        x,
        r.y + 324.0 * s,
        58.0 * s,
        rgba(LEAF_RED, 1.0),
    );
    let ow = text_width(font, Face::Title, &owed, 58.0 * s);
    text(
        font,
        v,
        Face::Italic,
        "oro (+30 %)",
        x + ow + 12.0 * s,
        r.y + 352.0 * s,
        22.0 * s,
        rgba(LEAF_RED, 0.85),
    );
    text(
        font,
        v,
        Face::Ui,
        &format!("EN {LOAN_DAYS} DÍAS"),
        x,
        r.y + 404.0 * s,
        18.0 * s,
        rgba(LEAF_INK, 0.7),
    );
    day_boxes(font, v, x, r.y + 430.0 * s, 0, false, 40.0 * s, s);
    wax_seal(
        font,
        v,
        [r.x + r.w - 58.0 * s, r.y + 448.0 * s],
        36.0 * s,
        s,
    );
    // The small print, across its foot.
    let fine = "Letra pequeña: vencido el plazo, la clienta elige cómo pagar: la mina, la casa o la calle. La casa no perdona… pero tampoco olvida.";
    let mut y = r.y + 498.0 * s;
    for line in wrap(font, Face::Italic, fine, 15.0 * s, r.w - 48.0 * s) {
        text(
            font,
            v,
            Face::Italic,
            &line,
            x,
            y,
            15.0 * s,
            rgba(LEAF_INK, 0.72),
        );
        y += 15.0 * s * 1.35;
    }
    if !lit && !hover {
        veil(v, r, s);
    }
}

/// Her ledger's page for the Queen's debt: the loan, her cut, what is paid, what is owed, the
/// days, and a stamp when it fell due or how it is being paid.
fn ledger_page(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, menu: &Menu, s: f32) {
    use super::super::venue::{owed_for, LOAN_DAYS, WAY_HOUSE, WAY_MINE};
    let p = &menu.progress;
    paper(v, r, 0.0, false, s);
    paper_band(
        font,
        v,
        r,
        "CUENTA DE LA CASA",
        &format!("Nº {:02}", p.loans.max(1)),
        s,
    );
    let x = r.x + 30.0 * s;
    let right = r.x + r.w - 30.0 * s;
    text(
        font,
        v,
        Face::Italic,
        "Clienta: «la de los cuernos»",
        x,
        r.y + 84.0 * s,
        23.0 * s,
        rgba(LEAF_INK, 0.8),
    );
    // The account, line by line.
    let mut rows: Vec<(String, String)> = Vec::new();
    if p.debt_lent > 0 {
        rows.push(("Préstamo".into(), format!("{} oro", p.debt_lent)));
        rows.push((
            "Interés de la casa (30 %)".into(),
            format!("{} oro", owed_for(p.debt_lent) - p.debt_lent),
        ));
        let paid = menu.debt_paid();
        if paid > 0 {
            let how = if p.debt_way == WAY_MINE {
                "Pagado en la mina"
            } else if p.debt_way == WAY_HOUSE || p.shifts_worked > 0 {
                "Pagado en turnos"
            } else {
                "Pagado"
            };
            rows.push((how.into(), format!("−{paid} oro")));
        }
    } else {
        rows.push(("Deuda".into(), format!("{} oro", p.debt)));
    }
    let size = 25.0 * s;
    let mut y = r.y + 126.0 * s;
    for (label, amount) in &rows {
        text(font, v, Face::Body, label, x, y, size, rgba(LEAF_INK, 0.95));
        let lw = text_width(font, Face::Body, label, size);
        let aw = text_width(font, Face::Ui, amount, size);
        leader(
            v,
            x + lw + 12.0 * s,
            right - aw - 12.0 * s,
            y + size * 0.78,
            s,
        );
        text(
            font,
            v,
            Face::Ui,
            amount,
            right - aw,
            y,
            size,
            rgba(LEAF_INK, 0.95),
        );
        y += 40.0 * s;
    }
    // The total, under a double rule.
    for k in 0..2 {
        let ly = y + 4.0 * s + k as f32 * 6.0 * s;
        wire(
            v,
            [x, ly],
            [right, ly],
            2.0 * s,
            rgba(LEAF_INK, 0.6),
            rgba(LEAF_INK, 0.6),
            0.0,
            false,
        );
    }
    y += 22.0 * s;
    let total_y = y;
    text(
        font,
        v,
        Face::Title,
        "DEBE",
        x,
        y + 14.0 * s,
        40.0 * s,
        rgba(LEAF_INK, 1.0),
    );
    let total = format!("{} oro", p.debt);
    let tw = text_width(font, Face::Title, &total, 62.0 * s);
    text(
        font,
        v,
        Face::Title,
        &total,
        right - tw,
        y,
        62.0 * s,
        rgba(LEAF_RED, 1.0),
    );
    // What is paid of it, as a bar.
    let owed = owed_for(p.debt_lent);
    let paid = menu.debt_paid();
    if paid > 0 && owed > 0 {
        let bar_y = y + 86.0 * s;
        let label = format!("Pagado {paid} de {owed} oro");
        let lw = text_width(font, Face::Ui, &label, 17.0 * s);
        text(
            font,
            v,
            Face::Ui,
            &label,
            right - lw,
            bar_y - 24.0 * s,
            17.0 * s,
            rgba(LEAF_INK, 0.75),
        );
        let track = Rect::new(x, bar_y, right - x, 14.0 * s);
        panel(
            v,
            track,
            7.0 * s,
            Panel::fill(0xe6d4b4, 0xdcc8a4, 1.0).rim(LEAF_INK, 0.4, 1.2 * s),
        );
        let f = paid as f32 / owed as f32;
        panel(
            v,
            Rect::new(track.x, track.y, (track.w * f).max(track.h), track.h),
            7.0 * s,
            Panel::fill(0xd23a5a, LEAF_RED, 1.0),
        );
    }
    // The days: boxes, crossed as they go, and what it means.
    let due = menu.overdue();
    let left = menu.days_left();
    let gone = (LOAN_DAYS - left).clamp(0, LOAN_DAYS);
    let by = r.y + r.h - 74.0 * s;
    text(
        font,
        v,
        Face::Ui,
        "PLAZO",
        x,
        by + 11.0 * s,
        18.0 * s,
        rgba(LEAF_INK, 0.7),
    );
    day_boxes(font, v, x + 78.0 * s, by, gone, due, 40.0 * s, s);
    let when = if due {
        let late = -left;
        if late == 1 {
            "Venció ayer".to_owned()
        } else {
            format!("Venció hace {late} días")
        }
    } else {
        match left {
            0 => "Vence hoy".to_owned(),
            1 => "Vence mañana".to_owned(),
            d => format!("Vence en {d} días"),
        }
    };
    let when_ink = if due || left == 0 { LEAF_RED } else { LEAF_INK };
    text(
        font,
        v,
        Face::Italic,
        &when,
        x + 78.0 * s + 3.0 * 50.0 * s + 12.0 * s,
        by + 6.0 * s,
        26.0 * s,
        rgba(when_ink, 1.0),
    );
    wax_seal(
        font,
        v,
        [r.x + r.w - 54.0 * s, r.y + r.h - 56.0 * s],
        34.0 * s,
        s,
    );
    // The stamp, slapped across the total between «DEBE» and the sum.
    let stamp_at = [r.x + r.w * 0.43, total_y + 32.0 * s];
    if p.debt_way == WAY_MINE {
        stamp(font, v, stamp_at, "EN LA MINA", "lo cobra el enano", 1.0, s);
    } else if p.debt_way == WAY_HOUSE {
        stamp(font, v, stamp_at, "EN LA CASA", "turno a turno", 1.0, s);
    } else if due {
        stamp(font, v, stamp_at, "VENCIDO", "Rosalía cobra hoy", 1.0, s);
    }
}

/// One row of what can be done about the debt, as the ask page's tiles show it.
fn loan_row(menu: &Menu, action: LoanAction, gold: u32) -> Asking {
    use super::super::venue::SHIFT_PAY;
    let debt = menu.progress.debt;
    let (label, hint, icon) = match action {
        LoanAction::Borrow(a) => (format!("Pedir {a}"), String::new(), "coin"),
        LoanAction::Repay => (
            format!("Devolver {debt} oro"),
            if gold >= debt {
                "Rosalía lo tacha de su libro".to_owned()
            } else {
                format!("Te faltan {} de oro", debt - gold)
            },
            "coin",
        ),
        LoanAction::Mine => (
            "El pozo".to_owned(),
            format!("Picar {debt} oro en mineral para ella"),
            "pick",
        ),
        LoanAction::House => (
            "La casa".to_owned(),
            format!("Turnos con máscara · −{SHIFT_PAY} por turno"),
            "mask",
        ),
        LoanAction::Refuse => (
            "La calle".to_owned(),
            "No pagas y no vuelves a entrar".to_owned(),
            "door",
        ),
        LoanAction::Shift => (
            "Hacer un turno".to_owned(),
            format!("Un cliente, con máscara · −{SHIFT_PAY}"),
            "mask",
        ),
        LoanAction::Back => ("Ahora no".to_owned(), "Volver al libro".to_owned(), "back"),
    };
    Asking {
        label,
        hint,
        icon: icon.to_owned(),
        locked: !menu.loan_enabled(action, gold),
        awake: false,
        back: matches!(action, LoanAction::Back),
    }
}

/// Rosalía's loan: before any loan her two promissory notes to choose from; with a debt her
/// ledger's page and what can be done about it (once due: the mine, the house or the street).
#[allow(clippy::too_many_arguments)]
fn draw_loan(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    host_face: Option<Uv>,
    time: f32,
) {
    let actions = menu.loan_actions();
    let l = loan_layout(w, h, menu);
    let s = l.s;
    let age = time - menu.page_at_ms as f32 / 1000.0;
    let rose = 0xd83c78;
    let due = menu.overdue() && menu.progress.debt_way.is_empty();
    let accent = if due { 0xe8a030 } else { rose };
    backdrop(
        v,
        w,
        h,
        if due { 0x5a2a08 } else { 0x6a0a3a },
        if due { 0xe8a030 } else { 0xff4f9a },
        age,
        time,
        s,
    );
    cut_in(v, l.portrait, host_face, rose, age, s);
    let pr = l.portrait;
    let subtitle = if menu.progress.debt == 0 {
        "La casa también presta"
    } else if menu.collecting() {
        "La casa cobra"
    } else {
        "La casa apunta"
    };
    let name_a = entrance(age, 0.2, 0.3);
    faded(v, name_a, |v| {
        shifted(v, [(1.0 - name_a) * -60.0 * s, 0.0], |v| {
            turned(v, [pr.x, pr.y + pr.h], -0.1, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "Rosalía",
                    pr.x + 20.0 * s,
                    pr.y + pr.h - 150.0 * s,
                    100.0 * s,
                    0xffffff,
                    rose,
                    1.0,
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    subtitle,
                    pr.x + 34.0 * s,
                    pr.y + pr.h - 36.0 * s,
                    32.0 * s,
                    rgba(0xffc8dc, 1.0),
                    rgba(0x0b0508, 0.9),
                );
            });
        });
    });
    speech_box(
        font,
        v,
        [pr.x + 30.0 * s, h - 118.0 * s],
        (l.targets.first().map_or(w * 0.42, |(r, _)| r.x) - pr.x - 70.0 * s).min(640.0 * s),
        "Rosalía",
        menu.loan_line(),
        rose,
        -0.02,
        entrance(age, 0.42, 0.25),
        s,
    );

    let hover = |r: Rect, angle: f32| pointer.is_some_and(|(x, y)| inside_turned(r, angle, x, y));
    if menu.progress.debt == 0 {
        // Her two notes, sliding in one after the other; the chosen one lifted, gold-edged.
        for (i, &act) in actions.iter().enumerate() {
            let LoanAction::Borrow(amount) = act else {
                continue;
            };
            let (r, angle) = l.targets[i];
            let a = entrance(age, 0.14 + 0.08 * i as f32, 0.34);
            let lit = menu.choice == i;
            let lift = if lit { -16.0 * s } else { 0.0 };
            faded(v, a, |v| {
                shifted(v, [(1.0 - a) * 320.0 * s, lift], |v| {
                    turned(v, center(r), angle, |v| {
                        promissory_note(
                            font,
                            v,
                            r,
                            amount,
                            menu.progress.loans + 1 + i as u32,
                            lit,
                            hover(r, angle),
                            s,
                        );
                    });
                });
            });
            if lit && a > 0.95 {
                cursor(v, [r.x - 16.0 * s, r.y + r.h * 0.5 + lift], 34.0 * s, time);
            }
        }
        let can = matches!(actions.get(menu.choice), Some(LoanAction::Borrow(_)));
        let sign_label = match actions.get(menu.choice) {
            Some(LoanAction::Borrow(a)) => format!("Firmar · {a} oro"),
            _ => "Firmar el pagaré".to_owned(),
        };
        let sh = pointer.is_some_and(|(x, y)| l.sign.contains(x, y));
        button(
            font,
            v,
            l.sign,
            "Enter",
            &sign_label,
            can,
            sh,
            can,
            accent,
            time,
            s,
        );
        if let Some(&(back, _)) = l.targets.get(actions.len().saturating_sub(1)) {
            let lit = matches!(actions.get(menu.choice), Some(LoanAction::Back));
            button(
                font,
                v,
                back,
                "Esc",
                "Ahora no",
                lit,
                hover(back, 0.0),
                true,
                accent,
                time,
                s,
            );
            if lit {
                cursor(
                    v,
                    [back.x - 14.0 * s, back.y + back.h * 0.5],
                    30.0 * s,
                    time,
                );
            }
        }
    } else {
        // Her ledger's page, then the rows.
        let c = l.ledger;
        let a = entrance(age, 0.12, 0.34);
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 320.0 * s, 0.0], |v| {
                turned(v, center(c), -0.02, |v| ledger_page(font, v, c, menu, s));
            });
        });
        for (i, &act) in actions.iter().enumerate() {
            let (r, angle) = l.targets[i];
            let row = loan_row(menu, act, gold);
            let a = entrance(age, 0.22 + 0.07 * i as f32, 0.3);
            let lit = i == menu.choice;
            let adult = matches!(act, LoanAction::House | LoanAction::Shift);
            faded(v, a, |v| {
                shifted(v, [(1.0 - a) * 280.0 * s, 0.0], |v| {
                    turned(v, center(r), angle, |v| {
                        tile(font, v, r, &row, lit, hover(r, angle), accent, time, s);
                        if adult {
                            let th = 30.0 * s;
                            let tw = warn_tag_width(font, "+18", th);
                            warn_tag(
                                font,
                                v,
                                Rect::new(r.x + r.w - tw - 18.0 * s, r.y + 16.0 * s, tw, th),
                                "+18",
                                s,
                            );
                        }
                    });
                });
            });
            if lit && a > 0.95 {
                let (sn, cs) = angle.sin_cos();
                let half = r.w * 0.5;
                let tip = [
                    r.x + r.w * 0.5 - half * cs - 16.0 * s,
                    r.y + r.h * 0.5 - half * sn,
                ];
                cursor(v, tip, 34.0 * s, time);
            }
        }
    }

    if !menu.notice.is_empty() {
        let at = l
            .targets
            .last()
            .map_or([l.sign.x, l.sign.y + l.sign.h], |(r, _)| [r.x, r.y + r.h]);
        text_shadow(
            font,
            v,
            Face::Italic,
            &menu.notice,
            at[0] + 20.0 * s,
            at[1] + 18.0 * s,
            23.0 * s,
            rgba(0xff8aa0, 1.0),
        );
    }
    purse(
        font,
        v,
        Rect::new(w - 300.0 * s, 46.0 * s, 196.0 * s, 60.0 * s),
        gold,
        s,
    );
}

/// Rosalía's desk (and the jobs page): her portrait on the left as on the loan, her account on a
/// paper card at the top right, and under it a tilted row for each thing to do.
struct DeskLayout {
    s: f32,
    portrait: Rect,
    card: Rect,
    rows: Vec<Rect>,
}

fn desk_count(menu: &Menu) -> usize {
    menu.desk_actions().len()
}

fn desk_layout(w: f32, h: f32, count: usize) -> DeskLayout {
    let o = offer_layout(w, h);
    let s = o.s;
    let portrait = Rect::new(o.portrait.x, o.portrait.y, o.portrait.w, 640.0 * s);
    let x0 = o.card.x;
    let right = (w - 470.0 * s).min(x0 + 690.0 * s);
    let card = Rect::new(x0, 92.0 * s, right - x0, 236.0 * s);
    let row_h = 92.0 * s;
    let gap = 8.0 * s;
    let top = card.y + card.h + 30.0 * s;
    let rows = (0..count)
        .map(|i| {
            Rect::new(
                x0 + 10.0 * s + i as f32 * 14.0 * s,
                top + i as f32 * (row_h + gap),
                right - x0 - 60.0 * s,
                row_h,
            )
        })
        .collect();
    DeskLayout {
        s,
        portrait,
        card,
        rows,
    }
}

/// Her account on a paper card: what is owed and how it stands, or a clean slate and what she
/// lends; the house's work under it.
fn account_card(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, menu: &Menu, s: f32) {
    use super::super::venue::{LOANS, WAY_HOUSE, WAY_MINE};
    let p = &menu.progress;
    paper(v, r, 0.0, false, s);
    paper_band(font, v, r, "SU CUENTA", "El Corazón Carmesí", s);
    let x = r.x + 30.0 * s;
    let right = r.x + r.w - 30.0 * s;
    if p.debt > 0 {
        let big = format!("DEBE {} oro", p.debt);
        text(
            font,
            v,
            Face::Title,
            &big,
            x,
            r.y + 84.0 * s,
            46.0 * s,
            rgba(0xa0102e, 1.0),
        );
        let days = menu.days_left();
        let state = if p.debt_way == WAY_MINE {
            "Encargo en el pozo".to_owned()
        } else if p.debt_way == WAY_HOUSE {
            "Pagando en turnos".to_owned()
        } else if days < 0 {
            "Plazo vencido".to_owned()
        } else if days == 0 {
            "Vence hoy".to_owned()
        } else {
            format!("Vence en {days} días")
        };
        let sw = text_width(font, Face::Italic, &state, 26.0 * s);
        text(
            font,
            v,
            Face::Italic,
            &state,
            right - sw,
            r.y + 98.0 * s,
            26.0 * s,
            rgba(if days < 0 { 0xa0102e } else { LEAF_INK }, 0.9),
        );
        // How much of it is paid.
        let paid = menu.debt_paid();
        let total = (paid + p.debt).max(1) as f32;
        let bar = Rect::new(x, r.y + 136.0 * s, right - x, 14.0 * s);
        panel(v, bar, 7.0 * s, Panel::fill(0xd8c4a4, 0xc8b090, 1.0));
        let k = paid as f32 / total;
        if k > 0.0 {
            panel(
                v,
                Rect::new(bar.x, bar.y, bar.w * k, bar.h),
                7.0 * s,
                Panel::fill(0x3a9a5a, 0x1a6a3a, 1.0),
            );
        }
        text(
            font,
            v,
            Face::Ui,
            &format!("Pagado {paid} de {}", paid + p.debt),
            x,
            bar.y + 22.0 * s,
            19.0 * s,
            rgba(LEAF_INK, 0.75),
        );
    } else {
        text(
            font,
            v,
            Face::Title,
            "Sin deudas",
            x,
            r.y + 84.0 * s,
            46.0 * s,
            rgba(0x1a6a3a, 1.0),
        );
        text(
            font,
            v,
            Face::Italic,
            &format!("La casa presta {} o {} oro, +30 %", LOANS[0], LOANS[1]),
            x,
            r.y + 138.0 * s,
            24.0 * s,
            rgba(LEAF_INK, 0.8),
        );
    }
    let jobs_line = if p.jobs_open {
        format!(
            "Clientes abiertos · {} turnos · {} encargos",
            p.jobs_shifts, p.jobs_errands
        )
    } else {
        "Clientes: se abren al saldar una deuda trabajando".to_owned()
    };
    text(
        font,
        v,
        Face::Italic,
        &jobs_line,
        x,
        r.y + r.h - 38.0 * s,
        21.0 * s,
        rgba(LEAF_INK, 0.7),
    );
}

/// Rosalía's desk, or the house's jobs: her portrait and word, the account, the rows.
#[allow(clippy::too_many_arguments)]
fn draw_desk(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    host_face: Option<Uv>,
    time: f32,
) {
    let rows = menu.desk_rows(gold);
    let l = desk_layout(w, h, rows.len());
    let s = l.s;
    let age = time - menu.page_at_ms as f32 / 1000.0;
    let rose = 0xd83c78;
    let accent = rose;
    backdrop(v, w, h, 0x6a0a3a, 0xff4f9a, age, time, s);
    cut_in(v, l.portrait, host_face, rose, age, s);
    let pr = l.portrait;
    let subtitle = if menu.progress.debt > 0 {
        "Su cuenta con la casa"
    } else {
        "Asuntos de la casa"
    };
    let name_a = entrance(age, 0.2, 0.3);
    faded(v, name_a, |v| {
        shifted(v, [(1.0 - name_a) * -60.0 * s, 0.0], |v| {
            turned(v, [pr.x, pr.y + pr.h], -0.1, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "Rosalía",
                    pr.x + 20.0 * s,
                    pr.y + pr.h - 150.0 * s,
                    100.0 * s,
                    0xffffff,
                    rose,
                    1.0,
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    subtitle,
                    pr.x + 34.0 * s,
                    pr.y + pr.h - 36.0 * s,
                    32.0 * s,
                    rgba(0xffc8dc, 1.0),
                    rgba(0x0b0508, 0.9),
                );
            });
        });
    });
    speech_box(
        font,
        v,
        [pr.x + 30.0 * s, h - 118.0 * s],
        (l.rows.first().map_or(w * 0.42, |r| r.x) - pr.x - 70.0 * s).min(640.0 * s),
        "Rosalía",
        menu.desk_line(),
        rose,
        -0.02,
        entrance(age, 0.42, 0.25),
        s,
    );
    // Her account, sliding in.
    let c = l.card;
    let a = entrance(age, 0.12, 0.34);
    faded(v, a, |v| {
        shifted(v, [(1.0 - a) * 320.0 * s, 0.0], |v| {
            turned(v, center(c), -0.015, |v| account_card(font, v, c, menu, s));
        });
    });
    let hover = |r: Rect| pointer.is_some_and(|(x, y)| inside_turned(r, TILE_ANGLE, x, y));
    for (i, (row, adult)) in rows.iter().enumerate() {
        let r = l.rows[i];
        let a = entrance(age, 0.2 + 0.06 * i as f32, 0.3);
        let lit = i == menu.choice;
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 280.0 * s, 0.0], |v| {
                turned(v, center(r), TILE_ANGLE, |v| {
                    tile(font, v, r, row, lit, hover(r), accent, time, s);
                    if *adult {
                        let th = 30.0 * s;
                        let tw = warn_tag_width(font, "+18", th);
                        warn_tag(
                            font,
                            v,
                            Rect::new(r.x + r.w - tw - 18.0 * s, r.y + 14.0 * s, tw, th),
                            "+18",
                            s,
                        );
                    }
                });
            });
        });
        if lit && a > 0.95 {
            let (sn, cs) = TILE_ANGLE.sin_cos();
            let half = r.w * 0.5;
            let tip = [
                r.x + r.w * 0.5 - half * cs - 16.0 * s,
                r.y + r.h * 0.5 - half * sn,
            ];
            cursor(v, tip, 32.0 * s, time);
        }
    }
    if !menu.notice.is_empty() {
        let at = l.rows.last().map_or([c.x, c.y + c.h], |r| [r.x, r.y + r.h]);
        text_shadow(
            font,
            v,
            Face::Italic,
            &menu.notice,
            at[0] + 20.0 * s,
            at[1] + 16.0 * s,
            23.0 * s,
            rgba(0xff8aa0, 1.0),
        );
    }
    purse(
        font,
        v,
        Rect::new(w - 300.0 * s, 46.0 * s, 196.0 * s, 60.0 * s),
        gold,
        s,
    );
}

/// How far each client's card leans and how far down it sits.
const CLIENT_TILT: [f32; 5] = [-0.034, 0.022, -0.026, 0.03, -0.02];
const CLIENT_DROP: [f32; 5] = [0.0, 18.0, 6.0, 22.0, 10.0];

/// Where everything is on the clients' page.
struct ClientsLayout {
    s: f32,
    cards: Vec<Rect>,
    serve: Rect,
    back: Rect,
    purse: Rect,
    host: Rect,
}

fn clients_layout(w: f32, h: f32, count: usize) -> ClientsLayout {
    let s = h / 1080.0;
    let count = count.clamp(1, 5);
    let gap = 30.0 * s;
    let card_w = ((w - 240.0 * s - gap * (count as f32 - 1.0)) / count as f32).min(330.0 * s);
    let total = card_w * count as f32 + gap * (count as f32 - 1.0);
    let x0 = w * 0.5 - total * 0.5;
    let cards = (0..count)
        .map(|i| {
            Rect::new(
                x0 + i as f32 * (card_w + gap),
                (214.0 + CLIENT_DROP[i]) * s,
                card_w,
                600.0 * s,
            )
        })
        .collect();
    let row = h - 112.0 * s;
    let back = Rect::new(w - 440.0 * s, row, 200.0 * s, 66.0 * s);
    let serve = Rect::new(back.x - 26.0 * s - 520.0 * s, row, 520.0 * s, 66.0 * s);
    ClientsLayout {
        s,
        cards,
        serve,
        back,
        purse: Rect::new(w - 300.0 * s, 46.0 * s, 196.0 * s, 60.0 * s),
        host: Rect::new(w * 0.5 - 10.0 * s, 42.0 * s, 104.0 * s, 104.0 * s),
    }
}

/// One client's card: his face with his name thrown over it and how often he has been served (new,
/// or ×N), what he is on a tag in his colour, a line on him, what he asks for (+18) and what he
/// pays; locked (no clips yet) under a veil with «PRÓXIMAMENTE».
#[allow(clippy::too_many_arguments)]
fn client_card(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    c: &ClientCard,
    uv: Option<Uv>,
    served: u32,
    debt: bool,
    chosen: bool,
    hover: bool,
    s: f32,
) {
    use super::super::venue::SHIFT_PAY;
    let accent = c.accent;
    let lit = chosen && c.ready;
    soft(
        v,
        Rect::new(r.x + 12.0 * s, r.y + 16.0 * s, r.w, r.h),
        14.0 * s,
        18.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    if lit {
        soft(v, r, 16.0 * s, 26.0 * s, rgba(accent, 0.55), true);
    }
    panel(
        v,
        grow(r, 5.0 * s),
        16.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    panel(
        v,
        r,
        12.0 * s,
        Panel::fill(
            mix(0x2a0f24, accent, if lit { 0.22 } else { 0.08 }),
            0x0e060c,
            1.0,
        )
        .rim(
            if lit {
                GOLD_HI
            } else if hover {
                GOLD
            } else {
                mix(GOLD_DEEP, accent, 0.4)
            },
            1.0,
            if lit { 3.0 } else { 2.0 } * s,
        ),
    );
    // His face, his name over its foot.
    let face_r = Rect::new(
        r.x + 14.0 * s,
        r.y + 14.0 * s,
        r.w - 28.0 * s,
        r.w - 28.0 * s,
    );
    panel(v, face_r, 8.0 * s, Panel::fill(0x1a0a16, 0x0b0508, 1.0));
    if let Some(uv) = uv {
        face_window(v, face_r, 8.0 * s, uv, 1.0);
    }
    if !c.ready {
        panel(v, face_r, 8.0 * s, Panel::fill(0x0b0508, 0x0b0508, 0.62));
        let size = 26.0 * s;
        let tw = text_width(font, Face::Title, "PRÓXIMAMENTE", size) + 30.0 * s;
        let t = Rect::new(
            face_r.x + (face_r.w - tw) * 0.5,
            face_r.y + face_r.h * 0.42,
            tw,
            44.0 * s,
        );
        panel(
            v,
            t,
            6.0 * s,
            Panel::fill(0x3a2a4a, 0x1a1024, 0.95).rim(GOLD, 1.0, 2.0 * s),
        );
        text_centered(
            font,
            v,
            Face::Title,
            "PRÓXIMAMENTE",
            t.x + t.w * 0.5,
            t.y + (t.h - size) * 0.5 - 2.0 * s,
            size,
            rgba(CREAM, 1.0),
        );
    }
    shout(
        font,
        v,
        Face::Title,
        c.name,
        face_r.x + 14.0 * s,
        face_r.y + face_r.h - 74.0 * s,
        62.0 * s,
        0xffffff,
        accent,
        if c.ready { 1.0 } else { 0.7 },
    );
    // How often she has served him: new, or a count.
    if c.ready {
        let (tag, fill) = if served == 0 {
            ("NUEVO".to_owned(), 0xffd56b)
        } else {
            (format!("×{served}"), accent)
        };
        let ts = 17.0 * s;
        let tw = text_width(font, Face::Title, &tag, ts) + 22.0 * s;
        let t = Rect::new(
            face_r.x + face_r.w - tw - 8.0 * s,
            face_r.y + 8.0 * s,
            tw,
            30.0 * s,
        );
        panel(
            v,
            t,
            5.0 * s,
            Panel::fill(fill, mix(fill, 0x000000, 0.25), 1.0).rim(0x0b0508, 1.0, 2.0 * s),
        );
        text_centered(
            font,
            v,
            Face::Title,
            &tag,
            t.x + t.w * 0.5,
            t.y + (t.h - ts) * 0.5 - 1.0 * s,
            ts,
            rgba(0x1a0a10, 1.0),
        );
    }
    // What he is, on a tag of his colour.
    let mut y = face_r.y + face_r.h + 14.0 * s;
    let ts = 20.0 * s;
    let tw = (text_width(font, Face::Italic, c.title, ts) + 30.0 * s).min(r.w - 28.0 * s);
    panel(
        v,
        Rect::new(r.x + 14.0 * s, y, tw, 34.0 * s),
        6.0 * s,
        Panel::fill(mix(0x1a0a16, accent, 0.35), 0x12060e, 1.0).rim(accent, 1.0, 1.6 * s),
    );
    text(
        font,
        v,
        Face::Italic,
        c.title,
        r.x + 29.0 * s,
        y + (34.0 * s - ts) * 0.5 - 2.0 * s,
        ts,
        rgba(CREAM, 1.0),
    );
    y += 46.0 * s;
    // A line on him.
    let size = 17.0 * s;
    for line in wrap(font, Face::Body, c.about, size, r.w - 32.0 * s)
        .iter()
        .take(2)
    {
        text(
            font,
            v,
            Face::Body,
            line,
            r.x + 16.0 * s,
            y,
            size,
            rgba(CREAM, 0.9),
        );
        y += 24.0 * s;
    }
    // What he asks for, and what he pays.
    let foot = r.y + r.h - 50.0 * s;
    if c.ready {
        let ask = format!("Pide: {}", c.act);
        text(
            font,
            v,
            Face::Italic,
            &ask,
            r.x + 16.0 * s,
            foot - 38.0 * s,
            18.0 * s,
            rgba(mix(CREAM, accent, 0.4), 1.0),
        );
        let th = 26.0 * s;
        let ww = warn_tag_width(font, "+18", th);
        warn_tag(
            font,
            v,
            Rect::new(r.x + r.w - ww - 14.0 * s, foot - 40.0 * s, ww, th),
            "+18",
            s,
        );
        coin(v, [r.x + 30.0 * s, foot + 16.0 * s], 12.0 * s, 1.0);
        let pay = if debt {
            format!("−{SHIFT_PAY} de deuda")
        } else {
            format!("+{SHIFT_PAY} oro y propina")
        };
        let ps = 19.0 * s;
        let mut size = ps;
        while size > 13.0 * s && text_width(font, Face::Ui, &pay, size) > r.w - 70.0 * s {
            size -= 0.5 * s;
        }
        text(
            font,
            v,
            Face::Ui,
            &pay,
            r.x + 50.0 * s,
            foot + 16.0 * s - size * 0.6,
            size,
            rgba(0xffe08a, 1.0),
        );
    }
}

/// Rosalía's clients: the house's name across the top as on the girls' page, the clients dealt as
/// cards, the chosen one lifted with the cursor at his side, her word at the foot, and the keys.
#[allow(clippy::too_many_arguments)]
fn draw_clients(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    faces: &[Option<Uv>],
    host_face: Option<Uv>,
    time: f32,
) {
    use super::super::venue::SHIFT_PAY;
    let l = clients_layout(w, h, ROSTER.len());
    let s = l.s;
    let age = time - menu.page_at_ms as f32 / 1000.0;
    let chosen = ROSTER.get(menu.choice);
    let accent = chosen.map_or(CRIMSON, |c| c.accent);
    backdrop(v, w, h, 0x3a0c2a, accent, age, time, s);

    // The page's name thrown across the top, the book's ribbon under it.
    let ta = entrance(age, 0.04, 0.3);
    faded(v, ta, |v| {
        shifted(v, [-(1.0 - ta) * 90.0 * s, 0.0], |v| {
            turned(v, [440.0 * s, 100.0 * s], -0.045, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "Clientes de la casa",
                    96.0 * s,
                    40.0 * s,
                    72.0 * s,
                    0xffffff,
                    0x8a1030,
                    1.0,
                );
                let tag = "EL LIBRO DE ROSALÍA";
                let tw = text_width(font, Face::Title, tag, 22.0 * s);
                let rib = Rect::new(108.0 * s, 124.0 * s, tw + 44.0 * s, 40.0 * s);
                panel(
                    v,
                    grow(rib, 4.0 * s),
                    6.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 1.0),
                );
                panel(
                    v,
                    rib,
                    4.0 * s,
                    Panel::fill(0xe8306a, 0x9a1040, 1.0).sheen(time * 0.3),
                );
                text(
                    font,
                    v,
                    Face::Title,
                    tag,
                    rib.x + 22.0 * s,
                    rib.y + 8.0 * s,
                    22.0 * s,
                    rgba(0xffffff, 1.0),
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    "¿A quién atiendes esta noche?",
                    rib.x + rib.w + 20.0 * s,
                    rib.y + 4.0 * s,
                    27.0 * s,
                    rgba(CREAM, 1.0),
                    rgba(0x0b0508, 0.9),
                );
            });
        });
    });
    // Rosalía, beside the title.
    let host = l.host;
    faded(v, entrance(age, 0.12, 0.3), |v| {
        soft(v, host, host.w * 0.5, 14.0 * s, rgba(0xd83c78, 0.32), true);
        panel(
            v,
            grow(host, 4.0 * s),
            host.w * 0.5 + 4.0 * s,
            Panel::fill(0x3a1230, 0x12060e, 1.0).rim(GOLD, 1.0, 2.6 * s),
        );
        if let Some(uv) = host_face {
            face(v, host, host.w * 0.5, uv, 1.0);
        }
        text_shadow(
            font,
            v,
            Face::Title,
            "Rosalía",
            host.x + host.w + 18.0 * s,
            host.y + 26.0 * s,
            26.0 * s,
            rgba(0xf0a8c8, 1.0),
        );
    });
    purse(font, v, l.purse, gold, s);

    // The clients, dealt one after another; the chosen one on top.
    let debt = menu.progress.debt > 0;
    let paint = |v: &mut Vec<MapVertex>, i: usize, c: &ClientCard, chosen: bool| {
        let Some(base) = l.cards.get(i).copied() else {
            return;
        };
        let tilt = CLIENT_TILT[i];
        let hover = pointer.is_some_and(|(x, y)| inside_turned(base, tilt, x, y));
        let a = entrance(age, 0.08 + 0.07 * i as f32, 0.34);
        let lift = if chosen { -14.0 * s } else { 0.0 };
        faded(v, a, |v| {
            shifted(
                v,
                [(1.0 - a) * 60.0 * s, (1.0 - a) * 70.0 * s + lift],
                |v| {
                    turned(v, center(base), tilt, |v| {
                        client_card(
                            font,
                            v,
                            base,
                            c,
                            faces.get(i).copied().flatten(),
                            menu.served(c.id),
                            debt,
                            chosen,
                            hover,
                            s,
                        );
                    });
                },
            );
        });
    };
    for (i, c) in ROSTER.iter().enumerate() {
        if i != menu.choice {
            paint(v, i, c, false);
        }
    }
    if let Some(c) = chosen {
        paint(v, menu.choice, c, true);
        if let Some(&r) = l.cards.get(menu.choice) {
            if entrance(age, 0.08 + 0.07 * menu.choice as f32, 0.34) > 0.95 {
                let tip = turn_point(
                    [r.x - 14.0 * s, r.y + 150.0 * s],
                    center(r),
                    CLIENT_TILT[menu.choice],
                );
                cursor(v, tip, 34.0 * s, time);
            }
        }
    }

    // Her word (or the notice) at the foot.
    let words = if menu.notice.is_empty() {
        menu.desk_line()
    } else {
        menu.notice.as_str()
    };
    speech_box(
        font,
        v,
        [96.0 * s, h - 40.0 * s],
        (l.serve.x - 140.0 * s).min(560.0 * s),
        "Rosalía",
        words,
        0xd83c78,
        -0.015,
        entrance(age, 0.3, 0.25),
        s,
    );

    // The keys.
    let hover = |r: Rect| pointer.is_some_and(|(x, y)| r.contains(x, y));
    let (label, can) = match chosen {
        Some(c) if c.ready => (
            if debt {
                format!("Atender a {} · −{SHIFT_PAY} de deuda", c.name)
            } else {
                format!("Atender a {} · +{SHIFT_PAY} oro", c.name)
            },
            true,
        ),
        Some(c) => (format!("{} · pronto", c.name), false),
        None => ("Atender".to_owned(), false),
    };
    button(
        font,
        v,
        l.serve,
        "Enter",
        &label,
        true,
        hover(l.serve),
        can,
        accent,
        time,
        s,
    );
    button(
        font,
        v,
        l.back,
        "Esc",
        "Volver",
        false,
        hover(l.back),
        true,
        accent,
        time,
        s,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_girls(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    faces: &[Option<Uv>],
    host_face: Option<Uv>,
    time: f32,
) {
    let entries = menu.entries();
    let l = layout(w, h, entries.len());
    let s = l.s;
    let age = time - menu.page_at_ms as f32 / 1000.0;
    let hover = |r: Rect| pointer.is_some_and(|(x, y)| r.contains(x, y));
    let selected = entries.get(menu.selected);
    let accent = selected.map_or(CRIMSON, |e| hex(&e.accent, CRIMSON));

    // The house behind, hushed; slashes of the chosen girl's colour sweeping in under the cards.
    backdrop(v, w, h, 0x5a0c22, accent, age, time, s);

    // The house's name thrown across the top, its book on a ribbon under it.
    let ta = entrance(age, 0.04, 0.3);
    faded(v, ta, |v| {
        shifted(v, [-(1.0 - ta) * 90.0 * s, 0.0], |v| {
            turned(v, [440.0 * s, 100.0 * s], -0.045, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "El Corazón Carmesí",
                    96.0 * s,
                    40.0 * s,
                    72.0 * s,
                    0xffffff,
                    0x8a1030,
                    1.0,
                );
                let tag = "EL LIBRO DE ROSALÍA";
                let tw = text_width(font, Face::Title, tag, 22.0 * s);
                let rib = Rect::new(108.0 * s, 124.0 * s, tw + 44.0 * s, 40.0 * s);
                panel(
                    v,
                    grow(rib, 4.0 * s),
                    6.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 1.0),
                );
                panel(
                    v,
                    rib,
                    4.0 * s,
                    Panel::fill(0xe8306a, 0x9a1040, 1.0).sheen(time * 0.3),
                );
                text(
                    font,
                    v,
                    Face::Title,
                    tag,
                    rib.x + 22.0 * s,
                    rib.y + 8.0 * s,
                    22.0 * s,
                    rgba(0xffffff, 1.0),
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    "¿Quién te acompaña esta noche?",
                    rib.x + rib.w + 20.0 * s,
                    rib.y + 4.0 * s,
                    27.0 * s,
                    rgba(CREAM, 1.0),
                    rgba(0x0b0508, 0.9),
                );
            });
        });
    });

    // Rosalía, beside the title.
    let host = l.host;
    faded(v, entrance(age, 0.12, 0.3), |v| {
        soft(v, host, host.w * 0.5, 14.0 * s, rgba(0xd83c78, 0.32), true);
        panel(
            v,
            grow(host, 4.0 * s),
            host.w * 0.5 + 4.0 * s,
            Panel::fill(0x3a1230, 0x12060e, 1.0).rim(GOLD, 1.0, 2.6 * s),
        );
        if let Some(uv) = host_face {
            face(v, host, host.w * 0.5, uv, 1.0);
        }
        let tx = host.x + host.w + 18.0 * s;
        text_shadow(
            font,
            v,
            Face::Title,
            "Rosalía",
            tx,
            host.y + 10.0 * s,
            26.0 * s,
            rgba(0xf0a8c8, 1.0),
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            "—Elige, cariño.",
            tx,
            host.y + 46.0 * s,
            21.0 * s,
            rgba(CREAM, 0.95),
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            "Todas son de confianza.",
            tx,
            host.y + 72.0 * s,
            21.0 * s,
            rgba(CREAM, 0.95),
        );
    });

    // The purse, and once Rosalía has offered it, her elixir with its warning.
    purse(font, v, l.purse, gold, s);
    if menu.progress.elixir_offered {
        elixir_sticker(font, v, l.elixir, menu, hover(l.elixir), time, s);
    }

    // The girls, dealt one after another when the book opens; the chosen one on top.
    let paint_card =
        |v: &mut Vec<MapVertex>, i: usize, e: &super::super::venue::Entry, chosen: bool| {
            let Some(base) = l.cards.get(i).copied() else {
                return;
            };
            let tilt = CARD_TILT[i];
            let lit = pointer.is_some_and(|(x, y)| inside_turned(base, tilt, x, y));
            let a = entrance(age, 0.08 + 0.07 * i as f32, 0.34);
            faded(v, a, |v| {
                shifted(v, [(1.0 - a) * 60.0 * s, (1.0 - a) * 70.0 * s], |v| {
                    turned(v, center(base), tilt, |v| {
                        card(
                            font,
                            v,
                            base,
                            i,
                            e,
                            faces.get(i).copied().flatten(),
                            menu,
                            chosen,
                            lit,
                            gold,
                            time,
                            s,
                        );
                    });
                });
            });
        };
    for (i, e) in entries.iter().enumerate() {
        if i != menu.selected {
            paint_card(v, i, e, false);
        }
    }
    if let Some(e) = selected {
        paint_card(v, menu.selected, e, true);
        // The cursor at her side once she has landed.
        if let Some(&r) = l.cards.get(menu.selected) {
            if entrance(age, 0.08 + 0.07 * menu.selected as f32, 0.34) > 0.95 {
                let tip = turn_point(
                    [r.x - 16.0 * s, r.y + 170.0 * s],
                    center(r),
                    CARD_TILT[menu.selected],
                );
                cursor(v, tip, 36.0 * s, time);
            }
        }
    }

    // Her word (or Rosalía's, on the gold) in a speech box at the foot.
    if let Some(e) = selected {
        let (who, words, tone) = if !menu.notice.is_empty() && menu.notice != e.quote {
            ("Rosalía", menu.notice.as_str(), 0xd83c78)
        } else {
            (e.name.as_str(), e.quote.as_str(), accent)
        };
        if !words.is_empty() {
            speech_box(
                font,
                v,
                [96.0 * s, h - 40.0 * s],
                540.0 * s,
                who,
                words,
                tone,
                -0.015,
                entrance(age, 0.3, 0.25),
                s,
            );
        }
    }

    // The keys.
    let (choose_label, can) = match selected {
        Some(e) => {
            let price = if e.company_price > 0 {
                e.company_price
            } else {
                e.price
            };
            (format!("Elegir a {} · {price} oro", e.name), gold >= price)
        }
        None => ("Elegir".to_owned(), false),
    };
    button(
        font,
        v,
        l.choose,
        "Enter",
        &choose_label,
        true,
        hover(l.choose),
        can,
        accent,
        time,
        s,
    );
    // Rosalía's desk: the account, the loan and the house's work (what is owed on a red tag).
    button(
        font,
        v,
        l.desk,
        "R",
        "Rosalía",
        false,
        hover(l.desk),
        true,
        0xd83c78,
        time,
        s,
    );
    let owed = menu.progress.debt;
    let tag = if owed > 0 {
        format!("DEBES {owed}")
    } else if menu.progress.jobs_open {
        "TRABAJO".to_owned()
    } else {
        "CUENTA".to_owned()
    };
    let th = 28.0 * s;
    let tw = warn_tag_width(font, &tag, th);
    if owed > 0 {
        warn_tag(
            font,
            v,
            Rect::new(
                l.desk.x + l.desk.w - tw + 10.0 * s,
                l.desk.y - th * 0.6,
                tw,
                th,
            ),
            &tag,
            s,
        );
    } else {
        let r = Rect::new(
            l.desk.x + l.desk.w - tw + 10.0 * s,
            l.desk.y - th * 0.6,
            tw,
            th,
        );
        panel(
            v,
            r,
            4.0 * s,
            Panel::fill(0xd83c78, 0x8a1040, 1.0).rim(0x0b0508, 1.0, 2.0 * s),
        );
        text_centered(
            font,
            v,
            Face::Title,
            &tag,
            r.x + r.w * 0.5,
            r.y + (th - 16.0 * s) * 0.5 - 1.0 * s,
            16.0 * s,
            rgba(0xffffff, 1.0),
        );
    }
    button(
        font,
        v,
        l.close,
        "Esc",
        "Cerrar",
        false,
        hover(l.close),
        true,
        accent,
        time,
        s,
    );
    // Left and right choose: two small chevrons and the word.
    let hint = "elegir";
    let hw = text_width(font, Face::Italic, hint, 19.0 * s);
    let hx = l.choose.x - 34.0 * s - hw;
    let hy = l.choose.y + l.choose.h * 0.5;
    text_shadow(
        font,
        v,
        Face::Italic,
        hint,
        hx,
        hy - 13.0 * s,
        19.0 * s,
        rgba(DIM, 1.0),
    );
    for (k, dir) in [(0.0f32, -1.0f32), (1.0, 1.0)] {
        let c = [hx - 46.0 * s + k * 22.0 * s, hy];
        let tip = [c[0] + dir * 6.0 * s, c[1]];
        for side in [-1.0f32, 1.0] {
            wire(
                v,
                [c[0] - dir * 3.0 * s, c[1] + side * 7.0 * s],
                tip,
                2.6 * s,
                rgba(GOLD, 0.9),
                rgba(GOLD, 0.9),
                0.0,
                true,
            );
        }
    }
}

/// The seal: a ring of runes round a spark, pink and breathing when it is awake.
fn seal_icon(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, awake: bool, time: f32, s: f32) {
    let pink = 0xff4f9a;
    let beat = if awake {
        0.75 + 0.25 * wave(time, 1.1)
    } else {
        0.0
    };
    if awake {
        halo(v, c, r, 10.0 * s, rgba(pink, 0.45 * beat));
    }
    let ink = if awake { 0xffc2dc } else { 0xb8a2c4 };
    arc(
        v,
        c,
        r * 0.82,
        (r * 0.16).max(1.2),
        time * 0.15,
        0.86,
        rgba(ink, 1.0),
        rgba(ink, 0.25),
    );
    sparkle(
        v,
        c,
        r * 0.62,
        rgba(if awake { 0xffffff } else { 0xd8c8e0 }, 1.0),
        if awake { 0.8 * beat } else { 0.2 },
    );
}

/// A tile's emblem: its medallion and the drawing on it.
#[allow(clippy::too_many_arguments)]
fn emblem(
    v: &mut Vec<MapVertex>,
    kind: &str,
    c: [f32; 2],
    r: f32,
    lit: bool,
    locked: bool,
    awake: bool,
    accent: u32,
    time: f32,
    s: f32,
) {
    let ring = if locked {
        0x6a5a74
    } else if lit {
        GOLD_HI
    } else {
        GOLD
    };
    panel(
        v,
        Rect::new(c[0] - r, c[1] - r, r * 2.0, r * 2.0),
        r,
        Panel::fill(
            mix(0x2a1238, accent, if lit { 0.4 } else { 0.18 }),
            0x0b0508,
            1.0,
        )
        .rim(ring, 1.0, 2.4 * s)
        .bevel(0.8),
    );
    let ink = rgba(if locked { 0x9a8aa6 } else { 0xfff2f6 }, 1.0);
    let k = r * 0.5;
    match kind {
        "seal" => seal_icon(v, c, r * 0.72, awake, time, s),
        "heart" => {
            disc(v, [c[0] - k * 0.48, c[1] - k * 0.25], k * 0.55, ink);
            disc(v, [c[0] + k * 0.48, c[1] - k * 0.25], k * 0.55, ink);
            push_triangle(
                v,
                [
                    [c[0] - k * 1.02, c[1] - k * 0.1],
                    [c[0] + k * 1.02, c[1] - k * 0.1],
                    [c[0], c[1] + k * 0.95],
                ],
                ink,
            );
        }
        "chat" => {
            panel(
                v,
                Rect::new(c[0] - k, c[1] - k * 0.75, k * 2.0, k * 1.3),
                k * 0.45,
                Panel::fill(0xfff2f6, 0xf0dce6, ink[3]),
            );
            push_triangle(
                v,
                [
                    [c[0] - k * 0.5, c[1] + k * 0.4],
                    [c[0] - k * 0.05, c[1] + k * 0.4],
                    [c[0] - k * 0.75, c[1] + k * 1.0],
                ],
                ink,
            );
            for i in 0..3 {
                disc(
                    v,
                    [c[0] - k * 0.5 + i as f32 * k * 0.5, c[1] - k * 0.1],
                    k * 0.13,
                    rgba(0x3a1230, 1.0),
                );
            }
        }
        "shout" => {
            // A comic shout: a burst with an exclamation mark.
            burst(
                v,
                c,
                k * 0.62,
                k * 1.05,
                9,
                time * 0.4,
                ink,
                rgba(0x3a1230, 1.0),
                0.0,
            );
            panel(
                v,
                Rect::new(c[0] - k * 0.11, c[1] - k * 0.55, k * 0.22, k * 0.7),
                k * 0.1,
                Panel::fill(0x3a1230, 0x3a1230, 1.0),
            );
            disc(v, [c[0], c[1] + k * 0.42], k * 0.13, rgba(0x3a1230, 1.0));
        }
        "candle" => {
            panel(
                v,
                Rect::new(c[0] - k * 0.32, c[1] - k * 0.1, k * 0.64, k * 1.05),
                k * 0.12,
                Panel::fill(0xfff2e0, 0xd8c4a8, 1.0),
            );
            let flicker = 1.0 + 0.12 * wave(time, 3.1);
            halo(
                v,
                [c[0], c[1] - k * 0.5],
                k * 0.3,
                k * 0.5,
                rgba(0xffb24a, 0.5),
            );
            gem_turned(
                v,
                [c[0], c[1] - k * 0.5],
                k * 0.22,
                k * 0.42 * flicker,
                0.0,
                0xffc860,
                0xffe0a0,
                1.0,
                0.0,
            );
        }
        "coin" => {
            coin(v, [c[0] - k * 0.32, c[1] + k * 0.22], k * 0.62, ink[3]);
            coin(v, [c[0] + k * 0.3, c[1] - k * 0.22], k * 0.62, ink[3]);
        }
        "pick" => {
            // A miner's pick: the haft from bottom left, the iron head across its top.
            let a = [c[0] - k * 0.72, c[1] + k * 0.78];
            let b = [c[0] + k * 0.42, c[1] - k * 0.48];
            let wood = rgba(0xd8a066, ink[3]);
            wire(v, a, b, k * 0.24, wood, wood, 0.0, false);
            let d = [1.14f32 / 1.7, -1.26f32 / 1.7];
            let n = [-d[1], d[0]];
            let iron = rgba(0xe8eef4, ink[3]);
            for side in [-1.0f32, 1.0] {
                let tip = [
                    b[0] + side * n[0] * k * 0.82 - d[0] * k * 0.3,
                    b[1] + side * n[1] * k * 0.82 - d[1] * k * 0.3,
                ];
                wire(v, b, tip, k * 0.28, iron, iron, 0.0, false);
            }
        }
        "mask" => {
            // A masquerade half-mask with its two eyes and its ribbons.
            panel(
                v,
                Rect::new(c[0] - k * 1.0, c[1] - k * 0.4, k * 2.0, k * 0.8),
                k * 0.4,
                Panel::fill(0xfff2f6, 0xf0c8dc, ink[3]),
            );
            for side in [-1.0f32, 1.0] {
                disc(
                    v,
                    [c[0] + side * k * 0.46, c[1] - k * 0.02],
                    k * 0.2,
                    rgba(0x3a1230, 1.0),
                );
                wire(
                    v,
                    [c[0] + side * k * 0.95, c[1]],
                    [c[0] + side * k * 1.1, c[1] + k * 0.75],
                    k * 0.12,
                    ink,
                    ink,
                    0.0,
                    false,
                );
            }
        }
        "door" => {
            // A shut door with an arched top and its knob.
            let r = Rect::new(c[0] - k * 0.55, c[1] - k * 0.88, k * 1.1, k * 1.76);
            panel(v, r, k * 0.55, Panel::fill(0xfff2f6, 0xe8d0dc, ink[3]));
            panel(
                v,
                Rect::new(r.x, c[1] - k * 0.1, r.w, k * 0.98),
                k * 0.06,
                Panel::fill(0xe8d0dc, 0xe8d0dc, ink[3]),
            );
            panel(
                v,
                grow(r, -k * 0.16),
                k * 0.42,
                Panel::fill(0x5a2440, 0x3a1230, 1.0),
            );
            disc(
                v,
                [c[0] + k * 0.22, c[1] + k * 0.18],
                k * 0.1,
                rgba(GOLD_HI, 1.0),
            );
        }
        _ => {
            // Back: a chevron to the left.
            for side in [-1.0f32, 1.0] {
                wire(
                    v,
                    [c[0] + k * 0.35, c[1] + side * k * 0.6],
                    [c[0] - k * 0.4, c[1]],
                    k * 0.28,
                    ink,
                    ink,
                    0.0,
                    false,
                );
            }
        }
    }
}

/// One tile of what to ask: lit, it glows in her colour (in pink if the seal is awake on it);
/// locked, it dims behind a padlock.
#[allow(clippy::too_many_arguments)]
fn tile(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    ask: &Asking,
    lit: bool,
    hover: bool,
    accent: u32,
    time: f32,
    s: f32,
) {
    let pink = 0xff4f9a;
    let glow = if ask.awake { pink } else { accent };
    let (top, bottom, rim) = if ask.locked {
        (
            if lit { 0x2c1c32 } else { 0x1c1420 },
            0x0e0a12,
            if lit { 0xb8a2c4 } else { 0x6a5a74 },
        )
    } else if lit {
        (
            mix(0x3a0f2a, glow, 0.42),
            mix(0x14040c, glow, 0.16),
            GOLD_HI,
        )
    } else if hover {
        (0x2c1236, 0x12060e, GOLD)
    } else {
        (0x1e0c24, 0x0b0508, GOLD_DEEP)
    };
    soft(
        v,
        Rect::new(r.x + 12.0 * s, r.y + 14.0 * s, r.w, r.h),
        10.0 * s,
        14.0 * s,
        rgba(SHADOW, 0.7),
        false,
    );
    if lit {
        soft(
            v,
            r,
            10.0 * s,
            24.0 * s,
            rgba(glow, 0.42 + 0.12 * wave(time, 0.8)),
            true,
        );
    }
    panel(
        v,
        grow(r, 4.0 * s),
        11.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    let mut style = Panel::fill(top, bottom, 0.98)
        .rim(rim, 1.0, 2.4 * s)
        .hairline(6.0 * s, if lit { 0.5 } else { 0.22 })
        .bevel(0.6);
    if lit && !ask.locked {
        style = style.sheen(time * 0.25);
    }
    panel(v, r, 8.0 * s, style);
    // A stripe of her colour down its left edge.
    panel(
        v,
        Rect::new(r.x, r.y, 12.0 * s, r.h),
        3.0 * s,
        Panel::fill(
            mix(glow, 0xffffff, 0.25),
            glow,
            if ask.locked { 0.4 } else { 1.0 },
        ),
    );
    let c = [r.x + 66.0 * s, r.y + r.h * 0.5];
    emblem(
        v,
        &ask.icon,
        c,
        34.0 * s,
        lit,
        ask.locked,
        ask.awake,
        accent,
        time,
        s,
    );
    let ink = if ask.locked { 0xb0a0ba } else { INK };
    text_outline(
        font,
        v,
        Face::Title,
        &ask.label,
        r.x + 120.0 * s,
        r.y + 18.0 * s,
        38.0 * s,
        rgba(ink, 1.0),
        rgba(0x0b0508, 0.85),
    );
    let hint_ink = if ask.awake {
        0xffa8d0
    } else if ask.locked {
        0xe892a8
    } else {
        CREAM
    };
    text_shadow(
        font,
        v,
        Face::Italic,
        &ask.hint,
        r.x + 122.0 * s,
        r.y + 68.0 * s,
        24.0 * s,
        rgba(hint_ink, 0.95),
    );
    if ask.locked {
        padlock(
            v,
            [r.x + r.w - 48.0 * s, r.y + r.h * 0.5],
            38.0 * s,
            0xe0a0b4,
            1.0,
        );
    } else if ask.awake {
        for k in 0..3 {
            let phase = (time * 0.6 + k as f32 * 0.33).fract();
            sparkle(
                v,
                [
                    r.x + r.w - 70.0 * s + k as f32 * 18.0 * s,
                    r.y + r.h * (0.75 - 0.5 * phase),
                ],
                (5.0 + 4.0 * (1.0 - phase)) * s,
                rgba(0xffd0e4, 1.0 - phase),
                0.6,
            );
        }
    }
}

/// A portrait cutting in from the left across a dark slab, her colour bleeding up from its foot.
#[allow(clippy::too_many_arguments)]
fn cut_in(v: &mut Vec<MapVertex>, r: Rect, uv: Option<Uv>, accent: u32, age: f32, s: f32) {
    let a = entrance(age, 0.04, 0.36);
    faded(v, a, |v| {
        shifted(v, [-(1.0 - a) * 340.0 * s, 0.0], |v| {
            let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
            turned(v, c, -0.07, |v| {
                panel(
                    v,
                    grow(r, 30.0 * s),
                    4.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 0.96),
                );
                panel(
                    v,
                    Rect::new(
                        r.x - 30.0 * s,
                        r.y + r.h * 0.45,
                        r.w + 60.0 * s,
                        r.h * 0.55 + 30.0 * s,
                    ),
                    4.0 * s,
                    Panel::fill(0x0b0508, mix(0x0b0508, accent, 0.6), 0.9),
                );
                panel(
                    v,
                    Rect::new(r.x - 30.0 * s, r.y - 30.0 * s, r.w + 60.0 * s, 8.0 * s),
                    0.0,
                    Panel::fill(accent, accent, 1.0),
                );
            });
            soft(v, r, 16.0 * s, 30.0 * s, rgba(accent, 0.32), true);
            panel(
                v,
                grow(r, 4.0 * s),
                14.0 * s,
                Panel::fill(0x1a0a14, 0x0b0508, 1.0).rim(GOLD, 1.0, 3.0 * s),
            );
            if let Some(uv) = uv {
                face_window(v, r, 12.0 * s, uv, 1.0);
            }
            vignette(v, r, 12.0 * s, r.h * 0.3, rgba(0x0b0508, 0.5));
        });
    });
}

/// Slashes of colour sweeping in from the left, a halftone and speed lines over them.
fn backdrop(
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    deep: u32,
    bright: u32,
    age: f32,
    time: f32,
    s: f32,
) {
    push_rect(v, Rect::new(0.0, 0.0, w, h), rgba(0x05020a, 0.74));
    let sweep = entrance(age, 0.0, 0.34);
    let angle = -0.26;
    let center = [w * 0.5, h * 0.54];
    shifted(v, [-(1.0 - sweep) * w * 1.2, 0.0], |v| {
        slash(
            v,
            center,
            w * 1.8,
            600.0 * s,
            angle,
            rgba(mix(deep, bright, 0.35), 0.95),
            rgba(0x12030a, 0.95),
        );
        slash(
            v,
            [center[0], center[1] + 326.0 * s],
            w * 1.8,
            30.0 * s,
            angle,
            rgba(0x0b0508, 0.96),
            rgba(0x0b0508, 0.96),
        );
        slash(
            v,
            [center[0], center[1] - 312.0 * s],
            w * 1.8,
            6.0 * s,
            angle,
            rgba(GOLD_HI, 0.85),
            rgba(GOLD, 0.85),
        );
        slash(
            v,
            [center[0], center[1] - 350.0 * s],
            w * 1.8,
            54.0 * s,
            angle,
            rgba(bright, 0.55),
            rgba(deep, 0.55),
        );
    });
    faded(v, sweep, |v| {
        halftone(
            v,
            Rect::new(w * 0.56, 0.0, w * 0.44, h),
            26.0 * s,
            12.0 * s,
            [0.9, -0.7],
            rgba(bright, 0.16),
        );
        speed_lines(
            v,
            Rect::new(0.0, h * 0.18, w, h * 0.64),
            angle,
            22,
            time,
            rgba(0xffffff, 0.08),
        );
    });
}

/// Keys at the foot of a page.
fn key_hints(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    keys: &[(&str, &str)],
    arrows: bool,
    accent: u32,
    s: f32,
) {
    let mut x = 110.0 * s;
    let y = h - 86.0 * s;
    if arrows {
        let c = [x + 22.0 * s, y + 20.0 * s];
        arrow(
            v,
            [c[0] - 9.0 * s, c[1]],
            18.0 * s,
            true,
            rgba(GOLD_HI, 1.0),
        );
        arrow(
            v,
            [c[0] + 9.0 * s, c[1]],
            18.0 * s,
            false,
            rgba(GOLD_HI, 1.0),
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            "elegir",
            x + 50.0 * s,
            y + 8.0 * s,
            21.0 * s,
            rgba(CREAM, 0.95),
        );
        x += 50.0 * s + text_width(font, Face::Italic, "elegir", 21.0 * s) + 40.0 * s;
    }
    for (key, what) in keys {
        let cap_w = (text_width(font, Face::Title, key, 18.0 * s) + 26.0 * s).max(42.0 * s);
        keycap(
            font,
            v,
            Rect::new(x, y, cap_w, 40.0 * s),
            key,
            18.0 * s,
            false,
            false,
            accent,
            1.0,
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            what,
            x + cap_w + 12.0 * s,
            y + 8.0 * s,
            21.0 * s,
            rgba(CREAM, 0.95),
        );
        x += cap_w + 12.0 * s + text_width(font, Face::Italic, what, 21.0 * s) + 40.0 * s;
    }
    let _ = w;
}

#[allow(clippy::too_many_arguments)]
fn draw_ask(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    faces: &[Option<Uv>],
    time: f32,
) {
    let Some(girl) = menu.girl() else {
        return;
    };
    let asking = menu.asking();
    let l = ask_layout(w, h, asking.len());
    let s = l.s;
    let accent = hex(&girl.accent, CRIMSON);
    let age = time - menu.page_at_ms as f32 / 1000.0;
    backdrop(v, w, h, 0x5a0c22, accent, age, time, s);
    cut_in(
        v,
        l.portrait,
        faces.get(menu.selected).copied().flatten(),
        accent,
        age,
        s,
    );

    // Her name thrown across the portrait's foot.
    let pr = l.portrait;
    let name_a = entrance(age, 0.2, 0.3);
    faded(v, name_a, |v| {
        shifted(v, [(1.0 - name_a) * -60.0 * s, 0.0], |v| {
            turned(v, [pr.x, pr.y + pr.h], -0.1, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    &girl.name,
                    pr.x + 20.0 * s,
                    pr.y + pr.h - 170.0 * s,
                    118.0 * s,
                    0xffffff,
                    accent,
                    1.0,
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    &girl.tier,
                    pr.x + 34.0 * s,
                    pr.y + pr.h - 40.0 * s,
                    34.0 * s,
                    rgba(mix(accent, 0xffffff, 0.55), 1.0),
                    rgba(0x0b0508, 0.9),
                );
            });
        });
    });

    // The question.
    let head_a = entrance(age, 0.1, 0.3);
    faded(v, head_a, |v| {
        turned(v, l.head, -0.06, |v| {
            let size = 52.0 * s;
            let q = format!("¿Qué le pides a {}?", girl.name);
            shout(
                font,
                v,
                Face::Title,
                &q,
                l.head[0],
                l.head[1],
                size,
                0xffffff,
                0x8a1030,
                1.0,
            );
            let qw = text_width(font, Face::Title, &q, size);
            wire(
                v,
                [l.head[0], l.head[1] + size + 14.0 * s],
                [l.head[0] + qw + 60.0 * s, l.head[1] + size + 14.0 * s],
                6.0 * s,
                rgba(0xffffff, 0.95),
                rgba(0xffffff, 0.0),
                0.0,
                false,
            );
        });
    });

    // What can be asked, on tilted tiles sliding in one after another; the cursor at the lit one.
    for (i, ask) in asking.iter().enumerate() {
        let r = l.tiles[i];
        let a = entrance(age, 0.16 + 0.07 * i as f32, 0.3);
        let lit = i == menu.choice;
        let hover = pointer.is_some_and(|(x, y)| inside_turned(r, TILE_ANGLE, x, y));
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 280.0 * s, 0.0], |v| {
                turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], TILE_ANGLE, |v| {
                    tile(font, v, r, ask, lit, hover, accent, time, s);
                });
            });
        });
        if lit && a > 0.95 {
            let (sn, cs) = TILE_ANGLE.sin_cos();
            let half = r.w * 0.5;
            let tip = [
                r.x + r.w * 0.5 - half * cs - 16.0 * s,
                r.y + r.h * 0.5 - half * sn,
            ];
            cursor(v, tip, 34.0 * s, time);
        }
    }

    // Her price and room, and the purse.
    let foot = l.tiles.last().map_or(h * 0.8, |r| r.y + r.h + 54.0 * s);
    let fx = l.tiles.first().map_or(w * 0.5, |r| r.x) + 40.0 * s;
    let info = format!("{} oro · Habitación {}", menu.girl_price(), girl.room + 1);
    coin(v, [fx, foot + 16.0 * s], 16.0 * s, 1.0);
    text_shadow(
        font,
        v,
        Face::Ui,
        &info,
        fx + 28.0 * s,
        foot + 1.0 * s,
        26.0 * s,
        rgba(
            if gold >= menu.girl_price() {
                GOLD_HI
            } else {
                0xff8aa0
            },
            1.0,
        ),
    );
    purse(
        font,
        v,
        Rect::new(w - 300.0 * s, 46.0 * s, 196.0 * s, 60.0 * s),
        gold,
        s,
    );

    // Her answer (or Rosalía's word on the gold).
    if !menu.notice.is_empty() {
        let (who, tone) = if menu.notice.starts_with('—') {
            (girl.name.as_str(), accent)
        } else {
            ("Rosalía", 0xd83c78)
        };
        speech_box(
            font,
            v,
            [pr.x + 40.0 * s, pr.y + pr.h + 70.0 * s],
            760.0 * s,
            who,
            &menu.notice,
            tone,
            -0.02,
            1.0,
            s,
        );
    }

    key_hints(
        font,
        v,
        w,
        h,
        &[("Enter", "pedir"), ("Esc", "volver")],
        true,
        accent,
        s,
    );
    let back_hover = pointer.is_some_and(|(x, y)| l.back.contains(x, y));
    button(
        font, v, l.back, "Esc", "Volver", false, back_hover, true, accent, time, s,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_offer(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    gold: u32,
    pointer: Option<(f32, f32)>,
    host_face: Option<Uv>,
    time: f32,
    liquid: super::shop_preview::LiquidMotion,
) {
    let l = offer_layout(w, h);
    let s = l.s;
    let age = time - menu.page_at_ms as f32 / 1000.0;
    let pink = 0xff4f9a;
    let leaflet_hover = pointer
        .is_some_and(|(x, y)| inside_turned_about(l.leaflet, center(l.card), OFFER_TILT, x, y));
    let rose = 0xd83c78;
    let first = menu.progress.elixirs_bought == 0;
    let price = menu.elixir_price();
    backdrop(v, w, h, 0x6a0a3a, pink, age, time, s);
    cut_in(v, l.portrait, host_face, rose, age, s);
    let pr = l.portrait;
    let name_a = entrance(age, 0.2, 0.3);
    faded(v, name_a, |v| {
        turned(v, [pr.x, pr.y + pr.h], -0.1, |v| {
            shout(
                font,
                v,
                Face::Title,
                "Rosalía",
                pr.x + 20.0 * s,
                pr.y + pr.h - 150.0 * s,
                100.0 * s,
                0xffffff,
                rose,
                1.0,
            );
            text_outline(
                font,
                v,
                Face::Italic,
                "La dueña de la casa",
                pr.x + 34.0 * s,
                pr.y + pr.h - 36.0 * s,
                32.0 * s,
                rgba(0xffc8dc, 1.0),
                rgba(0x0b0508, 0.9),
            );
        });
    });
    let line = if first {
        "—Oferta especial, solo para usted, mi reina, por ser clienta nueva. Invita la casa… casi."
    } else {
        "—¿Otra copita del elixir, cariño? Las chicas ya preguntan por usted."
    };
    let speak = entrance(age, 0.42, 0.25);
    speech_box(
        font,
        v,
        [pr.x + 30.0 * s, pr.y + pr.h + 150.0 * s],
        700.0 * s,
        "Rosalía",
        line,
        rose,
        -0.02,
        speak,
        s,
    );

    // The elixir's card, tilted, sliding in from the right.
    let c = l.card;
    let a = entrance(age, 0.14, 0.34);
    faded(v, a, |v| {
        shifted(v, [(1.0 - a) * 320.0 * s, 0.0], |v| {
            turned(v, center(c), OFFER_TILT, |v| {
                soft(
                    v,
                    Rect::new(c.x + 16.0 * s, c.y + 22.0 * s, c.w, c.h),
                    18.0 * s,
                    26.0 * s,
                    rgba(SHADOW, 0.8),
                    false,
                );
                soft(
                    v,
                    c,
                    18.0 * s,
                    30.0 * s,
                    rgba(pink, 0.26 + 0.08 * wave(time, 0.6)),
                    true,
                );
                panel(
                    v,
                    grow(c, 6.0 * s),
                    20.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 1.0),
                );
                panel(
                    v,
                    c,
                    16.0 * s,
                    Panel {
                        top: rgba(0x3a1034, 0.98),
                        bottom: rgba(0x12040e, 0.98),
                        ..Panel::velvet(0.98, 3.0 * s)
                    }
                    .rim(GOLD_HI, 1.0, 3.0 * s)
                    .hairline(9.0 * s, 0.5),
                );
                // The ribbon across its top.
                let ribbon = Rect::new(c.x - 24.0 * s, c.y + 26.0 * s, c.w * 0.62, 64.0 * s);
                panel(
                    v,
                    grow(ribbon, 4.0 * s),
                    6.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 1.0),
                );
                panel(
                    v,
                    ribbon,
                    4.0 * s,
                    Panel::fill(0xe8306a, 0x9a1040, 1.0).sheen(time * 0.35),
                );
                let tag = if first {
                    "¡OFERTA ESPECIAL!"
                } else {
                    "ELIXIR DE LA CASA"
                };
                text_outline(
                    font,
                    v,
                    Face::Title,
                    tag,
                    ribbon.x + 36.0 * s,
                    ribbon.y + 10.0 * s,
                    40.0 * s,
                    rgba(0xffffff, 1.0),
                    rgba(0x3a0618, 1.0),
                );
                // The stage under the flask.
                let st = l.stage;
                panel(
                    v,
                    grow(st, 3.0 * s),
                    18.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 1.0),
                );
                super::shop_preview::backdrop(v, st, [1.0, 0.36, 0.66], time);
                panel(
                    v,
                    st,
                    16.0 * s,
                    Panel::fill(0, 0, 0.0).rim(GOLD, 0.9, 2.2 * s),
                );
                // Its name, what it does, its price.
                let tx = st.x + st.w + 40.0 * s;
                let room = c.x + c.w - 40.0 * s - tx;
                let mut y = c.y + 130.0 * s;
                text_outline(
                    font,
                    v,
                    Face::Title,
                    "Elixir de Rosalía",
                    tx,
                    y,
                    50.0 * s,
                    rgba(GOLD_HI, 1.0),
                    rgba(0x12060e, 0.9),
                );
                y += 64.0 * s;
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    "Receta secreta de la casa",
                    tx,
                    y,
                    25.0 * s,
                    rgba(0xffb8d4, 1.0),
                );
                y += 50.0 * s;
                let about = "Despierta la forma del sello durante una noche. Las chicas sabrán apreciarlo… o saldrán corriendo.";
                for line in wrap(font, Face::Body, about, 25.0 * s, room) {
                    text_shadow(
                        font,
                        v,
                        Face::Body,
                        &line,
                        tx,
                        y,
                        25.0 * s,
                        rgba(CREAM, 0.96),
                    );
                    y += 25.0 * s * 1.42;
                }
                y += 34.0 * s;
                let big = 86.0 * s;
                if first {
                    // The usual price struck through, and the welcome one.
                    let old = format!("{}", super::super::venue::ELIXIR_PRICE);
                    let ow = text_width(font, Face::Title, &old, 44.0 * s);
                    text_shadow(
                        font,
                        v,
                        Face::Title,
                        &old,
                        tx,
                        y + 26.0 * s,
                        44.0 * s,
                        rgba(DIM, 1.0),
                    );
                    wire(
                        v,
                        [tx - 6.0 * s, y + 52.0 * s],
                        [tx + ow + 6.0 * s, y + 38.0 * s],
                        5.0 * s,
                        rgba(CRIMSON, 1.0),
                        rgba(CRIMSON, 1.0),
                        0.0,
                        false,
                    );
                    let nx = tx + ow + 40.0 * s;
                    shout(
                        font,
                        v,
                        Face::Title,
                        &format!("{price}"),
                        nx,
                        y,
                        big,
                        GOLD_HI,
                        0x8a1030,
                        1.0,
                    );
                    let nw = text_width(font, Face::Title, &format!("{price}"), big);
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        "oro",
                        nx + nw + 14.0 * s,
                        y + 44.0 * s,
                        30.0 * s,
                        rgba(CREAM, 1.0),
                    );
                    let bc = [
                        (nx + nw + 170.0 * s).min(c.x + c.w - 90.0 * s),
                        y + 40.0 * s,
                    ];
                    burst(
                        v,
                        bc,
                        52.0 * s,
                        86.0 * s,
                        12,
                        time * 0.3,
                        rgba(0xffe14a, 1.0),
                        rgba(0x0b0508, 1.0),
                        6.0 * s,
                    );
                    let off = format!(
                        "−{}%",
                        100 - price * 100 / super::super::venue::ELIXIR_PRICE.max(1)
                    );
                    let fw = text_width(font, Face::Title, &off, 40.0 * s);
                    text_outline(
                        font,
                        v,
                        Face::Title,
                        &off,
                        bc[0] - fw * 0.5,
                        bc[1] - 24.0 * s,
                        40.0 * s,
                        rgba(0xc0103c, 1.0),
                        rgba(0xffffff, 1.0),
                    );
                } else {
                    shout(
                        font,
                        v,
                        Face::Title,
                        &format!("{price}"),
                        tx,
                        y,
                        big,
                        GOLD_HI,
                        0x8a1030,
                        1.0,
                    );
                    let nw = text_width(font, Face::Title, &format!("{price}"), big);
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        "oro",
                        tx + nw + 14.0 * s,
                        y + 44.0 * s,
                        30.0 * s,
                        rgba(CREAM, 1.0),
                    );
                }
                // What it does, on chips.
                let mut cx = tx;
                let cy = y + big + 30.0 * s;
                for chip in ["Una noche, una chica"] {
                    let cw = text_width(font, Face::Body, chip, 20.0 * s) + 52.0 * s;
                    let r = Rect::new(cx, cy, cw, 38.0 * s);
                    panel(
                        v,
                        r,
                        r.h * 0.5,
                        Panel::fill(0x5a1440, 0x2a0820, 0.95)
                            .rim(0xffb0d0, 1.0, 1.6 * s)
                            .bevel(1.0),
                    );
                    seal_icon(
                        v,
                        [r.x + 20.0 * s, r.y + r.h * 0.5],
                        10.0 * s,
                        true,
                        time,
                        s,
                    );
                    text_shadow(
                        font,
                        v,
                        Face::Body,
                        chip,
                        r.x + 38.0 * s,
                        r.y + (r.h - 20.0 * s) * 0.5 - 2.0 * s,
                        20.0 * s,
                        rgba(0xffe4ee, 1.0),
                    );
                    cx += cw + 12.0 * s;
                }
                // Its leaflet, under the stage, and what it is, said plainly before buying.
                button(
                    font,
                    v,
                    l.leaflet,
                    "P",
                    "Efectos secundarios",
                    menu.choice == OFFER_LEAFLET,
                    leaflet_hover,
                    true,
                    pink,
                    time,
                    s,
                );
                content_warning(font, v, l.warning, s);
            });
        });
    });
    // The flask itself turns upright on its stage, its liquid sloshing.
    if a > 0.6 {
        faded(v, ((a - 0.6) / 0.4).clamp(0.0, 1.0), |v| {
            super::shop_preview::draw(
                v,
                Some(ELIXIR_MODEL),
                grow(l.stage, -24.0 * s),
                time,
                None,
                liquid,
            );
        });
    }

    // Buy and drink, or not now.
    let hover = |r: Rect| pointer.is_some_and(|(x, y)| r.contains(x, y));
    let can = gold >= price;
    let buy_label = format!("Comprar y beber · {price} oro");
    button(
        font,
        v,
        l.buy,
        "Enter",
        &buy_label,
        menu.choice == 0,
        hover(l.buy),
        can,
        pink,
        time,
        s,
    );
    button(
        font,
        v,
        l.decline,
        "Esc",
        "Ahora no",
        menu.choice == 1,
        hover(l.decline),
        true,
        pink,
        time,
        s,
    );
    if menu.choice < OFFER_LEAFLET {
        let r = if menu.choice == 0 { l.buy } else { l.decline };
        cursor(v, [r.x - 14.0 * s, r.y + r.h * 0.5], 30.0 * s, time);
    } else if a > 0.95 {
        let r = l.leaflet;
        let tip = turn_point(
            [r.x - 12.0 * s, r.y + r.h * 0.5],
            center(l.card),
            OFFER_TILT,
        );
        cursor(v, tip, 30.0 * s, time);
    }
    if !menu.notice.is_empty() {
        text_shadow(
            font,
            v,
            Face::Italic,
            &menu.notice,
            l.buy.x,
            l.buy.y + l.buy.h + 18.0 * s,
            23.0 * s,
            rgba(0xff8aa0, 1.0),
        );
    }
    purse(
        font,
        v,
        Rect::new(w - 300.0 * s, 46.0 * s, 196.0 * s, 60.0 * s),
        gold,
        s,
    );
}

/// Hazard tape: yellow slats leaning across a strip.
fn hazard_tape(v: &mut Vec<MapVertex>, r: Rect, s: f32) {
    let step = 22.0 * s;
    let slat = 11.0 * s;
    let lean = r.h;
    let yellow = rgba(0xffd23a, 1.0);
    let mut x = r.x;
    while x + slat + lean <= r.x + r.w {
        let q = [
            [x + lean, r.y],
            [x + lean + slat, r.y],
            [x + slat, r.y + r.h],
            [x, r.y + r.h],
        ];
        push_triangle(v, [q[0], q[1], q[2]], yellow);
        push_triangle(v, [q[0], q[2], q[3]], yellow);
        x += step;
    }
}

/// The warning sign: a yellow triangle in a black edge, an exclamation mark on it.
fn warning_sign(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32) {
    let tri = |k: f32| {
        [
            [c[0], c[1] - r * k],
            [c[0] + r * 1.12 * k, c[1] + r * 0.86 * k],
            [c[0] - r * 1.12 * k, c[1] + r * 0.86 * k],
        ]
    };
    push_triangle(v, tri(1.22), rgba(0x0b0508, 1.0));
    push_triangle(v, tri(1.0), rgba(0xffd23a, 1.0));
    panel(
        v,
        Rect::new(c[0] - r * 0.1, c[1] - r * 0.48, r * 0.2, r * 0.78),
        r * 0.08,
        Panel::fill(0x120808, 0x120808, 1.0),
    );
    disc(v, [c[0], c[1] + r * 0.56], r * 0.12, rgba(0x120808, 1.0));
}

/// The content warning across the offer's foot: hazard tape, the warning sign, what the elixir
/// does to the Queen, and a big «+18» at its far end. Its words keep to the left, clear of
/// Chiquita's lines.
fn content_warning(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, s: f32) {
    let yellow = 0xffd23a;
    panel(
        v,
        grow(r, 4.0 * s),
        9.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    panel(
        v,
        r,
        7.0 * s,
        Panel::fill(0x2c1d08, 0x130b03, 0.98).rim(yellow, 1.0, 2.2 * s),
    );
    hazard_tape(
        v,
        Rect::new(r.x + 14.0 * s, r.y + 8.0 * s, r.w - 28.0 * s, 7.0 * s),
        s,
    );
    hazard_tape(
        v,
        Rect::new(
            r.x + 14.0 * s,
            r.y + r.h - 15.0 * s,
            r.w - 28.0 * s,
            7.0 * s,
        ),
        s,
    );
    warning_sign(v, [r.x + 50.0 * s, r.y + r.h * 0.5 + 4.0 * s], 26.0 * s);
    let tx = r.x + 94.0 * s;
    text_shadow(
        font,
        v,
        Face::Title,
        "ADVERTENCIA · CONTENIDO FUTA",
        tx,
        r.y + 18.0 * s,
        23.0 * s,
        rgba(yellow, 1.0),
    );
    let words =
        "Con el elixir, a la Reina le crece un pene durante una noche y las escenas lo muestran.";
    let room = (r.w - 94.0 * s - 150.0 * s).min(560.0 * s);
    let size = 20.0 * s;
    let mut y = r.y + 47.0 * s;
    for line in wrap(font, Face::Body, words, size, room).iter().take(2) {
        text_shadow(font, v, Face::Body, line, tx, y, size, rgba(CREAM, 0.96));
        y += size * 1.25;
    }
    let c = [r.x + r.w - 64.0 * s, r.y + r.h * 0.5];
    disc(v, c, 39.0 * s, rgba(0x0b0508, 1.0));
    disc(v, c, 34.0 * s, rgba(0xd0103c, 1.0));
    let mark = "+18";
    let mw = text_width(font, Face::Title, mark, 30.0 * s);
    text(
        font,
        v,
        Face::Title,
        mark,
        c[0] - mw * 0.5,
        c[1] - 19.0 * s,
        30.0 * s,
        rgba(0xffffff, 1.0),
    );
}

/// A block of the elixir's leaflet.
enum Leaf {
    /// The framed notice at its head (its lines starting «• » are its points).
    Notice(&'static str),
    /// A numbered section.
    Head(&'static str),
    /// A heading inside a section: a frequency, a caution.
    Sub(&'static str),
    Para(&'static str),
    /// A point of a list.
    Item(&'static str),
    /// A warning in red, framed.
    Alert(&'static str),
}

/// What the leaflet says: a medicine's, side effects and all.
const LEAFLET: &[Leaf] = &[
    Leaf::Notice(
        "Lea todo el prospecto antes de tomar este elixir: contiene información importante para usted.\n• Conserve este prospecto, por si tiene que volver a leerlo (nadie lo hace).\n• Se le ha vendido solo a usted, mi reina. No se lo dé a otras personas aunque tengan los mismos síntomas (calor, curiosidad).",
    ),
    Leaf::Head("1. Qué es y para qué se utiliza"),
    Leaf::Para(
        "Es un despertador de sello (sigilotrópico): da forma física al sello de quien lo bebe durante una noche; en la práctica, le crece un pene funcional. Indicado en adultas con ganas y sin prisa.",
    ),
    Leaf::Head("2. Antes de tomarlo"),
    Leaf::Sub("No tome el elixir:"),
    Leaf::Item("si es alérgica a la fresa, al rubí molido o a las consecuencias;"),
    Leaf::Item("si mañana tiene audiencia en el trono."),
    Leaf::Sub("Advertencias y precauciones:"),
    Leaf::Para("No lo mezcle con hidromiel ni con pociones de crecimiento: los efectos se suman, literalmente."),
    Leaf::Sub("Conducción y uso de máquinas:"),
    Leaf::Para("No monte a caballo ni en dragón en las 8 horas siguientes. Las sillas de montar, especialmente desaconsejadas."),
    Leaf::Head("3. Cómo tomarlo"),
    Leaf::Para(
        "Un frasco por noche, de un trago, mirando a los ojos a quien la acompañe. Si toma más del que debe, acuda a Rosalía con el frasco vacío y la cabeza alta. Si olvidó tomarlo, no tome una dosis doble: ya sabe lo que pasaría (vea «difalia»).",
    ),
    Leaf::Head("4. Posibles efectos adversos"),
    Leaf::Para(
        "Al igual que todos los elixires, este puede producir efectos adversos, aunque no todas las personas los sufran. Las que los sufren suelen repetir.",
    ),
    Leaf::Sub("Muy frecuentes (pueden afectar a más de 1 de cada 10 personas):"),
    Leaf::Item("aparición de un pene funcional en usuarias que no lo tenían (falogénesis aguda transitoria);"),
    Leaf::Item("crecimiento anormal del miembro (macrofalia inducida), con un aumento repentino de la autoestima;"),
    Leaf::Item("calor en el bajo vientre, rubor facial y sonrisa de medio lado;"),
    Leaf::Item("erección espontánea al oír la palabra «sello»."),
    Leaf::Sub("Frecuentes (pueden afectar hasta a 1 de cada 10 personas):"),
    Leaf::Item("brote repentino de vello en zonas inesperadas (hipertricosis caprichosa): cejas, nudillos, cola;"),
    Leaf::Item("voz grave durante cinco a diez minutos;"),
    Leaf::Item("sudor con aroma a fresa;"),
    Leaf::Item("deseo incontrolable de hacer flexiones."),
    Leaf::Sub("Poco frecuentes (pueden afectar hasta a 1 de cada 100 personas):"),
    Leaf::Item("crecimiento de los cuernos de hasta 3 cm, con picor;"),
    Leaf::Item("brillo rosado de los ojos en la oscuridad (fosforescencia libidinosa);"),
    Leaf::Item("tintineo al caminar, de origen desconocido;"),
    Leaf::Item("ronroneo involuntario."),
    Leaf::Sub("Raros (pueden afectar hasta a 1 de cada 1.000 personas):"),
    Leaf::Item("aparición de un segundo miembro (difalia temporal); Rosalía no se hace responsable;"),
    Leaf::Item("aumento del pecho de una talla, o de dos (macromastia de cortesía);"),
    Leaf::Item("atracción magnética hacia los capibaras."),
    Leaf::Sub("Muy raros (pueden afectar hasta a 1 de cada 10.000 personas):"),
    Leaf::Item("el efecto se contagia a quien la acompañe (falogénesis por contacto);"),
    Leaf::Item("pérdida total de la vergüenza (anaidosis aguda);"),
    Leaf::Item("cantar coplas en sueños."),
    Leaf::Sub("Frecuencia no conocida:"),
    Leaf::Item("Chiquita se pone celosa;"),
    Leaf::Item("las chicas preguntan por usted;"),
    Leaf::Item("ganas irresistibles de volver al burdel."),
    Leaf::Alert("Si la erección dura más de cuatro horas, no consulte a ningún sanador: ya es tarde. Hable con Rosalía."),
    Leaf::Para("Comunicación de efectos adversos: a Rosalía, en el mostrador, en voz baja."),
    Leaf::Head("5. Conservación"),
    Leaf::Item("Manténgase fuera del alcance de los goblins y de Chiquita."),
    Leaf::Item("No agitar delante de las clientas."),
    Leaf::Item("Caduca cuando se acaba la noche."),
    Leaf::Head("6. Información adicional"),
    Leaf::Para(
        "Composición: sigilotropina carmesí, jarabe de fresa, rubí molido, una lágrima de súcubo y el secreto de la casa. Fabricante: Rosalía (y nadie más). Última revisión de este prospecto: nunca.",
    ),
];

const LEAF_INK: u32 = 0x1c1418;
const LEAF_RED: u32 = 0x9a1034;

/// A line of the leaflet laid out: its words, where, how big, in which face and ink; `bullet`
/// puts a point at that x before it.
struct LeafLine {
    face: Face,
    text: String,
    x: f32,
    y: f32,
    size: f32,
    ink: u32,
    bullet: Option<f32>,
}

/// The leaflet laid out in its columns: the lines, the frames (red for an alert) and whether it
/// all fit.
struct Laid {
    lines: Vec<LeafLine>,
    frames: Vec<(Rect, bool)>,
    fits: bool,
}

/// The leaflet flowed down `cols` columns of `area` at `size` px.
fn flow(font: &FontAtlas, area: Rect, cols: usize, gutter: f32, size: f32) -> Laid {
    let col_w = (area.w - gutter * (cols as f32 - 1.0)) / cols as f32;
    let bottom = area.y + area.h;
    let mut laid = Laid {
        lines: Vec::new(),
        frames: Vec::new(),
        fits: true,
    };
    let mut col = 0usize;
    let mut y = area.y;
    'blocks: for block in LEAFLET {
        // Its face, size, ink, indent, inner padding, room before and after, its frame (red or
        // not), whether it keeps with what follows, and its words.
        let (face, sz, ink, indent, pad, before, after, framed, keep, words) = match *block {
            Leaf::Notice(t) => (
                Face::Body,
                size,
                LEAF_INK,
                0.0,
                size * 0.7,
                0.0,
                size * 0.8,
                Some(false),
                true,
                t,
            ),
            Leaf::Head(t) => (
                Face::Title,
                size * 1.14,
                LEAF_RED,
                0.0,
                0.0,
                size * 0.75,
                size * 0.3,
                None,
                true,
                t,
            ),
            Leaf::Sub(t) => (
                Face::Title,
                size * 0.95,
                0x3a1424,
                0.0,
                0.0,
                size * 0.35,
                size * 0.12,
                None,
                true,
                t,
            ),
            Leaf::Para(t) => (
                Face::Body,
                size,
                LEAF_INK,
                0.0,
                0.0,
                0.0,
                size * 0.35,
                None,
                false,
                t,
            ),
            Leaf::Item(t) => (
                Face::Body,
                size,
                LEAF_INK,
                size * 1.05,
                0.0,
                0.0,
                size * 0.1,
                None,
                false,
                t,
            ),
            Leaf::Alert(t) => (
                Face::Body,
                size,
                LEAF_RED,
                0.0,
                size * 0.55,
                size * 0.4,
                size * 0.6,
                Some(true),
                true,
                t,
            ),
        };
        let line_h = sz * 1.3;
        let lines = wrap(font, face, words, sz, col_w - indent - pad * 2.0);
        let height = lines.len() as f32 * line_h + pad * 2.0;
        // A frame keeps together; a heading keeps two lines of what follows with it.
        let needed = if framed.is_some() {
            height
        } else if keep {
            height + line_h * 2.0
        } else {
            line_h
        };
        if y > area.y {
            y += before;
        }
        if y + needed > bottom && y > area.y {
            col += 1;
            y = area.y;
        }
        if col >= cols {
            laid.fits = false;
            break;
        }
        if let Some(alert) = framed {
            laid.frames.push((
                Rect::new(area.x + col as f32 * (col_w + gutter), y, col_w, height),
                alert,
            ));
        }
        y += pad;
        for (i, text) in lines.into_iter().enumerate() {
            if y + line_h > bottom {
                col += 1;
                y = area.y;
                if col >= cols {
                    laid.fits = false;
                    break 'blocks;
                }
            }
            let x = area.x + col as f32 * (col_w + gutter) + indent + pad;
            laid.lines.push(LeafLine {
                face,
                text,
                x,
                y,
                size: sz,
                ink,
                bullet: (indent > 0.0 && i == 0).then_some(x - indent + size * 0.25),
            });
            y += line_h;
        }
        y += pad + after;
    }
    laid
}

/// The leaflet as large as it fits in `area`. It only changes with the screen, so it is laid out
/// once and kept.
fn laid_out(font: &FontAtlas, area: Rect, s: f32) -> Rc<Laid> {
    thread_local! {
        static LAID: RefCell<Option<([u32; 4], Rc<Laid>)>> = const { RefCell::new(None) };
    }
    let key = [
        area.x.to_bits(),
        area.y.to_bits(),
        area.w.to_bits(),
        area.h.to_bits(),
    ];
    LAID.with(|cell| {
        let kept = cell
            .borrow()
            .as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, laid)| laid.clone());
        if let Some(laid) = kept {
            return laid;
        }
        let mut size = 20.0 * s;
        let laid = loop {
            let laid = flow(font, area, 3, 46.0 * s, size);
            if laid.fits || size <= 12.0 * s {
                break Rc::new(laid);
            }
            size -= 0.5 * s;
        };
        *cell.borrow_mut() = Some((key, laid.clone()));
        laid
    })
}

/// A barcode of made-up bars.
fn barcode(v: &mut Vec<MapVertex>, r: Rect, ink: [f32; 4]) {
    const BARS: &[u8] = &[
        2, 1, 1, 1, 3, 2, 1, 1, 2, 3, 1, 1, 1, 2, 2, 1, 3, 1, 1, 2, 1, 1, 2, 3, 1, 2, 1, 1, 3, 1,
        2, 2, 1, 1, 1, 3, 1, 2, 1, 1, 2,
    ];
    let units: u32 = BARS.iter().map(|&b| b as u32).sum();
    let unit = r.w / units as f32;
    let mut x = r.x;
    for (i, &b) in BARS.iter().enumerate() {
        let bw = b as f32 * unit;
        if i % 2 == 0 {
            push_rect(v, Rect::new(x, r.y, bw, r.h), ink);
        }
        x += bw;
    }
}

/// The leaflet's head: what it is, the elixir's name and strength, a barcode and a red rule.
fn leaflet_header(font: &FontAtlas, v: &mut Vec<MapVertex>, p: Rect, s: f32) {
    let x = p.x + 52.0 * s;
    let grey = 0x6a5c62;
    text(
        font,
        v,
        Face::Ui,
        "PROSPECTO: INFORMACIÓN PARA LA USUARIA",
        x,
        p.y + 30.0 * s,
        20.0 * s,
        rgba(grey, 1.0),
    );
    text(
        font,
        v,
        Face::Title,
        "Elixir de Rosalía 69 mg/ml solución oral",
        x,
        p.y + 60.0 * s,
        38.0 * s,
        rgba(LEAF_RED, 1.0),
    );
    text(
        font,
        v,
        Face::Italic,
        "Sigilotropina carmesí (como hidrato de corazonina) · Vía oral · Solo para adultas",
        x,
        p.y + 110.0 * s,
        21.0 * s,
        rgba(grey, 1.0),
    );
    wire(
        v,
        [x, p.y + 154.0 * s],
        [p.x + p.w - 52.0 * s, p.y + 154.0 * s],
        2.0 * s,
        rgba(LEAF_RED, 0.9),
        rgba(LEAF_RED, 0.9),
        0.0,
        false,
    );
    let code = Rect::new(p.x + p.w - 740.0 * s, p.y + 36.0 * s, 200.0 * s, 64.0 * s);
    barcode(v, code, rgba(LEAF_INK, 1.0));
    let cn = "CN 696969.6";
    let cw = text_width(font, Face::Ui, cn, 16.0 * s);
    text(
        font,
        v,
        Face::Ui,
        cn,
        code.x + (code.w - cw) * 0.5,
        code.y + code.h + 6.0 * s,
        16.0 * s,
        rgba(LEAF_INK, 1.0),
    );
}

/// Rosalía's rubber stamp: red words in a double frame, at a slant, `alpha` in.
#[allow(clippy::too_many_arguments)]
fn stamp(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    words: &str,
    under: &str,
    alpha: f32,
    s: f32,
) {
    let red = 0xc0123c;
    let size = 34.0 * s;
    let ww = text_width(font, Face::Title, words, size);
    let uw = text_width(font, Face::Ui, under, 17.0 * s);
    let wide = ww.max(uw);
    let r = Rect::new(
        c[0] - wide * 0.5 - 26.0 * s,
        c[1] - 46.0 * s,
        wide + 52.0 * s,
        92.0 * s,
    );
    turned(v, c, -0.11, |v| {
        panel(
            v,
            r,
            8.0 * s,
            Panel::fill(red, red, 0.07 * alpha).rim(red, 0.85 * alpha, 4.0 * s),
        );
        panel(
            v,
            grow(r, -8.0 * s),
            5.0 * s,
            Panel::fill(0, 0, 0.0).rim(red, 0.75 * alpha, 1.8 * s),
        );
        text(
            font,
            v,
            Face::Title,
            words,
            c[0] - ww * 0.5,
            r.y + 14.0 * s,
            size,
            rgba(red, 0.9 * alpha),
        );
        text(
            font,
            v,
            Face::Ui,
            under,
            c[0] - uw * 0.5,
            r.y + 58.0 * s,
            17.0 * s,
            rgba(red, 0.85 * alpha),
        );
    });
}

/// The elixir's leaflet: a folded sheet with the medicine's small print, Rosalía's stamp on it and
/// her word under it.
#[allow(clippy::too_many_arguments)]
fn draw_leaflet(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    menu: &Menu,
    pointer: Option<(f32, f32)>,
    time: f32,
) {
    let l = leaflet_layout(w, h);
    let s = l.s;
    let age = time - menu.page_at_ms as f32 / 1000.0;
    let pink = 0xff4f9a;
    backdrop(v, w, h, 0x6a0a3a, pink, age, time, s);
    let p = l.paper;
    let laid = laid_out(
        font,
        Rect::new(
            p.x + 46.0 * s,
            p.y + 176.0 * s,
            p.w - 92.0 * s,
            p.h - 200.0 * s,
        ),
        s,
    );
    let a = entrance(age, 0.0, 0.36);
    faded(v, a, |v| {
        shifted(v, [0.0, (1.0 - a) * 140.0 * s], |v| {
            turned(v, center(p), LEAFLET_TILT, |v| {
                // The sheet: cream paper on a hard shadow, creased where it was folded in three.
                panel(
                    v,
                    Rect::new(p.x + 18.0 * s, p.y + 22.0 * s, p.w, p.h),
                    4.0 * s,
                    Panel::fill(0x000000, 0x000000, 0.55),
                );
                panel(
                    v,
                    p,
                    4.0 * s,
                    Panel::fill(0xfdfbf5, 0xf0e9de, 1.0).rim(0xd6cdbf, 1.0, 1.5 * s),
                );
                for k in 1..3 {
                    let x = p.x + p.w * k as f32 / 3.0;
                    let (top, foot) = (p.y + 3.0 * s, p.y + p.h - 3.0 * s);
                    wire(
                        v,
                        [x, top],
                        [x, foot],
                        2.0 * s,
                        rgba(0x8a7a6a, 0.18),
                        rgba(0x8a7a6a, 0.18),
                        0.0,
                        false,
                    );
                    wire(
                        v,
                        [x + 2.5 * s, top],
                        [x + 2.5 * s, foot],
                        2.0 * s,
                        rgba(0xffffff, 0.9),
                        rgba(0xffffff, 0.9),
                        0.0,
                        false,
                    );
                }
                leaflet_header(font, v, p, s);
                for (r, alert) in &laid.frames {
                    let (fill, rim) = if *alert {
                        (0xfde4e8, LEAF_RED)
                    } else {
                        (0xfbf1ec, 0xb07a86)
                    };
                    panel(
                        v,
                        *r,
                        5.0 * s,
                        Panel::fill(fill, mix(fill, 0x000000, 0.03), 1.0).rim(rim, 1.0, 1.6 * s),
                    );
                }
                for line in &laid.lines {
                    if let Some(bx) = line.bullet {
                        text(
                            font,
                            v,
                            Face::Body,
                            "•",
                            bx,
                            line.y,
                            line.size,
                            rgba(line.ink, 1.0),
                        );
                    }
                    text(
                        font,
                        v,
                        line.face,
                        &line.text,
                        line.x,
                        line.y,
                        line.size,
                        rgba(line.ink, 1.0),
                    );
                }
            });
        });
    });
    // Rosalía's stamp lands on it.
    let landed = entrance(age, 0.5, 0.16);
    if landed > 0.0 {
        shifted(v, [0.0, -(1.0 - landed) * 30.0 * s], |v| {
            stamp(
                font,
                v,
                [p.x + p.w - 262.0 * s, p.y + 84.0 * s],
                "APROBADO POR ROSALÍA",
                "Ningún sanador lo ha revisado",
                landed,
                s,
            );
        });
    }
    speech_box(
        font,
        v,
        [p.x, h - 24.0 * s],
        720.0 * s,
        "Rosalía",
        "—Letra pequeña, cariño. Nadie se la lee.",
        0xd83c78,
        -0.02,
        entrance(age, 0.65, 0.25),
        s,
    );
    let back_hover = pointer.is_some_and(|(x, y)| l.back.contains(x, y));
    button(
        font,
        v,
        l.back,
        "Esc",
        "Volver a la oferta",
        true,
        back_hover,
        true,
        pink,
        time,
        s,
    );
}
