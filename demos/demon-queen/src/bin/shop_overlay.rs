//! The village shop, in the manner of Persona's menus (as the hypnotist's stall is): the world dims
//! under slashes of emerald with a halftone and speed lines; the shopkeeper stands cut out at the
//! left (his illustration, or the frame waiting for it) with his name thrown across his chest and
//! his word in a comic box. Three pages: the shelves (tilted tiles with the red cursor at the lit
//! one, and its card with the turning model), a haggle (his patience, the price coming down, a
//! needle sweeping a bar of zones, the tactics on tiles and a stamp for each try) and the fusion
//! altar (her things on tiles, two sockets, what they make, a spiral and a burst). Authored at
//! 1920×1080: drawing and picking share the same rectangles, and what the keys and the pointer
//! light only changes light and colour, never a place or a size.
use super::super::shop::{self, Category, Tab, PAGE_SIZE};
use super::persona::*;
use super::ui_paint::*;
use super::*;

type Uv = ([f32; 2], [f32; 2]);

/// What the overlay shows of the shop beyond its shelves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShopView {
    /// 0 the shelves, 1 a haggle, 2 the fusion altar.
    pub page: u8,
    /// Since the shop and the page opened (ms): their entrances.
    pub open_ms: u32,
    pub page_ms: u32,
    /// The shopkeeper's word and its age (ms).
    pub said: Option<(String, u32)>,
    /// His illustration (the face of the moment), once it exists; the face before it and how long
    /// ago it changed (ms), for the cross-fade between them.
    pub merchant: Option<PortraitRef>,
    pub merchant_from: Option<PortraitRef>,
    pub merchant_ms: u32,
    pub haggle: Option<HaggleView>,
    pub fuse: Option<FuseView>,
    /// Products he will not haggle over again today (catalog indices).
    pub refused: Vec<usize>,
}

/// A haggle on show.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HaggleView {
    /// The product (catalog index), whether she sells it, how many.
    pub product: usize,
    pub selling: bool,
    pub quantity: u32,
    /// The unit price before, and now; how far it moved (percent).
    pub base: u32,
    pub price: u32,
    pub percent: u32,
    pub patience: u8,
    pub rounds: u8,
    pub max_rounds: u8,
    /// The lit tactic (0 flatter, 1 press, 2 threaten, 3 the deal, 4 leave).
    pub tactic: u8,
    /// The needle on the bar, the zone's centre and the lit tactic's half width (0..1).
    pub needle: f32,
    pub zone: f32,
    pub half: f32,
    /// The last try (0 dead on, 1 good, 2 miss, 3 offended, 4 deal, 5 out) and its age (ms).
    pub result: Option<(u8, u32)>,
    pub closed: bool,
    /// Gold she lacks for the deal as it stands.
    pub gold_short: u32,
    /// «Otro trato», once he has hinted at it.
    pub favor: Option<FavorTile>,
}

/// «Otro trato» on show: the favour he asks, the cut it buys, whether it is a new one, how many
/// she can choose among, and since when it is offered (ms).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FavorTile {
    pub name: String,
    pub percent: u32,
    pub new: bool,
    pub choices: u8,
    pub age_ms: u32,
}

/// The fusion altar on show.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FuseView {
    /// Her things: id, name, how many.
    pub items: Vec<(String, String, u32)>,
    pub cursor: usize,
    /// What lies in the two sockets (id, name).
    pub slots: [Option<(String, String)>; 2],
    /// What the two would make (a known recipe).
    pub result: Option<(String, String)>,
    pub fee: u32,
    /// The fusion going off: its age (ms), what it makes, what went in.
    pub show: Option<FuseShow>,
    /// The last thing made.
    pub made: Option<(String, String)>,
    pub notice: String,
}

/// A fusion's show.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FuseShow {
    pub ms: u32,
    pub made: (String, String),
    pub from: [String; 2],
}

const EMERALD: u32 = 0x22c08a;
const MINT: u32 = 0x8affd6;
const DEEP: u32 = 0x03251c;
const QUEEN: u32 = 0xff6ea0;
const EDGE: u32 = 0x050a08;
/// How far the plates and tiles lean.
const TILT: f32 = -0.025;

pub(super) struct Layout;
impl Layout {
    pub const TABS: [Rect; 3] = [
        Rect::new(620.0, 118.0, 236.0, 58.0),
        Rect::new(870.0, 118.0, 236.0, 58.0),
        Rect::new(1120.0, 118.0, 236.0, 58.0),
    ];
    pub const CLOSE: Rect = Rect::new(1756.0, 118.0, 124.0, 58.0);
    pub const PREVIOUS: Rect = Rect::new(620.0, 928.0, 64.0, 46.0);
    pub const NEXT: Rect = Rect::new(808.0, 928.0, 64.0, 46.0);
    pub const CARD: Rect = Rect::new(1206.0, 246.0, 680.0, 736.0);
    pub const PREVIEW: Rect = Rect::new(1228.0, 268.0, 300.0, 300.0);
    pub const MINUS: Rect = Rect::new(1386.0, 820.0, 56.0, 50.0);
    pub const PLUS: Rect = Rect::new(1538.0, 820.0, 56.0, 50.0);
    pub const ACTION: Rect = Rect::new(1228.0, 914.0, 330.0, 54.0);
    pub const HAGGLE: Rect = Rect::new(1574.0, 914.0, 290.0, 54.0);
    pub const FUSE_GO: Rect = Rect::new(1228.0, 914.0, 330.0, 54.0);
    pub const FUSE_CLEAR: Rect = Rect::new(1574.0, 914.0, 290.0, 54.0);
    /// The altar's sockets (centre and radius): the two and what they make.
    pub const SOCKETS: [([f32; 2], f32); 3] = [
        ([1340.0, 430.0], 84.0),
        ([1752.0, 430.0], 84.0),
        ([1546.0, 690.0], 104.0),
    ];
    /// The haggle's bar.
    pub const BAR: Rect = Rect::new(650.0, 604.0, 730.0, 56.0);
    /// «Otro trato»: above the tactics, once he has hinted at it.
    pub const FAVOR: Rect = Rect::new(1452.0, 312.0, 418.0, 116.0);

    pub fn row(row: usize) -> Rect {
        Rect::new(620.0, 250.0 + row as f32 * 84.0, 560.0, 74.0)
    }
    pub fn category(index: usize) -> Rect {
        Rect::new(620.0 + index as f32 * 156.0, 190.0, 148.0, 42.0)
    }
    pub fn tactic(index: usize) -> Rect {
        Rect::new(
            1460.0 + index as f32 * 6.0,
            452.0 + index as f32 * 94.0,
            410.0,
            82.0,
        )
    }
    /// The favour tile's arrows (the other favours she can offer).
    pub fn favor_arrow(by: i8) -> Rect {
        let r = Self::FAVOR;
        let x = if by < 0 {
            r.x + r.w - 112.0
        } else {
            r.x + r.w - 58.0
        };
        Rect::new(x, r.y + r.h - 50.0, 46.0, 40.0)
    }
    pub fn fuse_tile(index: usize) -> Rect {
        let (col, row) = (index % FUSE_COLUMNS, index / FUSE_COLUMNS);
        Rect::new(
            620.0 + col as f32 * 142.0,
            250.0 + row as f32 * 142.0,
            132.0,
            132.0,
        )
    }

    /// What lies under the pointer on the page on show.
    pub fn hit(x: f32, y: f32, pick: &Pick) -> Option<ShopHit> {
        if Self::CLOSE.contains(x, y) {
            return Some(ShopHit::Close);
        }
        if pick.page == 1 {
            if pick.favor {
                if pick.favor_choices > 1 {
                    for (by, hit) in [(-1, ShopHit::FavorPrev), (1, ShopHit::FavorNext)] {
                        if Self::favor_arrow(by).contains(x, y) {
                            return Some(hit);
                        }
                    }
                }
                if inside_turned(Self::FAVOR, TILT, x, y) {
                    return Some(ShopHit::Tactic(5));
                }
            }
            return (0..5)
                .find(|&i| inside_turned(Self::tactic(i), TILT, x, y))
                .map(|i| ShopHit::Tactic(i as u8));
        }
        for (rect, hit) in
            Self::TABS
                .iter()
                .zip([ShopHit::TabBuy, ShopHit::TabSell, ShopHit::TabFuse])
        {
            if inside_turned(*rect, TILT, x, y) {
                return Some(hit);
            }
        }
        if pick.page == 2 {
            let start = pick.fuse_cursor / FUSE_PAGE * FUSE_PAGE;
            for i in start..pick.fuse_count.min(start + FUSE_PAGE) {
                if Self::fuse_tile(i - start).contains(x, y) {
                    return Some(ShopHit::FuseItem(i));
                }
            }
            for (slot, (c, r)) in Self::SOCKETS.iter().take(2).enumerate() {
                if (x - c[0]).hypot(y - c[1]) <= *r {
                    return Some(ShopHit::FuseSlot(slot as u8));
                }
            }
            if inside_turned(Self::FUSE_GO, TILT, x, y) {
                return Some(ShopHit::FuseGo);
            }
            if inside_turned(Self::FUSE_CLEAR, TILT, x, y) {
                return Some(ShopHit::FuseClear);
            }
            return None;
        }
        for (i, category) in Category::ALL.iter().enumerate() {
            if Self::category(i).contains(x, y) {
                return Some(ShopHit::Category(*category));
            }
        }
        let count = pick.count;
        let start = pick.selection.min(count.saturating_sub(1)) / PAGE_SIZE * PAGE_SIZE;
        for row in 0..PAGE_SIZE.min(count.saturating_sub(start)) {
            if inside_turned(Self::row(row), TILT, x, y) {
                return Some(ShopHit::Item(start + row));
            }
        }
        if inside_turned(Self::ACTION, TILT, x, y) {
            return Some(if count == 0 {
                ShopHit::TabBuy
            } else {
                ShopHit::Action
            });
        }
        if count > 0 {
            if inside_turned(Self::HAGGLE, TILT, x, y) {
                return Some(ShopHit::Haggle);
            }
            if Self::MINUS.contains(x, y) {
                return Some(ShopHit::QuantityMinus);
            }
            if Self::PLUS.contains(x, y) {
                return Some(ShopHit::QuantityPlus);
            }
        }
        if Self::PREVIOUS.contains(x, y) && start > 0 {
            return Some(ShopHit::PagePrevious);
        }
        if Self::NEXT.contains(x, y) && start + PAGE_SIZE < count {
            return Some(ShopHit::PageNext);
        }
        None
    }
}

/// The altar shows this many of her things at once, this many to a row.
pub(super) const FUSE_PAGE: usize = 16;
pub(super) const FUSE_COLUMNS: usize = 4;

/// What picking needs to know of the page on show.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Pick {
    pub page: u8,
    pub selection: usize,
    pub count: usize,
    pub fuse_count: usize,
    pub fuse_cursor: usize,
    /// «Otro trato» is on show, and among how many favours.
    pub favor: bool,
    pub favor_choices: u8,
}

fn mid(r: Rect) -> [f32; 2] {
    [r.x + r.w * 0.5, r.y + r.h * 0.5]
}

/// The point (x, y) lies on `r` turned by `angle` about its centre.
fn inside_turned(r: Rect, angle: f32, x: f32, y: f32) -> bool {
    let c = mid(r);
    let (sn, cs) = (-angle).sin_cos();
    let (dx, dy) = (x - c[0], y - c[1]);
    r.contains(c[0] + dx * cs - dy * sn, c[1] + dx * sn + dy * cs)
}

fn item_icon(icons: &IconAtlas, v: &mut Vec<MapVertex>, id: &str, r: Rect, alpha: f32) {
    if let Some(entry) = icons.get_sized(id, r.w as u32, r.h as u32) {
        push_icon_rect(v, r, entry, [1.0, 1.0, 1.0, alpha]);
    }
}

/// The largest size up to `size` at which `s` fits `w` (down to 60 %).
fn fit(font: &FontAtlas, face: Face, s: &str, size: f32, w: f32) -> f32 {
    let width = text_width(font, face, s, size);
    if width <= w {
        size
    } else {
        (size * w / width).max(size * 0.6)
    }
}

/// What the overlay needs besides the view.
pub(super) struct ShopFrame<'a> {
    pub tab: Tab,
    pub category: Category,
    pub selection: usize,
    pub quantity: u32,
    pub gold: u32,
    pub inventory: &'a BTreeMap<String, u32>,
    pub stock: &'a BTreeMap<String, u32>,
    pub notice: Option<&'a str>,
    pub hover: Option<ShopHit>,
    pub time: f32,
    pub preview_rotation: Option<(f32, f32)>,
    pub liquid_motion: shop_preview::LiquidMotion,
}

/// The whole shop.
pub(super) fn draw(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    view: Option<&ShopView>,
    merchant: Option<Uv>,
    merchant_from: Option<Uv>,
) {
    let fallback = ShopView {
        open_ms: 60_000,
        page_ms: 60_000,
        merchant_ms: 60_000,
        ..ShopView::default()
    };
    let view = view.unwrap_or(&fallback);
    let open = view.open_ms as f32 / 1000.0;
    let age = view.page_ms as f32 / 1000.0;
    backdrop(v, open, f.time);
    figure(font, v, view, merchant, merchant_from, open, f.time);
    let title = match view.page {
        1 => "¡A regatear!",
        2 => "El altar de fusión",
        _ => "Tienda de la villa",
    };
    heading(font, v, 620.0, 28.0, title, open.min(age + 0.3));
    purse(font, v, Rect::new(1630.0, 40.0, 250.0, 60.0), f.gold);
    plate(
        font,
        v,
        Layout::CLOSE,
        "Salir",
        false,
        f.hover == Some(ShopHit::Close),
        true,
        24.0,
        f.time,
    );
    match view.page {
        1 => draw_haggle(font, icons, v, f, view, age),
        2 => draw_fuse(font, icons, v, f, view, age),
        _ => draw_shelves(font, icons, v, f, view, age),
    }
    if let Some((words, ms)) = &view.said {
        let k = entrance(*ms as f32 / 1000.0, 0.0, 0.25);
        speech_box(
            font,
            v,
            [44.0, 330.0],
            540.0,
            "Tendero",
            words,
            EMERALD,
            -0.02,
            k,
            1.0,
        );
    }
    let favor = view.haggle.as_ref().is_some_and(|h| h.favor.is_some());
    let keys: &[(&str, &str)] = match view.page {
        1 if favor => &[
            ("W S", "táctica"),
            ("1-6", "jugar ya"),
            ("A D", "otro favor"),
            ("Enter", "jugar"),
            ("Esc", "dejarlo"),
        ],
        1 => &[
            ("W S", "táctica"),
            ("1-5", "jugar ya"),
            ("Enter", "jugar"),
            ("Esc", "dejarlo"),
        ],
        2 => &[
            ("WASD", "elegir"),
            ("Enter", "al altar"),
            ("E", "fusionar"),
            ("Q", "vaciar"),
            ("Tab", "pestañas"),
            ("Esc", "volver"),
        ],
        _ => &[
            ("W S", "elegir"),
            ("Tab", "pestañas"),
            ("1-5", "filtros"),
            ("+ −", "cantidad"),
            ("Enter", "aceptar"),
            ("H", "regatear"),
            ("Esc", "salir"),
        ],
    };
    hints(font, v, keys);
}

/// Slashes of emerald sweeping in, a halftone and speed lines over the dimmed world.
fn backdrop(v: &mut Vec<MapVertex>, open: f32, time: f32) {
    push_rect(v, Rect::new(0.0, 0.0, 1920.0, 1080.0), rgba(0x01060a, 0.72));
    let sweep = entrance(open, 0.0, 0.36);
    let angle = -0.22;
    let c = [1060.0, 560.0];
    shifted(v, [-(1.0 - sweep) * 2300.0, 0.0], |v| {
        slash(
            v,
            c,
            3600.0,
            600.0,
            angle,
            rgba(mix(DEEP, EMERALD, 0.4), 0.94),
            rgba(0x02130e, 0.95),
        );
        slash(
            v,
            [c[0], c[1] + 326.0],
            3600.0,
            28.0,
            angle,
            rgba(EDGE, 0.96),
            rgba(EDGE, 0.96),
        );
        slash(
            v,
            [c[0], c[1] - 312.0],
            3600.0,
            6.0,
            angle,
            rgba(GOLD_HI, 0.85),
            rgba(GOLD, 0.85),
        );
        slash(
            v,
            [c[0], c[1] - 350.0],
            3600.0,
            52.0,
            angle,
            rgba(MINT, 0.3),
            rgba(DEEP, 0.4),
        );
    });
    faded(v, sweep, |v| {
        halftone(
            v,
            Rect::new(1000.0, 0.0, 920.0, 1080.0),
            26.0,
            12.0,
            [0.9, -0.7],
            rgba(MINT, 0.12),
        );
        speed_lines(
            v,
            Rect::new(0.0, 170.0, 1920.0, 720.0),
            angle,
            22,
            time,
            rgba(0xffffff, 0.06),
        );
    });
}

/// The shopkeeper at the left, his name across his chest. In a haggle he shakes at a miss,
/// flushes at an offence and bobs at a good try; his face changes with each outcome (the new one
/// fading in over the old with a little hop), and he bobs as he talks.
#[allow(clippy::too_many_arguments)]
fn figure(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &ShopView,
    uv: Option<Uv>,
    from: Option<Uv>,
    open: f32,
    time: f32,
) {
    let a = entrance(open, 0.06, 0.4);
    let r = Rect::new(-10.0, 400.0, 680.0, 680.0);
    let breathe = (time * 1.3).sin() * 3.0;
    let (mut shake, mut bob, mut tint) = (0.0, 0.0, [1.0; 4]);
    // A word just said: a few quick bobs, as if he were talking.
    if let Some((_, said_ms)) = view.said.as_ref() {
        let t = *said_ms as f32 / 1000.0;
        if t < 0.7 {
            bob -= (t * 15.0).sin().abs() * 7.0 * (1.0 - t / 0.7);
        }
    }
    // A new face: it fades in over the old one and he hops a little as it lands.
    let change = ease_out(view.merchant_ms as f32 / 220.0);
    let hop = (1.0 - change) * 14.0;
    let grow = 1.0 + (1.0 - change) * 0.04;
    if let Some((code, ms)) = view.haggle.as_ref().and_then(|h| h.result) {
        let t = ms as f32 / 1000.0;
        let fade = (1.0 - t / 0.5).max(0.0);
        match code {
            2 | 3 | 5 => shake = (t * 60.0).sin() * 16.0 * fade,
            0 | 1 => bob = -(t * 18.0).sin().abs() * 14.0 * fade,
            _ => {}
        }
        if matches!(code, 3 | 5) {
            let k = (1.0 - t / 1.2).max(0.0);
            tint = [1.0, 1.0 - 0.35 * k, 1.0 - 0.35 * k, 1.0];
        }
    }
    faded(v, a, |v| {
        shifted(v, [-(1.0 - a) * 300.0 + shake, breathe + bob], |v| {
            soft(
                v,
                Rect::new(r.x + 40.0, r.y + 60.0, r.w * 0.8, r.h),
                60.0,
                50.0,
                rgba(0x010605, 0.55),
                false,
            );
            match uv {
                Some(uv) => {
                    if let Some(old) = from.filter(|_| change < 1.0) {
                        cutout(v, r, old, [tint[0], tint[1], tint[2], 1.0 - change]);
                    }
                    // Grown from his feet, so the hop lifts him off the counter's edge.
                    let (w, h) = (r.w * grow, r.h * grow);
                    let now = Rect::new(r.x + (r.w - w) * 0.5, r.y + r.h - h - hop, w, h);
                    cutout(v, now, uv, [tint[0], tint[1], tint[2], change.max(0.02)]);
                }
                None => stand_in(font, v, r, tint, time),
            }
        });
    });
    let na = entrance(open, 0.25, 0.3);
    let base = [40.0, 862.0];
    faded(v, na, |v| {
        shifted(v, [(1.0 - na) * -80.0, 0.0], |v| {
            turned(v, base, -0.09, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "El Tendero",
                    base[0],
                    base[1],
                    100.0,
                    0xffffff,
                    0x0b6a4a,
                    1.0,
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    "Tienda de la villa · se aceptan regateos",
                    base[0] + 14.0,
                    base[1] + 112.0,
                    27.0,
                    rgba(0xc8ffe8, 1.0),
                    rgba(EDGE, 0.9),
                );
            });
        });
    });
}

/// Until his illustration exists: his shape in shadow, rimmed in emerald, and a tag saying what
/// goes here.
fn stand_in(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, tint: [f32; 4], time: f32) {
    let body = Rect::new(r.x + 150.0, r.y + 250.0, 380.0, 560.0);
    let head = [r.x + 340.0, r.y + 190.0];
    let shade = |hex: u32| {
        [
            rgba(hex, 0.94)[0] * tint[0],
            rgba(hex, 0.94)[1] * tint[1],
            rgba(hex, 0.94)[2] * tint[2],
            0.94,
        ]
    };
    soft(
        v,
        body,
        170.0,
        22.0,
        rgba(EMERALD, 0.30 + 0.08 * wave(time, 0.4)),
        true,
    );
    halo(v, head, 82.0, 20.0, rgba(EMERALD, 0.28));
    panel(
        v,
        body,
        170.0,
        Panel {
            top: shade(0x0a2a20),
            bottom: shade(0x020806),
            ..Panel::fill(0, 0, 1.0)
        },
    );
    disc(v, head, 76.0, shade(0x0b2b21));
    // His hat: a brim and a crown.
    panel(
        v,
        Rect::new(head[0] - 118.0, head[1] - 74.0, 236.0, 26.0),
        13.0,
        Panel::fill(0x0e3326, 0x041510, 0.96),
    );
    panel(
        v,
        Rect::new(head[0] - 66.0, head[1] - 150.0, 132.0, 84.0),
        20.0,
        Panel::fill(0x0e3326, 0x041510, 0.96),
    );
    let tag = [r.x + 120.0, r.y + 300.0];
    turned(v, tag, -0.05, |v| {
        panel(
            v,
            Rect::new(tag[0] - 4.0, tag[1] - 4.0, 448.0, 108.0),
            10.0,
            Panel::fill(EDGE, EDGE, 0.95),
        );
        panel(
            v,
            Rect::new(tag[0], tag[1], 440.0, 100.0),
            8.0,
            Panel::fill(0xfaf6ee, 0xe6dccc, 1.0),
        );
        text(
            font,
            v,
            Face::Title,
            "ILUSTRACIÓN DEL TENDERO",
            tag[0] + 22.0,
            tag[1] + 14.0,
            26.0,
            rgba(0x0b3a2c, 1.0),
        );
        text(
            font,
            v,
            Face::Italic,
            "Próximamente · ui/shop/tendero.png",
            tag[0] + 22.0,
            tag[1] + 56.0,
            21.0,
            rgba(0x3a4a44, 1.0),
        );
    });
}

/// A big title over the page.
fn heading(font: &FontAtlas, v: &mut Vec<MapVertex>, x: f32, y: f32, words: &str, age: f32) {
    let a = entrance(age, 0.08, 0.3);
    faded(v, a, |v| {
        turned(v, [x, y], -0.04, |v| {
            let size = 54.0;
            shout(
                font,
                v,
                Face::Title,
                words,
                x,
                y,
                size,
                0xffffff,
                0x0b6a4a,
                1.0,
            );
            let tw = text_width(font, Face::Title, words, size);
            wire(
                v,
                [x, y + size + 12.0],
                [x + tw + 60.0, y + size + 12.0],
                6.0,
                rgba(0xffffff, 0.95),
                rgba(0xffffff, 0.0),
                0.0,
                false,
            );
        });
    });
}

/// A gold coin.
fn coin(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, alpha: f32) {
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
}

/// Her gold, top right.
fn purse(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, gold: u32) {
    soft(
        v,
        Rect::new(r.x, r.y + 8.0, r.w, r.h),
        r.h * 0.5,
        14.0,
        rgba(SHADOW, 0.7),
        false,
    );
    panel(
        v,
        r,
        r.h * 0.5,
        Panel::fill(0x0b3a2c, 0x04140f, 0.97)
            .rim(GOLD, 1.0, 2.2)
            .bevel(1.0),
    );
    coin(v, [r.x + 34.0, r.y + r.h * 0.5], 17.0, 1.0);
    text_shadow(
        font,
        v,
        Face::Ui,
        &format!("{gold} oro"),
        r.x + 62.0,
        r.y + (r.h - 30.0) * 0.5,
        30.0,
        rgba(GOLD_HI, 1.0),
    );
}

/// A slanted plate of a button: black-edged, emerald when `lit`, brighter rimmed under the
/// pointer, greyed when it cannot be used.
#[allow(clippy::too_many_arguments)]
fn plate(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    label: &str,
    lit: bool,
    hover: bool,
    enabled: bool,
    size: f32,
    time: f32,
) {
    turned(v, mid(r), TILT, |v| {
        soft(
            v,
            Rect::new(r.x + 8.0, r.y + 10.0, r.w, r.h),
            8.0,
            10.0,
            rgba(SHADOW, 0.7),
            false,
        );
        if lit && enabled {
            soft(
                v,
                r,
                8.0,
                18.0,
                rgba(EMERALD, 0.35 + 0.1 * wave(time, 0.8)),
                true,
            );
        }
        panel(v, grow(r, 3.0), 9.0, Panel::fill(EDGE, EDGE, 1.0));
        let (top, bottom, rim) = if !enabled {
            (0x1a2420, 0x0e1412, 0x4a5a52)
        } else if lit {
            (
                mix(EMERALD, 0xffffff, 0.1),
                mix(EMERALD, 0x000000, 0.45),
                GOLD_HI,
            )
        } else if hover {
            (0x1d4235, 0x0a1a15, GOLD_HI)
        } else {
            (0x12261f, 0x07120e, GOLD_DEEP)
        };
        let mut style = Panel::fill(top, bottom, 0.98).rim(rim, 1.0, 2.0).bevel(0.6);
        if lit && enabled {
            style = style.sheen(time * 0.25);
        }
        panel(v, r, 7.0, style);
        let ink = if enabled { INK } else { 0x8a9a92 };
        let size = fit(font, Face::Title, label, size, r.w - 24.0);
        let w = text_width(font, Face::Title, label, size);
        text_outline(
            font,
            v,
            Face::Title,
            label,
            r.x + (r.w - w) * 0.5,
            r.y + (r.h - size) * 0.5 - 2.0,
            size,
            rgba(ink, 1.0),
            rgba(EDGE, 0.8),
        );
    });
}

/// The red cursor at the left end of a tile turned by `angle`.
fn tile_cursor(v: &mut Vec<MapVertex>, r: Rect, angle: f32, time: f32) {
    let (sn, cs) = angle.sin_cos();
    let half = r.w * 0.5;
    let c = mid(r);
    cursor(v, [c[0] - half * cs - 14.0, c[1] - half * sn], 32.0, time);
}

/// The three tabs: the shelves to buy from, her things to sell, the altar.
fn tabs(font: &FontAtlas, v: &mut Vec<MapVertex>, f: &ShopFrame, page: u8, age: f32) {
    let lit = [
        page == 0 && f.tab == Tab::Buy,
        page == 0 && f.tab == Tab::Sell,
        page == 2,
    ];
    let hits = [ShopHit::TabBuy, ShopHit::TabSell, ShopHit::TabFuse];
    for (i, label) in ["Comprar", "Vender", "Fusión"].iter().enumerate() {
        let a = entrance(age, 0.04 * i as f32, 0.24);
        faded(v, a, |v| {
            shifted(v, [0.0, (1.0 - a) * -30.0], |v| {
                plate(
                    font,
                    v,
                    Layout::TABS[i],
                    label,
                    lit[i],
                    f.hover == Some(hits[i]),
                    true,
                    28.0,
                    f.time,
                );
            });
        });
    }
}

/// The shelves: tabs, filters, the products on tilted tiles and the lit one's card.
fn draw_shelves(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    view: &ShopView,
    age: f32,
) {
    tabs(font, v, f, 0, age);
    for (i, cat) in Category::ALL.iter().enumerate() {
        let r = Layout::category(i);
        let active = f.category == *cat;
        let over = f.hover == Some(ShopHit::Category(*cat));
        let (top, bottom, rim) = if active {
            (
                mix(EMERALD, 0xffffff, 0.08),
                mix(EMERALD, 0x000000, 0.5),
                GOLD_HI,
            )
        } else if over {
            (0x1d4235, 0x0a1a15, GOLD_HI)
        } else {
            (0x10221b, 0x07120e, GOLD_DEEP)
        };
        panel(v, grow(r, 2.0), r.h * 0.5, Panel::fill(EDGE, EDGE, 0.95));
        panel(
            v,
            r,
            r.h * 0.5,
            Panel::fill(top, bottom, 0.97).rim(rim, 1.0, 1.6),
        );
        text_shadow(
            font,
            v,
            Face::Title,
            &format!("{}", i + 1),
            r.x + 16.0,
            r.y + 9.0,
            20.0,
            rgba(GOLD_HI, 1.0),
        );
        let size = fit(font, Face::Ui, cat.label(), 19.0, r.w - 46.0);
        text_shadow(
            font,
            v,
            Face::Ui,
            cat.label(),
            r.x + 38.0,
            r.y + (r.h - size) * 0.5 - 1.0,
            size,
            rgba(INK, 1.0),
        );
    }
    let catalog = shop::catalog();
    let indices = shop::visible_indices(f.tab, f.category, f.inventory);
    let selected = f.selection.min(indices.len().saturating_sub(1));
    let start = selected / PAGE_SIZE * PAGE_SIZE;
    for (row, &index) in indices.iter().skip(start).take(PAGE_SIZE).enumerate() {
        let p = &catalog.products[index];
        let r = Layout::row(row);
        let lit = start + row == selected;
        let over = f.hover == Some(ShopHit::Item(start + row));
        let a = entrance(age, 0.08 + 0.035 * row as f32, 0.28);
        let price = if f.tab == Tab::Buy {
            p.buy_price
        } else {
            p.sell_price
        };
        let refused = view.refused.contains(&index);
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 260.0, 0.0], |v| {
                turned(v, mid(r), TILT, |v| {
                    soft(
                        v,
                        Rect::new(r.x + 10.0, r.y + 12.0, r.w, r.h),
                        9.0,
                        12.0,
                        rgba(SHADOW, 0.7),
                        false,
                    );
                    if lit {
                        soft(
                            v,
                            r,
                            9.0,
                            20.0,
                            rgba(EMERALD, 0.38 + 0.12 * wave(f.time, 0.8)),
                            true,
                        );
                    }
                    panel(v, grow(r, 4.0), 10.0, Panel::fill(EDGE, EDGE, 1.0));
                    let (top, bottom, rim) = if lit {
                        (
                            mix(0x0b4a3a, EMERALD, 0.45),
                            mix(0x03140f, EMERALD, 0.14),
                            GOLD_HI,
                        )
                    } else if over {
                        (0x173a2e, 0x07120e, GOLD)
                    } else {
                        (0x0e2219, 0x050c0a, GOLD_DEEP)
                    };
                    let mut style = Panel::fill(top, bottom, 0.98)
                        .rim(rim, 1.0, 2.2)
                        .hairline(6.0, if lit { 0.45 } else { 0.18 })
                        .bevel(0.6);
                    if lit {
                        style = style.sheen(f.time * 0.25);
                    }
                    panel(v, r, 8.0, style);
                    panel(
                        v,
                        Rect::new(r.x, r.y, 10.0, r.h),
                        3.0,
                        Panel::fill(mix(EMERALD, 0xffffff, 0.25), EMERALD, 1.0),
                    );
                    item_icon(
                        icons,
                        v,
                        &p.id,
                        Rect::new(r.x + 24.0, r.y + 7.0, 60.0, 60.0),
                        1.0,
                    );
                    let name_size = fit(font, Face::Title, &p.name, 28.0, 300.0);
                    text_outline(
                        font,
                        v,
                        Face::Title,
                        &p.name,
                        r.x + 98.0,
                        r.y + 8.0,
                        name_size,
                        rgba(INK, 1.0),
                        rgba(EDGE, 0.85),
                    );
                    let summary = if p.summary.is_empty() {
                        p.description.as_str()
                    } else {
                        p.summary.as_str()
                    };
                    let summary_size = fit(font, Face::Italic, summary, 19.0, 300.0);
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        summary,
                        r.x + 100.0,
                        r.y + 44.0,
                        summary_size,
                        rgba(CREAM, 0.92),
                    );
                    coin(v, [r.x + r.w - 112.0, r.y + 28.0], 13.0, 1.0);
                    text_shadow(
                        font,
                        v,
                        Face::Ui,
                        &price.to_string(),
                        r.x + r.w - 92.0,
                        r.y + 12.0,
                        28.0,
                        rgba(GOLD_HI, 1.0),
                    );
                    if refused {
                        text_shadow(
                            font,
                            v,
                            Face::Italic,
                            "sin regateo",
                            r.x + r.w - 128.0,
                            r.y + 46.0,
                            17.0,
                            rgba(0xff9aa8, 1.0),
                        );
                    } else if let Some(stock) = p.stock {
                        let left = f.stock.get(&p.id).copied().unwrap_or(stock);
                        text_shadow(
                            font,
                            v,
                            Face::Italic,
                            &format!("quedan {left}"),
                            r.x + r.w - 110.0,
                            r.y + 46.0,
                            17.0,
                            rgba(0xbfe8d0, 1.0),
                        );
                    }
                });
            });
        });
        if lit && a > 0.95 {
            tile_cursor(v, r, TILT, f.time);
        }
    }
    // Pages.
    let pages = indices.len().div_ceil(PAGE_SIZE).max(1);
    for (r, hit, enabled, dir) in [
        (Layout::PREVIOUS, ShopHit::PagePrevious, start > 0, -1.0),
        (
            Layout::NEXT,
            ShopHit::PageNext,
            start + PAGE_SIZE < indices.len(),
            1.0,
        ),
    ] {
        plate(
            font,
            v,
            r,
            "",
            false,
            f.hover == Some(hit),
            enabled,
            30.0,
            f.time,
        );
        let c = mid(r);
        let ink = rgba(if enabled { INK } else { 0x6a7a72 }, 1.0);
        for side in [-1.0, 1.0] {
            wire(
                v,
                [c[0] + 7.0 * dir, c[1]],
                [c[0] - 6.0 * dir, c[1] + 11.0 * side],
                4.0,
                ink,
                ink,
                0.0,
                false,
            );
        }
    }
    let label = format!(
        "{} / {pages}",
        if indices.is_empty() {
            1
        } else {
            start / PAGE_SIZE + 1
        }
    );
    let lw = text_width(font, Face::Ui, &label, 26.0);
    text_shadow(
        font,
        v,
        Face::Ui,
        &label,
        746.0 - lw * 0.5,
        936.0,
        26.0,
        rgba(CREAM, 1.0),
    );
    // The card.
    let a = entrance(age, 0.12, 0.3);
    faded(v, a, |v| {
        shifted(v, [(1.0 - a) * 120.0, 0.0], |v| {
            card(font, icons, v, f, view, &indices, selected);
        });
    });
}

/// The lit product's card: its model turning, its name on a white plate, what it is, the
/// Queen's thought, how many, what it costs, and the deal's buttons.
fn card(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    view: &ShopView,
    indices: &[usize],
    selected: usize,
) {
    let r = Layout::CARD;
    soft(
        v,
        Rect::new(r.x + 12.0, r.y + 16.0, r.w, r.h),
        14.0,
        18.0,
        rgba(SHADOW, 0.8),
        false,
    );
    panel(v, grow(r, 5.0), 16.0, Panel::fill(EDGE, EDGE, 1.0));
    panel(
        v,
        r,
        12.0,
        Panel::fill(0x0e221b, 0x040b09, 0.97)
            .rim(GOLD, 1.0, 2.2)
            .hairline(7.0, 0.28)
            .bevel(0.8),
    );
    panel(
        v,
        Rect::new(r.x, r.y, r.w, 8.0),
        4.0,
        Panel::fill(EMERALD, mix(EMERALD, 0x000000, 0.4), 1.0),
    );
    let Some(&index) = indices.get(selected) else {
        let words = if f.tab == Tab::Sell {
            "Tu mochila espera aventuras"
        } else {
            "Sin productos en esta categoría"
        };
        text_centered(
            font,
            v,
            Face::Title,
            words,
            r.x + r.w * 0.5,
            r.y + 300.0,
            36.0,
            rgba(INK, 1.0),
        );
        let hint = if f.tab == Tab::Sell {
            "Lo que compres aparecerá aquí para venderlo."
        } else {
            "Prueba otro filtro (1-5)."
        };
        text_centered(
            font,
            v,
            Face::Italic,
            hint,
            r.x + r.w * 0.5,
            r.y + 360.0,
            24.0,
            rgba(CREAM, 0.9),
        );
        plate(
            font,
            v,
            Layout::ACTION,
            "Ver productos",
            true,
            f.hover == Some(ShopHit::TabBuy),
            true,
            26.0,
            f.time,
        );
        return;
    };
    let p = &shop::catalog().products[index];
    // The stage, lit by the product's own colour.
    let stage = shop_preview::accent(p.preview_model.as_deref()).unwrap_or(match p.category {
        Category::Consumables => [0.36, 0.86, 0.66],
        Category::Materials => [0.88, 0.66, 0.36],
        Category::Equipment => [0.52, 0.72, 1.0],
        _ => [0.76, 0.56, 1.0],
    });
    let preview = Layout::PREVIEW;
    panel(v, grow(preview, 4.0), 10.0, Panel::fill(EDGE, EDGE, 1.0));
    shop_preview::backdrop(v, preview, stage, f.time);
    let art = Rect::new(
        preview.x + 16.0,
        preview.y + 16.0,
        preview.w - 32.0,
        preview.h - 32.0,
    );
    if !shop_preview::draw(
        v,
        p.preview_model.as_deref(),
        art,
        f.time,
        f.preview_rotation,
        f.liquid_motion,
    ) {
        let edge = art.w.min(art.h) - 20.0;
        item_icon(
            icons,
            v,
            &p.id,
            Rect::new(
                art.x + (art.w - edge) * 0.5,
                art.y + (art.h - edge) * 0.5,
                edge,
                edge,
            ),
            1.0,
        );
    }
    panel(v, preview, 8.0, Panel::fill(0, 0, 0.0).rim(GOLD, 1.0, 2.0));
    // Its name on a white plate.
    let name_size = fit(font, Face::Title, &p.name, 34.0, 300.0);
    let name_w = text_width(font, Face::Title, &p.name, name_size) + 40.0;
    let plate_r = Rect::new(1548.0, 272.0, name_w.min(330.0), 54.0);
    turned(v, mid(plate_r), -0.04, |v| {
        panel(v, grow(plate_r, 3.0), 8.0, Panel::fill(EDGE, EDGE, 1.0));
        panel(v, plate_r, 6.0, Panel::fill(0xfffaf4, 0xe8e0d2, 1.0));
        text(
            font,
            v,
            Face::Title,
            &p.name,
            plate_r.x + 20.0,
            plate_r.y + (plate_r.h - name_size) * 0.5 - 2.0,
            name_size,
            rgba(0x0b1a14, 1.0),
        );
    });
    let kind = p.category.singular();
    let kind_w = text_width(font, Face::Ui, kind, 17.0) + 28.0;
    panel(
        v,
        Rect::new(1556.0, 338.0, kind_w, 30.0),
        15.0,
        Panel::fill(EMERALD, mix(EMERALD, 0x000000, 0.4), 1.0),
    );
    text(
        font,
        v,
        Face::Ui,
        kind,
        1570.0,
        342.0,
        17.0,
        rgba(0xffffff, 1.0),
    );
    for (i, line) in wrap(font, Face::Body, &p.description, 21.0, 312.0)
        .iter()
        .take(8)
        .enumerate()
    {
        text_shadow(
            font,
            v,
            Face::Body,
            line,
            1552.0,
            380.0 + i as f32 * 27.0,
            21.0,
            rgba(INK, 0.95),
        );
    }
    let quote = p
        .queen_thought
        .as_deref()
        .unwrap_or("Todo objeto tiene una historia. Quizá esta merezca la pena.");
    speech_box(
        font,
        v,
        [1240.0, 764.0],
        620.0,
        "Reina",
        &format!("«{}»", quote.trim_matches(['«', '»'])),
        QUEEN,
        -0.012,
        1.0,
        0.86,
    );
    // What it does.
    let effect = if p.effect.is_empty() {
        &p.description
    } else {
        &p.effect
    };
    let effect_size = fit(font, Face::Italic, effect, 20.0, 540.0);
    text_shadow(
        font,
        v,
        Face::Title,
        "Efecto",
        1236.0,
        778.0,
        21.0,
        rgba(GOLD_HI, 1.0),
    );
    text_shadow(
        font,
        v,
        Face::Italic,
        effect,
        1314.0,
        779.0,
        effect_size,
        rgba(0xc8ffe8, 1.0),
    );
    // How many.
    let owned = f.inventory.get(&p.id).copied().unwrap_or(0);
    let available = if f.tab == Tab::Sell {
        Some(owned)
    } else {
        p.stock.map(|s| f.stock.get(&p.id).copied().unwrap_or(s))
    };
    text_shadow(
        font,
        v,
        Face::Ui,
        "Cantidad",
        1236.0,
        832.0,
        23.0,
        rgba(CREAM, 1.0),
    );
    plate(
        font,
        v,
        Layout::MINUS,
        "−",
        false,
        f.hover == Some(ShopHit::QuantityMinus),
        f.quantity > 1,
        30.0,
        f.time,
    );
    let can_add = f.quantity < 99 && available.is_none_or(|s| f.quantity < s);
    plate(
        font,
        v,
        Layout::PLUS,
        "+",
        false,
        f.hover == Some(ShopHit::QuantityPlus),
        can_add,
        30.0,
        f.time,
    );
    let value = Rect::new(1452.0, 820.0, 76.0, 50.0);
    panel(
        v,
        value,
        8.0,
        Panel::fill(0xfffaf4, 0xe8e0d2, 1.0).rim(GOLD_DEEP, 1.0, 1.6),
    );
    let q = f.quantity.to_string();
    let qw = text_width(font, Face::Title, &q, 30.0);
    text(
        font,
        v,
        Face::Title,
        &q,
        value.x + (value.w - qw) * 0.5,
        value.y + 8.0,
        30.0,
        rgba(0x0b1a14, 1.0),
    );
    text_shadow(
        font,
        v,
        Face::Italic,
        &format!("en mochila: {owned}"),
        1612.0,
        834.0,
        19.0,
        rgba(0xbfe8d0, 1.0),
    );
    // What it costs, or what is wrong.
    let unit = if f.tab == Tab::Buy {
        p.buy_price
    } else {
        p.sell_price
    };
    let total = unit.saturating_mul(f.quantity);
    let can_act =
        available.is_none_or(|n| n >= f.quantity) && (f.tab == Tab::Sell || f.gold >= total);
    let status = f.notice.map(str::to_owned).unwrap_or_else(|| {
        if available.is_some_and(|n| n < f.quantity) {
            format!("Disponibles: {}.", available.unwrap_or(0))
        } else if f.tab == Tab::Buy && total > f.gold {
            format!("Te faltan {} de oro.", total - f.gold)
        } else if f.tab == Tab::Buy {
            format!("Él te la compraría por {} oro.", p.sell_price)
        } else {
            format!("En la tienda cuesta {} oro.", p.buy_price)
        }
    });
    let status_size = fit(font, Face::Italic, &status, 20.0, 400.0);
    text_shadow(
        font,
        v,
        Face::Italic,
        &status,
        1236.0,
        882.0,
        status_size,
        rgba(if can_act { 0xc8ffe8 } else { 0xff9aa8 }, 1.0),
    );
    let price = total.to_string();
    let pw = text_width(font, Face::Title, &price, 36.0);
    coin(v, [1840.0 - pw - 22.0, 896.0], 14.0, 1.0);
    text_outline(
        font,
        v,
        Face::Title,
        &price,
        1840.0 - pw,
        874.0,
        36.0,
        rgba(GOLD_HI, 1.0),
        rgba(EDGE, 0.85),
    );
    let label = if f.tab == Tab::Buy {
        "Comprar"
    } else {
        "Vender"
    };
    plate(
        font,
        v,
        Layout::ACTION,
        label,
        true,
        f.hover == Some(ShopHit::Action),
        can_act,
        28.0,
        f.time,
    );
    let refused = view.refused.contains(&index);
    plate(
        font,
        v,
        Layout::HAGGLE,
        if refused {
            "Sin regateo"
        } else {
            "Regatear · H"
        },
        false,
        f.hover == Some(ShopHit::Haggle),
        !refused,
        26.0,
        f.time,
    );
}

/// The haggle: the deal as it stands, his patience, the bar with its needle, the tactics, and
/// the stamp of the last try.
fn draw_haggle(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    view: &ShopView,
    age: f32,
) {
    let Some(h) = view.haggle.as_ref() else {
        return;
    };
    let Some(p) = shop::catalog().products.get(h.product) else {
        return;
    };
    let fresh = h.result.map_or(9.0, |(_, ms)| ms as f32 / 1000.0);
    // ---- the deal
    let r = Rect::new(620.0, 236.0, 790.0, 300.0);
    let a = entrance(age, 0.06, 0.3);
    faded(v, a, |v| {
        shifted(v, [0.0, (1.0 - a) * -50.0], |v| {
            turned(v, mid(r), -0.012, |v| {
                soft(
                    v,
                    Rect::new(r.x + 12.0, r.y + 16.0, r.w, r.h),
                    14.0,
                    18.0,
                    rgba(SHADOW, 0.8),
                    false,
                );
                panel(v, grow(r, 5.0), 16.0, Panel::fill(EDGE, EDGE, 1.0));
                panel(
                    v,
                    r,
                    12.0,
                    Panel::fill(0x0e221b, 0x040b09, 0.97)
                        .rim(GOLD, 1.0, 2.2)
                        .hairline(7.0, 0.28)
                        .bevel(0.8),
                );
                panel(
                    v,
                    Rect::new(r.x, r.y, r.w, 8.0),
                    4.0,
                    Panel::fill(EMERALD, mix(EMERALD, 0x000000, 0.4), 1.0),
                );
                disc(v, [r.x + 86.0, r.y + 96.0], 66.0, rgba(0x02100b, 1.0));
                halo(v, [r.x + 86.0, r.y + 96.0], 66.0, 16.0, rgba(EMERALD, 0.3));
                item_icon(
                    icons,
                    v,
                    &p.id,
                    Rect::new(r.x + 36.0, r.y + 46.0, 100.0, 100.0),
                    1.0,
                );
                let name = format!("{} × {}", p.name, h.quantity);
                let size = fit(font, Face::Title, &name, 36.0, 430.0);
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &name,
                    r.x + 176.0,
                    r.y + 26.0,
                    size,
                    rgba(INK, 1.0),
                    rgba(EDGE, 0.85),
                );
                let (was_label, now_label) = if h.selling {
                    ("Él pagaba", "Ahora te paga")
                } else {
                    ("Precio de la tienda", "Tu precio")
                };
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    was_label,
                    r.x + 178.0,
                    r.y + 80.0,
                    20.0,
                    rgba(CREAM, 0.85),
                );
                let was = (h.base * h.quantity).to_string();
                text_shadow(
                    font,
                    v,
                    Face::Ui,
                    &format!("{was} oro"),
                    r.x + 178.0,
                    r.y + 102.0,
                    30.0,
                    rgba(0xb8c8c0, 1.0),
                );
                if h.percent > 0 {
                    let w = text_width(font, Face::Ui, &format!("{was} oro"), 30.0);
                    wire(
                        v,
                        [r.x + 172.0, r.y + 124.0],
                        [r.x + 186.0 + w, r.y + 119.0],
                        3.0,
                        rgba(CRIMSON, 1.0),
                        rgba(CRIMSON, 1.0),
                        0.0,
                        false,
                    );
                }
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    now_label,
                    r.x + 178.0,
                    r.y + 150.0,
                    20.0,
                    rgba(CREAM, 0.85),
                );
                let now = (h.price * h.quantity).to_string();
                coin(v, [r.x + 200.0, r.y + 222.0], 20.0, 1.0);
                // The price swells a moment when it moves.
                let pop = if fresh < 0.4 && matches!(h.result, Some((0 | 1 | 6, _))) {
                    1.0 + 0.18 * (1.0 - fresh / 0.4)
                } else {
                    1.0
                };
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &now,
                    r.x + 232.0,
                    r.y + 222.0 - 36.0 * pop,
                    72.0 * pop,
                    rgba(GOLD_HI, 1.0),
                    rgba(EDGE, 0.9),
                );
                if h.gold_short > 0 {
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        &format!("Te faltan {} de oro.", h.gold_short),
                        r.x + 26.0,
                        r.y + 262.0,
                        20.0,
                        rgba(0xff9aa8, 1.0),
                    );
                } else {
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        &format!("Intento {} / {}", h.rounds, h.max_rounds),
                        r.x + 26.0,
                        r.y + 262.0,
                        20.0,
                        rgba(CREAM, 0.85),
                    );
                }
                // His patience: hearts.
                text_shadow(
                    font,
                    v,
                    Face::Title,
                    "Paciencia",
                    r.x + 560.0,
                    r.y + 200.0,
                    22.0,
                    rgba(CREAM, 1.0),
                );
                for k in 0..3u8 {
                    let c = [r.x + 580.0 + k as f32 * 58.0, r.y + 258.0];
                    let full = k < h.patience;
                    heart(
                        v,
                        c,
                        22.0,
                        if full { CRIMSON } else { 0x2a1a20 },
                        if full { 1.0 } else { 0.8 },
                    );
                }
            });
        });
    });
    // How far the price moved: a burst with its figure.
    if h.percent > 0 {
        let c = [1300.0, 300.0];
        let pulse = if fresh < 0.5 {
            1.0 + 0.25 * (1.0 - fresh / 0.5)
        } else {
            1.0
        };
        let words = format!("{}{}%", if h.selling { "+" } else { "−" }, h.percent);
        turned(v, c, -0.14, |v| {
            burst(
                v,
                c,
                46.0 * pulse,
                74.0 * pulse,
                14,
                f.time * 0.3,
                rgba(GOLD_HI, 0.96),
                rgba(EDGE, 0.95),
                5.0,
            );
            let size = 38.0 * pulse;
            let w = text_width(font, Face::Title, &words, size);
            shout(
                font,
                v,
                Face::Title,
                &words,
                c[0] - w * 0.5,
                c[1] - size * 0.55,
                size,
                0xffffff,
                0x8a5a10,
                1.0,
            );
        });
    }
    // ---- the bar
    let bar = Layout::BAR;
    let a = entrance(age, 0.14, 0.3);
    faded(v, a, |v| {
        panel(v, grow(bar, 4.0), 14.0, Panel::fill(EDGE, EDGE, 1.0));
        panel(
            v,
            bar,
            10.0,
            Panel::fill(0x07120e, 0x020604, 1.0).rim(GOLD_DEEP, 1.0, 2.0),
        );
        if h.half > 0.0 && !h.closed {
            let zx = bar.x + bar.w * (h.zone - h.half).max(0.0);
            let zw = bar.w * ((h.zone + h.half).min(1.0) - (h.zone - h.half).max(0.0));
            panel(
                v,
                Rect::new(zx, bar.y + 6.0, zw, bar.h - 12.0),
                6.0,
                Panel::fill(
                    mix(EMERALD, 0xffffff, 0.15),
                    mix(EMERALD, 0x000000, 0.3),
                    0.9,
                ),
            );
            let pw = bar.w * h.half * 0.7;
            let px = bar.x + bar.w * h.zone - pw * 0.5;
            panel(
                v,
                Rect::new(px, bar.y + 10.0, pw, bar.h - 20.0),
                4.0,
                Panel::fill(GOLD_HI, GOLD, 1.0),
            );
        }
        for k in 1..10 {
            let x = bar.x + bar.w * k as f32 / 10.0;
            wire(
                v,
                [x, bar.y + bar.h - 10.0],
                [x, bar.y + bar.h - 4.0],
                2.0,
                rgba(0xffffff, 0.3),
                rgba(0xffffff, 0.3),
                0.0,
                false,
            );
        }
        if !h.closed {
            let x = bar.x + bar.w * h.needle;
            wire(
                v,
                [x, bar.y - 16.0],
                [x, bar.y + bar.h + 16.0],
                6.0,
                rgba(0xffffff, 1.0),
                rgba(0xffffff, 1.0),
                10.0,
                false,
            );
            push_triangle(
                v,
                [
                    [x - 12.0, bar.y - 30.0],
                    [x + 12.0, bar.y - 30.0],
                    [x, bar.y - 14.0],
                ],
                rgba(CRIMSON, 1.0),
            );
        }
        text_shadow(
            font,
            v,
            Face::Italic,
            "Para la aguja dentro de la zona: cuanto más estrecha, más se mueve el precio.",
            bar.x,
            bar.y + bar.h + 22.0,
            20.0,
            rgba(CREAM, 0.9),
        );
    });
    // ---- the tactics
    let total = h.price * h.quantity;
    let tiles = [
        ("Halagar", "Zona amplia · poco · sin riesgo".to_owned()),
        (
            "Regatear",
            "Zona media · más · −1 paciencia si fallas".to_owned(),
        ),
        (
            "Intimidar",
            "Zona estrecha · mucho · −2 paciencia".to_owned(),
        ),
        (
            "Cerrar trato",
            if h.selling {
                format!("Cobrar {total} oro")
            } else {
                format!("Pagar {total} oro")
            },
        ),
        ("Dejarlo", "Volver sin trato".to_owned()),
    ];
    for (i, (label, hint)) in tiles.iter().enumerate() {
        let r = Layout::tactic(i);
        let lit = i == h.tactic as usize && !h.closed;
        let over = f.hover == Some(ShopHit::Tactic(i as u8));
        let locked = h.closed || (i < 3 && h.rounds >= h.max_rounds);
        let a = entrance(age, 0.12 + 0.06 * i as f32, 0.3);
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 280.0, 0.0], |v| {
                turned(v, mid(r), TILT, |v| {
                    tactic_tile(font, v, r, i, label, hint, lit, over, locked, f.time);
                });
            });
        });
        if lit && a > 0.95 {
            tile_cursor(v, r, TILT, f.time);
        }
    }
    // ---- «Otro trato»: the favour he asks, once he has hinted at it
    if let Some(fv) = h.favor.as_ref() {
        let r = Layout::FAVOR;
        let lit = h.tactic as usize == 5 && !h.closed;
        let over = f.hover == Some(ShopHit::Tactic(5));
        let a = entrance(fv.age_ms as f32 / 1000.0, 0.0, 0.4);
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 320.0, 0.0], |v| {
                turned(v, mid(r), TILT, |v| {
                    favor_tile(font, v, r, fv, lit, over, f.hover, f.time);
                });
            });
        });
        if lit && a > 0.95 {
            tile_cursor(v, r, TILT, f.time);
        }
    }
    // ---- the stamp of the last try
    if let Some((code, ms)) = h.result {
        let t = ms as f32 / 1000.0;
        if t < 1.4 {
            let pop = entrance(t, 0.0, 0.16);
            let alpha = 1.0 - ((t - 1.05) / 0.35).clamp(0.0, 1.0);
            let (words, fill, shadow) = match code {
                0 => ("¡GENIAL!", GOLD_HI, 0x8a5a10),
                1 => ("¡BIEN!", MINT, 0x0b6a4a),
                2 => ("¡FALLO!", 0xa8b4c0, 0x2a3440),
                3 => ("¡OFENDIDO!", 0xff6a7a, 0x6a0a1a),
                4 => ("¡TRATO!", GOLD_HI, 0x8a5a10),
                6 => ("¡TRATO!", 0xffc4dc, 0x8a1a4a),
                _ => ("¡FUERA!", 0xff6a7a, 0x6a0a1a),
            };
            let size = 84.0 * (0.6 + 0.4 * pop);
            let tw = text_width(font, Face::Title, words, size);
            let c = [1015.0, 828.0];
            faded(v, alpha, |v| {
                turned(v, c, -0.12, |v| {
                    burst(
                        v,
                        c,
                        tw * 0.36,
                        tw * 0.6 * pop,
                        16,
                        f.time * 0.6,
                        rgba(fill, 0.9),
                        rgba(EDGE, 0.95),
                        6.0,
                    );
                    shout(
                        font,
                        v,
                        Face::Title,
                        words,
                        c[0] - tw * 0.5,
                        c[1] - size * 0.55,
                        size,
                        0xffffff,
                        shadow,
                        1.0,
                    );
                });
            });
        }
    }
}

/// A heart (his patience).
fn heart(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, hex: u32, alpha: f32) {
    let color = rgba(hex, alpha);
    disc(v, [c[0] - r * 0.48, c[1] - r * 0.25], r * 0.55, color);
    disc(v, [c[0] + r * 0.48, c[1] - r * 0.25], r * 0.55, color);
    push_triangle(
        v,
        [
            [c[0] - r * 1.02, c[1] - r * 0.1],
            [c[0] + r * 1.02, c[1] - r * 0.1],
            [c[0], c[1] + r * 0.95],
        ],
        color,
    );
}

/// «Otro trato»'s tile: rose where the others are green, its number, the favour and its cut, a
/// «NUEVO» badge on a favour he has not asked before, and arrows to the others she can offer.
#[allow(clippy::too_many_arguments)]
fn favor_tile(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    fv: &FavorTile,
    lit: bool,
    over: bool,
    hover: Option<ShopHit>,
    time: f32,
) {
    // It calls a moment when it comes, and softly after.
    let fresh = (1.0 - fv.age_ms as f32 / 1600.0).clamp(0.0, 1.0);
    let call = 0.22 + 0.16 * wave(time, 0.7) + 0.4 * fresh;
    soft(
        v,
        Rect::new(r.x + 10.0, r.y + 12.0, r.w, r.h),
        9.0,
        12.0,
        rgba(SHADOW, 0.7),
        false,
    );
    soft(
        v,
        r,
        9.0,
        26.0,
        rgba(
            QUEEN,
            if lit {
                0.5 + 0.12 * wave(time, 0.8)
            } else {
                call
            },
        ),
        true,
    );
    panel(v, grow(r, 4.0), 10.0, Panel::fill(EDGE, EDGE, 1.0));
    let (top, bottom, rim) = if lit {
        (
            mix(0x3a0b24, QUEEN, 0.5),
            mix(0x14040c, QUEEN, 0.18),
            GOLD_HI,
        )
    } else if over {
        (0x4a1430, 0x1a0610, GOLD)
    } else {
        (0x34102a, 0x12050c, GOLD_DEEP)
    };
    let mut style = Panel::fill(top, bottom, 0.98).rim(rim, 1.0, 2.2).bevel(0.6);
    if lit {
        style = style.sheen(time * 0.25);
    }
    panel(v, r, 8.0, style);
    panel(
        v,
        Rect::new(r.x, r.y, 10.0, r.h),
        3.0,
        Panel::fill(mix(QUEEN, 0xffffff, 0.25), QUEEN, 1.0),
    );
    text_shadow(
        font,
        v,
        Face::Title,
        "6",
        r.x + 26.0,
        r.y + 24.0,
        34.0,
        rgba(GOLD_HI, 1.0),
    );
    heart(v, [r.x + 40.0, r.y + 84.0], 13.0, QUEEN, 1.0);
    text_outline(
        font,
        v,
        Face::Title,
        "Otro trato",
        r.x + 70.0,
        r.y + 10.0,
        32.0,
        rgba(INK, 1.0),
        rgba(EDGE, 0.85),
    );
    let cut = if fv.percent >= 100 {
        "gratis".to_owned()
    } else {
        format!("−{} %", fv.percent)
    };
    let name = fit(font, Face::Italic, &fv.name, 21.0, r.w - 210.0);
    text_shadow(
        font,
        v,
        Face::Italic,
        &fv.name,
        r.x + 72.0,
        r.y + 52.0,
        name,
        rgba(CREAM, 0.98),
    );
    text_outline(
        font,
        v,
        Face::Title,
        &cut,
        r.x + 72.0,
        r.y + 76.0,
        28.0,
        rgba(0xffc4dc, 1.0),
        rgba(EDGE, 0.9),
    );
    if fv.new {
        let b = Rect::new(r.x + r.w - 118.0, r.y + 12.0, 102.0, 30.0);
        panel(
            v,
            b,
            6.0,
            Panel::fill(GOLD_HI, GOLD, 1.0).rim(EDGE, 1.0, 1.6),
        );
        let w = text_width(font, Face::Title, "NUEVO", 20.0);
        text_shadow(
            font,
            v,
            Face::Title,
            "NUEVO",
            b.x + (b.w - w) * 0.5,
            b.y + 3.0,
            20.0,
            rgba(EDGE, 1.0),
        );
    }
    if fv.choices > 1 {
        for by in [-1i8, 1] {
            let a = Layout::favor_arrow(by);
            let on = hover
                == Some(if by < 0 {
                    ShopHit::FavorPrev
                } else {
                    ShopHit::FavorNext
                });
            panel(
                v,
                a,
                6.0,
                Panel::fill(if on { 0x5a1a3a } else { 0x24081a }, 0x10040a, 1.0).rim(
                    if on { GOLD_HI } else { GOLD_DEEP },
                    1.0,
                    1.6,
                ),
            );
            let c = mid(a);
            let s = if by < 0 { -1.0 } else { 1.0 };
            push_triangle(
                v,
                [
                    [c[0] - 8.0 * s, c[1] - 10.0],
                    [c[0] - 8.0 * s, c[1] + 10.0],
                    [c[0] + 9.0 * s, c[1]],
                ],
                rgba(INK, 1.0),
            );
        }
    }
}

/// One tactic's tile: its number, its name, what it risks.
#[allow(clippy::too_many_arguments)]
fn tactic_tile(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    i: usize,
    label: &str,
    hint: &str,
    lit: bool,
    over: bool,
    locked: bool,
    time: f32,
) {
    let glow = match i {
        2 => CRIMSON,
        3 => GOLD,
        4 => 0x6a8a80,
        _ => EMERALD,
    };
    soft(
        v,
        Rect::new(r.x + 10.0, r.y + 12.0, r.w, r.h),
        9.0,
        12.0,
        rgba(SHADOW, 0.7),
        false,
    );
    if lit {
        soft(
            v,
            r,
            9.0,
            22.0,
            rgba(glow, 0.4 + 0.12 * wave(time, 0.8)),
            true,
        );
    }
    panel(v, grow(r, 4.0), 10.0, Panel::fill(EDGE, EDGE, 1.0));
    let (top, bottom, rim) = if locked {
        (0x1a2420, 0x0b100e, 0x4a5a52)
    } else if lit {
        (
            mix(0x0b3a2c, glow, 0.45),
            mix(0x03140f, glow, 0.16),
            GOLD_HI,
        )
    } else if over {
        (0x173a2e, 0x07120e, GOLD)
    } else {
        (0x0e2219, 0x050c0a, GOLD_DEEP)
    };
    let mut style = Panel::fill(top, bottom, 0.98).rim(rim, 1.0, 2.2).bevel(0.6);
    if lit {
        style = style.sheen(time * 0.25);
    }
    panel(v, r, 8.0, style);
    panel(
        v,
        Rect::new(r.x, r.y, 10.0, r.h),
        3.0,
        Panel::fill(
            mix(glow, 0xffffff, 0.25),
            glow,
            if locked { 0.4 } else { 1.0 },
        ),
    );
    let ink = if locked { 0x8a9a92 } else { INK };
    let n = format!("{}", i + 1);
    text_shadow(
        font,
        v,
        Face::Title,
        &n,
        r.x + 26.0,
        r.y + 20.0,
        34.0,
        rgba(if locked { 0x6a7a72 } else { GOLD_HI }, 1.0),
    );
    text_outline(
        font,
        v,
        Face::Title,
        label,
        r.x + 70.0,
        r.y + 10.0,
        32.0,
        rgba(ink, 1.0),
        rgba(EDGE, 0.85),
    );
    let size = fit(font, Face::Italic, hint, 19.0, r.w - 90.0);
    text_shadow(
        font,
        v,
        Face::Italic,
        hint,
        r.x + 72.0,
        r.y + 50.0,
        size,
        rgba(if locked { 0x7a8a82 } else { CREAM }, 0.95),
    );
}

/// The fusion altar: her things on tiles, the two sockets, what they make, the fee, and the
/// show when it goes off.
fn draw_fuse(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    view: &ShopView,
    age: f32,
) {
    tabs(font, v, f, 2, age);
    let Some(fz) = view.fuse.as_ref() else {
        return;
    };
    // ---- her things
    let start = fz.cursor / FUSE_PAGE * FUSE_PAGE;
    if fz.items.is_empty() {
        text_shadow(
            font,
            v,
            Face::Title,
            "No llevas nada que fusionar.",
            640.0,
            300.0,
            30.0,
            rgba(INK, 1.0),
        );
    }
    for (k, (id, name, count)) in fz.items.iter().enumerate().skip(start).take(FUSE_PAGE) {
        let r = Layout::fuse_tile(k - start);
        let lit = k == fz.cursor;
        let over = f.hover == Some(ShopHit::FuseItem(k));
        let placed = fz
            .slots
            .iter()
            .flatten()
            .filter(|(sid, _)| sid == id)
            .count();
        let a = entrance(age, 0.06 + 0.02 * (k - start) as f32, 0.26);
        faded(v, a, |v| {
            shifted(v, [0.0, (1.0 - a) * 40.0], |v| {
                soft(
                    v,
                    Rect::new(r.x + 8.0, r.y + 10.0, r.w, r.h),
                    10.0,
                    12.0,
                    rgba(SHADOW, 0.7),
                    false,
                );
                if lit {
                    soft(
                        v,
                        r,
                        12.0,
                        20.0,
                        rgba(EMERALD, 0.42 + 0.12 * wave(f.time, 0.8)),
                        true,
                    );
                }
                panel(v, grow(r, 3.0), 13.0, Panel::fill(EDGE, EDGE, 1.0));
                let (top, bottom, rim) = if lit {
                    (
                        mix(0x0b4a3a, EMERALD, 0.4),
                        mix(0x03140f, EMERALD, 0.14),
                        GOLD_HI,
                    )
                } else if over {
                    (0x173a2e, 0x07120e, GOLD)
                } else {
                    (0x0e2219, 0x050c0a, GOLD_DEEP)
                };
                panel(
                    v,
                    r,
                    10.0,
                    Panel::fill(top, bottom, 0.98).rim(rim, 1.0, 2.0).bevel(0.6),
                );
                item_icon(
                    icons,
                    v,
                    id,
                    Rect::new(r.x + 30.0, r.y + 10.0, 72.0, 72.0),
                    1.0,
                );
                let size = fit(font, Face::Ui, name, 16.0, r.w - 14.0);
                let w = text_width(font, Face::Ui, name, size);
                text_shadow(
                    font,
                    v,
                    Face::Ui,
                    name,
                    r.x + (r.w - w) * 0.5,
                    r.y + 92.0,
                    size,
                    rgba(INK, 1.0),
                );
                let badge = format!("×{count}");
                text_shadow(
                    font,
                    v,
                    Face::Title,
                    &badge,
                    r.x + r.w - 12.0 - text_width(font, Face::Title, &badge, 18.0),
                    r.y + 6.0,
                    18.0,
                    rgba(GOLD_HI, 1.0),
                );
                if placed > 0 {
                    disc(v, [r.x + 16.0, r.y + 16.0], 7.0, rgba(MINT, 1.0));
                }
            });
        });
    }
    if fz.items.len() > FUSE_PAGE {
        let pages = fz.items.len().div_ceil(FUSE_PAGE);
        let label = format!("{} / {pages}", start / FUSE_PAGE + 1);
        text_shadow(
            font,
            v,
            Face::Ui,
            &label,
            1100.0,
            830.0,
            24.0,
            rgba(CREAM, 1.0),
        );
    }
    // ---- the altar
    let a = entrance(age, 0.1, 0.3);
    faded(v, a, |v| {
        shifted(v, [(1.0 - a) * 120.0, 0.0], |v| {
            altar(font, icons, v, f, fz);
        });
    });
}

fn altar(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    fz: &FuseView,
) {
    let r = Layout::CARD;
    soft(
        v,
        Rect::new(r.x + 12.0, r.y + 16.0, r.w, r.h),
        14.0,
        18.0,
        rgba(SHADOW, 0.8),
        false,
    );
    panel(v, grow(r, 5.0), 16.0, Panel::fill(EDGE, EDGE, 1.0));
    panel(
        v,
        r,
        12.0,
        Panel::fill(0x0e221b, 0x040b09, 0.97)
            .rim(GOLD, 1.0, 2.2)
            .hairline(7.0, 0.28)
            .bevel(0.8),
    );
    panel(
        v,
        Rect::new(r.x, r.y, r.w, 8.0),
        4.0,
        Panel::fill(EMERALD, mix(EMERALD, 0x000000, 0.4), 1.0),
    );
    // A seal turning under the result's socket.
    let (rc, rr) = Layout::SOCKETS[2];
    let spin = f.time * 0.4;
    for k in 0..3 {
        arc(
            v,
            rc,
            rr + 34.0 + k as f32 * 12.0,
            2.0,
            spin * (1.0 + k as f32 * 0.3) + k as f32 * 0.21,
            0.66,
            rgba(EMERALD, 0.35 - 0.08 * k as f32),
            [0.0; 4],
        );
    }
    let tag = "EN PRUEBAS · más recetas pronto";
    let tw = text_width(font, Face::Ui, tag, 15.0) + 26.0;
    panel(
        v,
        Rect::new(r.x + r.w - tw - 18.0, r.y + 22.0, tw, 28.0),
        14.0,
        Panel::fill(0x3a2a0a, 0x1a1204, 0.95).rim(GOLD, 1.0, 1.2),
    );
    text(
        font,
        v,
        Face::Ui,
        tag,
        r.x + r.w - tw - 5.0,
        r.y + 26.0,
        15.0,
        rgba(GOLD_HI, 1.0),
    );
    let fusing = fz.show.as_ref().map(|s| s.ms as f32 / 1000.0);
    // The sockets.
    for slot in 0..2 {
        let (c, rad) = Layout::SOCKETS[slot];
        let over = f.hover == Some(ShopHit::FuseSlot(slot as u8));
        socket(v, c, rad, over, f.time);
        if let Some((id, name)) = &fz.slots[slot] {
            item_icon(
                icons,
                v,
                id,
                Rect::new(c[0] - 52.0, c[1] - 52.0, 104.0, 104.0),
                1.0,
            );
            let size = fit(font, Face::Ui, name, 19.0, 200.0);
            let w = text_width(font, Face::Ui, name, size);
            text_shadow(
                font,
                v,
                Face::Ui,
                name,
                c[0] - w * 0.5,
                c[1] + rad + 12.0,
                size,
                rgba(INK, 1.0),
            );
        } else if fusing.is_none() {
            text_centered(
                font,
                v,
                Face::Italic,
                "vacío",
                c[0],
                c[1] - 12.0,
                22.0,
                rgba(0x7a9a8e, 1.0),
            );
        }
    }
    let plus = [
        (Layout::SOCKETS[0].0[0] + Layout::SOCKETS[1].0[0]) * 0.5,
        Layout::SOCKETS[0].0[1],
    ];
    wire(
        v,
        [plus[0] - 22.0, plus[1]],
        [plus[0] + 22.0, plus[1]],
        8.0,
        rgba(GOLD_HI, 1.0),
        rgba(GOLD_HI, 1.0),
        0.0,
        true,
    );
    wire(
        v,
        [plus[0], plus[1] - 22.0],
        [plus[0], plus[1] + 22.0],
        8.0,
        rgba(GOLD_HI, 1.0),
        rgba(GOLD_HI, 1.0),
        0.0,
        true,
    );
    for slot in 0..2 {
        let (c, rad) = Layout::SOCKETS[slot];
        let from = [c[0] + (rc[0] - c[0]) * 0.3, c[1] + rad * 0.9];
        let to = [rc[0] + (c[0] - rc[0]) * 0.25, rc[1] - rr * 0.95];
        wire(
            v,
            from,
            to,
            3.0,
            rgba(EMERALD, 0.5),
            rgba(EMERALD, 0.9),
            4.0,
            false,
        );
    }
    // What comes out.
    socket(v, rc, rr, false, f.time);
    match (fusing, &fz.show) {
        (Some(t), Some(show)) => fusion_show(font, icons, v, show, t, f.time),
        _ => {
            if let Some((id, name)) = &fz.made {
                halo(
                    v,
                    rc,
                    rr * 0.9,
                    30.0,
                    rgba(GOLD_HI, 0.35 + 0.1 * wave(f.time, 0.6)),
                );
                item_icon(
                    icons,
                    v,
                    id,
                    Rect::new(rc[0] - 66.0, rc[1] - 66.0, 132.0, 132.0),
                    1.0,
                );
                text_centered(
                    font,
                    v,
                    Face::Title,
                    name,
                    rc[0],
                    rc[1] + rr + 14.0,
                    26.0,
                    rgba(GOLD_HI, 1.0),
                );
            } else if let Some((id, name)) = &fz.result {
                item_icon(
                    icons,
                    v,
                    id,
                    Rect::new(rc[0] - 60.0, rc[1] - 60.0, 120.0, 120.0),
                    0.85,
                );
                text_centered(
                    font,
                    v,
                    Face::Title,
                    name,
                    rc[0],
                    rc[1] + rr + 14.0,
                    26.0,
                    rgba(INK, 1.0),
                );
                text_centered(
                    font,
                    v,
                    Face::Italic,
                    "receta conocida",
                    rc[0],
                    rc[1] + rr + 48.0,
                    19.0,
                    rgba(MINT, 1.0),
                );
            } else if fz.slots.iter().all(Option::is_some) {
                text_centered(
                    font,
                    v,
                    Face::Title,
                    "???",
                    rc[0],
                    rc[1] - 26.0,
                    48.0,
                    rgba(0x7a9a8e, 1.0),
                );
                text_centered(
                    font,
                    v,
                    Face::Italic,
                    "no reaccionan",
                    rc[0],
                    rc[1] + rr + 14.0,
                    21.0,
                    rgba(0xff9aa8, 1.0),
                );
            } else {
                text_centered(
                    font,
                    v,
                    Face::Italic,
                    "dos cosas en el altar",
                    rc[0],
                    rc[1] - 14.0,
                    21.0,
                    rgba(0x7a9a8e, 1.0),
                );
            }
        }
    }
    // The fee, a notice, the buttons.
    coin(v, [1250.0, 866.0], 14.0, 1.0);
    text_shadow(
        font,
        v,
        Face::Ui,
        &format!("Tarifa: {} oro", fz.fee),
        1272.0,
        852.0,
        23.0,
        rgba(GOLD_HI, 1.0),
    );
    if !fz.notice.is_empty() {
        let size = fit(font, Face::Italic, &fz.notice, 21.0, 380.0);
        let ok = fz.notice.starts_with("Obtuviste");
        text_shadow(
            font,
            v,
            Face::Italic,
            &fz.notice,
            1480.0,
            853.0,
            size,
            rgba(if ok { MINT } else { 0xff9aa8 }, 1.0),
        );
    }
    let ready = fz.result.is_some() && fusing.is_none() && f.gold >= fz.fee;
    plate(
        font,
        v,
        Layout::FUSE_GO,
        "Fusionar · E",
        true,
        f.hover == Some(ShopHit::FuseGo),
        ready,
        26.0,
        f.time,
    );
    plate(
        font,
        v,
        Layout::FUSE_CLEAR,
        "Vaciar · Q",
        false,
        f.hover == Some(ShopHit::FuseClear),
        fusing.is_none() && fz.slots.iter().any(Option::is_some),
        26.0,
        f.time,
    );
}

/// An altar socket: a dark well in a gold ring, lit under the pointer.
fn socket(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, over: bool, time: f32) {
    halo(
        v,
        c,
        r,
        18.0,
        rgba(
            EMERALD,
            if over {
                0.45
            } else {
                0.22 + 0.06 * wave(time, 0.5)
            },
        ),
    );
    panel(
        v,
        Rect::new(
            c[0] - r - 5.0,
            c[1] - r - 5.0,
            (r + 5.0) * 2.0,
            (r + 5.0) * 2.0,
        ),
        r + 5.0,
        Panel::fill(EDGE, EDGE, 1.0),
    );
    panel(
        v,
        Rect::new(c[0] - r, c[1] - r, r * 2.0, r * 2.0),
        r,
        Panel::fill(0x0a2a20, 0x020806, 1.0)
            .rim(if over { GOLD_HI } else { GOLD }, 1.0, 2.6)
            .bevel(0.8),
    );
}

/// The fusion's show `t` seconds in: the two spiral into the well, a flash, a burst with
/// «¡FUSIÓN!» and what it made.
fn fusion_show(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    show: &FuseShow,
    t: f32,
    time: f32,
) {
    let (rc, rr) = Layout::SOCKETS[2];
    let travel = (t / 0.7).clamp(0.0, 1.0);
    if travel < 1.0 {
        let e = travel * travel;
        for (slot, id) in show.from.iter().enumerate() {
            let (c, _) = Layout::SOCKETS[slot];
            let side = if slot == 0 { 1.0 } else { -1.0 };
            let angle = e * std::f32::consts::TAU * 1.2 * side;
            let along = [c[0] + (rc[0] - c[0]) * e, c[1] + (rc[1] - c[1]) * e];
            let swirl = 70.0 * (1.0 - e);
            let at = [
                along[0] + angle.cos() * swirl,
                along[1] + angle.sin() * swirl,
            ];
            let size = 104.0 * (1.0 - 0.6 * e);
            halo(v, at, size * 0.5, 16.0, rgba(MINT, 0.4));
            item_icon(
                icons,
                v,
                id,
                Rect::new(at[0] - size * 0.5, at[1] - size * 0.5, size, size),
                1.0 - 0.3 * e,
            );
            for k in 0..3 {
                let back = (e - 0.08 * (k + 1) as f32).max(0.0);
                let ang = back * std::f32::consts::TAU * 1.2 * side;
                let p = [
                    c[0] + (rc[0] - c[0]) * back + ang.cos() * 70.0 * (1.0 - back),
                    c[1] + (rc[1] - c[1]) * back + ang.sin() * 70.0 * (1.0 - back),
                ];
                sparkle(v, p, 10.0 - 2.0 * k as f32, rgba(0xfff0c8, 0.8), 0.8);
            }
        }
    }
    let flash = ((t - 0.62) / 0.3).clamp(0.0, 1.0);
    if flash > 0.0 && flash < 1.0 {
        halo(
            v,
            rc,
            rr * (0.4 + 1.6 * flash),
            60.0,
            rgba(0xffffff, 0.8 * (1.0 - flash)),
        );
    }
    if t >= 0.85 {
        let pop = entrance(t, 0.85, 0.2);
        halo(v, rc, rr, 34.0, rgba(GOLD_HI, 0.45));
        let size = 132.0 * (0.5 + 0.5 * pop);
        item_icon(
            icons,
            v,
            &show.made.0,
            Rect::new(rc[0] - size * 0.5, rc[1] - size * 0.5, size, size),
            1.0,
        );
        let words = "¡FUSIÓN!";
        let s = 70.0 * (0.6 + 0.4 * pop);
        let w = text_width(font, Face::Title, words, s);
        let c = [rc[0], rc[1] - rr - 70.0];
        turned(v, c, -0.1, |v| {
            burst(
                v,
                c,
                w * 0.36,
                w * 0.58 * pop,
                16,
                time * 0.6,
                rgba(MINT, 0.92),
                rgba(EDGE, 0.95),
                6.0,
            );
            shout(
                font,
                v,
                Face::Title,
                words,
                c[0] - w * 0.5,
                c[1] - s * 0.55,
                s,
                0xffffff,
                0x0b6a4a,
                1.0,
            );
        });
        text_centered(
            font,
            v,
            Face::Title,
            &show.made.1,
            rc[0],
            rc[1] + rr + 14.0,
            26.0,
            rgba(GOLD_HI, pop),
        );
    }
}

/// Keys at the foot.
fn hints(font: &FontAtlas, v: &mut Vec<MapVertex>, keys: &[(&str, &str)]) {
    let mut x = 620.0;
    let y = 1012.0;
    for (key, what) in keys {
        let cap_w = (text_width(font, Face::Title, key, 17.0) + 24.0).max(40.0);
        keycap(
            font,
            v,
            Rect::new(x, y, cap_w, 38.0),
            key,
            17.0,
            false,
            false,
            EMERALD,
            1.0,
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            what,
            x + cap_w + 10.0,
            y + 8.0,
            20.0,
            rgba(CREAM, 0.95),
        );
        x += cap_w + 10.0 + text_width(font, Face::Italic, what, 20.0) + 30.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_filters_and_buttons_share_layout() {
        let shelves = |selection, count| Pick {
            page: 0,
            selection,
            count,
            ..Pick::default()
        };
        let r = Layout::row(7);
        let c = mid(r);
        assert_eq!(
            Layout::hit(c[0], c[1], &shelves(0, 16)),
            Some(ShopHit::Item(7))
        );
        let r = Layout::row(0);
        assert_eq!(
            Layout::hit(r.x + 40.0, mid(r)[1], &shelves(8, 16)),
            Some(ShopHit::Item(8))
        );
        let next = mid(Layout::NEXT);
        assert_eq!(
            Layout::hit(next[0], next[1], &shelves(0, 16)),
            Some(ShopHit::PageNext)
        );
        assert_eq!(Layout::hit(next[0], next[1], &shelves(8, 16)), None);
        let previous = mid(Layout::PREVIOUS);
        assert_eq!(
            Layout::hit(previous[0], previous[1], &shelves(8, 16)),
            Some(ShopHit::PagePrevious)
        );
        let all = mid(Layout::category(0));
        assert_eq!(
            Layout::hit(all[0], all[1], &shelves(0, 16)),
            Some(ShopHit::Category(Category::All))
        );
        let minus = mid(Layout::MINUS);
        assert_eq!(
            Layout::hit(minus[0], minus[1], &shelves(0, 16)),
            Some(ShopHit::QuantityMinus)
        );
        let close = mid(Layout::CLOSE);
        assert_eq!(
            Layout::hit(close[0], close[1], &shelves(0, 0)),
            Some(ShopHit::Close)
        );
        let action = mid(Layout::ACTION);
        assert_eq!(
            Layout::hit(action[0], action[1], &shelves(0, 0)),
            Some(ShopHit::TabBuy)
        );
        let haggle = mid(Layout::HAGGLE);
        assert_eq!(
            Layout::hit(haggle[0], haggle[1], &shelves(0, 4)),
            Some(ShopHit::Haggle)
        );
        let fuse = mid(Layout::TABS[2]);
        assert_eq!(
            Layout::hit(fuse[0], fuse[1], &shelves(0, 4)),
            Some(ShopHit::TabFuse)
        );
        // A haggle answers only on its tiles (and the way out).
        let haggling = Pick {
            page: 1,
            ..Pick::default()
        };
        let t = mid(Layout::tactic(3));
        assert_eq!(Layout::hit(t[0], t[1], &haggling), Some(ShopHit::Tactic(3)));
        assert_eq!(Layout::hit(fuse[0], fuse[1], &haggling), None);
        // The altar: her things, its sockets, its buttons.
        let altar = Pick {
            page: 2,
            fuse_count: 20,
            fuse_cursor: 17,
            ..Pick::default()
        };
        let tile = mid(Layout::fuse_tile(1));
        assert_eq!(
            Layout::hit(tile[0], tile[1], &altar),
            Some(ShopHit::FuseItem(17))
        );
        let (socket, _) = Layout::SOCKETS[1];
        assert_eq!(
            Layout::hit(socket[0], socket[1], &altar),
            Some(ShopHit::FuseSlot(1))
        );
        let go = mid(Layout::FUSE_GO);
        assert_eq!(Layout::hit(go[0], go[1], &altar), Some(ShopHit::FuseGo));
        assert_eq!(shop::page_selection(7, 16, 1), 8);
        assert_eq!(shop::page_selection(8, 16, 1), 8);
        assert_eq!(shop::page_selection(0, 0, 1), 0);
        let mut inv = BTreeMap::new();
        inv.insert("tela_resistente".to_owned(), 2);
        let i = shop::visible_indices(Tab::Sell, Category::Materials, &inv);
        assert_eq!(i.len(), 1);
        assert_eq!(shop::catalog().products[i[0]].id, "tela_resistente");
        assert!(shop::visible_indices(Tab::Sell, Category::Consumables, &inv).is_empty());
    }
}
