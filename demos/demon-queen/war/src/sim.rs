//! The battle, simulated: every soldier is an agent (structure of arrays), regiments hold the
//! formation, the orders, morale, the spark and ammunition. Fixed 30 Hz steps; the game's view
//! reads the arrays and the events.
//!
//! Soldier loop (each step): grid → targets (staggered) → steering (slot / enemy / flight) →
//! separation → move (blocked water, terrain speed) → facing → melee strikes → charges.
//! Regiment loop (each step, morale every 0.5 s): centroid, the anchor along its path, engagement,
//! ranged fire, the spark (cohesion), morale, rout and rally, the end of the battle.
use std::f32::consts::{PI, TAU};

use crate::data::{unit, Arc, AuraKind, Faction, Projectile, Role, UnitType};
use crate::field::{Field, Point};
use crate::noise::{ang_diff, clamp, turn_to, Rng};

pub const DT: f32 = 1.0 / 30.0;
const CELL: f32 = 2.5;
pub const S_ALIVE: u8 = 0;
pub const S_DEAD: u8 = 1;
pub const S_FLED: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RState {
    Ready,
    Moving,
    Melee,
    Firing,
    Routing,
    Shattered,
    Fled,
    Dead,
    Awake,
}

impl RState {
    pub fn routed(self) -> bool {
        matches!(self, RState::Routing | RState::Shattered)
    }
}

#[derive(Clone, Debug)]
pub enum Order {
    Hold {
        files: u32,
    },
    Move {
        path: Vec<Point>,
        i: usize,
        heading: f32,
        files: u32,
        run: bool,
    },
    Attack {
        target: usize,
        run: bool,
        files: u32,
        charged: Option<f32>,
        path_t: f32,
        route: Option<Vec<Point>>,
    },
    Flee,
}

#[derive(Clone, Copy, Debug)]
enum Pending {
    Move {
        x: f32,
        z: f32,
        heading: f32,
        files: u32,
        run: bool,
    },
    Attack {
        target: usize,
        run: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Ability {
    pub cd: f32,
    pub cd_max: f32,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct Regiment {
    pub index: usize,
    pub id: String,
    pub ty: &'static UnitType,
    pub side: u8,
    pub look: Faction,
    pub soldiers: Vec<usize>,
    pub men0: u32,
    pub alive: u32,
    pub cx: f32,
    pub cz: f32,
    /// The formation's anchor (where the block is going to stand).
    pub ax: f32,
    pub az: f32,
    pub heading: f32,
    pub files: u32,
    pub order: Order,
    pub state: RState,
    pub morale: f32,
    pub morale_base: f32,
    /// The spark still in it (100 lit … 0 dimmed).
    pub cohesion: f32,
    pub ammo: u32,
    pub ammo0: u32,
    pub rout_t: f32,
    pub rout_count: u32,
    flank_t: f32,
    rear_t: f32,
    losses: [f32; 20],
    loss_i: usize,
    pub lost: u32,
    pub kills: u32,
    pub fled: u32,
    pub engaged: u32,
    idle_t: f32,
    pub fire_target: Option<usize>,
    pub skirmish: bool,
    pub fire_at_will: bool,
    pub run: bool,
    pub speed_now: f32,
    pub lag: f32,
    pub wavering: bool,
    pub dimmed: bool,
    pub ability: Option<Ability>,
    pub awake: bool,
    delay: f32,
    pending: Option<Pending>,
    charge_shock: f32,
    pub last_fire: f32,
    pub selected: bool,
    pub firing: bool,
    pub charging: bool,
    pub diving: bool,
    skirmish_cd: f32,
    flee_from: Option<usize>,
    flee_t: f32,
    flee_to: Option<Point>,
    pub crumble_told: bool,
}

impl Regiment {
    pub fn order_files(&self) -> u32 {
        match &self.order {
            Order::Hold { files } | Order::Move { files, .. } | Order::Attack { files, .. } => {
                *files
            }
            Order::Flee => self.files,
        }
    }

    pub fn order_target(&self) -> Option<usize> {
        match &self.order {
            Order::Attack { target, .. } => Some(*target),
            _ => None,
        }
    }

    pub fn is_moving_order(&self) -> bool {
        matches!(self.order, Order::Move { .. })
    }

    pub fn is_hold(&self) -> bool {
        matches!(self.order, Order::Hold { .. })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Spark {
        x: f32,
        z: f32,
        r: f32,
        side: u8,
    },
    Awaken {
        x: f32,
        z: f32,
        reg: usize,
    },
    Death {
        x: f32,
        z: f32,
        side: u8,
        large: bool,
        reg: usize,
    },
    QueenDown {
        reg: usize,
    },
    HeroDown {
        reg: usize,
    },
    Clash {
        x: f32,
        z: f32,
    },
    Hit {
        x: f32,
        z: f32,
        big: bool,
    },
    Charge {
        reg: usize,
        x: f32,
        z: f32,
    },
    Arrive {
        reg: usize,
    },
    Dimmed {
        reg: usize,
    },
    Rekindled {
        reg: usize,
    },
    Rally {
        reg: usize,
    },
    Crumble {
        reg: usize,
    },
    Rout {
        reg: usize,
    },
    Shatter {
        reg: usize,
    },
    End {
        winner: u8,
    },
    Brace {
        x: f32,
        z: f32,
    },
    Impact {
        x: f32,
        z: f32,
        big: bool,
    },
    Fire {
        x: f32,
        z: f32,
        kind: Projectile,
        reg: usize,
    },
    Glow {
        x: f32,
        z: f32,
    },
    Block {
        x: f32,
        z: f32,
    },
    Pierce {
        x: f32,
        z: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Missile {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    g: f32,
    t: f32,
    big_t: f32,
    side: u8,
    reg: usize,
    pub kind: Projectile,
    dmg: f32,
    ap: f32,
    cohesion: f32,
    ax: f32,
    ay: f32,
    az: f32,
    pub done: bool,
    checked: bool,
    pub stuck: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Over {
    pub winner: u8,
    pub time: f32,
}

pub struct Sim {
    pub field: Field,
    rand: Rng,
    pub time: f32,
    pub dimming: bool,
    pub n: usize,
    pub px: Vec<f32>,
    pub pz: Vec<f32>,
    pub py: Vec<f32>,
    pub vx: Vec<f32>,
    pub vz: Vec<f32>,
    pub yaw: Vec<f32>,
    pub hp: Vec<f32>,
    pub reg: Vec<usize>,
    pub state: Vec<u8>,
    pub target: Vec<i32>,
    cool: Vec<f32>,
    pub stagger: Vec<f32>,
    pub knock: Vec<f32>,
    charge: Vec<f32>,
    plow: Vec<f32>,
    /// Action animation phase 0..1 (0 = none).
    pub act: Vec<f32>,
    act_len: Vec<f32>,
    hit_at: Vec<f32>,
    pub death: Vec<f32>,
    pub flash: Vec<f32>,
    pub walk: Vec<f32>,
    pub alt: Vec<f32>,
    reload: Vec<f32>,
    seek_t: Vec<f32>,
    last_knock: Vec<f32>,
    /// A fixed jitter of each soldier's slot.
    pub jx: Vec<f32>,
    pub jz: Vec<f32>,
    slot: Vec<usize>,
    /// A soldier's own way round something, and when to look for it again.
    detour: Vec<Vec<Point>>,
    detour_t: Vec<f32>,
    pub regiments: Vec<Regiment>,
    pub missiles: Vec<Missile>,
    pub events: Vec<Event>,
    pub over: Option<Over>,
    end_t: f32,
    gx0: f32,
    gz0: f32,
    gw: i32,
    gh: i32,
    head: Vec<i32>,
    next: Vec<i32>,
    step_count: u64,
}

impl Sim {
    pub fn new(field: Field, seed: u32, dimming: bool) -> Sim {
        let gw = (field.w / CELL).ceil() as i32 + 1;
        let gh = (field.d / CELL).ceil() as i32 + 1;
        Sim {
            gx0: field.x0,
            gz0: field.z0,
            gw,
            gh,
            head: vec![-1; (gw * gh) as usize],
            field,
            rand: Rng::new(seed.wrapping_mul(101).wrapping_add(7)),
            time: 0.0,
            dimming,
            n: 0,
            px: Vec::new(),
            pz: Vec::new(),
            py: Vec::new(),
            vx: Vec::new(),
            vz: Vec::new(),
            yaw: Vec::new(),
            hp: Vec::new(),
            reg: Vec::new(),
            state: Vec::new(),
            target: Vec::new(),
            cool: Vec::new(),
            stagger: Vec::new(),
            knock: Vec::new(),
            charge: Vec::new(),
            plow: Vec::new(),
            act: Vec::new(),
            act_len: Vec::new(),
            hit_at: Vec::new(),
            death: Vec::new(),
            flash: Vec::new(),
            walk: Vec::new(),
            alt: Vec::new(),
            reload: Vec::new(),
            seek_t: Vec::new(),
            last_knock: Vec::new(),
            jx: Vec::new(),
            jz: Vec::new(),
            slot: Vec::new(),
            detour: Vec::new(),
            detour_t: Vec::new(),
            regiments: Vec::new(),
            missiles: Vec::new(),
            events: Vec::new(),
            over: None,
            end_t: 0.0,
            next: Vec::new(),
            step_count: 0,
        }
    }

    fn r(&mut self) -> f32 {
        self.rand.next()
    }

    // -------------------------------------------------------------- set up
    /// A regiment of `type_id` for `side` (0 = the player, 1 = the enemy), deployed at (x, z).
    #[allow(clippy::too_many_arguments)]
    pub fn add_regiment(
        &mut self,
        type_id: &str,
        side: u8,
        men: Option<u32>,
        x: f32,
        z: f32,
        facing: Option<f32>,
        files: Option<u32>,
        look: Option<Faction>,
        id: Option<String>,
    ) -> usize {
        let ty = unit(type_id);
        let men = men.unwrap_or(ty.men).clamp(1, 400);
        let facing = facing.unwrap_or(if side == 0 { PI } else { 0.0 });
        let index = self.regiments.len();
        let morale = if ty.morale >= 999.0 { 100.0 } else { ty.morale };
        let ammo = ty.ranged.map_or(0, |r| r.ammo * men);
        let files = files.unwrap_or(ty.files).min(men);
        self.regiments.push(Regiment {
            index,
            id: id.unwrap_or_else(|| format!("{type_id}-{index}")),
            ty,
            side,
            look: look.unwrap_or(ty.faction),
            soldiers: Vec::with_capacity(men as usize),
            men0: men,
            alive: men,
            cx: x,
            cz: z,
            ax: x,
            az: z,
            heading: facing,
            files,
            order: Order::Hold { files },
            state: RState::Ready,
            morale,
            morale_base: morale,
            cohesion: 100.0,
            ammo,
            ammo0: ammo,
            rout_t: 0.0,
            rout_count: 0,
            flank_t: -99.0,
            rear_t: -99.0,
            losses: [0.0; 20],
            loss_i: 0,
            lost: 0,
            kills: 0,
            fled: 0,
            engaged: 0,
            idle_t: 0.0,
            fire_target: None,
            skirmish: ty.role == Role::Skirmisher,
            fire_at_will: true,
            run: false,
            speed_now: 0.0,
            lag: 0.0,
            wavering: false,
            dimmed: false,
            ability: (ty.id == "reina").then_some(Ability {
                cd: 0.0,
                cd_max: 45.0,
                radius: 32.0,
            }),
            awake: false,
            delay: 0.0,
            pending: None,
            charge_shock: 0.0,
            last_fire: -9.0,
            selected: false,
            firing: false,
            charging: false,
            diving: false,
            skirmish_cd: 0.0,
            flee_from: None,
            flee_t: 0.0,
            flee_to: None,
            crumble_told: false,
        });
        for k in 0..men as usize {
            let i = self.n;
            self.n += 1;
            let (jx, jz) = ((self.r() - 0.5) * 0.36, (self.r() - 0.5) * 0.36);
            let walk = self.r() * TAU;
            let reload = self.r() * ty.ranged.map_or(1.0, |r| r.reload);
            let cool = self.r();
            let seek = self.r() * 0.35;
            self.px.push(0.0);
            self.pz.push(0.0);
            self.py.push(0.0);
            self.vx.push(0.0);
            self.vz.push(0.0);
            self.yaw.push(facing);
            self.hp.push(ty.hp);
            self.reg.push(index);
            self.state.push(S_ALIVE);
            self.target.push(-1);
            self.cool.push(cool);
            self.stagger.push(0.0);
            self.knock.push(0.0);
            self.charge.push(0.0);
            self.plow.push(0.0);
            self.act.push(0.0);
            self.act_len.push(0.6);
            self.hit_at.push(-1.0);
            self.death.push(0.0);
            self.flash.push(0.0);
            self.walk.push(walk);
            self.alt.push(if ty.flyer { 6.0 } else { 0.0 });
            self.reload.push(reload);
            self.seek_t.push(seek);
            self.last_knock.push(-9.0);
            self.jx.push(jx);
            self.jz.push(jz);
            self.slot.push(k);
            self.detour.push(Vec::new());
            self.detour_t.push(0.0);
            self.next.push(-1);
            self.regiments[index].soldiers.push(i);
        }
        for k in 0..men as usize {
            let i = self.regiments[index].soldiers[k];
            let (sx, sz) = self.slot_pos(index, k, x, z, facing, files);
            self.px[i] = sx;
            self.pz[i] = sz;
            self.py[i] = self.field.walk_height(sx, sz) + self.alt[i];
        }
        self.reindex(index);
        index
    }

    /// Where soldier number k of the formation stands for an anchor and heading.
    pub fn slot_pos(
        &self,
        ri: usize,
        k: usize,
        ax: f32,
        az: f32,
        heading: f32,
        files: u32,
    ) -> (f32, f32) {
        let r = &self.regiments[ri];
        let t = r.ty;
        let alive = r.alive.max(1);
        let f = files.min(alive).max(1) as usize;
        let alive = alive as usize;
        let rows = alive.div_ceil(f);
        let (row, col) = (k / f, k % f);
        let in_row = if row == rows - 1 { alive - row * f } else { f };
        let lat = (col as f32 - (in_row as f32 - 1.0) / 2.0) * t.spacing;
        let dep = ((rows as f32 - 1.0) / 2.0 - row as f32) * t.row_gap;
        let (fx, fz) = (heading.sin(), heading.cos());
        let (rx, rz) = (-fz, fx);
        let (jx, jz) = match r.soldiers.get(k) {
            Some(&i) => (self.jx[i], self.jz[i]),
            None => (0.0, 0.0),
        };
        (ax + rx * lat + fx * dep + jx, az + rz * lat + fz * dep + jz)
    }

    /// The formation's frontage in metres for `files`.
    pub fn width(&self, ri: usize, files: u32) -> f32 {
        let r = &self.regiments[ri];
        (files.min(r.alive) as f32 - 1.0).max(1.0) * r.ty.spacing
    }

    fn reindex(&mut self, ri: usize) {
        for (k, &i) in self.regiments[ri].soldiers.iter().enumerate() {
            self.slot[i] = k;
        }
    }

    /// Re-sorts who stands where for a new heading, so nobody crosses the block to reach a slot.
    pub fn reassign(&mut self, ri: usize, heading: f32, files: u32) {
        let (cx, cz) = (self.regiments[ri].cx, self.regiments[ri].cz);
        let n = self.regiments[ri].soldiers.len();
        if n < 2 {
            self.reindex(ri);
            return;
        }
        let (fx, fz) = (heading.sin(), heading.cos());
        let (rx, rz) = (-fz, fx);
        let f = files.min(n as u32).max(1) as usize;
        let mut items: Vec<(usize, f32, f32)> = self.regiments[ri]
            .soldiers
            .iter()
            .map(|&i| {
                let (dx, dz) = (self.px[i] - cx, self.pz[i] - cz);
                (i, dx * fx + dz * fz, dx * rx + dz * rz)
            })
            .collect();
        items.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut out = Vec::with_capacity(n);
        for row in items.chunks_mut(f) {
            row.sort_by(|a, b| a.2.total_cmp(&b.2));
            out.extend(row.iter().map(|it| it.0));
        }
        self.regiments[ri].soldiers = out;
        self.reindex(ri);
    }

    // ------------------------------------------------------------------ orders
    pub fn can_command(&self, ri: usize) -> bool {
        let r = &self.regiments[ri];
        r.alive > 0
            && !matches!(
                r.state,
                RState::Routing | RState::Fled | RState::Dead | RState::Shattered
            )
            && !r.dimmed
            && !r.awake
    }

    /// Move: the formation's centre to (x, z), facing `heading` (radians, 0 = +z), `files` wide.
    #[allow(clippy::too_many_arguments)]
    pub fn order_move(
        &mut self,
        ri: usize,
        x: f32,
        z: f32,
        heading: Option<f32>,
        files: Option<u32>,
        run: bool,
        force: bool,
    ) -> bool {
        if !force && !self.can_command(ri) {
            return false;
        }
        let (rcx, rcz, rheading, rfiles, alive, flyer, spark) = {
            let r = &self.regiments[ri];
            (
                r.cx, r.cz, r.heading, r.files, r.alive, r.ty.flyer, r.ty.spark,
            )
        };
        let (dx, dz) = (x - rcx, z - rcz);
        let heading = heading.unwrap_or(if dx.hypot(dz) > 4.0 {
            dx.atan2(dz)
        } else {
            rheading
        });
        let files = files.unwrap_or(rfiles).clamp(1, alive.max(1));
        if !force && self.regiments[ri].cohesion < 25.0 && self.dimming && spark {
            // the dimming: orders arrive late, and some are not obeyed at all
            let delay = 1.6 + self.r() * 1.6;
            let r = &mut self.regiments[ri];
            r.pending = Some(Pending::Move {
                x,
                z,
                heading,
                files,
                run,
            });
            r.delay = delay;
            return true;
        }
        let path = if flyer {
            vec![Point { x, z }]
        } else {
            self.field.route(rcx, rcz, x, z)
        };
        let first = path[0];
        let travel = if (first.x - rcx).hypot(first.z - rcz) > 6.0 {
            (first.x - rcx).atan2(first.z - rcz)
        } else {
            heading
        };
        {
            let r = &mut self.regiments[ri];
            r.order = Order::Move {
                path,
                i: 0,
                heading,
                files,
                run,
            };
            r.run = run;
            r.files = files;
            r.state = RState::Moving;
            r.ax = r.cx;
            r.az = r.cz;
            r.heading = travel;
        }
        self.reassign(ri, travel, files);
        let soldiers = self.regiments[ri].soldiers.clone();
        for i in soldiers {
            if !flyer || self.alt[i] > 2.0 {
                self.target[i] = -1;
            }
        }
        true
    }

    /// Attack a regiment (melee units close in and charge; missile units shoot at it).
    pub fn order_attack(&mut self, ri: usize, target: usize, run: bool, force: bool) -> bool {
        if !force && !self.can_command(ri) {
            return false;
        }
        let Some(t) = self.regiments.get(target) else {
            return false;
        };
        if t.alive == 0 || t.side == self.regiments[ri].side {
            return false;
        }
        if !force
            && self.regiments[ri].cohesion < 25.0
            && self.dimming
            && self.regiments[ri].ty.spark
        {
            let delay = 1.6 + self.r() * 1.6;
            let r = &mut self.regiments[ri];
            r.pending = Some(Pending::Attack { target, run });
            r.delay = delay;
            return true;
        }
        let r = &mut self.regiments[ri];
        r.order = Order::Attack {
            target,
            run,
            files: r.files,
            charged: None,
            path_t: 0.0,
            route: None,
        };
        r.state = RState::Moving;
        r.fire_target = Some(target);
        r.ax = r.cx;
        r.az = r.cz;
        true
    }

    pub fn order_halt(&mut self, ri: usize) -> bool {
        if !self.can_command(ri) {
            return false;
        }
        let r = &mut self.regiments[ri];
        r.order = Order::Hold { files: r.files };
        r.ax = r.cx;
        r.az = r.cz;
        r.state = if r.engaged > 0 {
            RState::Melee
        } else {
            RState::Ready
        };
        true
    }

    /// The Queen's «Chispa»: rallies and rekindles her own, staggers the enemy round her.
    pub fn cast_spark(&mut self, ri: usize) -> bool {
        let Some(ability) = self.regiments[ri].ability else {
            return false;
        };
        if ability.cd > 0.0 || self.regiments[ri].alive == 0 || self.regiments[ri].awake {
            return false;
        }
        let q = self.regiments[ri].soldiers[0];
        let (x, z) = (self.px[q], self.pz[q]);
        let big_r = ability.radius;
        let side = self.regiments[ri].side;
        if let Some(a) = self.regiments[ri].ability.as_mut() {
            a.cd = a.cd_max;
        }
        self.act[q] = 0.001;
        self.act_len[q] = 1.4;
        self.hit_at[q] = -1.0;
        self.events.push(Event::Spark {
            x,
            z,
            r: big_r,
            side,
        });
        for oi in 0..self.regiments.len() {
            let (alive, state, oside, ocx, ocz) = {
                let o = &self.regiments[oi];
                (o.alive, o.state, o.side, o.cx, o.cz)
            };
            if alive == 0 || state == RState::Fled {
                continue;
            }
            let d = (ocx - x).hypot(ocz - z);
            if oside == side {
                if d > big_r + 14.0 {
                    continue;
                }
                let rally = {
                    let o = &mut self.regiments[oi];
                    o.morale = (o.morale + 35.0).min(100.0);
                    o.cohesion = 100.0;
                    o.dimmed = false;
                    o.state == RState::Routing && o.rout_count < 3
                };
                if rally {
                    self.rally(oi, true);
                }
            } else {
                let mut hit = 0;
                let soldiers = self.regiments[oi].soldiers.clone();
                let hero = self.regiments[oi].ty.role == Role::Hero;
                for i in soldiers {
                    let dd = (self.px[i] - x).hypot(self.pz[i] - z);
                    if dd > big_r {
                        continue;
                    }
                    let k = 1.0 - dd / big_r;
                    self.stagger[i] = self.stagger[i].max(1.0 + 1.6 * k);
                    self.knock[i] = 1.0;
                    let push = 5.0 * k + 1.5;
                    let l = if dd > 0.0 { dd } else { 1.0 };
                    self.vx[i] += (self.px[i] - x) / l * push;
                    self.vz[i] += (self.pz[i] - z) / l * push;
                    self.target[i] = -1;
                    hit += 1;
                    if !hero {
                        self.damage(i, 2.0 + 6.0 * k, Some(ri));
                    }
                }
                if hit > 0 {
                    self.regiments[oi].morale -= 18.0;
                    // Varrak, weak enough: the spark reaches the general he was
                    let o = &self.regiments[oi];
                    if o.ty.id == "varrak" && !o.awake {
                        if let Some(&v) = o.soldiers.first() {
                            if self.hp[v] < o.ty.hp * 0.45 {
                                self.awaken(oi);
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn awaken(&mut self, oi: usize) {
        let (v, side) = {
            let o = &mut self.regiments[oi];
            o.awake = true;
            o.state = RState::Awake;
            o.order = Order::Hold { files: o.files };
            (o.soldiers[0], o.side)
        };
        self.target[v] = -1;
        self.events.push(Event::Awaken {
            x: self.px[v],
            z: self.pz[v],
            reg: oi,
        });
        // his pack loses the one they followed
        for p in self.regiments.iter_mut() {
            if p.side == side && p.ty.follows_leader {
                p.morale = p.morale.min(12.0);
            }
        }
    }

    // --------------------------------------------------------------- the grid
    fn build_grid(&mut self) {
        self.head.fill(-1);
        for i in 0..self.n {
            if self.state[i] != S_ALIVE {
                continue;
            }
            let cx = ((self.px[i] - self.gx0) / CELL).floor() as i32;
            let cz = ((self.pz[i] - self.gz0) / CELL).floor() as i32;
            if cx < 0 || cz < 0 || cx >= self.gw || cz >= self.gh {
                self.next[i] = -1;
                continue;
            }
            let c = (cz * self.gw + cx) as usize;
            self.next[i] = self.head[c];
            self.head[c] = i as i32;
        }
    }

    /// The nearest living enemy of soldier i within `radius` (optionally of one regiment first).
    fn nearest_enemy(
        &self,
        i: usize,
        radius: f32,
        prefer: Option<usize>,
        ground_only: bool,
    ) -> i32 {
        let (x, z) = (self.px[i], self.pz[i]);
        let side = self.regiments[self.reg[i]].side;
        let c0x = (((x - radius - self.gx0) / CELL).floor() as i32).max(0);
        let c1x = (((x + radius - self.gx0) / CELL).floor() as i32).min(self.gw - 1);
        let c0z = (((z - radius - self.gz0) / CELL).floor() as i32).max(0);
        let c1z = (((z + radius - self.gz0) / CELL).floor() as i32).min(self.gh - 1);
        let (mut best, mut best_d, mut best_p, mut best_pd) =
            (-1i32, radius * radius, -1i32, radius * radius);
        for cz in c0z..=c1z {
            for cx in c0x..=c1x {
                let mut j = self.head[(cz * self.gw + cx) as usize];
                while j >= 0 {
                    let ju = j as usize;
                    let rj = &self.regiments[self.reg[ju]];
                    let next = self.next[ju];
                    if rj.side == side || rj.awake || (ground_only && self.alt[ju] > 2.6) {
                        j = next;
                        continue;
                    }
                    let (dx, dz) = (self.px[ju] - x, self.pz[ju] - z);
                    let d2 = dx * dx + dz * dz;
                    let preferred = prefer == Some(rj.index);
                    if d2 >= best_d && !(preferred && d2 < best_pd) {
                        j = next;
                        continue;
                    }
                    // walkers only go for what they can reach in a straight line
                    if ground_only
                        && d2 > 4.0
                        && !self.field.line_clear(x, z, self.px[ju], self.pz[ju])
                    {
                        j = next;
                        continue;
                    }
                    if d2 < best_d {
                        best_d = d2;
                        best = j;
                    }
                    if preferred && d2 < best_pd {
                        best_pd = d2;
                        best_p = j;
                    }
                    j = next;
                }
            }
        }
        // a preferred enemy wins unless another is much closer (someone is already on us)
        if best_p >= 0 && (best < 0 || best_d > 6.25 || best_pd < best_d * 2.5) {
            return best_p;
        }
        best
    }

    // ------------------------------------------------------------------ damage
    fn damage(&mut self, i: usize, amount: f32, by: Option<usize>) -> bool {
        if self.state[i] != S_ALIVE {
            return false;
        }
        self.hp[i] -= amount;
        self.flash[i] = 1.0;
        if self.hp[i] > 0.0 {
            return false;
        }
        self.kill(i, by);
        true
    }

    fn kill(&mut self, i: usize, by: Option<usize>) {
        let ri = self.reg[i];
        self.state[i] = S_DEAD;
        self.death[i] = 0.001;
        self.target[i] = -1;
        self.vx[i] *= 0.3;
        self.vz[i] *= 0.3;
        {
            let r = &mut self.regiments[ri];
            r.alive = r.alive.saturating_sub(1);
            r.lost += 1;
            r.losses[r.loss_i] += 1.0;
        }
        if let Some(b) = by {
            self.regiments[b].kills += 1;
        }
        if let Some(k) = self.regiments[ri].soldiers.iter().position(|&s| s == i) {
            self.regiments[ri].soldiers.remove(k);
            self.reindex(ri);
        }
        let r = &self.regiments[ri];
        self.events.push(Event::Death {
            x: self.px[i],
            z: self.pz[i],
            side: r.side,
            large: r.ty.large,
            reg: ri,
        });
        if r.ty.role == Role::Hero {
            self.hero_down(ri);
        }
    }

    fn hero_down(&mut self, ri: usize) {
        let side = self.regiments[ri].side;
        if self.regiments[ri].ty.id == "reina" {
            self.events.push(Event::QueenDown { reg: ri });
            for (oi, o) in self.regiments.iter_mut().enumerate() {
                if o.side == side && oi != ri {
                    o.morale -= 30.0;
                }
            }
        } else {
            self.events.push(Event::HeroDown { reg: ri });
            for o in self.regiments.iter_mut() {
                if o.side == side && o.ty.follows_leader {
                    o.morale = o.morale.min(10.0);
                }
            }
        }
    }

    /// One blow from soldier a at soldier b.
    fn strike(&mut self, a: usize, b: usize, charge: f32) {
        let (ra, rb) = (self.reg[a], self.reg[b]);
        let (ta, tb) = (self.regiments[ra].ty, self.regiments[rb].ty);
        if self.state[b] != S_ALIVE {
            return;
        }
        // where the blow comes from, for the one struck (its own facing) and for its regiment
        let (dx, dz) = (self.px[a] - self.px[b], self.pz[a] - self.pz[b]);
        let d = dx.hypot(dz);
        let d = if d > 0.0 { d } else { 1.0 };
        let fb = self.yaw[b].sin() * (dx / d) + self.yaw[b].cos() * (dz / d);
        let rbh = self.regiments[rb].heading;
        let fr = rbh.sin() * (dx / d) + rbh.cos() * (dz / d);
        {
            let r = &mut self.regiments[rb];
            if r.alive > 1 && matches!(r.state, RState::Melee | RState::Ready | RState::Moving) {
                if fr < -0.5 {
                    r.rear_t = self.time;
                } else if fr < 0.3 {
                    r.flank_t = self.time;
                }
            }
        }
        let mut atk = ta.atk + charge * 0.6;
        if tb.large {
            atk += ta.anti_large;
        }
        let mut def = tb.def;
        if fb > 0.35 {
            def += tb.shield * 0.1;
        } else if fb > -0.5 {
            def *= 0.6;
        } else {
            def *= 0.3;
        }
        atk *= self.fight_mod(ra, true);
        def *= self.fight_mod(rb, false);
        if self.stagger[b] > 0.0 {
            def *= 0.5;
        }
        // high ground
        let dh = self.py[a] - self.py[b];
        atk += clamp(dh * 0.8, -3.0, 3.0);
        let chance = clamp(0.45 + (atk - def) * 0.022, 0.08, 0.93);
        if self.r() > chance {
            self.events.push(Event::Clash {
                x: self.px[b],
                z: self.pz[b],
            });
            return;
        }
        let mut dmg = ta.dmg * (1.0 + charge / 45.0) * (0.8 + self.r() * 0.4);
        if tb.large {
            dmg += ta.anti_large * 0.35;
        }
        let dealt = dmg * (1.0 - tb.armor / 100.0) + ta.ap;
        self.events.push(Event::Hit {
            x: self.px[b],
            z: self.pz[b],
            big: charge > 0.0 || ta.large,
        });
        self.damage(b, dealt, Some(ra));
    }

    /// Morale and the spark change how well a regiment fights.
    fn fight_mod(&self, ri: usize, attacking: bool) -> f32 {
        let r = &self.regiments[ri];
        let mut k = 1.0;
        if r.wavering {
            k *= 0.85;
        }
        if r.ty.spark && self.dimming {
            if r.dimmed {
                k *= if attacking { 1.2 } else { 0.7 };
            } else if r.cohesion < 50.0 {
                k *= 0.9;
            }
        }
        k
    }

    // --------------------------------------------------------------- the step
    pub fn step(&mut self) {
        let dt = DT;
        self.time += dt;
        self.step_count += 1;
        self.build_grid();
        for ri in 0..self.regiments.len() {
            self.update_regiment(ri, dt);
        }
        self.update_soldiers(dt);
        self.update_missiles(dt);
        if self.step_count % 15 == 0 {
            for ri in 0..self.regiments.len() {
                self.update_morale(ri);
            }
            self.check_end();
        }
    }

    fn centroid(&mut self, ri: usize) {
        let (mut sx, mut sz, mut sp, mut n) = (0.0, 0.0, 0.0, 0usize);
        for &i in &self.regiments[ri].soldiers {
            sx += self.px[i];
            sz += self.pz[i];
            sp += self.vx[i].hypot(self.vz[i]);
            n += 1;
        }
        if n > 0 {
            let (mut cx, mut cz) = (sx / n as f32, sz / n as f32);
            // A few left far behind (caught on a wall, round a rock) do not hold the block back:
            // the centre is where most of it stands.
            if n >= 4 {
                let (ax, az) = (self.regiments[ri].ax, self.regiments[ri].az);
                let (mut nx, mut nz, mut m) = (0.0, 0.0, 0usize);
                for &i in &self.regiments[ri].soldiers {
                    if (self.px[i] - ax).hypot(self.pz[i] - az) < 30.0 {
                        nx += self.px[i];
                        nz += self.pz[i];
                        m += 1;
                    }
                }
                if m * 10 >= n * 7 && m < n {
                    cx = nx / m as f32;
                    cz = nz / m as f32;
                }
            }
            let r = &mut self.regiments[ri];
            r.cx = cx;
            r.cz = cz;
            r.speed_now = sp / n as f32;
        }
    }

    fn update_regiment(&mut self, ri: usize, dt: f32) {
        self.centroid(ri);
        if self.regiments[ri].alive == 0 {
            let r = &mut self.regiments[ri];
            if !matches!(r.state, RState::Fled | RState::Dead) {
                r.state = if r.fled > 0 && r.fled >= r.men0 - r.lost {
                    RState::Fled
                } else {
                    RState::Dead
                };
            }
            return;
        }
        if let Some(a) = self.regiments[ri].ability.as_mut() {
            if a.cd > 0.0 {
                a.cd = (a.cd - dt).max(0.0);
            }
        }
        // delayed orders (a dimming regiment)
        if let Some(o) = self.regiments[ri].pending {
            self.regiments[ri].delay -= dt;
            if self.regiments[ri].delay <= 0.0 {
                self.regiments[ri].pending = None;
                if self.r() < 0.25 {
                    if let Some(foe) = self.nearest_regiment(ri, 220.0, |_| true) {
                        self.order_attack(ri, foe, false, true);
                    }
                } else {
                    match o {
                        Pending::Move {
                            x,
                            z,
                            heading,
                            files,
                            run,
                        } => {
                            self.order_move(ri, x, z, Some(heading), Some(files), run, true);
                        }
                        Pending::Attack { target, run } => {
                            self.order_attack(ri, target, run, true);
                        }
                    }
                }
            }
        }
        // the spark: the Queen's units keep their minds near her
        if self.regiments[ri].ty.spark && self.dimming {
            self.update_spark(ri, dt);
        }
        if self.regiments[ri].awake || self.regiments[ri].state == RState::Awake {
            return;
        }
        if self.regiments[ri].state.routed() {
            self.regiments[ri].rout_t += dt;
            let foe = self.nearest_regiment(ri, 60.0, |_| true);
            self.regiments[ri].flee_from = foe;
            if self.regiments[ri].flee_t <= self.time {
                self.regiments[ri].flee_t = self.time + 1.0;
                let (side, cx, cz, flyer) = {
                    let r = &self.regiments[ri];
                    (r.side, r.cx, r.cz, r.ty.flyer)
                };
                let edge_z = if side == 0 {
                    self.field.play.z1 + 30.0
                } else {
                    self.field.play.z0 - 30.0
                };
                self.regiments[ri].flee_to = None;
                let safe_z = edge_z - edge_z.signum() * 36.0;
                if !flyer && !self.field.line_clear(cx, cz, cx, safe_z) {
                    let way = self.field.find_path(cx, cz, cx, safe_z);
                    if way.len() > 1 {
                        self.regiments[ri].flee_to =
                            Some(if (way[0].x - cx).hypot(way[0].z - cz) < 8.0 {
                                way[1]
                            } else {
                                way[0]
                            });
                    }
                }
            }
            return;
        }
        // engagement
        let fighting = self.regiments[ri]
            .soldiers
            .iter()
            .filter(|&&i| self.target[i] >= 0)
            .count() as u32;
        self.regiments[ri].engaged = fighting;
        // missile units: who to shoot
        if self.regiments[ri].ty.ranged.is_some() {
            self.update_fire(ri);
        }
        let (alive, flyer) = (self.regiments[ri].alive, self.regiments[ri].ty.flyer);
        match self.regiments[ri].order.clone() {
            Order::Move { .. } => {
                self.advance_anchor(ri, dt);
                if fighting as f32 > alive as f32 * 0.3 && !flyer {
                    // caught on the way: the block stops and fights
                    let r = &mut self.regiments[ri];
                    r.order = Order::Hold { files: r.files };
                    r.ax = r.cx;
                    r.az = r.cz;
                    r.state = RState::Melee;
                }
            }
            Order::Attack {
                target,
                run,
                charged,
                path_t,
                route,
                ..
            } => self.update_attack(ri, dt, target, run, charged, path_t, route, fighting),
            _ => {
                // holding: reform where we stand once the fighting is over
                if fighting > 0 {
                    let r = &mut self.regiments[ri];
                    r.state = RState::Melee;
                    r.idle_t = 0.0;
                } else {
                    self.regiments[ri].idle_t += dt;
                    let (state, idle) = (self.regiments[ri].state, self.regiments[ri].idle_t);
                    if state == RState::Melee && idle > 2.5 {
                        let (heading, files) = {
                            let r = &mut self.regiments[ri];
                            r.ax = r.cx;
                            r.az = r.cz;
                            r.state = RState::Ready;
                            (r.heading, r.files)
                        };
                        self.reassign(ri, heading, files);
                    } else if state != RState::Melee {
                        let r = &mut self.regiments[ri];
                        r.state = if r.firing {
                            RState::Firing
                        } else {
                            RState::Ready
                        };
                    }
                }
                if self.regiments[ri].state == RState::Firing {
                    if let Some(t) = self.regiments[ri].fire_target {
                        let (tcx, tcz) = (self.regiments[t].cx, self.regiments[t].cz);
                        let r = &mut self.regiments[ri];
                        r.heading = turn_to(r.heading, (tcx - r.cx).atan2(tcz - r.cz), dt * 0.8);
                    }
                }
            }
        }
        // skirmishers keep away from melee troops
        if self.regiments[ri].skirmish_cd > 0.0 {
            self.regiments[ri].skirmish_cd -= dt;
        }
        let r = &self.regiments[ri];
        if r.skirmish && r.ammo > 0 && fighting == 0 && !r.is_moving_order() && r.skirmish_cd <= 0.0
        {
            self.regiments[ri].skirmish_cd = 2.5;
            let foe = self.nearest_regiment(ri, 30.0, |q| {
                q.ty.ranged.is_none() || q.ty.role == Role::Cavalry
            });
            if let Some(foe) = foe {
                let (rcx, rcz) = (self.regiments[ri].cx, self.regiments[ri].cz);
                let (dx, dz) = (rcx - self.regiments[foe].cx, rcz - self.regiments[foe].cz);
                let d = dx.hypot(dz);
                let d = if d > 0.0 { d } else { 1.0 };
                let (tx, tz) = (rcx + dx / d * 34.0, rcz + dz / d * 34.0);
                if !self.field.blocked(tx, tz) {
                    let files = self.regiments[ri].files;
                    self.order_move(ri, tx, tz, Some((-dx).atan2(-dz)), Some(files), true, true);
                }
            }
        }
        // a flying regiment swoops down only to fight
        if self.regiments[ri].ty.flyer {
            let diving = match self.regiments[ri].order_target() {
                Some(t) => {
                    let o = &self.regiments[t];
                    (o.cx - self.regiments[ri].cx).hypot(o.cz - self.regiments[ri].cz) < 26.0
                }
                None => false,
            } || fighting > 0;
            self.regiments[ri].diving = diving;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_attack(
        &mut self,
        ri: usize,
        dt: f32,
        target: usize,
        run: bool,
        charged: Option<f32>,
        path_t: f32,
        route: Option<Vec<Point>>,
        fighting: u32,
    ) {
        let t_gone = {
            let t = &self.regiments[target];
            t.alive == 0 || t.state == RState::Fled || t.awake
        };
        if t_gone {
            let r = &mut self.regiments[ri];
            r.order = Order::Hold { files: r.files };
            r.ax = r.cx;
            r.az = r.cz;
            r.state = if fighting > 0 {
                RState::Melee
            } else {
                RState::Ready
            };
            return;
        }
        let (tcx, tcz) = (self.regiments[target].cx, self.regiments[target].cz);
        let (ty, rcx, rcz, ammo, alive) = {
            let r = &self.regiments[ri];
            (r.ty, r.cx, r.cz, r.ammo, r.alive)
        };
        if let (Some(rg), true) = (ty.ranged, ammo > 0) {
            // walk into range, then stand and shoot
            let d = (tcx - rcx).hypot(tcz - rcz);
            let range = rg.range * 0.92;
            if d > range {
                let k = (d - range * 0.9) / d;
                self.steer_anchor_to(
                    ri,
                    rcx + (tcx - rcx) * k,
                    rcz + (tcz - rcz) * k,
                    dt,
                    false,
                    false,
                );
                self.regiments[ri].state = RState::Moving;
            } else {
                let r = &mut self.regiments[ri];
                r.heading = turn_to(r.heading, (tcx - rcx).atan2(tcz - rcz), dt * 1.2);
                r.state = if fighting > 0 {
                    RState::Melee
                } else {
                    RState::Firing
                };
            }
            return;
        }
        // close in (round the river, through a crossing); charge in the last stretch
        let mut d = (tcx - rcx).hypot(tcz - rcz);
        let (mut gx, mut gz, mut across) = (tcx, tcz, false);
        let mut route = route;
        let mut path_t = path_t;
        let mut files = self.regiments[ri].order_files();
        if !ty.flyer {
            // round whatever is in the way: an A* path, renewed now and then
            let (ax, az) = (self.regiments[ri].ax, self.regiments[ri].az);
            if path_t <= self.time {
                path_t = self.time + 1.2;
                route = if self.field.line_clear(ax, az, tcx, tcz) {
                    None
                } else {
                    Some(self.field.find_path(ax, az, tcx, tcz))
                };
            }
            if let Some(rt) = route.as_mut() {
                if rt.len() > 1 {
                    across = true;
                    if (rt[0].x - ax).hypot(rt[0].z - az) < 6.0 {
                        rt.remove(0);
                    }
                    gx = rt[0].x;
                    gz = rt[0].z;
                    d += 30.0;
                }
            }
            files = self.narrow_files(ri, Some(Point { x: gx, z: gz }));
        }
        let charge_d = match ty.role {
            Role::Cavalry | Role::Beast => 55.0,
            _ if ty.flyer => 40.0,
            _ => 32.0,
        };
        let charging = !across && d < charge_d && (fighting as f32) < alive as f32 * 0.2;
        let mut charged = charged;
        // one charge per attack order (cavalry pull out and order a new one to charge again)
        if charging && charged.is_none() {
            charged = Some(self.time);
            let (soldiers, large, cx, cz) = {
                let r = &self.regiments[ri];
                (r.soldiers.clone(), r.ty.large, r.cx, r.cz)
            };
            for i in soldiers {
                self.charge[i] = if large { 3.2 } else { 2.6 };
            }
            self.events.push(Event::Charge {
                reg: ri,
                x: cx,
                z: cz,
            });
        }
        {
            let time = self.time;
            let r = &mut self.regiments[ri];
            r.charging = charged.is_some_and(|c| time - c < 3.5);
            r.order = Order::Attack {
                target,
                run,
                files,
                charged,
                path_t,
                route,
            };
            r.run = run || charging;
            r.state = if fighting > 0 {
                RState::Melee
            } else {
                RState::Moving
            };
        }
        // aim the block's front at the enemy's centre
        self.steer_anchor_to(ri, gx, gz, dt, !across, !across && d < 26.0);
    }

    /// Moves the formation's anchor along its path at the regiment's pace (waiting for laggards).
    fn advance_anchor(&mut self, ri: usize, dt: f32) {
        let (wp, last, run, want_heading) = match &self.regiments[ri].order {
            Order::Move {
                path,
                i,
                run,
                heading,
                ..
            } => match path.get(*i) {
                Some(&wp) => (wp, *i == path.len() - 1, *run, *heading),
                None => return,
            },
            _ => return,
        };
        let (rax, raz) = (self.regiments[ri].ax, self.regiments[ri].az);
        let (dx, dz) = (wp.x - rax, wp.z - raz);
        let d = dx.hypot(dz);
        let pace = self.pace(ri, run);
        // narrow into a column where the way is tight (a ford, a bridge, a gate, a gap in the rock)
        let narrow = self.narrow_files(ri, Some(wp));
        if let Order::Move { files, .. } = &mut self.regiments[ri].order {
            *files = narrow;
        }
        if d < 0.6 + pace * dt {
            self.regiments[ri].ax = wp.x;
            self.regiments[ri].az = wp.z;
            if last {
                let (heading, files) = (self.regiments[ri].heading, self.regiments[ri].files);
                if ang_diff(heading, want_heading).abs() > 0.6 {
                    self.reassign(ri, want_heading, files);
                }
                let r = &mut self.regiments[ri];
                r.heading = want_heading;
                r.order = Order::Hold { files: r.files };
                r.state = RState::Ready;
                r.idle_t = 0.0;
                self.events.push(Event::Arrive { reg: ri });
            } else {
                let r = &mut self.regiments[ri];
                if let Order::Move { path, i, .. } = &mut r.order {
                    *i += 1;
                    let nxt = path[*i];
                    r.heading = (nxt.x - r.ax).atan2(nxt.z - r.az);
                }
            }
            return;
        }
        let r = &mut self.regiments[ri];
        let want = dx.atan2(dz);
        r.heading = turn_to(r.heading, want, dt * if r.ty.large { 1.2 } else { 1.8 });
        r.ax += dx / d * pace * dt;
        r.az += dz / d * pace * dt;
    }

    /// How many files fit where the anchor is going: the gap's clearance either side of it.
    fn narrow_files(&self, ri: usize, toward: Option<Point>) -> u32 {
        let r = &self.regiments[ri];
        if r.ty.flyer || r.alive < 4 {
            return r.files;
        }
        let f = &self.field;
        let mut c = f.clearance_at(r.ax, r.az);
        if let Some(t) = toward {
            let (dx, dz) = (t.x - r.ax, t.z - r.az);
            let l = dx.hypot(dz);
            let l = if l > 0.0 { l } else { 1.0 };
            for ahead in [6.0f32, 12.0] {
                c = c.min(
                    f.clearance_at(r.ax + dx / l * ahead.min(l), r.az + dz / l * ahead.min(l)),
                );
            }
        }
        let fit = ((2.0 * c - 3.0).max(0.0) / r.ty.spacing).floor() as u32 + 1;
        r.files.min(3).max(r.files.min(fit))
    }

    /// Attack approach: the anchor heads for (x, z) and stops short of it so the lines meet.
    fn steer_anchor_to(&mut self, ri: usize, x: f32, z: f32, dt: f32, melee: bool, close: bool) {
        let run = self.regiments[ri].run || close;
        let pace = self.pace(ri, run);
        let r = &mut self.regiments[ri];
        let (dx, dz) = (x - r.ax, z - r.az);
        let d = dx.hypot(dz);
        let d = if d > 0.0 { d } else { 1.0 };
        let want = dx.atan2(dz);
        r.heading = turn_to(r.heading, want, dt * if r.ty.large { 1.4 } else { 2.0 });
        let stop = if melee {
            if close {
                0.0
            } else {
                2.0
            }
        } else {
            0.0
        };
        if d > stop {
            let step = (d - stop).min(pace * dt);
            r.ax += dx / d * step;
            r.az += dz / d * step;
        }
        // keep the anchor from running away from the block
        let lag = (r.ax - r.cx).hypot(r.az - r.cz);
        if lag > 9.0 {
            r.ax = r.cx + (r.ax - r.cx) / lag * 9.0;
            r.az = r.cz + (r.az - r.cz) / lag * 9.0;
        }
    }

    /// How fast the block moves: its speed, slowed by ground and by soldiers out of place.
    fn pace(&mut self, ri: usize, run: bool) -> f32 {
        let (t, ax, az, heading, files) = {
            let r = &self.regiments[ri];
            (r.ty, r.ax, r.az, r.heading, r.order_files())
        };
        let mut v = if run { t.run * 0.92 } else { t.walk };
        if !t.flyer {
            v *= self.field.speed_at(ax, az);
        }
        // wait for soldiers who are out of place
        let (mut lag, mut n) = (0.0, 0);
        let len = self.regiments[ri].soldiers.len();
        let mut k = 0;
        while k < len {
            let i = self.regiments[ri].soldiers[k];
            if self.target[i] < 0 {
                let (sx, sz) = self.slot_pos(ri, k, ax, az, heading, files);
                lag += (sx - self.px[i]).hypot(sz - self.pz[i]);
                n += 1;
            }
            k += 3;
        }
        let lag = if n > 0 { lag / n as f32 } else { 0.0 };
        self.regiments[ri].lag = lag;
        if lag > 3.0 {
            v *= clamp(1.0 - (lag - 3.0) / 8.0, 0.25, 1.0);
        }
        v
    }

    pub fn nearest_regiment(
        &self,
        ri: usize,
        max_d: f32,
        filter: impl Fn(&Regiment) -> bool,
    ) -> Option<usize> {
        let r = &self.regiments[ri];
        let mut best = None;
        let mut bd = max_d;
        for o in &self.regiments {
            if o.side == r.side || o.alive == 0 || o.state == RState::Fled || o.awake || !filter(o)
            {
                continue;
            }
            let d = (o.cx - r.cx).hypot(o.cz - r.cz);
            if d < bd {
                bd = d;
                best = Some(o.index);
            }
        }
        best
    }

    fn update_fire(&mut self, ri: usize) {
        self.regiments[ri].firing = false;
        if self.regiments[ri].ammo == 0 {
            return;
        }
        let range = self.regiments[ri].ty.ranged.map_or(0.0, |r| r.range);
        let mut t = self.regiments[ri].order_target();
        if let Some(ti) = t {
            let o = &self.regiments[ti];
            let r = &self.regiments[ri];
            if o.alive == 0 || (o.cx - r.cx).hypot(o.cz - r.cz) > range + 10.0 {
                t = None;
            }
        }
        if t.is_none() && self.regiments[ri].fire_at_will && !self.regiments[ri].is_moving_order() {
            let side = self.regiments[ri].side;
            t = self
                .nearest_regiment(ri, range, |q| q.state != RState::Routing || side == 1)
                .or_else(|| self.nearest_regiment(ri, range, |_| true));
        }
        let r = &mut self.regiments[ri];
        r.fire_target = t;
        r.firing = t.is_some();
    }

    fn update_spark(&mut self, ri: usize, dt: f32) {
        let side = self.regiments[ri].side;
        let queen = self
            .regiments
            .iter()
            .find(|q| q.side == side && q.ty.id == "reina" && q.alive > 0)
            .map(|q| (q.soldiers[0], q.ty.aura.map_or(70.0, |a| a.radius)));
        let (cx, cz) = (self.regiments[ri].cx, self.regiments[ri].cz);
        let near =
            queen.is_some_and(|(q, radius)| (self.px[q] - cx).hypot(self.pz[q] - cz) < radius);
        let mut cohesion = self.regiments[ri].cohesion;
        if near {
            cohesion = (cohesion + 6.0 * dt).min(100.0);
        } else {
            cohesion -= if queen.is_some() { 0.55 } else { 1.6 } * dt;
        }
        // the dogma: priests nearby put the spark out faster
        for p in &self.regiments {
            if p.side == side || p.alive == 0 || p.state == RState::Routing {
                continue;
            }
            if let Some(a) = p.ty.aura {
                if a.kind == AuraKind::Dogma && (p.cx - cx).hypot(p.cz - cz) < a.radius {
                    cohesion -= 1.8 * dt;
                }
            }
        }
        let cohesion = clamp(cohesion, 0.0, 100.0);
        self.regiments[ri].cohesion = cohesion;
        if cohesion < 1.0 && !self.regiments[ri].dimmed {
            self.regiments[ri].dimmed = true;
            self.events.push(Event::Dimmed { reg: ri });
        }
        if self.regiments[ri].dimmed {
            if cohesion > 30.0 {
                self.regiments[ri].dimmed = false;
                self.events.push(Event::Rekindled { reg: ri });
            } else if self.regiments[ri].state != RState::Routing {
                let has_target = self.regiments[ri]
                    .order_target()
                    .is_some_and(|t| self.regiments[t].alive > 0);
                if !has_target {
                    // a beast again: it goes for whatever enemy is nearest
                    if let Some(foe) = self.nearest_regiment(ri, 400.0, |_| true) {
                        self.order_attack(ri, foe, false, true);
                    }
                }
            }
        }
    }

    fn rally(&mut self, ri: usize, instant: bool) {
        {
            let r = &mut self.regiments[ri];
            r.state = RState::Ready;
            r.order = Order::Hold { files: r.files };
            r.morale = r.morale.max(if instant { 60.0 } else { 45.0 });
            if !instant {
                r.rout_count += 1;
            }
            r.ax = r.cx;
            r.az = r.cz;
        }
        if let Some(foe) = self.nearest_regiment(ri, 500.0, |_| true) {
            let (fx, fz) = (self.regiments[foe].cx, self.regiments[foe].cz);
            let r = &mut self.regiments[ri];
            r.heading = (fx - r.cx).atan2(fz - r.cz);
        }
        let (h, f) = (self.regiments[ri].heading, self.regiments[ri].files);
        self.reassign(ri, h, f);
        for &i in &self.regiments[ri].soldiers {
            self.target[i] = -1;
        }
        self.events.push(Event::Rally { reg: ri });
    }

    fn update_morale(&mut self, ri: usize) {
        if self.regiments[ri].alive == 0 || self.regiments[ri].state == RState::Fled {
            return;
        }
        // losses in the last ten seconds
        let recent: f32 = {
            let r = &mut self.regiments[ri];
            let recent = r.losses.iter().sum();
            r.loss_i = (r.loss_i + 1) % r.losses.len();
            r.losses[r.loss_i] = 0.0;
            recent
        };
        let t = self.regiments[ri].ty;
        if self.regiments[ri].awake {
            return;
        }
        if t.morale >= 999.0 && !t.follows_leader {
            // the dead do not run: when their will is spent they crumble
            let crumble = {
                let r = &mut self.regiments[ri];
                r.morale = clamp(r.morale - recent / r.men0 as f32 * 30.0 + 2.0, 0.0, 100.0);
                r.morale < 15.0 && r.alive > 0
            };
            if crumble {
                let n = ((self.regiments[ri].men0 as f32 * 0.02).round() as usize).max(1);
                for _ in 0..n {
                    let Some(&last) = self.regiments[ri].soldiers.last() else {
                        break;
                    };
                    self.kill(last, None);
                }
                self.events.push(Event::Crumble { reg: ri });
            }
            return;
        }
        let side = self.regiments[ri].side;
        let (cx, cz) = (self.regiments[ri].cx, self.regiments[ri].cz);
        if t.follows_leader {
            let leader = self.regiments.iter().find(|q| {
                q.side == side
                    && q.ty.aura.is_some_and(|a| a.kind == AuraKind::Leader)
                    && q.alive > 0
                    && !q.awake
            });
            if leader.is_some_and(|l| (l.cx - cx).hypot(l.cz - cz) < 400.0) {
                self.regiments[ri].morale = 90.0;
                return;
            }
        }
        let r = &self.regiments[ri];
        let mut target = if t.follows_leader {
            25.0
        } else {
            r.morale_base
        };
        let lost_frac = 1.0 - r.alive as f32 / r.men0 as f32;
        target -= 48.0 * lost_frac.powf(1.25);
        target -= recent / r.men0 as f32 * 140.0;
        if self.time - r.flank_t < 3.0 {
            target -= 10.0;
        }
        if self.time - r.rear_t < 3.0 {
            target -= 18.0;
        }
        let mut routing = 0;
        for o in &self.regiments {
            if o.index == ri || o.alive == 0 {
                continue;
            }
            let d = (o.cx - cx).hypot(o.cz - cz);
            if o.side == side {
                if o.state == RState::Routing && d < 55.0 {
                    routing += 1;
                }
                if let Some(a) = o.ty.aura {
                    if d < a.radius && !o.awake {
                        target += match a.kind {
                            AuraKind::Queen => 14.0,
                            AuraKind::Dogma => 10.0,
                            AuraKind::Leader => 12.0,
                        };
                    }
                }
            } else if o.ty.role == Role::Hero && d < 40.0 && !o.awake {
                target -= 6.0;
            }
        }
        target -= routing.min(3) as f32 * 6.0;
        if t.spark && self.dimming && r.cohesion < 30.0 {
            target -= 14.0;
        }
        // winning the fight cheers; standing on the hill too
        let foe = if r.engaged > 0 {
            self.nearest_regiment(ri, 30.0, |_| true)
        } else {
            None
        };
        if let Some(fi) = foe {
            let f = &self.regiments[fi];
            if f.morale < r.morale {
                target += 5.0;
            }
            if self.field.height_at(cx, cz) > self.field.height_at(f.cx, f.cz) + 2.5 {
                target += 5.0;
            }
        }
        let target = clamp(target, 0.0, 100.0);
        let rout = {
            let r = &mut self.regiments[ri];
            r.morale += (target - r.morale) * 0.16;
            if r.charge_shock > 0.0 {
                r.morale -= r.charge_shock;
                r.charge_shock = 0.0;
            }
            r.morale = clamp(r.morale, 0.0, 100.0);
            r.wavering = r.morale < 32.0 && r.state != RState::Routing;
            r.state.routed()
        };
        if rout {
            let enemy = self.nearest_regiment(ri, 45.0, |_| true);
            let rally = {
                let r = &mut self.regiments[ri];
                if r.state == RState::Routing && r.rout_t > 9.0 && enemy.is_none() {
                    r.morale += 6.0;
                    r.morale > 44.0 && r.rout_count < 2 && lost_frac < 0.8
                } else {
                    false
                }
            };
            if rally {
                self.rally(ri, false);
            }
            return;
        }
        if self.regiments[ri].morale < 16.0 {
            self.rout(ri, lost_frac);
        }
    }

    fn rout(&mut self, ri: usize, lost_frac: f32) {
        let shattered = {
            let r = &mut self.regiments[ri];
            r.rout_t = 0.0;
            r.state = if r.rout_count >= 2 || lost_frac > 0.8 {
                RState::Shattered
            } else {
                RState::Routing
            };
            r.order = Order::Flee;
            r.charging = false;
            r.state == RState::Shattered
        };
        for &i in &self.regiments[ri].soldiers {
            self.target[i] = -1;
        }
        self.events.push(if shattered {
            Event::Shatter { reg: ri }
        } else {
            Event::Rout { reg: ri }
        });
    }

    fn check_end(&mut self) {
        if self.over.is_some() {
            return;
        }
        for side in [0u8, 1] {
            let standing = self.regiments.iter().filter(|r| r.side == side).any(|r| {
                r.alive > 0
                    && !matches!(r.state, RState::Routing | RState::Shattered | RState::Fled)
                    && !r.awake
            });
            if !standing {
                self.end_t += 0.5;
                if self.end_t > 3.0 {
                    self.over = Some(Over {
                        winner: 1 - side,
                        time: self.time,
                    });
                    self.events.push(Event::End { winner: 1 - side });
                }
                return;
            }
        }
        self.end_t = 0.0;
    }

    // ---------------------------------------------------------- the soldiers
    fn update_soldiers(&mut self, dt: f32) {
        for i in 0..self.n {
            let st = self.state[i];
            if self.flash[i] > 0.0 {
                self.flash[i] = (self.flash[i] - dt * 4.0).max(0.0);
            }
            if st == S_DEAD {
                if self.death[i] < 1.0 {
                    self.death[i] = (self.death[i] + dt * 1.8).min(1.0);
                }
                // the dead fall from the sky too
                if self.alt[i] > 0.0 {
                    self.alt[i] = (self.alt[i] - dt * 9.0).max(0.0);
                    self.py[i] = self.field.walk_height(self.px[i], self.pz[i]) + self.alt[i];
                }
                if self.vx[i].abs() + self.vz[i].abs() > 0.01 {
                    self.px[i] += self.vx[i] * dt;
                    self.pz[i] += self.vz[i] * dt;
                    self.vx[i] *= 0.85;
                    self.vz[i] *= 0.85;
                    self.py[i] = self.field.walk_height(self.px[i], self.pz[i]) + self.alt[i];
                }
                continue;
            }
            if st != S_ALIVE {
                continue;
            }
            self.update_soldier(i, dt);
        }
    }

    fn update_soldier(&mut self, i: usize, dt: f32) {
        let ri = self.reg[i];
        let t = self.regiments[ri].ty;
        if self.act[i] > 0.0 {
            let prev = self.act[i];
            self.act[i] += dt / self.act_len[i];
            if self.hit_at[i] > 0.0 && prev < self.hit_at[i] && self.act[i] >= self.hit_at[i] {
                self.land(i);
            }
            if self.act[i] >= 1.0 {
                self.act[i] = 0.0;
            }
        }
        if self.state[i] != S_ALIVE {
            return;
        }
        if self.cool[i] > 0.0 {
            self.cool[i] -= dt;
        }
        if self.charge[i] > 0.0 {
            self.charge[i] -= dt;
        }
        if self.plow[i] > 0.0 {
            self.plow[i] -= dt;
        }
        if self.reload[i] > 0.0 {
            self.reload[i] -= dt;
        }
        if self.knock[i] > 0.0 {
            self.knock[i] = (self.knock[i] - dt * 1.6).max(0.0);
        }
        let staggered = self.stagger[i] > 0.0;
        if staggered {
            self.stagger[i] -= dt;
        }
        let (routing, awake) = {
            let r = &self.regiments[ri];
            (r.state.routed(), r.awake)
        };
        // ---------------------------------------------------- choose a target
        self.seek_t[i] -= dt;
        let mut tg = self.target[i];
        if tg >= 0
            && (self.state[tg as usize] != S_ALIVE || self.regiments[self.reg[tg as usize]].awake)
        {
            tg = -1;
            self.target[i] = -1;
        }
        let flying_high = t.flyer && self.alt[i] > 2.6;
        if !routing && !awake && self.seek_t[i] <= 0.0 {
            self.seek_t[i] = 0.3 + self.r() * 0.1;
            let r = &self.regiments[ri];
            let shooting = t.ranged.is_some() && r.ammo > 0 && r.firing;
            let mut radius = match &r.order {
                Order::Attack { target, .. }
                    if t.ranged.is_none() || r.ammo == 0 || t.role == Role::Flyer =>
                {
                    let tr = &self.regiments[*target];
                    if (tr.cx - r.cx).hypot(tr.cz - r.cz) < 34.0 {
                        13.0
                    } else {
                        3.0
                    }
                }
                _ if r.state == RState::Melee => 9.0,
                Order::Move { .. } => 2.2,
                _ => {
                    if shooting {
                        3.5
                    } else {
                        6.0
                    }
                }
            };
            if t.flyer && !r.diving {
                radius = 0.0;
            }
            if radius > 0.0 {
                let prefer = r.order_target();
                let e = self.nearest_enemy(i, radius, prefer, !t.flyer);
                if e >= 0 && e != tg {
                    tg = e;
                    self.target[i] = e;
                } else if e < 0 && tg >= 0 {
                    let dd = (self.px[tg as usize] - self.px[i])
                        .hypot(self.pz[tg as usize] - self.pz[i]);
                    if dd > radius * 1.6 {
                        tg = -1;
                        self.target[i] = -1;
                    }
                }
            } else if tg >= 0 && !flying_high {
                let dd =
                    (self.px[tg as usize] - self.px[i]).hypot(self.pz[tg as usize] - self.pz[i]);
                if dd > 4.0 {
                    tg = -1;
                    self.target[i] = -1;
                }
            }
        }
        // ---------------------------------------------------- where to go
        let (gx, gz, mut speed, stop_at);
        if awake {
            gx = self.px[i];
            gz = self.pz[i];
            speed = 0.0;
            stop_at = 0.0;
        } else if routing {
            // run for the own edge of the field, away from the enemy
            let r = &self.regiments[ri];
            let edge_z = if r.side == 0 {
                self.field.play.z1 + 30.0
            } else {
                self.field.play.z0 - 30.0
            };
            let (mut dx, mut dz) = (0.0f32, edge_z - self.pz[i]);
            if let Some(to) = r.flee_to {
                dx = to.x - self.px[i];
                dz = to.z - self.pz[i];
            } else if let Some(foe) = r.flee_from {
                let f = &self.regiments[foe];
                let (ex, ez) = (self.px[i] - f.cx, self.pz[i] - f.cz);
                let el = ex.hypot(ez);
                let el = if el > 0.0 { el } else { 1.0 };
                dx += ex / el * 40.0;
                dz += ez / el * 40.0;
            }
            let l = dx.hypot(dz);
            let l = if l > 0.0 { l } else { 1.0 };
            gx = self.px[i] + dx / l * 20.0;
            gz = self.pz[i] + dz / l * 20.0;
            speed = t.run * 0.95;
            stop_at = 0.0;
        } else if tg >= 0 {
            let tgu = tg as usize;
            gx = self.px[tgu];
            gz = self.pz[tgu];
            stop_at = t.reach + t.radius + self.regiments[self.reg[tgu]].ty.radius - 0.15;
            let r = &self.regiments[ri];
            speed = if self.charge[i] > 0.0 || r.run || r.order_target().is_some() {
                t.run
            } else {
                t.walk * 1.3
            };
        } else {
            let r = &self.regiments[ri];
            let files = r.order_files();
            let k = self.slot[i];
            let (ax, az, heading) = (r.ax, r.az, r.heading);
            let attack = r.order_target().is_some();
            let base = if r.run || attack { t.run } else { t.walk };
            let (sx, sz) = self.slot_pos(ri, k, ax, az, heading, files);
            let (mut wx, mut wz) = (sx, sz);
            // Far from its place with something in the way (a wall left behind, a rock): it finds
            // its own way round, every few seconds.
            let (px, pz) = (self.px[i], self.pz[i]);
            if !t.flyer && (sx - px).hypot(sz - pz) > 14.0 && !self.field.line_clear(px, pz, sx, sz)
            {
                if self.detour_t[i] <= self.time {
                    let wait = 2.0 + self.r();
                    self.detour_t[i] = self.time + wait;
                    self.detour[i] = self.field.find_path(px, pz, sx, sz);
                }
                if let Some(p) = self.detour[i].first().copied() {
                    if (p.x - px).hypot(p.z - pz) < 3.0 {
                        self.detour[i].remove(0);
                    }
                    if let Some(p) = self.detour[i].first() {
                        wx = p.x;
                        wz = p.z;
                    }
                }
            } else if !self.detour[i].is_empty() {
                self.detour[i].clear();
            }
            gx = wx;
            gz = wz;
            speed = base * 1.25;
            stop_at = 0.0;
        }
        if staggered {
            speed *= 0.15;
        }
        // -------------------------------------------- steering & separation
        let (dx, dz) = (gx - self.px[i], gz - self.pz[i]);
        let d = dx.hypot(dz);
        let (mut want_vx, mut want_vz) = (0.0, 0.0);
        if d > stop_at + 0.05 {
            let s = speed.min((d - stop_at) * 2.2);
            want_vx = dx / d * s;
            want_vz = dz / d * s;
        }
        // personal space
        if !flying_high {
            let cx = ((self.px[i] - self.gx0) / CELL).floor() as i32;
            let cz = ((self.pz[i] - self.gz0) / CELL).floor() as i32;
            let ri_r = t.radius;
            for oz in -1..=1 {
                let zz = cz + oz;
                if zz < 0 || zz >= self.gh {
                    continue;
                }
                for ox in -1..=1 {
                    let xx = cx + ox;
                    if xx < 0 || xx >= self.gw {
                        continue;
                    }
                    let mut j = self.head[(zz * self.gw + xx) as usize];
                    while j >= 0 {
                        let ju = j as usize;
                        let next = self.next[ju];
                        if ju != i {
                            let tj = self.regiments[self.reg[ju]].ty;
                            if !(t.flyer != tj.flyer && self.alt[ju] > 2.6) {
                                let (ex, ez) = (self.px[i] - self.px[ju], self.pz[i] - self.pz[ju]);
                                let rr = ri_r + tj.radius;
                                let d2 = ex * ex + ez * ez;
                                if d2 < rr * rr && d2 > 1e-6 {
                                    let dd = d2.sqrt();
                                    let push = (rr - dd) / rr;
                                    // heavier bodies push the lighter aside
                                    let share = tj.mass / (t.mass + tj.mass) * 2.0;
                                    want_vx += ex / dd * push * 4.5 * share;
                                    want_vz += ez / dd * push * 4.5 * share;
                                }
                            }
                        }
                        j = next;
                    }
                }
            }
        }
        // ------------------------------------------------- integrate
        let accel = (if t.large { 2.2 } else { 6.0 }) * if self.plow[i] > 0.0 { 0.15 } else { 1.0 };
        let a = (accel * dt).min(1.0);
        self.vx[i] += (want_vx - self.vx[i]) * a;
        self.vz[i] += (want_vz - self.vz[i]) * a;
        let (mut vx, mut vz) = (self.vx[i], self.vz[i]);
        if !t.flyer || self.alt[i] < 2.0 {
            let ground = self.field.speed_at(self.px[i], self.pz[i]);
            vx *= ground;
            vz *= ground;
        }
        let (mut nx, mut nz) = (self.px[i] + vx * dt, self.pz[i] + vz * dt);
        if !t.flyer && self.field.blocked(nx, nz) {
            // slide along the bank
            if !self.field.blocked(nx, self.pz[i]) {
                nz = self.pz[i];
            } else if !self.field.blocked(self.px[i], nz) {
                nx = self.px[i];
            } else {
                // a corner or a post: step round it at an angle (each soldier keeps to one side)
                let (ox, oz) = (self.px[i], self.pz[i]);
                let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                let mut moved = false;
                for turn in [0.8f32, 1.4, -0.8, -1.4] {
                    let a = turn * side;
                    let (c, s) = (a.cos(), a.sin());
                    let (tx, tz) = (ox + (vx * c + vz * s) * dt, oz + (-vx * s + vz * c) * dt);
                    if !self.field.blocked(tx, tz) {
                        nx = tx;
                        nz = tz;
                        moved = true;
                        break;
                    }
                }
                if !moved {
                    nx = ox;
                    nz = oz;
                    self.vx[i] *= 0.2;
                    self.vz[i] *= 0.2;
                }
            }
        }
        // the routed leave the field
        let play = self.field.play;
        if routing
            && (nz > play.z1 + 24.0
                || nz < play.z0 - 24.0
                || nx < play.x0 - 24.0
                || nx > play.x1 + 24.0)
        {
            self.state[i] = S_FLED;
            {
                let r = &mut self.regiments[ri];
                r.alive = r.alive.saturating_sub(1);
                r.fled += 1;
            }
            if let Some(k) = self.regiments[ri].soldiers.iter().position(|&s| s == i) {
                self.regiments[ri].soldiers.remove(k);
                self.reindex(ri);
            }
            if self.regiments[ri].alive == 0 {
                self.regiments[ri].state = RState::Fled;
            }
            return;
        }
        self.px[i] = nx;
        self.pz[i] = nz;
        // flight
        if t.flyer {
            let r = &self.regiments[ri];
            let want = if r.diving || tg >= 0 {
                0.9
            } else if routing {
                9.0
            } else {
                6.5
            };
            self.alt[i] += clamp(want - self.alt[i], -dt * 5.0, dt * 4.0);
        }
        self.py[i] = self.field.walk_height(nx, nz) + self.alt[i];
        // walk cycle
        let sp = self.vx[i].hypot(self.vz[i]);
        let stride = if t.large {
            2.6
        } else if t.flyer {
            1.2
        } else {
            1.35
        };
        self.walk[i] += sp / stride * dt * PI
            + if t.flyer && self.alt[i] > 1.5 {
                dt * 7.0
            } else {
                0.0
            };
        // facing: where we go, the enemy, or the formation's front
        let r = &self.regiments[ri];
        let mut face = self.yaw[i];
        if tg >= 0 {
            face = (self.px[tg as usize] - self.px[i]).atan2(self.pz[tg as usize] - self.pz[i]);
        } else if sp > 0.6 {
            face = self.vx[i].atan2(self.vz[i]);
        } else if let (RState::Firing, Some(ft)) = (r.state, r.fire_target) {
            let f = &self.regiments[ft];
            face = (f.cx - self.px[i]).atan2(f.cz - self.pz[i]);
        } else if !routing {
            face = r.heading;
        }
        self.yaw[i] = turn_to(self.yaw[i], face, dt * if t.large { 3.0 } else { 7.0 });
        // ------------------------------------------------------- fighting
        if tg >= 0 && !staggered && !routing {
            let tgu = tg as usize;
            let (ex, ez) = (self.px[tgu] - self.px[i], self.pz[tgu] - self.pz[i]);
            let dist = ex.hypot(ez);
            let reach = t.reach + t.radius + self.regiments[self.reg[tgu]].ty.radius;
            // a charge lands on the first enemy it meets
            if self.charge[i] > 0.0
                && sp > t.run * 0.55
                && dist < reach + 0.4
                && self.time - self.last_knock[tgu] > 0.6
            {
                self.impact(i, tgu, sp);
            } else if dist < reach
                && self.cool[i] <= 0.0
                && self.act[i] == 0.0
                && ang_diff(self.yaw[i], ex.atan2(ez)).abs() < 0.9
            {
                let jitter = self.r();
                self.cool[i] = t.cd * (0.85 + jitter * 0.3);
                self.act[i] = 0.001;
                self.act_len[i] = (t.cd * 0.55).min(0.75);
                self.hit_at[i] = 0.5;
            }
        }
        if self.state[i] != S_ALIVE {
            return;
        }
        // large chargers plough through whoever is in the way
        if self.plow[i] > 0.0 && sp > 2.5 {
            self.plough(i, sp);
        }
        // ---------------------------------------------------------- shooting
        let r = &self.regiments[ri];
        if t.ranged.is_some()
            && r.firing
            && r.ammo > 0
            && tg < 0
            && self.reload[i] <= 0.0
            && sp < 1.2
            && !routing
            && !staggered
            && r.state != RState::Moving
        {
            self.shoot(i, ri);
        }
    }

    /// The swing connects (if the target is still in reach). Heroes cleave several.
    fn land(&mut self, i: usize) {
        let ri = self.reg[i];
        let t = self.regiments[ri].ty;
        let side = self.regiments[ri].side;
        let tg = self.target[i];
        if tg < 0 || self.state[tg as usize] != S_ALIVE {
            return;
        }
        let tg = tg as usize;
        if t.cleave > 1 {
            let big_r = t.reach + t.radius + 0.9;
            let mut n = 0u32;
            let cx = ((self.px[i] - self.gx0) / CELL).floor() as i32;
            let cz = ((self.pz[i] - self.gz0) / CELL).floor() as i32;
            let mut victims = Vec::new();
            'outer: for oz in -2..=2 {
                for ox in -2..=2 {
                    let (xx, zz) = (cx + ox, cz + oz);
                    if xx < 0 || zz < 0 || xx >= self.gw || zz >= self.gh {
                        continue;
                    }
                    let mut j = self.head[(zz * self.gw + xx) as usize];
                    while j >= 0 {
                        if n >= t.cleave {
                            break 'outer;
                        }
                        let ju = j as usize;
                        let next = self.next[ju];
                        let rj = &self.regiments[self.reg[ju]];
                        if rj.side != side && self.state[ju] == S_ALIVE && self.alt[ju] <= 2.6 {
                            let (ex, ez) = (self.px[ju] - self.px[i], self.pz[ju] - self.pz[i]);
                            let d = ex.hypot(ez);
                            if d <= big_r && ang_diff(self.yaw[i], ex.atan2(ez)).abs() <= 1.2 {
                                victims.push((ju, ex, ez, d, rj.ty.large));
                                n += 1;
                            }
                        }
                        j = next;
                    }
                }
            }
            for (j, ex, ez, d, large) in &victims {
                self.strike(i, *j, 0.0);
                if !large && self.r() < 0.5 {
                    self.knock[*j] = 1.0;
                    self.stagger[*j] = self.stagger[*j].max(0.6);
                    let l = if *d > 0.0 { *d } else { 1.0 };
                    self.vx[*j] += ex / l * 3.0;
                    self.vz[*j] += ez / l * 3.0;
                }
            }
            if victims.is_empty() {
                self.strike(i, tg, 0.0);
            }
            return;
        }
        let (ex, ez) = (self.px[tg] - self.px[i], self.pz[tg] - self.pz[i]);
        if ex.hypot(ez) < t.reach + t.radius + self.regiments[self.reg[tg]].ty.radius + 0.5 {
            self.strike(i, tg, 0.0);
        }
    }

    /// A charging soldier hits soldier tg: knock-back, a blow with the charge bonus; braced spears
    /// turn the impact on a large charger.
    fn impact(&mut self, i: usize, tg: usize, sp: f32) {
        let (ra, rb) = (self.reg[i], self.reg[tg]);
        let (ta, tb) = (self.regiments[ra].ty, self.regiments[rb].ty);
        if !ta.large {
            self.charge[i] = 0.0;
        }
        self.last_knock[tg] = self.time;
        let (ex, ez) = (self.px[tg] - self.px[i], self.pz[tg] - self.pz[i]);
        let d = ex.hypot(ez);
        let d = if d > 0.0 { d } else { 1.0 };
        // braced spears facing the charge
        let facing = self.yaw[tg].sin() * (-ex / d) + self.yaw[tg].cos() * (-ez / d);
        let braced = tb.anti_large > 0.0
            && ta.large
            && facing > 0.4
            && self.regiments[rb].speed_now < 1.2
            && self.stagger[tg] <= 0.0;
        if braced {
            self.damage(i, tb.anti_large * 1.1 + tb.dmg, Some(rb));
            self.stagger[i] = 1.2;
            self.knock[i] = 1.0;
            self.vx[i] *= -0.3;
            self.vz[i] *= -0.3;
            self.charge[i] = 0.0;
            self.plow[i] = 0.0;
            self.events.push(Event::Brace {
                x: self.px[tg],
                z: self.pz[tg],
            });
            return;
        }
        let power = ta.mass * sp / (tb.mass * 7.0);
        if power > 0.3 {
            let k = power.min(1.6);
            self.knock[tg] = 1.0;
            self.stagger[tg] = self.stagger[tg].max(0.5 + k * 0.8);
            self.vx[tg] += ex / d * k * 4.5;
            self.vz[tg] += ez / d * k * 4.5;
            self.regiments[rb].charge_shock += if ta.large { 0.9 } else { 0.25 };
        }
        self.strike(i, tg, ta.charge);
        self.events.push(Event::Impact {
            x: self.px[tg],
            z: self.pz[tg],
            big: ta.large,
        });
        if ta.large {
            self.plow[i] = 0.7;
        }
    }

    fn plough(&mut self, i: usize, sp: f32) {
        let ri = self.reg[i];
        let (side, radius) = (self.regiments[ri].side, self.regiments[ri].ty.radius);
        let cx = ((self.px[i] - self.gx0) / CELL).floor() as i32;
        let cz = ((self.pz[i] - self.gz0) / CELL).floor() as i32;
        let mut hits = Vec::new();
        for oz in -1..=1 {
            for ox in -1..=1 {
                let (xx, zz) = (cx + ox, cz + oz);
                if xx < 0 || zz < 0 || xx >= self.gw || zz >= self.gh {
                    continue;
                }
                let mut j = self.head[(zz * self.gw + xx) as usize];
                while j >= 0 {
                    let ju = j as usize;
                    let next = self.next[ju];
                    if self.regiments[self.reg[ju]].side != side
                        && self.state[ju] == S_ALIVE
                        && self.time - self.last_knock[ju] >= 0.8
                        && (self.px[ju] - self.px[i]).hypot(self.pz[ju] - self.pz[i]) < radius + 0.8
                    {
                        hits.push(ju);
                    }
                    j = next;
                }
            }
        }
        for j in hits {
            if self.state[i] != S_ALIVE {
                break;
            }
            self.impact(i, j, sp);
        }
    }

    // ------------------------------------------------------------ missiles
    fn shoot(&mut self, i: usize, ri: usize) {
        let Some(rg) = self.regiments[ri].ty.ranged else {
            return;
        };
        let Some(ti) = self.regiments[ri].fire_target else {
            return;
        };
        if self.regiments[ti].alive == 0 || self.regiments[ti].soldiers.is_empty() {
            return;
        }
        let pick = self.r();
        let tgt = {
            let s = &self.regiments[ti].soldiers;
            s[((pick * s.len() as f32) as usize).min(s.len() - 1)]
        };
        let (sx, sz, sy) = (self.px[i], self.pz[i], self.py[i] + 1.5);
        let dist = (self.px[tgt] - sx).hypot(self.pz[tgt] - sz);
        if dist > rg.range * 1.05 {
            return;
        }
        let jitter = self.r();
        self.reload[i] = rg.reload * (0.85 + jitter * 0.35);
        self.regiments[ri].ammo -= 1;
        self.regiments[ri].last_fire = self.time;
        self.act[i] = 0.001;
        self.act_len[i] = 0.55;
        self.hit_at[i] = -1.0;
        let big_t = match rg.arc {
            Arc::High => dist / rg.speed + 1.0,
            Arc::Beam => dist / rg.speed + 0.05,
            Arc::Low => dist / rg.speed + 0.25,
        };
        let spread = rg.spread * (0.4 + dist / rg.range);
        let (j1, j2) = (self.r(), self.r());
        let ax = self.px[tgt] + self.vx[tgt] * big_t * 0.8 + (j1 - 0.5) * 2.0 * spread;
        let az = self.pz[tgt] + self.vz[tgt] * big_t * 0.8 + (j2 - 0.5) * 2.0 * spread;
        let ay = self.py[tgt] + 0.9;
        let g = if rg.arc == Arc::Beam { 0.0 } else { 9.8 };
        self.missiles.push(Missile {
            x: sx,
            y: sy,
            z: sz,
            vx: (ax - sx) / big_t,
            vz: (az - sz) / big_t,
            vy: (ay - sy + 0.5 * g * big_t * big_t) / big_t,
            g,
            t: 0.0,
            big_t,
            side: self.regiments[ri].side,
            reg: ri,
            kind: rg.projectile,
            dmg: rg.dmg,
            ap: rg.ap,
            cohesion: rg.cohesion,
            ax,
            ay,
            az,
            done: false,
            checked: false,
            stuck: 0.0,
        });
        if self.r() < 0.2 {
            self.events.push(Event::Fire {
                x: sx,
                z: sz,
                kind: rg.projectile,
                reg: ri,
            });
        }
    }

    fn update_missiles(&mut self, dt: f32) {
        let mut k = self.missiles.len();
        while k > 0 {
            k -= 1;
            let mut m = self.missiles[k];
            if m.done {
                m.stuck -= dt;
                if m.stuck <= 0.0 {
                    self.missiles.swap_remove(k);
                } else {
                    self.missiles[k] = m;
                }
                continue;
            }
            m.t += dt;
            m.vy -= m.g * dt;
            m.x += m.vx * dt;
            m.y += m.vy * dt;
            m.z += m.vz * dt;
            // the flight's end: at the aim point (maybe a harpy in the air) or on the ground
            let ground = self.field.walk_height(m.x, m.z);
            if m.t >= m.big_t && !m.checked {
                m.checked = true;
                let (ax, ay, az) = (m.ax, m.ay, m.az);
                if self.missile_hit(&mut m, ax, ay, az) {
                    self.missiles[k] = m;
                    continue;
                }
            }
            if m.y <= ground + 0.05 || m.t > m.big_t + 3.0 {
                if !m.checked {
                    let (x, y, z) = (m.x, m.y, m.z);
                    self.missile_hit(&mut m, x, y, z);
                }
                if !m.done {
                    m.done = true;
                    m.y = m.y.max(ground);
                    m.stuck = if m.kind == Projectile::Light {
                        0.2
                    } else if self.field.depth_at(m.x, m.z) > 0.0 {
                        0.3
                    } else {
                        7.0
                    };
                    if m.kind == Projectile::Light {
                        self.events.push(Event::Glow { x: m.x, z: m.z });
                    }
                }
            }
            self.missiles[k] = m;
        }
    }

    fn missile_hit(&mut self, m: &mut Missile, x: f32, y: f32, z: f32) -> bool {
        let cx = ((x - self.gx0) / CELL).floor() as i32;
        let cz = ((z - self.gz0) / CELL).floor() as i32;
        let (mut best, mut bd) = (-1i32, 1.0f32);
        for oz in -1..=1 {
            for ox in -1..=1 {
                let (xx, zz) = (cx + ox, cz + oz);
                if xx < 0 || zz < 0 || xx >= self.gw || zz >= self.gh {
                    continue;
                }
                let mut j = self.head[(zz * self.gw + xx) as usize];
                while j >= 0 {
                    let ju = j as usize;
                    let next = self.next[ju];
                    let rj = &self.regiments[self.reg[ju]];
                    if rj.side != m.side && self.state[ju] == S_ALIVE && !rj.awake {
                        let (dx, dz, dy) =
                            (self.px[ju] - x, self.pz[ju] - z, self.py[ju] + 0.9 - y);
                        let reach = rj.ty.radius + 0.35;
                        let d = dx.hypot(dz) / reach + (dy.abs() - 1.2).max(0.0) * 0.5;
                        if d < bd {
                            bd = d;
                            best = j;
                        }
                    }
                    j = next;
                }
            }
        }
        if best < 0 {
            return false;
        }
        let best = best as usize;
        let rb = self.reg[best];
        let tb = self.regiments[rb].ty;
        m.done = true;
        m.stuck = 0.05;
        // the wood stops some, and shields facing the shot
        if self.field.forest_at(self.px[best], self.pz[best]) > 0.5 && self.r() < 0.4 {
            return true;
        }
        let el = m.vx.hypot(m.vz);
        let el = if el > 0.0 { el } else { 1.0 };
        let facing = -(self.yaw[best].sin() * m.vx + self.yaw[best].cos() * m.vz) / el;
        if facing > 0.4 && self.r() * 100.0 < tb.shield {
            self.events.push(Event::Block {
                x: self.px[best],
                z: self.pz[best],
            });
            return true;
        }
        let dealt = m.dmg * (1.0 - tb.armor / 100.0) * (0.8 + self.r() * 0.4) + m.ap;
        if m.cohesion > 0.0 && tb.spark {
            let r = &mut self.regiments[rb];
            r.cohesion = (r.cohesion - 0.5).max(0.0);
        }
        self.events.push(Event::Pierce {
            x: self.px[best],
            z: self.pz[best],
        });
        self.damage(best, dealt, Some(m.reg));
        true
    }

    // ------------------------------------------------------------ reading
    /// Strength still on the field per side (for the balance bar).
    pub fn strength(&self) -> [f32; 2] {
        let mut s = [0.0; 2];
        for r in &self.regiments {
            if r.alive == 0 || r.state == RState::Fled {
                continue;
            }
            let mut k = r.alive as f32
                * (r.ty.hp / 15.0)
                * (1.0 + r.ty.armor / 100.0)
                * if r.ty.role == Role::Hero { 25.0 } else { 1.0 };
            if r.state.routed() {
                k *= 0.25;
            }
            if r.awake {
                k = 0.0;
            }
            s[r.side as usize] += k;
        }
        s
    }

    /// Whether soldier i's regiment is charging right now (its legs pumping).
    pub fn charging(&self, i: usize) -> bool {
        self.charge[i] > 0.0
    }
}
