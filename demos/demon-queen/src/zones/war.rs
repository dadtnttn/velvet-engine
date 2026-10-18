//! La guerra de la chispa en el juego: la mesa de guerra del castillo abre la campaña (el
//! continente por turnos, sus ejércitos, el oro, la comida y la chispa) y las batallas de cientos
//! de soldados por bando, en tiempo real, del prototipo de Three.js (`demon-queen-war-threejs`).
//! Aquí viven el ojo del general (la cámara), lo que hacen el ratón y las teclas, el paso de la
//! batalla y de la campaña, sus avisos y sonidos, y la vista que el HUD dibuja; las reglas, la
//! simulación y las mallas, en `demon-queen-war`.
use std::collections::VecDeque;

use super::map_overlay::{
    ArmyTag, BannerTag, BattleView, CampPanel, CampaignView, Glyph, MarchView, MiniDot, Minimap,
    Modal, Objective, ProvLabel, ResultRow, Results, TableCard, TableView, Tip, UnitCard, Versus,
    WarButton, WarHit, WarView,
};
use super::*;
use demon_queen_dungeon::i18n::tr;
use demon_queen_war as wr;
use wr::battle::{ArmySetup, Battle, LineSlot, Setup, SetupUnit};
use wr::data::{self as wd, Faction, Role, PROVINCES, SCENARIOS};
use wr::rules::{
    BattleResult, Campaign, LogKind, MoveResult, PendingBattle, Phase as CampPhase, Step,
};
use wr::sim::{Event, RState};

const SCREEN_W: f32 = 1920.0;
const SCREEN_H: f32 = 1080.0;
const SEASONS: [&str; 4] = ["Primavera", "Verano", "Otoño", "Invierno"];

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The war map 2.0 (`data/guerra`, exported by the prototype's `export_war_game.py`), put in place
/// once before the campaign's land is built; without it the land is the procedural one.
pub(crate) fn install_war_map() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = exploration3d_state::data_dir().join("guerra");
        match (
            std::fs::read(dir.join("mapa.bin")),
            std::fs::read_to_string(dir.join("mapa.json")),
        ) {
            (Ok(bin), Ok(json)) => match wr::campaign_map::MapData::parse(&bin, &json) {
                Ok(m) => {
                    eprintln!(
                        "[guerra] the war map 2.0: {}x{} land, {} trees",
                        m.nx,
                        m.nz,
                        m.trees.len()
                    );
                    wr::campaign_map::install(m);
                }
                Err(e) => eprintln!("[guerra] the war map 2.0 did not read: {e}"),
            },
            _ => eprintln!("[guerra] no war map 2.0 in data/guerra: the land of before"),
        }
    });
}

// ------------------------------------------------------------------ the general's eye
/// Pans over the field (WASD, arrows, the screen's edges; the middle button turns and tilts), Q/E
/// turn, the wheel zooms from shoulder height up to a hawk's view; the closer, the flatter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RtsCam {
    pub focus: Vec3,
    goal: [f32; 5],
    pub yaw: f32,
    pub dist: f32,
    pub tilt: f32,
    pub position: Vec3,
    look_y: f32,
    pub min_d: f32,
    pub max_d: f32,
    /// Where the focus may go: x0, x1, z0, z1.
    pub bounds: [f32; 4],
    pub fov: f32,
    /// The pitch below the horizon close up and far out, and the distances it eases between.
    pub pitch_near: f32,
    pub pitch_far: f32,
    pub pitch_span: [f32; 2],
}

impl RtsCam {
    pub(crate) fn new(bounds: [f32; 4], min_d: f32, max_d: f32, fov: f32) -> Self {
        Self {
            focus: Vec3::new(0.0, 0.0, 90.0),
            goal: [0.0, 90.0, 0.0, 120.0, 0.0],
            yaw: 0.0,
            dist: 120.0,
            tilt: 0.0,
            position: Vec3::new(0.0, 60.0, 190.0),
            look_y: 0.0,
            min_d,
            max_d,
            bounds,
            fov,
            pitch_near: 0.36,
            pitch_far: 1.06,
            pitch_span: [10.0, 280.0],
        }
    }

    /// The web map's campaign camera (Total War): close, low towards the horizon; far, a map's
    /// top view.
    pub(crate) fn campaign(bounds: [f32; 4]) -> Self {
        let mut c = Self::new(bounds, 9.0, 175.0, 42.0);
        c.pitch_near = 0.42;
        c.pitch_far = 1.12;
        c.pitch_span = [9.0, 150.0];
        c
    }

    pub(crate) fn pitch(&self) -> f32 {
        let t = smooth(self.pitch_span[0], self.pitch_span[1], self.dist);
        (self.pitch_near + (self.pitch_far - self.pitch_near) * t + self.tilt).clamp(0.2, 1.38)
    }

    /// Looks at (x, z), at once or gliding.
    pub(crate) fn frame(
        &mut self,
        x: f32,
        z: f32,
        dist: Option<f32>,
        yaw: Option<f32>,
        instant: bool,
        ground: &dyn Fn(f32, f32) -> f32,
    ) {
        self.goal[0] = x;
        self.goal[1] = z;
        if let Some(d) = dist {
            self.goal[3] = d.clamp(self.min_d, self.max_d);
        }
        if let Some(y) = yaw {
            self.goal[2] = y;
        }
        if instant {
            self.focus.x = x;
            self.focus.z = z;
            self.dist = self.goal[3];
            self.yaw = self.goal[2];
            self.focus.y = ground(x, z);
            self.place(ground);
        }
    }

    /// The wheel, in the browser's pixels (a notch is about 100).
    pub(crate) fn zoom(&mut self, delta: f32) {
        self.goal[3] = (self.goal[3] * 1.0012f32.powf(delta)).clamp(self.min_d, self.max_d);
    }

    pub(crate) fn drag(&mut self, dx: f32, dy: f32) {
        self.goal[2] -= dx * 0.005;
        self.goal[4] = (self.goal[4] + dy * 0.003).clamp(-0.25, 0.35);
    }

    pub(crate) fn goal_dist(&self) -> f32 {
        self.goal[3]
    }

    /// One step: `pan` (right, forward) from the keys and the edges, `turn` from Q/E.
    pub(crate) fn update(
        &mut self,
        dt: f32,
        pan: (f32, f32),
        turn: f32,
        fast: bool,
        ground: &dyn Fn(f32, f32) -> f32,
    ) {
        self.goal[2] += turn * dt * 1.5;
        let (px, pz) = pan;
        if px != 0.0 || pz != 0.0 {
            let l = px.hypot(pz);
            let speed = (18.0 + self.dist * 0.95) * if fast { 2.0 } else { 1.0 };
            let (fx, fz) = (-self.yaw.sin(), -self.yaw.cos());
            let (rx, rz) = (self.yaw.cos(), -self.yaw.sin());
            self.goal[0] += (rx * px + fx * pz) / l * speed * dt;
            self.goal[1] += (rz * px + fz * pz) / l * speed * dt;
        }
        let [x0, x1, z0, z1] = self.bounds;
        self.goal[0] = self.goal[0].clamp(x0, x1);
        self.goal[1] = self.goal[1].clamp(z0, z1);
        let k = (dt * 7.0).min(1.0);
        self.focus.x += (self.goal[0] - self.focus.x) * k;
        self.focus.z += (self.goal[1] - self.focus.z) * k;
        self.yaw += (self.goal[2] - self.yaw) * (dt * 8.0).min(1.0);
        self.dist += (self.goal[3] - self.dist) * (dt * 6.0).min(1.0);
        self.tilt += (self.goal[4] - self.tilt) * (dt * 6.0).min(1.0);
        let g = ground(self.focus.x, self.focus.z);
        self.focus.y += (g - self.focus.y) * (dt * 4.0).min(1.0);
        self.place(ground);
    }

    fn place(&mut self, ground: &dyn Fn(f32, f32) -> f32) {
        let pitch = self.pitch();
        let flat = self.dist * pitch.cos();
        let mut p = Vec3::new(
            self.focus.x + self.yaw.sin() * flat,
            self.focus.y + self.dist * pitch.sin(),
            self.focus.z + self.yaw.cos() * flat,
        );
        let under = ground(p.x, p.z) + 2.2;
        if p.y < under {
            p.y = under;
        }
        self.position = p;
        self.look_y = self.focus.y + (self.dist * 0.05).min(2.0);
    }

    pub(crate) fn camera(&self) -> Camera3D {
        Camera3D {
            position: self.position,
            target: Vec3::new(self.focus.x, self.look_y, self.focus.z),
            up: Vec3::Y,
            fov_y_radians: self.fov.to_radians(),
            orthographic_height: None,
            near: 0.8,
            far: 2600.0,
        }
    }
}

/// A point of the world on the 1920×1080 screen, if it is in front.
pub(crate) fn project(camera: Camera3D, p: Vec3) -> Option<[f32; 2]> {
    let view = camera.view_matrix().transform_point3(p);
    if !view.z.is_finite() || view.z >= -camera.near {
        return None;
    }
    let ndc = camera
        .view_projection(SCREEN_W / SCREEN_H)
        .transform_point3(p);
    if !ndc.x.is_finite() || !ndc.y.is_finite() {
        return None;
    }
    Some([
        (ndc.x * 0.5 + 0.5) * SCREEN_W,
        (1.0 - (ndc.y * 0.5 + 0.5)) * SCREEN_H,
    ])
}

/// The ray under a point of the screen: its origin and direction.
fn ray(camera: Camera3D, sx: f32, sy: f32) -> (Vec3, Vec3) {
    let inv = camera
        .view_projection(SCREEN_W / SCREEN_H)
        .inverse()
        .unwrap_or(Mat4::IDENTITY);
    let (nx, ny) = (sx / SCREEN_W * 2.0 - 1.0, 1.0 - sy / SCREEN_H * 2.0);
    let near = inv.transform_point3(Vec3::new(nx, ny, 0.0));
    let far = inv.transform_point3(Vec3::new(nx, ny, 0.9));
    (camera.position, (far - near).normalize_or_zero())
}

/// The ground under a point of the screen, marched along the ray over `ground`.
fn ground_at(
    camera: Camera3D,
    sx: f32,
    sy: f32,
    ground: &dyn Fn(f32, f32) -> f32,
    reach: f32,
) -> Option<(f32, f32)> {
    let (o, d) = ray(camera, sx, sy);
    let (mut t, mut prev) = (0.0f32, 0.0f32);
    for _ in 0..800 {
        t += (t * 0.012).max(0.6);
        let p = o + d * t;
        if p.y < ground(p.x, p.z) {
            let (mut a, mut b) = (prev, t);
            for _ in 0..12 {
                let m = (a + b) / 2.0;
                let q = o + d * m;
                if q.y < ground(q.x, q.z) {
                    b = m;
                } else {
                    a = m;
                }
            }
            let q = o + d * b;
            return Some((q.x, q.z));
        }
        prev = t;
        if t > reach {
            break;
        }
    }
    None
}

// ------------------------------------------------------------------ the session
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WarPage {
    /// The war table's front page (0 the main list, 1 the quick battles).
    Table(u8),
    Campaign,
    Battle,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WarKeys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub turn_l: bool,
    pub turn_r: bool,
    pub shift: bool,
    pub ctrl: bool,
}

/// A mouse button held down: where, which, the ground under it, whether it has moved.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Press {
    pub at: (f32, f32),
    pub button: u8,
    pub ground: Option<(f32, f32)>,
    pub moved: bool,
    pub shift: bool,
    pub hud: WarHit,
}

/// What flashes and flies, for the renderer to draw (it keeps how far it has read).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Fx {
    Sparks {
        x: f32,
        y: f32,
        z: f32,
        n: u32,
        color: [f32; 3],
    },
    Puff {
        x: f32,
        y: f32,
        z: f32,
        n: u32,
        size: f32,
        strength: f32,
    },
    Spark {
        x: f32,
        z: f32,
        r: f32,
    },
    Pillar {
        x: f32,
        z: f32,
        color: [f32; 3],
    },
    /// A power's ring going out on the ground.
    Ring {
        x: f32,
        z: f32,
        r: f32,
        color: [f32; 3],
    },
    /// An ember falling from the sky onto (x, z).
    Ember {
        x: f32,
        z: f32,
    },
}

/// The ghost of an order just given: the slots it fills, fading.
#[derive(Clone, Debug)]
pub(crate) struct Ghost {
    pub pts: Vec<(f32, f32)>,
    pub t: f32,
    pub size: f32,
}

/// Where a battle of the campaign came from, to bring its result back.
#[derive(Clone, Debug)]
pub(crate) struct CampaignFight {
    pub battle: PendingBattle,
    pub player_attacks: bool,
    pub scale: f32,
}

pub(crate) struct BattleRun {
    pub b: Battle,
    pub run_mode: bool,
    pub hover: Option<usize>,
    pub preview: Vec<LineSlot>,
    pub ghosts: Vec<Ghost>,
    pub messages: VecDeque<(String, u32, f32)>,
    pub help: bool,
    pub confirm: bool,
    pub end_t: f32,
    pub result: Option<(BattleResult, f32)>,
    pub from_campaign: Option<CampaignFight>,
    pub last_right: f32,
    pub last_card: (usize, f32),
    pub sound_budget: f32,
    pub hover_t: f32,
    pub scenario: Option<String>,
    /// The banners over the regiments, from one frame to the next (`world_tags`).
    pub tags: std::cell::RefCell<super::world_tags::TagLayout>,
    /// What she is placing (the build bar), and where it would stand under the pointer: x, z, its
    /// turn, why not (if not).
    pub build: Option<wr::siege::Kind>,
    pub ghost: Option<(f32, f32, f32, Option<&'static str>)>,
    /// The engine of hers chosen (a click on it), and the one under the pointer.
    pub engine_sel: Option<usize>,
    pub engine_hover: Option<usize>,
    /// Her war powers set and the field's relic laid (once, the first step of the battle).
    pub armed: bool,
    pub relic: Option<super::war_story::FieldRelic>,
    /// The power she is aiming (its ring under the pointer), and where it would land.
    pub power_aim: Option<wr::powers::Power>,
    pub power_at: Option<(f32, f32)>,
}

/// Which way a thing built at (x, z) faces: an engine at what it is for (the keep for a siege's
/// attackers, else the enemy's middle); a line across it.
fn build_yaw(b: &Battle, kind: wr::siege::Kind, x: f32, z: f32) -> f32 {
    let sim = &b.sim;
    let at = sim
        .siege
        .as_ref()
        .filter(|s| s.attacker(0))
        .and_then(|s| s.hold_at.map(|(kx, kz, _)| (kx, kz)))
        .unwrap_or_else(|| {
            let foes: Vec<&wr::sim::Regiment> = sim
                .regiments
                .iter()
                .filter(|r| r.side == 1 && r.alive > 0)
                .collect();
            let n = foes.len().max(1) as f32;
            (
                foes.iter().map(|r| r.cx).sum::<f32>() / n,
                foes.iter().map(|r| r.cz).sum::<f32>() / n,
            )
        });
    let face = (at.0 - x).atan2(at.1 - z);
    if kind.line() {
        face + std::f32::consts::FRAC_PI_2
    } else {
        face
    }
}

/// The engine standing at (x, z), if any (alive), of `side` if given.
fn engine_at(b: &Battle, x: f32, z: f32, side: Option<u8>) -> Option<usize> {
    let s = b.sim.siege.as_ref()?;
    s.engines
        .iter()
        .enumerate()
        .filter(|(_, e)| e.alive && side.is_none_or(|sd| e.side == sd))
        .map(|(i, e)| {
            let r = e.spec().radius.max(e.spec().length / 2.0) + 1.5;
            (i, (e.x - x).hypot(e.z - z) - r)
        })
        .filter(|&(_, d)| d < 0.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// An army's token on the campaign map, sliding from one province to the next.
#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub at: Vec3,
    pub from: Option<Vec3>,
    pub to: Vec3,
    pub t: f32,
    pub yaw: f32,
}

/// What the campaign's view shows of the story (`war_story.rs`).
#[derive(Clone, Debug, Default)]
pub(crate) struct StoryView {
    pub objective: String,
    /// Each region's relic: the line about it, and whether it has been found.
    pub relic: HashMap<String, (String, bool)>,
    /// The river is still closed to her armies.
    pub river_closed: bool,
    /// Today's turn has been ended already (the next one, after she sleeps).
    pub turn_today: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CampModal {
    Intro,
    Battle(PendingBattle),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Sel {
    Army(String),
    Province(usize),
}

pub(crate) struct CampaignRun {
    pub sel: Option<Sel>,
    pub hover: Option<usize>,
    pub hover_t: f32,
    pub tokens: HashMap<String, Token>,
    /// The others' moves still to be shown, and the battle they end in.
    pub steps: VecDeque<Step>,
    pub battle_after: Option<PendingBattle>,
    pub busy: bool,
    pub wait: f32,
    pub modal: Option<CampModal>,
    pub toast: Option<(String, String, f32)>,
    pub provinces: wr::campaign_map::Provinces,
    pub land: wr::campaign_map::Land,
    /// Bumped whenever the province picture must be painted again.
    pub paint_version: u64,
    pub end_shown: bool,
    /// The names and the armies' plates over the map, from one frame to the next (`world_tags`).
    pub tags: std::cell::RefCell<super::world_tags::TagLayout>,
    /// The Queen's armies on a march that still walk this turn (one after the other).
    pub marching: VecDeque<String>,
}

pub(crate) struct WarSession {
    pub page: WarPage,
    pub battle: Option<BattleRun>,
    pub camp: Option<CampaignRun>,
    pub cam: RtsCam,
    pub time: f32,
    pub keys: WarKeys,
    pub pointer: Option<(f32, f32)>,
    pub press: Option<Press>,
    /// Men per regiment (the size of the units).
    pub scale: f32,
    pub fx: Vec<Fx>,
    pub fx_first: u64,
    pub view: WarView,
    /// The picture of the minimap is up for this battle.
    pub minimap_ready: bool,
    /// The renderer has the scene of this battle (by a counter of battles begun).
    pub battle_serial: u64,
    /// The war table's entry the keys are on (the mouse moves it too).
    pub focus: usize,
    /// A pointer held where it is for the captures (QA), when the window has none.
    pub qa_pointer: Option<(f32, f32)>,
    /// The quick battles' map of the continent chosen (in `baked::BATTLE_MAPS`), and whether she
    /// attacks on it.
    pub map_pick: usize,
    pub map_attack: bool,
    /// The war's scenes waiting for a quiet moment (`war_story.rs`).
    pub scenes: VecDeque<String>,
}

impl WarSession {
    fn push_fx(&mut self, fx: Fx) {
        self.fx.push(fx);
        if self.fx.len() > 4000 {
            let drop = self.fx.len() - 3000;
            self.fx.drain(0..drop);
            self.fx_first += drop as u64;
        }
    }
}

fn faction_edge(f: Faction) -> u32 {
    f.info().color
}

fn role_glyph(t: &wd::UnitType) -> Glyph {
    match t.role {
        Role::Hero => Glyph::Crown,
        Role::Spear => Glyph::Spear,
        Role::Skirmisher => Glyph::Skirmish,
        Role::Archer => Glyph::Bow,
        Role::Flyer => Glyph::Wing,
        Role::Cavalry => Glyph::Horse,
        Role::Support => Glyph::Staff,
        Role::Beast => Glyph::Fang,
        Role::Infantry => match t.model {
            "skeleton" => Glyph::Skull,
            "lamia" => Glyph::Tail,
            _ => Glyph::Swords,
        },
    }
}

fn sound(sfx: dungeon::sound::Sfx, at: Vec3, volume: f32) -> dungeon::sound::Sound {
    dungeon::sound::Sound {
        sfx,
        at: Some(at),
        volume,
    }
}

impl App {
    // ------------------------------------------------------------------ in and out
    /// F at the war table: its front page.
    pub(super) fn open_war_table(&mut self) {
        let ground = |_: f32, _: f32| 0.0;
        let mut cam = RtsCam::new([-100.0, 100.0, -100.0, 100.0], 9.0, 340.0, 38.0);
        cam.frame(0.0, 0.0, Some(120.0), Some(0.0), true, &ground);
        self.war = Some(WarSession {
            page: WarPage::Table(0),
            battle: None,
            camp: None,
            cam,
            time: 0.0,
            keys: WarKeys::default(),
            pointer: None,
            press: None,
            scale: self.war_scale,
            fx: Vec::new(),
            fx_first: 0,
            view: WarView::default(),
            minimap_ready: false,
            battle_serial: 0,
            focus: 0,
            qa_pointer: None,
            map_pick: 0,
            map_attack: true,
            scenes: VecDeque::new(),
        });
        self.map_open = false;
        self.clear_input();
        ui_sfx::play(ui_sfx::UiSfx::Open);
        self.treasury_in();
        self.build_war_view();
    }

    /// Back to the castle, by the table.
    pub(super) fn leave_war(&mut self) {
        if let Some(w) = self.war.take() {
            if let Some(c) = w.camp {
                let _ = c;
            }
        }
        ui_sfx::play(ui_sfx::UiSfx::Close);
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// The war's own panels that Esc closes before the pause menu.
    pub(crate) fn war_modal(&self) -> bool {
        self.war.as_ref().is_some_and(|w| {
            w.battle.as_ref().is_some_and(|b| b.help || b.confirm)
                || w.camp
                    .as_ref()
                    .is_some_and(|c| c.modal.is_some() || c.sel.is_some())
                || matches!(w.page, WarPage::Table(1))
        })
    }

    fn battle_ground(b: &Battle) -> impl Fn(f32, f32) -> f32 + '_ {
        move |x, z| b.sim.field.walk_height(x, z)
    }

    /// A quick battle from the table's list.
    fn start_quick_battle(&mut self, index: usize) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let sc = &SCENARIOS[index.min(SCENARIOS.len() - 1)];
        let mut setup = Setup::scenario(sc.id, w.scale);
        setup.seed = (self.elapsed * 1000.0) as u32 % 9973 + 1;
        Self::begin_battle(w, setup, None, Some(sc.id.to_owned()));
        eprintln!("[guerra] quick battle «{}»", sc.id);
    }

    /// A battle on one of the war map 2.0's battle maps, the quick battles' armies (QA, the table).
    pub(crate) fn start_map_battle(&mut self, map: &str, player_attacks: bool) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let mut setup = Setup::scenario(SCENARIOS[0].id, w.scale);
        setup.map = map.to_owned();
        setup.player_attacks = player_attacks;
        setup.title = String::new();
        if let Some(m) = wr::baked::ensure(map, &super::war_render::battle_maps_root()) {
            setup.kicker = m.kicker.clone();
            setup.subtitle = m.blurb.clone();
        }
        setup.enemy_posture = Some(if player_attacks {
            wr::ai::Posture::Defend
        } else {
            wr::ai::Posture::Attack
        });
        setup.seed = (self.elapsed * 1000.0) as u32 % 9973 + 1;
        let again = format!("mapa:{map}:{}", player_attacks as u8);
        Self::begin_battle(w, setup, None, Some(again));
        eprintln!("[guerra] battle on the map «{map}»");
    }

    fn begin_battle(
        w: &mut WarSession,
        setup: Setup,
        from_campaign: Option<CampaignFight>,
        scenario: Option<String>,
    ) {
        // the battle maps of the war map 2.0 load the first time they are fought on
        let _ = wr::baked::ensure(&setup.map, &super::war_render::battle_maps_root());
        let b = Battle::new(setup);
        let p = b.sim.field.play;
        let mut cam = RtsCam::new(
            [p.x0 - 30.0, p.x1 + 30.0, p.z0 - 30.0, p.z1 + 30.0],
            9.0,
            340.0,
            38.0,
        );
        let mine: Vec<&wr::sim::Regiment> =
            b.sim.regiments.iter().filter(|r| r.side == 0).collect();
        let n = mine.len().max(1) as f32;
        let cx = mine.iter().map(|r| r.cx).sum::<f32>() / n;
        let cz = mine.iter().map(|r| r.cz).sum::<f32>() / n;
        // behind her army, looking at the enemy's (on the hand-made fields: north)
        let theirs: Vec<&wr::sim::Regiment> =
            b.sim.regiments.iter().filter(|r| r.side == 1).collect();
        let m = theirs.len().max(1) as f32;
        let ex = theirs.iter().map(|r| r.cx).sum::<f32>() / m;
        let ez = theirs.iter().map(|r| r.cz).sum::<f32>() / m;
        let (dx, dz) = (ex - cx, ez - cz);
        let l = dx.hypot(dz).max(1.0);
        let yaw = if b.sim.field.baked.is_some() {
            (-dx).atan2(-dz)
        } else {
            0.0
        };
        let (fx, fz) = if b.sim.field.baked.is_some() {
            (dx / l, dz / l)
        } else {
            (0.0, -1.0)
        };
        cam.frame(
            cx + fx * 40.0,
            cz + fz * 40.0,
            Some(150.0),
            Some(yaw),
            true,
            &Self::battle_ground(&b),
        );
        w.cam = cam;
        w.battle = Some(BattleRun {
            b,
            run_mode: false,
            hover: None,
            preview: Vec::new(),
            ghosts: Vec::new(),
            messages: VecDeque::new(),
            help: false,
            confirm: false,
            end_t: 0.0,
            result: None,
            from_campaign,
            last_right: -10.0,
            last_card: (usize::MAX, -10.0),
            sound_budget: 0.0,
            hover_t: 0.0,
            scenario,
            tags: Default::default(),
            build: None,
            ghost: None,
            engine_sel: None,
            engine_hover: None,
            armed: false,
            relic: None,
            power_aim: None,
            power_at: None,
        });
        w.page = WarPage::Battle;
        w.press = None;
        w.minimap_ready = false;
        w.battle_serial += 1;
        w.fx.clear();
        w.fx_first = 0;
        ui_sfx::play(ui_sfx::UiSfx::Select);
    }

    /// The campaign: the saved one, or a new one.
    fn start_campaign(&mut self, fresh: bool) {
        install_war_map();
        let varrak = self.varrak_awake();
        // a campaign saved before the war map 2.0 moves onto it
        if let Some(c) = self.war_campaign.as_mut() {
            c.migrate();
        }
        if fresh || self.war_campaign.is_none() {
            let seed = (self.elapsed * 1000.0) as u32 % 9973 + 1;
            self.war_campaign = Some(Campaign::new(seed, varrak));
            if let Some(s) = self.war_campaign.as_mut() {
                // (Varrak woken in the story is in it from the start: no scene for him joining)
                s.story_mark("varrak");
            }
        }
        self.treasury_in();
        // (the campaign's first scene before the ones the story asks for)
        self.queue_war_scene("guerra.intro");
        self.story_into_campaign();
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let land = wr::campaign_map::Land::new();
        let land_ground = |x: f32, z: f32| land.top_at(x, z);
        let mut cam = RtsCam::campaign([-110.0, 110.0, -67.0, 67.0]);
        let s = self.war_campaign.as_ref().expect("a campaign");
        let at = s
            .queen_army()
            .map(|a| wd::province(&a.at).site())
            .unwrap_or_else(|| wd::province("castillo").site());
        cam.frame(
            at.0 + 18.0,
            at.1 + 4.0,
            Some(150.0),
            Some(0.0),
            true,
            &land_ground,
        );
        w.cam = cam;
        let intro = !s.intro_seen && s.turn == 1;
        let pending = (s.phase == CampPhase::Ai)
            .then(|| s.pending.clone())
            .flatten();
        let mut run = CampaignRun {
            sel: None,
            hover: None,
            hover_t: 0.0,
            tokens: HashMap::new(),
            steps: VecDeque::new(),
            battle_after: None,
            busy: false,
            wait: 0.0,
            modal: if intro {
                Some(CampModal::Intro)
            } else {
                pending.map(CampModal::Battle)
            },
            toast: None,
            provinces: wr::campaign_map::Provinces::new(),
            land,
            paint_version: 1,
            end_shown: false,
            tags: Default::default(),
            marching: VecDeque::new(),
        };
        Self::sync_tokens(&mut run, s, true);
        if s.phase == CampPhase::Player {
            Self::queue_marches(&mut run, s);
        }
        w.camp = Some(run);
        w.page = WarPage::Campaign;
        w.press = None;
        if let Some(s) = self.war_campaign.as_mut() {
            s.intro_seen = true;
        }
        ui_sfx::play(ui_sfx::UiSfx::Open);
        eprintln!(
            "[guerra] campaign (turn {})",
            self.war_campaign.as_ref().map_or(0, |s| s.turn)
        );
    }

    // ------------------------------------------------------------------ input
    pub(crate) fn war_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        // a conversation over the war takes the keys (the war waits under it)
        if self.talk.is_some() {
            if pressed && !repeat {
                match code {
                    KeyCode::KeyF | KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter => {
                        self.talk_confirm()
                    }
                    KeyCode::ArrowUp | KeyCode::KeyW => self.talk_move(-1),
                    KeyCode::ArrowDown | KeyCode::KeyS => self.talk_move(1),
                    KeyCode::Digit1 | KeyCode::Numpad1 => self.talk_pick(0),
                    KeyCode::Digit2 | KeyCode::Numpad2 => self.talk_pick(1),
                    KeyCode::Digit3 | KeyCode::Numpad3 => self.talk_pick(2),
                    KeyCode::Digit4 | KeyCode::Numpad4 => self.talk_pick(3),
                    KeyCode::Escape | KeyCode::Backspace if !self.stage_up() => self.end_talk(),
                    _ => {}
                }
            }
            if let Some(w) = self.war.as_mut() {
                w.keys = WarKeys::default();
            }
            return;
        }
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let k = &mut w.keys;
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => k.up = pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => k.down = pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => k.left = pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => k.right = pressed,
            KeyCode::KeyQ => k.turn_l = pressed,
            KeyCode::KeyE => k.turn_r = pressed,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => k.shift = pressed,
            KeyCode::ControlLeft | KeyCode::ControlRight => k.ctrl = pressed,
            _ => {}
        }
        if !pressed || repeat {
            return;
        }
        let ctrl = w.keys.ctrl;
        match w.page {
            WarPage::Table(page) => match code {
                KeyCode::ArrowUp | KeyCode::KeyW | KeyCode::ArrowDown | KeyCode::KeyS => {
                    let n = w
                        .view
                        .buttons
                        .iter()
                        .filter(|b| b.style == 4)
                        .count()
                        .max(1);
                    let down = matches!(code, KeyCode::ArrowDown | KeyCode::KeyS);
                    w.focus = if down {
                        (w.focus + 1) % n
                    } else {
                        (w.focus + n - 1) % n
                    };
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
                KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::ArrowRight | KeyCode::KeyD => {
                    let on_size = w
                        .view
                        .buttons
                        .iter()
                        .filter(|b| b.style == 4)
                        .nth(w.focus)
                        .is_some_and(|b| b.hit == WarHit::Table(3));
                    let on_map = w
                        .view
                        .buttons
                        .iter()
                        .filter(|b| b.style == 4)
                        .nth(w.focus)
                        .is_some_and(|b| b.hit == WarHit::Table(30));
                    if on_map {
                        let right = matches!(code, KeyCode::ArrowRight | KeyCode::KeyD);
                        self.war_hit(WarHit::Table(if right { 32 } else { 31 }));
                    } else if on_size {
                        let at = SIZES
                            .iter()
                            .position(|&k| (k - w.scale).abs() < 0.01)
                            .unwrap_or(1);
                        let right = matches!(code, KeyCode::ArrowRight | KeyCode::KeyD);
                        let to = if right {
                            (at + 1).min(SIZES.len() - 1)
                        } else {
                            at.saturating_sub(1)
                        };
                        if to != at {
                            self.war_hit(WarHit::Table(20 + to as u8));
                        }
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyF => {
                    if let Some(hit) = w
                        .view
                        .buttons
                        .iter()
                        .filter(|b| b.style == 4)
                        .nth(w.focus)
                        .map(|b| b.hit)
                    {
                        self.war_hit(hit);
                    }
                }
                KeyCode::Escape | KeyCode::Backspace => {
                    if page == 1 {
                        self.war_hit(WarHit::Table(5));
                    } else {
                        self.leave_war();
                    }
                }
                KeyCode::Digit1
                | KeyCode::Digit2
                | KeyCode::Digit3
                | KeyCode::Digit4
                | KeyCode::Digit5
                    if page == 1 =>
                {
                    let i = match code {
                        KeyCode::Digit1 => 0,
                        KeyCode::Digit2 => 1,
                        KeyCode::Digit3 => 2,
                        KeyCode::Digit4 => 3,
                        _ => 4,
                    };
                    self.start_quick_battle(i);
                }
                _ => {}
            },
            WarPage::Battle => {
                let Some(run) = w.battle.as_mut() else {
                    return;
                };
                if run.result.is_some() {
                    if matches!(code, KeyCode::Enter | KeyCode::Space) {
                        let hit = if run.from_campaign.is_some() {
                            WarHit::Result(0)
                        } else {
                            WarHit::Result(2)
                        };
                        self.war_hit(hit);
                    }
                    return;
                }
                if run.confirm {
                    match code {
                        KeyCode::Enter => self.war_hit(WarHit::Confirm(true)),
                        KeyCode::Escape | KeyCode::Backspace => {
                            self.war_hit(WarHit::Confirm(false))
                        }
                        _ => {}
                    }
                    return;
                }
                match code {
                    KeyCode::Space | KeyCode::KeyP => self.war_hit(WarHit::Speed(0)),
                    KeyCode::Digit1 => self.war_hit(WarHit::Speed(1)),
                    KeyCode::Digit2 => self.war_hit(WarHit::Speed(2)),
                    KeyCode::Digit3 => self.war_hit(WarHit::Speed(3)),
                    KeyCode::KeyH => self.war_hit(WarHit::Order(0)),
                    KeyCode::KeyR => self.war_hit(WarHit::Order(1)),
                    KeyCode::KeyG => self.war_hit(WarHit::Order(2)),
                    KeyCode::KeyK => self.war_hit(WarHit::Order(3)),
                    KeyCode::KeyF => self.war_hit(WarHit::Order(4)),
                    KeyCode::KeyZ => self.war_hit(WarHit::Power(0)),
                    KeyCode::KeyX => self.war_hit(WarHit::Power(1)),
                    KeyCode::KeyC => self.war_hit(WarHit::Power(2)),
                    KeyCode::KeyV => self.war_hit(WarHit::Power(3)),
                    KeyCode::KeyB => self.war_hit(WarHit::Power(4)),
                    KeyCode::KeyN => self.war_hit(WarHit::Power(5)),
                    KeyCode::F1 => self.war_hit(WarHit::Help),
                    KeyCode::Enter => self.war_hit(WarHit::Begin),
                    KeyCode::KeyA if ctrl => {
                        for r in &mut run.b.sim.regiments {
                            r.selected = r.side == 0 && r.alive > 0;
                        }
                        ui_sfx::play(ui_sfx::UiSfx::Move);
                    }
                    KeyCode::Escape | KeyCode::Backspace => {
                        if run.build.is_some() {
                            run.build = None;
                            run.ghost = None;
                        } else if run.help {
                            run.help = false;
                        } else {
                            for r in &mut run.b.sim.regiments {
                                r.selected = false;
                            }
                            Self::select_engine(run, None);
                        }
                    }
                    KeyCode::Delete => {
                        if let (Some(i), true) = (run.engine_sel, run.b.deploying) {
                            if let Some(mut s) = run.b.sim.siege.take() {
                                s.unbuild(&mut run.b.sim, i);
                                run.b.sim.siege = Some(s);
                            }
                            Self::select_engine(run, None);
                            ui_sfx::play(ui_sfx::UiSfx::Move);
                        }
                    }
                    _ => {}
                }
            }
            WarPage::Campaign => match code {
                KeyCode::Tab => self.war_hit(WarHit::NextArmy),
                KeyCode::Enter => {
                    let modal = w.camp.as_ref().and_then(|c| c.modal.clone());
                    match modal {
                        Some(CampModal::Intro) => self.war_hit(WarHit::Modal(0)),
                        Some(_) => {}
                        None => self.war_hit(WarHit::EndTurn),
                    }
                }
                KeyCode::Escape | KeyCode::Backspace => {
                    if let Some(c) = w.camp.as_mut() {
                        if matches!(c.modal, Some(CampModal::Intro)) {
                            c.modal = None;
                        } else if c.modal.is_none() {
                            c.sel = None;
                            c.paint_version += 1;
                        }
                    }
                }
                _ => {}
            },
        }
    }

    /// The war's mouse buttons (`button`: 0 left, 1 middle, 2 right).
    pub(crate) fn war_button(&mut self, button: MouseButton, pressed: bool) {
        let b = match button {
            MouseButton::Left => 0u8,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        let pointer = self.shop_pointer_position();
        let now = self.elapsed;
        // a conversation over the war takes the clicks
        if self.talk.is_some() {
            if pressed && b == 0 {
                self.talk_click();
            }
            if let Some(w) = self.war.as_mut() {
                w.press = None;
            }
            return;
        }
        let Some(w) = self.war.as_mut() else {
            return;
        };
        w.pointer = pointer;
        let Some((x, y)) = pointer else {
            if !pressed {
                w.press = None;
            }
            return;
        };
        if pressed {
            let hud = w.view.hit(x, y);
            let over = w.view.over_hud(x, y);
            let camera = w.cam.camera();
            let ground = match (&w.battle, &w.camp, w.page) {
                (Some(run), _, WarPage::Battle) if !over => {
                    ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0)
                }
                (_, Some(c), WarPage::Campaign) if !over => ground_at(
                    camera,
                    x,
                    y,
                    &|gx: f32, gz: f32| c.land.top_at(gx, gz),
                    1600.0,
                )
                .filter(|(gx, gz)| gx.abs() < 120.0 && gz.abs() < 75.0),
                _ => None,
            };
            w.press = Some(Press {
                at: (x, y),
                button: b,
                ground,
                moved: false,
                shift: w.keys.shift || w.keys.ctrl,
                hud,
            });
            return;
        }
        let Some(press) = w.press.take() else {
            return;
        };
        if press.button != b {
            return;
        }
        if let Some(run) = w.battle.as_mut() {
            run.preview.clear();
        }
        // A click on the HUD (pressed and let go on the same thing).
        if press.hud != WarHit::None {
            let hit = w.view.hit(x, y);
            if hit == press.hud
                || matches!(hit, WarHit::Minimap(..)) && matches!(press.hud, WarHit::Minimap(..))
            {
                if b == 2 {
                    if let WarHit::Minimap(u, v) = hit {
                        self.war_minimap(u, v, true);
                    }
                    return;
                }
                if b == 0 {
                    if let WarHit::Card(reg) = hit {
                        self.war_card(reg, now);
                        return;
                    }
                    if let WarHit::Minimap(u, v) = hit {
                        self.war_minimap(u, v, false);
                        return;
                    }
                    self.war_hit(hit);
                }
            }
            return;
        }
        match w.page {
            WarPage::Battle => self.battle_release(press, x, y, now),
            WarPage::Campaign => self.campaign_release(press, x, y),
            WarPage::Table(_) => {}
        }
    }

    /// The pointer moved: the middle button turns the camera, a held left draws the box, a held
    /// right the line.
    pub(crate) fn war_pointer(&mut self) {
        if self.talk.is_some() {
            self.talk_hover();
            return;
        }
        let pointer = self.shop_pointer_position();
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let before = w.pointer;
        w.pointer = pointer;
        if let (WarPage::Table(_), Some((x, y))) = (w.page, pointer) {
            if let Some(i) = w
                .view
                .buttons
                .iter()
                .filter(|b| b.style == 4)
                .position(|b| b.contains(x, y))
            {
                if w.focus != i {
                    w.focus = i;
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
            }
        }
        let (Some((x, y)), Some(press)) = (pointer, w.press.as_mut()) else {
            return;
        };
        if (x - press.at.0).hypot(y - press.at.1) > 7.0 {
            press.moved = true;
        }
        if press.button == 1 {
            if let Some((bx, by)) = before {
                w.cam.drag(x - bx, y - by);
            }
            return;
        }
        if press.button == 2 && press.moved && press.hud == WarHit::None {
            if let (Some(run), Some(a)) = (w.battle.as_mut(), press.ground) {
                let camera = w.cam.camera();
                if let Some(g) = ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0) {
                    let sel = run.b.selected();
                    run.preview = run.b.line_slots(&sel, a, g);
                }
            }
        }
    }

    pub(crate) fn war_wheel(&mut self, amount: f32) {
        if let Some(w) = self.war.as_mut() {
            if w.page != WarPage::Table(0) && w.page != WarPage::Table(1) {
                w.cam.zoom(-amount * 100.0);
            }
        }
    }

    fn war_card(&mut self, reg: usize, now: f32) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let add = w.keys.shift || w.keys.ctrl;
        let Some(run) = w.battle.as_mut() else {
            return;
        };
        let double = run.last_card.0 == reg && now - run.last_card.1 < 0.35;
        run.last_card = (reg, now);
        if double {
            let (cx, cz) = (run.b.sim.regiments[reg].cx, run.b.sim.regiments[reg].cz);
            let d = w.cam.goal_dist().min(70.0);
            w.cam
                .frame(cx, cz, Some(d), None, false, &Self::battle_ground(&run.b));
            return;
        }
        if add {
            let r = &mut run.b.sim.regiments[reg];
            r.selected = !r.selected;
        } else {
            for r in &mut run.b.sim.regiments {
                r.selected = r.index == reg;
            }
        }
        ui_sfx::play(ui_sfx::UiSfx::Move);
    }

    fn war_minimap(&mut self, u: f32, v: f32, order: bool) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let Some(run) = w.battle.as_mut() else {
            return;
        };
        let p = run.b.sim.field.play;
        let (x0, x1, z0, z1) = (p.x0 - 14.0, p.x1 + 14.0, p.z0 - 14.0, p.z1 + 14.0);
        let (x, z) = (x0 + u * (x1 - x0), z0 + v * (z1 - z0));
        if order {
            let sel = run.b.selected();
            let slots = run.b.command_move(&sel, x, z, run.run_mode);
            Self::ghosts(run, &slots);
            ui_sfx::play(ui_sfx::UiSfx::Select);
        } else {
            w.cam
                .frame(x, z, None, None, false, &Self::battle_ground(&run.b));
        }
    }

    fn ghosts(run: &mut BattleRun, slots: &[LineSlot]) {
        for s in slots {
            let r = &run.b.sim.regiments[s.reg];
            let n = r.alive as usize;
            let step = if n > 60 { 2 } else { 1 };
            let pts = (0..n)
                .step_by(step)
                .map(|k| run.b.sim.slot_pos(s.reg, k, s.x, s.z, s.heading, s.files))
                .collect();
            run.ghosts.push(Ghost {
                pts,
                t: 0.0,
                size: r.ty.radius * 2.4,
            });
        }
        while run.ghosts.len() > 12 {
            run.ghosts.remove(0);
        }
    }

    /// The regiment under a point of the screen.
    fn pick_regiment(
        run: &BattleRun,
        camera: Camera3D,
        sx: f32,
        sy: f32,
        side: Option<u8>,
    ) -> Option<usize> {
        let sim = &run.b.sim;
        let mut best = None;
        let mut bd = 22.0f32;
        for r in &sim.regiments {
            if r.alive == 0 || r.state == RState::Fled || side.is_some_and(|s| r.side != s) {
                continue;
            }
            let lim = if r.ty.role == Role::Hero { 40.0 } else { 22.0 };
            let step = (r.soldiers.len() / 24).max(1);
            for &i in r.soldiers.iter().step_by(step) {
                if sim.state[i] != 0 {
                    continue;
                }
                let lift = if r.ty.large { 1.6 } else { 1.0 };
                let Some(p) = project(camera, Vec3::new(sim.px[i], sim.py[i] + lift, sim.pz[i]))
                else {
                    continue;
                };
                let d = (p[0] - sx).hypot(p[1] - sy);
                if d < bd.min(lim) {
                    bd = d;
                    best = Some(r.index);
                }
            }
        }
        // the banners over their heads count too
        if best.is_none() {
            for tag in run.tags.borrow().placed().iter().rev() {
                let [x, y, w, h] = tag.rect;
                if tag.alpha > 0.5
                    && sx >= x
                    && sx <= x + w
                    && sy >= y
                    && sy <= y + h
                    && side.is_none_or(|s| sim.regiments[tag.id].side == s)
                {
                    return Some(tag.id);
                }
            }
        }
        best
    }

    /// Chooses one of her engines (or none): the others let go.
    fn select_engine(run: &mut BattleRun, i: Option<usize>) {
        run.engine_sel = i;
        if let Some(s) = run.b.sim.siege.as_mut() {
            for (k, e) in s.engines.iter_mut().enumerate() {
                e.selected = Some(k) == i;
            }
        }
    }

    fn battle_release(&mut self, press: Press, x: f32, y: f32, now: f32) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let camera = w.cam.camera();
        let Some(run) = w.battle.as_mut() else {
            return;
        };
        if run.result.is_some() || run.help || run.confirm {
            return;
        }
        // a war power aimed: a click casts it there, the right button stops aiming
        if let Some(p) = run.power_aim {
            if press.button == 2 {
                run.power_aim = None;
                return;
            }
            if press.button == 0 && !press.moved {
                let gp = ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0);
                if let Some((gx, gz)) = gp {
                    if let Some(mut pw) = run.b.sim.powers.take() {
                        let cast = pw.cast(&mut run.b.sim, 0, p, gx, gz);
                        run.b.sim.powers = Some(pw);
                        ui_sfx::play(if cast {
                            ui_sfx::UiSfx::Equip
                        } else {
                            ui_sfx::UiSfx::Error
                        });
                    }
                    run.power_aim = None;
                    run.power_at = None;
                }
                return;
            }
        }
        // building: a click puts it there (lines: one after another), the right button stops
        if let Some(kind) = run.build {
            if press.button == 2 {
                run.build = None;
                run.ghost = None;
                return;
            }
            if press.button == 0 && !press.moved {
                let gp = ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0);
                if let Some((gx, gz)) = gp {
                    let yaw = build_yaw(&run.b, kind, gx, gz);
                    if let Some(mut s) = run.b.sim.siege.take() {
                        let ok = s.check(
                            &run.b.sim,
                            0,
                            kind,
                            gx,
                            gz,
                            yaw,
                            run.b.deploying,
                            run.b.zone,
                        );
                        match ok {
                            Ok(()) => {
                                let deploying = run.b.deploying;
                                s.place(&mut run.b.sim, 0, kind, gx, gz, yaw, deploying);
                                ui_sfx::play(ui_sfx::UiSfx::Equip);
                                if !kind.line() || s.wood[0] < kind.spec().cost {
                                    run.build = None;
                                }
                            }
                            Err(why) => {
                                ui_sfx::play(ui_sfx::UiSfx::Fail);
                                run.messages.push_front((why.to_owned(), 0xe2334a, 0.0));
                            }
                        }
                        run.b.sim.siege = Some(s);
                    }
                }
                return;
            }
        }
        // an engine of hers: a click chooses it; with it chosen, the right button gives it a target
        if press.button == 0 && !press.moved {
            let gp = ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0);
            if let Some((gx, gz)) = gp {
                if let Some(i) = engine_at(&run.b, gx, gz, Some(0)) {
                    if Self::pick_regiment(run, camera, x, y, Some(0)).is_none() {
                        for r in &mut run.b.sim.regiments {
                            r.selected = false;
                        }
                        Self::select_engine(run, Some(i));
                        ui_sfx::play(ui_sfx::UiSfx::Move);
                        return;
                    }
                }
            }
            Self::select_engine(run, None);
        }
        if press.button == 2 && !press.moved {
            if let Some(i) = run.engine_sel {
                let gp = ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0);
                let target = if let Some(foe) = Self::pick_regiment(run, camera, x, y, Some(1)) {
                    Some(wr::siege::Target::Regiment(foe))
                } else if let Some((gx, gz)) = gp {
                    let f = &run.b.sim.field;
                    let work = (-3..=3)
                        .flat_map(|a| (-3..=3).map(move |c| (a as f32 * 1.5, c as f32 * 1.5)))
                        .find_map(|(dx, dz)| f.work_at(gx + dx, gz + dz));
                    match (engine_at(&run.b, gx, gz, Some(1)), work) {
                        (Some(e), _) => Some(wr::siege::Target::Engine(e)),
                        (None, Some(w)) => Some(wr::siege::Target::Work(w)),
                        _ => Some(wr::siege::Target::Auto),
                    }
                } else {
                    None
                };
                if let (Some(t), Some(s)) = (target, run.b.sim.siege.as_mut()) {
                    s.order(i, t);
                    ui_sfx::play(ui_sfx::UiSfx::Equip);
                }
                return;
            }
        }
        match press.button {
            0 => {
                if press.moved {
                    let (ax, bx) = (press.at.0.min(x), press.at.0.max(x));
                    let (ay, by) = (press.at.1.min(y), press.at.1.max(y));
                    if !press.shift {
                        for r in &mut run.b.sim.regiments {
                            r.selected = false;
                        }
                    }
                    let sim = &run.b.sim;
                    let mut chosen = Vec::new();
                    for r in sim.regiments.iter().filter(|r| r.side == 0 && r.alive > 0) {
                        if r.soldiers.iter().any(|&i| {
                            sim.state[i] == 0
                                && project(camera, Vec3::new(sim.px[i], sim.py[i] + 1.0, sim.pz[i]))
                                    .is_some_and(|p| {
                                        p[0] >= ax && p[0] <= bx && p[1] >= ay && p[1] <= by
                                    })
                        }) {
                            chosen.push(r.index);
                        }
                    }
                    for &i in &chosen {
                        run.b.sim.regiments[i].selected = true;
                    }
                    if !chosen.is_empty() {
                        ui_sfx::play(ui_sfx::UiSfx::Move);
                    }
                } else {
                    let pick = Self::pick_regiment(run, camera, x, y, Some(0));
                    if !press.shift {
                        for r in &mut run.b.sim.regiments {
                            r.selected = false;
                        }
                    }
                    if let Some(i) = pick {
                        let r = &mut run.b.sim.regiments[i];
                        r.selected = if press.shift { !r.selected } else { true };
                        ui_sfx::play(ui_sfx::UiSfx::Move);
                    }
                }
            }
            2 => {
                let run_order = run.run_mode || now - run.last_right < 0.35;
                run.last_right = now;
                let sel = run.b.selected();
                if sel.is_empty() {
                    return;
                }
                if press.moved {
                    if let Some(a) = press.ground {
                        let end = ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0);
                        if let Some(g) = end {
                            if (g.0 - a.0).hypot(g.1 - a.1) > 6.0 {
                                let slots = run.b.command_line(&sel, a, g, run_order);
                                Self::ghosts(run, &slots);
                                ui_sfx::play(ui_sfx::UiSfx::Select);
                                return;
                            }
                        }
                    }
                }
                if let Some(foe) = Self::pick_regiment(run, camera, x, y, Some(1)) {
                    if !run.b.deploying {
                        run.b.command_attack(&sel, foe);
                        ui_sfx::play(ui_sfx::UiSfx::Equip);
                    }
                } else if let Some((gx, gz)) = {
                    let g = press
                        .ground
                        .or_else(|| ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0));
                    g
                } {
                    let slots = run.b.command_move(&sel, gx, gz, run_order);
                    Self::ghosts(run, &slots);
                    ui_sfx::play(ui_sfx::UiSfx::Select);
                }
            }
            _ => {}
        }
    }

    /// A button of the war's (or its key).
    pub(crate) fn war_hit(&mut self, hit: WarHit) {
        let saved = self.war_campaign.is_some();
        let Some(w) = self.war.as_mut() else {
            return;
        };
        match hit {
            WarHit::None => {}
            WarHit::Leave => self.leave_war(),
            WarHit::Table(i) => match i {
                0 => self.start_campaign(false),
                1 => self.start_campaign(true),
                2 => {
                    w.page = WarPage::Table(1);
                    w.focus = 0;
                    ui_sfx::play(ui_sfx::UiSfx::Page);
                }
                3 => {
                    let at = SIZES
                        .iter()
                        .position(|&k| (k - w.scale).abs() < 0.01)
                        .unwrap_or(1);
                    w.scale = SIZES[(at + 1) % SIZES.len()];
                    self.war_scale = w.scale;
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
                20..=23 => {
                    w.scale = SIZES[(i - 20) as usize];
                    self.war_scale = w.scale;
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
                6 => {
                    w.page = WarPage::Table(2);
                    w.focus = 0;
                    ui_sfx::play(ui_sfx::UiSfx::Page);
                }
                60..=99 => ui_sfx::play(ui_sfx::UiSfx::Move),
                30 => {
                    let (map, attack) = (wr::baked::BATTLE_MAPS[w.map_pick].0, w.map_attack);
                    self.start_map_battle(map, attack);
                }
                31 | 32 => {
                    let n = wr::baked::BATTLE_MAPS.len();
                    w.map_pick = if i == 31 {
                        (w.map_pick + n - 1) % n
                    } else {
                        (w.map_pick + 1) % n
                    };
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
                33 => {
                    w.map_attack = !w.map_attack;
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
                4 => {
                    // The old way of the table: Chiquita and the Queen over the map.
                    self.war = None;
                    self.start_guion_at_place("mesa-guerra");
                }
                5 => {
                    w.page = WarPage::Table(0);
                    // back on «Batallas rápidas»
                    w.focus = if saved { 2 } else { 1 };
                    ui_sfx::play(ui_sfx::UiSfx::Back);
                }
                _ => self.leave_war(),
            },
            WarHit::Quick(i) => self.start_quick_battle(i as usize),
            WarHit::Speed(s) => {
                if let Some(run) = w.battle.as_mut() {
                    if s == 0 {
                        if !run.b.deploying {
                            run.b.paused = !run.b.paused;
                        }
                    } else {
                        run.b.speed = s as f32;
                        if !run.b.deploying {
                            run.b.paused = false;
                        }
                    }
                    ui_sfx::play(ui_sfx::UiSfx::Move);
                }
            }
            WarHit::Help => {
                if let Some(run) = w.battle.as_mut() {
                    run.help = !run.help;
                    ui_sfx::play(if run.help {
                        ui_sfx::UiSfx::Open
                    } else {
                        ui_sfx::UiSfx::Close
                    });
                }
            }
            WarHit::HelpClose => {
                if let Some(run) = w.battle.as_mut() {
                    run.help = false;
                }
            }
            WarHit::Retreat => {
                if let Some(run) = w.battle.as_mut() {
                    if !run.b.deploying && run.result.is_none() {
                        run.confirm = true;
                        ui_sfx::play(ui_sfx::UiSfx::Open);
                    }
                }
            }
            WarHit::Confirm(yes) => {
                if let Some(run) = w.battle.as_mut() {
                    run.confirm = false;
                    if yes {
                        run.b.withdraw();
                        let res = run.b.result();
                        run.result = Some((res, 0.0));
                        ui_sfx::play(ui_sfx::UiSfx::Fail);
                    }
                }
            }
            WarHit::Begin => {
                if let Some(run) = w.battle.as_mut() {
                    if run.b.deploying {
                        run.b.begin();
                        let title = run.b.title.clone();
                        run.messages.push_front((
                            format!("¡Comienza la batalla! {}", title),
                            0xf2b54a,
                            0.0,
                        ));
                        if let Some(audio) = self.audio.as_mut() {
                            let at = w.cam.position;
                            audio.play(&sound(dungeon::sound::Sfx::Summon, at, 0.9), at, Vec3::Z);
                        }
                    }
                }
            }
            WarHit::Power(k) => {
                if let Some(run) = w.battle.as_mut() {
                    let p = wr::powers::Power::ALL[(k as usize).min(5)];
                    let ready = run.b.sim.powers.as_ref().is_some_and(|pw| pw.ready(p));
                    if run.power_aim == Some(p) {
                        run.power_aim = None;
                        ui_sfx::play(ui_sfx::UiSfx::Back);
                    } else if ready && !run.b.deploying && run.result.is_none() {
                        run.power_aim = Some(p);
                        run.build = None;
                        run.ghost = None;
                        Self::select_engine(run, None);
                        ui_sfx::play(ui_sfx::UiSfx::Select);
                    } else {
                        ui_sfx::play(ui_sfx::UiSfx::Error);
                    }
                }
            }
            WarHit::Build(k) => {
                if let Some(run) = w.battle.as_mut() {
                    let kind =
                        wr::siege::Kind::ALL[(k as usize).min(wr::siege::Kind::ALL.len() - 1)];
                    run.build = if run.build == Some(kind) {
                        None
                    } else {
                        Some(kind)
                    };
                    run.ghost = None;
                    Self::select_engine(run, None);
                    ui_sfx::play(ui_sfx::UiSfx::Select);
                }
            }
            WarHit::Order(o) => {
                if let Some(run) = w.battle.as_mut() {
                    let sel = run.b.selected();
                    match o {
                        0 => {
                            for &i in &sel {
                                run.b.sim.order_halt(i);
                            }
                            ui_sfx::play(ui_sfx::UiSfx::Select);
                        }
                        1 => {
                            run.run_mode = !run.run_mode;
                            ui_sfx::play(ui_sfx::UiSfx::Move);
                        }
                        2 => {
                            let list: Vec<usize> = sel
                                .iter()
                                .copied()
                                .filter(|&i| run.b.sim.regiments[i].ty.ranged.is_some())
                                .collect();
                            let on = !list.iter().all(|&i| run.b.sim.regiments[i].fire_at_will);
                            for i in list {
                                run.b.sim.regiments[i].fire_at_will = on;
                            }
                            ui_sfx::play(ui_sfx::UiSfx::Move);
                        }
                        3 => {
                            let list: Vec<usize> = sel
                                .iter()
                                .copied()
                                .filter(|&i| run.b.sim.regiments[i].ty.role == Role::Skirmisher)
                                .collect();
                            let on = !list.iter().all(|&i| run.b.sim.regiments[i].skirmish);
                            for i in list {
                                run.b.sim.regiments[i].skirmish = on;
                            }
                            ui_sfx::play(ui_sfx::UiSfx::Move);
                        }
                        _ => {
                            if !run.b.cast_spark() {
                                ui_sfx::play(ui_sfx::UiSfx::Error);
                            }
                        }
                    }
                }
            }
            WarHit::Result(k) => {
                let Some(run) = w.battle.take() else {
                    return;
                };
                match k {
                    0 => {
                        let res = run.result.map(|(r, _)| r).unwrap_or_else(|| run.b.result());
                        let fight = run.from_campaign.clone();
                        self.back_to_campaign(res, fight);
                    }
                    1 => {
                        if let Some(id) = run.scenario.clone() {
                            if let Some(rest) = id.strip_prefix("mapa:") {
                                let (map, attack) = rest.split_once(':').unwrap_or((rest, "1"));
                                let (map, attack) = (map.to_owned(), attack == "1");
                                self.start_map_battle(&map, attack);
                            } else if let Some(i) = SCENARIOS.iter().position(|s| s.id == id) {
                                self.start_quick_battle(i);
                            }
                        }
                    }
                    _ => {
                        if let Some(w) = self.war.as_mut() {
                            w.page = WarPage::Table(1);
                        }
                    }
                }
            }
            WarHit::EndTurn => self.campaign_end_turn(),
            WarHit::Go(p) => {
                self.campaign_order(p as usize);
                self.build_war_view();
            }
            WarHit::NextArmy => self.next_army(),
            WarHit::Panel(i) => self.campaign_panel(i),
            WarHit::Modal(i) => self.campaign_modal(i),
            WarHit::Card(reg) => {
                let now = self.elapsed;
                self.war_card(reg, now);
            }
            WarHit::Minimap(u, v) => self.war_minimap(u, v, false),
        }
        self.build_war_view();
    }

    /// The table's «Estudiar el mapa»: its guion, as before.
    fn start_guion_at_place(&mut self, place: &str) {
        let _ = self.start_talk(None, Some(place));
    }

    // ------------------------------------------------------------------ the step
    pub(super) fn tick_war(&mut self, dt: f32) {
        // The world's clock runs here too, as in every place: the HUD is drawn again only when its
        // tick moves, so without it the war's menus and banners stayed frozen on their first frame.
        self.elapsed += dt;
        let pointer = self.shop_pointer_position();
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let dt = dt.min(0.1);
        w.time += dt;
        w.pointer = pointer.or(w.qa_pointer);
        let k = w.keys;
        let mut pan = (
            (k.right as u8 as f32) - (k.left as u8 as f32),
            (k.up as u8 as f32) - (k.down as u8 as f32),
        );
        if let Some((x, y)) = pointer {
            let m = 6.0;
            if x < m {
                pan.0 -= 1.0;
            }
            if x > SCREEN_W - m {
                pan.0 += 1.0;
            }
            if y < m {
                pan.1 += 1.0;
            }
            if y > SCREEN_H - m {
                pan.1 -= 1.0;
            }
        }
        let turn = (k.turn_l as u8 as f32) - (k.turn_r as u8 as f32);
        // while someone talks over it the war holds still
        if self.talk.is_some() || self.stage.is_some() {
            self.build_war_view();
            return;
        }
        let Some(w) = self.war.as_mut() else {
            return;
        };
        match w.page {
            WarPage::Battle => {
                if let Some(run) = w.battle.as_ref() {
                    w.cam
                        .update(dt, pan, turn, k.shift, &Self::battle_ground(&run.b));
                }
                self.tick_battle(dt);
            }
            WarPage::Campaign => {
                if let Some(c) = w.camp.as_ref() {
                    let land = &c.land;
                    w.cam
                        .update(dt, pan, turn, k.shift, &|x: f32, z: f32| land.top_at(x, z));
                }
                self.tick_campaign(dt);
            }
            WarPage::Table(_) => {}
        }
        self.arm_battle();
        self.war_story_tick();
        self.build_war_view();
    }

    /// The first step of a battle: her war powers, the field's relic, the siege's scene.
    fn arm_battle(&mut self) {
        let powers = self.war_powers();
        let found: Vec<String> = super::war_story::relics()
            .iter()
            .filter(|r| self.story.get(&format!("reliquia_{}", r.id)) > 0)
            .map(|r| r.id.clone())
            .collect();
        let Some(run) = self.war.as_mut().and_then(|w| w.battle.as_mut()) else {
            return;
        };
        if run.armed {
            return;
        }
        run.armed = true;
        run.b.sim.powers = Some(wr::powers::Powers::new(powers));
        let region = wr::baked::BATTLE_MAPS
            .iter()
            .find(|(m, _)| *m == run.b.setup.map)
            .map(|(_, r)| *r);
        if let Some(r) = region.and_then(|reg| {
            super::war_story::relics()
                .iter()
                .find(|r| r.region == reg && r.on_field() && !found.contains(&r.id))
        }) {
            let (x, z) = super::war_story::relic_spot(&run.b);
            run.relic = Some(super::war_story::FieldRelic {
                id: r.id.clone(),
                name: r.nombre.clone(),
                x,
                z,
                taken: false,
            });
            run.messages.push_front((
                format!("Reliquia en el campo: {} (la luz dorada)", r.nombre),
                0xf2b54a,
                0.0,
            ));
        }
        let siege = run
            .b
            .sim
            .siege
            .as_ref()
            .is_some_and(|s| s.mode == wr::siege::Mode::Siege);
        if siege {
            self.queue_war_scene("guerra.asedio");
        }
    }

    fn tick_battle(&mut self, dt: f32) {
        let listener = self.war.as_ref().map(|w| (w.cam.position, w.cam.camera()));
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let camera = w.cam.camera();
        let Some(run) = w.battle.as_mut() else {
            return;
        };
        if run.result.is_none() && !run.help && !run.confirm {
            run.b.update(dt);
        }
        // a regiment of hers on the relic picks it up
        let mut picked = None;
        if let Some(rel) = run.relic.as_mut().filter(|r| !r.taken) {
            let sim = &run.b.sim;
            let near = sim
                .soldiers_near(rel.x, rel.z, 5.0)
                .into_iter()
                .any(|i| sim.regiments[sim.reg[i]].side == 0);
            if near {
                rel.taken = true;
                picked = Some((rel.id.clone(), rel.x, rel.z));
            }
        }
        for g in &mut run.ghosts {
            g.t += dt;
        }
        run.ghosts.retain(|g| g.t < 1.6);
        for m in &mut run.messages {
            m.2 += dt;
        }
        run.messages.retain(|m| m.2 < 7.4);
        run.messages.truncate(6);
        if let Some((_, age)) = run.result.as_mut() {
            *age += dt;
        }
        run.hover_t -= dt;
        if run.hover_t <= 0.0 {
            run.hover_t = 0.1;
            run.hover = w
                .pointer
                .filter(|&(x, y)| !w.view.over_hud(x, y))
                .and_then(|(x, y)| Self::pick_regiment(run, camera, x, y, None));
            let at = w
                .pointer
                .filter(|&(x, y)| !w.view.over_hud(x, y))
                .and_then(|(x, y)| ground_at(camera, x, y, &Self::battle_ground(&run.b), 2400.0));
            run.engine_hover = at.and_then(|(gx, gz)| engine_at(&run.b, gx, gz, None));
            run.power_at = run.power_aim.and(at);
            run.ghost = match (run.build, at) {
                (Some(kind), Some((gx, gz))) => {
                    let yaw = build_yaw(&run.b, kind, gx, gz);
                    let why = run.b.sim.siege.as_ref().and_then(|s| {
                        s.check(
                            &run.b.sim,
                            0,
                            kind,
                            gx,
                            gz,
                            yaw,
                            run.b.deploying,
                            run.b.zone,
                        )
                        .err()
                    });
                    Some((gx, gz, yaw, why))
                }
                _ => None,
            };
        }
        // What happened, as messages, effects and sounds.
        let events = std::mem::take(&mut run.b.sim.events);
        let siege_events = run
            .b
            .sim
            .siege
            .as_mut()
            .map(|s| std::mem::take(&mut s.events))
            .unwrap_or_default();
        let power_events = run
            .b
            .sim
            .powers
            .as_mut()
            .map(|p| std::mem::take(&mut p.events))
            .unwrap_or_default();
        let mut fx = Vec::new();
        let mut sounds = Vec::new();
        let (mut sparks, mut clashes, mut thuds) = (0, 0, 0);
        let sim = &run.b.sim;
        let field = &sim.field;
        let near = |x: f32, z: f32| {
            let d = (camera.position - Vec3::new(x, field.walk_height(x, z), z)).length();
            (1.0 - d / 420.0).clamp(0.05, 1.0)
        };
        let at = |x: f32, z: f32| Vec3::new(x, field.walk_height(x, z) + 1.0, z);
        let name = |reg: usize| sim.regiments[reg].ty.call.to_owned();
        let sing = |reg: usize| sim.regiments[reg].ty.sing;
        let foe = |reg: usize| sim.regiments[reg].side == 1;
        let mut said = Vec::new();
        for e in &events {
            match *e {
                Event::Hit { x, z, big } | Event::Impact { x, z, big } => {
                    let hit = matches!(e, Event::Hit { .. });
                    if sparks < 40 {
                        fx.push(Fx::Sparks {
                            x,
                            y: field.walk_height(x, z) + 1.1,
                            z,
                            n: if hit { 2 } else { 6 },
                            color: [1.0, 0.78, 0.4],
                        });
                        sparks += 1;
                    }
                    if !hit {
                        fx.push(Fx::Puff {
                            x,
                            y: field.walk_height(x, z),
                            z,
                            n: if big { 3 } else { 1 },
                            size: if big { 2.4 } else { 1.4 },
                            strength: 1.4,
                        });
                        if thuds < 3 {
                            sounds.push(sound(
                                dungeon::sound::Sfx::Rubble,
                                at(x, z),
                                0.6 * near(x, z),
                            ));
                            thuds += 1;
                        }
                    } else if clashes < 4 && (x * 13.1 + z * 7.3).sin() > 0.3 {
                        sounds.push(sound(
                            dungeon::sound::Sfx::EnemySwing,
                            at(x, z),
                            0.55 * near(x, z),
                        ));
                        clashes += 1;
                    }
                }
                Event::Brace { x, z } => {
                    fx.push(Fx::Sparks {
                        x,
                        y: field.walk_height(x, z) + 1.1,
                        z,
                        n: 6,
                        color: [1.0, 1.0, 1.0],
                    });
                    fx.push(Fx::Puff {
                        x,
                        y: field.walk_height(x, z),
                        z,
                        n: 1,
                        size: 1.4,
                        strength: 1.4,
                    });
                    if thuds < 3 {
                        sounds.push(sound(
                            dungeon::sound::Sfx::HitBone,
                            at(x, z),
                            0.6 * near(x, z),
                        ));
                        thuds += 1;
                    }
                }
                Event::Clash { x, z } => {
                    if clashes < 4 && (x * 5.7 + z * 3.1).sin() > 0.76 {
                        sounds.push(sound(
                            dungeon::sound::Sfx::Swing,
                            at(x, z),
                            0.4 * near(x, z),
                        ));
                        clashes += 1;
                    }
                }
                Event::Block { x, z } => {
                    if sparks < 40 {
                        fx.push(Fx::Sparks {
                            x,
                            y: field.walk_height(x, z) + 1.2,
                            z,
                            n: 3,
                            color: [1.0, 1.0, 1.0],
                        });
                        sparks += 1;
                    }
                }
                Event::Glow { x, z } => fx.push(Fx::Sparks {
                    x,
                    y: field.walk_height(x, z) + 0.6,
                    z,
                    n: 3,
                    color: [1.0, 0.9, 0.6],
                }),
                Event::Fire { x, z, kind, .. } => {
                    if run.sound_budget < 6.0 {
                        let s = match kind {
                            wd::Projectile::Light => dungeon::sound::Sfx::SpellCharge,
                            _ => dungeon::sound::Sfx::CrossbowShot,
                        };
                        sounds.push(sound(s, at(x, z), 0.5 * near(x, z)));
                        run.sound_budget += 1.0;
                    }
                }
                Event::Pierce { .. } => {}
                Event::Death { x, z, large, .. } => {
                    if large && thuds < 3 {
                        sounds.push(sound(
                            dungeon::sound::Sfx::Rubble,
                            at(x, z),
                            0.5 * near(x, z),
                        ));
                        thuds += 1;
                    }
                }
                Event::Charge { reg, x, z } => {
                    let r = &sim.regiments[reg];
                    if matches!(r.ty.model, "orc" | "dimmed" | "wolf" | "varrak")
                        || r.ty.id == "varrak"
                    {
                        sounds.push(sound(dungeon::sound::Sfx::Roar, at(x, z), 0.8 * near(x, z)));
                    } else if r.ty.role == Role::Cavalry {
                        sounds.push(sound(dungeon::sound::Sfx::Summon, at(x, z), 0.6));
                    }
                    if matches!(r.ty.role, Role::Cavalry | Role::Beast | Role::Hero) || r.side == 0
                    {
                        let line = if sing(reg) {
                            format!("¡{} carga!", name(reg))
                        } else {
                            format!("¡{} cargan!", name(reg))
                        };
                        said.push((line, faction_edge(r.ty.faction), foe(reg)));
                    }
                }
                Event::Rout { reg } => {
                    let line = if sing(reg) {
                        format!("{} huye", name(reg))
                    } else {
                        format!("{} huyen", name(reg))
                    };
                    said.push((line, if foe(reg) { 0x4fd08a } else { 0xe2334a }, foe(reg)));
                }
                Event::Shatter { reg } => {
                    let line = if sing(reg) {
                        format!("{} se dispersa", name(reg))
                    } else {
                        format!("{} se dispersan", name(reg))
                    };
                    said.push((line, if foe(reg) { 0x4fd08a } else { 0xe2334a }, foe(reg)));
                }
                Event::Rally { reg } => {
                    let line = if sing(reg) {
                        format!("{} se reagrupa", name(reg))
                    } else {
                        format!("{} se reagrupan", name(reg))
                    };
                    said.push((line, 0xf2b54a, foe(reg)));
                }
                Event::Dimmed { reg } => {
                    let line = if sing(reg) {
                        format!("{} se apaga: ya no obedece", name(reg))
                    } else {
                        format!("{} se apagan: ya no obedecen", name(reg))
                    };
                    said.push((line, 0x9a5cff, false));
                }
                Event::Rekindled { reg } => {
                    let line = if sing(reg) {
                        format!("{} recupera la chispa", name(reg))
                    } else {
                        format!("{} recuperan la chispa", name(reg))
                    };
                    said.push((line, 0xc79aff, false));
                }
                Event::Crumble { reg } => {
                    if !sim.regiments[reg].crumble_told {
                        let line = if sing(reg) {
                            format!("{} se deshace", name(reg))
                        } else {
                            format!("{} se deshacen", name(reg))
                        };
                        said.push((line, 0xa898b8, false));
                    }
                }
                Event::Spark { x, z, r, .. } => {
                    fx.push(Fx::Spark { x, z, r });
                    sounds.push(sound(dungeon::sound::Sfx::SpellBurst, at(x, z), 1.0));
                    sounds.push(sound(dungeon::sound::Sfx::Shockwave, at(x, z), 0.8));
                    said.push(("¡La Reina libera la chispa!".to_owned(), 0x9a5cff, false));
                }
                Event::Awaken { x, z, .. } => {
                    fx.push(Fx::Pillar {
                        x,
                        z,
                        color: [1.0, 0.72, 0.3],
                    });
                    sounds.push(sound(dungeon::sound::Sfx::SealOpen, at(x, z), 1.0));
                    said.push((
                        "«Majestad… Ha tardado usted.» Varrak despierta".to_owned(),
                        0xf2b54a,
                        false,
                    ));
                }
                Event::QueenDown { reg } => {
                    let r = &sim.regiments[reg];
                    fx.push(Fx::Pillar {
                        x: r.cx,
                        z: r.cz,
                        color: [0.7, 0.45, 1.0],
                    });
                    said.push((
                        "La Reina cae herida: la chispa parpadea".to_owned(),
                        0xe2334a,
                        false,
                    ));
                }
                Event::HeroDown { reg } => {
                    said.push((
                        format!("{} cae en combate", sim.regiments[reg].ty.name),
                        0x4fd08a,
                        false,
                    ));
                }
                Event::Arrive { .. } | Event::End { .. } => {}
            }
        }
        // the siege: stones landing, walls breaking, engines built and wrecked
        if let Some(siege) = &sim.siege {
            use wr::siege::{Kind as Eng, SiegeEvent as Se};
            let mine_hold = siege.defender == Some(0);
            let top = |x: f32, z: f32| field.top_at(x, z).0;
            for e in &siege_events {
                match *e {
                    Se::Shot { engine } => {
                        let en = &siege.engines[engine];
                        if en.kind.stones() {
                            fx.push(Fx::Puff {
                                x: en.x,
                                y: field.walk_height(en.x, en.z),
                                z: en.z,
                                n: 2,
                                size: 1.6,
                                strength: 1.0,
                            });
                        }
                        if run.sound_budget < 6.0 {
                            let s = if en.kind.bolts() {
                                dungeon::sound::Sfx::CrossbowShot
                            } else {
                                dungeon::sound::Sfx::Swing
                            };
                            sounds.push(sound(s, at(en.x, en.z), 0.8 * near(en.x, en.z)));
                            run.sound_budget += 1.0;
                        }
                    }
                    Se::Impact { x, z, big } => {
                        let y = top(x, z);
                        fx.push(Fx::Puff {
                            x,
                            y,
                            z,
                            n: if big { 9 } else { 5 },
                            size: if big { 3.6 } else { 2.4 },
                            strength: 2.6,
                        });
                        fx.push(Fx::Sparks {
                            x,
                            y: y + 0.6,
                            z,
                            n: 8,
                            color: [0.9, 0.82, 0.7],
                        });
                        sounds.push(sound(dungeon::sound::Sfx::Rubble, at(x, z), near(x, z)));
                    }
                    Se::WorkHit { .. } => {}
                    Se::Breach { work, x, z } => {
                        let y = top(x, z);
                        for k in 0..3 {
                            fx.push(Fx::Puff {
                                x: x + k as f32 * 1.7 - 1.7,
                                y: y + k as f32 * 2.0,
                                z,
                                n: 10,
                                size: 5.0,
                                strength: 3.0,
                            });
                        }
                        sounds.push(sound(dungeon::sound::Sfx::Shockwave, at(x, z), 1.0));
                        let what = match siege.works.get(work).map(|w| w.kind) {
                            Some(wr::baked::WorkKind::Tower) => "¡Se derrumba una torre!",
                            Some(wr::baked::WorkKind::Palisade) => "¡La empalizada cede!",
                            _ => "¡Brecha en la muralla!",
                        };
                        said.push((
                            what.to_owned(),
                            if mine_hold { 0xe2334a } else { 0x4fd08a },
                            false,
                        ));
                    }
                    Se::GateDown { x, z, .. } => {
                        fx.push(Fx::Puff {
                            x,
                            y: field.walk_height(x, z) + 2.0,
                            z,
                            n: 14,
                            size: 4.0,
                            strength: 3.0,
                        });
                        sounds.push(sound(dungeon::sound::Sfx::Shockwave, at(x, z), 1.0));
                        said.push((
                            "¡La puerta ha caído!".to_owned(),
                            if mine_hold { 0xe2334a } else { 0x4fd08a },
                            false,
                        ));
                    }
                    Se::KeepDown { .. } => {
                        said.push((
                            "¡El torreón se derrumba!".to_owned(),
                            if mine_hold { 0xe2334a } else { 0x4fd08a },
                            false,
                        ));
                    }
                    Se::Built { engine } => {
                        let en = &siege.engines[engine];
                        if en.side == 0 && sim.time > 0.0 {
                            said.push((format!("Construido: {}", en.spec().name), 0xf2b54a, false));
                        }
                    }
                    Se::Wrecked { engine } => {
                        let en = &siege.engines[engine];
                        fx.push(Fx::Puff {
                            x: en.x,
                            y: field.walk_height(en.x, en.z) + 1.0,
                            z: en.z,
                            n: 8,
                            size: 3.0,
                            strength: 2.0,
                        });
                        fx.push(Fx::Sparks {
                            x: en.x,
                            y: field.walk_height(en.x, en.z) + 2.0,
                            z: en.z,
                            n: 14,
                            color: [1.0, 0.55, 0.2],
                        });
                        sounds.push(sound(
                            dungeon::sound::Sfx::Rubble,
                            at(en.x, en.z),
                            near(en.x, en.z),
                        ));
                        let name = en.spec().name;
                        if en.side == 0 {
                            said.push((
                                format!("Han destruido tu {}", name.to_lowercase()),
                                0xe2334a,
                                false,
                            ));
                        } else {
                            said.push((format!("{name} enemiga destruida"), 0x4fd08a, false));
                        }
                    }
                    Se::Docked { engine } => {
                        let en = &siege.engines[engine];
                        let line = match en.kind {
                            Eng::TorreAsedio => {
                                "La torre de asedio baja el puente sobre la muralla"
                            }
                            _ => "El ariete llega a la puerta",
                        };
                        said.push((
                            line.to_owned(),
                            if en.side == 0 { 0xf2b54a } else { 0xe2334a },
                            en.side == 1,
                        ));
                    }
                    Se::Ram { engine } => {
                        let en = &siege.engines[engine];
                        if thuds < 3 {
                            sounds.push(sound(
                                dungeon::sound::Sfx::HitBone,
                                at(en.x, en.z),
                                near(en.x, en.z),
                            ));
                            thuds += 1;
                        }
                        fx.push(Fx::Puff {
                            x: en.x + en.yaw.sin() * 3.2,
                            y: field.walk_height(en.x, en.z) + 1.2,
                            z: en.z + en.yaw.cos() * 3.2,
                            n: 3,
                            size: 1.8,
                            strength: 1.6,
                        });
                    }
                    Se::Held { side } => {
                        said.push((
                            if side == 0 {
                                "¡Tus tropas toman el patio del torreón! Aguanta ahí".to_owned()
                            } else {
                                "¡El enemigo está tomando el torreón!".to_owned()
                            },
                            if side == 0 { 0x4fd08a } else { 0xe2334a },
                            false,
                        ));
                    }
                    Se::Captured { side } => {
                        said.push((
                            if side == 0 {
                                "La plaza es tuya".to_owned()
                            } else {
                                "La plaza ha caído".to_owned()
                            },
                            if side == 0 { 0x4fd08a } else { 0xe2334a },
                            false,
                        ));
                    }
                }
            }
        }
        // her powers
        for e in &power_events {
            use wr::powers::PowerEvent as Pe;
            match *e {
                Pe::Cast { power, x, z, r } => {
                    let c = wd::rgb(power.spec().color);
                    fx.push(Fx::Ring { x, z, r, color: c });
                    fx.push(Fx::Pillar { x, z, color: c });
                    sounds.push(sound(dungeon::sound::Sfx::SpellBurst, at(x, z), 1.0));
                    said.push((format!("¡{}!", power.spec().name), 0x9a5cff, false));
                }
                Pe::Ember { x, z, .. } => {
                    fx.push(Fx::Ember { x, z });
                    if thuds < 3 {
                        sounds.push(sound(dungeon::sound::Sfx::Rubble, at(x, z), near(x, z)));
                        thuds += 1;
                    }
                }
                Pe::Healed { x, z } => fx.push(Fx::Sparks {
                    x,
                    y: field.walk_height(x, z) + 1.2,
                    z,
                    n: 4,
                    color: [0.5, 1.0, 0.8],
                }),
                Pe::Shaken { x, z } => fx.push(Fx::Puff {
                    x,
                    y: field.walk_height(x, z),
                    z,
                    n: 3,
                    size: 2.2,
                    strength: 2.4,
                }),
            }
        }
        if let Some((_, x, z)) = picked.as_ref() {
            fx.push(Fx::Pillar {
                x: *x,
                z: *z,
                color: [1.0, 0.8, 0.35],
            });
            sounds.push(sound(dungeon::sound::Sfx::SealOpen, at(*x, *z), 1.0));
        }
        // crumbled once is told once
        for e in &events {
            if let Event::Crumble { reg } = *e {
                run.b.sim.regiments[reg].crumble_told = true;
            }
        }
        run.sound_budget = (run.sound_budget - 0.5).max(0.0);
        for (text, edge, enemy) in said {
            let text = if enemy {
                format!("{text} (enemigo)")
            } else {
                text
            };
            run.messages.push_front((text, edge, 0.0));
        }
        run.messages.truncate(6);
        // The end, a moment after it is decided.
        if run.b.sim.over.is_some() && run.result.is_none() {
            run.end_t += dt;
            if run.end_t > 2.4 {
                let res = run.b.result();
                let win = res.winner == 0;
                run.result = Some((res, 0.0));
                ui_sfx::play(if win {
                    ui_sfx::UiSfx::Deal
                } else {
                    ui_sfx::UiSfx::Fail
                });
            }
        }
        for f in fx {
            w.push_fx(f);
        }
        if let Some((listener, cam)) = listener {
            self.poll_audio_boot();
            if !self.audio_tried {
                self.audio_tried = true;
                self.audio = game_audio::GameAudio::new();
            }
            if let Some(audio) = self.audio.as_mut() {
                let forward = (cam.target - cam.position).normalize_or_zero();
                for s in &sounds {
                    audio.play(s, listener, forward);
                }
            }
        }
        if let Some((id, ..)) = picked {
            self.grant_relic(&id);
        }
    }

    // ------------------------------------------------------------------ the campaign
    /// Where an army stands in its province: the armies there in a row south of the settlement,
    /// each base clear of the next (a base grows with its regiments).
    fn slot_of(s: &Campaign, land: &wr::campaign_map::Land, army: &wr::rules::Army) -> Vec3 {
        let p = wd::province(&army.at);
        let here: Vec<&wr::rules::Army> = s
            .armies
            .iter()
            .filter(|a| a.at == army.at && !a.units.is_empty())
            .collect();
        let k = here.iter().position(|a| a.id == army.id).unwrap_or(0);
        let radii: Vec<f32> = here.iter().map(|a| formation(a.units.len()).1).collect();
        let gap = 1.6;
        let total = radii.iter().sum::<f32>() * 2.0 + gap * radii.len().saturating_sub(1) as f32;
        let mut x = -total / 2.0;
        let mut ox = 0.0;
        for (i, r) in radii.iter().enumerate() {
            if i == k {
                ox = x + r;
                break;
            }
            x += 2.0 * r + gap;
        }
        let (sx, sz) = p.site();
        let (x, z) = (sx + ox, sz + 10.0);
        Vec3::new(x, land.top_at(x, z) + 0.05, z)
    }

    fn sync_tokens(c: &mut CampaignRun, s: &Campaign, instant: bool) {
        c.tokens
            .retain(|id, _| s.army(id).is_some_and(|a| !a.units.is_empty()));
        for a in s.armies.iter().filter(|a| !a.units.is_empty()) {
            let want = Self::slot_of(s, &c.land, a);
            match c.tokens.get_mut(&a.id) {
                Some(t) => {
                    if instant {
                        t.at = want;
                        t.to = want;
                        t.from = None;
                        t.t = 1.0;
                    } else if t.from.is_none() {
                        t.to = want;
                    }
                }
                None => {
                    c.tokens.insert(
                        a.id.clone(),
                        Token {
                            at: want,
                            from: None,
                            to: want,
                            t: 1.0,
                            yaw: 0.0,
                        },
                    );
                }
            }
        }
    }

    /// The fog: which provinces the Queen's people cannot see.
    fn fogged(s: &Campaign) -> Vec<bool> {
        s.visible().into_iter().map(|v| !v).collect()
    }

    fn sel_army<'a>(c: &CampaignRun, s: &'a Campaign) -> Option<&'a wr::rules::Army> {
        match &c.sel {
            Some(Sel::Army(id)) => s.army(id),
            _ => None,
        }
    }

    fn army_under(
        c: &CampaignRun,
        s: &Campaign,
        camera: Camera3D,
        sx: f32,
        sy: f32,
    ) -> Option<String> {
        let fog = Self::fogged(s);
        let mut best = None;
        let mut bd = 40.0f32;
        for a in s.armies.iter().filter(|a| !a.units.is_empty()) {
            if a.faction != Faction::Reina && fog[wd::province_index(&a.at)] {
                continue;
            }
            let Some(t) = c.tokens.get(&a.id) else {
                continue;
            };
            if let Some(p) = project(camera, t.at + Vec3::new(0.0, 3.0, 0.0)) {
                let r = formation(a.units.len()).1;
                let reach = project(camera, t.at + Vec3::new(r, 3.0, 0.0))
                    .map_or(0.0, |q| (q[0] - p[0]).hypot(q[1] - p[1]));
                // (a big formation is picked anywhere over its base)
                let d = (p[0] - sx).hypot(p[1] - sy) - (reach - 40.0).max(0.0);
                if d < bd {
                    bd = d;
                    best = Some(a.id.clone());
                }
            }
        }
        best
    }

    fn province_under(c: &CampaignRun, camera: Camera3D, sx: f32, sy: f32) -> Option<usize> {
        let land = &c.land;
        let g = ground_at(camera, sx, sy, &|x: f32, z: f32| land.top_at(x, z), 1600.0)?;
        c.provinces.at(g.0, g.1)
    }

    fn campaign_release(&mut self, press: Press, x: f32, y: f32) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        if press.moved || press.button == 1 {
            return;
        }
        let camera = w.cam.camera();
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_ref()) else {
            return;
        };
        if c.modal.is_some() || c.busy {
            return;
        }
        if press.button == 0 {
            let plate = c
                .tags
                .borrow()
                .placed()
                .iter()
                .rev()
                .find(|t| {
                    t.id >= 1000
                        && t.alpha > 0.5
                        && x >= t.rect[0]
                        && x <= t.rect[0] + t.rect[2]
                        && y >= t.rect[1]
                        && y <= t.rect[1] + t.rect[3]
                })
                .and_then(|t| s.armies.get(t.id - 1000))
                .map(|a| a.id.clone());
            if let Some(id) = plate.or_else(|| Self::army_under(c, s, camera, x, y)) {
                c.sel = Some(Sel::Army(id));
            } else {
                let p = Self::province_under(c, camera, x, y);
                // with one of her field armies chosen, a click on another province sends it there
                let send = Self::sel_army(c, s)
                    .filter(|a| {
                        a.faction == Faction::Reina && !a.garrison && s.phase == CampPhase::Player
                    })
                    .zip(p)
                    .filter(|(a, p)| a.at != PROVINCES[*p].id)
                    .map(|(_, p)| p);
                if let Some(p) = send {
                    self.campaign_go(p);
                    self.build_war_view();
                    return;
                }
                c.sel = p.map(Sel::Province);
            }
            c.paint_version += 1;
            ui_sfx::play(ui_sfx::UiSfx::Move);
        } else if press.button == 2 {
            let target = Self::army_under(c, s, camera, x, y)
                .and_then(|id| s.army(&id).map(|a| wd::province_index(&a.at)))
                .or_else(|| Self::province_under(c, camera, x, y));
            if let Some(p) = target {
                self.campaign_go(p);
            }
        }
        self.build_war_view();
    }

    /// The Queen's armies on a march with moves left: they walk now, one after the other.
    fn queue_marches(c: &mut CampaignRun, s: &Campaign) {
        c.marching = s
            .armies
            .iter()
            .filter(|a| {
                a.faction == Faction::Reina && a.march.is_some() && a.mp > 0 && !a.units.is_empty()
            })
            .map(|a| a.id.clone())
            .collect();
    }

    /// One army walks its march as far as its moves go this turn: through her own lands, taking
    /// what is another's (that ends its turn), stopping at a battle (its card comes up). Where the
    /// eye should go.
    fn march_step(c: &mut CampaignRun, s: &mut Campaign, id: &str) -> Option<(f32, f32)> {
        let start = s.army(id)?.at.clone();
        let mut battle = None;
        loop {
            let a = s.army(id)?.clone();
            let Some(target) = a.march.clone() else {
                break;
            };
            if a.at == target {
                s.army_mut(id)?.march = None;
                s.note(
                    format!("{} llega a {}.", a.name, wd::province(&target).name),
                    LogKind::Good,
                );
                break;
            }
            if a.mp == 0 {
                break;
            }
            let Some(way) = s.route(&a, &target) else {
                s.army_mut(id)?.march = None;
                break;
            };
            match s.move_army(id, way[0]) {
                MoveResult::Move => {}
                MoveResult::Capture => ui_sfx::play(ui_sfx::UiSfx::Deal),
                MoveResult::Battle(b) => {
                    battle = Some(b);
                    break;
                }
                MoveResult::Blocked => {
                    s.army_mut(id)?.march = None;
                    s.note(
                        format!("{} no puede seguir su marcha.", a.name),
                        LogKind::Info,
                    );
                    break;
                }
            }
        }
        let end = s.army(id)?.at.clone();
        if let Some(b) = battle {
            ui_sfx::play(ui_sfx::UiSfx::Equip);
            Self::slide(c, s, id, &end, &b.prov.clone(), true);
            c.battle_after = Some(b);
            c.wait = 0.8;
        } else if end != start {
            ui_sfx::play(ui_sfx::UiSfx::Select);
            Self::slide(c, s, id, &start, &end, false);
            c.wait = 0.5;
        }
        Self::sync_tokens(c, s, false);
        c.paint_version += 1;
        let p = wd::province(&end);
        Some((p.x, p.z))
    }

    /// The chosen army goes to province `p`: a step if it is next door, else a march.
    pub(crate) fn campaign_go(&mut self, p: usize) {
        let reach = {
            let (Some(c), Some(s)) = (
                self.war.as_ref().and_then(|w| w.camp.as_ref()),
                self.war_campaign.as_ref(),
            ) else {
                return;
            };
            let Some(a) = Self::sel_army(c, s) else {
                return;
            };
            if a.faction != Faction::Reina || a.garrison || s.phase != CampPhase::Player || c.busy {
                return;
            }
            s.reachable(a).contains(&PROVINCES[p].id) && a.mp > 0
        };
        // the story keeps her armies on this side of the river until the first act ends
        if self.river_closed(PROVINCES[p].id) {
            self.river_blocked();
            return;
        }
        if reach {
            self.campaign_order(p);
        } else {
            self.campaign_march(p);
        }
    }

    /// The chosen army marches to province `p`, turn after turn; it sets out at once if it can.
    fn campaign_march(&mut self, p: usize) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_mut()) else {
            return;
        };
        let Some(a) = Self::sel_army(c, s).cloned() else {
            return;
        };
        let to = PROVINCES[p].id;
        let Some(way) = s.route(&a, to) else {
            ui_sfx::play(ui_sfx::UiSfx::Error);
            return;
        };
        let turns = s.march_turns(&a, &way).last().copied().unwrap_or(1);
        if let Some(m) = s.army_mut(&a.id) {
            m.march = Some(to.to_owned());
        }
        s.note(
            if turns <= 1 {
                format!("{} marcha hacia {}.", a.name, PROVINCES[p].name)
            } else {
                format!(
                    "{} marcha hacia {} ({} turnos).",
                    a.name, PROVINCES[p].name, turns
                )
            },
            LogKind::Info,
        );
        ui_sfx::play(ui_sfx::UiSfx::Select);
        if a.mp > 0 {
            c.marching.push_front(a.id.clone());
        }
        c.paint_version += 1;
    }

    /// Chooses the next of the Queen's armies that can still move, and brings the eye to it.
    fn next_army(&mut self) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_ref()) else {
            return;
        };
        if s.phase != CampPhase::Player || c.busy || c.modal.is_some() {
            return;
        }
        let movable: Vec<&wr::rules::Army> = s.armies.iter().filter(|a| can_move(a)).collect();
        if movable.is_empty() {
            ui_sfx::play(ui_sfx::UiSfx::Error);
            return;
        }
        let now = match &c.sel {
            Some(Sel::Army(id)) => movable.iter().position(|a| &a.id == id),
            _ => None,
        };
        let next = movable[now.map_or(0, |i| (i + 1) % movable.len())];
        c.sel = Some(Sel::Army(next.id.clone()));
        c.paint_version += 1;
        if let Some(t) = c.tokens.get(&next.id) {
            let land = &c.land;
            w.cam
                .frame(t.at.x, t.at.z, None, None, false, &|x: f32, z: f32| {
                    land.top_at(x, z)
                });
        }
        ui_sfx::play(ui_sfx::UiSfx::Move);
        self.build_war_view();
    }

    /// The chosen army goes to (or attacks) province `p`.
    fn campaign_order(&mut self, p: usize) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_mut()) else {
            return;
        };
        let Some(a) = Self::sel_army(c, s).cloned() else {
            return;
        };
        if a.faction != Faction::Reina || s.phase != CampPhase::Player || c.busy {
            return;
        }
        let to = PROVINCES[p].id;
        if !s.reachable(&a).contains(&to) {
            ui_sfx::play(ui_sfx::UiSfx::Error);
            return;
        }
        let from = a.at.clone();
        if let Some(m) = s.army_mut(&a.id) {
            m.march = None;
        }
        let res = s.move_army(&a.id, to);
        match res {
            MoveResult::Blocked => {
                ui_sfx::play(ui_sfx::UiSfx::Error);
                return;
            }
            MoveResult::Battle(b) => {
                ui_sfx::play(ui_sfx::UiSfx::Equip);
                Self::slide(c, s, &a.id, &from, to, true);
                c.wait = 0.8;
                c.battle_after = Some(b);
            }
            r => {
                ui_sfx::play(ui_sfx::UiSfx::Select);
                // Walking into a friendly army joins it if there is room.
                let friend = s
                    .armies
                    .iter()
                    .find(|b| {
                        b.id != a.id && b.at == to && b.faction == Faction::Reina && !b.garrison
                    })
                    .map(|b| (b.id.clone(), b.units.len(), b.has("reina"), b.name.clone()));
                Self::slide(c, s, &a.id, &from, to, false);
                if let Some((fid, n, queen, fname)) = friend {
                    if n + a.units.len() <= wd::MAX_UNITS {
                        let (keep, give, kname, gname) = if a.has("reina") || !queen {
                            (a.id.clone(), fid, a.name.clone(), fname)
                        } else {
                            (fid, a.id.clone(), fname, a.name.clone())
                        };
                        if s.merge(&keep, &give) {
                            s.note(format!("{gname} se une a {kname}."), LogKind::Good);
                            c.sel = Some(Sel::Army(keep));
                            Self::sync_tokens(c, s, false);
                        }
                    }
                }
                if r == MoveResult::Capture {
                    ui_sfx::play(ui_sfx::UiSfx::Deal);
                }
            }
        }
        c.paint_version += 1;
    }

    /// A token hops from one province to the next (part of the way, for a battle).
    fn slide(c: &mut CampaignRun, s: &Campaign, army: &str, from: &str, to: &str, battle: bool) {
        let Some(a) = s.army(army) else {
            return;
        };
        let (fp, tp) = (wd::province(from), wd::province(to));
        let land = &c.land;
        let (fx, fz) = fp.site();
        let mut start = Vec3::new(fx, 0.0, fz + 10.0);
        let mut end = Self::slot_of(s, land, a);
        if battle {
            let (tx, tz) = (tp.x, tp.z + 10.0);
            end = Vec3::new(
                start.x + (tx - start.x) * 0.55,
                0.0,
                start.z + (tz - start.z) * 0.55,
            );
        }
        start.y = land.top_at(start.x, start.z);
        end.y = land.top_at(end.x, end.z);
        let t = c.tokens.entry(army.to_owned()).or_insert(Token {
            at: start,
            from: None,
            to: end,
            t: 1.0,
            yaw: 0.0,
        });
        t.from = Some(start);
        t.to = end;
        t.t = 0.0;
        t.yaw = (end.x - start.x).atan2(end.z - start.z);
    }

    /// The tokens again, after the map changed under them.
    pub(super) fn resync_tokens(c: &mut CampaignRun, s: &Campaign) {
        Self::sync_tokens(c, s, true);
    }

    fn campaign_panel(&mut self, i: u8) {
        if i == 103 {
            self.campaign_awaken();
            return;
        }
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_mut()) else {
            return;
        };
        if s.phase != CampPhase::Player || c.busy {
            return;
        }
        let (prov, army) = match &c.sel {
            Some(Sel::Army(id)) => match s.army(id) {
                Some(a) => (a.at.clone(), Some(a.clone())),
                None => return,
            },
            Some(Sel::Province(p)) => (PROVINCES[*p].id.to_owned(), None),
            None => return,
        };
        let opts = s.recruit_options(&prov, Faction::Reina);
        let mine = army.as_ref().is_none_or(|a| a.faction == Faction::Reina);
        if !mine {
            return;
        }
        let n = if s.prov(&prov).owner == Faction::Reina {
            opts.len()
        } else {
            0
        };
        if (i as usize) < n {
            let ty = opts[i as usize].ty;
            if let Some(id) = s.recruit(&prov, ty, Faction::Reina) {
                ui_sfx::play(ui_sfx::UiSfx::Coins);
                if army.is_none() {
                    c.sel = Some(Sel::Army(id));
                }
                Self::sync_tokens(c, s, true);
            } else {
                ui_sfx::play(ui_sfx::UiSfx::Error);
            }
        } else if i == 102 {
            if let Some(a) = army.as_ref().and_then(|a| s.army_mut(&a.id)) {
                a.march = None;
                ui_sfx::play(ui_sfx::UiSfx::Back);
            }
        } else if i == 100 {
            if s.rekindle(&prov) {
                ui_sfx::play(ui_sfx::UiSfx::Fuse);
            } else {
                ui_sfx::play(ui_sfx::UiSfx::Error);
            }
        } else if i == 101 {
            if let Some(a) = army {
                let other = s
                    .armies
                    .iter()
                    .find(|b| {
                        b.id != a.id && b.at == a.at && b.faction == Faction::Reina && !b.garrison
                    })
                    .map(|b| (b.id.clone(), b.has("reina")));
                if let Some((oid, queen)) = other {
                    let (keep, give) = if queen {
                        (oid, a.id.clone())
                    } else {
                        (a.id.clone(), oid)
                    };
                    if s.merge(&keep, &give) {
                        c.sel = Some(Sel::Army(keep));
                        ui_sfx::play(ui_sfx::UiSfx::Fuse);
                        Self::sync_tokens(c, s, true);
                    }
                }
            }
        }
        c.paint_version += 1;
    }

    fn campaign_modal(&mut self, i: u8) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let Some(c) = w.camp.as_mut() else {
            return;
        };
        let modal = c.modal.take();
        match modal {
            Some(CampModal::Intro) | None => {}
            Some(CampModal::End) => {
                if i == 1 {
                    self.start_campaign(true);
                } else {
                    w.page = WarPage::Table(0);
                    w.camp = None;
                }
            }
            Some(CampModal::Battle(b)) => match i {
                0 => self.campaign_fight(b),
                1 => self.campaign_auto(b),
                2 => {
                    // cancel (her own attack, and the march that led to it)
                    if let Some(s) = self.war_campaign.as_mut() {
                        s.pending = None;
                        if let Some(a) = s.army_mut(&b.attacker) {
                            a.march = None;
                        }
                        if let Some(c) = self.war.as_mut().and_then(|w| w.camp.as_mut()) {
                            Self::sync_tokens(c, s, true);
                        }
                    }
                }
                _ => {
                    // retreat
                    if let Some(s) = self.war_campaign.as_mut() {
                        let mine = if b.af == Faction::Reina {
                            b.attacker.clone()
                        } else {
                            b.defender.clone()
                        };
                        s.retreat_from(&b, &mine);
                        s.pending = None;
                        if let Some(c) = self.war.as_mut().and_then(|w| w.camp.as_mut()) {
                            Self::sync_tokens(c, s, true);
                            c.wait = 0.6;
                        }
                    }
                }
            },
        }
    }

    fn campaign_fight(&mut self, b: PendingBattle) {
        let Some(s) = self.war_campaign.as_ref() else {
            return;
        };
        let (Some(att), Some(def)) = (s.army(&b.attacker), s.army(&b.defender)) else {
            return;
        };
        let player_attacks = att.faction == Faction::Reina;
        let (mine, theirs) = if player_attacks {
            (att, def)
        } else {
            (def, att)
        };
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let scale = w.scale;
        let units = |army: &wr::rules::Army| -> Vec<SetupUnit> {
            army.units
                .iter()
                .enumerate()
                .map(|(k, u)| SetupUnit {
                    ty: u.ty.clone(),
                    men: Some(if wd::unit(&u.ty).role == Role::Hero {
                        1
                    } else {
                        ((u.men as f32 * scale).round() as u32).max(4)
                    }),
                    id: Some(format!("u{k}")),
                })
                .collect()
        };
        let p = wd::province(&b.prov);
        let setup = Setup {
            // the region's own battle map (the war map 2.0's), else the field it used to have
            map: wr::baked::map_of_region(p.id)
                .unwrap_or(p.battle)
                .to_owned(),
            title: format!("Batalla de {}", p.name),
            kicker: format!("CAMPAÑA · TURNO {}", s.turn),
            subtitle: if player_attacks {
                format!("{} ataca a {}.", mine.name, theirs.name)
            } else {
                format!("{} ataca a {}.", theirs.name, mine.name)
            },
            player: ArmySetup {
                faction: Faction::Reina,
                units: units(mine),
            },
            enemy: ArmySetup {
                faction: theirs.faction,
                units: units(theirs),
            },
            player_attacks,
            enemy_posture: Some(if player_attacks {
                wr::ai::Posture::Defend
            } else {
                wr::ai::Posture::Attack
            }),
            seed: s.seed.wrapping_add(s.turn * 31),
            dimming: true,
            deploy: true,
        };
        Self::begin_battle(
            w,
            setup,
            Some(CampaignFight {
                battle: b,
                player_attacks,
                scale,
            }),
            None,
        );
        self.persist_3d_state(true);
    }

    fn campaign_auto(&mut self, b: PendingBattle) {
        let Some(s) = self.war_campaign.as_mut() else {
            return;
        };
        let res = s.auto_resolve(&b);
        let queen_attacks = s
            .army(&b.attacker)
            .is_some_and(|a| a.faction == Faction::Reina);
        let won = (res.winner == 0) == queen_attacks;
        s.apply_battle(&b, &res, 0, 1.0);
        ui_sfx::play(if won {
            ui_sfx::UiSfx::Deal
        } else {
            ui_sfx::UiSfx::Fail
        });
        let name = wd::province(&b.prov).name;
        if let Some(c) = self.war.as_mut().and_then(|w| w.camp.as_mut()) {
            c.toast = Some((
                if won {
                    "Victoria (resolución automática)".into()
                } else {
                    "Derrota (resolución automática)".into()
                },
                name.to_owned(),
                0.0,
            ));
            Self::sync_tokens(c, s, true);
            c.paint_version += 1;
            c.wait = 0.7;
        }
    }

    /// Back on the map after a battle of the campaign.
    fn back_to_campaign(&mut self, res: BattleResult, fight: Option<CampaignFight>) {
        if let (Some(f), Some(s)) = (fight.as_ref(), self.war_campaign.as_mut()) {
            s.apply_battle(
                &f.battle,
                &res,
                if f.player_attacks { 0 } else { 1 },
                f.scale,
            );
        }
        self.start_campaign(false);
        if fight.is_some() {
            if res.winner == 0 {
                let n = self.story.get("guerra_victorias");
                self.story.values.insert("guerra_victorias".into(), n + 1);
                self.queue_war_scene("guerra.primera_victoria");
            } else {
                self.queue_war_scene("guerra.primera_derrota");
            }
        }
        if let (Some(f), Some(c)) = (fight, self.war.as_mut().and_then(|w| w.camp.as_mut())) {
            c.toast = Some((
                if res.winner == 0 {
                    format!("Victoria en {}", wd::province(&f.battle.prov).name)
                } else {
                    format!("Derrota en {}", wd::province(&f.battle.prov).name)
                },
                res.label.clone(),
                0.0,
            ));
            c.wait = 0.9;
            c.modal = None;
        }
        self.persist_3d_state(true);
    }

    fn campaign_end_turn(&mut self) {
        // one turn a day: the war's gold is hers, and turns are not to be farmed for it
        let done_today = self.story.get(super::war_story::TURN_TODAY) > 0;
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_mut()) else {
            return;
        };
        if s.phase != CampPhase::Player || c.busy || s.winner.is_some() || c.modal.is_some() {
            return;
        }
        if done_today {
            ui_sfx::play(ui_sfx::UiSfx::Error);
            c.toast = Some((
                "Un turno al día".into(),
                "El consejo de guerra se reúne mañana: duerme para el siguiente turno".into(),
                0.0,
            ));
            c.paint_version += 1;
            return;
        }
        c.sel = None;
        s.end_player_turn();
        self.story
            .values
            .insert(super::war_story::TURN_TODAY.to_owned(), 1);
        ui_sfx::play(ui_sfx::UiSfx::Page);
        c.toast = Some((
            "Fin del turno".into(),
            "Mueven los clanes del norte y la Alianza…".into(),
            0.0,
        ));
        c.wait = 0.5;
        c.paint_version += 1;
    }

    fn tick_campaign(&mut self, dt: f32) {
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let camera = w.cam.camera();
        let pointer = w.pointer;
        let over_hud = pointer.is_some_and(|(x, y)| w.view.over_hud(x, y));
        let (Some(c), Some(s)) = (w.camp.as_mut(), self.war_campaign.as_mut()) else {
            return;
        };
        if let Some(t) = c.toast.as_mut() {
            t.2 += dt;
            if t.2 > 2.8 {
                c.toast = None;
            }
        }
        // the tokens slide
        let mut sliding = false;
        for t in c.tokens.values_mut() {
            if let Some(from) = t.from {
                t.t += dt / 0.75;
                let k = t.t.min(1.0);
                let e = k * k * (3.0 - 2.0 * k);
                t.at = from
                    + (t.to - from) * e
                    + Vec3::new(0.0, (k * std::f32::consts::PI).sin() * 2.2, 0.0);
                if t.t >= 1.0 {
                    t.at = t.to;
                    t.from = None;
                } else {
                    sliding = true;
                }
            } else {
                t.at = t.at + (t.to - t.at) * (dt * 6.0).min(1.0);
            }
        }
        // the province under the pointer
        c.hover_t -= dt;
        if c.hover_t <= 0.0 && !c.busy {
            c.hover_t = 0.12;
            let h = pointer
                .filter(|_| !over_hud)
                .and_then(|(x, y)| Self::province_under(c, camera, x, y));
            if h != c.hover {
                c.hover = h;
                c.paint_version += 1;
            }
        }
        if c.wait > 0.0 {
            c.wait -= dt;
            return;
        }
        if sliding {
            return;
        }
        if let Some(b) = c.battle_after.take() {
            c.modal = Some(CampModal::Battle(b));
            return;
        }
        if s.winner.is_some() && !c.end_shown {
            c.end_shown = true;
            c.modal = Some(CampModal::End);
            return;
        }
        if c.modal.is_some() {
            return;
        }
        if s.phase == CampPhase::Player && !c.busy {
            if let Some(id) = c.marching.pop_front() {
                if let Some((x, z)) = Self::march_step(c, s, &id) {
                    let land = &c.land;
                    w.cam
                        .frame(x, z + 4.0, None, None, false, &|x: f32, z: f32| {
                            land.top_at(x, z)
                        });
                }
                return;
            }
        }
        if s.phase == CampPhase::Ai {
            if c.steps.is_empty() && !c.busy {
                // the others move
                c.busy = true;
                let fog_before = Self::fogged(s);
                let (steps, battle) = s.run_queue();
                for st in steps {
                    let (fi, ti) = (wd::province_index(&st.from), wd::province_index(&st.to));
                    if !fog_before[fi] || !fog_before[ti] {
                        c.steps.push_back(st);
                    }
                }
                c.battle_after = battle;
                if c.steps.is_empty() {
                    c.busy = false;
                    Self::sync_tokens(c, s, true);
                    if c.battle_after.is_none() {
                        Self::new_turn(c, s);
                    }
                }
                c.paint_version += 1;
                return;
            }
            if let Some(st) = c.steps.pop_front() {
                // show one hop (the state already holds where it ends)
                let to_slot = s.army(&st.army).map(|a| Self::slot_of(s, &c.land, a));
                if let Some(end) = to_slot {
                    let (fp, tp) = (wd::province(&st.from), wd::province(&st.to));
                    let (fx, fz) = fp.site();
                    let start = Vec3::new(fx, c.land.top_at(fx, fz + 10.0), fz + 10.0);
                    let end = if st.battle {
                        let (tx, tz) = (tp.x, tp.z + 10.0);
                        let m = Vec3::new(
                            start.x + (tx - start.x) * 0.55,
                            0.0,
                            start.z + (tz - start.z) * 0.55,
                        );
                        Vec3::new(m.x, c.land.top_at(m.x, m.z), m.z)
                    } else {
                        end
                    };
                    let t = c.tokens.entry(st.army.clone()).or_insert(Token {
                        at: start,
                        from: None,
                        to: end,
                        t: 1.0,
                        yaw: 0.0,
                    });
                    t.from = Some(start);
                    t.to = end;
                    t.t = 0.0;
                    t.yaw = (end.x - start.x).atan2(end.z - start.z);
                    // followed when the Queen's people can see where it goes (or it is hers)
                    let hers = s
                        .army(&st.army)
                        .is_some_and(|a| a.faction == Faction::Reina);
                    let seen = !Self::fogged(s)[wd::province_index(&st.to)];
                    if hers || seen {
                        let mid = ((start.x + end.x) / 2.0, (start.z + end.z) / 2.0 - 6.0);
                        let land = &c.land;
                        w.cam
                            .frame(mid.0, mid.1, None, None, false, &|x: f32, z: f32| {
                                land.top_at(x, z)
                            });
                    }
                }
                if c.steps.is_empty() {
                    c.busy = false;
                    c.wait = 0.8;
                    if c.battle_after.is_none() {
                        Self::sync_tokens(c, s, false);
                    }
                }
                return;
            }
            if c.battle_after.is_none() && !c.busy {
                Self::new_turn(c, s);
            }
        }
    }

    fn new_turn(c: &mut CampaignRun, s: &mut Campaign) {
        s.start_turn();
        Self::sync_tokens(c, s, true);
        Self::queue_marches(c, s);
        let inc = s.income(Faction::Reina);
        let signed = |n: i32| {
            if n >= 0 {
                format!("+{n}")
            } else {
                format!("−{}", -n)
            }
        };
        c.toast = Some((
            format!("Turno {}", s.turn),
            format!(
                "{} · {} oro · {} comida",
                SEASONS[((s.turn - 1) % 4) as usize],
                signed(inc.net_gold),
                signed(inc.net_food)
            ),
            0.0,
        ));
        c.paint_version += 1;
        ui_sfx::play(ui_sfx::UiSfx::Page);
    }

    // ------------------------------------------------------------------ the view
    pub(super) fn build_war_view(&mut self) {
        let campaign = self.war_campaign.clone();
        let story = self.war_story_view();
        let relics_found: Vec<bool> = super::war_story::relics()
            .iter()
            .map(|r| self.story.get(&format!("reliquia_{}", r.id)) > 0)
            .collect();
        let powers = self.war_powers();
        let Some(w) = self.war.as_mut() else {
            return;
        };
        let mut view = WarView {
            time: w.time,
            pointer: w.pointer.map(|(x, y)| [x, y]),
            minimap_ready: w.minimap_ready,
            ..Default::default()
        };
        match w.page {
            WarPage::Table(page) => {
                table_view(
                    &mut view,
                    page,
                    w.scale,
                    campaign.as_ref(),
                    w.focus,
                    (w.map_pick, w.map_attack),
                );
                if page == 2 {
                    relics_view(&mut view, w.focus, &relics_found, &powers);
                }
            }
            WarPage::Battle => {
                if let Some(run) = w.battle.as_ref() {
                    battle_view(&mut view, run, &w.cam, w.press.as_ref(), w.pointer);
                }
            }
            WarPage::Campaign => {
                if let (Some(c), Some(s)) = (w.camp.as_ref(), campaign.as_ref()) {
                    campaign_view(&mut view, c, s, &w.cam, w.pointer, &story);
                }
            }
        }
        w.view = view;
    }
}

fn btn(rect: [f32; 4], label: &str, key: &str, detail: &str, hit: WarHit) -> WarButton {
    WarButton {
        rect,
        label: label.to_owned(),
        key: key.to_owned(),
        detail: detail.to_owned(),
        enabled: true,
        hit,
        ..Default::default()
    }
}

/// Where an army's regiments stand on its base on the campaign's map (x across, z forward, the
/// lead in front, in rows behind it), and the base's radius.
pub(super) fn formation(n: usize) -> (Vec<[f32; 2]>, f32) {
    let n = n.clamp(1, wd::MAX_UNITS);
    let mut at = vec![[0.0, 0.0]];
    let rest = n - 1;
    if rest > 0 {
        at[0][1] = 2.0;
        let cols = if rest <= 3 {
            rest
        } else if rest <= 6 {
            3
        } else {
            4
        };
        for i in 0..rest {
            let (r, c) = (i / cols, i % cols);
            let in_row = (rest - r * cols).min(cols);
            at.push([
                (c as f32 - (in_row as f32 - 1.0) / 2.0) * 2.3,
                -0.4 - r as f32 * 2.3,
            ]);
        }
        // centred on the base
        let (lo, hi) = at.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p[1]), hi.max(p[1]))
        });
        let mid = (lo + hi) / 2.0;
        for p in &mut at {
            p[1] -= mid;
        }
    }
    let radius = at.iter().map(|p| p[0].hypot(p[1])).fold(0.0, f32::max) + 1.8;
    (at, radius.max(2.75))
}

/// The sizes of the units the table offers (men per regiment, ×).
const SIZES: [f32; 4] = [0.5, 1.0, 1.5, 2.0];

/// «×0,5», «×1»… (the fonts have no ½).
fn size_label(size: f32) -> String {
    if size.fract().abs() < 0.01 {
        format!("×{}", size as u32)
    } else {
        format!("×{}", size).replace('.', ",")
    }
}

/// The war table's page: numbered rows (style 4, in order), the units' size as chips (style 5), and
/// the card of the row the focus is on.
fn table_view(
    view: &mut WarView,
    page: u8,
    scale: f32,
    campaign: Option<&Campaign>,
    focus: usize,
    (map_pick, map_attack): (usize, bool),
) {
    let mut t = TableView {
        title: "La mesa de guerra".into(),
        tag: "LA GUERRA DE LA CHISPA".into(),
        keys: vec![
            ("^v".into(), "Elegir".into()),
            ("Intro".into(), "Aceptar".into()),
            ("Esc".into(), "Volver".into()),
        ],
        ..Default::default()
    };
    let (x, w) = (120.0, 780.0);
    let row = |rect: [f32; 4], label: &str, key: &str, detail: &str, hit: WarHit, icon: Glyph| {
        let mut b = btn(rect, label, key, detail, hit);
        b.style = 4;
        b.icon = icon;
        b
    };
    let lines = |ls: &[&str]| ls.iter().map(|l| (*l).to_owned()).collect::<Vec<String>>();
    let mut cards: Vec<TableCard> = Vec::new();
    let mut y = 300.0;
    if page == 0 {
        let world = lines(&[
            "Sobre la mesa, el oeste del Continente Oscuro: el castillo en el Bosque Oscuro, la cueva del sello, la madriguera de los trasgos, los clanes del norte y, al otro lado del río, la Alianza con su Puerta del Este.",
            "La chispa de la Reina da mente a sus pueblos. Mueve sus ejércitos provincia a provincia, recluta, enciende lo que conquistes y, cuando dos ejércitos se encuentren, libra la batalla con cientos de soldados en el campo.",
            "Toma la capital caída para ganar. Si cae el castillo, todo se pierde.",
        ]);
        if let Some(s) = campaign {
            let f = s.purse(Faction::Reina);
            view.buttons.push(row(
                [x, y, w, 92.0],
                "Continuar la campaña",
                "",
                &format!("Turno {} · {} de oro", s.turn, f.gold),
                WarHit::Table(0),
                Glyph::Map,
            ));
            cards.push(TableCard {
                kicker: "LA CAMPAÑA".into(),
                title: "Continuar la campaña".into(),
                lines: world.clone(),
                stats: vec![
                    (Glyph::Hourglass, format!("Turno {}", s.turn)),
                    (Glyph::Gold, f.gold.to_string()),
                    (Glyph::Food, f.food.to_string()),
                    (Glyph::Flame, f.spark.to_string()),
                ],
                ..Default::default()
            });
            y += 106.0;
            view.buttons.push(row(
                [x, y, w, 92.0],
                "Empezar otra campaña",
                "",
                "La campaña guardada se pierde",
                WarHit::Table(1),
                Glyph::Crown,
            ));
            cards.push(TableCard {
                kicker: "OTRA CAMPAÑA".into(),
                title: "Empezar otra campaña".into(),
                lines: lines(&["La Reina vuelve a despertar en la primavera del año 1, con el castillo, la cueva del sello y poco más."]),
                warning: "La campaña guardada se pierde.".into(),
                ..Default::default()
            });
        } else {
            view.buttons.push(row(
                [x, y, w, 92.0],
                "La guerra de la chispa",
                "",
                "Una campaña por turnos sobre el continente",
                WarHit::Table(1),
                Glyph::Map,
            ));
            cards.push(TableCard {
                kicker: "LA CAMPAÑA".into(),
                title: "La guerra de la chispa".into(),
                lines: world,
                ..Default::default()
            });
        }
        y += 106.0;
        view.buttons.push(row(
            [x, y, w, 92.0],
            "Batallas rápidas",
            "",
            "Cinco batallas sueltas, sin campaña",
            WarHit::Table(2),
            Glyph::Swords,
        ));
        cards.push(TableCard {
            kicker: "BATALLAS RÁPIDAS".into(),
            title: "Batallas rápidas".into(),
            lines: lines(&[
                "Una batalla suelta en uno de los campos del continente, con los ejércitos ya formados.",
                "Antes de empezar se despliega: elige un regimiento y colócalo con el clic derecho en la zona marcada. Arrastra el clic derecho para darle anchura y frente.",
                "F1 en la batalla muestra todos los controles.",
            ]),
            ..Default::default()
        });
        y += 106.0;
        view.buttons.push(row(
            [x, y, w, 92.0],
            "Reliquias y poderes",
            "",
            "Lo encontrado en la guerra y lo que la Reina puede lanzar en batalla",
            WarHit::Table(6),
            Glyph::Crown,
        ));
        cards.push(TableCard {
            kicker: "LA COLECCIÓN".into(),
            title: "Reliquias y poderes".into(),
            lines: lines(&[
                "Las reliquias se encuentran tomando regiones o en los campos de batalla: llévale una tropa a la luz dorada.",
                "Algunas dan oro, chispa o comida; otras, poderes de guerra. Otras tiran de un hilo de la historia.",
            ]),
            ..Default::default()
        });
        y += 106.0;
        view.buttons.push(row(
            [x, y, w, 92.0],
            "Tamaño de las unidades",
            "",
            "Soldados por regimiento",
            WarHit::Table(3),
            Glyph::Army,
        ));
        for (k, &size) in SIZES.iter().enumerate() {
            let mut chip = btn(
                [
                    x + w - 36.0 - (SIZES.len() - k) as f32 * 76.0,
                    y + 26.0,
                    68.0,
                    40.0,
                ],
                &size_label(size),
                "",
                "",
                WarHit::Table(20 + k as u8),
            );
            chip.style = 5;
            chip.on = (size - scale).abs() < 0.01;
            view.buttons.push(chip);
        }
        cards.push(TableCard {
            kicker: "EL CAMPO".into(),
            title: "Tamaño de las unidades".into(),
            lines: lines(&[
                "Cuántos soldados forman cada regimiento, en las batallas rápidas y en las de la campaña.",
                "×2: unos 2.300 en el campo; pide más equipo.",
            ]),
            stats: vec![(Glyph::Army, size_label(scale))],
            ..Default::default()
        });
        y += 106.0;
        view.buttons.push(row(
            [x, y + 6.0, 440.0, 66.0],
            "Volver al castillo",
            "Esc",
            "",
            WarHit::Leave,
            Glyph::Back,
        ));
        cards.push(TableCard {
            kicker: "EL CASTILLO".into(),
            title: "Volver al castillo".into(),
            lines: lines(&["La Reina deja el mapa y vuelve a la sala de guerra."]),
            ..Default::default()
        });
    } else {
        t.title = "Batallas rápidas".into();
        // a battle on any of the continent's battle maps: which, and whether she attacks
        let (id, region) = wr::baked::BATTLE_MAPS[map_pick.min(wr::baked::BATTLE_MAPS.len() - 1)];
        let place = wd::PROVINCES
            .iter()
            .find(|p| p.id == region)
            .map_or(id, |p| p.name);
        let siege = id.starts_with("asedio_");
        view.buttons.push(row(
            [x, y, w, 88.0],
            "Batalla en un mapa del continente",
            "",
            &format!(
                "{} · {}",
                place,
                if map_attack {
                    "Atacas tú"
                } else {
                    "Te atacan"
                }
            ),
            WarHit::Table(30),
            Glyph::Map,
        ));
        for (k, (label, hit, on)) in [
            ("‹", WarHit::Table(31), false),
            ("›", WarHit::Table(32), false),
            (
                if map_attack { "Ataque" } else { "Defensa" },
                WarHit::Table(33),
                true,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let wdt = if k == 2 { 110.0 } else { 46.0 };
            let cx = x + w - 36.0 - [214.0, 160.0, 110.0][k];
            let mut chip = btn([cx, y + 24.0, wdt, 40.0], label, "", "", hit);
            chip.style = 5;
            chip.on = on;
            view.buttons.push(chip);
        }
        cards.push(TableCard {
            kicker: format!("LOS {} CAMPOS · {}", wr::baked::BATTLE_MAPS.len(), place.to_uppercase()),
            title: "Batalla en un mapa del continente".into(),
            lines: vec![
                format!("{}: el campo de batalla de esta región, tal como es en el mapa de guerra.", place),
                if siege {
                    if map_attack {
                        "Asedio: llevas madera para catapultas, trabuquetes, torres de asedio y arietes. Abre paso y toma el patio del torreón.".into()
                    } else {
                        "Asedio: defiendes los muros. Construye catapultas y balistas, tapa las brechas con empalizadas y aguanta hasta el final.".into()
                    }
                } else {
                    "Campo abierto: con tu madera puedes levantar estacas contra la caballería, empalizadas y escorpiones.".into()
                },
                "‹ › (o las flechas) cambian de mapa; el botón de la derecha, de bando.".into(),
            ],
            warning: if map_attack {
                "Atacas tú.".into()
            } else {
                "Te atacan: aguanta.".into()
            },
            ..Default::default()
        });
        y += 100.0;
        for (i, sc) in SCENARIOS.iter().enumerate() {
            view.buttons.push(row(
                [x, y, w, 88.0],
                sc.title,
                &format!("{}", i + 1),
                sc.blurb,
                WarHit::Quick(i as u8),
                Glyph::Swords,
            ));
            let side = |name: &str, (faction, ids): (Faction, &[&str])| {
                let mut units: Vec<(Glyph, String, u32)> = Vec::new();
                for id in ids {
                    let ty = wd::unit(id);
                    match units.iter_mut().find(|u| u.1 == ty.name) {
                        Some(u) => u.2 += 1,
                        None => units.push((role_glyph(ty), ty.name.to_owned(), 1)),
                    }
                }
                (name.to_owned(), faction.info().color, units)
            };
            cards.push(TableCard {
                kicker: sc.kicker.into(),
                title: sc.title.into(),
                lines: vec![sc.subtitle.into()],
                sides: vec![side("TUS TROPAS", sc.player), side("EL ENEMIGO", sc.enemy)],
                warning: if sc.player_attacks {
                    "Atacas tú.".into()
                } else {
                    "Te atacan: aguanta.".into()
                },
                ..Default::default()
            });
            y += 100.0;
        }
        view.buttons.push(row(
            [x, y + 6.0, 360.0, 66.0],
            "Volver",
            "Esc",
            "",
            WarHit::Table(5),
            Glyph::Back,
        ));
        cards.push(TableCard {
            kicker: "LA MESA".into(),
            title: "Volver".into(),
            lines: lines(&["A la mesa de guerra."]),
            ..Default::default()
        });
    }
    t.focus = focus.min(cards.len().saturating_sub(1));
    t.card = cards.swap_remove(t.focus);
    view.table = Some(t);
}

/// The war table's page of relics and powers: the relics in two columns (unknown ones as «¿?»), a row
/// for her powers, back; the card of the one the keys are on.
fn relics_view(view: &mut WarView, focus: usize, found: &[bool], powers: &[wr::powers::Power]) {
    let Some(t) = view.table.as_mut() else {
        return;
    };
    view.buttons.clear();
    t.title = "Reliquias y poderes".into();
    let all = super::war_story::relics();
    t.tag = format!(
        "{} / {} ENCONTRADAS · {} / {} PODERES",
        found.iter().filter(|f| **f).count(),
        all.len(),
        powers.len(),
        wr::powers::Power::ALL.len()
    );
    let (x, y0, w, h) = (120.0, 290.0, 382.0, 50.0);
    let mut cards: Vec<TableCard> = Vec::new();
    let place = |id: &str| {
        wd::PROVINCES
            .iter()
            .find(|p| p.id == id)
            .map_or(id.to_owned(), |p| p.name.to_owned())
    };
    for (k, r) in all.iter().enumerate() {
        let have = found.get(k).copied().unwrap_or(false);
        let (cx, cy) = (
            x + (k % 2) as f32 * (w + 16.0),
            y0 + (k / 2) as f32 * (h + 8.0),
        );
        let hint = if r.on_field() {
            "En su campo de batalla"
        } else {
            "Al tomar la región"
        };
        let (title, detail) = if have {
            (r.nombre.clone(), place(&r.region))
        } else {
            (place(&r.region), format!("Sin encontrar · {hint}"))
        };
        let mut b = btn(
            [cx, cy, w, h],
            &title,
            "",
            &detail,
            WarHit::Table(60 + k as u8),
        );
        b.style = 4;
        b.icon = if have { Glyph::Crown } else { Glyph::Map };
        view.buttons.push(b);
        // what it gives, as the card's stats (hidden until it is found)
        let rw = &r.recompensa;
        let mut stats = Vec::new();
        if have {
            if let Some(p) = rw.poder.as_deref().and_then(super::war_story::power_of) {
                stats.push((Glyph::Spark, format!("Poder: {}", p.spec().name)));
            }
            if rw.oro > 0 {
                stats.push((Glyph::Gold, format!("+{} oro", rw.oro)));
            }
            if rw.chispa > 0 {
                stats.push((Glyph::Flame, format!("+{} chispa", rw.chispa)));
            }
            if rw.comida > 0 {
                stats.push((Glyph::Food, format!("+{} comida", rw.comida)));
            }
        }
        cards.push(TableCard {
            kicker: if have { "RELIQUIA ENCONTRADA".into() } else { "SIN ENCONTRAR".into() },
            title: if have { r.nombre.clone() } else { "Una reliquia perdida".into() },
            lines: if have {
                let mut l = vec![r.texto.clone(), format!("Se encontró en {}.", place(&r.region))];
                if rw.var.is_some() {
                    l.push("Tira de un hilo de la historia.".into());
                }
                l
            } else {
                vec![
                    format!("Dónde: {}", place(&r.region)),
                    if r.on_field() {
                        "Brilla en el campo de batalla de esa región: lleva una de tus tropas a la luz dorada.".into()
                    } else {
                        "Se encuentra al tomar esa región.".into()
                    },
                ]
            },
            stats,
            ..Default::default()
        });
    }
    let rows = all.len().div_ceil(2) as f32;
    let y = y0 + rows * (h + 8.0) + 10.0;
    let names: Vec<&str> = powers.iter().map(|p| p.spec().name).collect();
    let mut b = btn(
        [x, y, w * 2.0 + 16.0, 56.0],
        "Poderes de la Reina",
        "",
        &if names.is_empty() {
            "Ninguno todavía".to_owned()
        } else {
            names.join(" · ")
        },
        WarHit::Table(99),
    );
    b.style = 4;
    b.icon = Glyph::Spark;
    view.buttons.push(b);
    cards.push(TableCard {
        kicker: "EN BATALLA · TECLAS Z X C V B N".into(),
        title: "Poderes de la Reina".into(),
        lines: wr::powers::Power::ALL
            .iter()
            .map(|p| {
                let sp = p.spec();
                if powers.contains(p) {
                    format!("{} ({}): {}", sp.name, sp.key, sp.what)
                } else {
                    format!("{}: bloqueado · {}", sp.name, sp.from)
                }
            })
            .collect(),
        ..Default::default()
    });
    let mut back = btn(
        [x, y + 66.0, 360.0, 60.0],
        "Volver",
        "Esc",
        "",
        WarHit::Table(5),
    );
    back.style = 4;
    back.icon = Glyph::Back;
    view.buttons.push(back);
    cards.push(TableCard {
        kicker: "LA MESA".into(),
        title: "Volver".into(),
        lines: vec!["A la mesa de guerra.".into()],
        ..Default::default()
    });
    t.focus = focus.min(cards.len().saturating_sub(1));
    t.card = cards.swap_remove(t.focus);
}

/// One of the Queen's field armies with moves left this turn and no march to walk them.
fn can_move(a: &wr::rules::Army) -> bool {
    a.faction == Faction::Reina
        && !a.garrison
        && !a.units.is_empty()
        && a.mp > 0
        && a.march.is_none()
}

/// How wide a word draws in the banners' face (about): Latin letters, or the wide ones of Chinese,
/// Japanese and Korean.
fn label_width(s: &str, size: f32) -> f32 {
    s.chars()
        .map(|c| {
            if c as u32 >= 0x2e80 {
                size * 1.0
            } else {
                size * 0.54
            }
        })
        .sum()
}

/// The banners over the regiments: each anchored to where its soldiers show on the screen (the top
/// of the block, a pole down to its middle), laid out off the HUD (`avoid`) and off each other by
/// `world_tags`, gliding from frame to frame.
fn run_banners(run: &BattleRun, camera: Camera3D, avoid: &[[f32; 4]], now: f32) -> Vec<BannerTag> {
    use super::world_tags::TagAnchor;
    let sim = &run.b.sim;
    let mut anchors = Vec::new();
    for r in &sim.regiments {
        if r.alive == 0 || r.state == RState::Fled {
            continue;
        }
        // the heads of (some of) its soldiers, as seen
        let lift = if r.ty.flyer {
            1.4
        } else if r.ty.large {
            if r.ty.role == Role::Hero {
                4.2
            } else {
                3.4
            }
        } else {
            2.1
        };
        let step = (r.soldiers.len() / 40).max(1);
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for &i in r.soldiers.iter().step_by(step) {
            if sim.state[i] != 0 {
                continue;
            }
            if let Some(p) = project(camera, Vec3::new(sim.px[i], sim.py[i] + lift, sim.pz[i])) {
                xs.push(p[0]);
                ys.push(p[1]);
            }
        }
        if xs.is_empty() {
            continue;
        }
        ys.sort_by(f32::total_cmp);
        let top = ys[(ys.len() / 10).min(ys.len() - 1)];
        let mx = xs.iter().sum::<f32>() / xs.len() as f32;
        if !(-80.0..=SCREEN_W + 80.0).contains(&mx) || !(-40.0..=SCREEN_H + 60.0).contains(&top) {
            continue;
        }
        let ground = sim.field.walk_height(r.cx, r.cz);
        let foot = project(camera, Vec3::new(r.cx, ground + lift * 0.55, r.cz))
            .filter(|f| f[1] > top)
            .unwrap_or([mx, top + 18.0]);
        let depth = (camera.position - Vec3::new(r.cx, ground, r.cz)).length();
        let hero = r.ty.role == Role::Hero;
        let men = banner_men(sim, r);
        let name = tr(r.ty.short);
        let w = 46.0 + label_width(&name, 15.5) + 12.0 + label_width(&men, 14.0) + 16.0;
        let priority = if r.selected {
            1000.0
        } else if run.hover == Some(r.index) {
            900.0
        } else {
            (if hero { 300.0 } else { 100.0 }) + if r.side == 0 { 20.0 } else { 0.0 }
        } - depth * 0.2;
        anchors.push(TagAnchor {
            id: r.index,
            head: [mx, top],
            foot,
            size: [w, 36.0],
            compact: [46.0, 32.0],
            priority,
            far: depth > 240.0 && !r.selected && run.hover != Some(r.index),
        });
    }
    let mut layout = run.tags.borrow_mut();
    let placed = layout
        .update(&anchors, avoid, [0.0, 0.0, SCREEN_W, SCREEN_H], now)
        .to_vec();
    placed
        .into_iter()
        .map(|p| {
            let r = &sim.regiments[p.id];
            let routing = r.state.routed();
            let edge = if r.side == 1 {
                if r.ty.faction == Faction::Rebeldes {
                    0xc0563a
                } else {
                    0x3d6fd0
                }
            } else if r.ty.faction == Faction::Rebeldes {
                0xc0563a
            } else if r.ty.role == Role::Hero {
                0xf2b54a
            } else {
                0x9a5cff
            };
            let state = if routing {
                Glyph::Flag
            } else if r.dimmed {
                Glyph::Eye
            } else if r.state == RState::Melee {
                Glyph::Swords
            } else if r.state == RState::Firing {
                Glyph::Arrow
            } else {
                Glyph::None
            };
            BannerTag {
                rect: p.rect,
                pole: p.pole,
                alpha: p.alpha,
                compact: p.compact,
                reg: r.index,
                short: r.ty.short.to_owned(),
                men: banner_men(sim, r),
                glyph: role_glyph(r.ty),
                hp: if r.ty.role == Role::Hero {
                    r.soldiers.first().map_or(0.0, |&i| sim.hp[i].max(0.0)) / r.ty.hp
                } else {
                    r.alive as f32 / r.men0.max(1) as f32
                },
                edge,
                enemy: r.side == 1,
                selected: r.selected,
                hover: run.hover == Some(r.index),
                routing,
                state,
            }
        })
        .collect()
}

/// What a banner counts: the men standing, or the hero's life.
fn banner_men(sim: &wr::sim::Sim, r: &wr::sim::Regiment) -> String {
    if r.ty.role == Role::Hero {
        let hp = r.soldiers.first().map_or(0.0, |&i| sim.hp[i].max(0.0));
        format!("{}%", (100.0 * hp / r.ty.hp).round().max(0.0))
    } else {
        r.alive.to_string()
    }
}

fn unit_tip(t: &wd::UnitType, at: [f32; 2]) -> Tip {
    let mut rows = vec![
        ("Ataque".to_owned(), format!("{}", t.atk)),
        ("Defensa".to_owned(), format!("{}", t.def)),
        ("Daño".to_owned(), format!("{}", t.dmg)),
        ("Armadura".to_owned(), format!("{}", t.armor)),
        ("Carga".to_owned(), format!("{}", t.charge)),
        ("Vida".to_owned(), format!("{}", t.hp)),
        ("Velocidad".to_owned(), format!("{}", t.run)),
        (
            "Moral".to_owned(),
            if t.morale >= 999.0 {
                "∞".to_owned()
            } else {
                format!("{}", t.morale)
            },
        ),
    ];
    if let Some(r) = t.ranged {
        rows.push(("Alcance".to_owned(), format!("{}", r.range)));
        rows.push(("Proyectil".to_owned(), format!("{}", r.dmg)));
    }
    Tip {
        at,
        edge: t.faction.info().color,
        kicker: format!("{} · {}", tr(t.role.name()), tr(t.faction.info().short)),
        title: t.name.to_owned(),
        rows,
        text: t.blurb.to_owned(),
        action: String::new(),
    }
}

fn battle_view(
    view: &mut WarView,
    run: &BattleRun,
    cam: &RtsCam,
    press: Option<&Press>,
    pointer: Option<(f32, f32)>,
) {
    let b = &run.b;
    let sim = &b.sim;
    let camera = cam.camera();
    let pf = b.setup.player.faction.info();
    let ef = b.setup.enemy.faction.info();
    let [a, e] = sim.strength();
    let total = (a + e).max(1.0);
    let t = sim.time as u32;
    let mut bv = BattleView {
        kicker: b.setup.kicker.clone(),
        title: b.title.clone(),
        subtitle: b.setup.subtitle.clone(),
        names: [pf.short.to_uppercase(), ef.short.to_uppercase()],
        colors: [pf.bright, ef.bright],
        darks: [pf.dark, ef.dark],
        balance: a / total,
        clock: format!("{}:{:02}", t / 60, t % 60),
        messages: run.messages.iter().cloned().collect(),
        deploying: b.deploying,
        paused: b.paused,
        help: run.help,
        confirm: run.confirm,
        ..Default::default()
    };
    // the minimap
    let p = sim.field.play;
    let (mx0, mx1, mz0, mz1) = (p.x0 - 14.0, p.x1 + 14.0, p.z0 - 14.0, p.z1 + 14.0);
    let mw = 360.0;
    let mh = (mw * (mz1 - mz0) / (mx1 - mx0)).round();
    let norm = |x: f32, z: f32| [(x - mx0) / (mx1 - mx0), (z - mz0) / (mz1 - mz0)];
    let mut mm = Minimap {
        rect: [40.0, SCREEN_H - 28.0 - mh, mw, mh],
        label: b.map_title.to_uppercase(),
        zone: Some({
            let z = b.zone;
            let (a0, a1) = (norm(z.x0, z.z0), norm(z.x1, z.z1));
            [a0[0], a0[1], a1[0], a1[1]]
        }),
        ..Default::default()
    };
    for r in &sim.regiments {
        if r.alive == 0 || r.state == RState::Fled {
            continue;
        }
        let f = r.ty.faction.info();
        mm.dots.push(MiniDot {
            at: norm(r.cx, r.cz),
            width: sim.width(r.index, r.files) / (mx1 - mx0),
            heading: r.heading,
            color: if r.state.routed() {
                0xff8a7a
            } else if r.side == 0 {
                f.bright
            } else {
                f.color
            },
            hero: r.ty.role == Role::Hero,
            selected: r.selected,
        });
    }
    // what the camera sees, on the ground plane under the focus
    for (sx, sy) in [
        (0.0, 0.0),
        (SCREEN_W, 0.0),
        (SCREEN_W, SCREEN_H),
        (0.0, SCREEN_H),
    ] {
        let (o, d) = ray(camera, sx, sy);
        let fy = cam.focus.y;
        let pt = if d.y < -1e-3 {
            let t = (fy - o.y) / d.y;
            if t > 0.0 && t < 1400.0 {
                o + d * t
            } else {
                let f = Vec3::new(d.x, 0.0, d.z).normalize_or_zero() * 900.0;
                o + f
            }
        } else {
            let f = Vec3::new(d.x, 0.0, d.z).normalize_or_zero() * 900.0;
            o + f
        };
        mm.view.push(norm(pt.x, pt.z));
    }
    if let Some(rel) = run.relic.as_ref().filter(|r| !r.taken) {
        mm.dots.push(MiniDot {
            at: norm(rel.x, rel.z),
            width: 0.012,
            heading: 0.0,
            color: 0xffd27a,
            hero: true,
            selected: false,
        });
    }
    bv.minimap = Some(mm);
    // the cards
    let mine: Vec<&wr::sim::Regiment> = sim.regiments.iter().filter(|r| r.side == 0).collect();
    let (cw, ch, gap) = (104.0, 156.0, 8.0);
    let room_x0 = 440.0;
    let room_x1 = SCREEN_W - 360.0;
    let n = mine.len() as f32;
    let total_w = n * cw + (n - 1.0).max(0.0) * gap;
    let k = ((room_x1 - room_x0) / total_w.max(1.0)).min(1.0);
    let (cw, gap) = (cw * k, gap * k);
    let mut x = (room_x0 + room_x1) / 2.0 - (n * cw + (n - 1.0).max(0.0) * gap) / 2.0;
    let y = SCREEN_H - 24.0 - ch;
    for r in &mine {
        let hero = r.ty.role == Role::Hero;
        let lead_hp = r.soldiers.first().map_or(0.0, |&i| sim.hp[i].max(0.0));
        let out = r.alive == 0 || r.state == RState::Fled;
        let state = r.state;
        let (glyph, tone) = if state.routed() {
            (Glyph::Flag, 1)
        } else if r.dimmed {
            (Glyph::Eye, 2)
        } else if r.ty.spark && r.cohesion < 30.0 {
            (Glyph::Flame, 2)
        } else if state == RState::Melee {
            (Glyph::Swords, 0)
        } else if state == RState::Firing {
            (Glyph::Arrow, 0)
        } else if state == RState::Moving {
            (Glyph::Run, 0)
        } else if r.wavering {
            (Glyph::Help, 1)
        } else {
            (Glyph::None, 0)
        };
        let rect = [x, y, cw, ch];
        let hover =
            pointer.is_some_and(|(px, py)| px >= x && px <= x + cw && py >= y && py <= y + ch);
        bv.cards.push(UnitCard {
            rect,
            reg: r.index,
            edge: if hero {
                0xf2b54a
            } else {
                r.ty.faction.info().color
            },
            glyph: role_glyph(r.ty),
            short: r.ty.short.to_owned(),
            men: if hero {
                format!("{}%", (100.0 * lead_hp / r.ty.hp).round().max(0.0))
            } else {
                r.alive.to_string()
            },
            hp: if hero {
                lead_hp / r.ty.hp
            } else {
                r.alive as f32 / r.men0.max(1) as f32
            },
            morale: r.morale / 100.0,
            spark: r.ty.spark.then_some(r.cohesion / 100.0),
            ammo: r.ty.ranged.map(|_| {
                if r.ammo0 > 0 {
                    r.ammo as f32 / r.ammo0 as f32
                } else {
                    0.0
                }
            }),
            state: glyph,
            tone,
            selected: r.selected,
            hover,
            dead: out,
            routing: state.routed(),
            dimmed: r.dimmed,
            cd: r.ability.map(|ab| ab.cd / ab.cd_max.max(0.01)),
        });
        if hover {
            view.tip = Some(unit_tip(r.ty, [x - 100.0, y - 330.0]));
        }
        x += cw + gap;
    }
    // the tip over a banner or a soldier
    if view.tip.is_none() {
        if let (Some(reg), Some((px, py))) = (run.hover, pointer) {
            let r = &sim.regiments[reg];
            let mut tip = unit_tip(r.ty, [px + 22.0, py + 22.0]);
            if r.side == 1 && !b.selected().is_empty() {
                tip.action = "Clic derecho: atacar".into();
            }
            view.tip = Some(tip);
        }
    }
    // the box being dragged
    if let (Some(pr), Some((px, py))) = (press, pointer) {
        if pr.button == 0 && pr.moved && pr.hud == WarHit::None {
            let (x0, y0) = (pr.at.0.min(px), pr.at.1.min(py));
            bv.selbox = Some([x0, y0, (pr.at.0 - px).abs(), (pr.at.1 - py).abs()]);
        }
    }
    // the speed and the flag, top right
    let mut bx = SCREEN_W - 24.0;
    let mut top = |label: &str, icon: Glyph, w: f32, hit: WarHit, on: bool, style: u8| {
        bx -= w;
        let mut bt = btn([bx, 34.0, w, 48.0], label, "", "", hit);
        bt.icon = icon;
        bt.on = on;
        bt.style = style;
        view.buttons.push(bt);
        bx -= 10.0;
    };
    top("", Glyph::Flag, 58.0, WarHit::Retreat, false, 3);
    top("", Glyph::Help, 58.0, WarHit::Help, run.help, 0);
    for s in (1..=3u8).rev() {
        top(
            &format!("×{s}"),
            Glyph::None,
            64.0,
            WarHit::Speed(s),
            !b.paused && b.speed as u8 == s,
            0,
        );
    }
    top(
        "",
        if b.paused { Glyph::Play } else { Glyph::Pause },
        58.0,
        WarHit::Speed(0),
        b.paused && !b.deploying,
        0,
    );
    if let Some(last) = view.buttons.last_mut() {
        last.enabled = !b.deploying;
    }
    if let Some(retreat) = view.buttons.iter_mut().find(|bt| bt.hit == WarHit::Retreat) {
        retreat.enabled = !b.deploying && run.result.is_none();
    }
    // the orders, bottom right, and the spark over them
    let sel = b.selected();
    let ranged: Vec<usize> = sel
        .iter()
        .copied()
        .filter(|&i| sim.regiments[i].ty.ranged.is_some())
        .collect();
    let skirm: Vec<usize> = sel
        .iter()
        .copied()
        .filter(|&i| sim.regiments[i].ty.role == Role::Skirmisher)
        .collect();
    let (ox, oy) = (SCREEN_W - 24.0 - 316.0, SCREEN_H - 24.0 - 104.0);
    let orders = [
        ("Alto", "H", Glyph::Halt, 0u8, !sel.is_empty(), false),
        ("Correr", "R", Glyph::Run, 1, true, run.run_mode),
        (
            "Libre",
            "G",
            Glyph::Bow,
            2,
            !ranged.is_empty(),
            !ranged.is_empty() && ranged.iter().all(|&i| sim.regiments[i].fire_at_will),
        ),
        (
            "Hostigar",
            "K",
            Glyph::Skirmish,
            3,
            !skirm.is_empty(),
            !skirm.is_empty() && skirm.iter().all(|&i| sim.regiments[i].skirmish),
        ),
    ];
    for (k, (label, key, icon, o, enabled, on)) in orders.into_iter().enumerate() {
        let (cx, cy) = (ox + (k % 2) as f32 * 162.0, oy + (k / 2) as f32 * 56.0);
        let mut bt = btn([cx, cy, 154.0, 48.0], label, key, "", WarHit::Order(o));
        bt.icon = icon;
        bt.enabled = enabled && !b.deploying;
        bt.on = on;
        view.buttons.push(bt);
    }
    if let Some(q) = b.queen {
        let r = &sim.regiments[q];
        if let Some(ab) = r.ability {
            let dead = r.alive == 0;
            let ready = !dead && ab.cd <= 0.0 && !b.deploying;
            let cd = if dead {
                1.0
            } else {
                ab.cd / ab.cd_max.max(0.01)
            };
            bv.spark = Some((
                [SCREEN_W - 24.0 - 70.0, oy - 84.0],
                62.0,
                ready,
                cd.clamp(0.0, 1.0),
                dead,
            ));
        }
    }
    // her war powers: a row over the cards, left of the spark
    if let (Some(pw), None) = (sim.powers.as_ref(), &run.result) {
        let list: Vec<wr::powers::Power> = wr::powers::Power::ALL
            .iter()
            .copied()
            .filter(|p| pw.has(*p))
            .collect();
        let (bw, bh, gap) = (176.0, 50.0, 8.0);
        let x0 = SCREEN_W - 24.0 - 160.0 - list.len() as f32 * (bw + gap);
        for (k, p) in list.iter().enumerate() {
            let sp = p.spec();
            let cd = pw.cooldown[p.index()];
            let short = match p {
                wr::powers::Power::Grito => "Grito",
                wr::powers::Power::Rugido => "Rugido",
                wr::powers::Power::Bendicion => "Bendición",
                wr::powers::Power::Brasas => "Brasas",
                wr::powers::Power::Niebla => "Niebla",
                wr::powers::Power::Raices => "Raíces",
            };
            let detail = if cd > 0.0 {
                format!("{} s", cd.ceil() as u32)
            } else {
                "lista".into()
            };
            let mut bt = btn(
                [
                    x0 + k as f32 * (bw + gap),
                    SCREEN_H - 24.0 - 104.0 - 150.0,
                    bw,
                    bh,
                ],
                short,
                sp.key,
                &detail,
                WarHit::Power(p.index() as u8),
            );
            bt.enabled = cd <= 0.0 && !b.deploying;
            bt.on = run.power_aim == Some(*p);
            bt.icon = match p {
                wr::powers::Power::Grito => Glyph::Flag,
                wr::powers::Power::Rugido => Glyph::Fang,
                wr::powers::Power::Bendicion => Glyph::Staff,
                wr::powers::Power::Brasas => Glyph::Flame,
                wr::powers::Power::Niebla => Glyph::Wing,
                wr::powers::Power::Raices => Glyph::Tail,
            };
            view.buttons.push(bt);
        }
        if let (Some(p), Some((px, py))) = (run.power_aim, pointer) {
            let sp = p.spec();
            view.tip = Some(Tip {
                at: [px + 24.0, py + 24.0],
                edge: sp.color,
                kicker: "PODER DE LA REINA".into(),
                title: sp.name.into(),
                rows: vec![
                    ("Radio".into(), format!("{} m", sp.radius as u32)),
                    ("Espera".into(), format!("{} s", sp.cooldown as u32)),
                ],
                text: sp.what.into(),
                action: "Clic: lanzarlo aquí · Clic derecho: cancelar".into(),
            });
        }
    }
    // the build bar (right, under the clock), what this battle is for
    if let (Some(s), None) = (&sim.siege, &run.result) {
        let wood = s.wood[0];
        let x = SCREEN_W - 24.0 - 250.0;
        for (k, kind) in s.allowed(0).iter().enumerate() {
            let sp = kind.spec();
            let idx = wr::siege::Kind::ALL
                .iter()
                .position(|q| q == kind)
                .unwrap_or(0) as u8;
            let mut bt = btn(
                [x, 300.0 + k as f32 * 54.0, 250.0, 46.0],
                sp.name,
                "",
                &format!("{} madera", sp.cost as u32),
                WarHit::Build(idx),
            );
            bt.enabled = wood + 1e-3 >= sp.cost;
            bt.on = run.build == Some(*kind);
            view.buttons.push(bt);
        }
        let attacking = s.attacker(0);
        let left = (s.limit - sim.time).max(0.0) as u32;
        let clock = format!("{}:{:02}", left / 60, left % 60);
        bv.objective = Some(match s.mode {
            wr::siege::Mode::Siege if attacking => Objective {
                kicker: "ASEDIO · ATAQUE".into(),
                title: "Toma el torreón".into(),
                text: format!(
                    "Abre paso (rompe la puerta, abre brecha o arrima una torre) y aguanta en el patio del torreón {:.0}/{:.0} s · Quedan {clock}",
                    s.hold, s.hold_need
                ),
                progress: Some(s.hold / s.hold_need),
                edge: 0x4fd08a,
            },
            wr::siege::Mode::Siege => Objective {
                kicker: "ASEDIO · DEFENSA".into(),
                title: "Que no caiga el torreón".into(),
                text: if s.hold > 0.0 {
                    format!("¡Están tomando el patio del torreón! {:.0}/{:.0} s · Aguanta {clock}", s.hold, s.hold_need)
                } else {
                    format!("Defiende los muros, cierra las brechas y aguanta {clock}")
                },
                progress: Some(if s.hold > 0.0 {
                    s.hold / s.hold_need
                } else {
                    sim.time / s.limit
                }),
                edge: if s.hold > 0.0 { 0xe2334a } else { 0xf2b54a },
            },
            wr::siege::Mode::Field => Objective {
                kicker: "CAMPO ABIERTO".into(),
                title: "Derrota al enemigo".into(),
                text: "Puedes levantar estacas contra la caballería, empalizadas y escorpiones con tu madera.".into(),
                progress: None,
                edge: 0xf2b54a,
            },
        });
        if run.build.is_some() || run.engine_sel.is_some() {
            bv.messages.truncate(3);
        }
        // the tip: what is being placed, or the engine under the pointer
        if let (Some((_, _, _, why)), Some(kind), Some((px, py))) = (run.ghost, run.build, pointer)
        {
            let sp = kind.spec();
            view.tip = Some(Tip {
                at: [px + 24.0, py + 24.0],
                edge: if why.is_none() { 0x4fd08a } else { 0xe2334a },
                kicker: "CONSTRUIR".into(),
                title: sp.name.into(),
                rows: vec![
                    (
                        "Coste".into(),
                        format!("{} de {} madera", sp.cost as u32, wood as u32),
                    ),
                    (
                        "Tarda".into(),
                        if b.deploying {
                            "lista al empezar".into()
                        } else {
                            format!("{} s", sp.build as u32)
                        },
                    ),
                ],
                text: why.map_or_else(|| sp.what.to_owned(), |w| w.to_owned()),
                action: "Clic: construir · Clic derecho: cancelar".into(),
            });
        } else if let (Some(i), Some((px, py))) = (run.engine_hover.or(run.engine_sel), pointer) {
            let e = &s.engines[i];
            let sp = e.spec();
            let state = if !e.alive {
                "Destruida".to_owned()
            } else if e.built < 1.0 {
                format!("Construyendo {:.0}%", e.built * 100.0)
            } else if e.docked.is_some() {
                if e.kind == wr::siege::Kind::Ariete {
                    "Golpeando la puerta".into()
                } else {
                    "Arrimada a la muralla".into()
                }
            } else if matches!(
                e.kind,
                wr::siege::Kind::TorreAsedio | wr::siege::Kind::Ariete
            ) {
                if b.deploying {
                    "Esperando".into()
                } else {
                    "Avanzando".into()
                }
            } else if e.reload > 0.0 && (e.kind.stones() || e.kind.bolts()) {
                "Recargando".into()
            } else {
                "Lista".into()
            };
            let mut rows = vec![
                ("Salud".into(), format!("{:.0}/{:.0}", e.hp.max(0.0), e.hp0)),
                ("Estado".into(), state),
            ];
            if e.kind.stones() || e.kind.bolts() {
                rows.push((
                    "Alcance".into(),
                    format!("{:.0}–{:.0} m", sp.range.0, sp.range.1),
                ));
            }
            let mine = e.side == 0;
            view.tip = Some(Tip {
                at: [px + 24.0, py + 24.0],
                edge: if mine { 0xf2b54a } else { 0xe2334a },
                kicker: if mine {
                    "TU MÁQUINA".into()
                } else {
                    "MÁQUINA ENEMIGA".into()
                },
                title: sp.name.into(),
                rows,
                text: sp.what.into(),
                action: if !mine {
                    "Mándale tropas: si la rodean, la destrozan".into()
                } else if run.engine_sel == Some(i) {
                    if b.deploying {
                        "Clic derecho: objetivo · Supr: desmontarla".into()
                    } else {
                        "Clic derecho en un muro, una puerta o tropas: objetivo".into()
                    }
                } else {
                    "Clic: elegirla".into()
                },
            });
        }
    }
    if b.deploying {
        let mut bt = btn(
            [SCREEN_W / 2.0 + 470.0 - 336.0, 120.0, 300.0, 58.0],
            "¡A la batalla!",
            "Intro",
            "",
            WarHit::Begin,
        );
        bt.style = 1;
        bt.icon = Glyph::Swords;
        view.buttons.push(bt);
    }
    if run.help {
        let mut bt = btn(
            [SCREEN_W / 2.0 + 300.0, 850.0, 200.0, 52.0],
            "Cerrar",
            "F1",
            "",
            WarHit::HelpClose,
        );
        bt.layer = 1;
        view.buttons.push(bt);
    }
    if run.confirm {
        let mut yes = btn(
            [SCREEN_W / 2.0 - 340.0, 530.0, 320.0, 56.0],
            "Retirarse",
            "Intro",
            "",
            WarHit::Confirm(true),
        );
        yes.style = 3;
        yes.layer = 1;
        yes.icon = Glyph::Flag;
        view.buttons.push(yes);
        let mut no = btn(
            [SCREEN_W / 2.0 + 20.0, 530.0, 320.0, 56.0],
            "Seguir luchando",
            "Esc",
            "",
            WarHit::Confirm(false),
        );
        no.layer = 1;
        no.icon = Glyph::Swords;
        view.buttons.push(no);
    }
    if let Some((res, age)) = &run.result {
        // the result over a quiet field: no HUD under it
        view.buttons.clear();
        view.tip = None;
        bv.minimap = None;
        bv.cards.clear();
        bv.spark = None;
        bv.banners.clear();
        bv.messages.clear();
        let rows = |side: u8| -> (String, u32, Vec<ResultRow>) {
            let f = if side == 0 {
                b.setup.player.faction
            } else {
                b.setup.enemy.faction
            };
            let list = sim
                .regiments
                .iter()
                .filter(|r| r.side == side)
                .map(|r| ResultRow {
                    glyph: role_glyph(r.ty),
                    name: r.ty.name.to_owned(),
                    men0: r.men0,
                    lost: r.men0.saturating_sub(r.alive + r.fled),
                    kills: r.kills,
                    awake: r.awake,
                })
                .collect();
            (f.info().name.to_uppercase(), f.info().color, list)
        };
        bv.results = Some(Results {
            win: res.winner == 0,
            label: res.label.clone(),
            sides: [rows(0), rows(1)],
            note: res.note.clone(),
            age: *age,
        });
        let mut x = SCREEN_W / 2.0 + 620.0;
        let fy = SCREEN_H - 120.0;
        let mut push = |label: &str, icon: Glyph, hit: WarHit, style: u8| {
            x -= 330.0;
            let mut bt = btn([x, fy, 316.0, 62.0], label, "", "", hit);
            bt.icon = icon;
            bt.style = style;
            bt.layer = 1;
            view.buttons.push(bt);
            x -= 14.0;
        };
        if run.from_campaign.is_some() {
            push("Volver a la campaña", Glyph::Map, WarHit::Result(0), 1);
        } else {
            push("A la mesa de guerra", Glyph::Map, WarHit::Result(2), 1);
            push("Otra vez", Glyph::Swords, WarHit::Result(1), 0);
        }
    }
    if bv.results.is_none() {
        let mut avoid: Vec<[f32; 4]> = vec![
            // the title, the balance of power, the messages
            [16.0, 4.0, 800.0, 132.0],
            [SCREEN_W / 2.0 - 380.0, 6.0, 760.0, 104.0],
            [
                16.0,
                140.0,
                700.0,
                42.0 * bv.messages.len().min(5) as f32 + 8.0,
            ],
        ];
        if bv.deploying {
            avoid.push([SCREEN_W / 2.0 - 490.0, 86.0, 980.0, 132.0]);
        }
        for bt in view.buttons.iter().filter(|bt| bt.layer == 0) {
            let [x, y, w, h] = bt.rect;
            avoid.push([x - 10.0, y - 10.0, w + 20.0, h + 20.0]);
        }
        if let Some(m) = &bv.minimap {
            let [x, y, w, h] = m.rect;
            avoid.push([x - 16.0, y - 42.0, w + 32.0, h + 58.0]);
        }
        if let (Some(first), Some(last)) = (bv.cards.first(), bv.cards.last()) {
            let (x0, y0) = (first.rect[0], first.rect[1]);
            avoid.push([
                x0 - 10.0,
                y0 - 16.0,
                last.rect[0] + last.rect[2] - x0 + 20.0,
                SCREEN_H - y0 + 16.0,
            ]);
        }
        if let Some((c, r, ..)) = bv.spark {
            avoid.push([
                c[0] - r - 12.0,
                c[1] - r - 12.0,
                (r + 12.0) * 2.0,
                (r + 12.0) * 2.0,
            ]);
        }
        bv.banners = run_banners(run, camera, &avoid, view.time);
    }
    // the groups the HUD's controls stand in (their plates and kickers)
    let group = |hits: &dyn Fn(WarHit) -> bool| -> Option<[f32; 4]> {
        let rs: Vec<[f32; 4]> = view
            .buttons
            .iter()
            .filter(|bt| bt.layer == 0 && hits(bt.hit))
            .map(|bt| bt.rect)
            .collect();
        let x0 = rs.iter().map(|r| r[0]).fold(f32::MAX, f32::min);
        let y0 = rs.iter().map(|r| r[1]).fold(f32::MAX, f32::min);
        let x1 = rs.iter().map(|r| r[0] + r[2]).fold(f32::MIN, f32::max);
        let y1 = rs.iter().map(|r| r[1] + r[3]).fold(f32::MIN, f32::max);
        (!rs.is_empty()).then_some([x0, y0, x1 - x0, y1 - y0])
    };
    bv.speed_group = group(&|h| matches!(h, WarHit::Speed(_)));
    bv.orders_group = group(&|h| matches!(h, WarHit::Order(o) if o < 4));
    bv.powers_group = group(&|h| matches!(h, WarHit::Power(_)));
    if let Some(s) = &sim.siege {
        if let Some(r) = group(&|h| matches!(h, WarHit::Build(_))) {
            bv.build_group = Some((r, format!("CONSTRUIR · MADERA {}", s.wood[0] as u32)));
        }
    }
    view.battle = Some(bv);
}

/// How tall the panel's words stand (from its top), as the HUD lays them out: the title, the line
/// under it, its facts, its units three to a row, its notes (about as many letters a line as fit).
fn panel_height(p: &CampPanel) -> f32 {
    let lines = |s: &str, per: usize| s.chars().count().div_ceil(per).max(1) as f32;
    let mut y = 64.0;
    if !p.sub.is_empty() {
        y += lines(&p.sub, 44) * 25.0;
    }
    if !p.stats.is_empty() {
        y += 34.0;
    }
    if !p.units.is_empty() {
        y += 4.0 + p.units.len().div_ceil(3) as f32 * 34.0 + 6.0;
    }
    for (n, _) in &p.notes {
        y += lines(n, 52) * 23.0 + 4.0;
    }
    y + 14.0
}

fn campaign_view(
    view: &mut WarView,
    c: &CampaignRun,
    s: &Campaign,
    cam: &RtsCam,
    pointer: Option<(f32, f32)>,
    story: &StoryView,
) {
    let camera = cam.camera();
    let fog = App::fogged(s);
    let f = s.purse(Faction::Reina);
    let inc = s.income(Faction::Reina);
    let drain = s.spark_drain().len() as i32;
    let signed = |n: i32| {
        if n >= 0 {
            format!("+{n}")
        } else {
            format!("−{}", -n)
        }
    };
    let mut cv = CampaignView {
        resources: vec![
            ("ORO".into(), f.gold.to_string(), signed(inc.net_gold), inc.net_gold < 0, Glyph::Gold, 0xf2b54a),
            ("COMIDA".into(), f.food.to_string(), signed(inc.net_food), inc.net_food < 0, Glyph::Food, 0x4fd08a),
            ("CHISPA".into(), f.spark.to_string(), signed(inc.spark - drain), inc.spark - drain < 0, Glyph::Flame, 0x9a5cff),
        ],
        turn: if s.phase == CampPhase::Ai { format!("TURNO {} · MUEVEN LOS DEMÁS", s.turn) } else { format!("TURNO {}", s.turn) },
        season: format!("{} del año {}", SEASONS[((s.turn - 1) % 4) as usize], 1 + (s.turn - 1) / 4),
        toast: c.toast.clone(),
        hint: "CLIC: ELEGIR · CON UN EJÉRCITO, CLIC EN UNA PROVINCIA: MOVER, ATACAR O MARCHAR · TAB: SIGUIENTE EJÉRCITO · INTRO: FIN DEL TURNO".into(),
        objective: story.objective.clone(),
        ..Default::default()
    };
    // the names of the provinces (laid out at the end, off the HUD)
    use super::world_tags::TagAnchor;
    let mut anchors: Vec<TagAnchor> = Vec::new();
    let far = cam.dist;
    for (i, p) in PROVINCES.iter().enumerate() {
        let (sx, sz) = p.site();
        let Some(at) = project(camera, Vec3::new(sx, c.land.top_at(sx, sz) + 14.0, sz)) else {
            continue;
        };
        if at[0] < -100.0 || at[0] > SCREEN_W + 100.0 || at[1] < 0.0 || at[1] > SCREEN_H + 40.0 {
            continue;
        }
        let st = s.prov(p.id);
        let own = st.owner.info();
        let hidden = fog[i];
        let scale = (150.0 / far).clamp(0.7, 1.15);
        let owner = if hidden {
            "TIERRA SIN EXPLORAR".to_owned()
        } else if st.lit {
            own.short.to_uppercase()
        } else {
            format!("{} · SIN CHISPA", own.short.to_uppercase())
        };
        let (ns, os) = (24.0 * scale, 15.0 * scale);
        let w = label_width(&tr(p.name), ns).max(label_width(&tr(&owner), os) * 0.8) + 12.0;
        let h = ns * 1.25 + os * 1.2;
        anchors.push(TagAnchor {
            id: i,
            head: [at[0], at[1] - os * 0.3 + 10.0],
            foot: [at[0], at[1]],
            size: [w, h],
            compact: [w, h],
            priority: 50.0 + scale * 10.0,
            far: false,
        });
        cv.labels.push(ProvLabel {
            id: i,
            name: p.name.to_owned(),
            owner,
            color: if hidden { 0x9a8fb0 } else { own.bright },
            scale,
            ..Default::default()
        });
    }
    // the armies' plates
    let sel_id = match &c.sel {
        Some(Sel::Army(id)) => Some(id.clone()),
        _ => None,
    };
    for (k, a) in s
        .armies
        .iter()
        .enumerate()
        .filter(|(_, a)| !a.units.is_empty())
    {
        if a.faction != Faction::Reina && fog[wd::province_index(&a.at)] {
            continue;
        }
        let Some(t) = c.tokens.get(&a.id) else {
            continue;
        };
        let Some(at) = project(camera, t.at + Vec3::new(0.0, 7.5, 0.0)) else {
            continue;
        };
        if at[1] < 0.0 || at[1] > SCREEN_H + 30.0 {
            continue;
        }
        let foot = project(camera, t.at + Vec3::new(0.0, 4.0, 0.0))
            .filter(|f| f[1] > at[1])
            .unwrap_or([at[0], at[1] + 8.0]);
        let selected = sel_id.as_deref() == Some(a.id.as_str());
        let mine = a.faction == Faction::Reina;
        let men = a.men().to_string();
        let w = label_width(&tr(&a.name), 17.0)
            + label_width(&men, 17.0)
            + 60.0
            + if mine { 30.0 } else { 0.0 };
        anchors.push(TagAnchor {
            id: 1000 + k,
            head: [at[0], at[1] + 6.0],
            foot,
            size: [w, 30.0],
            compact: [w, 30.0],
            priority: if selected {
                1000.0
            } else if mine {
                520.0
            } else {
                500.0
            },
            far: false,
        });
        cv.armies.push(ArmyTag {
            id: 1000 + k,
            name: a.name.clone(),
            men,
            marching: a.march.is_some(),
            moves: mine.then_some((a.mp, 2)),
            edge: a.faction.info().color,
            glyph: role_glyph(wd::unit(&a.lead().ty)),
            selected,
            ..Default::default()
        });
    }
    // the panel
    let rect = [24.0, SCREEN_H - 24.0 - 430.0, 560.0, 430.0];
    let mut panel = CampPanel {
        rect,
        edge: 0x9a5cff,
        ..Default::default()
    };
    let mut buttons_y = rect[1] + 250.0;
    let recruit_buttons = |view: &mut WarView, prov: &str, y: &mut f32| {
        if s.prov(prov).owner != Faction::Reina {
            return;
        }
        for (k, o) in s.recruit_options(prov, Faction::Reina).iter().enumerate() {
            let t = wd::unit(o.ty);
            let cost = if t.cost.spark > 0 {
                format!("{} oro · {} chispa", t.cost.gold, t.cost.spark)
            } else {
                format!("{} oro", t.cost.gold)
            };
            let detail = if o.why.is_empty() {
                cost
            } else {
                format!("{} · {}", tr(&cost), tr(o.why))
            };
            let mut bt = btn(
                [rect[0] + 30.0, *y, rect[2] - 60.0, 54.0],
                t.name,
                "",
                &detail,
                WarHit::Panel(k as u8),
            );
            bt.icon = role_glyph(t);
            bt.enabled = o.ok && s.phase == CampPhase::Player;
            view.buttons.push(bt);
            *y += 60.0;
        }
        if !s.prov(prov).lit {
            let mut bt = btn(
                [rect[0] + 30.0, *y, rect[2] - 60.0, 54.0],
                "Encender la chispa aquí",
                "",
                "4 chispa",
                WarHit::Panel(100),
            );
            bt.icon = Glyph::Flame;
            bt.enabled = s.purse(Faction::Reina).spark >= 4 && s.phase == CampPhase::Player;
            view.buttons.push(bt);
            *y += 60.0;
        }
    };
    match &c.sel {
        Some(Sel::Army(id)) if s.army(id).is_some() => {
            let a = s.army(id).expect("an army");
            let fi = a.faction.info();
            let mine = a.faction == Faction::Reina;
            let p = wd::province(&a.at);
            panel.edge = fi.color;
            panel.kicker = if mine {
                "TU EJÉRCITO".into()
            } else {
                fi.short.to_uppercase()
            };
            panel.title = a.name.clone();
            panel.sub = if mine {
                format!(
                    "En {} · movimientos: {}/2 · {} soldados · fuerza {}",
                    p.name,
                    a.mp,
                    a.men(),
                    a.power().round()
                )
            } else {
                format!(
                    "En {} · {} soldados · fuerza {}",
                    p.name,
                    a.men(),
                    a.power().round()
                )
            };
            panel.units = a
                .units
                .iter()
                .map(|u| {
                    let t = wd::unit(&u.ty);
                    (
                        role_glyph(t),
                        t.short.to_owned(),
                        if t.role == Role::Hero {
                            String::new()
                        } else {
                            format!("{}/{}", u.men, u.max)
                        },
                    )
                })
                .collect();
            if mine && s.spark_drain().iter().any(|d| d == &a.id) {
                panel.notes.push((
                    "Lejos de la Reina y del sello: cuesta 1 de chispa por turno.".into(),
                    0xc9a2ff,
                ));
            }
            if let (true, Some(to)) = (mine, a.march.as_deref()) {
                let turns = s
                    .route(a, to)
                    .map_or(0, |w| s.march_turns(a, &w).last().copied().unwrap_or(1));
                let note = if turns <= 1 {
                    format!(
                        "En marcha hacia {}: llega este turno o el próximo.",
                        wd::province(to).name
                    )
                } else {
                    format!(
                        "En marcha hacia {}: {} turnos.",
                        wd::province(to).name,
                        turns
                    )
                };
                panel.notes.push((note, 0xf2b54a));
            } else if mine && s.phase == CampPhase::Player {
                panel.notes.push(("Elige un destino en el mapa: Mover, Tomar o Atacar al lado, o cualquier provincia para marchar hacia ella.".into(), 0xd9cce8));
            }
            buttons_y = rect[1] + panel_height(&panel);
            if mine && a.march.is_some() {
                let mut bt = btn(
                    [rect[0] + 30.0, buttons_y, rect[2] - 60.0, 54.0],
                    "Cancelar marcha",
                    "",
                    "",
                    WarHit::Panel(102),
                );
                bt.icon = Glyph::Flag;
                view.buttons.push(bt);
                buttons_y += 60.0;
            }
            if mine {
                recruit_buttons(view, &a.at.clone(), &mut buttons_y);
                let other = s.armies.iter().find(|b| {
                    b.id != a.id && b.at == a.at && b.faction == Faction::Reina && !b.garrison
                });
                if let Some(o) = other {
                    if o.units.len() + a.units.len() <= wd::MAX_UNITS {
                        let mut bt = btn(
                            [rect[0] + 30.0, buttons_y, rect[2] - 60.0, 54.0],
                            &format!("Unir con {}", o.name),
                            "",
                            "",
                            WarHit::Panel(101),
                        );
                        bt.icon = Glyph::Army;
                        view.buttons.push(bt);
                        buttons_y += 60.0;
                    }
                }
            }
        }
        Some(Sel::Province(i)) => {
            let p = &PROVINCES[*i];
            let st = s.prov(p.id);
            let fi = st.owner.info();
            let hidden = fog[*i];
            panel.edge = if hidden { 0xa898b8 } else { fi.color };
            panel.kicker = p.kicker.to_owned();
            panel.title = p.name.to_owned();
            panel.sub = if hidden {
                "Tierra sin explorar".into()
            } else if st.lit {
                fi.name.to_owned()
            } else {
                format!("{} · sin chispa (rinde una cuarta parte)", fi.name)
            };
            panel.notes.push((p.lore.to_owned(), 0xd9cce8));
            if let Some((line, found)) = story.relic.get(p.id) {
                panel
                    .notes
                    .push((line.clone(), if *found { 0x4fd08a } else { 0xf2b54a }));
            }
            if story.river_closed && super::war_story::EAST.contains(&p.id) {
                panel.notes.push((
                    "Al otro lado del río: tus ejércitos no cruzarán hasta que despiertes a Varrak.".into(),
                    0xe2334a,
                ));
            }
            if !hidden {
                let k = if st.lit { 1.0 } else { 0.25 };
                panel
                    .stats
                    .push((Glyph::Gold, format!("{} oro", (p.gold * k).round())));
                panel
                    .stats
                    .push((Glyph::Food, format!("{} comida", (p.food * k).round())));
                if p.spark > 0.0 {
                    panel
                        .stats
                        .push((Glyph::Flame, format!("{} chispa", p.spark)));
                }
                for a in s
                    .armies
                    .iter()
                    .filter(|a| a.at == p.id && !a.units.is_empty())
                {
                    panel.notes.push((
                        format!(
                            "{}: {} ({})",
                            tr(a.faction.info().short),
                            tr(&a.name),
                            a.men()
                        ),
                        0xa898b8,
                    ));
                }
                buttons_y = rect[1] + panel_height(&panel);
                recruit_buttons(view, p.id, &mut buttons_y);
                if st.owner == Faction::Libres {
                    let why = s.awaken_blocked(p.id);
                    let mut bt = btn(
                        [rect[0] + 30.0, buttons_y, rect[2] - 60.0, 54.0],
                        "Despertar con la chispa",
                        "",
                        &why.map_or_else(
                            || format!("{} chispa · se une sin batalla", wr::rules::AWAKEN_SPARK),
                            |w| w.to_owned(),
                        ),
                        WarHit::Panel(103),
                    );
                    bt.icon = Glyph::Flame;
                    bt.enabled = why.is_none() && s.phase == CampPhase::Player;
                    view.buttons.push(bt);
                    buttons_y += 60.0;
                }
            } else {
                panel
                    .notes
                    .push(("Nadie de la Reina ve lo que pasa aquí.".into(), 0xa898b8));
            }
        }
        _ => {
            panel.kicker = "LA CORTE DE LA REINA".into();
            panel.title = "El Continente Oscuro".into();
            panel.sub = "Elige un ejército o una provincia.".into();
            panel.notes.push(("La chispa de la Reina da mente a sus pueblos. Lejos de ella y del sello, sus ejércitos cuestan chispa; sin chispa, se apagan. Toma la capital caída para ganar; si cae el castillo, todo se pierde.".into(), 0xd9cce8));
        }
    }
    // the panel grows to hold its buttons
    let bottom = buttons_y
        .max(rect[1] + panel_height(&panel))
        .max(rect[1] + 260.0)
        + 16.0;
    let h = bottom - rect[1];
    let y0 = (SCREEN_H - 24.0 - h).max(110.0);
    let shift = y0 - rect[1];
    for bt in view.buttons.iter_mut() {
        if matches!(bt.hit, WarHit::Panel(_)) {
            bt.rect[1] += shift;
        }
    }
    panel.rect = [rect[0], y0, rect[2], h];
    cv.panel = panel;
    // the log
    // the latest four lines; the same news told again in a row counts (×3), not repeats
    let mut log: Vec<(u32, &str, LogKind, u32)> = Vec::new();
    for l in &s.log {
        let again = log
            .last()
            .is_some_and(|last| last.0 == l.turn && last.1 == l.text);
        if again {
            if let Some(last) = log.last_mut() {
                last.3 += 1;
            }
        } else if log.len() == 4 {
            break;
        } else {
            log.push((l.turn, &l.text, l.kind, 1));
        }
    }
    cv.log = log
        .into_iter()
        .map(|(turn, text, kind, n)| {
            let edge = match kind {
                LogKind::Good => 0x4fd08a,
                LogKind::Bad => 0xe2334a,
                LogKind::Spark => 0x9a5cff,
                LogKind::Lore => 0xf2b54a,
                LogKind::Info => 0xa898b8,
            };
            let times = if n > 1 {
                format!("  ×{n}")
            } else {
                String::new()
            };
            (format!("T{}  {}{}", turn, tr(text), times), edge)
        })
        .collect();
    // end of the turn, and back to the table
    let movable = s.armies.iter().filter(|a| can_move(a)).count();
    let playing =
        s.phase == CampPhase::Player && !c.busy && s.winner.is_none() && c.modal.is_none();
    let waiting = match movable {
        _ if story.turn_today => "Mañana: duerme para el siguiente turno".to_owned(),
        0 => String::new(),
        1 => "1 ejército puede moverse".to_owned(),
        n => format!("{n} ejércitos pueden moverse"),
    };
    let mut end = btn(
        [SCREEN_W - 24.0 - 380.0, SCREEN_H - 24.0 - 76.0, 380.0, 76.0],
        "Fin del turno",
        "Intro",
        if playing { &waiting } else { "" },
        WarHit::EndTurn,
    );
    end.style = 1;
    end.icon = Glyph::Hourglass;
    end.enabled = playing && !story.turn_today;
    view.buttons.push(end);
    let mut next = btn(
        [
            SCREEN_W - 24.0 - 380.0,
            SCREEN_H - 24.0 - 76.0 - 62.0,
            380.0,
            50.0,
        ],
        "Siguiente ejército",
        "Tab",
        "",
        WarHit::NextArmy,
    );
    next.icon = Glyph::Army;
    next.enabled = playing && movable > 0;
    view.buttons.push(next);
    let mut back = btn(
        [SCREEN_W - 24.0 - 58.0, 22.0, 58.0, 48.0],
        "",
        "",
        "",
        WarHit::Leave,
    );
    back.icon = Glyph::Back;
    view.buttons.push(back);
    // the tip over a province
    if let (Some(p), Some((px, py)), None) = (c.hover, pointer, &c.modal) {
        let st = s.prov(PROVINCES[p].id);
        let hidden = fog[p];
        let reach = App::sel_army(c, s)
            .filter(|a| a.faction == Faction::Reina && s.phase == CampPhase::Player)
            .is_some_and(|a| s.reachable(a).contains(&PROVINCES[p].id));
        let enemies: Vec<&wr::rules::Army> = s
            .armies
            .iter()
            .filter(|a| {
                a.at == PROVINCES[p].id
                    && a.faction != Faction::Reina
                    && !hidden
                    && !a.units.is_empty()
            })
            .collect();
        let far = App::sel_army(c, s)
            .filter(|a| {
                a.faction == Faction::Reina && !a.garrison && s.phase == CampPhase::Player && !reach
            })
            .and_then(|a| {
                s.route(a, PROVINCES[p].id)
                    .map(|w| s.march_turns(a, &w).last().copied().unwrap_or(1))
            });
        let action = if reach {
            if let Some(e) = enemies.first() {
                format!("Clic: atacar a {}", e.name)
            } else if st.owner != Faction::Reina {
                "Clic: tomar la provincia".into()
            } else {
                "Clic: mover aquí".into()
            }
        } else if let Some(turns) = far {
            if turns <= 1 {
                "Clic: marchar aquí (este turno)".into()
            } else {
                format!("Clic: marchar aquí ({turns} turnos)")
            }
        } else {
            String::new()
        };
        let mut rows = Vec::new();
        if let (true, Some(a)) = (reach && !enemies.is_empty(), App::sel_army(c, s)) {
            let mine = a.power();
            let theirs: f32 = enemies.iter().map(|e| e.power()).sum();
            rows.push((
                "Victoria".to_owned(),
                format!("{}%", (100.0 * mine / (mine + theirs).max(1.0)).round()),
            ));
        }
        view.tip = Some(Tip {
            at: [px + 18.0, py + 18.0],
            edge: if hidden {
                0xa898b8
            } else {
                st.owner.info().color
            },
            kicker: String::new(),
            title: PROVINCES[p].name.to_owned(),
            rows,
            text: if hidden {
                "Tierra sin explorar".into()
            } else {
                st.owner.info().name.to_owned()
            },
            action,
        });
    }
    // an open card
    match &c.modal {
        Some(CampModal::Intro) => {
            cv.modal = Some(Modal {
                rect: [SCREEN_W / 2.0 - 520.0, 200.0, 1040.0, 520.0],
                edge: 0xf2b54a,
                kicker: "CAMPAÑA · LA GUERRA DE LA CHISPA".into(),
                title: "El Continente Oscuro".into(),
                text: vec![
                    "La Reina ha despertado en su castillo del Bosque Oscuro. Su chispa da mente a orcos, trasgos, harpías y lamias, pero no llega a todos: en el norte, los clanes de Varrak y del Nido no pagan tributo y guían manadas de apagados. Al este del río, la Alianza vigila la Puerta del Este con miedo y con lanzas.".into(),
                    "Mueve tus ejércitos con clic derecho, recluta en tus asentamientos, enciende con la chispa lo que conquistes. Los ejércitos que se alejan de la Reina y del sello cuestan chispa cada turno. Toma la capital caída para ganar.".into(),
                ],
                versus: None,
            });
            let mut bt = btn(
                [SCREEN_W / 2.0 + 140.0, 630.0, 340.0, 62.0],
                "Que empiece",
                "Intro",
                "",
                WarHit::Modal(0),
            );
            bt.style = 1;
            bt.icon = Glyph::Crown;
            bt.layer = 1;
            view.buttons.push(bt);
        }
        Some(CampModal::End) => {
            let won = s.winner == Some(Faction::Reina);
            cv.modal = Some(Modal {
                rect: [SCREEN_W / 2.0 - 520.0, 240.0, 1040.0, 420.0],
                edge: if won { 0xf2b54a } else { 0xe2334a },
                kicker: if won {
                    format!("VICTORIA · TURNO {}", s.turn)
                } else {
                    format!("DERROTA · TURNO {}", s.turn)
                },
                title: if won {
                    "La capital vuelve a ser de la Reina".into()
                } else {
                    "El castillo ha caído".into()
                },
                text: vec![if won {
                    "Las agujas violetas se encienden una a una. Al otro lado del río, la Alianza cuenta sus miedos. Esto no ha terminado: ahora saben que la Reina ha vuelto.".into()
                } else {
                    "La Reina huye hacia la cueva del sello con lo que queda de su corte. La chispa parpadea, pero no se apaga.".into()
                }],
                versus: None,
            });
            for (k, (label, hit)) in [
                ("A la mesa de guerra", WarHit::Modal(0)),
                ("Nueva campaña", WarHit::Modal(1)),
            ]
            .into_iter()
            .enumerate()
            {
                let mut bt = btn(
                    [
                        SCREEN_W / 2.0 - 360.0 + k as f32 * 380.0,
                        570.0,
                        340.0,
                        62.0,
                    ],
                    label,
                    "",
                    "",
                    hit,
                );
                bt.style = if k == 1 { 1 } else { 0 };
                bt.layer = 1;
                view.buttons.push(bt);
            }
        }
        Some(CampModal::Battle(b)) => {
            if let (Some(att), Some(def)) = (s.army(&b.attacker), s.army(&b.defender)) {
                let player_attacks = att.faction == Faction::Reina;
                let (mine, theirs) = if player_attacks {
                    (att, def)
                } else {
                    (def, att)
                };
                let pm = mine.power();
                let pt = theirs.power() * if player_attacks { 1.1 } else { 1.0 };
                let ft = theirs.faction.info();
                let p = wd::province(&b.prov);
                let map = wr::maps::map_spec(p.battle, None);
                let list = |a: &wr::rules::Army| -> Vec<(Glyph, String, String)> {
                    a.units
                        .iter()
                        .map(|u| {
                            let t = wd::unit(&u.ty);
                            (
                                role_glyph(t),
                                t.short.to_owned(),
                                if t.role == Role::Hero {
                                    "héroe".to_owned()
                                } else {
                                    u.men.to_string()
                                },
                            )
                        })
                        .collect()
                };
                let rows = mine.units.len().max(theirs.units.len()) as f32;
                let h = 330.0 + rows * 26.0;
                let y = (SCREEN_H / 2.0 - h / 2.0 - 30.0).max(60.0);
                cv.modal = Some(Modal {
                    rect: [SCREEN_W / 2.0 - 560.0, y, 1120.0, h],
                    edge: ft.color,
                    kicker: format!(
                        "{} · {}",
                        tr(if player_attacks {
                            "ATACAS"
                        } else {
                            "TE ATACAN"
                        }),
                        tr(p.name).to_uppercase()
                    ),
                    title: format!("Batalla de {}", p.name),
                    text: vec![
                        if player_attacks {
                            format!(
                                "{} cae sobre {}. Campo de batalla: {}.",
                                mine.name, theirs.name, map.title
                            )
                        } else {
                            format!(
                                "{} ({}) cae sobre {}. Campo de batalla: {}.",
                                theirs.name, ft.short, mine.name, map.title
                            )
                        },
                        map.blurb.to_owned(),
                    ],
                    versus: Some(Versus {
                        names: [mine.name.clone(), theirs.name.clone()],
                        colors: [0xc79aff, ft.bright],
                        units: [list(mine), list(theirs)],
                        odds: pm / (pm + pt).max(1.0),
                        note: if player_attacks {
                            format!("Fuerzas: {} contra {}", pm.round(), pt.round())
                        } else {
                            format!(
                                "Fuerzas: {} contra {} (el defensor tiene ventaja)",
                                pm.round(),
                                pt.round()
                            )
                        },
                    }),
                });
                // (an attack on her is fought: there is no running from it)
                let can_retreat = false;
                let fy = y + h + 16.0;
                let mut x = SCREEN_W / 2.0 - 560.0;
                let mut push = |label: &str, icon: Glyph, hit: WarHit, style: u8, w: f32| {
                    let mut bt = btn([x, fy, w, 62.0], label, "", "", hit);
                    bt.icon = icon;
                    bt.style = style;
                    bt.layer = 1;
                    view.buttons.push(bt);
                    x += w + 14.0;
                };
                push(
                    "Librar la batalla",
                    Glyph::Swords,
                    WarHit::Modal(0),
                    1,
                    380.0,
                );
                push(
                    "Resolución automática",
                    Glyph::Hourglass,
                    WarHit::Modal(1),
                    0,
                    380.0,
                );
                if player_attacks {
                    push("Cancelar", Glyph::Back, WarHit::Modal(2), 0, 220.0);
                } else if can_retreat {
                    push("Retirarse", Glyph::Flag, WarHit::Modal(3), 3, 220.0);
                }
            }
        }
        None => {}
    }
    // the marches: the route of each, the turn each stretch is walked in, the goal (the chosen army's
    // bright, the others' faint); and, over a far province, the march the chosen army would make
    let chosen = match &c.sel {
        Some(Sel::Army(id)) => Some(id.clone()),
        _ => None,
    };
    let preview = App::sel_army(c, s)
        .filter(|a| {
            a.faction == Faction::Reina
                && !a.garrison
                && s.phase == CampPhase::Player
                && c.modal.is_none()
        })
        .zip(c.hover)
        .filter(|(a, h)| {
            a.at != PROVINCES[*h].id && !(a.mp > 0 && s.reachable(a).contains(&PROVINCES[*h].id))
        })
        .map(|(a, h)| (a.id.clone(), PROVINCES[h].id.to_owned()));
    let mut walks: Vec<(String, String, u8)> = s
        .armies
        .iter()
        .filter(|a| a.faction == Faction::Reina && !a.units.is_empty())
        .filter_map(|a| {
            a.march.clone().map(|m| {
                (
                    a.id.clone(),
                    m,
                    if chosen.as_deref() == Some(a.id.as_str()) {
                        2
                    } else {
                        1
                    },
                )
            })
        })
        .collect();
    if let Some((id, to)) = preview {
        walks.retain(|w| w.0 != id || w.2 != 2);
        walks.push((id, to, 3));
    }
    for (id, to, kind) in walks {
        let Some(a) = s.army(&id) else {
            continue;
        };
        let Some(way) = s.route(a, &to) else {
            continue;
        };
        let turns = s.march_turns(a, &way);
        let Some(start) = c
            .tokens
            .get(&id)
            .and_then(|t| project(camera, t.at + Vec3::new(0.0, 2.0, 0.0)))
        else {
            continue;
        };
        let mut pts = vec![start];
        let mut marks = Vec::new();
        for (k, p) in way.iter().enumerate() {
            let pr = wd::province(p);
            let Some(at) = project(
                camera,
                Vec3::new(pr.x, c.land.top_at(pr.x, pr.z) + 2.0, pr.z),
            ) else {
                continue;
            };
            pts.push(at);
            // where each turn's walk ends
            if k + 1 == way.len() || turns[k + 1] != turns[k] {
                marks.push((turns[k], at));
            }
        }
        cv.marches.push(MarchView {
            points: pts,
            turns: marks,
            kind,
        });
    }
    // where the chosen army can go: a chip on each province it reaches (slid along its route toward
    // the army when the HUD covers the place), a route to each
    if let Some(a) = App::sel_army(c, s).filter(|a| {
        a.faction == Faction::Reina
            && !a.garrison
            && a.mp > 0
            && s.phase == CampPhase::Player
            && !c.busy
            && c.modal.is_none()
    }) {
        let from = c
            .tokens
            .get(&a.id)
            .and_then(|t| project(camera, t.at + Vec3::new(0.0, 2.0, 0.0)));
        let mut hud: Vec<[f32; 4]> = vec![
            [12.0, 8.0, 1090.0, 150.0],
            [SCREEN_W / 2.0 - 540.0, SCREEN_H - 42.0, 1080.0, 36.0],
        ];
        let [px, py, pw, ph] = cv.panel.rect;
        hud.push([px - 10.0, py - 28.0, pw + 20.0, ph + 36.0]);
        if !cv.log.is_empty() {
            let top = SCREEN_H - 258.0 - (cv.log.len() as f32 - 1.0) * 44.0;
            hud.push([
                SCREEN_W - 652.0,
                top - 6.0,
                640.0,
                SCREEN_H - 258.0 + 42.0 - top + 6.0,
            ]);
        }
        hud.extend(view.buttons.iter().filter(|b| b.layer == 0).map(|b| {
            [
                b.rect[0] - 8.0,
                b.rect[1] - 8.0,
                b.rect[2] + 16.0,
                b.rect[3] + 16.0,
            ]
        }));
        let clear = |r: [f32; 4], hud: &[[f32; 4]]| {
            r[0] >= 8.0
                && r[1] >= 8.0
                && r[0] + r[2] <= SCREEN_W - 8.0
                && r[1] + r[3] <= SCREEN_H - 8.0
                && !hud.iter().any(|h| {
                    r[0] < h[0] + h[2]
                        && h[0] < r[0] + r[2]
                        && r[1] < h[1] + h[3]
                        && h[1] < r[1] + r[3]
                })
        };
        for to in s.reachable(a) {
            let i = wd::province_index(to);
            let p = &PROVINCES[i];
            let Some(at) = project(camera, Vec3::new(p.x, c.land.top_at(p.x, p.z) + 2.0, p.z))
            else {
                continue;
            };
            let hidden = fog[i];
            let enemies = !hidden
                && s.armies
                    .iter()
                    .any(|e| e.at == to && e.faction != Faction::Reina && !e.units.is_empty());
            let (label, glyph, style, color) = if enemies {
                ("Atacar", Glyph::Swords, 3, 0xe2334a)
            } else if hidden {
                ("Explorar", Glyph::Eye, 0, 0xc8b8d8)
            } else if s.prov(to).owner != Faction::Reina {
                ("Tomar", Glyph::Flag, 0, 0xf2b54a)
            } else {
                ("Mover", Glyph::Run, 0, 0xf2b54a)
            };
            // the action, and under it where (the chip may stand aside from the place)
            let w = label_width(&tr(label), 20.0).max(label_width(&tr(p.name), 15.0)) + 80.0;
            let (h, hh) = (54.0, 27.0);
            let place = |(t, dy): (f32, f32)| {
                let c = from.map_or(at, |f| {
                    [f[0] + (at[0] - f[0]) * t, f[1] + (at[1] - f[1]) * t]
                });
                [c[0] - w / 2.0, c[1] - hh + dy, w, h]
            };
            let tries = [1.0, 0.85, 0.7, 0.55, 0.4]
                .into_iter()
                .flat_map(|t| [(t, 0.0), (t, -52.0), (t, 52.0)]);
            // last, by the province's name (where it shows over the settlement)
            let (sx, sz) = p.site();
            let by_name = project(camera, Vec3::new(sx, c.land.top_at(sx, sz) + 14.0, sz))
                .into_iter()
                .flat_map(|n| {
                    [
                        [n[0] - w / 2.0, n[1] + 24.0, w, h],
                        [n[0] - w / 2.0, n[1] - 122.0, w, h],
                    ]
                });
            // or near the army, toward the place (turned up to 50° to find room)
            let near = from.into_iter().flat_map(|f| {
                let (dx, dy) = (at[0] - f[0], at[1] - f[1]);
                let len = dx.hypot(dy).max(1.0);
                let (ux, uy) = (dx / len, dy / len);
                [170.0f32, 130.0].into_iter().flat_map(move |d| {
                    [0.0f32, 25.0, -25.0, 50.0, -50.0]
                        .into_iter()
                        .map(move |deg| {
                            let (sn, cs) = deg.to_radians().sin_cos();
                            let (rx, ry) = (ux * cs - uy * sn, ux * sn + uy * cs);
                            [f[0] + rx * d - w / 2.0, f[1] + ry * d - hh, w, h]
                        })
                })
            });
            let Some(rect) = tries
                .map(place)
                .chain(by_name)
                .chain(near)
                .find(|r| clear(*r, &hud))
            else {
                continue;
            };
            hud.push([rect[0] - 6.0, rect[1] - 6.0, rect[2] + 12.0, rect[3] + 12.0]);
            let mut bt = btn(rect, label, "", p.name, WarHit::Go(i as u8));
            bt.icon = glyph;
            bt.style = style;
            let lit = pointer.is_some_and(|(px, py)| bt.contains(px, py));
            view.buttons.push(bt);
            if let Some(f) = from {
                cv.routes.push((
                    f,
                    [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0],
                    color,
                    lit,
                ));
            }
        }
    }
    let mut avoid: Vec<[f32; 4]> = vec![
        // the resources, the turn and the goal, the hint along the foot
        [12.0, 8.0, 1090.0, 150.0],
        [SCREEN_W / 2.0 - 540.0, SCREEN_H - 42.0, 1080.0, 36.0],
    ];
    let [px, py, pw, ph] = cv.panel.rect;
    avoid.push([px - 10.0, py - 28.0, pw + 20.0, ph + 36.0]);
    if !cv.log.is_empty() {
        let n = cv.log.len() as f32;
        let top = SCREEN_H - 258.0 - (n - 1.0) * 44.0;
        avoid.push([
            SCREEN_W - 652.0,
            top - 6.0,
            640.0,
            SCREEN_H - 258.0 + 42.0 - top + 6.0,
        ]);
    }
    if cv.toast.is_some() {
        avoid.push([SCREEN_W / 2.0 - 420.0, 110.0, 840.0, 112.0]);
    }
    for bt in view.buttons.iter().filter(|bt| bt.layer == 0) {
        let [x, y, w, h] = bt.rect;
        avoid.push([x - 10.0, y - 10.0, w + 20.0, h + 20.0]);
    }
    let placed = c
        .tags
        .borrow_mut()
        .update(&anchors, &avoid, [0.0, 0.0, SCREEN_W, SCREEN_H], view.time)
        .to_vec();
    let find = |id: usize| placed.iter().find(|p| p.id == id).copied();
    cv.labels.retain_mut(|l| {
        find(l.id)
            .map(|p| {
                l.rect = p.rect;
                l.alpha = p.alpha;
            })
            .is_some()
    });
    cv.armies.retain_mut(|a| {
        find(a.id)
            .map(|p| {
                a.rect = p.rect;
                a.pole = p.pole;
                a.alpha = p.alpha;
            })
            .is_some()
    });
    view.campaign = Some(cv);
}

/// The war as a place.
pub(super) struct WarZone;

impl Zone for WarZone {
    fn id(&self) -> ZoneId {
        ZoneId::War
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_war(dt);
    }
    fn prompt(&self, _app: &App) -> Option<Prompt> {
        None
    }
    fn interact(&self, _app: &mut App) -> bool {
        false
    }
    fn soundtrack(&self, app: &App) -> soundtrack::Zone {
        match app.war.as_ref().map(|w| w.page) {
            Some(WarPage::Battle) => soundtrack::Zone::Cave,
            _ => soundtrack::Zone::Castle,
        }
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        if let Some(war) = frame.war {
            state.render_war(frame, war);
        }
    }
    fn title(&self, _app: &App) -> (&'static str, &'static str) {
        ("La mesa de guerra", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_army_stands_its_regiments_apart_on_a_base_that_holds_them() {
        for n in 1..=wd::MAX_UNITS {
            let (spots, radius) = formation(n);
            assert_eq!(spots.len(), n);
            for (i, a) in spots.iter().enumerate() {
                assert!(
                    a[0].hypot(a[1]) + 1.0 <= radius,
                    "{n}: {a:?} off the base {radius}"
                );
                for b in &spots[i + 1..] {
                    assert!(
                        (a[0] - b[0]).hypot(a[1] - b[1]) >= 2.0,
                        "{n}: {a:?} on {b:?}"
                    );
                }
            }
        }
        // the base grows with the army
        assert!(formation(10).1 > formation(4).1 && formation(4).1 > formation(1).1 - 0.01);
    }
}
