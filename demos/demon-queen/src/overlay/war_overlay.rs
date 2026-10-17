//! La mesa de guerra en el estilo Persona del juego, como la tienda y la corte: la portada de la
//! mesa (campaña, batallas rápidas), el HUD de batalla (la placa del título, el equilibrio de
//! fuerzas, la velocidad, los avisos, el minimapa, las cartas de los regimientos con sus barras,
//! las órdenes, la chispa de la Reina, los estandartes sobre cada regimiento, las fichas, el
//! despliegue, la pausa, la ayuda y el resultado) y el de la campaña (los recursos, el turno, los
//! nombres de las provincias y de los ejércitos sobre el mapa, el panel de lo elegido, el diario,
//! el fin de turno y los avisos de batalla). Todo lo que se pulsa es un botón con su rectángulo,
//! puesto por quien arma la vista; el ratón y las teclas cambian luz y color, nunca un sitio ni un
//! tamaño. Hecho a 1920×1080.
use std::f32::consts::{PI, TAU};

use super::persona::*;
use super::persona_screens::{fill, heading, rim, slanted};
use super::stage_overlay::PictureRun;
use super::ui_paint::*;
use super::*;

const INK_DARK: u32 = 0x0e0816;
const INK2: u32 = 0x1a1028;
const INK3: u32 = 0x251838;
const BONE: u32 = 0xf6efe4;
const IVORY: u32 = 0xefe7cf;
const MUTED: u32 = 0xa898b8;
const AMBER: u32 = 0xf2b54a;
const AMBER_HI: u32 = 0xffd88a;
const VIOLET: u32 = 0x9a5cff;
const VIOLET_HI: u32 = 0xc79aff;
const CRIMSON: u32 = 0xe2334a;
const GOOD: u32 = 0x4fd08a;

/// The minimap's picture in the overlay's store.
pub(crate) const MINIMAP: &str = "war-minimap";

/// What a click on something of the war's does.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) enum WarHit {
    #[default]
    None,
    /// 0 pause, 1–3 speed.
    Speed(u8),
    Help,
    Retreat,
    /// The retreat asked: yes or no.
    Confirm(bool),
    /// A regiment's card (by regiment index).
    Card(usize),
    /// 0 halt, 1 run, 2 fire at will, 3 skirmish, 4 the spark.
    Order(u8),
    /// The minimap, at this point of the field (0..1 both ways).
    Minimap(f32, f32),
    Begin,
    HelpClose,
    /// The result's buttons: 0 to the campaign, 1 again, 2 to the table.
    Result(u8),
    /// The war table's entries.
    Table(u8),
    /// A quick battle (by its place in the list).
    Quick(u8),
    /// The campaign: end the turn, a button of the panel, a button of the open card.
    EndTurn,
    Panel(u8),
    Modal(u8),
    /// Leave the war (back to the castle).
    Leave,
    /// The chosen army goes to this province (a chip on the map).
    Go(u8),
    /// The next army that can still move.
    NextArmy,
}

/// A button: where, what it says, its key, a line under it, its state.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WarButton {
    pub rect: [f32; 4],
    pub label: String,
    pub key: String,
    pub detail: String,
    pub enabled: bool,
    pub on: bool,
    pub hit: WarHit,
    /// 0 plain, 1 big (gold), 2 a square icon, 3 danger, 4 a row of the war table, 5 a chip of it
    /// (those two drawn by the table).
    pub style: u8,
    pub icon: Glyph,
    /// Drawn over an open card (1) or with the HUD (0).
    pub layer: u8,
}

impl WarButton {
    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        let [rx, ry, rw, rh] = self.rect;
        x >= rx - 6.0 && x <= rx + rw + 6.0 && y >= ry && y <= ry + rh
    }
}

/// The little pictures the HUD draws (shapes, no textures).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Glyph {
    #[default]
    None,
    Pause,
    Play,
    Help,
    Flag,
    Swords,
    Run,
    Bow,
    Skirmish,
    Halt,
    Spark,
    Gold,
    Food,
    Flame,
    Crown,
    Map,
    Hourglass,
    Army,
    Eye,
    Arrow,
    Wing,
    Fang,
    Skull,
    Spear,
    Horse,
    Staff,
    Tail,
    Back,
}

/// A regiment's card at the foot of the battle.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct UnitCard {
    pub rect: [f32; 4],
    pub reg: usize,
    pub edge: u32,
    pub glyph: Glyph,
    pub short: String,
    pub men: String,
    pub hp: f32,
    pub morale: f32,
    pub spark: Option<f32>,
    pub ammo: Option<f32>,
    pub state: Glyph,
    /// 0 plain, 1 bad, 2 dimmed.
    pub tone: u8,
    pub selected: bool,
    pub hover: bool,
    pub dead: bool,
    pub routing: bool,
    pub dimmed: bool,
    /// What is left of its ability's wait (0..1).
    pub cd: Option<f32>,
}

/// A regiment's standard over its troops: where its plate stands (laid out by `world_tags`), the
/// pole down to the troops, how visible it is, and what it says.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BannerTag {
    pub rect: [f32; 4],
    pub pole: [[f32; 2]; 2],
    pub alpha: f32,
    /// Only its emblem and its bar (far, or no room).
    pub compact: bool,
    pub reg: usize,
    pub short: String,
    pub men: String,
    pub glyph: Glyph,
    pub hp: f32,
    pub edge: u32,
    pub enemy: bool,
    pub selected: bool,
    pub hover: bool,
    pub routing: bool,
    pub state: Glyph,
}

/// A dot on the minimap (0..1 both ways).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MiniDot {
    pub at: [f32; 2],
    pub width: f32,
    pub heading: f32,
    pub color: u32,
    pub hero: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Minimap {
    pub rect: [f32; 4],
    pub label: String,
    pub dots: Vec<MiniDot>,
    /// The deployment zone (0..1).
    pub zone: Option<[f32; 4]>,
    /// What the camera sees (0..1).
    pub view: Vec<[f32; 2]>,
}

/// A card of facts by the pointer (a regiment, a province).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Tip {
    pub at: [f32; 2],
    pub edge: u32,
    pub kicker: String,
    pub title: String,
    pub rows: Vec<(String, String)>,
    pub text: String,
    pub action: String,
}

/// A row of the result's tables.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ResultRow {
    pub glyph: Glyph,
    pub name: String,
    pub men0: u32,
    pub lost: u32,
    pub kills: u32,
    pub awake: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Results {
    pub win: bool,
    pub label: String,
    pub sides: [(String, u32, Vec<ResultRow>); 2],
    pub note: String,
    pub age: f32,
}

/// The battle's HUD.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BattleView {
    pub kicker: String,
    pub title: String,
    pub subtitle: String,
    pub names: [String; 2],
    pub colors: [u32; 2],
    pub darks: [u32; 2],
    pub balance: f32,
    pub clock: String,
    pub messages: Vec<(String, u32, f32)>,
    pub minimap: Option<Minimap>,
    pub cards: Vec<UnitCard>,
    pub banners: Vec<BannerTag>,
    pub selbox: Option<[f32; 4]>,
    pub deploying: bool,
    pub paused: bool,
    /// The spark's round button: centre, radius, ready, what is left of its wait, the Queen down.
    pub spark: Option<([f32; 2], f32, bool, f32, bool)>,
    pub help: bool,
    pub confirm: bool,
    pub results: Option<Results>,
    /// Where the speed buttons and the orders stand (their plates and kickers).
    pub speed_group: Option<[f32; 4]>,
    pub orders_group: Option<[f32; 4]>,
}

/// A province's name over the map.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ProvLabel {
    pub id: usize,
    /// Where it stands (laid out by `world_tags`), and how visible.
    pub rect: [f32; 4],
    pub alpha: f32,
    pub name: String,
    pub owner: String,
    pub color: u32,
    pub scale: f32,
}

/// An army's plate over its figure.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ArmyTag {
    pub id: usize,
    /// Where its plate stands (laid out by `world_tags`), its pole down to the figure, how visible.
    pub rect: [f32; 4],
    pub pole: [[f32; 2]; 2],
    pub alpha: f32,
    pub name: String,
    pub men: String,
    pub moves: Option<(u32, u32)>,
    /// On a march (a flag on its plate).
    pub marching: bool,
    pub edge: u32,
    pub glyph: Glyph,
    pub selected: bool,
}

/// The two sides of a battle about to be fought.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Versus {
    pub names: [String; 2],
    pub colors: [u32; 2],
    pub units: [Vec<(Glyph, String, String)>; 2],
    pub odds: f32,
    pub note: String,
}

/// A card open over the map (the intro, a battle, the end).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Modal {
    pub rect: [f32; 4],
    pub edge: u32,
    pub kicker: String,
    pub title: String,
    pub text: Vec<String>,
    pub versus: Option<Versus>,
}

/// The panel at the foot of the campaign: what is chosen.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CampPanel {
    pub rect: [f32; 4],
    pub edge: u32,
    pub kicker: String,
    pub title: String,
    pub sub: String,
    pub stats: Vec<(Glyph, String)>,
    pub units: Vec<(Glyph, String, String)>,
    pub notes: Vec<(String, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CampaignView {
    /// Gold, food, spark: (what, how much, its change a turn, the change is a loss).
    pub resources: Vec<(String, String, String, bool, Glyph, u32)>,
    pub turn: String,
    pub season: String,
    pub labels: Vec<ProvLabel>,
    pub armies: Vec<ArmyTag>,
    pub panel: CampPanel,
    pub log: Vec<(String, u32)>,
    pub modal: Option<Modal>,
    pub toast: Option<(String, String, f32)>,
    pub hint: String,
    /// What wins the campaign (under the turn).
    pub objective: String,
    /// The chosen army's ways to its destinations: (from, to, colour, the pointer is on it).
    pub routes: Vec<([f32; 2], [f32; 2], u32, bool)>,
    /// The marches on the map.
    pub marches: Vec<MarchView>,
}

/// A march on the campaign's map: its way on the screen (from the army), where each turn's walk
/// ends (the turn's number), and what it is: 1 another army's, 2 the chosen army's, 3 the one the
/// chosen army would make (over the province under the pointer).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct MarchView {
    pub points: Vec<[f32; 2]>,
    pub turns: Vec<(u32, [f32; 2])>,
    pub kind: u8,
}

/// The war table's front page.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TableView {
    pub title: String,
    pub tag: String,
    pub keys: Vec<(String, String)>,
    /// The row the focus is on (among the buttons of style 4, in order), and its card.
    pub focus: usize,
    pub card: TableCard,
}

/// What the war table tells of the row the focus is on.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TableCard {
    pub kicker: String,
    pub title: String,
    pub lines: Vec<String>,
    pub stats: Vec<(Glyph, String)>,
    /// The two sides of a battle: (what, colour, its units: emblem, name, how many).
    pub sides: Vec<(String, u32, Vec<(Glyph, String, u32)>)>,
    pub warning: String,
}

/// What the war shows this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WarView {
    pub time: f32,
    pub pointer: Option<[f32; 2]>,
    pub table: Option<TableView>,
    pub battle: Option<BattleView>,
    pub campaign: Option<CampaignView>,
    pub buttons: Vec<WarButton>,
    pub tip: Option<Tip>,
    /// The picture of the minimap is up.
    pub minimap_ready: bool,
}

impl WarView {
    /// What is under the pointer (the open card's buttons first, then the HUD's, the cards, the
    /// banners, the minimap).
    pub(crate) fn hit(&self, x: f32, y: f32) -> WarHit {
        let modal_open = self.buttons.iter().any(|b| b.layer == 1);
        for layer in [1u8, 0] {
            if modal_open && layer == 0 {
                break;
            }
            if let Some(b) = self
                .buttons
                .iter()
                .rev()
                .find(|b| b.layer == layer && b.contains(x, y))
            {
                return if b.enabled { b.hit } else { WarHit::None };
            }
        }
        if modal_open {
            return WarHit::None;
        }
        if let Some(b) = &self.battle {
            if b.results.is_some() || b.help || b.confirm {
                return WarHit::None;
            }
            if let Some((c, r, ..)) = b.spark {
                if (x - c[0]).hypot(y - c[1]) < r {
                    return WarHit::Order(4);
                }
            }
            for c in &b.cards {
                let [rx, ry, rw, rh] = c.rect;
                if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                    return WarHit::Card(c.reg);
                }
            }
            if let Some(m) = &b.minimap {
                let [rx, ry, rw, rh] = m.rect;
                if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                    return WarHit::Minimap((x - rx) / rw, (y - ry) / rh);
                }
            }
        }
        WarHit::None
    }

    /// Whether the pointer is over the HUD (the world under it does not take the click).
    pub(crate) fn over_hud(&self, x: f32, y: f32) -> bool {
        if self.hit(x, y) != WarHit::None {
            return true;
        }
        if let Some(c) = &self.campaign {
            let [rx, ry, rw, rh] = c.panel.rect;
            if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                return true;
            }
            if c.modal.is_some() {
                return true;
            }
        }
        if let Some(b) = &self.battle {
            if b.results.is_some() || b.help || b.confirm {
                return true;
            }
        }
        self.table.is_some()
    }
}

fn r4(r: [f32; 4]) -> Rect {
    Rect::new(r[0], r[1], r[2], r[3])
}

fn plate(v: &mut Vec<MapVertex>, x: f32, y: f32, w: f32, h: f32, lean: f32, edge: u32, alpha: f32) {
    fill(
        v,
        slanted(x + 8.0, y + 9.0, w, h, lean),
        rgba(0x000000, 0.45 * alpha),
    );
    let q = slanted(x, y, w, h, lean);
    push_triangle(v, [q[0], q[1], q[2]], rgba(INK2, 0.94 * alpha));
    push_triangle(v, [q[0], q[2], q[3]], rgba(INK_DARK, 0.94 * alpha));
    push_outline_fringe(v, &q, rgba(INK_DARK, 0.94 * alpha));
    // the coloured edge on the left
    let e = [
        q[0],
        [q[0][0] + 6.0, q[0][1]],
        [q[3][0] + 6.0, q[3][1]],
        q[3],
    ];
    fill(v, e, rgba(edge, alpha));
    // a hatch of fine diagonals
    let mut k = x + 20.0;
    while k < x + w - 10.0 {
        wire(
            v,
            [k, y + h - 3.0],
            [k + 10.0, y + 3.0],
            1.0,
            rgba(0xffffff, 0.025 * alpha),
            rgba(0xffffff, 0.0),
            0.0,
            false,
        );
        k += 14.0;
    }
}

fn kicker(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    s: &str,
    x: f32,
    y: f32,
    color: u32,
    alpha: f32,
) {
    let w = text_width(font, Face::Display, s, 18.0) + 26.0;
    fill(v, slanted(x, y, w, 28.0, 7.0), rgba(color, alpha));
    text(
        font,
        v,
        Face::Display,
        s,
        x + 13.0,
        y + 2.0,
        18.0,
        rgba(INK_DARK, alpha),
    );
}

/// Fits `s` in `w` pixels at `size` or less (down to 70 %).
fn fit(font: &FontAtlas, face: Face, s: &str, size: f32, w: f32) -> f32 {
    let tw = text_width(font, face, s, size).max(1.0);
    if tw <= w {
        size
    } else {
        (size * w / tw).max(size * 0.7)
    }
}

/// The glyphs, drawn with lines and discs round `c`, `s` pixels across.
pub(super) fn glyph(v: &mut Vec<MapVertex>, g: Glyph, c: [f32; 2], s: f32, color: [f32; 4]) {
    let w = (s * 0.11).max(1.6);
    let p = |x: f32, y: f32| [c[0] + x * s * 0.5, c[1] + y * s * 0.5];
    let line = |v: &mut Vec<MapVertex>, a: [f32; 2], b: [f32; 2]| {
        wire(v, a, b, w, color, color, 0.0, false)
    };
    match g {
        Glyph::None => {}
        Glyph::Pause => {
            line(v, p(-0.35, -0.6), p(-0.35, 0.6));
            line(v, p(0.35, -0.6), p(0.35, 0.6));
        }
        Glyph::Play => {
            push_triangle(v, [p(-0.4, -0.6), p(0.6, 0.0), p(-0.4, 0.6)], color);
        }
        Glyph::Help => {
            arc(v, p(0.0, -0.25), s * 0.24, w, 0.62, 0.72, color, [0.0; 4]);
            line(v, p(0.05, 0.05), p(0.0, 0.3));
            disc(v, p(0.0, 0.62), w * 0.7, color);
        }
        Glyph::Flag => {
            line(v, p(-0.5, -0.7), p(-0.5, 0.75));
            push_triangle(v, [p(-0.45, -0.7), p(0.6, -0.42), p(-0.45, -0.1)], color);
        }
        Glyph::Swords => {
            line(v, p(-0.6, -0.6), p(0.6, 0.6));
            line(v, p(0.6, -0.6), p(-0.6, 0.6));
            line(v, p(-0.65, 0.3), p(-0.3, 0.65));
            line(v, p(0.65, 0.3), p(0.3, 0.65));
        }
        Glyph::Run => {
            disc(v, p(0.25, -0.62), w * 1.3, color);
            line(v, p(0.15, -0.35), p(-0.1, 0.15));
            line(v, p(-0.1, 0.15), p(-0.5, 0.65));
            line(v, p(-0.1, 0.15), p(0.35, 0.65));
            line(v, p(0.1, -0.2), p(0.55, 0.05));
            line(v, p(0.1, -0.2), p(-0.45, -0.15));
        }
        Glyph::Bow | Glyph::Arrow => {
            if g == Glyph::Bow {
                // the limbs bent back from the grip, the string straight across
                let n = 10;
                let mut prev = None;
                for k in 0..=n {
                    let a = -1.15 + 2.3 * k as f32 / n as f32;
                    let q = p(-0.15 - 0.45 * a.cos(), 0.75 * a.sin());
                    if let Some(o) = prev {
                        line(v, o, q);
                    }
                    prev = Some(q);
                }
                let (t0, t1) = (
                    p(-0.15 - 0.45 * 1.15f32.cos(), -0.75 * 1.15f32.sin()),
                    p(-0.15 - 0.45 * 1.15f32.cos(), 0.75 * 1.15f32.sin()),
                );
                wire(v, t0, t1, w * 0.5, color, color, 0.0, false);
            }
            line(v, p(-0.55, 0.0), p(0.62, 0.0));
            push_triangle(v, [p(0.8, 0.0), p(0.5, -0.2), p(0.5, 0.2)], color);
        }
        Glyph::Skirmish => {
            line(v, p(-0.6, 0.4), p(0.5, -0.5));
            push_triangle(v, [p(0.62, -0.62), p(0.3, -0.52), p(0.52, -0.3)], color);
            line(v, p(-0.6, -0.1), p(0.0, 0.6));
        }
        Glyph::Halt => {
            panel(
                v,
                Rect::new(c[0] - s * 0.36, c[1] - s * 0.36, s * 0.72, s * 0.72),
                s * 0.08,
                Panel::fill(rgb_of(color), rgb_of(color), color[3]),
            );
        }
        Glyph::Spark | Glyph::Flame => {
            let k = if g == Glyph::Spark { 1.0 } else { 0.85 };
            push_triangle(
                v,
                [p(0.0, -0.85 * k), p(0.42 * k, 0.25), p(-0.42 * k, 0.25)],
                color,
            );
            disc(v, p(0.0, 0.3), s * 0.25 * k, color);
            if g == Glyph::Spark {
                for a in 0..4 {
                    let t = a as f32 / 4.0 * TAU + PI / 4.0;
                    line(
                        v,
                        p(t.cos() * 0.62, t.sin() * 0.62),
                        p(t.cos() * 0.9, t.sin() * 0.9),
                    );
                }
            }
        }
        Glyph::Gold => {
            disc(v, c, s * 0.38, color);
            disc(
                v,
                c,
                s * 0.24,
                [color[0] * 0.65, color[1] * 0.55, color[2] * 0.3, color[3]],
            );
        }
        Glyph::Food => {
            disc(v, p(0.0, 0.15), s * 0.3, color);
            line(v, p(0.0, -0.2), p(0.15, -0.7));
            push_triangle(v, [p(0.1, -0.5), p(0.55, -0.7), p(0.25, -0.35)], color);
        }
        Glyph::Crown => {
            push_triangle(v, [p(-0.7, 0.4), p(-0.55, -0.45), p(-0.2, 0.15)], color);
            push_triangle(v, [p(-0.3, 0.4), p(0.0, -0.65), p(0.3, 0.4)], color);
            push_triangle(v, [p(0.2, 0.15), p(0.55, -0.45), p(0.7, 0.4)], color);
            line(v, p(-0.7, 0.5), p(0.7, 0.5));
        }
        Glyph::Map => {
            for (a, b) in [
                ((-0.7, -0.5), (-0.2, -0.7)),
                ((-0.2, -0.7), (0.25, -0.5)),
                ((0.25, -0.5), (0.7, -0.7)),
                ((-0.7, 0.6), (-0.2, 0.4)),
                ((-0.2, 0.4), (0.25, 0.6)),
                ((0.25, 0.6), (0.7, 0.4)),
            ] {
                line(v, p(a.0, a.1), p(b.0, b.1));
            }
            for x in [-0.7, -0.2, 0.25, 0.7] {
                let (y0, y1) = if x == -0.2 || x == 0.7 {
                    (-0.7, 0.4)
                } else {
                    (-0.5, 0.6)
                };
                line(v, p(x, y0), p(x, y1));
            }
        }
        Glyph::Hourglass => {
            push_triangle(v, [p(-0.5, -0.7), p(0.5, -0.7), p(0.0, 0.0)], color);
            push_triangle(v, [p(-0.5, 0.7), p(0.5, 0.7), p(0.0, 0.0)], color);
        }
        Glyph::Army => {
            for (x, y) in [(-0.45, 0.1), (0.0, -0.25), (0.45, 0.1)] {
                disc(v, p(x, y - 0.3), s * 0.13, color);
                line(v, p(x, y - 0.12), p(x, y + 0.5));
            }
        }
        Glyph::Eye => {
            // an almond: two curves meeting at the corners, the pupil in it
            let n = 10;
            for side in [-1.0f32, 1.0] {
                let mut prev = None;
                for k in 0..=n {
                    let t = k as f32 / n as f32;
                    let x = -0.75 + 1.5 * t;
                    let y = side * 0.42 * (1.0 - (2.0 * t - 1.0).powi(2));
                    let q = p(x, y);
                    if let Some(o) = prev {
                        line(v, o, q);
                    }
                    prev = Some(q);
                }
            }
            disc(v, c, s * 0.15, color);
        }
        Glyph::Wing => {
            for k in 0..4 {
                let f = k as f32 * 0.25;
                line(
                    v,
                    p(-0.6, -0.4 + f * 0.3),
                    p(0.65 - f * 0.4, -0.6 + f * 1.4),
                );
            }
        }
        Glyph::Fang => {
            push_triangle(v, [p(-0.6, -0.5), p(-0.1, -0.5), p(-0.35, 0.7)], color);
            push_triangle(v, [p(0.1, -0.5), p(0.6, -0.5), p(0.35, 0.7)], color);
        }
        Glyph::Skull => {
            disc(v, p(0.0, -0.15), s * 0.38, color);
            panel(
                v,
                Rect::new(c[0] - s * 0.18, c[1] + s * 0.05, s * 0.36, s * 0.3),
                2.0,
                Panel::fill(rgb_of(color), rgb_of(color), color[3]),
            );
            disc(v, p(-0.17, -0.12), s * 0.09, rgba(INK_DARK, color[3]));
            disc(v, p(0.17, -0.12), s * 0.09, rgba(INK_DARK, color[3]));
        }
        Glyph::Spear => {
            line(v, p(-0.6, 0.7), p(0.45, -0.45));
            push_triangle(v, [p(0.7, -0.7), p(0.3, -0.52), p(0.52, -0.3)], color);
        }
        Glyph::Horse => {
            line(v, p(-0.6, 0.0), p(0.3, 0.0));
            line(v, p(0.3, 0.0), p(0.6, -0.55));
            line(v, p(-0.5, 0.0), p(-0.55, 0.65));
            line(v, p(0.2, 0.0), p(0.25, 0.65));
            disc(v, p(0.62, -0.58), s * 0.12, color);
        }
        Glyph::Staff => {
            line(v, p(-0.2, 0.75), p(0.15, -0.4));
            disc(v, p(0.2, -0.55), s * 0.2, color);
        }
        Glyph::Tail => {
            arc(v, p(0.0, 0.1), s * 0.4, w * 1.2, 0.0, 0.7, color, [0.0; 4]);
            disc(v, p(0.4, -0.35), s * 0.12, color);
        }
        Glyph::Back => {
            line(v, p(0.5, 0.0), p(-0.5, 0.0));
            line(v, p(-0.5, 0.0), p(-0.1, -0.4));
            line(v, p(-0.5, 0.0), p(-0.1, 0.4));
        }
    }
}

fn rgb_of(c: [f32; 4]) -> u32 {
    super::persona_screens::rgb(c)
}

fn button(font: &FontAtlas, v: &mut Vec<MapVertex>, b: &WarButton, hover: bool, time: f32) {
    if b.style >= 4 {
        return;
    }
    let [x, y, w, h] = b.rect;
    let lean = (h * 0.2).min(10.0);
    let alpha = if b.enabled { 1.0 } else { 0.45 };
    let lit = hover && b.enabled;
    let (top, bottom, edge) = match b.style {
        1 => (
            if lit { 0xffcf6a } else { AMBER },
            if lit { 0xd9952a } else { 0xb7822e },
            INK_DARK,
        ),
        3 => (if lit { 0x8a2030 } else { 0x5a1420 }, 0x2a080c, CRIMSON),
        _ if b.on => (0x5a3490, 0x24143e, AMBER),
        _ => (
            if lit { 0x3a2458 } else { INK3 },
            INK_DARK,
            if lit { AMBER_HI } else { 0x5a4a6e },
        ),
    };
    fill(
        v,
        slanted(x + 6.0, y + 7.0, w, h, lean),
        rgba(0x000000, 0.45 * alpha),
    );
    let q = slanted(x, y, w, h, lean);
    push_triangle(v, [q[0], q[1], q[2]], rgba(top, alpha));
    push_triangle(v, [q[0], q[2], q[3]], rgba(bottom, alpha));
    push_outline_fringe(v, &q, rgba(bottom, alpha));
    rim(
        v,
        q,
        if lit { 2.4 } else { 1.4 },
        rgba(edge, if lit { 1.0 } else { 0.8 } * alpha),
    );
    if lit {
        soft(
            v,
            r4(b.rect),
            8.0,
            16.0,
            rgba(AMBER, 0.25 + 0.1 * wave(time, 1.0)),
            true,
        );
    }
    let ink = if b.style == 1 { INK_DARK } else { IVORY };
    let mut tx = x + 16.0;
    if b.icon != Glyph::None {
        let gs = (h * 0.5).min(30.0);
        let gc = if b.label.is_empty() {
            [x + w / 2.0, y + h / 2.0]
        } else {
            [x + 12.0 + gs * 0.6, y + h / 2.0]
        };
        glyph(
            v,
            b.icon,
            gc,
            gs,
            rgba(
                if b.style == 1 {
                    INK_DARK
                } else if b.on {
                    AMBER_HI
                } else {
                    AMBER
                },
                alpha,
            ),
        );
        tx = gc[0] + gs * 0.8;
    }
    if !b.label.is_empty() {
        let key_w = if b.key.is_empty() {
            0.0
        } else {
            text_width(font, Face::Title, &b.key, 15.0) + 22.0
        };
        let size = if b.style == 1 {
            27.0
        } else if h >= 56.0 {
            23.0
        } else {
            20.0
        };
        let room = x + w - tx - key_w - 16.0;
        let fitted = fit(font, Face::Round, &b.label, size, room.max(30.0));
        // a long label (Russian, German…) that does not fit even small goes on two lines
        let two = b.detail.is_empty()
            && demon_queen_dungeon::i18n::tr(&b.label).contains(' ')
            && h >= 40.0
            && text_width(font, Face::Round, &b.label, fitted) > room.max(30.0);
        if two {
            let s2 = (h * 0.34).min(size * 0.8);
            // small enough that its longest word stays whole on a line
            let said = demon_queen_dungeon::i18n::tr(&b.label);
            let widest = said
                .split(' ')
                .map(|w| text_width(font, Face::Round, w, s2))
                .fold(1.0, f32::max);
            let s2 = s2 * (room.max(30.0) / widest).min(1.0);
            let lines = wrap(font, Face::Round, &b.label, s2, room.max(30.0));
            let n = lines.len().min(2) as f32;
            let ty = y + (h - n * s2 * 1.18) / 2.0;
            for (i, l) in lines.iter().take(2).enumerate() {
                let ls =
                    s2 * (room.max(30.0) / text_width(font, Face::Round, l, s2).max(1.0)).min(1.0);
                text(
                    font,
                    v,
                    Face::Round,
                    l,
                    tx,
                    ty + i as f32 * s2 * 1.18,
                    ls,
                    rgba(ink, alpha),
                );
            }
        }
        let size = fitted;
        let ds = if b.detail.is_empty() {
            0.0
        } else {
            fit(
                font,
                Face::RoundItalic,
                &b.detail,
                if h >= 70.0 { 18.0 } else { 15.0 },
                room.max(30.0),
            )
        };
        let block = size * 1.25 + if ds > 0.0 { ds * 1.4 } else { 0.0 };
        let ty = y + (h - block) / 2.0;
        if !two {
            text(
                font,
                v,
                Face::Round,
                &b.label,
                tx,
                ty,
                size,
                rgba(ink, alpha),
            );
        }
        if ds > 0.0 {
            text(
                font,
                v,
                Face::RoundItalic,
                &b.detail,
                tx,
                ty + size * 1.3,
                ds,
                rgba(if b.style == 1 { 0x3a2410 } else { 0xc8b8d8 }, alpha),
            );
        }
        if !b.key.is_empty() {
            keycap(
                font,
                v,
                Rect::new(x + w - key_w - 10.0, y + (h - 28.0) / 2.0, key_w, 28.0),
                &b.key,
                15.0,
                lit,
                !b.enabled,
                AMBER,
                alpha,
            );
        }
    }
}

fn bar_slant(v: &mut Vec<MapVertex>, x: f32, y: f32, w: f32, h: f32, k: f32, color: u32) {
    fill(v, slanted(x, y, w, h, 2.0), rgba(0x000000, 0.7));
    let k = k.clamp(0.0, 1.0);
    if k > 0.001 {
        fill(v, slanted(x, y, w * k, h, 2.0), rgba(color, 1.0));
    }
}

// ------------------------------------------------------------------ the war table
fn draw_table(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    view: &TableView,
    buttons: &[WarButton],
    pointer: Option<[f32; 2]>,
    height: f32,
    time: f32,
) {
    // the diorama stays in view: an ink band under the list, its slanted edge in gold
    fill(
        v,
        [[0.0, 0.0], [1010.0, 0.0], [940.0, height], [0.0, height]],
        rgba(0x07040c, 0.74),
    );
    wire(
        v,
        [1010.0, 0.0],
        [940.0, height],
        3.0,
        rgba(AMBER, 0.9),
        rgba(AMBER, 0.9),
        0.0,
        false,
    );
    wire(
        v,
        [1026.0, 0.0],
        [956.0, height],
        1.2,
        rgba(AMBER, 0.35),
        rgba(AMBER, 0.35),
        0.0,
        false,
    );
    speed_lines(
        v,
        Rect::new(0.0, height * 0.2, 940.0, height * 0.6),
        -0.2,
        14,
        time,
        rgba(0xffffff, 0.03),
    );
    heading(font, v, 120.0, 70.0, &view.title, &view.tag, AMBER);
    // the units' size row ends where its chips begin
    let chips_x = buttons
        .iter()
        .filter(|b| b.style == 5)
        .map(|b| b.rect[0])
        .fold(f32::MAX, f32::min);
    for (n, b) in buttons.iter().filter(|b| b.style == 4).enumerate() {
        let room = if b.hit == WarHit::Table(3) {
            chips_x - 16.0
        } else {
            b.rect[0] + b.rect[2]
        };
        table_row(font, v, b, n, n == view.focus, room, time);
    }
    for b in buttons.iter().filter(|b| b.style == 5) {
        let lit = pointer.is_some_and(|p| b.contains(p[0], p[1]));
        let [x, y, w, h] = b.rect;
        let q = slanted(x, y, w, h, 8.0);
        fill(
            v,
            slanted(x + 3.0, y + 4.0, w, h, 8.0),
            rgba(0x000000, 0.45),
        );
        fill(
            v,
            q,
            rgba(
                if b.on {
                    INK_DARK
                } else if lit {
                    0x3a2458
                } else {
                    INK2
                },
                0.96,
            ),
        );
        rim(
            v,
            q,
            if b.on { 2.2 } else { 1.0 },
            rgba(if b.on || lit { AMBER } else { 0x5a4a6e }, 1.0),
        );
        // (Anton has no × nor ½: these take the round face)
        let tw = text_width(font, Face::Round, &b.label, 21.0);
        text(
            font,
            v,
            Face::Round,
            &b.label,
            x + w / 2.0 - tw / 2.0 + 2.0,
            y + h / 2.0 - 14.0,
            21.0,
            rgba(if b.on { AMBER } else { IVORY }, 1.0),
        );
    }
    table_card(font, v, &view.card);
    // the keys
    let mut x = 120.0;
    for (k, what) in &view.keys {
        let arrows = k == "^v";
        let cap = if arrows {
            54.0
        } else {
            text_width(font, Face::Title, k, 16.0) + 24.0
        };
        let r = Rect::new(x, height - 78.0, cap, 34.0);
        keycap(
            font,
            v,
            r,
            if arrows { "" } else { k },
            16.0,
            false,
            false,
            AMBER,
            1.0,
        );
        if arrows {
            let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
            fill(
                v,
                [
                    [cx - 9.0, cy - 6.0],
                    [cx - 3.0, cy + 5.0],
                    [cx - 15.0, cy + 5.0],
                    [cx - 9.0, cy - 6.0],
                ],
                rgba(IVORY, 1.0),
            );
            fill(
                v,
                [
                    [cx + 3.0, cy - 5.0],
                    [cx + 15.0, cy - 5.0],
                    [cx + 9.0, cy + 6.0],
                    [cx + 3.0, cy - 5.0],
                ],
                rgba(IVORY, 1.0),
            );
        }
        text_shadow(
            font,
            v,
            Face::Italic,
            what,
            x + cap + 10.0,
            height - 72.0,
            19.0,
            rgba(CREAM, 0.9),
        );
        x += cap + 30.0 + text_width(font, Face::Italic, what, 19.0);
    }
}

/// A row of the war table: its number on a block, its emblem, what it is and a line under it; the
/// focus lights it gold (colour only: the row keeps its place and size).
fn table_row(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    b: &WarButton,
    n: usize,
    focus: bool,
    right: f32,
    time: f32,
) {
    let [x, y, w, h] = b.rect;
    let lean = 16.0;
    let alpha = if b.enabled { 1.0 } else { 0.45 };
    fill(
        v,
        slanted(x + 8.0, y + 9.0, w, h, lean),
        rgba(0x000000, 0.5 * alpha),
    );
    let q = slanted(x, y, w, h, lean);
    let (top, bottom) = if focus {
        (0xffcf6a, 0xdf9f3a)
    } else {
        (INK3, INK_DARK)
    };
    push_triangle(v, [q[0], q[1], q[2]], rgba(top, 0.97 * alpha));
    push_triangle(v, [q[0], q[2], q[3]], rgba(bottom, 0.97 * alpha));
    push_outline_fringe(v, &q, rgba(bottom, 0.97 * alpha));
    if focus {
        soft(
            v,
            r4(b.rect),
            8.0,
            18.0,
            rgba(AMBER, 0.22 + 0.1 * wave(time, 1.0)),
            true,
        );
    }
    rim(
        v,
        q,
        if focus { 2.4 } else { 1.2 },
        rgba(if focus { INK_DARK } else { 0x5a4a6e }, 0.9 * alpha),
    );
    // the number, on its own block along the slant
    let nb = 78.0;
    let block = [q[0], [q[0][0] + nb, q[0][1]], [q[3][0] + nb, q[3][1]], q[3]];
    fill(v, block, rgba(if focus { INK_DARK } else { AMBER }, alpha));
    let num = format!("{:02}", n + 1);
    let ns = (h * 0.48).min(40.0);
    let cx = (q[0][0] + q[3][0]) / 2.0 + nb / 2.0;
    let nw = text_width(font, Face::Display, &num, ns);
    text(
        font,
        v,
        Face::Display,
        &num,
        cx - nw / 2.0,
        y + h / 2.0 - ns * 0.66,
        ns,
        rgba(if focus { AMBER } else { INK_DARK }, alpha),
    );
    let ink = if focus { INK_DARK } else { IVORY };
    let gx = x + nb + 44.0;
    glyph(
        v,
        b.icon,
        [gx, y + h / 2.0],
        (h * 0.36).min(30.0),
        rgba(if focus { INK_DARK } else { AMBER }, alpha),
    );
    let tx = gx + 34.0;
    let key_w = if b.key.is_empty() {
        0.0
    } else {
        text_width(font, Face::Title, &b.key, 16.0) + 24.0
    };
    let room = (right - tx - key_w - 30.0).max(60.0);
    let size = fit(
        font,
        Face::Round,
        &b.label,
        if h >= 90.0 { 27.0 } else { 24.0 },
        room,
    );
    if b.detail.is_empty() {
        text(
            font,
            v,
            Face::Round,
            &b.label,
            tx,
            y + h / 2.0 - size * 0.66,
            size,
            rgba(ink, alpha),
        );
    } else {
        let ds = fit(font, Face::RoundItalic, &b.detail, 16.0, room);
        let ty = y + (h - size * 1.25 - ds * 1.45) / 2.0;
        text(
            font,
            v,
            Face::Round,
            &b.label,
            tx,
            ty,
            size,
            rgba(ink, alpha),
        );
        text(
            font,
            v,
            Face::RoundItalic,
            &b.detail,
            tx,
            ty + size * 1.3,
            ds,
            rgba(if focus { 0x3a2410 } else { 0xc8b8d8 }, alpha),
        );
    }
    if !b.key.is_empty() {
        keycap(
            font,
            v,
            Rect::new(x + w - key_w - 22.0, y + (h - 34.0) / 2.0, key_w, 34.0),
            &b.key,
            16.0,
            focus,
            false,
            AMBER,
            alpha,
        );
    }
}

/// The card of the war table's focused row, at the right over the diorama.
fn table_card(font: &FontAtlas, v: &mut Vec<MapVertex>, c: &TableCard) {
    let (x, y, w) = (1100.0, 300.0, 700.0);
    let body: Vec<Vec<String>> = c
        .lines
        .iter()
        .map(|l| wrap(font, Face::Round, l, 21.0, w - 84.0))
        .collect();
    let most = c.sides.iter().map(|s| s.2.len()).max().unwrap_or(0) as f32;
    let mut h = 100.0
        + body
            .iter()
            .map(|ls| ls.len() as f32 * 31.0 + 12.0)
            .sum::<f32>();
    if !c.stats.is_empty() {
        h += 56.0;
    }
    if !c.sides.is_empty() {
        h += 48.0 + most * 30.0 + 8.0;
    }
    if !c.warning.is_empty() {
        h += 40.0;
    }
    plate(v, x, y, w, h, 16.0, AMBER, 1.0);
    kicker(font, v, &c.kicker, x + 30.0, y - 14.0, AMBER, 1.0);
    let ts = fit(font, Face::Title, &c.title, 36.0, w - 80.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &c.title,
        x + 36.0,
        y + 28.0,
        ts,
        rgba(BONE, 1.0),
    );
    let mut yy = y + 88.0;
    for ls in &body {
        for l in ls {
            text(
                font,
                v,
                Face::Round,
                l,
                x + 40.0,
                yy,
                21.0,
                rgba(CREAM, 0.95),
            );
            yy += 31.0;
        }
        yy += 12.0;
    }
    if !c.stats.is_empty() {
        let mut sx = x + 40.0;
        for (g, s) in &c.stats {
            let sw = text_width(font, Face::Round, s, 20.0) + 58.0;
            fill(
                v,
                slanted(sx + 3.0, yy + 4.0, sw, 38.0, 8.0),
                rgba(0x000000, 0.4),
            );
            fill(v, slanted(sx, yy, sw, 38.0, 8.0), rgba(INK3, 1.0));
            glyph(v, *g, [sx + 24.0, yy + 19.0], 20.0, rgba(AMBER, 1.0));
            text(
                font,
                v,
                Face::Round,
                s,
                sx + 42.0,
                yy + 5.0,
                20.0,
                rgba(BONE, 1.0),
            );
            sx += sw + 12.0;
        }
        yy += 56.0;
    }
    if !c.sides.is_empty() {
        let half = (w - 100.0) / 2.0;
        for (k, (name, color, units)) in c.sides.iter().enumerate() {
            let sx = x + 40.0 + k as f32 * (half + 20.0);
            let nw = text_width(font, Face::Display, name, 17.0) + 26.0;
            fill(v, slanted(sx, yy, nw, 28.0, 7.0), rgba(*color, 1.0));
            text(
                font,
                v,
                Face::Display,
                name,
                sx + 13.0,
                yy + 3.0,
                17.0,
                rgba(INK_DARK, 1.0),
            );
            for (j, (g, uname, count)) in units.iter().enumerate() {
                let ry = yy + 42.0 + j as f32 * 30.0;
                glyph(
                    v,
                    *g,
                    [sx + 12.0, ry + 12.0],
                    18.0,
                    rgba(mix(*color, 0xffffff, 0.35), 1.0),
                );
                let cnt = format!("×{count}");
                let cw = text_width(font, Face::Round, &cnt, 18.0);
                let us = fit(font, Face::Round, uname, 19.0, half - 44.0 - cw);
                text(
                    font,
                    v,
                    Face::Round,
                    uname,
                    sx + 30.0,
                    ry,
                    us,
                    rgba(IVORY, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Round,
                    &cnt,
                    sx + half - cw,
                    ry + 1.0,
                    18.0,
                    rgba(AMBER, 1.0),
                );
            }
        }
        yy += 48.0 + most * 30.0 + 8.0;
    }
    if !c.warning.is_empty() {
        glyph(
            v,
            Glyph::Flag,
            [x + 52.0, yy + 13.0],
            18.0,
            rgba(0xff8a7a, 1.0),
        );
        text(
            font,
            v,
            Face::RoundItalic,
            &c.warning,
            x + 72.0,
            yy,
            19.0,
            rgba(0xff8a7a, 1.0),
        );
    }
}

// ------------------------------------------------------------------ the battle
fn draw_banner(font: &FontAtlas, v: &mut Vec<MapVertex>, b: &BannerTag, time: f32) {
    faded(v, b.alpha, |v| {
        let [x, y, w, h] = b.rect;
        let lit = b.selected || b.hover;
        // the pole, from the plate's foot down to the troops, and a diamond where it lands
        let [p0, p1] = b.pole;
        if p1[1] - p0[1] > 2.0 {
            wire(
                v,
                p0,
                p1,
                4.0,
                rgba(INK_DARK, 0.7),
                rgba(INK_DARK, 0.2),
                0.0,
                false,
            );
            wire(
                v,
                p0,
                p1,
                2.0,
                rgba(b.edge, 0.95),
                rgba(b.edge, 0.35),
                0.0,
                false,
            );
            let d = 4.5;
            fill(
                v,
                [
                    [p1[0], p1[1] - d],
                    [p1[0] + d, p1[1]],
                    [p1[0], p1[1] + d],
                    [p1[0] - d, p1[1]],
                ],
                rgba(if lit { AMBER_HI } else { b.edge }, 0.95),
            );
        }
        let lean = 7.0;
        let ph = h - 6.0;
        let top = if b.routing {
            if (time * 4.0).fract() < 0.5 {
                0x7a1820
            } else {
                0x3a0a0e
            }
        } else if b.selected {
            0x4a2c78
        } else if b.enemy {
            0x1a1424
        } else {
            0x24163a
        };
        fill(
            v,
            slanted(x + 4.0, y + 5.0, w, ph, lean),
            rgba(0x000000, 0.5),
        );
        let q = slanted(x, y, w, ph, lean);
        push_triangle(v, [q[0], q[1], q[2]], rgba(top, 0.95));
        push_triangle(v, [q[0], q[2], q[3]], rgba(INK_DARK, 0.95));
        push_outline_fringe(v, &q, rgba(INK_DARK, 0.95));
        // the emblem's block of the side's colour (an enemy's is dark with a coloured rim)
        let bw = 32.0;
        let e = slanted(x, y, bw + lean, ph, lean);
        if b.enemy {
            fill(v, e, rgba(mix(b.edge, 0x000000, 0.55), 1.0));
            rim(v, e, 1.5, rgba(b.edge, 1.0));
        } else {
            fill(v, e, rgba(b.edge, 1.0));
        }
        let gc = [x + lean * 0.5 + bw * 0.5 + 1.0, y + ph * 0.5];
        glyph(
            v,
            b.glyph,
            gc,
            18.0,
            rgba(if b.enemy { IVORY } else { INK_DARK }, 1.0),
        );
        if !b.compact {
            let tx = x + bw + lean + 9.0;
            text(
                font,
                v,
                Face::Round,
                &b.short,
                tx,
                y + ph * 0.5 - 10.0,
                15.5,
                rgba(IVORY, 1.0),
            );
            let mw = text_width(font, Face::Display, &b.men, 14.0);
            text(
                font,
                v,
                Face::Display,
                &b.men,
                x + w - lean - 8.0 - mw,
                y + ph * 0.5 - 9.0,
                14.0,
                rgba(if b.enemy { 0xd0c4dc } else { AMBER }, 1.0),
            );
        }
        if lit {
            rim(
                v,
                q,
                2.0,
                rgba(if b.selected { AMBER } else { AMBER_HI }, 1.0),
            );
        }
        // its men left, along its foot
        let (bx, by, bw2) = (x + 2.0, y + ph + 1.0, w - lean - 2.0);
        push_rect(v, Rect::new(bx, by, bw2, 4.0), rgba(0x000000, 0.75));
        let hp_col = if b.hp < 0.33 {
            CRIMSON
        } else if b.enemy {
            b.edge
        } else {
            GOOD
        };
        push_rect(
            v,
            Rect::new(bx, by, bw2 * b.hp.clamp(0.0, 1.0), 4.0),
            rgba(hp_col, 1.0),
        );
        // what it is doing, on a chip at its right
        if b.state != Glyph::None {
            let c = [x + w + 6.0, y + ph * 0.5];
            disc(v, c, 12.0, rgba(INK_DARK, 0.95));
            glyph(
                v,
                b.state,
                c,
                14.0,
                rgba(if b.routing { 0xff8a7a } else { AMBER }, 1.0),
            );
        }
    });
}

/// A plate behind a group of the HUD's controls, with its kicker.
fn group_plate(font: &FontAtlas, v: &mut Vec<MapVertex>, r: [f32; 4], label: &str) {
    let [x, y, w, h] = r;
    let pad = 10.0;
    fill(
        v,
        slanted(
            x - pad + 6.0,
            y - pad + 8.0,
            w + pad * 2.0,
            h + pad * 2.0,
            12.0,
        ),
        rgba(0x000000, 0.4),
    );
    fill(
        v,
        slanted(x - pad, y - pad, w + pad * 2.0, h + pad * 2.0, 12.0),
        rgba(INK_DARK, 0.86),
    );
    rim(
        v,
        slanted(x - pad, y - pad, w + pad * 2.0, h + pad * 2.0, 12.0),
        1.2,
        rgba(AMBER, 0.35),
    );
    let kw = text_width(font, Face::Display, label, 14.0) + 20.0;
    fill(
        v,
        slanted(x - pad + 14.0, y - pad - 12.0, kw, 22.0, 6.0),
        rgba(AMBER, 1.0),
    );
    text(
        font,
        v,
        Face::Display,
        label,
        x - pad + 24.0,
        y - pad - 11.0,
        14.0,
        rgba(INK_DARK, 1.0),
    );
}

fn draw_card(font: &FontAtlas, v: &mut Vec<MapVertex>, c: &UnitCard, time: f32) {
    let [x, y, w, h] = c.rect;
    let lean = 9.0;
    let alpha = if c.dead { 0.38 } else { 1.0 };
    let blink = c.routing && (time * 2.2).fract() < 0.5;
    let (top, bottom) = if c.selected {
        (0x4a2c78, 0x1c1030)
    } else if blink {
        (0x5a1018, 0x200608)
    } else {
        (0x24163a, 0x120a1e)
    };
    fill(
        v,
        slanted(x + 6.0, y + 7.0, w, h, lean),
        rgba(0x000000, 0.45 * alpha),
    );
    let q = slanted(x, y, w, h, lean);
    push_triangle(v, [q[0], q[1], q[2]], rgba(top, 0.96 * alpha));
    push_triangle(v, [q[0], q[2], q[3]], rgba(bottom, 0.96 * alpha));
    push_outline_fringe(v, &q, rgba(bottom, 0.96 * alpha));
    fill(
        v,
        [
            q[0],
            [q[0][0] + 5.0, q[0][1]],
            [q[3][0] + 5.0, q[3][1]],
            q[3],
        ],
        rgba(c.edge, alpha),
    );
    if c.hover {
        rim(v, q, 2.0, rgba(AMBER_HI, 0.9));
    }
    if c.selected {
        wire(
            v,
            [q[3][0] + 6.0, q[3][1] - 2.0],
            [q[2][0], q[2][1] - 2.0],
            4.0,
            rgba(AMBER, 1.0),
            rgba(AMBER, 1.0),
            0.0,
            false,
        );
    }
    // its face: the glyph on a glow of its colour
    let fc = [x + w / 2.0 + 2.0, y + 44.0];
    halo(v, fc, 30.0, 18.0, rgba(c.edge, 0.42 * alpha));
    glyph(v, c.glyph, fc, 42.0, rgba(IVORY, alpha));
    if let Some(cd) = c.cd.filter(|cd| *cd > 0.001) {
        arc(v, fc, 30.0, 6.0, 0.0, cd, rgba(0x08040e, 0.6), [0.0; 4]);
    }
    if c.dimmed {
        wire(
            v,
            [x + 14.0, y + 20.0],
            [x + w - 6.0, y + 72.0],
            3.0,
            rgba(VIOLET_HI, 0.6 * alpha),
            rgba(VIOLET_HI, 0.0),
            0.0,
            false,
        );
    }
    text(
        font,
        v,
        Face::Display,
        &c.men,
        x + w - 12.0 - text_width(font, Face::Display, &c.men, 19.0),
        y + 4.0,
        19.0,
        rgba(AMBER, alpha),
    );
    if c.state != Glyph::None {
        let col = match c.tone {
            1 => CRIMSON,
            2 => VIOLET_HI,
            _ => AMBER,
        };
        glyph(v, c.state, [x + 22.0, y + 16.0], 18.0, rgba(col, alpha));
    }
    let size = fit(font, Face::Round, &c.short, 17.0, w - 20.0);
    let tw = text_width(font, Face::Round, &c.short, size);
    text(
        font,
        v,
        Face::Round,
        &c.short,
        x + 8.0 + (w - 8.0 - tw) / 2.0,
        y + 82.0,
        size,
        rgba(IVORY, alpha),
    );
    let mut by = y + h - 12.0;
    let bars: Vec<(f32, u32)> = [
        c.ammo.map(|a| (a, 0xd8d0c0)),
        c.spark.map(|s| (s, VIOLET_HI)),
        Some((c.morale, AMBER)),
        Some((c.hp, GOOD)),
    ]
    .into_iter()
    .flatten()
    .collect();
    for (k, col) in bars {
        bar_slant(v, x + 12.0, by, w - 22.0, 6.0, k, col);
        by -= 9.0;
    }
}

fn draw_minimap(font: &FontAtlas, v: &mut Vec<MapVertex>, m: &Minimap, deploying: bool) {
    let [x, y, w, h] = m.rect;
    // the frame round the picture (drawn under the shapes)
    let q = slanted(x - 10.0, y - 10.0, w + 20.0, h + 20.0, 12.0);
    for i in 0..4 {
        wire(
            v,
            q[i],
            q[(i + 1) % 4],
            4.0,
            rgba(INK_DARK, 1.0),
            rgba(INK_DARK, 1.0),
            0.0,
            false,
        );
    }
    rim(v, q, 1.5, rgba(AMBER, 0.85));
    kicker(font, v, &m.label, x + 4.0, y - 26.0, AMBER, 1.0);
    let to = |p: [f32; 2]| [x + p[0] * w, y + p[1] * h];
    if let (Some(z), true) = (m.zone, deploying) {
        let a = to([z[0], z[1]]);
        let b = to([z[2], z[3]]);
        for (p0, p1) in [
            ([a[0], a[1]], [b[0], a[1]]),
            ([b[0], a[1]], [b[0], b[1]]),
            ([b[0], b[1]], [a[0], b[1]]),
            ([a[0], b[1]], [a[0], a[1]]),
        ] {
            wire(
                v,
                p0,
                p1,
                1.6,
                rgba(AMBER, 0.9),
                rgba(AMBER, 0.9),
                0.0,
                false,
            );
        }
    }
    for d in &m.dots {
        let c = to(d.at);
        if d.hero {
            disc(v, c, 5.5, rgba(0x000000, 0.6));
            disc(v, c, 4.5, rgba(d.color, 1.0));
        } else {
            let half = (d.width * w / 2.0).max(3.0);
            let (s, co) = d.heading.sin_cos();
            // across the front: (cos, -sin) on the picture (z down)
            let ax = [co, -s];
            let a = [c[0] - ax[0] * half, c[1] - ax[1] * half];
            let b = [c[0] + ax[0] * half, c[1] + ax[1] * half];
            wire(
                v,
                a,
                b,
                6.0,
                rgba(0x000000, 0.55),
                rgba(0x000000, 0.55),
                0.0,
                false,
            );
            wire(
                v,
                a,
                b,
                4.5,
                rgba(d.color, 1.0),
                rgba(d.color, 1.0),
                0.0,
                false,
            );
            // a tick on its front
            let f = [s, co];
            disc(
                v,
                [c[0] + f[0] * 3.0, c[1] + f[1] * 3.0],
                1.4,
                rgba(INK_DARK, 1.0),
            );
        }
        if d.selected {
            halo(v, c, 7.0, 5.0, rgba(AMBER_HI, 0.8));
        }
    }
    if m.view.len() == 4 {
        for i in 0..4 {
            let (a, b) = (to(m.view[i]), to(m.view[(i + 1) % 4]));
            let clip = |p: [f32; 2]| [p[0].clamp(x, x + w), p[1].clamp(y, y + h)];
            wire(
                v,
                clip(a),
                clip(b),
                1.4,
                rgba(0xfff0d2, 0.85),
                rgba(0xfff0d2, 0.85),
                0.0,
                false,
            );
        }
    }
}

fn draw_tip(font: &FontAtlas, v: &mut Vec<MapVertex>, t: &Tip, width: f32, height: f32) {
    let w = 340.0;
    let lines = wrap(font, Face::RoundItalic, &t.text, 15.0, w - 50.0);
    let rows = t.rows.len().div_ceil(2);
    let h = 70.0
        + rows as f32 * 22.0
        + lines.len() as f32 * 21.0
        + if t.action.is_empty() { 8.0 } else { 30.0 };
    let x = t.at[0].clamp(12.0, width - w - 12.0);
    let y = if t.at[1] + h > height - 12.0 {
        (t.at[1] - h - 18.0).max(12.0)
    } else {
        t.at[1]
    };
    plate(v, x, y, w, h, 10.0, t.edge, 1.0);
    if !t.kicker.is_empty() {
        text(
            font,
            v,
            Face::Display,
            &t.kicker,
            x + 22.0,
            y + 10.0,
            14.0,
            rgba(MUTED, 1.0),
        );
    }
    let ts = fit(font, Face::Title, &t.title, 22.0, w - 44.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &t.title,
        x + 22.0,
        y + 30.0,
        ts,
        rgba(BONE, 1.0),
    );
    let mut yy = y + 64.0;
    for (i, (k, val)) in t.rows.iter().enumerate() {
        let cx = x + 24.0 + (i % 2) as f32 * 150.0;
        let cy = yy + (i / 2) as f32 * 22.0;
        text(font, v, Face::Round, k, cx, cy, 14.0, rgba(MUTED, 1.0));
        text(
            font,
            v,
            Face::Display,
            val,
            cx + 118.0 - text_width(font, Face::Display, val, 15.0),
            cy - 1.0,
            15.0,
            rgba(AMBER, 1.0),
        );
    }
    yy += rows as f32 * 22.0 + 4.0;
    for l in &lines {
        text(
            font,
            v,
            Face::RoundItalic,
            l,
            x + 24.0,
            yy,
            15.0,
            rgba(0xd9cce8, 1.0),
        );
        yy += 21.0;
    }
    if !t.action.is_empty() {
        text(
            font,
            v,
            Face::Round,
            &t.action,
            x + 24.0,
            yy + 4.0,
            15.0,
            rgba(AMBER, 1.0),
        );
    }
}

fn draw_results(font: &FontAtlas, v: &mut Vec<MapVertex>, r: &Results, width: f32, height: f32) {
    let k = entrance(r.age, 0.0, 0.45);
    push_rect(
        v,
        Rect::new(0.0, 0.0, width, height),
        rgba(0x06030c, 0.68 * k),
    );
    shifted(v, [0.0, (1.0 - k) * 30.0], |v| {
        faded(v, k, |v| {
            let (x, w) = (width / 2.0 - 620.0, 1240.0);
            let y = 150.0;
            let edge = if r.win { AMBER } else { CRIMSON };
            let s = [
                [x + 0.03 * w, y + 16.0],
                [x + w, y],
                [x + 0.97 * w, y + 140.0],
                [x, y + 160.0],
            ];
            fill(
                v,
                [
                    [s[0][0] + 12.0, s[0][1] + 14.0],
                    [s[1][0] + 12.0, s[1][1] + 14.0],
                    [s[2][0] + 12.0, s[2][1] + 14.0],
                    [s[3][0] + 12.0, s[3][1] + 14.0],
                ],
                rgba(0x000000, 0.5),
            );
            fill(v, s, rgba(edge, 1.0));
            let title = if r.win { "VICTORIA" } else { "DERROTA" };
            text(
                font,
                v,
                Face::Title,
                title,
                x + 0.06 * w,
                y + 22.0,
                96.0,
                rgba(INK_DARK, 1.0),
            );
            let lw = text_width(font, Face::RoundItalic, &r.label, 24.0);
            text(
                font,
                v,
                Face::RoundItalic,
                &r.label,
                x + 0.94 * w - lw,
                y + 104.0,
                24.0,
                rgba(INK_DARK, 1.0),
            );
            let ty = y + 190.0;
            for (side, (name, color, rows)) in r.sides.iter().enumerate() {
                let px = x + side as f32 * 630.0;
                let ph = 76.0 + rows.len().max(1) as f32 * 34.0;
                plate(v, px, ty, 610.0, ph, 10.0, *color, 1.0);
                kicker(font, v, name, px + 24.0, ty - 12.0, *color, 1.0);
                let heads = ["REGIMIENTO", "INICIO", "BAJAS", "MUERTES"];
                let cols = [px + 30.0, px + 380.0, px + 470.0, px + 560.0];
                for (i, hd) in heads.iter().enumerate() {
                    let hx = if i == 0 {
                        cols[0]
                    } else {
                        cols[i] - text_width(font, Face::Display, hd, 15.0)
                    };
                    text(
                        font,
                        v,
                        Face::Display,
                        hd,
                        hx,
                        ty + 22.0,
                        15.0,
                        rgba(MUTED, 1.0),
                    );
                }
                for (j, row) in rows.iter().enumerate() {
                    let ry = ty + 52.0 + j as f32 * 34.0;
                    push_rect(
                        v,
                        Rect::new(px + 26.0, ry - 4.0, 560.0, 1.0),
                        rgba(0xffffff, 0.07),
                    );
                    glyph(
                        v,
                        row.glyph,
                        [cols[0] + 9.0, ry + 10.0],
                        16.0,
                        rgba(IVORY, 1.0),
                    );
                    let ns = fit(
                        font,
                        Face::Round,
                        &row.name,
                        19.0,
                        if row.awake { 210.0 } else { 310.0 },
                    );
                    text(
                        font,
                        v,
                        Face::Round,
                        &row.name,
                        cols[0] + 24.0,
                        ry,
                        ns,
                        rgba(if row.awake { AMBER } else { IVORY }, 1.0),
                    );
                    if row.awake {
                        let nw = text_width(font, Face::Round, &row.name, ns);
                        text(
                            font,
                            v,
                            Face::RoundItalic,
                            "(despierto)",
                            cols[0] + 30.0 + nw,
                            ry,
                            15.0,
                            rgba(AMBER, 1.0),
                        );
                    }
                    let num = |v: &mut Vec<MapVertex>, s: String, cx: f32, c: u32| {
                        let tw = text_width(font, Face::Display, &s, 19.0);
                        text(font, v, Face::Display, &s, cx - tw, ry, 19.0, rgba(c, 1.0));
                    };
                    num(v, row.men0.to_string(), cols[1], IVORY);
                    num(
                        v,
                        if row.lost > 0 {
                            format!("−{}", row.lost)
                        } else {
                            "0".into()
                        },
                        cols[2],
                        if row.lost > 0 { 0xff8a7a } else { IVORY },
                    );
                    num(v, row.kills.to_string(), cols[3], IVORY);
                }
            }
            if !r.note.is_empty() {
                let rows = r.sides.iter().map(|s| s.2.len()).max().unwrap_or(1).max(1);
                let ny = ty + 96.0 + rows as f32 * 34.0;
                for (j, l) in wrap(font, Face::RoundItalic, &r.note, 21.0, 1180.0)
                    .iter()
                    .enumerate()
                {
                    text_shadow(
                        font,
                        v,
                        Face::RoundItalic,
                        l,
                        x + 24.0,
                        ny + j as f32 * 29.0,
                        21.0,
                        rgba(0xd9cce8, 1.0),
                    );
                }
            }
        });
    });
}

fn draw_help(font: &FontAtlas, v: &mut Vec<MapVertex>, width: f32, height: f32) {
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x06030c, 0.6));
    let (x, y, w, h) = (width / 2.0 - 520.0, 120.0, 1040.0, 800.0);
    plate(v, x, y, w, h, 14.0, AMBER, 1.0);
    kicker(font, v, "CONTROLES", x + 30.0, y - 12.0, AMBER, 1.0);
    text_shadow(
        font,
        v,
        Face::Title,
        "Cómo se manda un ejército",
        x + 34.0,
        y + 30.0,
        34.0,
        rgba(BONE, 1.0),
    );
    let rows: [(&str, &str); 10] = [
        ("Clic / arrastrar", "Seleccionar un regimiento o varios (Mayús añade, Ctrl+A todos)"),
        ("Clic derecho", "Mover; sobre un enemigo, atacar (doble clic derecho: corriendo)"),
        ("Arrastrar clic derecho", "Dibuja la línea: anchura y frente de la formación"),
        ("WASD · bordes", "Mover la cámara (Mayús: rápido) · Q / E girar · rueda: zoom · botón central: girar e inclinar"),
        ("Espacio · 1 2 3", "Pausa · velocidad (se pueden dar órdenes en pausa)"),
        ("H · R · G · K", "Alto · correr · disparo libre · hostigar"),
        ("F", "«Chispa»: la Reina reagrupa y reenciende a los suyos y derriba a los enemigos cercanos"),
        ("La chispa", "Los pueblos de la Reina lejos de ella se van apagando (barra violeta): lentos al obedecer y, a cero, bestias que no obedecen. Los sacerdotes la apagan antes."),
        ("Flancos", "Atacar por el flanco o la espalda baja la defensa y la moral; las cargas derriban; las lanzas quietas frenan a los caballos"),
        ("Doble clic en carta", "Centrar la cámara en el regimiento"),
    ];
    let mut yy = y + 96.0;
    for (k, d) in rows {
        text(
            font,
            v,
            Face::Display,
            k,
            x + 40.0,
            yy,
            18.0,
            rgba(AMBER, 1.0),
        );
        let lines = wrap(font, Face::Round, d, 18.0, w - 380.0);
        for (j, l) in lines.iter().enumerate() {
            text(
                font,
                v,
                Face::Round,
                l,
                x + 330.0,
                yy + j as f32 * 25.0,
                18.0,
                rgba(CREAM, 0.95),
            );
        }
        yy += 25.0 * lines.len().max(1) as f32 + 14.0;
    }
}

fn draw_battle(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    b: &BattleView,
    width: f32,
    height: f32,
    time: f32,
) {
    for tag in &b.banners {
        draw_banner(font, v, tag, time);
    }
    // the title
    let tw = text_width(font, Face::Title, &b.title, 32.0).max(text_width(
        font,
        Face::RoundItalic,
        &b.subtitle,
        18.0,
    )) + 80.0;
    plate(
        v,
        24.0,
        22.0,
        tw.clamp(360.0, 760.0),
        104.0,
        12.0,
        AMBER,
        1.0,
    );
    kicker(font, v, &b.kicker, 44.0, 10.0, AMBER, 1.0);
    let ts = fit(font, Face::Title, &b.title, 32.0, 680.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &b.title,
        48.0,
        40.0,
        ts,
        rgba(BONE, 1.0),
    );
    let ss = fit(font, Face::RoundItalic, &b.subtitle, 18.0, 680.0);
    text(
        font,
        v,
        Face::RoundItalic,
        &b.subtitle,
        48.0,
        86.0,
        ss,
        rgba(0xc8b8d8, 1.0),
    );
    // the balance of power: each side on a plate of its colour, the bar between, the clock under it
    let (bx, bw) = (width / 2.0 - 340.0, 680.0);
    let k = b.balance.clamp(0.0, 1.0);
    let side_plate =
        |v: &mut Vec<MapVertex>, name: &str, x: f32, right: bool, col: u32, share: f32| {
            let nw = text_width(font, Face::Display, name, 18.0);
            let pw = nw + 70.0;
            let px = if right { x - pw } else { x };
            fill(
                v,
                slanted(px + 4.0, 20.0, pw, 32.0, 9.0),
                rgba(0x000000, 0.5),
            );
            fill(v, slanted(px, 16.0, pw, 32.0, 9.0), rgba(col, 1.0));
            let pct = format!("{}%", (share * 100.0).round());
            let cw = text_width(font, Face::Display, &pct, 16.0);
            if right {
                text(
                    font,
                    v,
                    Face::Display,
                    name,
                    px + pw - 18.0 - nw,
                    21.0,
                    18.0,
                    rgba(INK_DARK, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Display,
                    &pct,
                    px + 16.0,
                    22.0,
                    16.0,
                    rgba(INK_DARK, 0.75),
                );
            } else {
                text(
                    font,
                    v,
                    Face::Display,
                    name,
                    px + 18.0,
                    21.0,
                    18.0,
                    rgba(INK_DARK, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Display,
                    &pct,
                    px + pw - 16.0 - cw,
                    22.0,
                    16.0,
                    rgba(INK_DARK, 0.75),
                );
            }
        };
    side_plate(v, &b.names[0], bx, false, b.colors[0], k);
    side_plate(v, &b.names[1], bx + bw, true, b.colors[1], 1.0 - k);
    let (gy, gh) = (56.0, 16.0);
    fill(
        v,
        slanted(bx + 4.0, gy + 4.0, bw, gh, 7.0),
        rgba(0x000000, 0.5),
    );
    fill(v, slanted(bx, gy, bw, gh, 7.0), rgba(INK_DARK, 0.95));
    fill(
        v,
        slanted(bx, gy, bw * k, gh, 7.0),
        rgba(mix(0x5a2aa0, VIOLET, 0.6), 1.0),
    );
    fill(
        v,
        slanted(bx + bw * k, gy, bw * (1.0 - k), gh, 7.0),
        rgba(mix(b.darks[1], b.colors[1], 0.55), 1.0),
    );
    let m = [bx + bw * k, gy + gh / 2.0];
    fill(
        v,
        [
            [m[0], m[1] - 14.0],
            [m[0] + 9.0, m[1]],
            [m[0], m[1] + 14.0],
            [m[0] - 9.0, m[1]],
        ],
        rgba(AMBER, 1.0),
    );
    fill(
        v,
        [
            [m[0], m[1] - 8.0],
            [m[0] + 5.0, m[1]],
            [m[0], m[1] + 8.0],
            [m[0] - 5.0, m[1]],
        ],
        rgba(INK_DARK, 1.0),
    );
    let cw = text_width(font, Face::Display, &b.clock, 18.0) + 54.0;
    let cx = width / 2.0 - cw / 2.0;
    fill(
        v,
        slanted(cx, 80.0, cw, 28.0, 7.0),
        rgba(if b.deploying { AMBER } else { INK_DARK }, 0.95),
    );
    glyph(
        v,
        Glyph::Hourglass,
        [cx + 20.0, 94.0],
        13.0,
        rgba(if b.deploying { INK_DARK } else { AMBER }, 1.0),
    );
    text(
        font,
        v,
        Face::Display,
        &b.clock,
        cx + 34.0,
        83.0,
        18.0,
        rgba(if b.deploying { INK_DARK } else { 0xe8dcf4 }, 1.0),
    );
    // the messages, newest on top
    for (i, (msg, edge, age)) in b.messages.iter().take(5).enumerate() {
        let a = (age / 0.3).clamp(0.0, 1.0) * (1.0 - ((age - 6.5) / 0.8).clamp(0.0, 1.0));
        if a <= 0.01 {
            continue;
        }
        let y = 150.0 + i as f32 * 42.0;
        let slide = (1.0 - (age / 0.3).clamp(0.0, 1.0)) * -30.0;
        // the older, the quieter
        let a = a * (1.0 - i as f32 * 0.12);
        shifted(v, [slide, 0.0], |v| {
            let mw = (text_width(font, Face::Round, msg, 17.5) + 54.0).min(680.0);
            plate(v, 24.0, y, mw, 34.0, 8.0, *edge, a);
            let ms = fit(font, Face::Round, msg, 17.5, mw - 44.0);
            text(font, v, Face::Round, msg, 46.0, y + 6.0, ms, rgba(IVORY, a));
        });
    }
    if let Some(m) = &b.minimap {
        draw_minimap(font, v, m, b.deploying);
    }
    if b.results.is_none() {
        if let Some(r) = b.speed_group {
            group_plate(font, v, r, "TIEMPO");
        }
        if let Some(r) = b.orders_group {
            group_plate(font, v, r, "ÓRDENES");
        }
    }
    for c in &b.cards {
        draw_card(font, v, c, time);
    }
    if let Some((c, r, ready, cd, dead)) = b.spark {
        let alpha = if dead { 0.4 } else { 1.0 };
        disc(v, [c[0] + 8.0, c[1] + 10.0], r + 6.0, rgba(0x000000, 0.45));
        disc(v, c, r + 6.0, rgba(AMBER, alpha));
        disc(v, c, r + 4.0, rgba(INK_DARK, alpha));
        disc(v, c, r, rgba(0x5a2aa0, alpha));
        disc(v, [c[0], c[1] - r * 0.15], r * 0.72, rgba(0x8a5ae0, alpha));
        if ready {
            halo(
                v,
                c,
                r + 6.0,
                22.0,
                rgba(VIOLET_HI, 0.45 + 0.25 * wave(time, 1.2)),
            );
        }
        if cd > 0.001 {
            arc(v, c, r - 8.0, 16.0, 0.0, cd, rgba(0x08040e, 0.62), [0.0; 4]);
        }
        glyph(
            v,
            Glyph::Spark,
            [c[0], c[1] - 14.0],
            34.0,
            rgba(0xffffff, alpha),
        );
        let lw = text_width(font, Face::Round, "CHISPA", 16.0);
        text(
            font,
            v,
            Face::Round,
            "CHISPA",
            c[0] - lw / 2.0 - 12.0,
            c[1] + 16.0,
            16.0,
            rgba(0xffffff, alpha),
        );
        keycap(
            font,
            v,
            Rect::new(c[0] + lw / 2.0 - 6.0, c[1] + 14.0, 26.0, 24.0),
            "F",
            14.0,
            ready,
            dead,
            AMBER,
            alpha,
        );
    }
    if let Some(s) = b.selbox {
        push_rect(v, r4(s), rgba(AMBER, 0.1));
        let (x0, y0, x1, y1) = (s[0], s[1], s[0] + s[2], s[1] + s[3]);
        for (a, c) in [
            ([x0, y0], [x1, y0]),
            ([x1, y0], [x1, y1]),
            ([x1, y1], [x0, y1]),
            ([x0, y1], [x0, y0]),
        ] {
            wire(v, a, c, 1.5, rgba(AMBER, 1.0), rgba(AMBER, 1.0), 0.0, false);
        }
    }
    if b.deploying {
        let (w, x, y) = (940.0, width / 2.0 - 470.0, 108.0);
        plate(v, x, y, w, 84.0, 12.0, AMBER, 1.0);
        kicker(font, v, "DESPLIEGUE", x + 30.0, y - 12.0, AMBER, 1.0);
        let room = w - 400.0;
        let t = "Coloca tus regimientos";
        let ts = fit(font, Face::Title, t, 26.0, room);
        text_shadow(
            font,
            v,
            Face::Title,
            t,
            x + 34.0,
            y + 12.0,
            ts,
            rgba(BONE, 1.0),
        );
        let d = "Selecciona y usa el clic derecho en la zona marcada (arrastra para la anchura y el frente).";
        let lines = wrap(font, Face::RoundItalic, d, 15.0, room);
        for (n, l) in lines.iter().take(2).enumerate() {
            text(
                font,
                v,
                Face::RoundItalic,
                l,
                x + 36.0,
                y + 46.0 + n as f32 * 18.0,
                15.0,
                rgba(0xd9cce8, 1.0),
            );
        }
    } else if b.paused && b.results.is_none() {
        let t = "PAUSA";
        let w = text_width(font, Face::Display, t, 24.0) + 120.0;
        let x = width / 2.0 - w / 2.0;
        fill(v, slanted(x, 106.0, w, 40.0, 10.0), rgba(AMBER, 1.0));
        text(
            font,
            v,
            Face::Display,
            t,
            x + 60.0,
            112.0,
            24.0,
            rgba(INK_DARK, 1.0),
        );
    }
    let _ = height;
}

fn draw_confirm(font: &FontAtlas, v: &mut Vec<MapVertex>, width: f32, height: f32) {
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x06030c, 0.55));
    let (x, y, w, h) = (width / 2.0 - 380.0, 380.0, 760.0, 230.0);
    plate(v, x, y, w, h, 12.0, CRIMSON, 1.0);
    kicker(font, v, "RETIRADA", x + 30.0, y - 12.0, CRIMSON, 1.0);
    text_shadow(
        font,
        v,
        Face::Title,
        "¿Retirarse de la batalla?",
        x + 36.0,
        y + 30.0,
        30.0,
        rgba(BONE, 1.0),
    );
    text(
        font,
        v,
        Face::RoundItalic,
        "Se salvan los que siguen en pie.",
        x + 38.0,
        y + 82.0,
        18.0,
        rgba(0xd9cce8, 1.0),
    );
}

// ------------------------------------------------------------------ the campaign
fn draw_campaign(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    c: &CampaignView,
    width: f32,
    height: f32,
    time: f32,
) {
    // the marches: a dotted way through the provinces, the turn each stretch ends in, the goal
    for m in &c.marches {
        let (color, alpha, width) = match m.kind {
            2 => (AMBER, 1.0, 3.2),
            3 => (IVORY, 0.85, 2.6),
            _ => (AMBER, 0.45, 2.2),
        };
        for seg in m.points.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = dx.hypot(dy);
            if len < 4.0 {
                continue;
            }
            let (ux, uy) = (dx / len, dy / len);
            let mut t = (time * 22.0) % 18.0;
            while t + 8.0 < len {
                let p0 = [a[0] + ux * t, a[1] + uy * t];
                let p1 = [a[0] + ux * (t + 8.0), a[1] + uy * (t + 8.0)];
                wire(
                    v,
                    p0,
                    p1,
                    width + 2.6,
                    rgba(INK_DARK, 0.55 * alpha),
                    rgba(INK_DARK, 0.55 * alpha),
                    0.0,
                    false,
                );
                wire(
                    v,
                    p0,
                    p1,
                    width,
                    rgba(color, alpha),
                    rgba(color, alpha),
                    0.0,
                    false,
                );
                t += 18.0;
            }
        }
        for (k, (turn, at)) in m.turns.iter().enumerate() {
            let last = k + 1 == m.turns.len();
            let r = if last { 17.0 } else { 13.0 };
            disc(v, *at, r + 2.5, rgba(INK_DARK, 0.9 * alpha));
            disc(v, *at, r, rgba(if last { color } else { INK3 }, alpha));
            if last {
                glyph(v, Glyph::Flag, *at, 18.0, rgba(INK_DARK, alpha));
            } else {
                let n = turn.to_string();
                let nw = text_width(font, Face::Display, &n, 16.0);
                text(
                    font,
                    v,
                    Face::Display,
                    &n,
                    at[0] - nw / 2.0,
                    at[1] - 11.0,
                    16.0,
                    rgba(color, alpha),
                );
            }
        }
    }
    // the chosen army's routes: dashes that march toward each destination
    for &(a, b, color, lit) in &c.routes {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        if len < 30.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (dash, step) = (12.0, 22.0);
        let alpha = if lit { 1.0 } else { 0.55 };
        let mut t = (time * 30.0) % step;
        while t + dash < len - 26.0 {
            let p0 = [a[0] + ux * t, a[1] + uy * t];
            let p1 = [a[0] + ux * (t + dash), a[1] + uy * (t + dash)];
            wire(
                v,
                p0,
                p1,
                if lit { 6.0 } else { 5.0 },
                rgba(INK_DARK, 0.6 * alpha),
                rgba(INK_DARK, 0.6 * alpha),
                0.0,
                false,
            );
            wire(
                v,
                p0,
                p1,
                if lit { 3.4 } else { 2.6 },
                rgba(color, alpha),
                rgba(color, alpha),
                0.0,
                false,
            );
            t += step;
        }
        // the arrowhead, just short of the chip
        let tip = [b[0] - ux * 24.0, b[1] - uy * 24.0];
        let (nx, ny) = (-uy, ux);
        let back = [tip[0] - ux * 16.0, tip[1] - uy * 16.0];
        fill(
            v,
            [
                tip,
                [back[0] + nx * 9.0, back[1] + ny * 9.0],
                [back[0] - nx * 9.0, back[1] - ny * 9.0],
                tip,
            ],
            rgba(color, alpha),
        );
    }
    for l in &c.labels {
        faded(v, l.alpha, |v| {
            let s = l.scale;
            let ns = 24.0 * s;
            let cx = l.rect[0] + l.rect[2] / 2.0;
            let w = text_width(font, Face::Title, &l.name, ns);
            text_outline(
                font,
                v,
                Face::Title,
                &l.name,
                cx - w / 2.0,
                l.rect[1],
                ns,
                rgba(BONE, 1.0),
                rgba(0x000000, 0.9),
            );
            let os = 15.0 * s;
            let ow = text_width(font, Face::Display, &l.owner, os);
            text_outline(
                font,
                v,
                Face::Display,
                &l.owner,
                cx - ow / 2.0,
                l.rect[1] + ns * 1.25,
                os,
                rgba(l.color, 1.0),
                rgba(0x000000, 0.85),
            );
        });
    }
    for a in &c.armies {
        faded(v, a.alpha, |v| {
            let w = text_width(font, Face::Round, &a.name, 17.0)
                + text_width(font, Face::Display, &a.men, 17.0)
                + 60.0
                + if a.moves.is_some() { 30.0 } else { 0.0 }
                + if a.marching { 22.0 } else { 0.0 };
            let (x, y) = (a.rect[0] + a.rect[2] / 2.0 - w / 2.0, a.rect[1]);
            // the pole down to its figure, when the plate had to stand aside
            let [p0, p1] = a.pole;
            if p1[1] - (y + 30.0) > 6.0 {
                let p0 = [p0[0], y + 30.0];
                wire(
                    v,
                    p0,
                    p1,
                    4.0,
                    rgba(INK_DARK, 0.7),
                    rgba(INK_DARK, 0.2),
                    0.0,
                    false,
                );
                wire(
                    v,
                    p0,
                    p1,
                    2.0,
                    rgba(a.edge, 0.95),
                    rgba(a.edge, 0.35),
                    0.0,
                    false,
                );
            }
            let q = slanted(x, y, w, 30.0, 6.0);
            fill(
                v,
                slanted(x + 3.0, y + 4.0, w, 30.0, 6.0),
                rgba(0x000000, 0.5),
            );
            push_triangle(
                v,
                [q[0], q[1], q[2]],
                rgba(if a.selected { 0x5a3490 } else { 0x1e122e }, 0.94),
            );
            push_triangle(v, [q[0], q[2], q[3]], rgba(INK_DARK, 0.94));
            push_outline_fringe(v, &q, rgba(INK_DARK, 0.94));
            fill(
                v,
                [
                    q[0],
                    [q[0][0] + 4.0, q[0][1]],
                    [q[3][0] + 4.0, q[3][1]],
                    q[3],
                ],
                rgba(a.edge, 1.0),
            );
            if a.selected {
                rim(v, q, 2.0, rgba(AMBER, 1.0));
            }
            glyph(v, a.glyph, [x + 20.0, y + 15.0], 18.0, rgba(IVORY, 1.0));
            text(
                font,
                v,
                Face::Round,
                &a.name,
                x + 34.0,
                y + 4.0,
                17.0,
                rgba(IVORY, 1.0),
            );
            let nw = text_width(font, Face::Round, &a.name, 17.0);
            text(
                font,
                v,
                Face::Display,
                &a.men,
                x + 42.0 + nw,
                y + 4.0,
                17.0,
                rgba(AMBER, 1.0),
            );
            if a.marching {
                let fx = x + w - if a.moves.is_some() { 48.0 } else { 18.0 };
                glyph(v, Glyph::Flag, [fx, y + 15.0], 15.0, rgba(AMBER, 1.0));
            }
            if let Some((mp, max)) = a.moves {
                for k in 0..max {
                    let cx = x + w - 26.0 + k as f32 * 11.0;
                    if k < mp {
                        disc(v, [cx, y + 15.0], 4.5, rgba(AMBER, 1.0));
                    } else {
                        arc(
                            v,
                            [cx, y + 15.0],
                            3.5,
                            1.6,
                            0.0,
                            1.0,
                            rgba(AMBER, 0.8),
                            [0.0; 4],
                        );
                    }
                }
            }
        });
    }
    // the resources and the turn
    let mut x = 24.0;
    for (what, value, delta, neg, g, edge) in &c.resources {
        plate(v, x, 20.0, 210.0, 80.0, 10.0, *edge, 1.0);
        text(
            font,
            v,
            Face::Display,
            what,
            x + 24.0,
            26.0,
            16.0,
            rgba(MUTED, 1.0),
        );
        glyph(v, *g, [x + 34.0, 72.0], 24.0, rgba(AMBER, 1.0));
        text(
            font,
            v,
            Face::Display,
            value,
            x + 54.0,
            50.0,
            30.0,
            rgba(BONE, 1.0),
        );
        let dw = text_width(font, Face::Display, delta, 18.0);
        text(
            font,
            v,
            Face::Display,
            delta,
            x + 192.0 - dw,
            60.0,
            18.0,
            rgba(if *neg { 0xff8a7a } else { GOOD }, 1.0),
        );
        x += 222.0;
    }
    plate(v, x + 10.0, 20.0, 380.0, 80.0, 10.0, AMBER, 1.0);
    text(
        font,
        v,
        Face::Display,
        &c.turn,
        x + 36.0,
        26.0,
        16.0,
        rgba(MUTED, 1.0),
    );
    let ss = fit(font, Face::Title, &c.season, 28.0, 330.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &c.season,
        x + 36.0,
        50.0,
        ss,
        rgba(BONE, 1.0),
    );
    if !c.objective.is_empty() {
        let kw = text_width(font, Face::Display, "OBJETIVO", 15.0) + 22.0;
        let ow = text_width(font, Face::Round, &c.objective, 17.0) + kw + 34.0;
        let ox = 24.0;
        fill(
            v,
            slanted(ox + 4.0, 114.0, ow, 32.0, 8.0),
            rgba(0x000000, 0.45),
        );
        fill(v, slanted(ox, 110.0, ow, 32.0, 8.0), rgba(INK_DARK, 0.92));
        fill(v, slanted(ox, 110.0, kw + 8.0, 32.0, 8.0), rgba(AMBER, 1.0));
        text(
            font,
            v,
            Face::Display,
            "OBJETIVO",
            ox + 14.0,
            114.0,
            15.0,
            rgba(INK_DARK, 1.0),
        );
        text(
            font,
            v,
            Face::Round,
            &c.objective,
            ox + kw + 22.0,
            114.0,
            17.0,
            rgba(IVORY, 1.0),
        );
    }
    // the log
    for (i, (line, edge)) in c.log.iter().enumerate() {
        let y = height - 258.0 - i as f32 * 44.0;
        let ls = fit(font, Face::Round, line, 17.0, 560.0);
        plate(v, width - 640.0, y, 616.0, 36.0, 8.0, *edge, 0.92);
        text(
            font,
            v,
            Face::Round,
            line,
            width - 614.0,
            y + 7.0,
            ls,
            rgba(IVORY, 0.95),
        );
    }
    // the panel of what is chosen
    let p = &c.panel;
    let [px, py, pw, ph] = p.rect;
    plate(v, px, py, pw, ph, 12.0, p.edge, 1.0);
    kicker(font, v, &p.kicker, px + 28.0, py - 12.0, p.edge, 1.0);
    let ts = fit(font, Face::Title, &p.title, 30.0, pw - 60.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &p.title,
        px + 32.0,
        py + 22.0,
        ts,
        rgba(BONE, 1.0),
    );
    let mut yy = py + 64.0;
    for l in wrap(font, Face::RoundItalic, &p.sub, 18.0, pw - 64.0) {
        text(
            font,
            v,
            Face::RoundItalic,
            &l,
            px + 32.0,
            yy,
            18.0,
            rgba(0xc8b8d8, 1.0),
        );
        yy += 25.0;
    }
    if !p.stats.is_empty() {
        let mut sx = px + 34.0;
        for (g, s) in &p.stats {
            glyph(v, *g, [sx + 9.0, yy + 13.0], 18.0, rgba(AMBER, 1.0));
            text(
                font,
                v,
                Face::Round,
                s,
                sx + 22.0,
                yy + 2.0,
                18.0,
                rgba(IVORY, 1.0),
            );
            sx += 48.0 + text_width(font, Face::Round, s, 18.0);
        }
        yy += 34.0;
    }
    if !p.units.is_empty() {
        // three to a row, all one width
        let cw = (pw - 64.0 - 12.0) / 3.0;
        for (k, (g, name, n)) in p.units.iter().enumerate() {
            let (ux, uy) = (
                px + 32.0 + (k % 3) as f32 * (cw + 6.0),
                yy + 4.0 + (k / 3) as f32 * 34.0,
            );
            fill(v, slanted(ux, uy, cw, 30.0, 5.0), rgba(INK3, 1.0));
            glyph(v, *g, [ux + 16.0, uy + 15.0], 17.0, rgba(IVORY, 1.0));
            let nw = text_width(font, Face::Display, n, 16.0);
            let ns = fit(font, Face::Round, name, 16.0, cw - 46.0 - nw);
            text(
                font,
                v,
                Face::Round,
                name,
                ux + 30.0,
                uy + 5.0,
                ns,
                rgba(IVORY, 1.0),
            );
            text(
                font,
                v,
                Face::Display,
                n,
                ux + cw - 10.0 - nw,
                uy + 5.0,
                16.0,
                rgba(AMBER, 1.0),
            );
        }
        yy += 4.0 + p.units.len().div_ceil(3) as f32 * 34.0 + 6.0;
    }
    for (note, color) in &p.notes {
        for l in wrap(font, Face::RoundItalic, note, 17.0, pw - 64.0) {
            text(
                font,
                v,
                Face::RoundItalic,
                &l,
                px + 32.0,
                yy,
                17.0,
                rgba(*color, 1.0),
            );
            yy += 23.0;
        }
        yy += 4.0;
    }
    if !c.hint.is_empty() {
        let hw = text_width(font, Face::Display, &c.hint, 16.0);
        text_outline(
            font,
            v,
            Face::Display,
            &c.hint,
            width / 2.0 - hw / 2.0,
            height - 32.0,
            16.0,
            rgba(0xc8b8d8, 1.0),
            rgba(0x000000, 0.8),
        );
    }
    if let Some((title, line, age)) = &c.toast {
        let a = (age / 0.25).clamp(0.0, 1.0) * (1.0 - ((age - 2.2) / 0.4).clamp(0.0, 1.0));
        if a > 0.01 {
            let w = (text_width(font, Face::Round, line, 21.0) + 80.0).max(460.0);
            let x = width / 2.0 - w / 2.0;
            plate(v, x, 136.0, w, 76.0, 10.0, AMBER, a);
            kicker(font, v, title, x + 28.0, 122.0, AMBER, a);
            text(
                font,
                v,
                Face::Round,
                line,
                x + 34.0,
                160.0,
                21.0,
                rgba(IVORY, a),
            );
        }
    }
    let _ = time;
}

fn draw_modal(font: &FontAtlas, v: &mut Vec<MapVertex>, m: &Modal, width: f32, height: f32) {
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x06030c, 0.6));
    let [x, y, w, h] = m.rect;
    plate(v, x, y, w, h, 14.0, m.edge, 1.0);
    kicker(font, v, &m.kicker, x + 30.0, y - 12.0, m.edge, 1.0);
    let ts = fit(font, Face::Title, &m.title, 34.0, w - 70.0);
    text_shadow(
        font,
        v,
        Face::Title,
        &m.title,
        x + 36.0,
        y + 30.0,
        ts,
        rgba(BONE, 1.0),
    );
    let mut yy = y + 86.0;
    for p in &m.text {
        for l in wrap(font, Face::Round, p, 21.0, w - 80.0) {
            text(
                font,
                v,
                Face::Round,
                &l,
                x + 38.0,
                yy,
                21.0,
                rgba(CREAM, 0.95),
            );
            yy += 30.0;
        }
        yy += 10.0;
    }
    if let Some(vs) = &m.versus {
        let half = (w - 140.0) / 2.0;
        for side in 0..2 {
            let sx = x + 38.0 + side as f32 * (half + 64.0);
            text_shadow(
                font,
                v,
                Face::Title,
                &vs.names[side],
                sx,
                yy,
                22.0,
                rgba(vs.colors[side], 1.0),
            );
            for (j, (g, name, n)) in vs.units[side].iter().enumerate() {
                let ry = yy + 38.0 + j as f32 * 28.0;
                glyph(v, *g, [sx + 9.0, ry + 11.0], 17.0, rgba(IVORY, 1.0));
                text(
                    font,
                    v,
                    Face::Round,
                    name,
                    sx + 26.0,
                    ry,
                    18.0,
                    rgba(IVORY, 1.0),
                );
                let nw = text_width(font, Face::Display, n, 17.0);
                text(
                    font,
                    v,
                    Face::Display,
                    n,
                    sx + half - nw,
                    ry,
                    17.0,
                    rgba(AMBER, 1.0),
                );
            }
        }
        let vsy = yy + 30.0;
        text_outline(
            font,
            v,
            Face::Title,
            "VS",
            x + 38.0 + half + 12.0,
            vsy,
            30.0,
            rgba(AMBER, 1.0),
            rgba(0x000000, 0.8),
        );
        let rows = vs.units[0].len().max(vs.units[1].len());
        let oy = yy + 48.0 + rows as f32 * 28.0;
        fill(
            v,
            slanted(x + 38.0, oy, w - 76.0, 14.0, 5.0),
            rgba(0x000000, 0.7),
        );
        fill(
            v,
            slanted(
                x + 38.0,
                oy,
                (w - 76.0) * vs.odds.clamp(0.0, 1.0),
                14.0,
                5.0,
            ),
            rgba(VIOLET, 1.0),
        );
        text(
            font,
            v,
            Face::RoundItalic,
            &vs.note,
            x + 38.0,
            oy + 22.0,
            17.0,
            rgba(0xc8b8d8, 1.0),
        );
    }
}

/// The minimap's picture under the HUD (drawn before the shapes).
pub(crate) fn pictures(view: &WarView, width: f32, height: f32) -> Vec<PictureRun> {
    let mut runs = Vec::new();
    let Some(m) = view.battle.as_ref().and_then(|b| b.minimap.as_ref()) else {
        return runs;
    };
    if !view.minimap_ready || view.battle.as_ref().is_some_and(|b| b.results.is_some()) {
        return runs;
    }
    let (sx, sy) = (width / 1920.0, height / 1080.0);
    let [x, y, w, h] = m.rect;
    let (x0, y0, x1, y1) = (x * sx, y * sy, (x + w) * sx, (y + h) * sy);
    let lean = 10.0 * sx;
    let p = [
        [x0 + lean, y0],
        [x1 + lean, y0],
        [x1 - lean, y1],
        [x0 - lean, y1],
    ];
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut vs = Vec::with_capacity(6);
    for i in [0usize, 1, 2, 0, 2, 3] {
        vs.push(MapVertex {
            position: p[i],
            uv: uv[i],
            color: [1.0; 4],
            text: 15.0,
            ..MapVertex::BLANK
        });
    }
    runs.push(PictureRun {
        image: MINIMAP.to_owned(),
        over: false,
        vertices: vs,
    });
    runs
}

/// The whole war's HUD.
pub(crate) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &WarView,
) {
    let time = view.time;
    let pointer = view.pointer;
    let hovered = |b: &WarButton| pointer.is_some_and(|p| b.contains(p[0], p[1]));
    let modal_open = view.buttons.iter().any(|b| b.layer == 1);
    if let Some(t) = &view.table {
        draw_table(font, v, t, &view.buttons, pointer, height, time);
    }
    if let Some(b) = &view.battle {
        draw_battle(font, v, b, width, height, time);
    }
    if let Some(c) = &view.campaign {
        draw_campaign(font, v, c, width, height, time);
    }
    for b in view.buttons.iter().filter(|b| b.layer == 0) {
        button(font, v, b, hovered(b) && !modal_open, time);
    }
    if let Some(t) = &view.tip {
        if !modal_open {
            draw_tip(font, v, t, width, height);
        }
    }
    if let Some(b) = &view.battle {
        if b.help {
            draw_help(font, v, width, height);
        }
        if b.confirm {
            draw_confirm(font, v, width, height);
        }
        if let Some(r) = &b.results {
            draw_results(font, v, r, width, height);
        }
    }
    if let Some(m) = view.campaign.as_ref().and_then(|c| c.modal.as_ref()) {
        draw_modal(font, v, m, width, height);
    }
    for b in view.buttons.iter().filter(|b| b.layer == 1) {
        button(font, v, b, hovered(b), time);
    }
    if let Some(p) = pointer {
        let _ = p;
    }
}
