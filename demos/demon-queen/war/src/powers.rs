//! The Queen's war powers: what the story and the war's relics give her to use in a battle,
//! besides the spark. Each is cast on a point of the field (a ring the player places), waits its
//! cooldown, and works for a while:
//!
//! - **Grito de la Reina** (her head up, from the story): her regiments round the point take heart
//!   (morale and spark back) and run faster for a while.
//! - **Rugido de Varrak** (Varrak awake): the enemy round the point loses heart at once.
//! - **Bendición de Nyxara** (the doctor's trust): her soldiers round the point heal.
//! - **Lluvia de brasas** (a relic): embers rain on the point for a few seconds.
//! - **Niebla del sello** (a relic): a violet fog where the enemy's arrows get lost and its feet
//!   go slow.
//! - **Raíces del bosque** (a relic): the roots hold the enemy's feet where they stand.
use crate::sim::{Sim, S_ALIVE};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Power {
    Grito,
    Rugido,
    Bendicion,
    Brasas,
    Niebla,
    Raices,
}

pub struct PowerSpec {
    pub name: &'static str,
    pub what: &'static str,
    /// Where it comes from (shown while it is still locked).
    pub from: &'static str,
    pub cooldown: f32,
    pub radius: f32,
    /// How long it works (0: at once).
    pub duration: f32,
    /// Its colour (sRGB hex), for the ring and the light.
    pub color: u32,
    /// The key it is cast with.
    pub key: &'static str,
}

impl Power {
    pub const ALL: [Power; 6] = [
        Power::Grito,
        Power::Rugido,
        Power::Bendicion,
        Power::Brasas,
        Power::Niebla,
        Power::Raices,
    ];

    pub fn spec(self) -> &'static PowerSpec {
        match self {
            Power::Grito => &PowerSpec {
                name: "Grito de la Reina",
                what: "Tus regimientos cercanos recuperan la moral y la chispa, y corren más durante un rato",
                from: "La Reina levanta la cabeza (acto 1)",
                cooldown: 70.0,
                radius: 34.0,
                duration: 14.0,
                color: 0xf2b54a,
                key: "Z",
            },
            Power::Rugido => &PowerSpec {
                name: "Rugido de Varrak",
                what: "El enemigo cercano pierde el valor de golpe",
                from: "Despierta a Varrak (acto 1)",
                cooldown: 80.0,
                radius: 30.0,
                duration: 0.0,
                color: 0xe2334a,
                key: "X",
            },
            Power::Bendicion => &PowerSpec {
                name: "Bendición de Nyxara",
                what: "Tus soldados cercanos sanan sus heridas",
                from: "Gánate la confianza de Nyxara",
                cooldown: 75.0,
                radius: 22.0,
                duration: 0.0,
                color: 0x64e0c8,
                key: "C",
            },
            Power::Brasas => &PowerSpec {
                name: "Lluvia de brasas",
                what: "Llueven brasas del cielo sobre el punto durante unos segundos",
                from: "Reliquia: la Brasa del Peñascal",
                cooldown: 65.0,
                radius: 15.0,
                duration: 4.0,
                color: 0xff7a2a,
                key: "V",
            },
            Power::Niebla => &PowerSpec {
                name: "Niebla del sello",
                what: "Una niebla violeta: las flechas enemigas se pierden y el enemigo anda despacio",
                from: "Reliquia: el Cristal de memoria",
                cooldown: 70.0,
                radius: 24.0,
                duration: 20.0,
                color: 0x9a5cff,
                key: "B",
            },
            Power::Raices => &PowerSpec {
                name: "Raíces del bosque",
                what: "Las raíces sujetan los pies del enemigo donde está",
                from: "Reliquia: la Semilla del Refugio",
                cooldown: 60.0,
                radius: 17.0,
                duration: 7.0,
                color: 0x7fbf4a,
                key: "N",
            },
        }
    }

    pub fn index(self) -> usize {
        Power::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }
}

/// A power working on the field.
#[derive(Clone, Copy, Debug)]
pub struct Active {
    pub power: Power,
    pub side: u8,
    pub x: f32,
    pub z: f32,
    pub r: f32,
    /// Seconds it has worked, and how long it works.
    pub t: f32,
    pub dur: f32,
    /// The embers still to fall.
    embers: u32,
    ember_t: f32,
}

/// What happened, for the view.
#[derive(Clone, Copy, Debug)]
pub enum PowerEvent {
    Cast {
        power: Power,
        x: f32,
        z: f32,
        r: f32,
    },
    Ember {
        x: f32,
        z: f32,
        fall: f32,
    },
    Healed {
        x: f32,
        z: f32,
    },
    Shaken {
        x: f32,
        z: f32,
    },
}

pub struct Powers {
    /// The powers she has (in this battle), and each one's wait.
    pub unlocked: Vec<Power>,
    pub cooldown: [f32; 6],
    pub active: Vec<Active>,
    pub events: Vec<PowerEvent>,
    seed: u32,
}

impl Powers {
    pub fn new(unlocked: Vec<Power>) -> Powers {
        Powers {
            unlocked,
            cooldown: [0.0; 6],
            active: Vec::new(),
            events: Vec::new(),
            seed: 0x51_7c_c1_b7,
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed % 10_000) as f32 / 10_000.0
    }

    pub fn has(&self, p: Power) -> bool {
        self.unlocked.contains(&p)
    }

    pub fn ready(&self, p: Power) -> bool {
        self.has(p) && self.cooldown[p.index()] <= 0.0
    }

    /// Casts `p` for `side` on (x, z), if it is hers and ready.
    pub fn cast(&mut self, sim: &mut Sim, side: u8, p: Power, x: f32, z: f32) -> bool {
        if !self.ready(p) {
            return false;
        }
        let sp = p.spec();
        self.cooldown[p.index()] = sp.cooldown;
        self.events.push(PowerEvent::Cast {
            power: p,
            x,
            z,
            r: sp.radius,
        });
        match p {
            Power::Rugido => {
                for r in sim.regiments.iter_mut() {
                    if r.side != side
                        && r.alive > 0
                        && !r.awake
                        && (r.cx - x).hypot(r.cz - z) < sp.radius
                    {
                        r.morale = (r.morale - 40.0).max(0.0);
                        self.events.push(PowerEvent::Shaken { x: r.cx, z: r.cz });
                    }
                }
            }
            Power::Bendicion => {
                for i in sim.soldiers_near(x, z, sp.radius) {
                    let r = sim.reg[i];
                    if sim.regiments[r].side != side {
                        continue;
                    }
                    let max = sim.regiments[r].ty.hp;
                    sim.hp[i] = (sim.hp[i] + max * 0.6).min(max);
                    if i % 3 == 0 {
                        self.events.push(PowerEvent::Healed {
                            x: sim.px[i],
                            z: sim.pz[i],
                        });
                    }
                }
            }
            Power::Grito => {
                for r in sim.regiments.iter_mut() {
                    if r.side == side && r.alive > 0 && (r.cx - x).hypot(r.cz - z) < sp.radius {
                        r.morale = (r.morale + 35.0).min(100.0);
                        r.cohesion = (r.cohesion + 40.0).min(100.0);
                    }
                }
            }
            _ => {}
        }
        if sp.duration > 0.0 {
            self.active.push(Active {
                power: p,
                side,
                x,
                z,
                r: sp.radius,
                t: 0.0,
                dur: sp.duration,
                embers: if p == Power::Brasas { 14 } else { 0 },
                ember_t: 0.0,
            });
        }
        true
    }

    pub fn step(&mut self, sim: &mut Sim, dt: f32) {
        for c in &mut self.cooldown {
            *c = (*c - dt).max(0.0);
        }
        let mut k = 0;
        while k < self.active.len() {
            let mut a = self.active[k];
            a.t += dt;
            if a.power == Power::Brasas && a.embers > 0 {
                a.ember_t -= dt;
                while a.ember_t <= 0.0 && a.embers > 0 {
                    a.ember_t += a.dur / 14.0;
                    a.embers -= 1;
                    let ang = self.rand() * std::f32::consts::TAU;
                    let d = self.rand().sqrt() * a.r;
                    let (ex, ez) = (a.x + ang.cos() * d, a.z + ang.sin() * d);
                    self.events.push(PowerEvent::Ember {
                        x: ex,
                        z: ez,
                        fall: 0.0,
                    });
                    for i in sim.soldiers_near(ex, ez, 4.0) {
                        let r = sim.reg[i];
                        if sim.regiments[r].side == a.side || sim.state[i] != S_ALIVE {
                            continue;
                        }
                        let dd = (sim.px[i] - ex).hypot(sim.pz[i] - ez);
                        let armor = sim.regiments[r].ty.armor;
                        sim.hurt(i, 70.0 * (1.0 - dd / 4.0 * 0.5) * (1.0 - armor / 160.0));
                    }
                    for r in sim.regiments.iter_mut() {
                        if r.side != a.side && (r.cx - ex).hypot(r.cz - ez) < 10.0 {
                            r.morale = (r.morale - 3.0).max(0.0);
                        }
                    }
                }
            }
            self.active[k] = a;
            if a.t >= a.dur {
                self.active.swap_remove(k);
            } else {
                k += 1;
            }
        }
    }

    /// How fast feet go at (x, z) for `side`: the roots and the fog slow the enemy, the cry speeds
    /// hers.
    pub fn speed(&self, x: f32, z: f32, side: u8) -> f32 {
        let mut k = 1.0;
        for a in &self.active {
            if (x - a.x).hypot(z - a.z) > a.r {
                continue;
            }
            match a.power {
                Power::Raices if a.side != side => k *= 0.08,
                Power::Niebla if a.side != side => k *= 0.6,
                Power::Grito if a.side == side => k *= 1.25,
                _ => {}
            }
        }
        k
    }

    /// Whether a shot of `side`'s landing at (x, z), or loosed from inside the fog, is lost in it.
    pub fn fogged(&self, x: f32, z: f32, side: u8) -> bool {
        self.active
            .iter()
            .any(|a| a.power == Power::Niebla && a.side != side && (x - a.x).hypot(z - a.z) < a.r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::{Battle, Setup};

    #[test]
    fn every_power_has_its_numbers_and_works_on_a_field() {
        for p in Power::ALL {
            let s = p.spec();
            assert!(
                s.cooldown > 0.0 && s.radius > 5.0 && !s.key.is_empty(),
                "{}",
                s.name
            );
        }
        let mut b = Battle::new(Setup::scenario("vado", 0.5));
        b.begin();
        let mut pw = Powers::new(Power::ALL.to_vec());
        let foe = b.sim.regiments.iter().find(|r| r.side == 1).unwrap();
        let (fx, fz) = (foe.cx, foe.cz);
        let men: u32 = b
            .sim
            .regiments
            .iter()
            .filter(|r| r.side == 1)
            .map(|r| r.alive)
            .sum();
        b.sim.step();
        assert!(pw.cast(&mut b.sim, 0, Power::Brasas, fx, fz));
        assert!(
            !pw.cast(&mut b.sim, 0, Power::Brasas, fx, fz),
            "it waits its cooldown"
        );
        b.sim.powers = Some(pw);
        for _ in 0..150 {
            b.sim.step();
        }
        let after: u32 = b
            .sim
            .regiments
            .iter()
            .filter(|r| r.side == 1)
            .map(|r| r.alive)
            .sum();
        assert!(after < men, "the embers hurt the enemy ({men} → {after})");
        let mut pw = b.sim.powers.take().unwrap();
        assert!(pw.cast(&mut b.sim, 0, Power::Raices, fx, fz));
        assert!(pw.speed(fx, fz, 1) < 0.2 && pw.speed(fx, fz, 0) == 1.0);
    }
}
