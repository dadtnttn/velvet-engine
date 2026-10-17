//! «Otro trato» played as a page of the shop (`explore/tienda.rs`), not on the intimate scenes'
//! stage: the same emerald slashes with the favour's rose across them, the shopkeeper at the left
//! with his word in a comic box (grinning while she pays, swamped while she hurries him, shaking
//! his head if she backs out), the favour's loop where a haggle's deal card stands, the ticket of
//! what it buys at the right (the price as it stands, what the haggle took off, the favour's cut,
//! the deal filling as it goes and her price), and what she can do on tiles under the picture. At
//! the end the ticket is stamped and the seal's share shows on it. Authored at 1920×1080 as the
//! rest of the shop: drawing and picking share the same rectangles, and what the keys and the
//! pointer light only changes light and colour, never a place or a size.
use super::*;

/// A favour on show, as the shop sees it (`explore/play_scene.rs` makes it with the scene).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct FavorDeal {
    /// The product being haggled over (catalog index; `None`: no haggle under it) and how many.
    pub product: Option<usize>,
    pub quantity: u32,
    /// Its unit price before the haggle, now, and once the favour is paid; how far the haggle has
    /// moved it already (percent).
    pub base: u32,
    pub price: u32,
    pub after: u32,
    pub percent: u32,
    /// The favour: its name, its cut (percent; 100 = free), its step on his ladder and how many
    /// steps there are, and whether it is his new one today.
    pub favor: String,
    pub cut: u32,
    pub step: u8,
    pub steps: u8,
    pub new: bool,
    /// Her gold.
    pub gold: u32,
    /// His illustration: the face of the moment, the one before it and since when it changed
    /// (ms); since his last word (ms).
    pub face: Option<PortraitRef>,
    pub face_from: Option<PortraitRef>,
    pub face_ms: u32,
    pub said_ms: Option<u32>,
    /// Since the favour opened (ms): the page's entrances.
    pub open_ms: u32,
}

impl Layout {
    /// A favour's page: its loop, the ticket at the right.
    pub const FAVOR_PICTURE: Rect = Rect::new(616.0, 132.0, 736.0, 736.0);
    /// High enough that Chiquita's balloons, over her corner, keep off it.
    pub const FAVOR_TICKET: Rect = Rect::new(1392.0, 156.0, 488.0, 400.0);
    /// «Continuar», once the card is up.
    pub const FAVOR_GO: Rect = Rect::new(616.0, 902.0, 736.0, 82.0);

    /// Her two moves under the picture: hurry him (0) and back out (1).
    pub fn favor_action(i: usize) -> Rect {
        if i == 0 {
            Rect::new(616.0, 902.0, 370.0, 82.0)
        } else {
            Rect::new(1004.0, 902.0, 348.0, 82.0)
        }
    }
}

/// What lies under the pointer on a favour's page: 0 hurry him, 1 back out, 2 carry on (with the
/// closing card up).
pub(crate) fn favor_hit(x: f32, y: f32, over: bool) -> Option<u8> {
    if over {
        return inside_turned(Layout::FAVOR_GO, TILT, x, y).then_some(2);
    }
    (0..2)
        .find(|&i| inside_turned(Layout::favor_action(i), TILT, x, y))
        .map(|i| i as u8)
}

fn hex(c: [u8; 3]) -> u32 {
    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}

/// The favour's cut as the ticket says it.
fn cut_words(cut: u32) -> String {
    if cut >= 100 {
        "gratis".to_owned()
    } else {
        format!("−{cut} %")
    }
}

/// The whole page; a shake (a slap, a jolt) moves all of it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_favor(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    view: &SceneView,
    deal: &FavorDeal,
    face: Option<Uv>,
    face_from: Option<Uv>,
    has_frame: bool,
    hover: Option<u8>,
    time: f32,
) {
    let shake = view.game.as_ref().map_or(0.0, |g| g.shake as f32 / 10.0);
    let by = if shake > 0.05 {
        [
            (time * 71.0).sin() * shake,
            (time * 53.0 + 1.3).cos() * shake * 0.6,
        ]
    } else {
        [0.0, 0.0]
    };
    shifted(v, by, |v| {
        page(
            font, icons, v, view, deal, face, face_from, has_frame, hover, time,
        )
    });
}

#[allow(clippy::too_many_arguments)]
fn page(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    view: &SceneView,
    deal: &FavorDeal,
    face: Option<Uv>,
    face_from: Option<Uv>,
    has_frame: bool,
    hover: Option<u8>,
    time: f32,
) {
    let age = deal.open_ms as f32 / 1000.0;
    let over = view.result.as_ref();
    let flash = view.flash as f32 / 1000.0;
    // The shop never left: its slashes stay put, and the favour's rose sweeps across them.
    backdrop(v, 60.0, time);
    rose_slashes(v, age);
    if flash > 0.0 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, 1920.0, 1080.0),
            rgba(0xffd27a, 0.1 * flash),
        );
    }
    figure(
        font,
        v,
        Moves {
            said_ms: deal.said_ms,
            face_ms: deal.face_ms,
            result: None,
        },
        face,
        face_from,
        60.0,
        time,
        "Otro trato · se aceptan favores",
    );
    let (title, title_age) = match over {
        Some(r) if r.complete => ("¡Trato hecho!", r.age_ms as f32 / 1000.0),
        Some(r) => ("Te echas atrás", r.age_ms as f32 / 1000.0),
        None => ("¡Otro trato!", age),
    };
    heading(font, v, 620.0, 28.0, title, title_age);
    purse(font, v, Rect::new(1630.0, 40.0, 250.0, 60.0), deal.gold);
    // The favour's loop, where a haggle has its deal.
    let pic = Layout::FAVOR_PICTURE;
    let a = entrance(age, 0.04, 0.32);
    faded(v, a, |v| {
        shifted(v, [0.0, (1.0 - a) * -40.0], |v| {
            loop_frame(font, v, view, pic, has_frame, flash, time);
        });
    });
    // The ticket.
    let r = Layout::FAVOR_TICKET;
    let a = entrance(age, 0.12, 0.32);
    faded(v, a, |v| {
        shifted(v, [(1.0 - a) * 260.0, 0.0], |v| {
            turned(v, mid(r), -0.012, |v| {
                ticket(font, icons, v, r, view, deal, time);
            });
        });
    });
    // At the end the ticket is stamped beside her price (the keys are on the tiles).
    if let Some(res) = over {
        let t = res.age_ms as f32 / 1000.0 - 0.35;
        let (words, ink) = if res.complete {
            ("REBAJADO", QUEEN)
        } else {
            ("SIN TRATO", 0xa8b4c0)
        };
        rubber_stamp(
            font,
            v,
            [r.x + r.w - 112.0, r.y + r.h - 40.0],
            words,
            ink,
            t,
        );
    }
    actions(font, v, view, hover, age, time);
    words(font, v, view);
}

/// The favour's rose across the shop's slashes: a band under the gold line and a thread under the
/// black edge, sweeping in as the favour opens.
fn rose_slashes(v: &mut Vec<MapVertex>, age: f32) {
    let a = entrance(age, 0.0, 0.42);
    let angle = -0.22;
    let c = [1060.0, 560.0];
    shifted(v, [(1.0 - a) * 2400.0, 0.0], |v| {
        slash(
            v,
            [c[0], c[1] - 292.0],
            3600.0,
            16.0,
            angle,
            rgba(mix(QUEEN, 0xffffff, 0.1), 0.78),
            rgba(mix(QUEEN, 0x000000, 0.35), 0.78),
        );
        slash(
            v,
            [c[0], c[1] + 348.0],
            3600.0,
            7.0,
            angle,
            rgba(QUEEN, 0.7),
            rgba(QUEEN, 0.7),
        );
    });
}

/// The loop in the page's frame: a dark card leaning behind it with the favour's rose on its top,
/// the room's light round it (warmer as the favours climb, bright at the seal), the picture, its
/// caption and the stamp of the deal.
#[allow(clippy::too_many_arguments)]
fn loop_frame(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &SceneView,
    pic: Rect,
    has_frame: bool,
    flash: f32,
    time: f32,
) {
    let game = view.game.as_ref();
    let tint = game.map_or(GOLD, |g| hex(g.tint));
    let back = grow(pic, 20.0);
    turned(v, mid(back), -0.035, |v| {
        soft(
            v,
            Rect::new(back.x + 14.0, back.y + 18.0, back.w, back.h),
            16.0,
            24.0,
            rgba(SHADOW, 0.85),
            false,
        );
        panel(v, grow(back, 5.0), 18.0, Panel::fill(EDGE, EDGE, 1.0));
        panel(
            v,
            back,
            14.0,
            Panel::fill(0x0e221b, 0x040b09, 0.98)
                .rim(GOLD, 1.0, 2.2)
                .hairline(7.0, 0.28)
                .bevel(0.8),
        );
        panel(
            v,
            Rect::new(back.x, back.y, back.w, 10.0),
            5.0,
            Panel::fill(mix(QUEEN, 0xffffff, 0.2), QUEEN, 1.0),
        );
    });
    let pulse = game.map_or(0.0, |g| g.pulse as f32 / 1000.0);
    soft(
        v,
        grow(pic, 4.0),
        10.0,
        34.0,
        rgba(
            tint,
            0.16 + 0.06 * wave(time, 0.3) + 0.25 * pulse + 0.4 * flash,
        ),
        true,
    );
    panel(v, grow(pic, 3.0), 10.0, Panel::fill(EDGE, EDGE, 1.0));
    if has_frame && !view.black {
        picture(v, pic, 8.0);
    } else {
        panel(v, pic, 8.0, Panel::fill(0x0a1612, 0x020604, 1.0));
    }
    panel(v, pic, 8.0, Panel::fill(0, 0, 0.0).rim(GOLD_HI, 0.85, 1.6));
    if let Some(caption) = &view.caption {
        let size = 24.0;
        let lines = wrap(font, Face::Italic, caption, size, pic.w - 96.0);
        let h = lines.len() as f32 * size * 1.3 + 24.0;
        let r = Rect::new(pic.x + 28.0, pic.y + pic.h - h - 26.0, pic.w - 56.0, h);
        panel(
            v,
            r,
            8.0,
            Panel::fill(0x04100c, 0x020604, 0.84).rim(GOLD_DEEP, 1.0, 1.4),
        );
        for (i, line) in lines.iter().enumerate() {
            let w = text_width(font, Face::Italic, line, size);
            text_shadow(
                font,
                v,
                Face::Italic,
                line,
                r.x + (r.w - w) * 0.5,
                r.y + 12.0 + i as f32 * size * 1.3,
                size,
                rgba(CREAM, 1.0),
            );
        }
    }
    // The deal's shout across the picture's foot, as a haggle stamps its tries.
    if let Some((words, color, ms)) = game.and_then(|g| g.stamp.as_ref()) {
        let t = *ms as f32 / 1000.0;
        if t < 1.6 {
            let pop = entrance(t, 0.0, 0.16);
            let alpha = 1.0 - ((t - 1.2) / 0.4).clamp(0.0, 1.0);
            let size = 100.0 * (0.6 + 0.4 * pop);
            let tw = text_width(font, Face::Title, words, size);
            let c = [pic.x + pic.w * 0.5, pic.y + pic.h * 0.8];
            faded(v, alpha, |v| {
                turned(v, c, -0.12, |v| {
                    burst(
                        v,
                        c,
                        tw * 0.36,
                        tw * 0.6 * pop,
                        16,
                        time * 0.6,
                        rgba(hex(*color), 0.92),
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
                        0x8a5a10,
                        1.0,
                    );
                });
            });
        }
    }
}

/// A dashed rule across the ticket, as a till's paper is torn.
fn dashes(v: &mut Vec<MapVertex>, x0: f32, x1: f32, y: f32) {
    let mut x = x0;
    while x < x1 {
        let end = (x + 12.0).min(x1);
        wire(
            v,
            [x, y],
            [end, y],
            2.0,
            rgba(GOLD_DEEP, 0.8),
            rgba(GOLD_DEEP, 0.8),
            0.0,
            false,
        );
        x += 20.0;
    }
}

/// A line of the ticket: what it is at the left, how much at the right; struck through when it no
/// longer counts.
#[allow(clippy::too_many_arguments)]
fn line(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    x: f32,
    y: f32,
    label: &str,
    label_ink: u32,
    value: &str,
    ink: u32,
    struck: bool,
) {
    let size = fit(font, Face::Italic, label, 21.0, r.x + r.w - 150.0 - x);
    text_shadow(
        font,
        v,
        Face::Italic,
        label,
        x,
        y + 3.0,
        size,
        rgba(label_ink, 0.95),
    );
    let w = text_width(font, Face::Title, value, 25.0);
    let vx = r.x + r.w - 26.0 - w;
    text_outline(
        font,
        v,
        Face::Title,
        value,
        vx,
        y,
        25.0,
        rgba(ink, 1.0),
        rgba(EDGE, 0.85),
    );
    if struck {
        wire(
            v,
            [vx - 6.0, y + 17.0],
            [vx + w + 6.0, y + 13.0],
            3.0,
            rgba(CRIMSON, 1.0),
            rgba(CRIMSON, 1.0),
            0.0,
            false,
        );
    }
}

/// The ticket of the deal: what she is buying, the price as it stands and what comes off it, the
/// deal filling as he goes (at the end, what it fed the seal), and her price.
#[allow(clippy::too_many_arguments)]
fn ticket(
    font: &FontAtlas,
    icons: &IconAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    view: &SceneView,
    deal: &FavorDeal,
    time: f32,
) {
    let heat = view.heat.unwrap_or(0) as f32 / 1000.0;
    let over = view.result.as_ref();
    let backed = over.is_some_and(|res| !res.complete);
    let sealed = !backed && (heat >= 0.999 || over.is_some());
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
        Panel::fill(mix(QUEEN, 0xffffff, 0.2), QUEEN, 1.0),
    );
    // What she is buying, and which favour this is on his ladder.
    let product = deal.product.and_then(|i| shop::catalog().products.get(i));
    let c = [r.x + 64.0, r.y + 66.0];
    disc(v, c, 44.0, rgba(0x02100b, 1.0));
    halo(v, c, 44.0, 14.0, rgba(EMERALD, 0.3));
    let name = match product {
        Some(p) => {
            item_icon(
                icons,
                v,
                &p.id,
                Rect::new(c[0] - 38.0, c[1] - 38.0, 76.0, 76.0),
                1.0,
            );
            format!("{} × {}", p.name, deal.quantity)
        }
        None => {
            heart(v, c, 22.0, QUEEN, 1.0);
            "Un favor".to_owned()
        }
    };
    let size = fit(font, Face::Title, &name, 30.0, r.w - 156.0);
    text_outline(
        font,
        v,
        Face::Title,
        &name,
        r.x + 126.0,
        r.y + 26.0,
        size,
        rgba(INK, 1.0),
        rgba(EDGE, 0.85),
    );
    text_shadow(
        font,
        v,
        Face::Italic,
        &format!("Favor {} de {}", deal.step, deal.steps),
        r.x + 128.0,
        r.y + 70.0,
        19.0,
        rgba(CREAM, 0.8),
    );
    if deal.new {
        let b = Rect::new(r.x + r.w - 122.0, r.y + 66.0, 100.0, 28.0);
        panel(
            v,
            b,
            6.0,
            Panel::fill(GOLD_HI, GOLD, 1.0).rim(EDGE, 1.0, 1.6),
        );
        let w = text_width(font, Face::Title, "NUEVO", 19.0);
        text_shadow(
            font,
            v,
            Face::Title,
            "NUEVO",
            b.x + (b.w - w) * 0.5,
            b.y + 3.0,
            19.0,
            rgba(EDGE, 1.0),
        );
    }
    dashes(v, r.x + 22.0, r.x + r.w - 22.0, r.y + 118.0);
    // The price as it stands, what the haggle took off, and the favour's cut.
    let x = r.x + 26.0;
    let total = deal.base * deal.quantity;
    line(
        font,
        v,
        r,
        x,
        r.y + 130.0,
        "Precio de la tienda",
        CREAM,
        &format!("{total} oro"),
        0xb8c8c0,
        deal.percent > 0 || sealed,
    );
    let (haggled, haggled_ink) = if deal.percent > 0 {
        (format!("−{} %", deal.percent), MINT)
    } else {
        ("—".to_owned(), 0x6a7a72)
    };
    line(
        font,
        v,
        r,
        x,
        r.y + 164.0,
        "Regateado",
        CREAM,
        &haggled,
        haggled_ink,
        false,
    );
    heart(
        v,
        [x + 10.0, r.y + 212.0],
        10.0,
        if backed { 0x6a5a62 } else { QUEEN },
        1.0,
    );
    line(
        font,
        v,
        r,
        x + 30.0,
        r.y + 198.0,
        &deal.favor,
        if backed { 0x8a8a8a } else { 0xffd6e6 },
        &cut_words(deal.cut),
        if backed { 0x8a8a8a } else { 0xffc4dc },
        backed,
    );
    dashes(v, r.x + 22.0, r.x + r.w - 22.0, r.y + 240.0);
    // The deal filling as he goes; at the end, the seal's share instead.
    let bar = Rect::new(r.x + 26.0, r.y + 282.0, r.w - 52.0, 24.0);
    match over {
        Some(res) => effects(font, v, r, res),
        None => {
            let (label, ink) = if sealed {
                ("¡Trato hecho!", GOLD_HI)
            } else if view.holding {
                ("Le das prisa…", MINT)
            } else {
                ("Él cobra su parte del trato", CREAM)
            };
            text_shadow(
                font,
                v,
                Face::Italic,
                label,
                r.x + 26.0,
                r.y + 252.0,
                20.0,
                rgba(ink, 0.95),
            );
            let pct = format!("{} %", (heat * 100.0).round() as u32);
            let w = text_width(font, Face::Title, &pct, 22.0);
            text_shadow(
                font,
                v,
                Face::Title,
                &pct,
                r.x + r.w - 26.0 - w,
                r.y + 250.0,
                22.0,
                rgba(GOLD_HI, 1.0),
            );
            deal_bar(v, bar, heat, view.holding, sealed, time);
        }
    }
    // Her price: as it stands, and what it comes to with the favour (the favour's price once it
    // is sealed; as it was if she backed out).
    text_shadow(
        font,
        v,
        Face::Italic,
        "Tu precio",
        r.x + 26.0,
        r.y + 318.0,
        20.0,
        rgba(CREAM, 0.85),
    );
    let now_total = deal.price * deal.quantity;
    let after_total = deal.after * deal.quantity;
    let shown = if sealed { after_total } else { now_total };
    let big = if shown == 0 {
        "Gratis".to_owned()
    } else {
        shown.to_string()
    };
    coin(v, [r.x + 46.0, r.y + 368.0], 19.0, 1.0);
    // The new price lands with a little swell.
    let land = over.map_or(1.0, |res| entrance(res.age_ms as f32 / 1000.0, 0.0, 0.3));
    let pop = if sealed {
        1.0 + 0.2 * (1.0 - land)
    } else {
        1.0
    };
    let size = 56.0 * pop;
    text_outline(
        font,
        v,
        Face::Title,
        &big,
        r.x + 76.0,
        r.y + 368.0 - size * 0.56,
        size,
        rgba(GOLD_HI, 1.0),
        rgba(EDGE, 0.9),
    );
    let bw = text_width(font, Face::Title, &big, size);
    if !sealed && !backed && after_total != now_total {
        // What it comes to with the favour.
        let ax = r.x + 76.0 + bw + 18.0;
        arrow_right(v, [ax + 14.0, r.y + 366.0], rgba(0xffc4dc, 1.0));
        let after = if after_total == 0 {
            "gratis".to_owned()
        } else {
            after_total.to_string()
        };
        text_outline(
            font,
            v,
            Face::Title,
            &after,
            ax + 38.0,
            r.y + 346.0,
            38.0,
            rgba(0xffc4dc, 1.0),
            rgba(EDGE, 0.9),
        );
    }
    // Whether her gold reaches (on the closing card the stamp takes its place).
    let note = if backed || over.is_some() {
        None
    } else if deal.gold < after_total {
        Some((
            format!("Aún te faltarán {} de oro", after_total - deal.gold),
            0xff9aa8,
        ))
    } else if deal.gold < now_total {
        Some(("Con el trato te llega el oro".to_owned(), MINT))
    } else {
        None
    };
    if let Some((note, ink)) = note {
        let size = fit(font, Face::Italic, &note, 18.0, 230.0);
        let w = text_width(font, Face::Italic, &note, size);
        text_shadow(
            font,
            v,
            Face::Italic,
            &note,
            r.x + r.w - 24.0 - w,
            r.y + 318.0,
            size,
            rgba(ink, 1.0),
        );
    }
}

/// A small arrow pointing right (her price → what it comes to).
fn arrow_right(v: &mut Vec<MapVertex>, c: [f32; 2], color: [f32; 4]) {
    wire(
        v,
        [c[0] - 14.0, c[1]],
        [c[0] + 4.0, c[1]],
        5.0,
        color,
        color,
        0.0,
        false,
    );
    push_triangle(
        v,
        [
            [c[0] + 14.0, c[1]],
            [c[0], c[1] - 10.0],
            [c[0], c[1] + 10.0],
        ],
        color,
    );
}

/// The deal's bar, as a haggle's: a dark track with its ticks, filling emerald to gold as he
/// goes, a coin riding its head; brighter while she hurries him, all gold once it is sealed.
fn deal_bar(v: &mut Vec<MapVertex>, bar: Rect, heat: f32, hurry: bool, sealed: bool, time: f32) {
    panel(v, grow(bar, 3.0), 12.0, Panel::fill(EDGE, EDGE, 1.0));
    panel(
        v,
        bar,
        10.0,
        Panel::fill(0x07120e, 0x020604, 1.0).rim(GOLD_DEEP, 1.0, 1.8),
    );
    let k = heat.clamp(0.0, 1.0);
    if k > 0.01 {
        let fill = Rect::new(bar.x + 3.0, bar.y + 3.0, (bar.w - 6.0) * k, bar.h - 6.0);
        let tip = mix(EMERALD, GOLD_HI, k);
        if hurry || sealed {
            soft(
                v,
                fill,
                8.0,
                12.0,
                rgba(
                    if sealed { GOLD_HI } else { MINT },
                    0.4 + 0.15 * wave(time, 1.4),
                ),
                true,
            );
        }
        let mut style = Panel::fill(mix(tip, 0xffffff, 0.2), mix(tip, 0x000000, 0.3), 1.0);
        if hurry || sealed {
            style = style.sheen(time * 0.8);
        }
        panel(v, fill, 8.0, style);
    }
    for t in 1..10 {
        let x = bar.x + bar.w * t as f32 / 10.0;
        wire(
            v,
            [x, bar.y + bar.h - 9.0],
            [x, bar.y + bar.h - 4.0],
            2.0,
            rgba(0xffffff, 0.3),
            rgba(0xffffff, 0.3),
            0.0,
            false,
        );
    }
    coin(
        v,
        [bar.x + 3.0 + (bar.w - 6.0) * k, bar.y + bar.h * 0.5],
        15.0,
        1.0,
    );
}

/// The seal's share, on the ticket where the bar was: a chip for each thing that changed.
fn effects(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, res: &SceneResult) {
    if res.effects.is_empty() {
        text_shadow(
            font,
            v,
            Face::Italic,
            "El precio sigue como estaba.",
            r.x + 26.0,
            r.y + 262.0,
            21.0,
            rgba(CREAM, 0.9),
        );
        return;
    }
    let age = res.age_ms as f32 / 1000.0;
    let mut x = r.x + 26.0;
    for (i, (name, by)) in res.effects.iter().enumerate() {
        let a = entrance(age, 0.45 + 0.12 * i as f32, 0.25);
        let up = *by > 0;
        let tone = if up { QUEEN } else { 0x6ab0ff };
        let value = format!("{}{}", if up { "+" } else { "−" }, by.abs());
        let w = text_width(font, Face::Ui, name, 19.0)
            + text_width(font, Face::Title, &value, 22.0)
            + 70.0;
        let chip = Rect::new(x, r.y + 254.0, w, 40.0);
        faded(v, a, |v| {
            panel(
                v,
                chip,
                20.0,
                Panel::fill(mix(0x0e221b, tone, 0.25), 0x040b09, 1.0).rim(tone, 0.9, 1.6),
            );
            arrow(v, [chip.x + 22.0, chip.y + 20.0], 9.0, up, rgba(tone, 1.0));
            text_shadow(
                font,
                v,
                Face::Ui,
                name,
                chip.x + 40.0,
                chip.y + 9.0,
                19.0,
                rgba(INK, 1.0),
            );
            text_outline(
                font,
                v,
                Face::Title,
                &value,
                chip.x + chip.w - 18.0 - text_width(font, Face::Title, &value, 22.0),
                chip.y + 7.0,
                22.0,
                rgba(mix(tone, 0xffffff, 0.35), 1.0),
                rgba(EDGE, 0.8),
            );
        });
        x += w + 12.0;
    }
}

/// A rubber stamp in ink across the ticket: a box and its words, turned, landing with a thump
/// `age` seconds after it was pressed.
fn rubber_stamp(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    words: &str,
    ink: u32,
    age: f32,
) {
    let land = entrance(age, 0.0, 0.14);
    if land <= 0.0 {
        return;
    }
    let k = 1.0 + 0.6 * (1.0 - land);
    let size = 32.0 * k;
    let tw = text_width(font, Face::Title, words, size);
    let r = Rect::new(
        c[0] - tw * 0.5 - 24.0 * k,
        c[1] - size * 0.5 - 15.0 * k,
        tw + 48.0 * k,
        size + 30.0 * k,
    );
    faded(v, land, |v| {
        turned(v, c, -0.14, |v| {
            panel(
                v,
                r,
                10.0 * k,
                Panel::fill(ink, ink, 0.12).rim(ink, 0.95, 5.0 * k),
            );
            panel(
                v,
                grow(r, -9.0 * k),
                6.0 * k,
                Panel::fill(0, 0, 0.0).rim(ink, 0.65, 1.8 * k),
            );
            text(
                font,
                v,
                Face::Title,
                words,
                c[0] - tw * 0.5,
                c[1] - size * 0.56,
                size,
                rgba(ink, 0.95),
            );
        });
    });
}

/// What she can do, on tiles under the picture: hurry him (lit while she does) and back out;
/// once the deal is sealed the second only skips to the ticket; on the closing card, one tile to
/// carry on, lit once it can be taken.
fn actions(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &SceneView,
    hover: Option<u8>,
    age: f32,
    time: f32,
) {
    if let Some(res) = view.result.as_ref() {
        let ready = res.age_ms > 1000;
        let r = Layout::FAVOR_GO;
        let a = entrance(res.age_ms as f32 / 1000.0, 0.3, 0.3);
        let hint = if res.complete {
            "De vuelta al regateo, con la rebaja"
        } else {
            "De vuelta al regateo, con el precio de antes"
        };
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 220.0, 0.0], |v| {
                turned(v, mid(r), TILT, |v| {
                    action_tile(
                        font,
                        v,
                        r,
                        "F",
                        "Continuar",
                        hint,
                        GOLD,
                        ready,
                        hover == Some(2),
                        !ready,
                        time,
                    );
                });
            });
        });
        if ready && a > 0.95 {
            tile_cursor(v, r, TILT, time);
        }
        return;
    }
    let building = view.keys;
    let tiles = [
        (
            "Espacio",
            "Darle prisa",
            if building {
                "Mantén: el trato se cierra antes"
            } else {
                "El trato ya está hecho"
            },
            EMERALD,
            building && view.holding,
            !building,
        ),
        (
            "Esc",
            if building { "Echarse atrás" } else { "Saltar" },
            if building {
                "El precio sigue como estaba"
            } else {
                "Al tique del trato"
            },
            0x6a8a80,
            false,
            false,
        ),
    ];
    for (i, (key, label, hint, glow, lit, locked)) in tiles.into_iter().enumerate() {
        let r = Layout::favor_action(i);
        let a = entrance(age, 0.18 + 0.06 * i as f32, 0.3);
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 240.0, 0.0], |v| {
                turned(v, mid(r), TILT, |v| {
                    action_tile(
                        font,
                        v,
                        r,
                        key,
                        label,
                        hint,
                        glow,
                        lit,
                        hover == Some(i as u8),
                        locked,
                        time,
                    );
                });
            });
        });
    }
}

/// One of her moves, as a haggle's tactic tile with its key on a cap instead of a number.
#[allow(clippy::too_many_arguments)]
fn action_tile(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    key: &str,
    label: &str,
    hint: &str,
    glow: u32,
    lit: bool,
    over: bool,
    locked: bool,
    time: f32,
) {
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
            rgba(glow, 0.45 + 0.12 * wave(time, 0.8)),
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
    let cap_w = (text_width(font, Face::Title, key, 18.0) + 26.0).max(46.0);
    keycap(
        font,
        v,
        Rect::new(r.x + 24.0, r.y + (r.h - 42.0) * 0.5, cap_w, 42.0),
        key,
        18.0,
        lit,
        locked,
        glow,
        1.0,
    );
    let x = r.x + 24.0 + cap_w + 18.0;
    let ink = if locked { 0x8a9a92 } else { INK };
    text_outline(
        font,
        v,
        Face::Title,
        label,
        x,
        r.y + 10.0,
        30.0,
        rgba(ink, 1.0),
        rgba(EDGE, 0.85),
    );
    let size = fit(font, Face::Italic, hint, 19.0, r.x + r.w - 18.0 - x);
    text_shadow(
        font,
        v,
        Face::Italic,
        hint,
        x + 2.0,
        r.y + 48.0,
        size,
        rgba(if locked { 0x7a8a82 } else { CREAM }, 0.95),
    );
}

/// What is said, in the shop's comic boxes over the shopkeeper: the newest line at the foot, the
/// one before it above, fainter; on the closing card, his last word.
fn words(font: &FontAtlas, v: &mut Vec<MapVertex>, view: &SceneView) {
    let shown: Vec<&SceneLine> = match view.result.as_ref() {
        Some(res) => vec![&res.quote],
        None => view.lines.iter().rev().take(2).collect(),
    };
    let mut foot = 414.0;
    for (k, l) in shown.into_iter().enumerate() {
        if l.text.is_empty() {
            continue;
        }
        let accent = match l.name.as_str() {
            "Tendero" => EMERALD,
            "Reina" => QUEEN,
            _ => hex(l.accent),
        };
        let (x, s, fade) = if k == 0 {
            (44.0, 1.0, 1.0)
        } else {
            (64.0, 0.84, 0.72)
        };
        let alpha = l.alpha as f32 / 255.0 * fade;
        let b = speech_box(
            font,
            v,
            [x, foot],
            560.0,
            &l.name,
            &l.text,
            accent,
            -0.02,
            alpha,
            s,
        );
        foot = b.y - 40.0 * s;
    }
}
