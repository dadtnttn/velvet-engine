//! A battle without its window: the field and both armies deployed, the simulation stepped at a
//! fixed rate (pause, ×1–×3), the enemy general, and the player's orders (move a group keeping its
//! shape, drag a line for width and facing, attack, the spark), deployment, and the result.
use std::f32::consts::PI;

use crate::ai::{BattleAi, Posture};
use crate::data::{unit, Faction, Role};
use crate::field::Field;
use crate::maps::{map_spec, Rect};
use crate::noise::clamp;
use crate::rules::{BattleResult, Survivor};
use crate::sim::{Sim, DT};

#[derive(Clone, Debug)]
pub struct SetupUnit {
    pub ty: String,
    pub men: Option<u32>,
    pub id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ArmySetup {
    pub faction: Faction,
    pub units: Vec<SetupUnit>,
}

impl ArmySetup {
    pub fn of(faction: Faction, units: &[&str]) -> Self {
        Self {
            faction,
            units: units
                .iter()
                .map(|t| SetupUnit {
                    ty: (*t).to_owned(),
                    men: None,
                    id: None,
                })
                .collect(),
        }
    }

    /// Unit size (Total War's setting): scales every regiment's men (heroes stay one).
    pub fn scaled(mut self, k: f32) -> Self {
        for u in &mut self.units {
            let t = unit(&u.ty);
            u.men = Some(if t.role == Role::Hero {
                1
            } else {
                ((u.men.unwrap_or(t.men) as f32 * k).round() as u32).max(4)
            });
        }
        self
    }
}

#[derive(Clone, Debug)]
pub struct Setup {
    pub map: String,
    pub title: String,
    pub kicker: String,
    pub subtitle: String,
    pub player: ArmySetup,
    pub enemy: ArmySetup,
    pub player_attacks: bool,
    pub enemy_posture: Option<Posture>,
    pub seed: u32,
    pub dimming: bool,
    pub deploy: bool,
}

impl Setup {
    /// A quick battle from the title's list.
    pub fn scenario(id: &str, scale: f32) -> Setup {
        let s = crate::data::scenario(id);
        Setup {
            map: s.map.to_owned(),
            title: s.title.to_owned(),
            kicker: s.kicker.to_owned(),
            subtitle: s.subtitle.to_owned(),
            player: ArmySetup::of(s.player.0, s.player.1).scaled(scale),
            enemy: ArmySetup::of(s.enemy.0, s.enemy.1).scaled(scale),
            player_attacks: s.player_attacks,
            enemy_posture: None,
            seed: 7,
            dimming: true,
            deploy: true,
        }
    }
}

/// One slot of a dragged line: which regiment, where, facing, how wide.
#[derive(Clone, Copy, Debug)]
pub struct LineSlot {
    pub reg: usize,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub files: u32,
}

pub struct Battle {
    pub setup: Setup,
    pub sim: Sim,
    pub ai: BattleAi,
    /// The AI playing the player's side too (QA, demo).
    pub auto_player: Option<BattleAi>,
    pub zone: Rect,
    pub enemy_zone: Rect,
    pub queen: Option<usize>,
    pub deploying: bool,
    pub paused: bool,
    pub speed: f32,
    acc: f32,
    pub withdrawn: bool,
    pub title: String,
    pub map_title: String,
}

impl Battle {
    pub fn new(setup: Setup) -> Battle {
        let spec = map_spec(&setup.map, None);
        let map_title = spec.title.to_owned();
        let deploy_player = spec.deploy_player;
        let deploy_enemy = spec.deploy_enemy;
        let field = Field::build(spec);
        let p = field.play;
        let zone = deploy_player.unwrap_or(Rect {
            x0: p.x0 + 40.0,
            x1: p.x1 - 40.0,
            z0: p.z1 - 95.0,
            z1: p.z1 - 12.0,
        });
        let enemy_zone = deploy_enemy.unwrap_or(Rect {
            x0: p.x0 + 40.0,
            x1: p.x1 - 40.0,
            z0: p.z0 + 12.0,
            z1: p.z0 + 95.0,
        });
        let sim = Sim::new(field, setup.seed, setup.dimming);
        let posture = setup.enemy_posture.unwrap_or(if setup.player_attacks {
            Posture::Defend
        } else {
            Posture::Attack
        });
        let title = if setup.title.is_empty() {
            map_title.clone()
        } else {
            setup.title.clone()
        };
        let mut b = Battle {
            ai: BattleAi::new(1, posture),
            auto_player: None,
            sim,
            zone,
            enemy_zone,
            queen: None,
            deploying: setup.deploy,
            paused: setup.deploy,
            speed: 1.0,
            acc: 0.0,
            withdrawn: false,
            title,
            map_title,
            setup,
        };
        let (player, enemy) = (b.setup.player.clone(), b.setup.enemy.clone());
        b.deploy_army(0, &player, zone, PI);
        b.deploy_army(1, &enemy, enemy_zone, 0.0);
        b.queen = b
            .sim
            .regiments
            .iter()
            .find(|r| r.side == 0 && r.ability.is_some())
            .map(|r| r.index);
        if let Some(q) = b.queen {
            b.sim.regiments[q].selected = true;
        }
        b
    }

    /// Lays an army out in its zone: a line of infantry, skirmishers ahead, missiles behind,
    /// horse/wolves/harpies on the wings, the hero behind the centre.
    fn deploy_army(&mut self, side: u8, army: &ArmySetup, zone: Rect, heading: f32) {
        let fwd = if heading == 0.0 { 1.0 } else { -1.0 };
        let front = if side == 0 {
            zone.z0 + 26.0
        } else {
            zone.z1 - 26.0
        };
        let cx = (zone.x0 + zone.x1) / 2.0;
        let by = |roles: &[Role]| -> Vec<SetupUnit> {
            army.units
                .iter()
                .filter(|u| roles.contains(&unit(&u.ty).role))
                .cloned()
                .collect()
        };
        let row = |b: &mut Battle, list: &[SetupUnit], z: f32, gap: f32| -> f32 {
            let widths: Vec<f32> = list
                .iter()
                .map(|u| {
                    let t = unit(&u.ty);
                    (t.files.min(u.men.unwrap_or(t.men)) as f32 - 1.0) * t.spacing + 4.0
                })
                .collect();
            let total = widths.iter().sum::<f32>() + gap * (list.len().saturating_sub(1)) as f32;
            let mut x = cx - total / 2.0;
            for (u, w) in list.iter().zip(&widths) {
                let rx = x + w / 2.0;
                x += w + gap;
                b.sim.add_regiment(
                    &u.ty,
                    side,
                    u.men,
                    rx,
                    z,
                    Some(heading),
                    None,
                    Some(army.faction),
                    u.id.clone(),
                );
            }
            total
        };
        let line_total = row(self, &by(&[Role::Infantry, Role::Spear]), front, 6.0);
        row(self, &by(&[Role::Skirmisher]), front + fwd * 22.0, 10.0);
        row(
            self,
            &by(&[Role::Archer, Role::Support]),
            front - fwd * 26.0,
            8.0,
        );
        let heroes = by(&[Role::Hero]);
        let n = heroes.len() as f32;
        for (k, u) in heroes.iter().enumerate() {
            self.sim.add_regiment(
                &u.ty,
                side,
                Some(1),
                cx + (k as f32 - (n - 1.0) / 2.0) * 12.0,
                front - fwd * 14.0,
                Some(heading),
                None,
                Some(army.faction),
                u.id.clone(),
            );
        }
        let half = (line_total / 2.0 + 22.0).max(40.0);
        for (k, u) in by(&[Role::Cavalry, Role::Beast, Role::Flyer])
            .iter()
            .enumerate()
        {
            let s = if k % 2 == 0 { -1.0 } else { 1.0 };
            let lane = (k / 2) as f32;
            let x = clamp(
                cx + s * (half + 18.0 + lane * 40.0),
                zone.x0 + 10.0,
                zone.x1 - 10.0,
            );
            self.sim.add_regiment(
                &u.ty,
                side,
                u.men,
                x,
                front - fwd * (6.0 + lane * 4.0),
                Some(heading),
                None,
                Some(army.faction),
                u.id.clone(),
            );
        }
        for ri in 0..self.sim.regiments.len() {
            if self.sim.regiments[ri].side == side {
                self.settle(ri, fwd);
            }
        }
    }

    /// Steps a freshly deployed regiment off water, rock and walls (towards the front, then aside).
    fn settle(&mut self, ri: usize, fwd: f32) {
        if self.sim.regiments[ri].ty.flyer {
            return;
        }
        let bad = |b: &Battle, x: f32, z: f32| -> f32 {
            let r = &b.sim.regiments[ri];
            let (mut n, mut m) = (0, 0);
            let mut k = 0;
            while k < r.alive as usize {
                let (sx, sz) = b.sim.slot_pos(ri, k, x, z, r.heading, r.files);
                if b.sim.field.blocked(sx, sz) {
                    n += 1;
                }
                m += 1;
                k += 3;
            }
            n as f32 / m.max(1) as f32
        };
        let (cx, cz) = (self.sim.regiments[ri].cx, self.sim.regiments[ri].cz);
        if bad(self, cx, cz) < 0.04 {
            return;
        }
        for d in [6.0, 12.0, 18.0, 24.0, 32.0, 40.0, 50.0, 62.0, 76.0] {
            for (ox, oz) in [(0.0, fwd * d), (d, 0.0), (-d, 0.0), (0.0, -fwd * d)] {
                if bad(self, cx + ox, cz + oz) < 0.04 {
                    let (h, f) = (self.sim.regiments[ri].heading, self.sim.regiments[ri].files);
                    self.place(ri, cx + ox, cz + oz, h, f, false);
                    return;
                }
            }
        }
    }

    /// In deployment, orders place the regiment at once (inside the zone).
    pub fn place(&mut self, ri: usize, x: f32, z: f32, heading: f32, files: u32, in_zone: bool) {
        let files = files.clamp(1, self.sim.regiments[ri].alive.max(1));
        self.sim.regiments[ri].files = files;
        let half_w = self.sim.width(ri, files) / 2.0 + 2.0;
        let (mut x, mut z) = (x, z);
        if in_zone {
            let zn = self.zone;
            x = clamp(x, zn.x0 + half_w * 0.5, zn.x1 - half_w * 0.5);
            z = clamp(z, zn.z0 + 4.0, zn.z1 - 4.0);
        }
        {
            let r = &mut self.sim.regiments[ri];
            r.heading = heading;
            r.ax = x;
            r.cx = x;
            r.az = z;
            r.cz = z;
            r.order = crate::sim::Order::Hold { files };
        }
        self.sim.reassign(ri, heading, files);
        let soldiers = self.sim.regiments[ri].soldiers.clone();
        for (k, i) in soldiers.into_iter().enumerate() {
            let (sx, sz) = self.sim.slot_pos(ri, k, x, z, heading, files);
            self.sim.px[i] = sx;
            self.sim.pz[i] = sz;
            self.sim.vx[i] = 0.0;
            self.sim.vz[i] = 0.0;
            self.sim.yaw[i] = heading;
            self.sim.py[i] = self.sim.field.walk_height(sx, sz) + self.sim.alt[i];
        }
    }

    pub fn begin(&mut self) {
        self.deploying = false;
        self.paused = false;
    }

    pub fn selected(&self) -> Vec<usize> {
        self.sim
            .regiments
            .iter()
            .filter(|r| r.selected && r.side == 0 && r.alive > 0)
            .map(|r| r.index)
            .collect()
    }

    /// Move the selected: one goes there; a group keeps its shape, turned to face where it goes.
    /// Returns where each will stand (for the order's ghost).
    pub fn command_move(&mut self, regs: &[usize], x: f32, z: f32, run: bool) -> Vec<LineSlot> {
        let mut out = Vec::new();
        if regs.is_empty() {
            return out;
        }
        if regs.len() == 1 {
            let ri = regs[0];
            let r = &self.sim.regiments[ri];
            let heading = if (x - r.cx).hypot(z - r.cz) > 4.0 {
                (x - r.cx).atan2(z - r.cz)
            } else {
                r.heading
            };
            let files = r.files;
            if self.deploying {
                self.place(ri, x, z, heading, files, true);
            } else {
                self.sim
                    .order_move(ri, x, z, Some(heading), Some(files), run, false);
            }
            out.push(LineSlot {
                reg: ri,
                x,
                z,
                heading,
                files,
            });
            return out;
        }
        let (mut gx, mut gz, mut n, mut hx, mut hz) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for &ri in regs {
            let r = &self.sim.regiments[ri];
            gx += r.cx * r.alive as f32;
            gz += r.cz * r.alive as f32;
            n += r.alive as f32;
            hx += r.heading.sin();
            hz += r.heading.cos();
        }
        let n = n.max(1.0);
        gx /= n;
        gz /= n;
        let h0 = hx.atan2(hz);
        let h1 = if (x - gx).hypot(z - gz) > 12.0 {
            (x - gx).atan2(z - gz)
        } else {
            h0
        };
        let a = h1 - h0;
        let (c, s) = (a.cos(), a.sin());
        for &ri in regs {
            let (ox, oz, rh, files) = {
                let r = &self.sim.regiments[ri];
                (r.cx - gx, r.cz - gz, r.heading, r.files)
            };
            let (rx, rz) = (ox * c + oz * s, -ox * s + oz * c);
            let (tx, tz) = (x + rx, z + rz);
            let heading = rh + a;
            if self.deploying {
                self.place(ri, tx, tz, heading, files, true);
            } else {
                self.sim
                    .order_move(ri, tx, tz, Some(heading), Some(files), run, false);
            }
            out.push(LineSlot {
                reg: ri,
                x: tx,
                z: tz,
                heading,
                files,
            });
        }
        out
    }

    /// The dragged line: the selected regiments share it in their left-to-right order.
    pub fn line_slots(&self, regs: &[usize], a: (f32, f32), b: (f32, f32)) -> Vec<LineSlot> {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dz);
        let l = if l > 0.0 { l } else { 1.0 };
        let (ux, uz) = (dx / l, dz / l);
        let heading = uz.atan2(-ux);
        let mut sorted: Vec<usize> = regs.to_vec();
        let along = |ri: usize| {
            let r = &self.sim.regiments[ri];
            (r.cx - a.0) * ux + (r.cz - a.1) * uz
        };
        sorted.sort_by(|p, q| along(*p).total_cmp(&along(*q)));
        let natural: Vec<f32> = sorted
            .iter()
            .map(|&ri| self.sim.width(ri, self.sim.regiments[ri].files).max(2.0) + 4.0)
            .collect();
        let sum: f32 = natural.iter().sum();
        let mut off = 0.0;
        sorted
            .iter()
            .zip(&natural)
            .map(|(&ri, w)| {
                let seg = w / sum * l;
                let mid = off + seg / 2.0;
                off += seg;
                let r = &self.sim.regiments[ri];
                let files = clamp(
                    ((seg - 4.0) / r.ty.spacing).round() + 1.0,
                    1.0,
                    r.alive.max(1) as f32,
                ) as u32;
                LineSlot {
                    reg: ri,
                    x: a.0 + ux * mid,
                    z: a.1 + uz * mid,
                    heading,
                    files,
                }
            })
            .collect()
    }

    pub fn command_line(
        &mut self,
        regs: &[usize],
        a: (f32, f32),
        b: (f32, f32),
        run: bool,
    ) -> Vec<LineSlot> {
        let slots = self.line_slots(regs, a, b);
        for s in &slots {
            if self.deploying {
                self.place(s.reg, s.x, s.z, s.heading, s.files, true);
            } else {
                self.sim
                    .order_move(s.reg, s.x, s.z, Some(s.heading), Some(s.files), run, false);
            }
        }
        slots
    }

    pub fn command_attack(&mut self, regs: &[usize], target: usize) {
        if self.deploying {
            return;
        }
        for &ri in regs {
            self.sim.order_attack(ri, target, true, false);
        }
    }

    pub fn cast_spark(&mut self) -> bool {
        match self.queen {
            Some(q) if !self.deploying && self.sim.regiments[q].alive > 0 => self.sim.cast_spark(q),
            _ => false,
        }
    }

    /// Time passes: the generals think, the simulation steps (at most 8 a frame).
    pub fn update(&mut self, dt: f32) {
        let play = if self.paused { 0.0 } else { dt * self.speed };
        self.ai.update(&mut self.sim, play);
        if let Some(ai) = self.auto_player.as_mut() {
            ai.update(&mut self.sim, play);
        }
        if self.paused {
            return;
        }
        self.acc += play;
        let mut steps = 0;
        while self.acc >= DT && steps < 8 {
            self.sim.step();
            self.acc -= DT;
            steps += 1;
        }
        if self.acc > DT * 4.0 {
            self.acc = 0.0;
        }
    }

    /// Runs the battle forward without drawing (the AI may play both sides).
    pub fn skip(&mut self, seconds: f32, auto_player: bool) {
        if auto_player && self.auto_player.is_none() {
            self.auto_player = Some(BattleAi::new(0, Posture::Attack));
        }
        let steps = (seconds / DT).round() as usize;
        for s in 0..steps {
            if self.sim.over.is_some() {
                break;
            }
            self.sim.step();
            if s % 6 == 0 {
                self.ai.update(&mut self.sim, DT * 6.0);
                if let Some(ai) = self.auto_player.as_mut() {
                    ai.update(&mut self.sim, DT * 6.0);
                }
            }
            if s % 30 == 0 {
                self.sim.events.truncate(200);
            }
        }
        self.sim.events.clear();
    }

    pub fn withdraw(&mut self) {
        self.withdrawn = true;
        self.sim.over = Some(crate::sim::Over {
            winner: 1,
            time: self.sim.time,
        });
    }

    /// The battle's outcome, for the end card and the campaign.
    pub fn result(&self) -> BattleResult {
        let side = |s: u8| -> Vec<Survivor> {
            self.sim
                .regiments
                .iter()
                .filter(|r| r.side == s)
                .map(|r| Survivor {
                    id: r.id.clone(),
                    ty: r.ty.id.to_owned(),
                    men0: r.men0,
                    men: if r.awake { 0 } else { r.alive + r.fled },
                    kills: r.kills,
                })
                .collect()
        };
        let winner = self.sim.over.map_or(1, |o| o.winner);
        let mine = side(0);
        let total: u32 = mine.iter().map(|u| u.men0).sum();
        let lost_men: u32 = mine.iter().map(|u| u.men0.saturating_sub(u.men)).sum();
        let lost = lost_men as f32 / total.max(1) as f32;
        let label = if winner == 0 {
            if lost < 0.15 {
                "Victoria heroica"
            } else if lost < 0.35 {
                "Victoria decisiva"
            } else if lost < 0.6 {
                "Victoria"
            } else {
                "Victoria pírrica"
            }
        } else if self.withdrawn {
            "Retirada"
        } else if lost > 0.8 {
            "Derrota aplastante"
        } else {
            "Derrota"
        };
        let varrak_awake = self
            .sim
            .regiments
            .iter()
            .any(|r| r.ty.id == "varrak" && r.awake);
        let queen_down = self.queen.is_some_and(|q| self.sim.regiments[q].alive == 0);
        let note = if varrak_awake {
            "Varrak ha despertado con la chispa de la Reina. «Tenemos una partida pendiente, Majestad.»"
        } else if queen_down {
            "La Reina se retira herida. Su chispa tardará en volver a brillar del todo."
        } else {
            ""
        };
        BattleResult {
            winner,
            player: mine,
            enemy: side(1),
            auto: false,
            label: label.to_owned(),
            varrak_awake,
            queen_down,
            note: note.to_owned(),
            lost,
            time: self.sim.time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_battle_fought_by_both_generals_ends() {
        for id in ["vado", "riscos", "cienaga", "atalaya", "puerta"] {
            let mut b = Battle::new(Setup::scenario(id, 0.5));
            b.begin();
            let t0 = std::time::Instant::now();
            b.skip(600.0, true);
            let res = b.result();
            eprintln!(
                "{id}: over {:?} at {:.0} s, winner {}, lost {:.2} ({} ms)",
                b.sim.over.map(|o| o.time),
                b.sim.time,
                res.winner,
                res.lost,
                t0.elapsed().as_millis()
            );
            assert!(b.sim.over.is_some(), "{id}: no end after ten minutes");
        }
    }

    #[test]
    fn deployment_keeps_everyone_on_open_ground() {
        for id in ["vado", "riscos", "cienaga", "atalaya", "puerta"] {
            let b = Battle::new(Setup::scenario(id, 1.0));
            let blocked = (0..b.sim.n)
                .filter(|&i| {
                    !b.sim.regiments[b.sim.reg[i]].ty.flyer
                        && b.sim.field.blocked(b.sim.px[i], b.sim.pz[i])
                })
                .count();
            assert!(
                blocked < b.sim.n / 25,
                "{id}: {blocked} of {} stand on rock or water",
                b.sim.n
            );
        }
    }

    #[test]
    fn the_spark_rallies_and_staggers() {
        let mut b = Battle::new(Setup::scenario("vado", 0.5));
        b.begin();
        b.skip(40.0, true);
        let q = b.queen.unwrap();
        b.sim.regiments[q].ability.as_mut().unwrap().cd = 0.0;
        assert!(b.cast_spark());
        assert!(!b.cast_spark(), "it waits for its cooldown");
    }
}
