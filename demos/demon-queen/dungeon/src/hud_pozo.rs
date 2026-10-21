//! The HUD down the miner's shaft (her depth and record, her bag, her pick, the rock in front of her
//! and how much of it is left, what the shaft just gave her, the keys). The Tendero's stall is the
//! game's shop window (the host draws it).
use super::glass::{self, diamond, Tone, IVORY};
use super::{rect, Dungeon, Hud, Vertex};
use crate::pozo::{self, Pozo};

impl Hud {
    /// Cards of the shaft, instead of the mine's.
    pub(super) fn pozo_hud(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let Some(pozo) = run.pozo.as_deref() else {
            return;
        };
        let amber = [1.0, 0.72, 0.3, 1.0];
        // where she is: depth, record, the rock
        let depth = Pozo::depth_of(run.controller.position);
        let rock = pozo::Rock::at(depth as i32, 0);
        let (x, y) = (40.0, 40.0);
        glass::plate(v, [x, y, 520.0, 112.0], amber, 1.0);
        self.kicker(v, "EL POZO DEL MINERO", x + 28.0, y - 10.0, amber);
        self.mid(v, &format!("{depth:.0} m"), x + 30.0, y + 48.0, 40.0, IVORY);
        self.mid(
            v,
            &format!("{} · marca {} m", rock.name(), (pozo.deepest + 1).max(0)),
            x + 30.0,
            y + 88.0,
            18.0,
            glass::mix(IVORY, amber, 0.4),
        );
        // the bag
        let carried = pozo::carried(&run.materials);
        let bag = run.gear.bag();
        let (bx, by, bw) = (1920.0 - 40.0 - 460.0, 40.0, 460.0);
        let full = carried >= bag.capacity;
        let edge = if full { [1.0, 0.3, 0.25, 1.0] } else { amber };
        glass::plate(v, [bx, by, bw, 112.0], edge, 1.0);
        self.kicker(v, &bag.name.to_uppercase(), bx + 28.0, by - 10.0, edge);
        self.mid(
            v,
            &format!("{carried} / {}", bag.capacity),
            bx + 30.0,
            by + 46.0,
            30.0,
            IVORY,
        );
        self.right(
            v,
            &format!("{} oro", run.gold + run.purse),
            bx + bw - 30.0,
            by + 46.0,
            22.0,
            [1.0, 0.84, 0.4, 1.0],
        );
        glass::meter(
            v,
            [bx + 30.0, by + 74.0, bw - 60.0, 16.0],
            (carried as f32 / bag.capacity.max(1) as f32).min(1.0),
            [0.4, 0.26, 0.08, 1.0],
            if full {
                [1.0, 0.3, 0.25, 1.0]
            } else {
                [1.0, 0.76, 0.3, 1.0]
            },
        );
        // the pick
        let pick = run.gear.pick();
        let (px, py) = (40.0, 1080.0 - 40.0 - 96.0);
        let metal = [pick.color[0], pick.color[1], pick.color[2], 1.0];
        glass::plate(v, [px, py, 470.0, 96.0], glass::mix(metal, amber, 0.4), 1.0);
        self.kicker(
            v,
            "TU PICO",
            px + 28.0,
            py - 10.0,
            glass::mix(metal, amber, 0.4),
        );
        self.mid(v, pick.name, px + 30.0, py + 40.0, 26.0, IVORY);
        self.mid(
            v,
            &format!("Golpe {} · {:.1} por segundo", pick.damage, pick.rate),
            px + 30.0,
            py + 72.0,
            17.0,
            glass::mix(IVORY, amber, 0.35),
        );
        // the rock in front of her: its name and what is left of it
        if let Some(b) = pozo.target {
            let (cx, cy) = (960.0, 610.0);
            if pozo.diggable(b) {
                let block = pozo.block(b[0], b[1], b[2]);
                let wear = pozo.wear(b);
                let hard = pick.tier < block.rock.tier();
                let label = match block.find {
                    Some(pozo::Find::Ore(i)) => {
                        format!("{} · {}", block.rock.name(), pozo::ORES[i].name)
                    }
                    _ => block.rock.name().to_string(),
                };
                self.centered(
                    v,
                    &label,
                    cx,
                    cy,
                    19.0,
                    if hard { [1.0, 0.45, 0.4, 1.0] } else { IVORY },
                );
                if wear > 0.0 {
                    glass::meter(
                        v,
                        [cx - 90.0, cy + 18.0, 180.0, 10.0],
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
        let (kx, ky) = (1920.0 - 40.0 - 640.0, 1080.0 - 40.0 - 44.0);
        let mut x = kx;
        for (key, label) in [
            ("CLIC", "Picar"),
            ("U", "Subir"),
            ("ESPACIO", "Saltar"),
            ("T", "Lámpara"),
        ] {
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
