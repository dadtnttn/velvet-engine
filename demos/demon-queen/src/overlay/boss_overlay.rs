//! El HUD del combate contra Varrak (del prototipo, `combate.html`): sus barras arriba (la chispa,
//! con las muescas de los tres recuerdos, y la guardia), la vida de la Reina abajo a la izquierda, la
//! mira, la palabra que dice sobre su cabeza, la frase de Chiquita abajo, el rojo en los bordes cuando
//! duele, el destello del sello y las tarjetas de antes y después.
use super::persona::{faded, speech_box};
use super::ui_paint::*;
use super::*;

/// A card over the fight: before (what to do), after (won or lost).
#[derive(Clone, Debug, Default, PartialEq, Hash)]
pub struct BossCard {
    pub kicker: String,
    pub title: String,
    pub text: String,
    /// The keys' list (the first card).
    pub keys: bool,
    /// What can be done now: (the key or button, what it does).
    pub actions: Vec<(String, String)>,
}

/// A quick event on show: its key, what to do, the line under, where on the screen, the closing
/// ring (how far it still has to close, 1 at the start, 0 on the mark; and its slack), how full
/// (held, mashed), how much of the window is left, which step of how many, whether it is a danger,
/// and how long it has been up.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BossQte {
    pub key: String,
    pub text: String,
    pub sub: String,
    pub at: [f32; 2],
    pub ring: Option<(f32, f32)>,
    pub fill: Option<f32>,
    pub left: Option<f32>,
    pub step: u8,
    pub steps: u8,
    pub danger: bool,
    pub age: f32,
}

/// What the HUD shows this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BossView {
    pub time: f32,
    /// The bars and the mark (not under the first card).
    pub bars: bool,
    /// 0..100.
    pub spark: f32,
    pub guard: f32,
    pub guard_label: &'static str,
    pub stunned: bool,
    /// The three memories' notches, and their names.
    pub reached: [bool; 3],
    pub notches: [&'static str; 3],
    /// 0..1.
    pub health: f32,
    /// The mark: 0 plain, 1 he is open to the spark, 2 it flows.
    pub crosshair: u8,
    /// A word over his head: its text, a growl (else a memory), where on the screen, how strong.
    pub float: Option<(String, bool, [f32; 2], f32)>,
    /// Chiquita's line as far as it has typed out.
    pub line: Option<String>,
    pub vignette: f32,
    pub flash: f32,
    pub card: Option<BossCard>,
    /// What to do now: heading, detail, urgency (0 usual, 1 an opening, 2 a danger), and since
    /// when it says so (s).
    pub objective: Option<(String, String, u8, f32)>,
    /// Held now: running, the seal.
    pub running: bool,
    pub channeling: bool,
    /// The quick event, the last verdict (good, how long ago) and the clock's pace (1 normal).
    pub qte: Option<BossQte>,
    pub stamp: Option<(bool, f32)>,
    pub slow: f32,
}

impl Hash for BossView {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let q = |v: f32| (v * 100.0).round() as i32;
        q(self.time).hash(h);
        self.bars.hash(h);
        (q(self.spark), q(self.guard), q(self.health)).hash(h);
        self.guard_label.hash(h);
        self.stunned.hash(h);
        self.reached.hash(h);
        self.crosshair.hash(h);
        self.float
            .as_ref()
            .map(|(t, g, p, a)| (t, g, p[0] as i32, p[1] as i32, q(*a)))
            .hash(h);
        self.line.hash(h);
        (q(self.vignette), q(self.flash)).hash(h);
        self.card.hash(h);
        self.objective
            .as_ref()
            .map(|(a, b, u, t)| (a, b, u, q(*t)))
            .hash(h);
        (self.running, self.channeling).hash(h);
        self.qte
            .as_ref()
            .map(|qte| {
                (
                    &qte.key,
                    &qte.text,
                    qte.at.map(|x| x as i32),
                    qte.ring.map(|(a, b)| (q(a), q(b))),
                    qte.fill.map(q),
                    qte.left.map(q),
                    qte.step,
                    q(qte.age),
                )
            })
            .hash(h);
        self.stamp.map(|(g, a)| (g, q(a))).hash(h);
        q(self.slow).hash(h);
    }
}

/// A bar: its frame, its fill (`k` of it, coloured from `from` to `to`).
fn bar(v: &mut Vec<MapVertex>, r: Rect, k: f32, from: u32, to: u32, rim: u32, glow: Option<u32>) {
    soft(v, r, 3.0, 8.0, rgba(SHADOW, 0.6), false);
    panel(
        v,
        grow(r, 2.0),
        4.0,
        Panel::fill(0x120a1a, 0x0a0612, 0.86).rim(rim, 1.0, 2.0),
    );
    let k = k.clamp(0.0, 1.0);
    if k > 0.002 {
        let fill = Rect::new(r.x, r.y, r.w * k, r.h);
        if let Some(g) = glow {
            soft(v, fill, 2.0, 9.0, rgba(g, 0.55), true);
        }
        // Left to right in a few bands (the shader shades top to bottom).
        let bands = 12;
        for i in 0..bands {
            let (a, b) = (i as f32 / bands as f32, (i + 1) as f32 / bands as f32);
            if a >= k {
                break;
            }
            let x0 = r.x + r.w * a;
            let x1 = r.x + r.w * b.min(k);
            let c = mix(from, to, (a + b) * 0.5);
            panel(
                v,
                Rect::new(x0, r.y, x1 - x0 + 0.5, r.h),
                1.0,
                Panel::fill(mix(c, 0xffffff, 0.12), c, 1.0),
            );
        }
    }
}

pub(super) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &BossView,
) {
    // Pain at the edges, the seal's flash over all.
    let low = if view.bars && view.health < 0.3 {
        0.35 + 0.15 * (view.time * 5.0).sin()
    } else {
        0.0
    };
    let red = view.vignette.max(low);
    if red > 0.01 {
        vignette(
            v,
            Rect::new(0.0, 0.0, width, height),
            0.0,
            height * 0.26,
            rgba(0xb0102a, 0.6 * red),
        );
    }
    if view.flash > 0.01 {
        halo(
            v,
            [width * 0.5, height * 0.5],
            height * 0.55,
            height * 0.45,
            rgba(0xffe8b0, 0.55 * view.flash),
        );
        halo(
            v,
            [width * 0.5, height * 0.5],
            height * 0.9,
            height * 0.6,
            rgba(0xb070ff, 0.30 * view.flash),
        );
    }
    if view.bars {
        draw_bars(font, v, width, height, view);
        // The mark: a small diamond; bigger and violet when he is open, gold when it flows.
        let c = [width * 0.5, height * 0.5];
        let (size, ink, glow) = match view.crosshair {
            1 => (11.0, 0xc890ff, 8.0),
            2 => (11.0, 0xffe0a0, 10.0),
            _ => (7.0, 0xf4e6c8, 0.0),
        };
        if glow > 0.0 {
            halo(v, c, size * 2.2, size * 1.6, rgba(ink, 0.45));
        }
        for (a, b) in [
            ([0.0, -1.0], [1.0, 0.0]),
            ([1.0, 0.0], [0.0, 1.0]),
            ([0.0, 1.0], [-1.0, 0.0]),
            ([-1.0, 0.0], [0.0, -1.0]),
        ] {
            let p = |d: [f32; 2]| [c[0] + d[0] * size, c[1] + d[1] * size];
            wire(
                v,
                p(a),
                p(b),
                2.0,
                rgba(ink, 0.9),
                rgba(ink, 0.9),
                0.0,
                false,
            );
        }
    }
    // The word he says.
    if let Some((text, growl, at, alpha)) = &view.float {
        let (size, ink, shadow) = if *growl {
            (26.0, 0xff9a7a, 0x5a0a08)
        } else {
            (44.0, 0xfff0c8, 0x4a1a7a)
        };
        let w = text_width(font, Face::Italic, text, size);
        // a soft glow behind the words, as the prototype's text-shadow
        soft(
            v,
            Rect::new(at[0] - w * 0.5, at[1] - size * 1.05, w, size * 1.1),
            size * 0.5,
            size * 0.5,
            rgba(if *growl { 0xff3a2a } else { 0xb070ff }, 0.22 * alpha),
            false,
        );
        text_outline(
            font,
            v,
            Face::Italic,
            text,
            at[0] - w * 0.5,
            at[1] - size * 1.1,
            size,
            rgba(ink, *alpha),
            rgba(shadow, 0.9 * alpha),
        );
    }
    // Chiquita's line: the game's comic speech card, her name on a pink tab. Its left edge and
    // its bottom stay put while the words type out, so it only grows to the right and upwards.
    if let Some(line) = &view.line {
        if !line.is_empty() {
            speech_box(
                font,
                v,
                [width * 0.5 - 400.0, height - 34.0],
                800.0,
                "Chiquita",
                line,
                0xff4fa8,
                -0.012,
                1.0,
                0.82,
            );
        }
    }
    if view.bars && view.card.is_none() {
        draw_objective(font, v, view);
        draw_keys(font, v, width, height, view);
    }
    draw_qte(font, v, width, height, view);
    if let Some(card) = &view.card {
        draw_card(font, v, width, height, card, view.time);
    }
}

/// A slanted plate (Persona's): its corners for a box `x, y, w, h` leaning `lean`.
fn slant(x: f32, y: f32, w: f32, h: f32, lean: f32) -> [[f32; 2]; 4] {
    [
        [x + lean, y],
        [x + w + lean, y],
        [x + w - lean, y + h],
        [x - lean, y + h],
    ]
}

fn fill4(v: &mut Vec<MapVertex>, q: [[f32; 2]; 4], color: [f32; 4]) {
    push_triangle(v, [q[0], q[1], q[2]], color);
    push_triangle(v, [q[0], q[2], q[3]], color);
    push_outline_fringe(v, &q, color);
}

/// What to do now, top left: a tag «OBJETIVO», the heading on a slanted dark plate with a wedge
/// of its colour (violet as usual, gold for an opening, red for a danger, beating), its detail
/// under it. A new one slides in.
fn draw_objective(font: &FontAtlas, v: &mut Vec<MapVertex>, view: &BossView) {
    let Some((head, detail, urgency, since)) = view.objective.as_ref() else {
        return;
    };
    let age = view.time - since;
    let k = (age / 0.3).clamp(0.0, 1.0);
    let come = 1.0 - (1.0 - k).powi(3);
    let accent = match urgency {
        1 => 0xf0c050,
        2 => 0xe2334a,
        _ => 0x9a5cff,
    };
    let beat = if *urgency > 0 {
        0.5 + 0.5 * (view.time * 7.0).sin()
    } else {
        0.0
    };
    let hs = 34.0;
    let hw = text_width(font, Face::Title, head, hs);
    let w = hw + 110.0;
    let (x, y) = (34.0 - (1.0 - come) * (w + 80.0), 96.0);
    faded(v, k.max(0.05), |v| {
        fill4(
            v,
            slant(x + 8.0, y + 8.0, w, 60.0, 12.0),
            rgba(0x000000, 0.5),
        );
        fill4(v, slant(x, y, w, 60.0, 12.0), rgba(0x120a1a, 0.94));
        fill4(v, slant(x, y, 44.0, 60.0, 12.0), rgba(accent, 1.0));
        if beat > 0.0 {
            soft(
                v,
                Rect::new(x, y, w, 60.0),
                6.0,
                16.0,
                rgba(accent, 0.35 * beat),
                true,
            );
        }
        // The tag.
        let tag = "OBJETIVO";
        let tw = text_width(font, Face::Display, tag, 15.0) + 24.0;
        fill4(
            v,
            slant(x + 60.0, y - 14.0, tw, 24.0, 6.0),
            rgba(accent, 1.0),
        );
        text(
            font,
            v,
            Face::Display,
            tag,
            x + 72.0,
            y - 12.0,
            15.0,
            rgba(0x0e0816, 1.0),
        );
        text_outline(
            font,
            v,
            Face::Title,
            head,
            x + 62.0,
            y + 12.0,
            hs,
            rgba(0xfff6ee, 1.0),
            rgba(0x000000, 0.9),
        );
        let lines = wrap(font, Face::Body, detail, 21.0, 560.0);
        let dw = lines
            .iter()
            .map(|l| text_width(font, Face::Body, l, 21.0))
            .fold(0.0, f32::max)
            + 44.0;
        let dh = 16.0 + lines.len() as f32 * 28.0;
        fill4(
            v,
            slant(x + 16.0, y + 68.0, dw, dh, 8.0),
            rgba(0x0a0612, 0.82),
        );
        for (i, l) in lines.iter().enumerate() {
            text_shadow(
                font,
                v,
                Face::Body,
                l,
                x + 34.0,
                y + 74.0 + i as f32 * 28.0,
                21.0,
                rgba(0xf6ecd8, 1.0),
            );
        }
    });
}

/// The controls, always there at the right: each key on a cap, lit while it is held.
fn draw_keys(font: &FontAtlas, v: &mut Vec<MapVertex>, width: f32, height: f32, view: &BossView) {
    let keys: [(&str, &str, bool); 6] = [
        ("Clic", "espada", false),
        ("Clic dcho.", "sello (mantener)", view.channeling),
        ("Mayús", "correr", view.running),
        ("Espacio", "saltar", false),
        ("Q", "curarse (mochila)", false),
        ("Esc", "pausa", false),
    ];
    let size = 16.0;
    let mut y = height - 52.0 - keys.len() as f32 * 40.0;
    // A slanted ink plate behind them, its violet edge on the left, so they read on any ground.
    let (px, pw, ph) = (width - 316.0, 300.0, keys.len() as f32 * 40.0 + 20.0);
    fill4(
        v,
        slant(px + 8.0, y - 6.0, pw, ph, 10.0),
        rgba(0x000000, 0.45),
    );
    fill4(v, slant(px, y - 14.0, pw, ph, 10.0), rgba(0x0e0816, 0.86));
    fill4(v, slant(px, y - 14.0, 8.0, ph, 10.0), rgba(0x9a5cff, 1.0));
    for (key, what, lit) in keys {
        let kw = text_width(font, Face::Ui, key, size) + 26.0;
        let ww = text_width(font, Face::Body, what, 19.0);
        let x = width - 44.0 - ww - 14.0 - kw;
        // The key on a slanted cap: ink with a pale rim, violet while it is held.
        fill4(
            v,
            slant(x + 3.0, y + 3.0, kw, 30.0, 5.0),
            rgba(0x000000, 0.6),
        );
        fill4(
            v,
            slant(x, y, kw, 30.0, 5.0),
            rgba(if lit { 0x9a5cff } else { 0x0e0816 }, 1.0),
        );
        let rim = slant(x, y, kw, 30.0, 5.0);
        for i in 0..4 {
            wire(
                v,
                rim[i],
                rim[(i + 1) % 4],
                1.4,
                rgba(0xf1e4cc, 0.6),
                rgba(0xf1e4cc, 0.6),
                0.0,
                false,
            );
        }
        text(
            font,
            v,
            Face::Ui,
            key,
            x + 13.0,
            y + 6.0,
            size,
            rgba(0xfff6ee, 1.0),
        );
        text_shadow(
            font,
            v,
            Face::Body,
            what,
            x + kw + 14.0,
            y + 4.0,
            19.0,
            rgba(if lit { 0xe6d0ff } else { 0xf1e4cc }, 0.95),
        );
        y += 40.0;
    }
}

fn draw_bars(font: &FontAtlas, v: &mut Vec<MapVertex>, width: f32, height: f32, view: &BossView) {
    // Varrak, on top.
    let w = 860.0f32.min(width * 0.62);
    let x = (width - w) * 0.5;
    let name = "Varrak";
    let small = "el Colmillo";
    let (ns, ss) = (32.0, 19.0);
    let nw = text_width(font, Face::Title, name, ns);
    let sw = text_width(font, Face::Italic, small, ss);
    let nx = width * 0.5 - (nw + 10.0 + sw) * 0.5;
    text_shadow(
        font,
        v,
        Face::Title,
        name,
        nx,
        16.0,
        ns,
        rgba(0xf4e6c8, 1.0),
    );
    text_shadow(
        font,
        v,
        Face::Italic,
        small,
        nx + nw + 10.0,
        28.0,
        ss,
        rgba(0xc9b08a, 1.0),
    );
    let spark = Rect::new(x, 64.0, w, 22.0);
    bar(
        v,
        spark,
        view.spark / 100.0,
        0x6a2acc,
        0xffd27a,
        0xd9b060,
        Some(0xa060ff),
    );
    text(
        font,
        v,
        Face::Ui,
        "CHISPA",
        spark.x + 10.0,
        spark.y + 3.0,
        14.0,
        rgba(0xffffff, 0.55),
    );
    for (i, k) in [0.25f32, 0.5, 0.75].into_iter().enumerate() {
        let nx = spark.x + spark.w * k;
        let lit = view.reached[i];
        let ink = if lit { 0xffe0a0 } else { 0xd9b060 };
        if lit {
            halo(
                v,
                [nx, spark.y + spark.h * 0.5],
                14.0,
                10.0,
                rgba(0xffd27a, 0.6),
            );
        }
        wire(
            v,
            [nx, spark.y - 6.0],
            [nx, spark.y + spark.h + 6.0],
            2.4,
            rgba(ink, 1.0),
            rgba(ink, 1.0),
            0.0,
            false,
        );
        if lit {
            let label = view.notches[i];
            let lw = text_width(font, Face::Italic, label, 16.0);
            text_outline(
                font,
                v,
                Face::Italic,
                label,
                nx - lw * 0.5,
                spark.y - 26.0,
                16.0,
                rgba(0xffe6b0, 1.0),
                rgba(0x2a0a40, 0.9),
            );
        }
    }
    let gw = w * 0.7;
    let guard = Rect::new((width - gw) * 0.5, spark.y + spark.h + 10.0, gw, 10.0);
    if view.stunned {
        soft(v, guard, 3.0, 12.0, rgba(0xb070ff, 0.55), true);
    }
    bar(
        v,
        guard,
        view.guard / 100.0,
        0x7a2a1a,
        0xd8642a,
        if view.stunned { 0xb070ff } else { 0xd9b060 },
        None,
    );
    let gl = view.guard_label.to_uppercase();
    let lw = text_width(font, Face::Ui, &gl, 12.0);
    text_outline(
        font,
        v,
        Face::Ui,
        &gl,
        width * 0.5 - lw * 0.5,
        guard.y + guard.h + 3.0,
        12.0,
        rgba(if view.stunned { 0xe8d0ff } else { 0xf4e6c8 }, 0.9),
        rgba(SHADOW, 0.8),
    );
    // The Queen, bottom left.
    let qw = 360.0;
    text_shadow(
        font,
        v,
        Face::Title,
        "La Reina",
        28.0,
        height - 72.0,
        20.0,
        rgba(0xf4e6c8, 1.0),
    );
    bar(
        v,
        Rect::new(28.0, height - 42.0, qw, 18.0),
        view.health,
        0x6a1a3a,
        0xd0406a,
        0xd9b060,
        None,
    );
}

fn draw_card(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    card: &BossCard,
    time: f32,
) {
    // The world behind, dimmed, and a band of violet across it.
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x07040c, 0.66));
    let (w, lean) = (1080.0, 30.0);
    let lead = wrap(font, Face::Body, &card.text, 22.0, w - 200.0);
    let keys_h = if card.keys { 4.0 * 52.0 + 16.0 } else { 0.0 };
    let h = 150.0 + lead.len() as f32 * 32.0 + 24.0 + keys_h + 80.0;
    let x = (width - w) * 0.5;
    let y = (height - h) * 0.5;
    fill4(
        v,
        slant(x - 400.0, y + h * 0.5 - 90.0, w + 800.0, 180.0, 40.0),
        rgba(0x3a1a6a, 0.45),
    );
    fill4(
        v,
        slant(x + 16.0, y + 16.0, w, h, lean),
        rgba(0x000000, 0.6),
    );
    fill4(v, slant(x, y, w, h, lean), rgba(0x0e0816, 0.97));
    fill4(v, slant(x, y, 22.0, h, lean), rgba(0x9a5cff, 1.0));
    let rim = slant(x, y, w, h, lean);
    for i in 0..4 {
        wire(
            v,
            rim[i],
            rim[(i + 1) % 4],
            2.0,
            rgba(0xd6b8ff, 0.8),
            rgba(0xd6b8ff, 0.8),
            0.0,
            false,
        );
    }
    // The kicker on its tag, the title big.
    let (tx, mut ty) = (x + 80.0, y + 34.0);
    let kw = text_width(font, Face::Display, &card.kicker.to_uppercase(), 17.0) + 30.0;
    fill4(v, slant(tx, ty, kw, 30.0, 8.0), rgba(0xd6b8ff, 1.0));
    text(
        font,
        v,
        Face::Display,
        &card.kicker.to_uppercase(),
        tx + 15.0,
        ty + 3.0,
        17.0,
        rgba(0x0e0816, 1.0),
    );
    ty += 44.0;
    text_outline(
        font,
        v,
        Face::Title,
        &card.title,
        tx,
        ty,
        58.0,
        rgba(0xfff6ee, 1.0),
        rgba(0x000000, 0.9),
    );
    ty += 82.0;
    for l in &lead {
        text_shadow(font, v, Face::Body, l, tx, ty, 22.0, rgba(0xf1e4cc, 1.0));
        ty += 32.0;
    }
    ty += 18.0;
    if card.keys {
        let keys = [
            ("WASD", "moverte"),
            ("Mayús", "correr y esquivar"),
            ("Espacio", "saltar la onda"),
            ("Clic", "espada: rompe su guardia"),
            ("Clic dcho.", "mantén: la chispa del sello"),
            ("Q", "curarte con la mochila"),
            ("Esc", "pausa · usar objetos"),
        ];
        for (i, (key, what)) in keys.iter().enumerate() {
            let col = (i % 2) as f32;
            let row = (i / 2) as f32;
            let kx = tx + col * 470.0;
            let ky = ty + row * 52.0;
            let kw = text_width(font, Face::Title, key, 18.0) + 28.0;
            keycap(
                font,
                v,
                Rect::new(kx, ky, kw, 38.0),
                key,
                18.0,
                false,
                false,
                0x9a5cff,
                1.0,
            );
            text_shadow(
                font,
                v,
                Face::Body,
                what,
                kx + kw + 14.0,
                ky + 7.0,
                20.0,
                rgba(0xf1e4cc, 1.0),
            );
        }
        ty += keys_h;
    }
    // What to do now, on bright plates.
    let mut ax = tx;
    for (key, what) in &card.actions {
        let kw = text_width(font, Face::Title, key, 18.0) + 28.0;
        let pulse = 0.5 + 0.5 * (time * 4.0).sin();
        keycap(
            font,
            v,
            Rect::new(ax, ty, kw, 40.0),
            key,
            18.0,
            true,
            false,
            0xffd27a,
            0.85 + 0.15 * pulse,
        );
        text_outline(
            font,
            v,
            Face::Title,
            what,
            ax + kw + 14.0,
            ty + 4.0,
            26.0,
            rgba(0xffe6b0, 1.0),
            rgba(0x000000, 0.8),
        );
        ax += kw + 14.0 + text_width(font, Face::Title, what, 26.0) + 50.0;
    }
}

/// The quick event: the world slowed (violet at the edges), a key badge pinned where it happens,
/// the ring closing on it, its fill, the time left, a plate with what to do; the verdict stamped.
fn draw_qte(font: &FontAtlas, v: &mut Vec<MapVertex>, width: f32, height: f32, view: &BossView) {
    if view.slow < 0.97 {
        let k = ((1.0 - view.slow) / 0.84).clamp(0.0, 1.0);
        vignette(
            v,
            Rect::new(0.0, 0.0, width, height),
            0.0,
            height * 0.3,
            rgba(0x5a1aa8, 0.55 * k),
        );
    }
    if let Some(q) = &view.qte {
        let accent = if q.danger { 0xe2334a } else { 0xf0c050 };
        // Pinned where it happens, but never off the screen nor over the bars.
        let c = [
            q.at[0].clamp(140.0, width - 520.0),
            q.at[1].clamp(190.0, height - 210.0),
        ];
        let pop = 1.0 + 0.35 * (1.0 - (q.age / 0.18).clamp(0.0, 1.0)).powi(2);
        let r = 46.0 * pop;
        let beat = 0.5 + 0.5 * (view.time * 10.0).sin();
        // The badge.
        halo(v, c, r * 1.9, r * 1.3, rgba(accent, 0.22 + 0.18 * beat));
        disc(v, c, r + 5.0, rgba(0x000000, 0.55));
        disc(v, c, r, rgba(0x120a1a, 0.95));
        arc(
            v,
            c,
            r - 3.0,
            3.0,
            0.0,
            1.0,
            rgba(accent, 1.0),
            rgba(accent, 0.0),
        );
        let ks = if q.key.chars().count() > 5 {
            17.0
        } else {
            24.0
        };
        let kw = text_width(font, Face::Display, &q.key, ks);
        text_outline(
            font,
            v,
            Face::Display,
            &q.key,
            c[0] - kw * 0.5,
            c[1] - ks * 0.62,
            ks,
            rgba(0xfff3d6, 1.0),
            rgba(0x000000, 0.8),
        );
        // The ring closing on its mark: gold inside the slack.
        if let Some((k, slack)) = q.ring {
            let on = k.abs() <= slack;
            let rr = r + 4.0 + k.max(0.0) * 120.0;
            let ink = if on { 0xfff0a0 } else { 0xf4e6c8 };
            arc(
                v,
                c,
                rr,
                if on { 6.0 } else { 3.0 },
                0.0,
                1.0,
                rgba(ink, if on { 1.0 } else { 0.8 }),
                rgba(ink, 0.0),
            );
            arc(
                v,
                c,
                r + 4.0,
                2.0,
                0.0,
                1.0,
                rgba(0xffffff, 0.35),
                rgba(0xffffff, 0.0),
            );
        }
        // How full (held, mashed).
        if let Some(fill) = q.fill {
            arc(
                v,
                c,
                r + 10.0,
                8.0,
                0.75,
                fill,
                rgba(mix(0x9a5cff, 0xffe08a, fill), 1.0),
                rgba(0x2a1d33, 0.85),
            );
        }
        // The time left.
        if let Some(left) = q.left {
            let ink = if q.danger { 0xe2334a } else { 0xb070ff };
            arc(
                v,
                c,
                r + 22.0,
                3.0,
                0.75,
                left,
                rgba(ink, 0.9),
                rgba(0x000000, 0.25),
            );
        }
        // The plate: what to do, the line under, the steps.
        let ts = 34.0;
        let tw = text_width(font, Face::Title, &q.text, ts);
        let sw = text_width(font, Face::Round, &q.sub, 17.0);
        let w = tw.max(sw) + 56.0;
        let (x, y) = (c[0] + r + 30.0, c[1] - 40.0);
        fill4(
            v,
            slant(x + 7.0, y + 7.0, w, 80.0, 12.0),
            rgba(0x000000, 0.5),
        );
        fill4(v, slant(x, y, w, 80.0, 12.0), rgba(0x120a1a, 0.94));
        fill4(v, slant(x, y, w, 5.0, 1.0), rgba(accent, 1.0));
        text_outline(
            font,
            v,
            Face::Title,
            &q.text,
            x + 26.0,
            y + 10.0,
            ts,
            rgba(0xfff3d6, 1.0),
            rgba(0x000000, 0.6),
        );
        text(
            font,
            v,
            Face::Round,
            &q.sub,
            x + 24.0,
            y + 50.0,
            17.0,
            rgba(0xd8c8f0, 0.92),
        );
        if q.steps > 1 {
            for i in 0..q.steps {
                let done = i < q.step;
                let now = i == q.step;
                let p = [x + w - 22.0 - (q.steps - 1 - i) as f32 * 18.0, y + 64.0];
                disc(
                    v,
                    p,
                    if now { 6.0 } else { 4.5 },
                    rgba(if done || now { accent } else { 0x4a3a5a }, 1.0),
                );
            }
        }
    }
    // The verdict.
    if let Some((good, age)) = view.stamp {
        if age < 0.75 {
            let k = (age / 0.75).clamp(0.0, 1.0);
            let alpha = 1.0 - k * k;
            let size = 72.0 * (1.0 + 0.4 * (1.0 - (age / 0.12).clamp(0.0, 1.0)));
            let (word, ink, rim) = if good {
                ("¡BIEN!", 0xffe08a, 0x6a3a00)
            } else {
                ("¡FALLO!", 0xff6a6a, 0x4a0010)
            };
            let w = text_width(font, Face::Display, word, size);
            let (x, y) = (width * 0.5 - w * 0.5, height * 0.3 - size * 0.6 - k * 24.0);
            fill4(
                v,
                slant(x - 30.0, y + size * 0.1, w + 60.0, size * 1.05, 18.0),
                rgba(0x120a1a, 0.75 * alpha),
            );
            text_outline(
                font,
                v,
                Face::Display,
                word,
                x,
                y,
                size,
                rgba(ink, alpha),
                rgba(rim, alpha),
            );
        }
    }
}
