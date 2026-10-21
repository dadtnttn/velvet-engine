//! Sieges, and what armies build in a battle.
//!
//! A battle on a walled place (a battle map with enclosures) is a siege: one side holds the walls
//! (the defender), the other comes to take the place. The attackers win by bringing the keep down
//! or by holding the ground round it; the defenders win by lasting the clock out or breaking the
//! attackers. The walls, towers, gates and keep are the map's works, each with its strength.
//!
//! Both sides build with the wood they have (more for whoever brings a siege): before the battle
//! (in their deployment zone, ready at once) or during it (near their own troops, taking a while
//! and open to attack while the carpenters work). What each can build depends on the battle:
//!
//! - attacking walls: catapults and trebuchets (stones: walls, towers, the keep, men in the open),
//!   siege towers (rolled to a wall, the bridge down: the wall is crossed there), rams (rolled to
//!   a gate and batter it in), ballistas, stakes;
//! - holding walls: catapults and ballistas behind them (for the engines and the crowds outside),
//!   palisades to close a breach, stakes;
//! - in the open, rudimentary things: stakes (horses cannot charge through them), palisades, a
//!   scorpion (a light ballista).
//!
//! Engines shoot on their own at what their side needs most (the gate, then the wall, then the
//! keep; or the enemy's engines, then the thickest crowd), unless given a target. Soldiers next to
//! an enemy engine, a gate or a palisade hack at it.
use crate::baked::{BattleMap, WorkKind};
use crate::data::Role;
use crate::field::{Barrier, Point};
use crate::sim::{Over, Sim};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// In the open: both sides may build rudimentary things.
    Field,
    /// A walled place: `Siege::defender` holds it.
    Siege,
}

/// What can be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Catapulta,
    Trabuquete,
    Balista,
    Escorpion,
    TorreAsedio,
    Ariete,
    Estacas,
    Empalizada,
}

/// What a kind of engine is like.
pub struct Spec {
    pub name: &'static str,
    pub what: &'static str,
    pub cost: f32,
    /// Seconds to build during the battle (before it, built at once).
    pub build: f32,
    pub hp: f32,
    /// Range it shoots in (min, max), 0 for none.
    pub range: (f32, f32),
    pub reload: f32,
    /// Metres a second it is rolled (0: it stays where it is built).
    pub speed: f32,
    /// How wide it stands (its radius; lines: their half thickness).
    pub radius: f32,
    /// Lines (stakes, palisades): how long.
    pub length: f32,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Catapulta,
        Kind::Trabuquete,
        Kind::Balista,
        Kind::Escorpion,
        Kind::TorreAsedio,
        Kind::Ariete,
        Kind::Estacas,
        Kind::Empalizada,
    ];

    pub fn spec(self) -> &'static Spec {
        match self {
            Kind::Catapulta => &Spec {
                name: "Catapulta",
                what: "Piedras sobre muros, torres y tropas a lo lejos",
                cost: 30.0,
                build: 18.0,
                hp: 240.0,
                range: (40.0, 170.0),
                reload: 7.5,
                speed: 0.0,
                radius: 3.0,
                length: 0.0,
            },
            Kind::Trabuquete => &Spec {
                name: "Trabuquete",
                what: "Lanza rocas enormes muy lejos: derriba murallas y el torreón",
                cost: 45.0,
                build: 30.0,
                hp: 360.0,
                range: (70.0, 260.0),
                reload: 13.0,
                speed: 0.0,
                radius: 4.5,
                length: 0.0,
            },
            Kind::Balista => &Spec {
                name: "Balista",
                what: "Virotes que atraviesan filas enteras y dañan máquinas",
                cost: 20.0,
                build: 12.0,
                hp: 160.0,
                range: (12.0, 140.0),
                reload: 5.0,
                speed: 0.0,
                radius: 2.2,
                length: 0.0,
            },
            Kind::Escorpion => &Spec {
                name: "Escorpión",
                what: "Ballesta grande sobre un caballete: barata y rápida",
                cost: 10.0,
                build: 8.0,
                hp: 110.0,
                range: (10.0, 110.0),
                reload: 3.5,
                speed: 0.0,
                radius: 1.6,
                length: 0.0,
            },
            Kind::TorreAsedio => &Spec {
                name: "Torre de asedio",
                what: "Se arrima a la muralla y baja su puente: tus tropas suben por ella",
                cost: 40.0,
                build: 28.0,
                hp: 650.0,
                range: (0.0, 0.0),
                reload: 0.0,
                speed: 1.5,
                radius: 3.4,
                length: 0.0,
            },
            Kind::Ariete => &Spec {
                name: "Ariete",
                what: "Bajo su tejado, golpea la puerta hasta romperla",
                cost: 25.0,
                build: 16.0,
                hp: 520.0,
                range: (0.0, 0.0),
                reload: 2.4,
                speed: 1.3,
                radius: 2.6,
                length: 0.0,
            },
            Kind::Estacas => &Spec {
                name: "Estacas",
                what: "Una línea de estacas afiladas: la caballería no carga a través",
                cost: 5.0,
                build: 4.0,
                hp: 120.0,
                range: (0.0, 0.0),
                reload: 0.0,
                speed: 0.0,
                radius: 0.9,
                length: 9.0,
            },
            Kind::Empalizada => &Spec {
                name: "Empalizada",
                what: "Un muro de troncos: corta el paso (y tapa una brecha)",
                cost: 8.0,
                build: 6.0,
                hp: 280.0,
                range: (0.0, 0.0),
                reload: 0.0,
                speed: 0.0,
                radius: 0.6,
                length: 8.0,
            },
        }
    }

    /// Whether it shoots stones (an arc), bolts (straight), or neither.
    pub fn stones(self) -> bool {
        matches!(self, Kind::Catapulta | Kind::Trabuquete)
    }
    pub fn bolts(self) -> bool {
        matches!(self, Kind::Balista | Kind::Escorpion)
    }
    /// A line (placed by its middle and turned), not an engine.
    pub fn line(self) -> bool {
        matches!(self, Kind::Estacas | Kind::Empalizada)
    }
    /// It stands in the way of feet (paths go round it).
    pub fn blocks(self) -> bool {
        !matches!(self, Kind::Estacas | Kind::TorreAsedio | Kind::Ariete)
    }
}

/// A work of the place (a wall, a tower, a gate, the keep, a palisade), as strong as it is.
#[derive(Clone, Debug)]
pub struct Work {
    pub id: usize,
    pub kind: WorkKind,
    pub hp: f32,
    pub hp0: f32,
    pub broken: bool,
    pub x: f32,
    pub z: f32,
    pub top: f32,
    /// When it was last hit (for its dust and its shake).
    pub hit: f32,
}

/// What an engine is told to shoot at, or where to roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Auto,
    Work(usize),
    Regiment(usize),
    Engine(usize),
}

#[derive(Clone, Debug)]
pub struct Engine {
    pub kind: Kind,
    pub side: u8,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub hp: f32,
    pub hp0: f32,
    /// How far built (1: ready).
    pub built: f32,
    pub reload: f32,
    pub target: Target,
    pub alive: bool,
    /// Seconds since it last shot (or struck): its arm's swing.
    pub fired: f32,
    /// Rolling engines: where they go, and the wall they have reached (its work, the gap).
    pub path: Vec<Point>,
    pub path_i: usize,
    pub goal: Option<usize>,
    pub docked: Option<(usize, usize)>,
    /// Burning out after it was wrecked.
    pub burn: f32,
    pub selected: bool,
    barrier: Option<usize>,
}

impl Engine {
    pub fn spec(&self) -> &'static Spec {
        self.kind.spec()
    }
    pub fn ready(&self) -> bool {
        self.alive && self.built >= 1.0
    }
    /// A line's two ends.
    pub fn ends(&self) -> ((f32, f32), (f32, f32)) {
        let half = self.spec().length / 2.0;
        let (dx, dz) = (self.yaw.cos() * half, -self.yaw.sin() * half);
        ((self.x - dx, self.z - dz), (self.x + dx, self.z + dz))
    }
}

/// A stone or a bolt in the air.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub side: u8,
    pub from: Kind,
    pub done: bool,
    life: f32,
    pierce: u32,
}

/// What happened, for the view (dust, debris, sounds, words).
#[derive(Clone, Copy, Debug)]
pub enum SiegeEvent {
    Shot { engine: usize },
    Impact { x: f32, z: f32, big: bool },
    WorkHit { work: usize, x: f32, z: f32 },
    Breach { work: usize, x: f32, z: f32 },
    GateDown { work: usize, x: f32, z: f32 },
    KeepDown { work: usize },
    Built { engine: usize },
    Wrecked { engine: usize },
    Docked { engine: usize },
    Ram { engine: usize },
    Captured { side: u8 },
    Held { side: u8 },
}

pub struct Siege {
    pub mode: Mode,
    /// The side holding the walls (a field battle: none).
    pub defender: Option<u8>,
    pub works: Vec<Work>,
    pub engines: Vec<Engine>,
    pub shots: Vec<Shot>,
    /// Wood left to build with, per side, and what each had.
    pub wood: [f32; 2],
    pub wood0: [f32; 2],
    /// The keep's ground: where attackers who hold it win the place.
    pub hold_at: Option<(f32, f32, f32)>,
    /// How long the attackers have held it (they need `hold_need`).
    pub hold: f32,
    pub hold_need: f32,
    /// The defenders win when the clock reaches this.
    pub limit: f32,
    /// Where the attackers can get in (a gate down, a breach, a tower's bridge).
    pub openings: Vec<(f32, f32)>,
    pub events: Vec<SiegeEvent>,
    tick: f32,
    seed: u32,
    pending_removals: Vec<usize>,
}

impl Siege {
    /// The siege (or the field's building) of a battle: the map's works, the wood per side.
    pub fn new(map: Option<&BattleMap>, defender: Option<u8>, wood: [f32; 2]) -> Siege {
        let mut works = Vec::new();
        let mut hold_at = None;
        if let Some(m) = map {
            for w in &m.works {
                let kind = w.kind();
                let hp = match kind {
                    WorkKind::Wall => 1600.0,
                    WorkKind::Tower => 2200.0,
                    WorkKind::Gate => 1400.0,
                    WorkKind::Keep => 9000.0,
                    WorkKind::Palisade => 420.0,
                };
                let [x, z] = w.centre();
                works.push(Work {
                    id: w.id,
                    kind,
                    hp,
                    hp0: hp,
                    broken: false,
                    x,
                    z,
                    top: w.top(),
                    hit: -10.0,
                });
            }
            if defender.is_some() {
                hold_at = m
                    .keep()
                    .map(|k| {
                        let [x, z] = k.centre();
                        let r = k.w.unwrap_or(24.0).max(k.d.unwrap_or(24.0)) * 0.5 + 16.0;
                        (x, z, r)
                    })
                    .or_else(|| {
                        let ring = m.enclosures.first()?;
                        let n = ring.len() as f32;
                        let x = ring.iter().map(|p| p[0]).sum::<f32>() / n;
                        let z = ring.iter().map(|p| p[1]).sum::<f32>() / n;
                        Some((x, z, 26.0))
                    });
            }
        }
        Siege {
            mode: if defender.is_some() {
                Mode::Siege
            } else {
                Mode::Field
            },
            defender,
            works,
            engines: Vec::new(),
            shots: Vec::new(),
            wood,
            wood0: wood,
            hold_at,
            hold: 0.0,
            hold_need: 45.0,
            limit: 15.0 * 60.0,
            openings: Vec::new(),
            events: Vec::new(),
            tick: 0.0,
            seed: 0x9e37_79b9,
            pending_removals: Vec::new(),
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed % 10_000) as f32 / 10_000.0
    }

    /// Whether `side` is the one coming at the walls.
    pub fn attacker(&self, side: u8) -> bool {
        self.defender.is_some_and(|d| d != side)
    }

    /// What `side` may build in this battle.
    pub fn allowed(&self, side: u8) -> &'static [Kind] {
        match (self.mode, self.attacker(side)) {
            (Mode::Siege, true) => &[
                Kind::Catapulta,
                Kind::Trabuquete,
                Kind::TorreAsedio,
                Kind::Ariete,
                Kind::Balista,
                Kind::Estacas,
            ],
            (Mode::Siege, false) => &[
                Kind::Catapulta,
                Kind::Balista,
                Kind::Empalizada,
                Kind::Estacas,
            ],
            (Mode::Field, _) => &[Kind::Estacas, Kind::Empalizada, Kind::Escorpion],
        }
    }

    /// Why `kind` cannot be built at (x, z) by `side` now, if it cannot. `zone`: the side's
    /// deployment zone, where it builds before the battle.
    #[allow(clippy::too_many_arguments)]
    pub fn check(
        &self,
        sim: &Sim,
        side: u8,
        kind: Kind,
        x: f32,
        z: f32,
        yaw: f32,
        deploying: bool,
        zone: crate::maps::Rect,
    ) -> Result<(), &'static str> {
        if !self.allowed(side).contains(&kind) {
            return Err("No se puede construir aquí");
        }
        let sp = kind.spec();
        if self.wood[side as usize] + 1e-3 < sp.cost {
            return Err("No queda madera");
        }
        if deploying {
            if x < zone.x0 || x > zone.x1 || z < zone.z0 || z > zone.z1 {
                return Err("Fuera de tu zona");
            }
        } else {
            let near_own = sim
                .regiments
                .iter()
                .any(|r| r.side == side && r.alive > 0 && (r.cx - x).hypot(r.cz - z) < 45.0);
            if !near_own {
                return Err("Lejos de tus tropas");
            }
            let near_foe = sim
                .regiments
                .iter()
                .any(|r| r.side != side && r.alive > 0 && (r.cx - x).hypot(r.cz - z) < 28.0);
            if near_foe {
                return Err("Demasiado cerca del enemigo");
            }
        }
        let f = &sim.field;
        // the ground it stands on: free, not in deep water, not too steep
        let probe: Vec<(f32, f32)> = if kind.line() {
            let half = sp.length / 2.0;
            let (dx, dz) = (yaw.cos(), -yaw.sin());
            (0..=6)
                .map(|k| {
                    let t = -half + sp.length * k as f32 / 6.0;
                    (x + dx * t, z + dz * t)
                })
                .collect()
        } else {
            let r = sp.radius;
            vec![(x, z), (x + r, z), (x - r, z), (x, z + r), (x, z - r)]
        };
        // not on top of another engine
        for e in self.engines.iter().filter(|e| e.alive) {
            let room = e.spec().radius.max(e.spec().length / 2.0) + sp.radius.max(sp.length / 2.0);
            if (e.x - x).hypot(e.z - z) < room * 0.8 {
                return Err("Choca con otra máquina");
            }
        }
        for &(px, pz) in &probe {
            if f.blocked(px, pz) {
                return Err("El terreno no deja");
            }
        }
        let (gx, gz) = f.grad_at(x, z);
        if !kind.line() && gx.hypot(gz) > 0.45 {
            return Err("Demasiada pendiente");
        }
        if kind == Kind::TorreAsedio || kind == Kind::Ariete {
            if let Some(m) = &f.baked {
                if m.inside(x, z) {
                    return Err("Se construye fuera de los muros");
                }
            }
        }
        Ok(())
    }

    /// Builds `kind` at (x, z) for `side` (`instant`: before the battle). Its index.
    pub fn place(
        &mut self,
        sim: &mut Sim,
        side: u8,
        kind: Kind,
        x: f32,
        z: f32,
        yaw: f32,
        instant: bool,
    ) -> usize {
        let sp = kind.spec();
        self.wood[side as usize] = (self.wood[side as usize] - sp.cost).max(0.0);
        let mut e = Engine {
            kind,
            side,
            x,
            z,
            yaw,
            hp: sp.hp,
            hp0: sp.hp,
            built: if instant { 1.0 } else { 0.0 },
            reload: sp.reload * 0.5,
            target: Target::Auto,
            alive: true,
            fired: 10.0,
            path: Vec::new(),
            path_i: 0,
            goal: None,
            docked: None,
            burn: 0.0,
            selected: false,
            barrier: None,
        };
        if instant && kind.blocks() {
            e.barrier = Some(sim.field.add_barrier(barrier_of(&e)));
        }
        self.engines.push(e);
        let i = self.engines.len() - 1;
        if instant {
            self.events.push(SiegeEvent::Built { engine: i });
        }
        i
    }

    /// Gives back a building not yet begun on (the battle has not started): its wood returned.
    pub fn unbuild(&mut self, sim: &mut Sim, i: usize) {
        let Some(e) = self.engines.get_mut(i) else {
            return;
        };
        if !e.alive {
            return;
        }
        e.alive = false;
        e.burn = 0.0;
        self.wood[e.side as usize] += e.spec().cost;
        if let Some(b) = e.barrier.take() {
            sim.field.remove_barrier(b);
        }
    }

    /// The point to aim an engine at for a target, if it is still there.
    fn aim(&self, sim: &Sim, t: Target) -> Option<(f32, f32)> {
        match t {
            Target::Auto => None,
            Target::Work(w) => self.works.get(w).filter(|w| !w.broken).map(|w| (w.x, w.z)),
            Target::Regiment(r) => sim
                .regiments
                .get(r)
                .filter(|r| r.alive > 0 && !r.awake)
                .map(|r| (r.cx, r.cz)),
            Target::Engine(e) => self.engines.get(e).filter(|e| e.alive).map(|e| (e.x, e.z)),
        }
    }

    /// What an engine shoots at on its own, and where that is.
    fn auto_target(&self, sim: &Sim, i: usize) -> Option<(Target, f32, f32)> {
        let e = &self.engines[i];
        let (rmin, rmax) = e.spec().range;
        let in_range = |x: f32, z: f32| {
            let d = (x - e.x).hypot(z - e.z);
            d >= rmin && d <= rmax
        };
        let map = sim.field.baked.as_ref();
        let near_wall = |w: &Work| -> (f32, f32) {
            map.and_then(|m| m.works.get(w.id))
                .map(|s| {
                    let p = s.nearest(e.x, e.z);
                    (p[0], p[1])
                })
                .unwrap_or((w.x, w.z))
        };
        let foe_engines = || {
            self.engines
                .iter()
                .enumerate()
                .filter(|(_, o)| o.alive && o.side != e.side && in_range(o.x, o.z))
                .min_by(|a, b| {
                    let pa = matches!(a.1.kind, Kind::TorreAsedio | Kind::Ariete) as u8;
                    let pb = matches!(b.1.kind, Kind::TorreAsedio | Kind::Ariete) as u8;
                    pb.cmp(&pa).then(
                        (a.1.x - e.x)
                            .hypot(a.1.z - e.z)
                            .total_cmp(&(b.1.x - e.x).hypot(b.1.z - e.z)),
                    )
                })
                .map(|(k, o)| (Target::Engine(k), o.x, o.z))
        };
        let crowd = |prefer_big: bool| {
            sim.regiments
                .iter()
                .filter(|r| {
                    r.side != e.side
                        && r.alive > 0
                        && !r.awake
                        && !r.ty.flyer
                        && in_range(r.cx, r.cz)
                })
                .max_by(|a, b| {
                    let s = |r: &crate::sim::Regiment| {
                        r.alive as f32
                            + if prefer_big && (r.ty.large || r.ty.role == Role::Cavalry) {
                                40.0
                            } else {
                                0.0
                            }
                    };
                    s(a).total_cmp(&s(b))
                })
                .map(|r| (Target::Regiment(r.index), r.cx, r.cz))
        };
        if e.kind.bolts() {
            return crowd(true).or_else(foe_engines);
        }
        if self.attacker(e.side) {
            // the gate first, then the walls and towers nearest the keep, then the keep
            let keep = self.hold_at.map(|(x, z, _)| (x, z)).unwrap_or((0.0, 0.0));
            let pick = |kinds: &[WorkKind]| {
                self.works
                    .iter()
                    .filter(|w| !w.broken && kinds.contains(&w.kind))
                    .filter_map(|w| {
                        let (x, z) = near_wall(w);
                        in_range(x, z).then_some((w, x, z))
                    })
                    .min_by(|a, b| {
                        let d = |p: &(&Work, f32, f32)| {
                            (p.1 - e.x).hypot(p.2 - e.z) * 0.5 + (p.1 - keep.0).hypot(p.2 - keep.1)
                        };
                        d(a).total_cmp(&d(b))
                    })
                    .map(|(w, x, z)| (Target::Work(w.id), x, z))
            };
            if self.openings.is_empty() {
                if let Some(t) =
                    pick(&[WorkKind::Gate]).or_else(|| pick(&[WorkKind::Wall, WorkKind::Palisade]))
                {
                    return Some(t);
                }
            }
            return pick(&[WorkKind::Keep])
                .or_else(foe_engines)
                .or_else(|| crowd(false))
                .or_else(|| pick(&[WorkKind::Tower, WorkKind::Wall]));
        }
        foe_engines().or_else(|| crowd(false))
    }

    // ------------------------------------------------------------------ the step
    pub fn step(&mut self, sim: &mut Sim, dt: f32) {
        if sim.over.is_some() {
            self.fly(sim, dt);
            return;
        }
        self.tick += dt;
        let slow = self.tick >= 0.25;
        if slow {
            self.tick = 0.0;
        }
        for i in 0..self.engines.len() {
            if !self.engines[i].alive {
                self.engines[i].burn = (self.engines[i].burn - dt).max(0.0);
                continue;
            }
            self.engines[i].fired += dt;
            if self.engines[i].built < 1.0 {
                let b = self.engines[i].spec().build;
                self.engines[i].built = (self.engines[i].built + dt / b.max(0.1)).min(1.0);
                if self.engines[i].built >= 1.0 {
                    if self.engines[i].kind.blocks() {
                        let bar = barrier_of(&self.engines[i]);
                        self.engines[i].barrier = Some(sim.field.add_barrier(bar));
                    }
                    self.events.push(SiegeEvent::Built { engine: i });
                }
                continue;
            }
            match self.engines[i].kind {
                Kind::TorreAsedio | Kind::Ariete => self.roll(sim, i, dt),
                k if k.stones() || k.bolts() => self.shoot(sim, i, dt),
                _ => {}
            }
        }
        self.fly(sim, dt);
        if slow {
            self.hack(sim, 0.25);
            self.stakes(sim, 0.25);
            self.objective(sim, 0.25);
        }
    }

    fn shoot(&mut self, sim: &mut Sim, i: usize, dt: f32) {
        let t = self.engines[i].target;
        let aim = match self.aim(sim, t) {
            Some(p) => Some((t, p.0, p.1)),
            None => {
                if t != Target::Auto {
                    self.engines[i].target = Target::Auto;
                }
                self.auto_target(sim, i)
            }
        };
        let e = &mut self.engines[i];
        let Some((_, tx, tz)) = aim else {
            e.reload = e.reload.max(0.5);
            return;
        };
        // turn to it
        let want = (tx - e.x).atan2(tz - e.z);
        let mut d = want - e.yaw;
        while d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        while d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        e.yaw += d.clamp(-dt * 0.8, dt * 0.8);
        e.reload -= dt;
        if e.reload > 0.0 || d.abs() > 0.2 {
            return;
        }
        let dist = (tx - e.x).hypot(tz - e.z);
        let (rmin, rmax) = e.spec().range;
        if dist < rmin || dist > rmax {
            return;
        }
        e.reload = e.spec().reload * (0.9 + 0.2 * ((i as f32 * 7.31 + sim.time).sin() * 0.5 + 0.5));
        e.fired = 0.0;
        let (kind, side, ex, ez) = (e.kind, e.side, e.x, e.z);
        let ground = sim.field.height_at(ex, ez);
        if kind.stones() {
            let spread = dist * if kind == Kind::Trabuquete { 0.03 } else { 0.04 };
            let ox = (self.rand() - 0.5) * 2.0 * spread;
            let oz = (self.rand() - 0.5) * 2.0 * spread;
            let (gx, gz) = (tx + ox, tz + oz);
            let (gy, _) = sim.field.top_at(gx, gz);
            let y0 = ground + if kind == Kind::Trabuquete { 11.0 } else { 5.0 };
            let t = (dist / 52.0).clamp(1.4, 5.0);
            let g = 9.8;
            self.shots.push(Shot {
                x: ex,
                y: y0,
                z: ez,
                vx: (gx - ex) / t,
                vy: (gy - y0) / t + 0.5 * g * t,
                vz: (gz - ez) / t,
                side,
                from: kind,
                done: false,
                life: t + 2.0,
                pierce: 0,
            });
        } else {
            let y0 = ground + 1.6;
            let ty = sim.field.height_at(tx, tz) + 1.0;
            let speed = 70.0;
            let (dx, dy, dz) = (tx - ex, ty - y0, tz - ez);
            let l = (dx * dx + dy * dy + dz * dz).sqrt().max(0.1);
            self.shots.push(Shot {
                x: ex,
                y: y0,
                z: ez,
                vx: dx / l * speed,
                vy: dy / l * speed,
                vz: dz / l * speed,
                side,
                from: kind,
                done: false,
                life: rmax / speed + 0.5,
                pierce: if kind == Kind::Balista { 3 } else { 2 },
            });
        }
        self.events.push(SiegeEvent::Shot { engine: i });
    }

    /// A tower or a ram rolled to its wall or gate; at it, the tower's bridge comes down (the wall
    /// is crossed there) or the ram batters.
    fn roll(&mut self, sim: &mut Sim, i: usize, dt: f32) {
        let kind = self.engines[i].kind;
        let map = match sim.field.baked.clone() {
            Some(m) => m,
            None => return,
        };
        if let Some((w, _)) = self.engines[i].docked {
            if kind == Kind::Ariete {
                let e = &mut self.engines[i];
                e.reload -= dt;
                if e.reload <= 0.0 {
                    e.reload = e.spec().reload;
                    e.fired = 0.0;
                    let (x, z) = (e.x, e.z);
                    self.events.push(SiegeEvent::Ram { engine: i });
                    self.hurt_work(sim, w, 90.0, x, z);
                    if self.works[w].broken {
                        self.engines[i].docked = None;
                        self.engines[i].goal = None;
                    }
                }
            }
            return;
        }
        // the goal: the target given, or the nearest gate (a ram) / wall (a tower) still standing
        let wanted = |s: &Siege, k: WorkKind| {
            s.works
                .iter()
                .filter(|w| !w.broken && w.kind == k)
                .min_by(|a, b| {
                    let e = &s.engines[i];
                    (a.x - e.x)
                        .hypot(a.z - e.z)
                        .total_cmp(&(b.x - e.x).hypot(b.z - e.z))
                })
                .map(|w| w.id)
        };
        let goal = match self.engines[i].target {
            Target::Work(w) if !self.works.get(w).is_none_or(|w| w.broken) => Some(w),
            _ => {
                if kind == Kind::Ariete {
                    wanted(self, WorkKind::Gate).or_else(|| wanted(self, WorkKind::Palisade))
                } else {
                    wanted(self, WorkKind::Wall)
                }
            }
        };
        let Some(goal) = goal else {
            return;
        };
        let e = &self.engines[i];
        let spec = &map.works[goal];
        // the approach: just outside the work, on the engine's side of it
        let p = spec.nearest(e.x, e.z);
        let (dx, dz) = (e.x - p[0], e.z - p[1]);
        let l = dx.hypot(dz).max(0.1);
        let stand = if kind == Kind::Ariete { 2.6 } else { 3.6 } + spec.thick.unwrap_or(3.0) / 2.0;
        let (ax, az) = (p[0] + dx / l * stand, p[1] + dz / l * stand);
        if self.engines[i].goal != Some(goal) || self.engines[i].path.is_empty() {
            let path = sim.field.find_path(e.x, e.z, ax, az);
            let en = &mut self.engines[i];
            en.goal = Some(goal);
            en.path = path;
            en.path_i = 0;
        }
        let speed = self.engines[i].spec().speed;
        let en = &mut self.engines[i];
        let at = en
            .path
            .get(en.path_i)
            .copied()
            .unwrap_or(Point { x: ax, z: az });
        let (mx, mz) = (at.x - en.x, at.z - en.z);
        let d = mx.hypot(mz);
        if d < 0.6 {
            if en.path_i + 1 < en.path.len() {
                en.path_i += 1;
            } else {
                // arrived: dock against it
                en.yaw = (p[0] - en.x).atan2(p[1] - en.z);
                if kind == Kind::TorreAsedio {
                    let gap = sim.field.add_gap(p[0], p[1], 3.4);
                    en.docked = Some((goal, gap));
                    self.openings.push((p[0], p[1]));
                    self.events.push(SiegeEvent::Docked { engine: i });
                } else {
                    en.docked = Some((goal, usize::MAX));
                    en.reload = 0.8;
                    self.events.push(SiegeEvent::Docked { engine: i });
                }
            }
            return;
        }
        let step = (speed * dt).min(d);
        en.x += mx / d * step;
        en.z += mz / d * step;
        let want = mx.atan2(mz);
        en.yaw += (want - en.yaw).sin().clamp(-1.0, 1.0) * dt * 1.2;
    }

    fn fly(&mut self, sim: &mut Sim, dt: f32) {
        for k in 0..self.shots.len() {
            let mut s = self.shots[k];
            if s.done {
                continue;
            }
            s.life -= dt;
            if s.from.stones() {
                s.vy -= 9.8 * dt;
            }
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.z += s.vz * dt;
            if s.from.bolts() {
                // a bolt: the first men along its way, through a few of them
                for i in sim.soldiers_near(s.x, s.z, 1.3) {
                    let r = sim.reg[i];
                    if sim.regiments[r].side == s.side || (sim.py[i] + 1.0 - s.y).abs() > 2.0 {
                        continue;
                    }
                    let dmg = if s.from == Kind::Balista { 65.0 } else { 40.0 };
                    sim.hurt(i, dmg * (1.0 - sim.regiments[r].ty.armor / 140.0));
                    if s.pierce == 0 {
                        s.done = true;
                        break;
                    }
                    s.pierce -= 1;
                }
                for j in 0..self.engines.len() {
                    let e = &self.engines[j];
                    if e.alive && e.side != s.side && (e.x - s.x).hypot(e.z - s.z) < e.spec().radius
                    {
                        let dmg = if s.from == Kind::Balista { 45.0 } else { 25.0 };
                        self.hurt_engine(j, dmg);
                        s.done = true;
                    }
                }
                let (top, _) = sim.field.top_at(s.x, s.z);
                if s.y <= top || s.life <= 0.0 {
                    s.done = true;
                }
                self.shots[k] = s;
                continue;
            }
            let (top, work) = sim.field.top_at(s.x, s.z);
            if s.y <= top || s.life <= 0.0 {
                s.done = true;
                let big = s.from == Kind::Trabuquete;
                let (wd, md, r) = if big {
                    (260.0, 75.0, 4.2)
                } else {
                    (110.0, 55.0, 3.2)
                };
                self.events.push(SiegeEvent::Impact {
                    x: s.x,
                    z: s.z,
                    big,
                });
                if let Some(w) = work.or_else(|| self.work_near(sim, s.x, s.z, 3.0)) {
                    // only the enemy's works (a stone thrown over its own wall does it no harm)
                    if self.defender.is_some_and(|d| d != s.side) {
                        // (the keep is the last to fall: its walls are the thickest)
                        // (the keep is the last to fall: its walls are the thickest; a gate is a
                        // small mark for a stone: the ram is what breaks it)
                        let k = match self.works.get(w).map(|k| k.kind) {
                            Some(WorkKind::Keep) => 0.5,
                            Some(WorkKind::Gate) => 0.25,
                            _ => 1.0,
                        };
                        self.hurt_work(sim, w, wd * k, s.x, s.z);
                    }
                }
                for i in sim.soldiers_near(s.x, s.z, r) {
                    let rg = sim.reg[i];
                    if sim.regiments[rg].side == s.side {
                        continue;
                    }
                    let d = (sim.px[i] - s.x).hypot(sim.pz[i] - s.z);
                    sim.hurt(i, md * (1.0 - d / r * 0.5));
                }
                for j in 0..self.engines.len() {
                    let e = &self.engines[j];
                    if e.alive
                        && e.side != s.side
                        && (e.x - s.x).hypot(e.z - s.z) < e.spec().radius + r * 0.6
                    {
                        self.hurt_engine(j, if big { 160.0 } else { 90.0 });
                    }
                }
            }
            self.shots[k] = s;
        }
        self.shots.retain(|s| !s.done);
    }

    /// The work standing within `r` of (x, z), if any (the raster round the point).
    fn work_near(&self, sim: &Sim, x: f32, z: f32, r: f32) -> Option<usize> {
        let mut k = -r;
        while k <= r {
            let mut j = -r;
            while j <= r {
                if let Some(w) = sim.field.work_at(x + k, z + j) {
                    return Some(w);
                }
                j += 1.0;
            }
            k += 1.0;
        }
        None
    }

    fn hurt_work(&mut self, sim: &mut Sim, w: usize, dmg: f32, x: f32, z: f32) {
        let Some(work) = self.works.get_mut(w) else {
            return;
        };
        if work.broken {
            return;
        }
        work.hp -= dmg;
        work.hit = sim.time;
        self.events.push(SiegeEvent::WorkHit { work: w, x, z });
        if work.hp > 0.0 {
            return;
        }
        work.broken = true;
        let (wx, wz, kind) = (work.x, work.z, work.kind);
        sim.field.set_open(w, true);
        match kind {
            WorkKind::Gate => {
                self.openings.push((wx, wz));
                self.events.push(SiegeEvent::GateDown {
                    work: w,
                    x: wx,
                    z: wz,
                });
            }
            WorkKind::Keep => {
                self.events.push(SiegeEvent::KeepDown { work: w });
                if let Some(d) = self.defender {
                    sim.over = Some(Over {
                        winner: 1 - d,
                        time: sim.time,
                    });
                    sim.events.push(crate::sim::Event::End { winner: 1 - d });
                }
            }
            _ => {
                self.openings.push((x, z));
                self.events.push(SiegeEvent::Breach { work: w, x, z });
            }
        }
    }

    fn hurt_engine(&mut self, j: usize, dmg: f32) {
        let e = &mut self.engines[j];
        if !e.alive {
            return;
        }
        e.hp -= dmg;
        if e.hp <= 0.0 {
            e.alive = false;
            e.burn = 9.0;
            self.events.push(SiegeEvent::Wrecked { engine: j });
            self.pending_removals.push(j);
        }
    }

    /// Soldiers next to an enemy engine, a gate or a palisade hack at it; an engine against the
    /// walls takes the defenders' boiling oil and stones from above.
    fn hack(&mut self, sim: &mut Sim, dt: f32) {
        for j in 0..self.engines.len() {
            let e = &self.engines[j];
            if e.alive && e.docked.is_some() && self.defender.is_some_and(|d| d != e.side) {
                let burn = if e.kind == Kind::Ariete { 6.0 } else { 4.0 };
                self.hurt_engine(j, burn * dt);
            }
        }
        for j in 0..self.engines.len() {
            let (alive, side, x, z, r) = {
                let e = &self.engines[j];
                (
                    e.alive,
                    e.side,
                    e.x,
                    e.z,
                    e.spec().radius.max(e.spec().length / 2.0),
                )
            };
            if !alive {
                continue;
            }
            let n = sim
                .soldiers_near(x, z, r + 1.8)
                .into_iter()
                .filter(|&i| sim.regiments[sim.reg[i]].side != side)
                .count();
            if n > 0 {
                self.hurt_engine(j, n as f32 * 1.1 * dt);
            }
        }
        // Who breaks a work: in a siege, its attackers; in the open (a palisade round a village),
        // whoever presses on it to get through, not those holding behind it.
        let defender = self.defender;
        let breaks = |r: &crate::sim::Regiment| {
            r.alive > 0
                && match defender {
                    Some(d) => r.side != d,
                    None => !r.is_hold(),
                }
        };
        for w in 0..self.works.len() {
            let (kind, broken, x, z) = {
                let k = &self.works[w];
                (k.kind, k.broken, k.x, k.z)
            };
            if broken || !matches!(kind, WorkKind::Gate | WorkKind::Palisade) {
                continue;
            }
            let reach = if kind == WorkKind::Gate { 7.0 } else { 3.0 };
            let (px, pz) = match (&sim.field.baked, kind) {
                (Some(m), WorkKind::Palisade) => {
                    // the palisade's nearest stretch to the press of attackers
                    let near = sim
                        .regiments
                        .iter()
                        .filter(|r| breaks(r))
                        .map(|r| m.works[w].nearest(r.cx, r.cz))
                        .min_by(|a, b| {
                            (a[0] - x)
                                .hypot(a[1] - z)
                                .total_cmp(&(b[0] - x).hypot(b[1] - z))
                        });
                    near.map_or((x, z), |p| (p[0], p[1]))
                }
                _ => (x, z),
            };
            let n = sim
                .soldiers_near(px, pz, reach)
                .into_iter()
                .filter(|&i| breaks(&sim.regiments[sim.reg[i]]))
                .count();
            if n > 0 {
                self.hurt_work(sim, w, n as f32 * 0.8 * dt, px, pz);
            }
        }
        self.drop_barriers(sim);
    }

    /// The wrecked engines' barriers come down, a sunk tower's bridge goes.
    fn drop_barriers(&mut self, sim: &mut Sim) {
        for j in std::mem::take(&mut self.pending_removals) {
            let e = &mut self.engines[j];
            if let Some(b) = e.barrier.take() {
                sim.field.remove_barrier(b);
            }
            if let Some((_, gap)) = e.docked.take() {
                if gap != usize::MAX {
                    sim.field.remove_gap(gap);
                }
            }
        }
    }

    /// Stakes: horses and beasts that run into them are hurt and stopped.
    fn stakes(&mut self, sim: &mut Sim, dt: f32) {
        for j in 0..self.engines.len() {
            let e = &self.engines[j];
            if !e.ready() || e.kind != Kind::Estacas {
                continue;
            }
            let ((ax, az), (bx, bz)) = e.ends();
            let bar = Barrier {
                ax,
                az,
                bx,
                bz,
                half: 1.4,
            };
            let side = e.side;
            for i in sim.soldiers_near(e.x, e.z, e.spec().length / 2.0 + 2.0) {
                let r = sim.reg[i];
                let t = sim.regiments[r].ty;
                if sim.regiments[r].side == side || bar.dist(sim.px[i], sim.pz[i]) > 1.4 {
                    continue;
                }
                let horse = matches!(t.role, Role::Cavalry | Role::Beast) || t.large;
                sim.stall(i, if horse { 0.25 } else { 0.7 });
                if horse {
                    sim.hurt(i, 34.0 * dt);
                    self.hurt_engine(j, 6.0 * dt);
                }
            }
        }
    }

    /// The place falls to whoever holds its keep's ground long enough; it holds out if the
    /// attackers have not taken it when the clock runs out.
    fn objective(&mut self, sim: &mut Sim, dt: f32) {
        let Some(d) = self.defender else {
            return;
        };
        let attacker = 1 - d;
        if let Some((x, z, r)) = self.hold_at {
            let mut count = [0usize; 2];
            for i in sim.soldiers_near(x, z, r) {
                let rg = &sim.regiments[sim.reg[i]];
                if rg.state.routed() || rg.awake {
                    continue;
                }
                count[rg.side as usize] += 1;
            }
            let was = self.hold;
            // held: at least six of them there, twice as many as the defenders left round it
            if count[attacker as usize] >= 6 && count[attacker as usize] >= count[d as usize] * 2 {
                self.hold += dt;
            } else {
                self.hold = (self.hold - dt * 0.5).max(0.0);
            }
            if was == 0.0 && self.hold > 0.0 {
                self.events.push(SiegeEvent::Held { side: attacker });
            }
            if self.hold >= self.hold_need {
                self.events.push(SiegeEvent::Captured { side: attacker });
                sim.over = Some(Over {
                    winner: attacker,
                    time: sim.time,
                });
                sim.events.push(crate::sim::Event::End { winner: attacker });
                return;
            }
        }
        if sim.time >= self.limit {
            sim.over = Some(Over {
                winner: d,
                time: sim.time,
            });
            sim.events.push(crate::sim::Event::End { winner: d });
        }
    }

    /// A rolling engine sent at a wall or a gate (by its index), or a shooting one at a target.
    pub fn order(&mut self, i: usize, target: Target) {
        if let Some(e) = self.engines.get_mut(i) {
            if e.alive && !e.kind.line() {
                e.target = target;
                if matches!(e.kind, Kind::TorreAsedio | Kind::Ariete) && e.docked.is_none() {
                    e.path.clear();
                }
            }
        }
    }

    /// The engines `side` sets up before the battle by itself: the AI's siege (the map's own
    /// trebuchets, towers and ram where the prototype had them standing, else a battery of its
    /// own), the defenders' ballistas behind the gate, stakes in the open.
    pub fn ai_setup(&mut self, sim: &mut Sim, side: u8, zone: crate::maps::Rect) {
        let Some(map) = sim.field.baked.clone() else {
            return;
        };
        let mut tries = 0;
        let mut put =
            |s: &mut Siege, sim: &mut Sim, kind: Kind, x: f32, z: f32, yaw: f32| -> bool {
                tries += 1;
                for (ox, oz) in [
                    (0.0, 0.0),
                    (6.0, 0.0),
                    (-6.0, 0.0),
                    (0.0, 6.0),
                    (0.0, -6.0),
                    (9.0, 9.0),
                    (-9.0, -9.0),
                ] {
                    if s.check(sim, side, kind, x + ox, z + oz, yaw, true, zone)
                        .is_ok()
                    {
                        s.place(sim, side, kind, x + ox, z + oz, yaw, true);
                        return true;
                    }
                }
                false
            };
        let centre = ((zone.x0 + zone.x1) / 2.0, (zone.z0 + zone.z1) / 2.0);
        if self.attacker(side) {
            let mut wide = zone;
            wide.x0 -= 30.0;
            wide.x1 += 30.0;
            wide.z0 -= 30.0;
            wide.z1 += 30.0;
            let keep = self.hold_at.map(|(x, z, _)| (x, z)).unwrap_or((0.0, 0.0));
            for e in &map.engines {
                let kind = match e.kind.as_str() {
                    "trabuquete" => Kind::Trabuquete,
                    "catapulta" => Kind::Catapulta,
                    "torre_asedio" => Kind::TorreAsedio,
                    "ariete" => Kind::Ariete,
                    _ => continue,
                };
                if self.wood[side as usize] < kind.spec().cost {
                    continue;
                }
                let yaw = (keep.0 - e.x).atan2(keep.1 - e.z);
                // (the prototype's camp may stand beyond the deployment zone: allowed there)
                if self
                    .check(sim, side, kind, e.x, e.z, yaw, true, wide)
                    .is_ok()
                {
                    self.place(sim, side, kind, e.x, e.z, yaw, true);
                }
            }
            if !self
                .engines
                .iter()
                .any(|e| e.side == side && e.kind.stones())
            {
                let yaw = (keep.0 - centre.0).atan2(keep.1 - centre.1);
                for dx in [-30.0, 30.0] {
                    put(self, sim, Kind::Catapulta, centre.0 + dx, centre.1, yaw);
                }
            }
            if !self
                .engines
                .iter()
                .any(|e| e.side == side && e.kind == Kind::Ariete)
            {
                let yaw = (keep.0 - centre.0).atan2(keep.1 - centre.1);
                put(self, sim, Kind::Ariete, centre.0, centre.1 - 10.0, yaw);
            }
            if !self
                .engines
                .iter()
                .any(|e| e.side == side && e.kind == Kind::TorreAsedio)
            {
                let yaw = (keep.0 - centre.0).atan2(keep.1 - centre.1);
                put(
                    self,
                    sim,
                    Kind::TorreAsedio,
                    centre.0 + 45.0,
                    centre.1 - 6.0,
                    yaw,
                );
            }
        } else if self.mode == Mode::Siege {
            // ballistas behind each gate, a catapult by the keep
            let gates: Vec<(f32, f32)> = self
                .works
                .iter()
                .filter(|w| w.kind == WorkKind::Gate)
                .map(|w| (w.x, w.z))
                .collect();
            let keep = self.hold_at.map(|(x, z, _)| (x, z)).unwrap_or(centre);
            for (gx, gz) in gates.into_iter().take(2) {
                let (dx, dz) = (keep.0 - gx, keep.1 - gz);
                let l = dx.hypot(dz).max(1.0);
                let (bx, bz) = (gx + dx / l * 16.0, gz + dz / l * 16.0);
                let yaw = (-dx).atan2(-dz);
                for s in [-1.0, 1.0] {
                    put(
                        self,
                        sim,
                        Kind::Balista,
                        bx - dz / l * 9.0 * s,
                        bz + dx / l * 9.0 * s,
                        yaw,
                    );
                }
            }
            let yaw = 0.0;
            put(
                self,
                sim,
                Kind::Catapulta,
                keep.0 + 14.0,
                keep.1 + 14.0,
                yaw,
            );
        } else {
            // in the open: a scorpion behind the middle, stakes before the line
            let (fx, fz) = {
                let foe = sim
                    .regiments
                    .iter()
                    .filter(|r| r.side != side)
                    .map(|r| (r.cx, r.cz))
                    .fold((0.0, 0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1, a.2 + 1.0));
                let mine = (centre.0, centre.1);
                if foe.2 > 0.0 {
                    let (tx, tz) = (foe.0 / foe.2 - mine.0, foe.1 / foe.2 - mine.1);
                    let l = tx.hypot(tz).max(1.0);
                    (tx / l, tz / l)
                } else {
                    (0.0, -1.0)
                }
            };
            let yaw = fx.atan2(fz);
            put(
                self,
                sim,
                Kind::Escorpion,
                centre.0 - fx * 18.0,
                centre.1 - fz * 18.0,
                yaw,
            );
            for s in [-1.0, 0.0, 1.0] {
                let (x, z) = (
                    centre.0 + fx * 30.0 - fz * 12.0 * s,
                    centre.1 + fz * 30.0 + fx * 12.0 * s,
                );
                put(
                    self,
                    sim,
                    Kind::Estacas,
                    x,
                    z,
                    yaw + std::f32::consts::FRAC_PI_2,
                );
            }
        }
        let _ = tries;
    }
}

fn barrier_of(e: &Engine) -> Barrier {
    if e.kind.line() {
        let ((ax, az), (bx, bz)) = e.ends();
        Barrier {
            ax,
            az,
            bx,
            bz,
            half: e.spec().radius,
        }
    } else {
        Barrier {
            ax: e.x,
            az: e.z,
            bx: e.x,
            bz: e.z,
            half: e.spec().radius * 0.85,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_numbers() {
        for k in Kind::ALL {
            let s = k.spec();
            assert!(s.cost > 0.0 && s.hp > 0.0 && s.build > 0.0, "{}", s.name);
            if k.stones() || k.bolts() {
                assert!(s.range.1 > s.range.0 && s.reload > 0.0, "{}", s.name);
            }
            if k.line() {
                assert!(s.length > 0.0, "{}", s.name);
            }
        }
    }
}
