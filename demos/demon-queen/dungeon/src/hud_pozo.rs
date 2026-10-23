//! The HUD down the miner's shaft: her depth, record and the veins that run there; Rosalía's
//! errand when she works one (its quota, her wagon, the blocks and pieces sent up, the time, and
//! how to get out); her bag and what it is worth; her pick and the hardest rock it bites; the rock
//! in front of her and how much of it is left; what the shaft just gave her; the keys. The
//! Tendero's stall is the game's shop window (the host draws it).
use super::glass::{self, diamond, Tone, IVORY};
use super::{rect, Dungeon, Hud, Vertex};
use crate::pozo::{self, Pozo};

const AMBER: [f32; 4] = [1.0, 0.72, 0.3, 1.0];
const ROSE: [f32; 4] = [0.95, 0.32, 0.55, 1.0];
const GOLD: [f32; 4] = [1.0, 0.84, 0.4, 1.0];
const ALARM: [f32; 4] = [1.0, 0.3, 0.25, 1.0];

impl Hud {
    /// Cards of the shaft, instead of the mine's.
    pub(super) fn pozo_hud(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let Some(pozo) = run.pozo.as_deref() else {
            return;
        };
        let soft = glass::mix(IVORY, AMBER, 0.4);
        // where she is: depth, record, the rock, the veins that run here
        let depth = Pozo::depth_of(run.controller.position);
        let rock = pozo::Rock::at(depth as i32, 0);
        let (x, y) = (40.0, 40.0);
        glass::plate(v, [x, y, 500.0, 136.0], AMBER, 1.0);
        self.kicker(v, "EL POZO DEL MINERO", x + 28.0, y - 10.0, AMBER);
        self.mid(v, &format!("{depth:.0} m"), x + 30.0, y + 46.0, 40.0, IVORY);
        self.right(
            v,
            &format!("marca {} m", (pozo.deepest + 1).max(0)),
            x + 300.0,
            y + 46.0,
            19.0,
            soft,
        );
        self.mid(v, rock.name(), x + 30.0, y + 84.0, 19.0, soft);
        let k = depth.max(0.0) as i32;
        let veins: Vec<&pozo::Ore> = pozo::ORES
            .iter()
            .filter(|o| (o.from as i32) <= k && k <= o.peak as i32 + 12)
            .collect();
        let mut vx = x + 30.0;
        if veins.is_empty() {
            self.aside(
                v,
                "Sin vetas a esta altura",
                vx,
                y + 114.0,
                16.0,
                300.0,
                soft,
            );
        } else {
            self.mid(v, "Vetas:", vx, y + 114.0, 15.0, soft);
            vx += self.width("Vetas:", 15.0) + 12.0;
            for o in veins.iter().take(4) {
                let c = [
                    o.color[0].max(0.35),
                    o.color[1].max(0.35),
                    o.color[2].max(0.35),
                    1.0,
                ];
                diamond(v, [vx + 5.0, y + 114.0], 5.0, c);
                self.mid(v, o.name, vx + 15.0, y + 114.0, 15.0, IVORY);
                vx += self.width(o.name, 15.0) + 32.0;
            }
        }

        // Rosalía's errand
        if let Some(errand) = run.errand.as_ref().filter(|e| e.outcome.is_none()) {
            let (ex, ey, ew, eh) = (40.0, 214.0, 500.0, 210.0);
            let met = errand.met();
            let edge = if met { GOLD } else { ROSE };
            glass::plate(v, [ex, ey, ew, eh], edge, 1.0);
            self.kicker(
                v,
                if errand.debt {
                    "ENCARGO DE ROSALÍA · DEUDA"
                } else {
                    "ENCARGO DE ROSALÍA"
                },
                ex + 28.0,
                ey - 10.0,
                edge,
            );
            self.mid(
                v,
                &format!("{} / {} oro", errand.done.min(errand.quota), errand.quota),
                ex + 30.0,
                ey + 46.0,
                34.0,
                IVORY,
            );
            self.right(
                v,
                &format!("{:.0} %", errand.share() * 100.0),
                ex + ew - 30.0,
                ey + 46.0,
                24.0,
                edge,
            );
            glass::meter(
                v,
                [ex + 30.0, ey + 76.0, ew - 60.0, 18.0],
                errand.share(),
                [0.35, 0.08, 0.16, 1.0],
                edge,
            );
            let secs = (run.elapsed - errand.since).max(0.0) as u32;
            self.mid(
                v,
                &format!(
                    "Bloques {} · Mineral {} piezas · {:02}:{:02}",
                    errand.blocks,
                    errand.pieces,
                    secs / 60,
                    secs % 60
                ),
                ex + 30.0,
                ey + 116.0,
                17.0,
                glass::mix(IVORY, edge, 0.3),
            );
            self.aside(
                v,
                if errand.pay > 0 {
                    "Mineral a su precio, roca a 1 · paga al entregar"
                } else {
                    "Mineral a su precio, roca a 1 · salda tu deuda"
                },
                ex + 30.0,
                ey + 144.0,
                16.0,
                ew - 60.0,
                glass::mix(IVORY, edge, 0.2),
            );
            let (line, color) = if met {
                (
                    if errand.pay > 0 {
                        format!("¡Cumplido! Sal por la jaula (F) y cobra {} oro", errand.pay)
                    } else {
                        "¡Cumplido! Sal por la jaula (F) para entregarlo".to_owned()
                    },
                    GOLD,
                )
            } else if errand.arming(run.elapsed) {
                (
                    if errand.debt {
                        "C otra vez: abandonas y te vetan la casa".to_owned()
                    } else {
                        "C otra vez: abandonas sin cobrar".to_owned()
                    },
                    ALARM,
                )
            } else {
                (
                    "La jaula no sube hasta cumplirlo · C dos veces lo abandona".to_owned(),
                    glass::mix(IVORY, ROSE, 0.5),
                )
            };
            self.mid_fit(v, &line, ex + 30.0, ey + 178.0, 18.0, ew - 60.0, color);
        }

        // the bag, its worth at the village, and her gold
        let carried = pozo::carried(&run.materials);
        let worth: u32 = run
            .materials
            .iter()
            .filter_map(|(id, n)| pozo::value(id).map(|v| v * n))
            .sum();
        let bag = run.gear.bag();
        let (bx, by, bw) = (1920.0 - 40.0 - 480.0, 40.0, 480.0);
        let full = carried >= bag.capacity;
        let edge = if full { ALARM } else { AMBER };
        glass::plate(v, [bx, by, bw, 136.0], edge, 1.0);
        self.kicker(v, &bag.name.to_uppercase(), bx + 28.0, by - 10.0, edge);
        self.mid(
            v,
            &format!("{carried} / {}", bag.capacity),
            bx + 30.0,
            by + 44.0,
            30.0,
            IVORY,
        );
        self.right(
            v,
            &format!("{} oro", run.gold + run.purse),
            bx + bw - 30.0,
            by + 44.0,
            22.0,
            GOLD,
        );
        glass::meter(
            v,
            [bx + 30.0, by + 72.0, bw - 60.0, 16.0],
            (carried as f32 / bag.capacity.max(1) as f32).min(1.0),
            [0.4, 0.26, 0.08, 1.0],
            if full { ALARM } else { [1.0, 0.76, 0.3, 1.0] },
        );
        self.mid(
            v,
            &if full {
                "Lleno: sube (U) y véndelo al Tendero".to_owned()
            } else {
                format!("En el saco: unos {worth} oro en el pueblo")
            },
            bx + 30.0,
            by + 110.0,
            16.0,
            if full { ALARM } else { soft },
        );

        // the pick, and the hardest rock it bites
        let pick = run.gear.pick();
        let (px, py) = (40.0, 1080.0 - 40.0 - 120.0);
        let metal = [pick.color[0], pick.color[1], pick.color[2], 1.0];
        let rim = glass::mix(metal, AMBER, 0.4);
        glass::plate(v, [px, py, 500.0, 120.0], rim, 1.0);
        self.kicker(v, "TU PICO", px + 28.0, py - 10.0, rim);
        self.mid(v, pick.name, px + 30.0, py + 38.0, 26.0, IVORY);
        self.mid(
            v,
            &format!("Golpe {} · {:.1} por segundo", pick.damage, pick.rate),
            px + 30.0,
            py + 70.0,
            17.0,
            glass::mix(IVORY, AMBER, 0.35),
        );
        let hardest = pozo::Rock::ALL
            .iter()
            .filter(|r| r.tier() <= pick.tier)
            .last()
            .map_or("Tierra", |r| r.name());
        let bites = rock.tier() <= pick.tier;
        self.mid(
            v,
            &format!(
                "Muerde hasta {hardest}{}",
                if bites { "" } else { " · ¡esta roca no!" }
            ),
            px + 30.0,
            py + 98.0,
            16.0,
            if bites { soft } else { ALARM },
        );

        // the rock in front of her: its name, what it holds and how much of it is left
        if let Some(b) = pozo.target {
            let (cx, cy) = (960.0, 610.0);
            if pozo.diggable(b) {
                let block = pozo.block(b[0], b[1], b[2]);
                let wear = pozo.wear(b);
                let hard = pick.tier < block.rock.tier();
                let label = match block.find {
                    Some(pozo::Find::Ore(i)) => format!(
                        "{} · {} ({} oro)",
                        block.rock.name(),
                        pozo::ORES[i].name,
                        pozo::ORES[i].value
                    ),
                    _ => block.rock.name().to_string(),
                };
                let w = self.width(&label, 19.0) + 40.0;
                glass::chip(
                    v,
                    [cx - w * 0.5, cy - 17.0, w, 34.0],
                    if hard { ALARM } else { AMBER },
                    0.5,
                );
                self.centered(
                    v,
                    &label,
                    cx,
                    cy,
                    19.0,
                    if hard { [1.0, 0.55, 0.5, 1.0] } else { IVORY },
                );
                if wear > 0.0 {
                    glass::meter(
                        v,
                        [cx - 100.0, cy + 26.0, 200.0, 10.0],
                        1.0 - wear,
                        [0.3, 0.2, 0.1, 1.0],
                        [1.0, 0.8, 0.4, 1.0],
                    );
                }
            } else {
                self.centered(v, "Roca madre: no cede", cx, cy, 18.0, [0.9, 0.5, 0.4, 1.0]);
            }
        }

        // what the shaft just gave her
        let mut ny = 300.0;
        for (label, color, at) in run.mine_notes.iter().rev().take(6) {
            let age = run.elapsed - at;
            let fade = (1.0 - (age - 2.6).max(0.0)).clamp(0.0, 1.0);
            let c = [
                color[0].max(0.35),
                color[1].max(0.35),
                color[2].max(0.35),
                fade,
            ];
            let w = self.width(label, 22.0) + 60.0;
            let x = 1920.0 - 40.0 - w - (age * 40.0).min(0.0);
            glass::chip(v, [x, ny, w, 40.0], c, 0.22 * fade);
            diamond(v, [x + 20.0, ny + 20.0], 5.0, c);
            self.mid(
                v,
                label,
                x + 36.0,
                ny + 20.0,
                22.0,
                glass::alpha(IVORY, fade),
            );
            ny += 50.0;
        }

        // the keys
        let errand_open = run
            .errand
            .as_ref()
            .is_some_and(|e| e.outcome.is_none() && !e.met());
        let mut keys = vec![
            ("CLIC", "Picar"),
            ("U", "Subir"),
            ("ESPACIO", "Saltar"),
            ("T", "Lámpara"),
        ];
        if errand_open {
            keys.push(("C ×2", "Abandonar"));
        }
        let total: f32 = keys
            .iter()
            .map(|(k, l)| (self.width(k, 14.0) + 22.0).max(32.0) + 8.0 + self.width(l, 16.0) + 22.0)
            .sum();
        let ky = 1080.0 - 40.0 - 44.0;
        let mut x = 1920.0 - 40.0 - total;
        for (key, label) in keys {
            x += self.key(v, key, x, ky + 8.0) + 8.0;
            self.mid(v, label, x, ky + 22.0, 16.0, IVORY);
            x += self.width(label, 16.0) + 22.0;
        }
        // the winch hauling her up
        if let Some(left) = pozo.hauling {
            let k = 1.0 - left / pozo::HAUL_SECONDS;
            rect(
                v,
                [0.0, 0.0, 1920.0, 1080.0],
                [0.0, 0.0, 0.0, (k * 1.4).min(0.85)],
            );
            self.centered(
                v,
                "El torno te sube…",
                960.0,
                540.0,
                30.0,
                glass::alpha(IVORY, (k * 3.0).min(1.0)),
            );
        }
        let _ = tone;
    }
}
