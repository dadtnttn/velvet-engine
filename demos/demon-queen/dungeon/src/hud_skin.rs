//! Expedition chrome: native 1920x1080, one vector batch, shared glyph atlas.
//! Glass panes tinted by the biome the Queen stands in; icons come from the
//! same set as the world's F prompts. All values remain live.
use super::{rect, triangle, Dungeon, Hud, RenderStats, Vertex};
use std::f32::consts::{FRAC_PI_2, TAU};
#[path = "hud_glass.rs"]
mod glass;
#[path = "hud_map.rs"]
mod map;
#[path = "hud_map3d.rs"]
mod map3d;
use super::super::prompt_icons::{self, Icon, Prompt};
use glass::{Rgba, Tone, CRIMSON, GREEN, IVORY, MUTED};

const GOLD: [f32; 4] = [0.93, 0.57, 0.16, 1.0];
const PALE: [f32; 4] = [1.0, 0.79, 0.42, 1.0];
const MAP_MUTED: [f32; 4] = [0.57, 0.70, 0.68, 1.0];

// Coverage fringe keeps fine gilding smooth on the single-sample UI target.
fn line(v: &mut Vec<Vertex>, a: [f32; 2], b: [f32; 2], width: f32, color: [f32; 4]) {
    glass::segment(v, a, b, width, color);
}

fn polygon(v: &mut Vec<Vertex>, points: &[[f32; 2]], c: [f32; 4]) {
    if points.len() < 3 {
        return;
    }
    for i in 1..points.len() - 1 {
        triangle(v, points[0], points[i], points[i + 1], c);
    }
}
fn ring(v: &mut Vec<Vertex>, p: [f32; 2], r: f32, width: f32, c: [f32; 4]) {
    glass::ring(v, p, r, width, c);
}
fn disc(v: &mut Vec<Vertex>, p: [f32; 2], r: f32, c: [f32; 4]) {
    glass::disc(v, p, r, c);
}
fn diamond(v: &mut Vec<Vertex>, p: [f32; 2], r: f32, c: [f32; 4]) {
    glass::diamond(v, p, r, c);
}
fn coin(v: &mut Vec<Vertex>, p: [f32; 2]) {
    disc(v, [p[0] + 0.8, p[1] + 1.2], 10.5, [0.0, 0.0, 0.0, 0.35]);
    disc(v, p, 10.0, [1.0, 0.48, 0.025, 1.0]);
    ring(v, p, 8.6, 1.2, PALE);
    ring(v, p, 6.2, 0.9, [0.61, 0.27, 0.01, 1.0]);
    diamond(v, p, 3.8, PALE);
}
fn potion(v: &mut Vec<Vertex>, p: [f32; 2]) {
    let at = |x: f32, y: f32| [p[0] + x * 0.85, p[1] + y * 0.85];
    let outline = [
        at(-4.0, -12.0),
        at(4.0, -12.0),
        at(4.0, -5.0),
        at(9.0, 5.0),
        at(7.0, 12.0),
        at(0.0, 15.0),
        at(-7.0, 12.0),
        at(-9.0, 5.0),
        at(-4.0, -5.0),
    ];
    polygon(v, &outline, [0.62, 0.02, 0.012, 1.0]);
    glass::stroke(v, &outline, true, 1.2, PALE);
    rect(v, [p[0] - 4.4, p[1] - 15.0, 8.8, 3.4], GOLD);
    line(v, at(-4.5, 2.0), at(-3.0, 8.0), 2.4, [1.0, 0.34, 0.19, 1.0]);
}
fn leaf(v: &mut Vec<Vertex>, p: [f32; 2]) {
    let at = |x: f32, y: f32| [p[0] + x, p[1] + y];
    let blade = [
        at(-9.0, 7.0),
        at(-6.0, -3.0),
        at(1.0, -9.0),
        at(9.0, -9.0),
        at(8.0, -1.0),
        at(2.0, 6.0),
    ];
    polygon(v, &blade, [0.30, 0.68, 0.30, 1.0]);
    glass::stroke(v, &blade, true, 1.0, [0.62, 0.92, 0.5, 1.0]);
    line(
        v,
        at(-9.0, 9.0),
        at(4.0, -4.0),
        1.4,
        [0.10, 0.32, 0.12, 1.0],
    );
}

impl Hud {
    fn width(&self, text: &str, size: f32) -> f32 {
        text.chars()
            .filter_map(|c| self.glyphs.get(&c))
            .map(|g| g.advance * size / 38.0)
            .sum()
    }
    fn fitted(
        &self,
        v: &mut Vec<Vertex>,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        max: f32,
        color: [f32; 4],
    ) {
        let size = size * (max / self.width(text, size).max(1.0)).min(1.0);
        self.text(v, text, x, y, size, color);
    }
    /// Text whose visual middle sits on `cy`.
    fn mid(&self, v: &mut Vec<Vertex>, text: &str, x: f32, cy: f32, size: f32, color: Rgba) {
        self.text(v, text, x, cy - size * 0.58, size, color);
    }
    fn mid_fit(
        &self,
        v: &mut Vec<Vertex>,
        text: &str,
        x: f32,
        cy: f32,
        size: f32,
        max: f32,
        color: Rgba,
    ) {
        let size = size * (max / self.width(text, size).max(1.0)).min(1.0);
        self.mid(v, text, x, cy, size, color);
    }
    fn centered(&self, v: &mut Vec<Vertex>, text: &str, cx: f32, cy: f32, size: f32, color: Rgba) {
        self.mid(v, text, cx - self.width(text, size) * 0.5, cy, size, color);
    }
    fn right(&self, v: &mut Vec<Vertex>, text: &str, right: f32, cy: f32, size: f32, color: Rgba) {
        self.mid(v, text, right - self.width(text, size), cy, size, color);
    }
    /// A key cap with its legend; returns its width.
    fn key(&self, v: &mut Vec<Vertex>, label: &str, x: f32, y: f32) -> f32 {
        let size = 15.0;
        let w = (self.width(label, size) + 18.0).max(28.0);
        let h = 28.0;
        glass::round(
            v,
            [x, y + 3.0, w, h - 3.0],
            7.0,
            [0.0, 0.0, 0.0, 0.85],
            [0.0, 0.0, 0.0, 0.85],
        );
        glass::round(
            v,
            [x, y, w, h - 3.0],
            7.0,
            [0.075, 0.08, 0.095, 0.97],
            [0.03, 0.032, 0.04, 0.97],
        );
        glass::round_outline(v, [x, y, w, h - 1.0], 7.0, 1.0, [1.0, 1.0, 1.0, 0.20]);
        self.centered(v, label, x + w * 0.5, y + (h - 3.0) * 0.5, size, IVORY);
        w
    }
    /// A pill with a leading label; returns its width.
    fn tag(&self, v: &mut Vec<Vertex>, text: &str, x: f32, y: f32, color: Rgba) -> f32 {
        let size = 15.0;
        let w = self.width(text, size) + 30.0;
        glass::chip(v, [x, y, w, 28.0], color, 0.16);
        disc(v, [x + 13.0, y + 14.0], 3.0, color);
        self.mid(
            v,
            text,
            x + 24.0,
            y + 14.0,
            size,
            glass::mix(color, [1.0, 1.0, 1.0, 1.0], 0.5),
        );
        w
    }
    /// The F prompt: key, the interaction's icon in its colour, the action
    /// and what it acts on, on a glass plate.
    fn action_prompt(&self, v: &mut Vec<Vertex>, prompt: &Prompt, tone: &Tone) {
        let a = prompt.icon.accent();
        let accent = [a[0].min(1.0), a[1].min(1.0), a[2].min(1.0), 1.0];
        let (verb_size, detail_size) = (27.0, 17.0);
        let detail = prompt.detail.as_deref().unwrap_or("");
        let key = if prompt.keyed { 62.0 } else { 0.0 };
        let text_w = self
            .width(&prompt.verb, verb_size)
            .max(self.width(detail, detail_size));
        let width = (28.0 + key + 64.0 + 18.0 + text_w + 34.0).clamp(360.0, 1000.0);
        let (x, y, h) = ((1920.0 - width) * 0.5, 770.0, 84.0);
        glass::pane(v, [x, y, width, h], 28.0, tone, 1.0);
        // A lit edge of the interaction's colour along the top.
        line(
            v,
            [x + 34.0, y + 2.5],
            [x + width - 34.0, y + 2.5],
            1.8,
            glass::alpha(accent, 0.7),
        );
        let mid = y + h * 0.5;
        let mut cx = x + 26.0;
        if prompt.keyed {
            let size = 48.0;
            let top = mid - size * 0.5;
            glass::round(
                v,
                [cx, top + 4.0, size, size - 2.0],
                12.0,
                [0.0, 0.0, 0.0, 0.8],
                [0.0, 0.0, 0.0, 0.8],
            );
            glass::round(
                v,
                [cx, top, size, size - 3.0],
                12.0,
                [1.0, 0.86, 0.5, 1.0],
                [0.86, 0.6, 0.24, 1.0],
            );
            glass::round(
                v,
                [cx + 3.5, top + 3.0, size - 7.0, size - 10.0],
                9.0,
                [0.03, 0.02, 0.05, 1.0],
                [0.05, 0.03, 0.07, 1.0],
            );
            self.centered(
                v,
                "F",
                cx + size * 0.5,
                top + (size - 3.0) * 0.5,
                27.0,
                PALE,
            );
            cx += size + 14.0;
            line(
                v,
                [cx, y + 20.0],
                [cx, y + h - 20.0],
                1.0,
                glass::alpha(accent, 0.35),
            );
            cx += 14.0;
        }
        glass::icon_disc(v, prompt.icon, [cx + 30.0, mid], 30.0, accent, true);
        let tx = cx + 30.0 + 30.0 + 18.0;
        let room = x + width - 26.0 - tx;
        if detail.is_empty() {
            self.mid_fit(v, &prompt.verb, tx, mid, verb_size, room, IVORY);
        } else {
            self.mid_fit(v, &prompt.verb, tx, mid - 14.0, verb_size, room, IVORY);
            let soft = [
                0.55 + accent[0] * 0.4,
                0.55 + accent[1] * 0.4,
                0.55 + accent[2] * 0.4,
                0.95,
            ];
            self.mid_fit(v, detail, tx, mid + 17.0, detail_size, room, soft);
        }
    }

    /// The glass compass: its needle keeps pointing at the world's north.
    fn compass(&self, v: &mut Vec<Vertex>, c: [f32; 2], yaw: f32, tone: &Tone) {
        disc(v, [c[0] + 1.0, c[1] + 2.5], 37.0, [0.0, 0.0, 0.0, 0.3]);
        disc(v, c, 36.0, [0.004, 0.009, 0.013, 0.94]);
        ring(v, c, 35.0, 1.6, glass::alpha(tone.accent, 0.85));
        for i in 0..16 {
            let a = i as f32 * TAU / 16.0 - yaw;
            let (r0, r1) = (if i % 4 == 0 { 26.5 } else { 30.0 }, 33.0);
            line(
                v,
                [c[0] + a.cos() * r0, c[1] + a.sin() * r0],
                [c[0] + a.cos() * r1, c[1] + a.sin() * r1],
                if i % 4 == 0 { 1.5 } else { 0.9 },
                glass::alpha(tone.bright, if i % 4 == 0 { 0.9 } else { 0.4 }),
            );
        }
        let north = -FRAC_PI_2 - yaw;
        let (dx, dy) = (north.cos(), north.sin());
        let at = |r: f32, side: f32| [c[0] + dx * r - dy * side, c[1] + dy * r + dx * side];
        triangle(v, at(24.0, 0.0), at(0.0, 6.5), at(0.0, -6.5), CRIMSON);
        triangle(
            v,
            at(-19.0, 0.0),
            at(0.0, 6.5),
            at(0.0, -6.5),
            [0.52, 0.55, 0.52, 1.0],
        );
        disc(v, c, 3.2, tone.bright);
    }

    /// Top left: where we are, what the place is, how high up.
    fn location_plate(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let room = &run.map.rooms[run.map.region_at(run.controller.position)];
        let biome = &run.catalog.biomes[room.biome];
        glass::pane(v, [36.0, 36.0, 600.0, 116.0], 26.0, tone, 1.0);
        self.compass(v, [100.0, 94.0], run.controller.yaw, tone);
        self.text(
            v,
            "LAS CAVERNAS DEL SELLO",
            154.0,
            49.0,
            14.0,
            glass::alpha(tone.accent, 0.95),
        );
        let room_label = if run
            .map
            .spawns
            .iter()
            .filter(|spawn| spawn.room == room.id && spawn.definition == "bone_scout")
            .count()
            >= 2
        {
            "Acechadores / emboscada"
        } else {
            super::super::themes::of(&run.map, room)
                .map(|theme| theme.title())
                .unwrap_or_else(|| room.role.label())
        };
        self.mid_fit(v, room_label, 154.0, 86.0, 31.0, 410.0, IVORY);
        let subtitle = if biome.id == "forgotten_masonry" {
            "Vestíbulos olvidados"
        } else {
            &biome.name
        };
        let mut x = 154.0;
        x += self.tag(v, subtitle, x, 104.0, tone.accent) + 8.0;
        let y = run.controller.position.y;
        let label = format!("ALT {:+.0} m", y);
        let w = self.width(&label, 15.0) + 30.0;
        let alt = glass::altitude_color(self.altitude_fraction(run, y));
        glass::chip(v, [x, 104.0, w, 28.0], alt, 0.22);
        glass::caret(
            v,
            [x + 13.0, 118.0],
            4.2,
            if y >= 0.0 { -1.0 } else { 1.0 },
            glass::mix(alt, [1.0, 1.0, 1.0, 1.0], 0.4),
        );
        self.mid(
            v,
            &label,
            x + 24.0,
            118.0,
            15.0,
            glass::mix(alt, [1.0, 1.0, 1.0, 1.0], 0.55),
        );
        // The altimeter: every floor of the complex, and where we stand.
        let gauge = [612.0, 50.0, 8.0, 88.0];
        glass::altitude_gauge(v, gauge);
        let (low, high) = self.altitude_range(run);
        for room in &run.map.rooms {
            let f = ((room.center.y - low) / (high - low).max(1.0)).clamp(0.0, 1.0);
            let ty = gauge[1] + gauge[3] * (1.0 - f);
            line(
                v,
                [gauge[0] - 6.0, ty],
                [gauge[0] - 1.5, ty],
                1.0,
                [1.0, 1.0, 1.0, 0.35],
            );
        }
        let f = self.altitude_fraction(run, y);
        let my = gauge[1] + gauge[3] * (1.0 - f);
        triangle(
            v,
            [gauge[0] - 3.0, my],
            [gauge[0] - 13.0, my - 5.5],
            [gauge[0] - 13.0, my + 5.5],
            tone.bright,
        );
        disc(v, [gauge[0] + 4.0, my], 5.2, [1.0, 1.0, 1.0, 1.0]);
        ring(v, [gauge[0] + 4.0, my], 5.2, 1.4, [0.0, 0.0, 0.0, 0.7]);
    }

    fn altitude_range(&self, run: &Dungeon) -> (f32, f32) {
        let low = run
            .map
            .rooms
            .iter()
            .map(|r| r.center.y)
            .fold(f32::INFINITY, f32::min);
        let high = run
            .map
            .rooms
            .iter()
            .map(|r| r.center.y)
            .fold(f32::NEG_INFINITY, f32::max);
        if low.is_finite() && high.is_finite() {
            (low - 0.5, (high + 0.5).max(low + 4.0))
        } else {
            (-2.0, 2.0)
        }
    }
    fn altitude_fraction(&self, run: &Dungeon, y: f32) -> f32 {
        let (low, high) = self.altitude_range(run);
        ((y - low) / (high - low).max(1.0)).clamp(0.0, 1.0)
    }

    /// Top right: the seal and the key, and the next thing to do.
    fn objective_card(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let r = [1436.0, 36.0, 448.0, 136.0];
        glass::pane(v, r, 26.0, tone, 1.0);
        for (i, (icon, label, done, yes, no)) in [
            (Icon::Rune, "SELLO", run.seal, "Alineado", "Sin alinear"),
            (Icon::Lock, "LLAVE", run.crest, "En tu poder", "Pendiente"),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 1456.0 + i as f32 * 212.0;
            let color = if done {
                GREEN
            } else {
                glass::mix(tone.accent, MUTED, 0.35)
            };
            glass::chip(
                v,
                [x, 54.0, 196.0, 48.0],
                color,
                if done { 0.20 } else { 0.10 },
            );
            let badge = if done {
                color
            } else {
                glass::lift(color, 0.85)
            };
            glass::icon_disc(v, icon, [x + 27.0, 78.0], 18.0, badge, done);
            self.text(v, label, x + 54.0, 60.0, 12.0, glass::alpha(color, 0.95));
            self.mid_fit(
                v,
                if done { yes } else { no },
                x + 54.0,
                88.0,
                17.0,
                136.0,
                if done { IVORY } else { MUTED },
            );
            if done {
                // A check mark in the corner.
                let c = [x + 176.0, 78.0];
                line(v, [c[0] - 5.0, c[1]], [c[0] - 1.5, c[1] + 4.0], 2.2, GREEN);
                line(
                    v,
                    [c[0] - 1.5, c[1] + 4.0],
                    [c[0] + 6.0, c[1] - 5.0],
                    2.2,
                    GREEN,
                );
            }
        }
        line(
            v,
            [1456.0, 116.0],
            [1864.0, 116.0],
            1.0,
            glass::alpha(tone.accent, 0.28),
        );
        let aligned = (0..3).filter(|i| run.sigils[*i] as usize == i + 1).count();
        let objective = if !run.seal {
            "Alinea las runas del sello".to_string()
        } else if !run.crest {
            "Derrota al Custodio y toma su llave".to_string()
        } else if !run.won {
            "Abre el umbral y derrota al Corazón".to_string()
        } else {
            "Regresa al altar de salida".to_string()
        };
        diamond(v, [1463.0, 134.0], 5.0, tone.accent);
        self.text(
            v,
            "OBJETIVO",
            1476.0,
            124.0,
            12.0,
            glass::alpha(tone.accent, 0.95),
        );
        let room = if !run.seal { 300.0 } else { 388.0 };
        self.mid_fit(v, &objective, 1456.0, 155.0, 20.0, room, IVORY);
        if !run.seal {
            for k in 0..3 {
                let lit = k < aligned;
                let c = [1794.0 + k as f32 * 24.0, 155.0];
                disc(
                    v,
                    c,
                    8.0,
                    if lit {
                        glass::alpha(tone.bright, 1.0)
                    } else {
                        [0.02, 0.03, 0.04, 0.9]
                    },
                );
                ring(
                    v,
                    c,
                    8.0,
                    1.4,
                    glass::alpha(tone.accent, if lit { 1.0 } else { 0.55 }),
                );
            }
        }
    }

    /// Bottom left: health, tonics, herbs and gold.
    fn vitality_card(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let r = [36.0, 880.0, 484.0, 164.0];
        let fraction = (run.health / 100.0).clamp(0.0, 1.0);
        let low = fraction < 0.3;
        let pulse = 0.5 + 0.5 * (run.elapsed * 6.0).sin();
        glass::pane(v, r, 26.0, tone, 1.0);
        if low {
            glass::round_outline(v, r, 26.0, 2.0, glass::alpha(CRIMSON, 0.35 + 0.4 * pulse));
        }
        glass::icon_disc(
            v,
            Icon::Heart,
            [84.0, 924.0],
            27.0,
            [1.0, 0.30, 0.36, 1.0],
            low,
        );
        self.text(
            v,
            "VITALIDAD",
            126.0,
            902.0,
            14.0,
            glass::alpha(tone.accent, 0.95),
        );
        let value = format!("{:.0}", run.health.max(0.0).ceil());
        self.mid(v, &value, 126.0, 934.0, 34.0, IVORY);
        let vw = self.width(&value, 34.0);
        self.mid(v, "/ 100", 126.0 + vw + 8.0, 938.0, 18.0, MUTED);
        // The bar, with quarter marks.
        let bar = [60.0, 968.0, 428.0, 18.0];
        let (dark, bright) = if low {
            (
                [0.55, 0.02, 0.02, 1.0],
                [1.0, 0.25 + 0.2 * pulse, 0.12, 1.0],
            )
        } else {
            ([0.50, 0.018, 0.02, 1.0], [0.98, 0.20, 0.13, 1.0])
        };
        glass::meter(v, bar, fraction, dark, bright);
        for q in [0.25, 0.5, 0.75] {
            let x = bar[0] + bar[2] * q;
            line(
                v,
                [x, bar[1] + 3.0],
                [x, bar[1] + bar[3] - 3.0],
                1.0,
                [0.0, 0.0, 0.0, 0.5],
            );
        }
        // Consumables and gold.
        let y = 1000.0;
        let mut x = 60.0;
        x += self.key(v, "Q", x, y + 2.0) + 8.0;
        let chip = |v: &mut Vec<Vertex>,
                    draw: &dyn Fn(&mut Vec<Vertex>, [f32; 2]),
                    text: &str,
                    color: Rgba,
                    x: f32| {
            let w = self.width(text, 17.0) + 50.0;
            glass::chip(v, [x, y, w, 32.0], color, 0.14);
            draw(v, [x + 19.0, y + 16.0]);
            self.mid(v, text, x + 36.0, y + 16.0, 17.0, IVORY);
            w
        };
        x += chip(
            v,
            &potion,
            &format!("Tónico ×{}", run.potions),
            [0.9, 0.2, 0.2, 1.0],
            x,
        ) + 8.0;
        x += chip(
            v,
            &leaf,
            &format!("Hierbas ×{}", run.herbs),
            [0.4, 0.8, 0.4, 1.0],
            x,
        ) + 8.0;
        chip(v, &coin, &format!("{}", run.gold), [1.0, 0.65, 0.2, 1.0], x);
    }

    /// Bottom right: the keys, kept quiet.
    fn controls_card(&self, tone: &Tone, v: &mut Vec<Vertex>) {
        let r = [1236.0, 916.0, 648.0, 128.0];
        glass::pane(v, r, 24.0, tone, 0.72);
        let rows: [&[(&[&str], &str)]; 3] = [
            &[
                (&["W", "A", "S", "D"], "Mover"),
                (&["SHIFT"], "Correr"),
                (&["ESPACIO"], "Saltar"),
            ],
            &[
                (&["CLIC"], "Espada"),
                (&["F"], "Interactuar"),
                (&["T"], "Antorcha"),
            ],
            &[
                (&["M"], "Mapa"),
                (&["F2"], "Semilla"),
                (&["F3"], "Atlas"),
                (&["G"], "Volver"),
            ],
        ];
        for (i, row) in rows.into_iter().enumerate() {
            let y = 928.0 + i as f32 * 36.0;
            let mut x = 1262.0;
            for (keys, label) in row {
                for key in *keys {
                    x += self.key(v, key, x, y) + 4.0;
                }
                x += 4.0;
                self.mid(v, label, x, y + 14.0, 17.0, glass::mix(IVORY, MUTED, 0.35));
                x += self.width(label, 17.0) + 26.0;
            }
        }
    }

    /// A champion's health across the top while the fight lasts.
    fn boss_plate(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let Some((name, fraction, enraged)) = run.boss_bar() else {
            return;
        };
        let (x, y, w, h) = (664.0, 36.0, 592.0, 100.0);
        let pulse = 0.5 + 0.5 * (run.elapsed * 5.0).sin();
        let heat = if enraged {
            [1.0, 0.42, 0.12, 1.0]
        } else {
            [0.95, 0.16, 0.16, 1.0]
        };
        glass::pane(v, [x, y, w, h], 28.0, tone, 1.0);
        if enraged {
            glass::round_outline(
                v,
                [x, y, w, h],
                28.0,
                2.2,
                glass::alpha(heat, 0.35 + 0.45 * pulse),
            );
        }
        glass::icon_disc(v, Icon::Crown, [x + 50.0, y + h * 0.5], 28.0, heat, enraged);
        let tx = x + 96.0;
        let right = x + w - 28.0;
        self.text(
            v,
            if enraged {
                "CAMPEÓN · FURIOSO"
            } else {
                "CAMPEÓN"
            },
            tx,
            y + 14.0,
            12.0,
            glass::alpha(heat, 0.95),
        );
        self.mid_fit(
            v,
            &name.to_uppercase(),
            tx,
            y + 48.0,
            26.0,
            right - tx - 70.0,
            if enraged {
                [1.0, 0.78, 0.6, 1.0]
            } else {
                IVORY
            },
        );
        self.right(
            v,
            &format!("{:.0}%", fraction * 100.0),
            right,
            y + 48.0,
            22.0,
            glass::mix(IVORY, heat, 0.35),
        );
        let bar = [tx, y + 68.0, right - tx, 20.0];
        let (low, high) = if enraged {
            (
                [0.70, 0.12, 0.02, 1.0],
                [1.0, 0.55 + 0.2 * pulse, 0.10, 1.0],
            )
        } else {
            ([0.50, 0.015, 0.03, 1.0], [0.98, 0.16, 0.16, 1.0])
        };
        glass::meter(v, bar, fraction, low, high);
        // The half-way notch where it will roar.
        let notch = bar[0] + bar[2] * 0.5;
        line(
            v,
            [notch, bar[1] + 2.0],
            [notch, bar[1] + bar[3] - 2.0],
            1.6,
            [1.0, 0.9, 0.6, 0.75],
        );
    }

    pub(super) fn chrome(&self, run: &Dungeon, v: &mut Vec<Vertex>) {
        let room = &run.map.rooms[run.map.region_at(run.controller.position)];
        let tone = Tone::new(run.catalog.biomes[room.biome].accent);
        self.location_plate(run, &tone, v);
        self.objective_card(run, &tone, v);
        self.vitality_card(run, &tone, v);
        self.controls_card(&tone, v);
        // Small quiet aiming diamond leaves most of the scene unobstructed.
        let hit_marker = run.sword_hit_flash > 0.0;
        diamond(
            v,
            [960.0, 540.0],
            if hit_marker { 4.2 } else { 2.6 },
            if hit_marker {
                [1.0, 0.90, 0.52, 0.98]
            } else {
                [0.95, 0.80, 0.47, 0.85]
            },
        );
        if let Some(prompt) = run.interaction() {
            self.action_prompt(v, &prompt, &tone);
        }
        if run.notice_until > run.elapsed && run.seed_input.is_none() {
            let width = (self.width(&run.notice, 22.0) + 72.0).clamp(440.0, 900.0);
            let x = (1920.0 - width) * 0.5;
            let y = if run.boss_bar().is_some() {
                150.0
            } else {
                44.0
            };
            glass::pane(v, [x, y, width, 52.0], 26.0, &tone, 1.0);
            diamond(v, [x + 26.0, y + 26.0], 5.0, tone.accent);
            self.mid_fit(
                v,
                &run.notice,
                x + 44.0,
                y + 26.0,
                22.0,
                width - 76.0,
                IVORY,
            );
        }
        // Blood rain: the edges of sight run red.
        let rain = run.heart.rain;
        if rain > 0.01 {
            for (i, band) in [26.0f32, 16.0, 8.0].into_iter().enumerate() {
                let a = rain * (0.10 + i as f32 * 0.07);
                let c = [0.45, 0.0, 0.01, a];
                rect(v, [0.0, 0.0, 1920.0, band], c);
                rect(v, [0.0, 1080.0 - band, 1920.0, band], c);
                rect(v, [0.0, 0.0, band, 1080.0], c);
                rect(v, [1920.0 - band, 0.0, band, 1080.0], c);
            }
        }
        self.boss_plate(run, &tone, v);
        if run.hit_flash > 0.0 {
            for y in [0.0, 1074.0] {
                rect(
                    v,
                    [0.0, y, 1920.0, 6.0],
                    [0.8, 0.004, 0.0, (run.hit_flash * 3.0).min(1.0)],
                );
            }
        }
    }
}

impl Hud {
    /// The map screen. Returns the vertices that must be clipped to the map
    /// viewport (`start..end`, native pixels `[x, y, w, h]`).
    pub(super) fn map_overlay(
        &self,
        run: &Dungeon,
        stats: RenderStats,
        v: &mut Vec<Vertex>,
    ) -> Option<(usize, usize, [f32; 4])> {
        self.draw_map(run, stats, v)
    }

    fn map_metric(
        &self,
        v: &mut Vec<Vertex>,
        label: &str,
        value: &str,
        x: f32,
        cy: f32,
        width: f32,
    ) {
        self.mid(v, label, x, cy, 16.0, MAP_MUTED);
        self.right(v, value, x + width, cy, 17.0, IVORY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_vectors_are_finite_and_bounded() {
        let tone = Tone::new([0.4, 0.5, 0.9]);
        let mut v = Vec::new();
        glass::pane(&mut v, [36.0, 36.0, 600.0, 116.0], 26.0, &tone, 1.0);
        coin(&mut v, [337.0, 1007.0]);
        potion(&mut v, [81.0, 1006.0]);
        leaf(&mut v, [120.0, 1006.0]);
        glass::altitude_gauge(&mut v, [612.0, 50.0, 8.0, 88.0]);
        assert!(v.len() < 10000);
        assert!(v
            .iter()
            .all(|v| v.p.into_iter().chain(v.color).all(f32::is_finite)));
        assert!(v
            .iter()
            .all(|v| v.p[0] >= 0.0 && v.p[0] <= 1920.0 && v.p[1] >= 0.0 && v.p[1] <= 1080.0));
    }

    #[test]
    fn prompt_icons_are_reachable_from_the_hud() {
        assert!(!prompt_icons::shape(Icon::Crown).is_empty());
        assert!(!prompt_icons::shape(Icon::Rune).is_empty());
        assert!(!prompt_icons::shape(Icon::Lock).is_empty());
    }
}
