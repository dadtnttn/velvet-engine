//! El HUD del combate contra Varrak (del prototipo, `combate.html`): sus barras arriba (la chispa,
//! con las muescas de los tres recuerdos, y la guardia), la vida de la Reina abajo a la izquierda, la
//! mira, la palabra que dice sobre su cabeza, la frase de Chiquita abajo, el rojo en los bordes cuando
//! duele, el destello del sello y las tarjetas de antes y después.
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
    // Chiquita's line.
    if let Some(line) = &view.line {
        if !line.is_empty() {
            let size = 22.0;
            let who = "CHIQUITA";
            let lines = wrap(font, Face::Body, line, size, 760.0);
            let ww = text_width(font, Face::Ui, who, 15.0);
            let tw = lines
                .iter()
                .map(|l| text_width(font, Face::Body, l, size))
                .fold(0.0, f32::max);
            let w = (ww + 18.0 + tw + 50.0).max(260.0);
            let h = 26.0 + lines.len() as f32 * size * 1.35;
            let r = Rect::new((width - w) * 0.5, height - 40.0 - h, w, h);
            soft(v, r, 6.0, 12.0, rgba(SHADOW, 0.7), false);
            panel(
                v,
                r,
                5.0,
                Panel::fill(0x2a1440, 0x140a22, 0.94)
                    .rim(GOLD, 1.0, 2.0)
                    .hairline(4.0, 0.3),
            );
            text(
                font,
                v,
                Face::Ui,
                who,
                r.x + 24.0,
                r.y + 16.0,
                15.0,
                rgba(0xff8ad8, 1.0),
            );
            for (i, l) in lines.iter().enumerate() {
                text_shadow(
                    font,
                    v,
                    Face::Body,
                    l,
                    r.x + 24.0 + ww + 18.0,
                    r.y + 12.0 + i as f32 * size * 1.35,
                    size,
                    rgba(0xf6ecd0, 1.0),
                );
            }
        }
    }
    if let Some(card) = &view.card {
        draw_card(font, v, width, height, card, view.time);
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
    // The world behind, dimmed.
    panel(
        v,
        Rect::new(0.0, 0.0, width, height),
        0.0,
        Panel::fill(0x0a0612, 0x0a0612, 0.62),
    );
    let w = 640.0;
    let lead = wrap(font, Face::Body, &card.text, 19.0, w - 80.0);
    let keys_h = if card.keys { 3.0 * 34.0 + 10.0 } else { 0.0 };
    let h = 70.0 + 58.0 + lead.len() as f32 * 28.0 + 18.0 + keys_h + 64.0;
    let r = Rect::new((width - w) * 0.5, (height - h) * 0.5, w, h);
    soft(v, r, 8.0, 22.0, rgba(SHADOW, 0.8), false);
    panel(
        v,
        r,
        6.0,
        Panel::velvet(0.96, 2.0)
            .hairline(6.0, 0.35)
            .sheen(time * 0.08),
    );
    let cx = width * 0.5;
    let mut y = r.y + 28.0;
    let kw = text_width(font, Face::Italic, &card.kicker, 18.0);
    text(
        font,
        v,
        Face::Italic,
        &card.kicker,
        cx - kw * 0.5,
        y,
        18.0,
        rgba(0xc9b08a, 1.0),
    );
    y += 34.0;
    let tw = text_width(font, Face::Title, &card.title, 44.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &card.title,
        cx - tw * 0.5,
        y,
        44.0,
        rgba(0xf4e6c8, 1.0),
    );
    y += 66.0;
    for l in &lead {
        let lw = text_width(font, Face::Body, l, 19.0);
        text(
            font,
            v,
            Face::Body,
            l,
            cx - lw * 0.5,
            y,
            19.0,
            rgba(0xe8dcc0, 1.0),
        );
        y += 28.0;
    }
    y += 10.0;
    if card.keys {
        let keys = [
            ("WASD", "moverte"),
            ("Mayús", "correr"),
            ("Espacio", "saltar la onda"),
            ("Clic", "espada"),
            ("Clic dcho.", "el sello (mantener)"),
            ("Esc", "pausa"),
        ];
        for (i, (key, what)) in keys.iter().enumerate() {
            let col = (i % 2) as f32;
            let row = (i / 2) as f32;
            let kx = r.x + 60.0 + col * 280.0;
            let ky = y + row * 34.0;
            let kw = text_width(font, Face::Title, key, 15.0) + 22.0;
            keycap(
                font,
                v,
                Rect::new(kx, ky, kw, 28.0),
                key,
                15.0,
                false,
                false,
                GOLD,
                1.0,
            );
            text(
                font,
                v,
                Face::Body,
                what,
                kx + kw + 10.0,
                ky + 4.0,
                17.0,
                rgba(0xe8dcc0, 1.0),
            );
        }
        y += keys_h;
    }
    // What to do now.
    let total: f32 = card
        .actions
        .iter()
        .map(|(k, a)| {
            text_width(font, Face::Title, k, 16.0)
                + 22.0
                + 10.0
                + text_width(font, Face::Ui, a, 18.0)
                + 36.0
        })
        .sum();
    let mut ax = cx - total * 0.5;
    for (key, what) in &card.actions {
        let kw = text_width(font, Face::Title, key, 16.0) + 22.0;
        keycap(
            font,
            v,
            Rect::new(ax, y + 6.0, kw, 30.0),
            key,
            16.0,
            true,
            false,
            0xffd27a,
            1.0,
        );
        text_shadow(
            font,
            v,
            Face::Ui,
            what,
            ax + kw + 10.0,
            y + 10.0,
            18.0,
            rgba(0xffe6b0, 1.0),
        );
        ax += kw + 10.0 + text_width(font, Face::Ui, what, 18.0) + 36.0;
    }
}
