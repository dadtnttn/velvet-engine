//! The enemy general. Every 1.2 s it looks at the field and gives the same orders the player can:
//! the line advances together and picks fights it can win, missile units stand behind it and fall
//! back from melee, horse and wolves wait on the wings to hunt archers or hit an engaged enemy in
//! the flank, and pull out to charge again; priests follow the line; the hero joins the press.
//! A defending army holds its ground (and its hill) until the enemy comes close.
use std::collections::HashMap;

use crate::data::Role;
use crate::sim::{RState, Regiment, Sim};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Posture {
    Attack,
    Defend,
}

#[derive(Clone, Copy, Debug, Default)]
struct Memo {
    target: Option<usize>,
    at: f32,
    out: bool,
}

pub struct BattleAi {
    pub side: u8,
    pub posture: Posture,
    t: f32,
    memo: HashMap<usize, Memo>,
}

/// Distance between regiments along the way (round the river through a crossing).
fn dist(sim: &Sim, a: (f32, f32, bool), b: (f32, f32)) -> f32 {
    if a.2 {
        (a.0 - b.0).hypot(a.1 - b.1)
    } else {
        sim.field.route_length(a.0, a.1, b.0, b.1)
    }
}

fn center(list: &[&Regiment]) -> Option<(f32, f32)> {
    let (mut x, mut z, mut n) = (0.0, 0.0, 0.0);
    for r in list {
        x += r.cx * r.alive as f32;
        z += r.cz * r.alive as f32;
        n += r.alive as f32;
    }
    (n > 0.0).then(|| (x / n, z / n))
}

impl BattleAi {
    pub fn new(side: u8, posture: Posture) -> Self {
        Self {
            side,
            posture,
            t: 0.0,
            memo: HashMap::new(),
        }
    }

    pub fn update(&mut self, sim: &mut Sim, dt: f32) {
        self.t -= dt;
        if self.t > 0.0 {
            return;
        }
        self.t = 1.2;
        let side = self.side;
        let mine: Vec<usize> = (0..sim.regiments.len())
            .filter(|&i| sim.regiments[i].side == side && sim.can_command(i))
            .collect();
        let live: Vec<usize> = sim
            .regiments
            .iter()
            .filter(|r| {
                r.side != side
                    && r.alive > 0
                    && r.state != RState::Fled
                    && !r.awake
                    && !r.state.routed()
            })
            .map(|r| r.index)
            .collect();
        if mine.is_empty() || live.is_empty() {
            return;
        }
        let regs = &sim.regiments;
        let mine_regs: Vec<&Regiment> = mine.iter().map(|&i| &regs[i]).collect();
        let not_hero: Vec<&Regiment> = mine_regs
            .iter()
            .copied()
            .filter(|r| r.ty.role != Role::Hero)
            .collect();
        let Some(my_c) = center(&not_hero).or_else(|| center(&mine_regs)) else {
            return;
        };
        let live_regs: Vec<&Regiment> = live.iter().map(|&i| &regs[i]).collect();
        let Some(foe_c) = center(&live_regs) else {
            return;
        };
        let gap = dist(sim, (my_c.0, my_c.1, false), foe_c);
        if self.posture == Posture::Defend && (gap < 120.0 || sim.time > 75.0) {
            self.posture = Posture::Attack;
        }
        let g = if gap > 0.0 { gap } else { 1.0 };
        let fwd = ((foe_c.0 - my_c.0) / g, (foe_c.1 - my_c.1) / g);
        let line: Vec<&Regiment> = mine_regs
            .iter()
            .copied()
            .filter(|r| matches!(r.ty.role, Role::Infantry | Role::Spear))
            .collect();
        let line_c = center(&line).unwrap_or(my_c);
        let line_gap = live_regs
            .iter()
            .map(|f| dist(sim, (line_c.0, line_c.1, false), (f.cx, f.cz)))
            .fold(f32::INFINITY, f32::min);

        // Decide every order first (reading), then give them (writing).
        enum Do {
            Attack(usize, usize, bool),
            Move(usize, f32, f32, Option<f32>, bool),
        }
        let mut todo: Vec<Do> = Vec::new();
        let time = sim.time;
        for &ri in &mine {
            let r = &regs[ri];
            let role = r.ty.role;
            let me = (r.cx, r.cz, r.ty.flyer);
            let m = self.memo.entry(ri).or_default();
            let d_to = |f: &Regiment| dist(sim, me, (f.cx, f.cz));
            let best = |filter: &dyn Fn(&Regiment) -> bool,
                        score: &dyn Fn(&Regiment) -> f32|
             -> Option<usize> {
                let mut b = None;
                let mut bs = f32::INFINITY;
                for f in &live_regs {
                    if !filter(f) {
                        continue;
                    }
                    let s = score(f);
                    if s < bs {
                        bs = s;
                        b = Some(f.index);
                    }
                }
                b
            };
            let engaged_foe = |f: &Regiment| f.engaged > 0 || f.state == RState::Melee;
            let attacking = r.order_target().is_some();
            // an attack on the same target, given lately and still going, is not given again
            let order_attack = |m: &mut Memo, target: usize, run: bool, todo: &mut Vec<Do>| {
                if m.target == Some(target) && time - m.at < 6.0 && attacking {
                    return;
                }
                m.target = Some(target);
                m.at = time;
                todo.push(Do::Attack(ri, target, run));
            };

            if self.posture == Posture::Defend && role != Role::Archer && role != Role::Skirmisher {
                // hold the line; turn to face the threat
                if let Some(near) = best(&|_| true, &|f| d_to(f)) {
                    if d_to(&regs[near]) < 40.0 && role != Role::Support {
                        order_attack(m, near, false, &mut todo);
                    }
                }
                continue;
            }
            match role {
                Role::Infantry | Role::Spear | Role::Hero => {
                    let target = best(&|_| true, &|f| {
                        let mut s = d_to(f);
                        if f.ty.role == Role::Cavalry && role != Role::Spear {
                            s += 25.0;
                        }
                        if f.ty.flyer {
                            s += 40.0;
                        }
                        if f.state == RState::Routing {
                            s += 60.0;
                        }
                        if role == Role::Spear && f.ty.large {
                            s -= 30.0;
                        }
                        if engaged_foe(f) {
                            s -= 12.0;
                        }
                        s
                    });
                    let Some(target) = target else {
                        continue;
                    };
                    if line_gap > 85.0 && role != Role::Hero && d_to(&regs[target]) > 70.0 {
                        // advance as a line, not a stampede
                        if time - m.at > 4.0 || !r.is_moving_order() {
                            m.at = time;
                            m.target = None;
                            todo.push(Do::Move(
                                ri,
                                r.cx + fwd.0 * 36.0,
                                r.cz + fwd.1 * 36.0,
                                Some(fwd.0.atan2(fwd.1)),
                                false,
                            ));
                        }
                    } else {
                        let run = d_to(&regs[target]) < 60.0;
                        order_attack(m, target, run, &mut todo);
                    }
                }
                Role::Archer | Role::Skirmisher => {
                    let threat = best(
                        &|f| f.ty.ranged.is_none() && f.ty.role != Role::Support,
                        &|f| d_to(f),
                    );
                    if let Some(th) = threat {
                        let t = &regs[th];
                        if d_to(t) < 34.0 && r.engaged == 0 {
                            // fall back behind the line
                            let (ax, az) = (r.cx - t.cx, r.cz - t.cz);
                            let l = ax.hypot(az);
                            let l = if l > 0.0 { l } else { 1.0 };
                            let x = r.cx + ax / l * 32.0 - fwd.0 * 8.0;
                            let z = r.cz + az / l * 32.0 - fwd.1 * 8.0;
                            if time - m.at > 3.0 && !sim.field.blocked(x, z) {
                                m.at = time;
                                m.target = None;
                                todo.push(Do::Move(ri, x, z, Some(fwd.0.atan2(fwd.1)), true));
                            }
                            continue;
                        }
                    }
                    if r.ammo == 0 {
                        if let Some(t) = best(&engaged_foe, &|f| d_to(f))
                            .or_else(|| best(&|_| true, &|f| d_to(f)))
                        {
                            order_attack(m, t, false, &mut todo);
                        }
                        continue;
                    }
                    let t = best(&|_| true, &|f| {
                        d_to(f)
                            + if f.state == RState::Routing {
                                80.0
                            } else {
                                0.0
                            }
                            + if f.ty.large { -10.0 } else { 0.0 }
                    });
                    if let Some(t) = t {
                        order_attack(m, t, false, &mut todo);
                    }
                }
                Role::Cavalry | Role::Beast | Role::Flyer => {
                    // charge, pull out, charge again
                    if m.out {
                        if r.is_hold() || time - m.at > 9.0 {
                            m.out = false;
                        } else {
                            continue;
                        }
                    }
                    if r.engaged > 0
                        && time - m.at > 9.0
                        && (r.morale < 55.0 || (r.alive as f32) < r.men0 as f32 * 0.6)
                    {
                        if let Some(foe) = sim.nearest_regiment(ri, 30.0, |_| true) {
                            let f = &regs[foe];
                            let (ax, az) = (r.cx - f.cx, r.cz - f.cz);
                            let l = ax.hypot(az);
                            let l = if l > 0.0 { l } else { 1.0 };
                            let (x, z) = (r.cx + ax / l * 55.0, r.cz + az / l * 55.0);
                            if !sim.field.blocked(x, z) {
                                m.out = true;
                                m.at = time;
                                m.target = None;
                                todo.push(Do::Move(ri, x, z, None, true));
                                continue;
                            }
                        }
                    }
                    let soft = best(
                        &|f| f.ty.ranged.is_some() || f.ty.role == Role::Support,
                        &|f| d_to(f) + if f.engaged > 0 { 30.0 } else { 0.0 },
                    );
                    let busy = best(&engaged_foe, &|f| d_to(f));
                    let t = match (soft, busy) {
                        (Some(s), _) if d_to(&regs[s]) < 260.0 => Some(s),
                        (_, Some(b)) => Some(b),
                        _ if line_gap < 60.0 => best(&|_| true, &|f| d_to(f)),
                        _ => None,
                    };
                    if let Some(t) = t {
                        order_attack(m, t, false, &mut todo);
                    } else if time - m.at > 5.0 && !r.is_moving_order() {
                        // wait on the wing of the line
                        let s = if r.cx - line_c.0 >= 0.0 { 1.0 } else { -1.0 };
                        let x = line_c.0 + fwd.1 * s * 55.0 - fwd.0 * 5.0;
                        let z = line_c.1 - fwd.0 * s * 55.0 - fwd.1 * 5.0;
                        m.at = time;
                        todo.push(Do::Move(ri, x, z, Some(fwd.0.atan2(fwd.1)), false));
                    }
                }
                Role::Support => {
                    // stay behind the line, facing the enemy
                    let (x, z) = (line_c.0 - fwd.0 * 22.0, line_c.1 - fwd.1 * 22.0);
                    if (x - r.cx).hypot(z - r.cz) > 14.0 && time - m.at > 3.0 {
                        m.at = time;
                        todo.push(Do::Move(ri, x, z, Some(fwd.0.atan2(fwd.1)), false));
                    }
                }
            }
        }
        for d in todo {
            match d {
                Do::Attack(ri, t, run) => {
                    sim.order_attack(ri, t, run, false);
                }
                Do::Move(ri, x, z, heading, run) => {
                    let files = sim.regiments[ri].files;
                    sim.order_move(ri, x, z, heading, Some(files), run, false);
                }
            }
        }
    }
}
