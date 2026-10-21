//! The Tendero's counter at the mine's mouth: a shop of his own, built like the villa's (the same
//! entrances, plates, cursor and keys) but dressed as a miner's — lamp-lit timber and rock strata
//! instead of emerald slashes. Picks and bags sit on iron-strapped crates, each set against the
//! pick and bag she carries (how hard and how often it bites, how many blows the granite takes,
//! the hardest rock it breaks); what the shaft gave her goes on his scales at his price, one kind
//! at a time or everything at once.
use super::super::shop::{self, Tab, PAGE_SIZE};
use super::persona::*;
use super::shop_overlay::{coin, fit, inside_turned, item_icon, mid, Pick, ShopFrame, ShopView};
use super::ui_paint::*;
use super::*;
use demon_queen_dungeon::pozo::{Gear, Rock, BAGS, PICKS};

type Uv = ([f32; 2], [f32; 2]);

/// The lantern's light, its ember, the counter's copper and iron, coal, timber, his paper.
const LAMP: u32 = 0xffb347;
const EMBER: u32 = 0xff7a2a;
const COPPER: u32 = 0xc8743a;
const IRON: u32 = 0x2b2e33;
const IRON_HI: u32 = 0x9aa0a8;
const COAL: u32 = 0x0e0b09;
const WOOD: u32 = 0x6e4626;
const WOOD_DARK: u32 = 0x35200f;
const PAPER: u32 = 0xf3e4c4;
const DUST: u32 = 0xe8c9a0;
const EDGE: u32 = 0x070504;
/// How far the planks and crates lean.
const TILT: f32 = -0.018;

pub(super) struct Layout;
impl Layout {
    pub const TABS: [Rect; 2] = [
        Rect::new(620.0, 130.0, 270.0, 58.0),
        Rect::new(904.0, 130.0, 270.0, 58.0),
    ];
    pub const CLOSE: Rect = Rect::new(1756.0, 130.0, 124.0, 58.0);
    pub const CARD: Rect = Rect::new(1206.0, 214.0, 680.0, 770.0);
    pub const NICHE: Rect = Rect::new(1234.0, 242.0, 232.0, 232.0);
    pub const MINUS: Rect = Rect::new(1386.0, 818.0, 56.0, 50.0);
    pub const PLUS: Rect = Rect::new(1538.0, 818.0, 56.0, 50.0);
    pub const ACTION: Rect = Rect::new(1232.0, 910.0, 300.0, 56.0);
    pub const ALL: Rect = Rect::new(1548.0, 910.0, 314.0, 56.0);
    pub const PREVIOUS: Rect = Rect::new(620.0, 924.0, 64.0, 46.0);
    pub const NEXT: Rect = Rect::new(808.0, 924.0, 64.0, 46.0);

    pub fn row(row: usize) -> Rect {
        Rect::new(620.0, 214.0 + row as f32 * 87.0, 560.0, 78.0)
    }

    /// What lies under the pointer.
    pub fn hit(x: f32, y: f32, pick: &Pick) -> Option<ShopHit> {
        if Self::CLOSE.contains(x, y) {
            return Some(ShopHit::Close);
        }
        for (rect, hit) in Self::TABS.iter().zip([ShopHit::TabBuy, ShopHit::TabSell]) {
            if inside_turned(*rect, TILT, x, y) {
                return Some(hit);
            }
        }
        let count = pick.count;
        let start = pick.selection.min(count.saturating_sub(1)) / PAGE_SIZE * PAGE_SIZE;
        for row in 0..PAGE_SIZE.min(count.saturating_sub(start)) {
            if inside_turned(Self::row(row), TILT, x, y) {
                return Some(ShopHit::Item(start + row));
            }
        }
        if count > 0 {
            if inside_turned(Self::ACTION, TILT, x, y) {
                return Some(ShopHit::Action);
            }
            if pick.selling {
                if inside_turned(Self::ALL, TILT, x, y) {
                    return Some(ShopHit::SellAll);
                }
                if Self::MINUS.contains(x, y) {
                    return Some(ShopHit::QuantityMinus);
                }
                if Self::PLUS.contains(x, y) {
                    return Some(ShopHit::QuantityPlus);
                }
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

/// His counter.
pub(super) fn draw(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    view: Option<&ShopView>,
    merchant: Option<Uv>,
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
    dwarf(
        font,
        v,
        merchant,
        open,
        f.time,
        view.said.as_ref().map(|(_, ms)| *ms),
    );
    sign(font, v, open, f.time);
    purse(font, v, Rect::new(1630.0, 40.0, 250.0, 60.0), f.gold, open);
    let buying = f.tab == Tab::Buy;
    for (i, label) in ["Picos y sacos", "Vender mineral"].iter().enumerate() {
        let a = entrance(open, 0.1 + 0.05 * i as f32, 0.26);
        let hit = if i == 0 {
            ShopHit::TabBuy
        } else {
            ShopHit::TabSell
        };
        faded(v, a, |v| {
            shifted(v, [0.0, (1.0 - a) * -30.0], |v| {
                button(
                    font,
                    v,
                    Layout::TABS[i],
                    label,
                    buying == (i == 0),
                    f.hover == Some(hit),
                    true,
                    27.0,
                    f.time,
                );
            });
        });
    }
    button(
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
    let indices = shop::visible_indices(f.tab, f.category, f.inventory);
    let selected = f.selection.min(indices.len().saturating_sub(1));
    shelves(font, icons, v, f, &indices, selected, age);
    let a = entrance(age, 0.1, 0.3);
    faded(v, a, |v| {
        shifted(v, [(1.0 - a) * 60.0, 0.0], |v| {
            card(font, icons, v, f, indices.get(selected).copied(), &indices);
        });
    });
    if let Some((words, ms)) = &view.said {
        let k = entrance(*ms as f32 / 1000.0, 0.0, 0.25);
        speech_box(
            font,
            v,
            [44.0, 330.0],
            540.0,
            "Tendero",
            words,
            COPPER,
            -0.02,
            k,
            1.0,
        );
    }
    let mut keys = vec![
        ("W S", "elegir"),
        ("Tab", "pestañas"),
        ("Enter", "aceptar"),
    ];
    if !buying {
        keys.insert(2, ("+ −", "cantidad"));
        keys.push(("T", "vender todo"));
    }
    keys.push(("Esc", "salir"));
    hints(font, v, &keys);
}

/// Dark warm rock behind the counter: the shaft's strata leaning across, ore glinting in them,
/// the gallery's timbers framing it, a lantern's light and dust drifting down through it.
fn backdrop(v: &mut Vec<MapVertex>, open: f32, time: f32) {
    push_rect(v, Rect::new(0.0, 0.0, 1920.0, 1080.0), rgba(0x070402, 0.8));
    let a = entrance(open, 0.0, 0.4);
    faded(v, a, |v| {
        // the strata: earth, stone, slate, granite… one over the other, down to the right
        let bands: [(u32, f32); 8] = [
            (0x2a1e15, 120.0),
            (0x1d1510, 150.0),
            (0x33251a, 110.0),
            (0x181210, 170.0),
            (0x2b211a, 130.0),
            (0x14100e, 160.0),
            (0x231b17, 140.0),
            (0x100c0b, 200.0),
        ];
        shifted(v, [(1.0 - a) * 400.0, 0.0], |v| {
            let mut y = -40.0;
            for (hex, h) in bands {
                slash(
                    v,
                    [1260.0, y + h * 0.5],
                    2700.0,
                    h + 2.0,
                    -0.07,
                    rgba(hex, 0.95),
                    rgba(mix(hex, 0x000000, 0.45), 0.95),
                );
                // the seam between two rocks, and a pale vein along it
                slash(
                    v,
                    [1260.0, y + h],
                    2700.0,
                    3.0,
                    -0.07,
                    rgba(0x000000, 0.55),
                    rgba(0x000000, 0.55),
                );
                slash(
                    v,
                    [1260.0, y + h - 5.0],
                    2700.0,
                    1.5,
                    -0.07,
                    rgba(DUST, 0.07),
                    rgba(DUST, 0.07),
                );
                y += h;
            }
        });
        // ore in the rock, catching the lamp now and then
        for i in 0..22u32 {
            let h = |k: u32| ((i.wrapping_mul(2654435761).wrapping_add(k * 40503)) % 1000) as f32 / 1000.0;
            let c = [640.0 + h(1) * 1240.0, 220.0 + h(2) * 760.0];
            let hue = [LAMP, 0xd94a5a, 0x5ab0ff, 0x4fd18b, 0xc9c9d6][(h(3) * 5.0) as usize % 5];
            let glint = ((time * (0.4 + h(4) * 0.5) + h(5) * 6.28).sin() * 0.5 + 0.5).powf(10.0);
            // a little cluster of ore in the rock
            for (dx, dy, r) in [(0.0, 0.0, 2.6), (4.0, 2.5, 1.8), (-3.0, 3.5, 1.5)] {
                disc(v, [c[0] + dx, c[1] + dy], r, rgba(hue, 0.22 + 0.4 * glint));
            }
            if glint > 0.85 {
                sparkle(v, c, 7.0 * glint, rgba(0xffffff, 0.45 * glint), 0.5);
            }
        }
        // the gallery's timbers: a lintel and the posts at its sides
        timber(v, Rect::new(560.0, -6.0, 1380.0, 26.0), 1.0);
        timber(v, Rect::new(588.0, 0.0, 24.0, 1080.0), 0.9);
        timber(v, Rect::new(1898.0, 0.0, 24.0, 1080.0), 0.9);
        // his lantern's light over him, flickering
        let flicker = 0.85 + 0.15 * ((time * 7.3).sin() * 0.5 + (time * 3.1).sin() * 0.5);
        halo(
            v,
            [330.0, 560.0],
            300.0,
            160.0,
            rgba(LAMP, 0.07 * flicker),
        );
        // dust drifting down
        for i in 0..36u32 {
            let h = |k: u32| ((i.wrapping_mul(2246822519).wrapping_add(k * 3266489917)) % 1000) as f32 / 1000.0;
            let fall = 14.0 + h(1) * 22.0;
            let y = (h(2) * 1080.0 + time * fall).rem_euclid(1080.0);
            let x = h(3) * 1920.0 + (time * 0.4 + h(4) * 6.0).sin() * 12.0;
            disc(v, [x, y], 1.2 + h(5) * 1.8, rgba(DUST, 0.12 + 0.12 * h(6)));
        }
    });
}

/// A timber of the gallery: dark wood with an iron band near each end.
fn timber(v: &mut Vec<MapVertex>, r: Rect, alpha: f32) {
    panel(
        v,
        r,
        3.0,
        Panel::fill(WOOD_DARK, mix(WOOD_DARK, 0x000000, 0.45), alpha)
            .rim(0x120a05, alpha, 1.5)
            .bevel(0.5),
    );
    let along = r.w > r.h;
    for k in [0.12f32, 0.88] {
        let band = if along {
            Rect::new(r.x + r.w * k - 5.0, r.y, 10.0, r.h)
        } else {
            Rect::new(r.x, r.y + r.h * k - 5.0, r.w, 10.0)
        };
        push_rect(v, band, rgba(IRON, 0.9 * alpha));
    }
}

/// The Tendero at the left, lit by his lantern, his name across his chest.
fn dwarf(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    uv: Option<Uv>,
    open: f32,
    time: f32,
    said_ms: Option<u32>,
) {
    let a = entrance(open, 0.06, 0.4);
    let r = Rect::new(-30.0, 380.0, 700.0, 700.0);
    let breathe = (time * 1.3).sin() * 3.0;
    let mut bob = 0.0;
    if let Some(ms) = said_ms {
        let t = ms as f32 / 1000.0;
        if t < 0.7 {
            bob -= (t * 15.0).sin().abs() * 7.0 * (1.0 - t / 0.7);
        }
    }
    faded(v, a, |v| {
        shifted(v, [-(1.0 - a) * 300.0, breathe + bob], |v| {
            soft(
                v,
                Rect::new(r.x + 60.0, r.y + 80.0, r.w * 0.8, r.h),
                60.0,
                50.0,
                rgba(0x050302, 0.6),
                false,
            );
            if let Some(uv) = uv {
                cutout(v, r, uv, [1.0, 0.97, 0.92, 1.0]);
            }
        });
    });
    let na = entrance(open, 0.25, 0.3);
    let base = [40.0, 862.0];
    faded(v, na, |v| {
        shifted(v, [(1.0 - na) * -80.0, 0.0], |v| {
            turned(v, base, -0.07, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "El Tendero",
                    base[0],
                    base[1],
                    100.0,
                    PAPER,
                    0x6a300c,
                    1.0,
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    "La Bocamina · paga un poco menos que la villa",
                    base[0] + 14.0,
                    base[1] + 112.0,
                    26.0,
                    rgba(DUST, 1.0),
                    rgba(EDGE, 0.9),
                );
            });
        });
    });
}

/// His sign, hung from the lintel on two chains, swinging a little.
fn sign(font: &FontAtlas, v: &mut Vec<MapVertex>, open: f32, time: f32) {
    let a = entrance(open, 0.12, 0.35);
    let r = Rect::new(626.0, 34.0, 470.0, 78.0);
    let swing = (time * 0.9).sin() * 0.01;
    faded(v, a, |v| {
        shifted(v, [0.0, (1.0 - a) * -120.0], |v| {
            for x in [r.x + 56.0, r.x + r.w - 56.0] {
                wire(
                    v,
                    [x, 8.0],
                    [x, r.y + 10.0],
                    4.0,
                    rgba(IRON_HI, 0.95),
                    rgba(IRON, 0.95),
                    0.0,
                    true,
                );
            }
            turned(v, [r.x + r.w * 0.5, 8.0], swing, |v| {
                plank(v, r, 6.0, false, false, 1.0);
                let size = fit(font, Face::Title, "La Bocamina", 52.0, r.w - 50.0);
                let w = text_width(font, Face::Title, "La Bocamina", size);
                text_outline(
                    font,
                    v,
                    Face::Title,
                    "La Bocamina",
                    r.x + (r.w - w) * 0.5,
                    r.y + (r.h - size) * 0.5 - 4.0,
                    size,
                    rgba(PAPER, 1.0),
                    rgba(EDGE, 0.9),
                );
            });
            text_shadow(
                font,
                v,
                Face::Italic,
                "Picos · sacos · compra de mineral",
                r.x + r.w + 28.0,
                r.y + 26.0,
                26.0,
                rgba(LAMP, 0.95),
            );
        });
    });
}

/// Her gold, on an iron plate.
fn purse(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, gold: u32, open: f32) {
    let a = entrance(open, 0.18, 0.3);
    faded(v, a, |v| {
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
            Panel::fill(0x2a2420, 0x110d0b, 0.97)
                .rim(COPPER, 1.0, 2.2)
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
    });
}

/// A plank: timber edged in black, its grain, a nail at each corner; lamp-lit when `lit`.
fn plank(v: &mut Vec<MapVertex>, r: Rect, radius: f32, lit: bool, hover: bool, alpha: f32) {
    soft(
        v,
        Rect::new(r.x + 8.0, r.y + 10.0, r.w, r.h),
        radius + 2.0,
        10.0,
        rgba(SHADOW, 0.7 * alpha),
        false,
    );
    panel(v, grow(r, 3.0), radius + 2.0, Panel::fill(EDGE, EDGE, alpha));
    let (top, bottom, rim) = if lit {
        (mix(WOOD, LAMP, 0.42), mix(WOOD_DARK, EMBER, 0.22), LAMP)
    } else if hover {
        (mix(WOOD, 0xffffff, 0.1), WOOD_DARK, IRON_HI)
    } else {
        (WOOD, WOOD_DARK, 0x1c1410)
    };
    panel(
        v,
        r,
        radius,
        Panel::fill(top, bottom, alpha).rim(rim, alpha, 2.0).bevel(0.7),
    );
    for k in 1..4 {
        let y = r.y + r.h * k as f32 / 4.0 + ((k * 37) % 5) as f32 - 2.0;
        push_rect(
            v,
            Rect::new(r.x + 12.0, y, r.w - 24.0, 1.5),
            rgba(0x000000, 0.16 * alpha),
        );
    }
    for (x, y) in [
        (r.x + 11.0, r.y + 11.0),
        (r.x + r.w - 11.0, r.y + 11.0),
        (r.x + 11.0, r.y + r.h - 11.0),
        (r.x + r.w - 11.0, r.y + r.h - 11.0),
    ] {
        disc(v, [x, y], 3.4, rgba(IRON_HI, alpha));
        disc(v, [x - 0.9, y - 0.9], 1.2, rgba(0xffffff, 0.55 * alpha));
    }
}

/// A plank button with its words, leaning, greyed when it cannot be used.
#[allow(clippy::too_many_arguments)]
fn button(
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
        if lit && enabled {
            soft(
                v,
                r,
                8.0,
                18.0,
                rgba(LAMP, 0.32 + 0.1 * wave(time, 0.8)),
                true,
            );
        }
        plank(v, r, 7.0, lit && enabled, hover && enabled, if enabled { 1.0 } else { 0.55 });
        let ink = if enabled { PAPER } else { 0x9a8a78 };
        let size = fit(font, Face::Title, label, size, r.w - 30.0);
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
            rgba(EDGE, 0.85),
        );
    });
}

/// The crates: picks and bags (or her ore), a page of them, the lit one marked.
fn shelves(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    indices: &[usize],
    selected: usize,
    age: f32,
) {
    let catalog = shop::catalog();
    let start = selected / PAGE_SIZE * PAGE_SIZE;
    for (row, &index) in indices.iter().skip(start).take(PAGE_SIZE).enumerate() {
        let p = &catalog.products[index];
        let r = Layout::row(row);
        let lit = start + row == selected;
        let over = f.hover == Some(ShopHit::Item(start + row));
        let a = entrance(age, 0.08 + 0.035 * row as f32, 0.28);
        let (price, count) = match f.tab {
            Tab::Buy => (shop::buy_price(p), None),
            Tab::Sell => (
                shop::sell_price(p),
                f.inventory.get(&p.id).copied().filter(|n| *n > 0),
            ),
        };
        let line = match f.tab {
            Tab::Buy => gear_line(&p.id).unwrap_or_else(|| p.summary.clone()),
            Tab::Sell => format!("{} oro cada uno aquí", shop::sell_price(p)),
        };
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * -60.0, 0.0], |v| {
                turned(v, mid(r), TILT, |v| {
                    if lit {
                        soft(v, r, 8.0, 16.0, rgba(LAMP, 0.3 + 0.08 * wave(f.time, 0.7)), true);
                    }
                    plank(v, r, 6.0, lit, over, 1.0);
                    // the crate's iron straps
                    for x in [r.x + 22.0, r.x + r.w - 30.0] {
                        push_rect(v, Rect::new(x, r.y - 3.0, 8.0, r.h + 6.0), rgba(IRON, 0.95));
                        for y in [r.y + 6.0, r.y + r.h - 6.0] {
                            disc(v, [x + 4.0, y], 2.4, rgba(IRON_HI, 1.0));
                        }
                    }
                    // the thing, in a dark niche
                    let niche = Rect::new(r.x + 42.0, r.y + 8.0, 62.0, 62.0);
                    panel(
                        v,
                        niche,
                        8.0,
                        Panel::fill(COAL, 0x1d140d, 1.0).rim(0x000000, 1.0, 1.5),
                    );
                    item_icon(icons, v, &p.id, grow(niche, -5.0), 1.0);
                    if let Some(n) = count {
                        let badge = format!("×{n}");
                        let w = text_width(font, Face::Ui, &badge, 18.0) + 14.0;
                        let b = Rect::new(niche.x + niche.w - w + 6.0, niche.y + niche.h - 20.0, w, 24.0);
                        panel(v, b, 10.0, Panel::fill(EMBER, 0x8a3a10, 1.0).rim(EDGE, 1.0, 1.5));
                        text_shadow(font, v, Face::Ui, &badge, b.x + 7.0, b.y + 2.0, 18.0, rgba(PAPER, 1.0));
                    }
                    let name_size = fit(font, Face::Title, &p.name, 28.0, 300.0);
                    text_outline(
                        font,
                        v,
                        Face::Title,
                        &p.name,
                        r.x + 118.0,
                        r.y + 9.0,
                        name_size,
                        rgba(PAPER, 1.0),
                        rgba(EDGE, 0.8),
                    );
                    let line_size = fit(font, Face::Italic, &line, 19.0, 320.0);
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        &line,
                        r.x + 120.0,
                        r.y + 46.0,
                        line_size,
                        rgba(DUST, 0.95),
                    );
                    coin(v, [r.x + r.w - 118.0, r.y + r.h * 0.5], 12.0, 1.0);
                    text_shadow(
                        font,
                        v,
                        Face::Ui,
                        &price.to_string(),
                        r.x + r.w - 100.0,
                        r.y + r.h * 0.5 - 14.0,
                        25.0,
                        rgba(GOLD_HI, 1.0),
                    );
                });
                if lit {
                    cursor(v, [r.x - 14.0, r.y + r.h * 0.5], 32.0, f.time);
                }
            });
        });
    }
    if indices.is_empty() {
        let (title, line) = match f.tab {
            Tab::Buy => (
                "Ya tienes lo mejor que vendo",
                "—Más hondo no hay pico que valga. Vuelva por mineral.",
            ),
            Tab::Sell => (
                "No traes nada del pozo",
                "Baja al pozo y pica: él compra todo lo que sale.",
            ),
        };
        let r = Layout::row(0);
        plank(v, Rect::new(r.x, r.y, r.w, 120.0), 6.0, false, false, 0.9);
        text_outline(
            font,
            v,
            Face::Title,
            title,
            r.x + 28.0,
            r.y + 20.0,
            fit(font, Face::Title, title, 30.0, r.w - 56.0),
            rgba(PAPER, 1.0),
            rgba(EDGE, 0.8),
        );
        text_shadow(font, v, Face::Italic, line, r.x + 30.0, r.y + 70.0, 21.0, rgba(DUST, 0.95));
    }
    // the pages, when there are more than fit
    let pages = indices.len().div_ceil(PAGE_SIZE).max(1);
    if pages > 1 {
        let page = start / PAGE_SIZE;
        for (r, enabled, hit) in [
            (Layout::PREVIOUS, page > 0, ShopHit::PagePrevious),
            (Layout::NEXT, page + 1 < pages, ShopHit::PageNext),
        ] {
            plank(v, r, 6.0, false, f.hover == Some(hit) && enabled, if enabled { 1.0 } else { 0.4 });
            // (the chevron points down: turned to point back or on)
            let turn = if hit == ShopHit::PagePrevious {
                std::f32::consts::FRAC_PI_2
            } else {
                -std::f32::consts::FRAC_PI_2
            };
            turned(v, mid(r), turn, |v| {
                chevron(
                    v,
                    mid(r),
                    12.0,
                    rgba(PAPER, if enabled { 1.0 } else { 0.4 }),
                );
            });
        }
        text_centered(
            font,
            v,
            Face::Ui,
            &format!("{} / {pages}", page + 1),
            746.0,
            934.0,
            22.0,
            rgba(PAPER, 0.9),
        );
    }
}

/// A pick's or bag's line on its crate.
fn gear_line(id: &str) -> Option<String> {
    if let Some(p) = PICKS.iter().find(|p| p.id == id) {
        return Some(format!(
            "Golpe {} · {:.1} por segundo · rompe {}",
            p.damage,
            p.rate,
            breaks(p.tier).name().to_lowercase()
        ));
    }
    BAGS.iter()
        .find(|b| b.id == id)
        .map(|b| format!("Le caben {} piezas del pozo", b.capacity))
}

/// The hardest rock a pick of `tier` breaks.
fn breaks(tier: u8) -> Rock {
    Rock::ALL
        .iter()
        .rev()
        .copied()
        .find(|r| r.tier() <= tier)
        .unwrap_or(Rock::Tierra)
}

/// Blows a pick of `damage` takes on granite (how it feels halfway down).
fn granite_blows(damage: f32) -> u32 {
    (Rock::Granito.hardness(40) / damage.max(0.1)).ceil() as u32
}

/// The lit thing's sheet: on an iron plate, big in its niche, its words, set against what she
/// carries (or what he pays and the villa would), how many, and the counter's buttons.
fn card(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    f: &ShopFrame,
    index: Option<usize>,
    indices: &[usize],
) {
    let r = Layout::CARD;
    soft(
        v,
        Rect::new(r.x + 10.0, r.y + 14.0, r.w, r.h),
        14.0,
        22.0,
        rgba(SHADOW, 0.75),
        false,
    );
    panel(
        v,
        r,
        12.0,
        Panel::fill(0x251c15, 0x0f0a07, 0.97)
            .rim(COPPER, 1.0, 2.5)
            .hairline(9.0, 0.35)
            .bevel(0.8),
    );
    for (x, y) in [
        (r.x + 16.0, r.y + 16.0),
        (r.x + r.w - 16.0, r.y + 16.0),
        (r.x + 16.0, r.y + r.h - 16.0),
        (r.x + r.w - 16.0, r.y + r.h - 16.0),
    ] {
        disc(v, [x, y], 5.0, rgba(IRON_HI, 1.0));
        disc(v, [x - 1.2, y - 1.2], 1.8, rgba(0xffffff, 0.5));
    }
    let buying = f.tab == Tab::Buy;
    let total_ore: u32 = indices
        .iter()
        .map(|&i| {
            let p = &shop::catalog().products[i];
            shop::sell_price(p) * f.inventory.get(&p.id).copied().unwrap_or(0)
        })
        .sum();
    let Some(index) = index else {
        text_centered(
            font,
            v,
            Face::Italic,
            if buying {
                "Lo mejor de su vagoneta ya es tuyo."
            } else {
                "Su báscula espera mineral."
            },
            r.x + r.w * 0.5,
            r.y + r.h * 0.45,
            26.0,
            rgba(DUST, 0.9),
        );
        return;
    };
    let p = &shop::catalog().products[index];
    // the thing big in a stone niche, lit from above
    let n = Layout::NICHE;
    panel(
        v,
        n,
        14.0,
        Panel::fill(0x3a3029, 0x14100d, 1.0)
            .rim(0x000000, 1.0, 2.0)
            .bevel(0.9),
    );
    halo(v, [n.x + n.w * 0.5, n.y + n.h * 0.42], 110.0, 60.0, rgba(LAMP, 0.22));
    item_icon(icons, v, &p.id, grow(n, -22.0), 1.0);
    let x = n.x + n.w + 26.0;
    let w = r.x + r.w - x - 26.0;
    let name_size = fit(font, Face::Title, &p.name, 42.0, w);
    text_outline(
        font,
        v,
        Face::Title,
        &p.name,
        x,
        n.y + 2.0,
        name_size,
        rgba(PAPER, 1.0),
        rgba(EDGE, 0.85),
    );
    let kind = if PICKS.iter().any(|k| k.id == p.id) {
        "Pico"
    } else if BAGS.iter().any(|b| b.id == p.id) {
        "Para cargar"
    } else {
        "Del Pozo del Minero"
    };
    let chip = Rect::new(x, n.y + 58.0, text_width(font, Face::Ui, kind, 18.0) + 26.0, 30.0);
    panel(v, chip, 12.0, Panel::fill(EMBER, 0x7a300c, 1.0).rim(EDGE, 1.0, 1.5));
    text_shadow(font, v, Face::Ui, kind, chip.x + 13.0, chip.y + 4.0, 18.0, rgba(PAPER, 1.0));
    let words = if p.description.is_empty() {
        &p.summary
    } else {
        &p.description
    };
    for (i, line) in wrap(font, Face::Body, words, 21.0, w).iter().take(5).enumerate() {
        text(
            font,
            v,
            Face::Body,
            line,
            x,
            n.y + 102.0 + i as f32 * 27.0,
            21.0,
            rgba(CREAM, 0.95),
        );
    }
    // set against what she carries, or what he pays
    let top = n.y + n.h + 30.0;
    let label = if buying {
        "Frente a lo tuyo"
    } else {
        "En su báscula"
    };
    text_shadow(font, v, Face::Title, label, r.x + 30.0, top, 26.0, rgba(LAMP, 1.0));
    push_rect(
        v,
        Rect::new(r.x + 30.0, top + 36.0, r.w - 60.0, 2.0),
        rgba(COPPER, 0.6),
    );
    let gear = Gear::from_owned(|id| f.inventory.get(id).is_some_and(|n| *n > 0));
    let mut rows: Vec<(String, String, Option<(f32, f32)>)> = Vec::new();
    if let Some(new) = PICKS.iter().find(|k| k.id == p.id) {
        let now = &PICKS[gear.pick];
        let top_damage = PICKS.iter().map(|k| k.damage).fold(1.0, f32::max);
        let top_rate = PICKS.iter().map(|k| k.rate).fold(1.0, f32::max);
        rows.push((
            "Golpe".into(),
            format!("{} » {}", now.damage, new.damage),
            Some((now.damage / top_damage, new.damage / top_damage)),
        ));
        rows.push((
            "Ritmo".into(),
            format!("{:.1} » {:.1} por segundo", now.rate, new.rate),
            Some((now.rate / top_rate, new.rate / top_rate)),
        ));
        rows.push((
            "Golpes al granito".into(),
            format!("{} » {}", granite_blows(now.damage), granite_blows(new.damage)),
            None,
        ));
        rows.push((
            "Rompe hasta".into(),
            format!("{} » {}", breaks(now.tier).name(), breaks(new.tier).name()),
            None,
        ));
    } else if let Some(new) = BAGS.iter().find(|b| b.id == p.id) {
        let now = &BAGS[gear.bag];
        let top_bag = BAGS.iter().map(|b| b.capacity).max().unwrap_or(1) as f32;
        rows.push((
            "Le caben".into(),
            format!("{} » {} piezas", now.capacity, new.capacity),
            Some((now.capacity as f32 / top_bag, new.capacity as f32 / top_bag)),
        ));
        rows.push((
            "Viajes al torno".into(),
            "menos: más carga por bajada".into(),
            None,
        ));
    } else {
        let here = shop::sell_price(p);
        let carried = f.inventory.get(&p.id).copied().unwrap_or(0);
        rows.push(("Aquí te paga".into(), format!("{here} oro cada uno"), None));
        rows.push((
            "En la villa".into(),
            format!("{} oro (él se queda {})", p.sell_price, p.sell_price.saturating_sub(here)),
            None,
        ));
        rows.push((
            "Llevas".into(),
            format!("{carried} · valen {} oro", carried * here),
            None,
        ));
        rows.push(("Todo tu mineral".into(), format!("{total_ore} oro"), None));
    }
    for (i, (what, how, bars)) in rows.iter().enumerate() {
        let y = top + 52.0 + i as f32 * 46.0;
        text_shadow(font, v, Face::Body, what, r.x + 30.0, y, 22.0, rgba(DUST, 1.0));
        let bx = r.x + 250.0;
        if let Some((now, new)) = bars {
            let full = 170.0;
            push_rect(v, Rect::new(bx, y + 4.0, full, 18.0), rgba(0x000000, 0.5));
            push_rect(v, Rect::new(bx, y + 4.0, full * new.min(1.0), 18.0), rgba(LAMP, 0.9));
            push_rect(v, Rect::new(bx, y + 4.0, full * now.min(1.0), 18.0), rgba(IRON_HI, 0.85));
            text_shadow(font, v, Face::Ui, how, bx + full + 16.0, y, 22.0, rgba(PAPER, 1.0));
        } else {
            text_shadow(font, v, Face::Ui, how, bx, y, 22.0, rgba(PAPER, 1.0));
        }
    }
    // her thought, on a scrap of paper
    if let Some(thought) = p.queen_thought.as_deref() {
        let y = top + 52.0 + rows.len() as f32 * 46.0 + 6.0;
        let lines = wrap(font, Face::Body, thought, 19.0, r.w - 120.0);
        let h = 26.0 + lines.len().min(2) as f32 * 24.0;
        if y + h < Layout::MINUS.y - 8.0 {
            let note = Rect::new(r.x + 30.0, y, r.w - 60.0, h);
            panel(v, note, 6.0, Panel::fill(PAPER, 0xd8c39c, 0.96).rim(0x8a6a3a, 1.0, 1.5));
            text_shadow(font, v, Face::Title, "Reina", note.x + 14.0, note.y + 8.0, 18.0, rgba(0xc2386a, 1.0));
            for (i, line) in lines.iter().take(2).enumerate() {
                text(font, v, Face::Body, line, note.x + 84.0, note.y + 8.0 + i as f32 * 24.0, 19.0, rgba(0x3a2614, 1.0));
            }
        }
    }
    // how many (selling), what it comes to, and the buttons
    let unit = if buying {
        shop::buy_price(p)
    } else {
        shop::sell_price(p)
    };
    let quantity = if buying { 1 } else { f.quantity.max(1) };
    let total = unit.saturating_mul(quantity);
    if !buying {
        text_shadow(font, v, Face::Body, "Cantidad", r.x + 30.0, Layout::MINUS.y + 12.0, 22.0, rgba(DUST, 1.0));
        for (rect, label, hit) in [
            (Layout::MINUS, "−", ShopHit::QuantityMinus),
            (Layout::PLUS, "+", ShopHit::QuantityPlus),
        ] {
            plank(v, rect, 6.0, false, f.hover == Some(hit), 1.0);
            text_centered(font, v, Face::Title, label, mid(rect)[0], rect.y + 8.0, 28.0, rgba(PAPER, 1.0));
        }
        let q = Rect::new(1452.0, Layout::MINUS.y, 76.0, 50.0);
        panel(v, q, 6.0, Panel::fill(PAPER, 0xd8c39c, 1.0).rim(EDGE, 1.0, 2.0));
        text_centered(font, v, Face::Ui, &quantity.to_string(), mid(q)[0], q.y + 11.0, 26.0, rgba(0x2a1a0c, 1.0));
    }
    let can = buying.then_some(f.gold >= total).unwrap_or(true);
    let status = f.notice.map(str::to_owned).unwrap_or_else(|| {
        if buying && !can {
            format!("Te faltan {} de oro.", total - f.gold)
        } else if buying {
            "Se lo lleva puesto: lo usarás en la próxima bajada.".to_owned()
        } else {
            format!("En la villa te darían {} oro.", p.sell_price * quantity)
        }
    });
    text_shadow(
        font,
        v,
        Face::Italic,
        &status,
        r.x + 30.0,
        Layout::ACTION.y - 40.0,
        21.0,
        rgba(if can { DUST } else { 0xff9a7a }, 1.0),
    );
    let label = if buying {
        format!("Comprar · {total} oro")
    } else {
        format!("Vender · {total} oro")
    };
    button(
        font,
        v,
        Layout::ACTION,
        &label,
        true,
        f.hover == Some(ShopHit::Action),
        can,
        26.0,
        f.time,
    );
    if !buying {
        button(
            font,
            v,
            Layout::ALL,
            &format!("Vender todo · {total_ore} oro"),
            false,
            f.hover == Some(ShopHit::SellAll),
            total_ore > 0,
            25.0,
            f.time,
        );
    }
}

/// The keys, along the bottom, on copper caps.
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
            COPPER,
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
            rgba(DUST, 0.95),
        );
        x += cap_w + 10.0 + text_width(font, Face::Italic, what, 20.0) + 30.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn his_counter_answers_the_pointer() {
        let selling = Pick {
            count: 3,
            selling: true,
            ..Pick::default()
        };
        let c = mid(Layout::ALL);
        assert_eq!(Layout::hit(c[0], c[1], &selling), Some(ShopHit::SellAll));
        let buying = Pick {
            count: 3,
            ..Pick::default()
        };
        assert_eq!(Layout::hit(c[0], c[1], &buying), None);
        let row = mid(Layout::row(2));
        assert_eq!(Layout::hit(row[0], row[1], &buying), Some(ShopHit::Item(2)));
        let tab = mid(Layout::TABS[1]);
        assert_eq!(Layout::hit(tab[0], tab[1], &buying), Some(ShopHit::TabSell));
        let close = mid(Layout::CLOSE);
        assert_eq!(Layout::hit(close[0], close[1], &buying), Some(ShopHit::Close));
        // the shaft's rock: the worn pick breaks all down to basalt, the Heart's only the best
        assert_eq!(breaks(0), Rock::Basalto);
        assert_eq!(breaks(5), Rock::Corazon);
        assert!(granite_blows(1.0) > granite_blows(12.0));
    }
}
