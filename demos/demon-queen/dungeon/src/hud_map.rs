//! Expedition map (M) and technical atlas (F3): a glass chart with the
//! complex drawn in 3D, floors lifted to their true altitude (see
//! `hud_map3d`), plus the level list, objectives and finds. Authored in
//! native 1920x1080. Only visited rooms are charted; doorways of known rooms
//! reveal glimpsed neighbours as dashed sketches. Unopened secrets stay
//! hidden.
use super::glass::{self, alpha, altitude_color, lift, mix, Tone, CRIMSON, GREEN, IVORY, MUTED};
use super::{disc, line, polygon, ring, Dungeon, Hud, RenderStats, Vertex, PALE};
use std::f32::consts::TAU;
use velvet_dungeon::{gameplay::SpawnKind, ConnectionKind, Role, Room};

pub(super) const INK: [f32; 4] = [0.17, 0.105, 0.055, 1.0];
pub(super) const PARCH: [f32; 4] = [0.87, 0.78, 0.59, 1.0];
const PASSAGE: [f32; 4] = [0.84, 0.74, 0.54, 1.0];
const GILT: [f32; 4] = [0.78, 0.52, 0.12, 1.0];
const VIOLET: [f32; 4] = [0.42, 0.24, 0.58, 1.0];
const JADE: [f32; 4] = [0.2, 0.47, 0.3, 1.0];
const TEAL_INK: [f32; 4] = [0.1, 0.38, 0.42, 1.0];

/// Where the 3D chart lives on the 1920x1080 canvas.
pub(super) const VIEWPORT: [f32; 4] = super::super::super::MAP_VIEWPORT;

fn box_fill(v: &mut Vec<Vertex>, r: [f32; 4], c: [f32; 4]) {
    super::rect(v, r, c);
}

/// Room-type icon in ink, drawn on a parchment medallion at `p`.
pub(super) fn role_icon(v: &mut Vec<Vertex>, p: [f32; 2], role: Role) {
    let [x, y] = p;
    match role {
        Role::Entrance => {
            disc(v, [x, y - 1.0], 6.0, TEAL_INK);
            box_fill(v, [x - 6.0, y - 1.0, 12.0, 8.5], TEAL_INK);
            disc(v, [x, y - 1.0], 3.3, PARCH);
            box_fill(v, [x - 3.3, y - 1.0, 6.6, 8.5], PARCH);
        }
        Role::Exit => {
            for (k, (dx, dy)) in [(-7.0, 3.0), (-3.5, -0.8), (0.0, -4.6)]
                .into_iter()
                .enumerate()
            {
                box_fill(v, [x + dx, y + dy, 14.0 - k as f32 * 3.5, 3.6], INK);
            }
            super::triangle(
                v,
                [x + 4.5, y - 9.0],
                [x + 8.5, y - 4.5],
                [x + 0.5, y - 4.5],
                CRIMSON,
            );
        }
        Role::Treasure => {
            box_fill(v, [x - 7.5, y - 1.5, 15.0, 8.0], GILT);
            box_fill(v, [x - 7.5, y - 6.0, 15.0, 4.0], [0.55, 0.33, 0.08, 1.0]);
            for sx in [-4.5, 4.5] {
                box_fill(v, [x + sx - 1.0, y - 6.0, 2.0, 12.5], INK);
            }
            box_fill(v, [x - 1.6, y - 2.6, 3.2, 3.6], INK);
        }
        Role::Secret => {
            disc(v, [x, y - 2.5], 3.6, VIOLET);
            super::triangle(
                v,
                [x - 3.4, y + 7.0],
                [x + 3.4, y + 7.0],
                [x, y - 1.5],
                VIOLET,
            );
        }
        Role::Puzzle => {
            ring(v, p, 6.2, 1.5, TEAL_INK);
            for k in 0..3 {
                let a = k as f32 * TAU / 3.0 - std::f32::consts::FRAC_PI_2;
                disc(v, [x + a.cos() * 6.2, y + a.sin() * 6.2], 2.0, INK);
            }
            disc(v, p, 1.6, TEAL_INK);
        }
        Role::Event => {
            let eye: Vec<[f32; 2]> = (0..16)
                .map(|k| {
                    let a = k as f32 * TAU / 16.0;
                    [x + a.cos() * 8.0, y + a.sin() * 4.6]
                })
                .collect();
            polygon(v, &eye, INK);
            disc(v, p, 3.0, PASSAGE);
            disc(v, p, 1.5, INK);
        }
        Role::Rest => {
            let flame = [
                [x, y - 8.5],
                [x + 5.0, y + 0.5],
                [x + 3.2, y + 6.5],
                [x - 3.2, y + 6.5],
                [x - 5.0, y + 0.5],
            ];
            polygon(v, &flame, JADE);
            polygon(
                v,
                &[
                    [x, y - 3.0],
                    [x + 2.4, y + 2.5],
                    [x, y + 5.5],
                    [x - 2.4, y + 2.5],
                ],
                [0.62, 0.84, 0.5, 1.0],
            );
        }
        Role::Encounter | Role::Elite => {
            let color = if role == Role::Elite { CRIMSON } else { INK };
            for s in [-1.0f32, 1.0] {
                line(
                    v,
                    [x - 6.5 * s, y + 6.5],
                    [x + 6.0 * s, y - 6.0],
                    2.0,
                    color,
                );
                line(
                    v,
                    [x - 5.5 * s + 2.2, y + 3.3],
                    [x - 5.5 * s - 2.2, y + 7.6],
                    1.6,
                    color,
                );
            }
        }
        Role::Miniboss | Role::Boss => {
            disc(v, [x, y - 1.0], 6.0, CRIMSON);
            box_fill(v, [x - 3.4, y + 3.0, 6.8, 4.2], CRIMSON);
            disc(v, [x - 2.4, y - 1.2], 1.7, PARCH);
            disc(v, [x + 2.4, y - 1.2], 1.7, PARCH);
            if role == Role::Boss {
                for dx in [-4.5, 0.0, 4.5] {
                    super::triangle(
                        v,
                        [x + dx - 2.2, y - 6.5],
                        [x + dx + 2.2, y - 6.5],
                        [x + dx, y - 11.0],
                        GILT,
                    );
                }
            }
        }
        _ => {
            ring(v, p, 4.2, 1.2, INK);
            disc(v, p, 2.0, INK);
        }
    }
}

/// `role_icon` at `k` times its drawn size.
pub(super) fn role_icon_scaled(v: &mut Vec<Vertex>, p: [f32; 2], role: Role, k: f32) {
    let start = v.len();
    role_icon(v, p, role);
    for vertex in &mut v[start..] {
        vertex.p = [
            p[0] + (vertex.p[0] - p[0]) * k,
            p[1] + (vertex.p[1] - p[1]) * k,
        ];
    }
}

/// A lit parchment badge with the room's icon, the map's room marker.
pub(super) fn medallion(v: &mut Vec<Vertex>, p: [f32; 2], radius: f32, role: Role, resolved: bool) {
    let wash = role_wash(role);
    disc(
        v,
        [p[0] + 1.2, p[1] + 2.4],
        radius + 1.5,
        [0.0, 0.0, 0.0, 0.4],
    );
    disc(v, p, radius + 1.2, [0.01, 0.015, 0.02, 0.95]);
    disc(v, p, radius, mix(PARCH, wash, 0.16));
    ring(v, p, radius - 0.4, 1.6, alpha(wash, 0.95));
    role_icon_scaled(v, p, role, radius / 12.5);
    if resolved {
        let c = [p[0] + radius * 0.78, p[1] - radius * 0.78];
        disc(v, c, 6.5, [0.03, 0.10, 0.06, 1.0]);
        ring(v, c, 6.5, 1.2, GREEN);
        line(v, [c[0] - 3.0, c[1]], [c[0] - 0.8, c[1] + 2.6], 1.8, GREEN);
        line(
            v,
            [c[0] - 0.8, c[1] + 2.6],
            [c[0] + 3.6, c[1] - 3.2],
            1.8,
            GREEN,
        );
    }
}

pub(super) fn role_wash(role: Role) -> [f32; 4] {
    match role {
        Role::Entrance => [0.3, 0.6, 0.55, 1.0],
        Role::Exit => [0.86, 0.55, 0.25, 1.0],
        Role::Treasure => [0.95, 0.72, 0.25, 1.0],
        Role::Secret => [0.58, 0.42, 0.72, 1.0],
        Role::Puzzle => [0.36, 0.56, 0.7, 1.0],
        Role::Event => [0.62, 0.5, 0.36, 1.0],
        Role::Rest => [0.46, 0.66, 0.42, 1.0],
        Role::Encounter | Role::Elite => [0.78, 0.4, 0.32, 1.0],
        Role::Miniboss | Role::Boss => [0.74, 0.25, 0.22, 1.0],
        Role::Normal => [0.8, 0.72, 0.55, 1.0],
    }
}

pub(super) struct Knowledge {
    pub visited: Vec<bool>,
    pub glimpsed: Vec<bool>,
    pub corridor: Vec<bool>,
}

pub(super) fn knowledge(run: &Dungeon) -> Knowledge {
    let rooms = run.map.rooms.len();
    let visited: Vec<bool> = (0..rooms)
        .map(|i| run.debug || run.visited.contains(&i))
        .collect();
    let mut glimpsed = vec![false; rooms];
    let mut corridor = vec![false; run.map.connections.len()];
    for c in &run.map.connections {
        let hidden = c.kind == ConnectionKind::Secret && !run.opened.contains(&c.id) && !run.debug;
        if hidden {
            continue;
        }
        if visited[c.a] || visited[c.b] {
            corridor[c.id] = true;
            glimpsed[c.a] |= !visited[c.a];
            glimpsed[c.b] |= !visited[c.b];
        }
    }
    Knowledge {
        visited,
        glimpsed,
        corridor,
    }
}

/// One altitude band of the complex: rooms whose floors share a height.
struct Level {
    y: f32,
    rooms: usize,
    seen: usize,
    here: bool,
}

/// Floors grouped into at most six bands, highest first.
fn levels(run: &Dungeon, known: &Knowledge) -> Vec<Level> {
    let mut rooms: Vec<(f32, usize)> = run.map.rooms.iter().map(|r| (r.center.y, r.id)).collect();
    rooms.sort_by(|a, b| a.0.total_cmp(&b.0));
    let here = run.map.region_at(run.controller.position);
    let mut tolerance = 1.4;
    loop {
        let mut out: Vec<(Vec<(f32, usize)>, f32)> = Vec::new();
        for &(y, id) in &rooms {
            match out.last_mut() {
                Some((group, top)) if y - *top <= tolerance => {
                    group.push((y, id));
                    *top = y;
                }
                _ => out.push((vec![(y, id)], y)),
            }
        }
        if out.len() <= 6 || tolerance > 40.0 {
            return out
                .into_iter()
                .rev()
                .map(|(group, _)| Level {
                    y: group.iter().map(|g| g.0).sum::<f32>() / group.len() as f32,
                    rooms: group.len(),
                    seen: group.iter().filter(|g| known.visited[g.1]).count(),
                    here: group.iter().any(|g| g.1 == here),
                })
                .collect();
        }
        tolerance *= 1.5;
    }
}

impl Hud {
    pub(super) fn draw_map(
        &self,
        run: &Dungeon,
        stats: RenderStats,
        v: &mut Vec<Vertex>,
    ) -> Option<(usize, usize, [f32; 4])> {
        let current_id = run.map.region_at(run.controller.position);
        let current = &run.map.rooms[current_id];
        let biome = &run.catalog.biomes[current.biome.min(run.catalog.biomes.len() - 1)];
        let tone = Tone::new(biome.accent);
        let known = knowledge(run);
        super::rect(v, [0.0, 0.0, 1920.0, 1080.0], [0.002, 0.005, 0.008, 0.86]);

        // Header.
        glass::pane(v, [48.0, 36.0, 1824.0, 84.0], 26.0, &tone, 1.0);
        let title_w = self.title_ribbon(
            v,
            if run.debug {
                "Atlas técnico"
            } else {
                "Carta de la expedición"
            },
            74.0,
            44.0,
            30.0,
            700.0,
            &tone,
        );
        self.aside(
            v,
            if run.debug {
                "Todo el complejo revelado · diagnóstico del generador"
            } else {
                "Solo lo que has pisado se traza sólido; lo entrevisto, punteado"
            },
            74.0 + title_w + 30.0,
            68.0,
            16.0,
            1180.0 - title_w,
            IVORY,
        );
        let region = if biome.id == "forgotten_masonry" {
            "Vestíbulos olvidados"
        } else {
            biome.name.as_str()
        };
        let seed = if run.authored.is_some() {
            "La Mina".to_string()
        } else {
            run.seed().to_string()
        };
        let seed_w = self.width(&seed, 22.0).max(90.0);
        let sx = 1840.0 - seed_w;
        self.kicker(
            v,
            if run.authored.is_some() {
                "LUGAR"
            } else {
                "SEMILLA"
            },
            sx,
            46.0,
            tone.accent,
        );
        self.mid(v, &seed, sx, 92.0, 22.0, IVORY);
        let rx = sx - 60.0 - 250.0;
        self.kicker(v, "REGIÓN", rx, 46.0, tone.accent);
        self.serif(v, region, rx, 92.0, 23.0, 250.0, IVORY);

        // The 3D chart.
        let vp = VIEWPORT;
        super::rect(
            v,
            [vp[0] + 8.0, vp[1] + 8.0, vp[2], vp[3]],
            [0.0, 0.0, 0.0, 0.5],
        );
        glass::round(
            v,
            vp,
            0.5,
            [0.008, 0.016, 0.026, 0.97],
            [0.003, 0.006, 0.012, 0.98],
        );
        let clip_start = v.len();
        self.draw_chart(run, &known, &tone, vp, v);
        let clip_end = v.len();
        // A frame in the menus' manner: a fine rim, the place's colour along the top and a
        // wedge in the corner.
        glass::round_outline(v, vp, 0.5, 1.2, [1.0, 1.0, 1.0, 0.14]);
        super::rect(v, [vp[0], vp[1], vp[2], 3.0], alpha(tone.accent, 0.9));
        polygon(
            v,
            &[[vp[0], vp[1]], [vp[0] + 46.0, vp[1]], [vp[0], vp[1] + 46.0]],
            tone.accent,
        );
        self.chart_overlays(run, &tone, vp, v);

        self.map_side_panel(run, stats, &known, &tone, v, current);
        self.map_bottom_bar(run, &tone, v);
        Some((clip_start, clip_end, vp))
    }

    /// Over the chart: the altitude legend, the vertical exaggeration and a
    /// compass turning with the view.
    fn chart_overlays(&self, run: &Dungeon, tone: &Tone, vp: [f32; 4], v: &mut Vec<Vertex>) {
        let (low, high) = self.altitude_range(run);
        let (x, y) = (vp[0] + 30.0, vp[1] + 26.0);
        glass::pane(v, [x - 14.0, y - 12.0, 262.0, 74.0], 18.0, tone, 0.8);
        self.text(
            v,
            "ALTITUD",
            x + 4.0,
            y - 3.0,
            12.0,
            alpha(tone.accent, 0.95),
        );
        let bar = [x, y + 22.0, 232.0, 10.0];
        let slices = 14;
        for i in 0..slices {
            let (a, b) = (i as f32 / slices as f32, (i + 1) as f32 / slices as f32);
            let (ca, cb) = (altitude_color(a), altitude_color(b));
            let (xa, xb) = (bar[0] + bar[2] * a, bar[0] + bar[2] * b);
            for (p, k) in [
                ([xa, bar[1]], ca),
                ([xb, bar[1]], cb),
                ([xb, bar[1] + bar[3]], cb),
                ([xa, bar[1]], ca),
                ([xb, bar[1] + bar[3]], cb),
                ([xa, bar[1] + bar[3]], ca),
            ] {
                v.push(Vertex {
                    p,
                    uv: [0.0015; 2],
                    color: k,
                });
            }
        }
        glass::round_outline(v, bar, 0.5, 1.0, [1.0, 1.0, 1.0, 0.25]);
        self.text(
            v,
            &format!("{:+.0} m", low.round()),
            x,
            y + 38.0,
            13.0,
            MUTED,
        );
        self.right(
            v,
            &format!("{:+.0} m", high.round()),
            x + 232.0,
            y + 46.0,
            13.0,
            MUTED,
        );

        let c = [vp[0] + vp[2] - 84.0, vp[1] + 84.0];
        self.compass(v, c, -run.map_yaw, tone);
        self.centered(v, "N", c[0], c[1] - 52.0, 14.0, alpha(tone.bright, 0.9));
        self.aside(
            v,
            "A D girar  ·  W S inclinar  ·  Q E zoom",
            vp[0] + 34.0,
            vp[1] + vp[3] - 28.0,
            14.0,
            600.0,
            alpha(MUTED, 0.9),
        );
        let note = format!("Alturas ×{:.1}", super::map3d::EXAGGERATION);
        self.right(
            v,
            &note,
            vp[0] + vp[2] - 30.0,
            vp[1] + vp[3] - 24.0,
            13.0,
            alpha(MUTED, 0.8),
        );
    }

    fn map_bottom_bar(&self, run: &Dungeon, tone: &Tone, v: &mut Vec<Vertex>) {
        let r = [48.0, 950.0, 1824.0, 94.0];
        glass::pane(v, r, 26.0, tone, 1.0);
        let cy = r[1] + r[3] * 0.5;
        let items = [
            (Role::Entrance, "Entrada"),
            (Role::Treasure, "Tesoro"),
            (Role::Secret, "Secreto"),
            (Role::Puzzle, "Sello"),
            (Role::Rest, "Santuario"),
            (Role::Encounter, "Combate"),
            (Role::Boss, "Jefe"),
            (Role::Exit, "Salida"),
        ];
        let mut x = 90.0;
        for (role, label) in items {
            medallion(v, [x, cy], 15.0, role, false);
            self.mid(v, label, x + 24.0, cy, 16.0, IVORY);
            x += 24.0 + self.width(label, 16.0) + 28.0;
        }
        // View controls, right aligned.
        let hints: [(&[&str], &str); 5] = [
            (&["CLIC"], "Orbitar"),
            (&["CLIC DER."], "Mover"),
            (&["RUEDA"], "Zoom"),
            (&["R"], "Vista"),
            (&["M"], "Cerrar"),
        ];
        let mut total = 0.0;
        for (keys, label) in hints {
            total += keys
                .iter()
                .map(|k| self.width(k, 15.0).max(10.0) + 22.0)
                .sum::<f32>()
                + 8.0
                + self.width(label, 16.0)
                + 24.0;
        }
        let f3 = if run.debug { "Carta" } else { "Atlas" };
        total += 34.0 + 8.0 + self.width(f3, 16.0);
        let mut x = r[0] + r[2] - 36.0 - total;
        for (keys, label) in hints {
            for key in keys {
                x += self.key(v, key, x, cy - 14.0) + 4.0;
            }
            x += 4.0;
            self.mid(v, label, x, cy, 16.0, glass::mix(IVORY, MUTED, 0.3));
            x += self.width(label, 16.0) + 24.0;
        }
        x += self.key(v, "F3", x, cy - 14.0) + 8.0;
        self.mid(v, f3, x, cy, 16.0, glass::mix(IVORY, MUTED, 0.3));
    }

    fn map_side_panel(
        &self,
        run: &Dungeon,
        stats: RenderStats,
        known: &Knowledge,
        tone: &Tone,
        v: &mut Vec<Vertex>,
        current: &Room,
    ) {
        let r = [1312.0, 140.0, 560.0, 790.0];
        glass::pane(v, r, 28.0, tone, 1.0);
        let x = r[0] + 32.0;
        let w = r[2] - 64.0;
        let mut y = r[1] + 26.0;
        let heading = |v: &mut Vec<Vertex>, y: f32, label: &str| {
            self.kicker(v, label, x - 4.0, y - 6.0, tone.accent);
        };
        let divider = |v: &mut Vec<Vertex>, y: f32| {
            glass::segment(v, [x, y], [x + w, y], 1.0, alpha(tone.accent, 0.25));
        };

        // Progress.
        let total = run.map.rooms.len().max(1);
        let seen = known.visited.iter().filter(|s| **s).count().min(total);
        let ratio = seen as f32 / total as f32;
        heading(v, y, "EXPEDICIÓN");
        self.right(v, &format!("{seen} / {total}"), x + w, y + 9.0, 20.0, IVORY);
        y += 32.0;
        glass::meter(
            v,
            [x, y, w, 12.0],
            ratio,
            glass::lift(tone.accent, 0.6),
            glass::mix(tone.accent, [1.0, 0.95, 0.8, 1.0], 0.3),
        );
        self.text(v, "salas exploradas", x, y + 18.0, 13.0, MUTED);
        y += 48.0;

        // Altitude bands.
        divider(v, y);
        y += 14.0;
        heading(v, y, "NIVELES DE ALTURA");
        y += 30.0;
        let bands = levels(run, known);
        for band in &bands {
            let cy = y + 14.0;
            let color = altitude_color(self.altitude_fraction(run, band.y));
            if band.here {
                glass::chip(v, [x - 10.0, y, w + 20.0, 30.0], color, 0.18);
            }
            glass::slant(v, [x, cy - 8.0, 28.0, 16.0], 4.0, lift(color, 1.1));
            self.mid(
                v,
                &format!("{:+.0} m", band.y),
                x + 40.0,
                cy,
                18.0,
                if band.here {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    IVORY
                },
            );
            let frac = band.seen as f32 / band.rooms.max(1) as f32;
            glass::meter(
                v,
                [x + 122.0, cy - 5.0, 190.0, 10.0],
                frac,
                lift(color, 0.7),
                lift(color, 1.2),
            );
            self.right(
                v,
                &format!("{} / {}", band.seen, band.rooms),
                x + w - if band.here { 66.0 } else { 0.0 },
                cy,
                16.0,
                MUTED,
            );
            if band.here {
                self.right(
                    v,
                    "AQUÍ",
                    x + w - 4.0,
                    cy,
                    14.0,
                    glass::mix(color, [1.0, 1.0, 1.0, 1.0], 0.55),
                );
            }
            y += 33.0;
        }
        y += 6.0;

        if run.debug {
            divider(v, y);
            y += 14.0;
            heading(v, y, "DIAGNÓSTICO TÉCNICO");
            y += 26.0;
            let rows = [
                (
                    "Ruta principal",
                    format!("{} salas", run.map.stats.critical_length),
                ),
                (
                    "Ramas / ciclos",
                    format!("{} / {}", run.map.stats.branches, run.map.stats.loops),
                ),
                (
                    "Cambios de nivel",
                    run.map.stats.vertical_connections.to_string(),
                ),
                (
                    "Salas superpuestas",
                    run.map.stats.stacked_rooms.to_string(),
                ),
                (
                    "Generación",
                    format!("{:.2} ms", run.map.stats.generation_ms),
                ),
                (
                    "Rechazos",
                    format!(
                        "{} / {}",
                        run.map.stats.rejected_attempts, run.map.stats.attempts
                    ),
                ),
                ("Reparaciones", run.map.stats.local_repairs.to_string()),
                ("Salas visibles", stats.visible_rooms.to_string()),
                ("Chunks GPU", format!("{} / 48", stats.resident_chunks)),
                ("Draws", stats.draws.to_string()),
                ("Triángulos", stats.triangles.to_string()),
            ];
            for (label, value) in rows.iter() {
                self.map_metric(v, label, value, x, y + 11.0, w);
                y += 26.0;
            }
            self.mid(v, "Validación: sin errores", x, y + 12.0, 14.0, GREEN);
            return;
        }

        divider(v, y);
        y += 14.0;
        heading(v, y, "OBJETIVOS");
        y += 26.0;
        let aligned = (0..3).filter(|i| run.sigils[*i] as usize == i + 1).count();
        let objectives = [
            (
                run.seal,
                "Alinear las runas del sello".to_string(),
                format!("{aligned} / 3 runas en su fase"),
            ),
            (
                run.crest,
                "Obtener la llave del Custodio".to_string(),
                "Derrota al guardián del umbral".to_string(),
            ),
            (
                run.won,
                "Derrotar al Corazón".to_string(),
                "La salida despierta con su caída".to_string(),
            ),
            (
                false,
                "Regresar a la superficie".to_string(),
                if run.won {
                    "El altar de salida aguarda".to_string()
                } else {
                    "Tras la victoria".to_string()
                },
            ),
        ];
        for (done, title, note) in &objectives {
            let c = [x + 11.0, y + 16.0];
            let cell = [c[0] - 11.0, c[1] - 10.0, 24.0, 20.0];
            glass::slant(
                v,
                cell,
                5.0,
                if *done {
                    alpha(GREEN, 0.3)
                } else {
                    alpha(glass::INK, 0.95)
                },
            );
            glass::slant_outline(
                v,
                cell,
                5.0,
                1.4,
                if *done {
                    GREEN
                } else {
                    alpha(tone.accent, 0.7)
                },
            );
            if *done {
                line(v, [c[0] - 4.5, c[1]], [c[0] - 1.2, c[1] + 3.6], 2.0, GREEN);
                line(
                    v,
                    [c[0] - 1.2, c[1] + 3.6],
                    [c[0] + 5.0, c[1] - 4.2],
                    2.0,
                    GREEN,
                );
            }
            self.mid_fit(
                v,
                title,
                x + 34.0,
                y + 9.0,
                18.0,
                w - 36.0,
                if *done { MUTED } else { IVORY },
            );
            if *done {
                let tw = self.width(title, 18.0).min(w - 36.0);
                line(
                    v,
                    [x + 34.0, y + 9.0],
                    [x + 34.0 + tw, y + 9.0],
                    1.0,
                    alpha(MUTED, 0.7),
                );
            }
            self.aside(
                v,
                note,
                x + 34.0,
                y + 29.0,
                14.0,
                w - 36.0,
                alpha(MUTED, 0.9),
            );
            y += 44.0;
        }

        divider(v, y);
        y += 14.0;
        heading(v, y, "HALLAZGOS");
        y += 26.0;
        let chests: Vec<_> = run
            .actors
            .iter()
            .filter(|a| run.map.spawns[a.spawn].kind == SpawnKind::Chest)
            .collect();
        let opened = chests.iter().filter(|a| a.used).count();
        let slain = run
            .actors
            .iter()
            .filter(|a| run.map.spawns[a.spawn].health > 0.0 && a.health <= 0.0)
            .count();
        for (label, value) in [
            (
                "Relicarios abiertos",
                format!("{opened} / {}", chests.len()),
            ),
            ("Pasajes secretos", run.opened.len().to_string()),
            ("Enemigos abatidos", slain.to_string()),
            ("Oro de la expedición", run.gold.to_string()),
        ] {
            self.map_metric(v, label, &value, x, y + 11.0, w);
            y += 27.0;
        }

        // Where we stand.
        y += 8.0;
        let here = [x - 10.0, y, w + 20.0, r[1] + r[3] - 22.0 - y];
        if here[3] >= 52.0 {
            glass::chip(v, here, tone.accent, 0.14);
            let cy = here[1] + here[3] * 0.5;
            glass::diamond(v, [x + 10.0, cy], 6.0, tone.bright);
            self.text(
                v,
                "AQUÍ",
                x + 28.0,
                cy - 22.0,
                12.0,
                alpha(tone.accent, 0.95),
            );
            let name = run
                .authored
                .as_ref()
                .and_then(|mine| mine.names.get(current.id))
                .map_or(current.role.label(), String::as_str);
            self.serif(v, name, x + 28.0, cy + 8.0, 22.0, 250.0, IVORY);
            self.right(
                v,
                &format!("Altura {:+.0} m", run.controller.position.y),
                x + w - 6.0,
                cy + 6.0,
                16.0,
                MUTED,
            );
        }
    }
}

impl Hud {
    /// F2 dialog: replay any expedition from its seed.
    pub(in super::super) fn seed_dialog(&self, run: &Dungeon, seed: &str, v: &mut Vec<Vertex>) {
        let room = &run.map.rooms[run.map.region_at(run.controller.position)];
        let tone = Tone::new(run.catalog.biomes[room.biome].accent);
        super::rect(v, [0.0, 0.0, 1920.0, 1080.0], [0.002, 0.006, 0.008, 0.86]);
        let r = [520.0, 314.0, 880.0, 448.0];
        glass::pane(v, r, 30.0, &tone, 1.0);
        let x = r[0] + 46.0;
        let w = r[2] - 92.0;
        self.compass(v, [x + 30.0, r[1] + 62.0], 0.0, &tone);
        self.title_ribbon(
            v,
            "Reproducir una expedición",
            x + 84.0,
            r[1] + 26.0,
            28.0,
            w - 170.0,
            &tone,
        );
        self.aside(
            v,
            "Cada semilla talla siempre la misma caverna",
            x + 92.0,
            r[1] + 98.0,
            17.0,
            w - 100.0,
            IVORY,
        );
        glass::segment(
            v,
            [x, r[1] + 124.0],
            [x + w, r[1] + 124.0],
            1.0,
            alpha(tone.accent, 0.3),
        );
        self.kicker(v, "SEMILLA", x, r[1] + 136.0, tone.accent);
        self.right(
            v,
            &format!("actual: {}", run.seed()),
            x + w,
            r[1] + 150.0,
            14.0,
            MUTED,
        );
        // Input well with a blinking caret.
        let field = [x, r[1] + 170.0, w, 72.0];
        glass::slant(
            v,
            [field[0] + 5.0, field[1] + 5.0, field[2], field[3]],
            10.0,
            [0.0, 0.0, 0.0, 0.5],
        );
        glass::slant(v, field, 10.0, alpha(glass::INK, 0.97));
        glass::slant_outline(v, field, 10.0, 1.8, alpha(tone.accent, 0.85));
        let shown = if seed.is_empty() { "" } else { seed };
        self.mid(
            v,
            shown,
            field[0] + 24.0,
            field[1] + field[3] * 0.5,
            34.0,
            IVORY,
        );
        if seed.is_empty() {
            self.mid(
                v,
                "42",
                field[0] + 24.0,
                field[1] + field[3] * 0.5,
                34.0,
                alpha(MUTED, 0.4),
            );
        }
        if (run.elapsed * 2.2).fract() < 0.6 || seed.is_empty() {
            let cx = field[0] + 26.0 + self.width(shown, 34.0);
            super::rect(v, [cx, field[1] + 16.0, 2.6, 40.0], PALE);
        }
        self.text(
            v,
            "Decimal o hexadecimal  ·  ej. 42, 8675309, 0x1F2E",
            x,
            r[1] + 256.0,
            15.0,
            MUTED,
        );
        let invalid = run.notice_until > run.elapsed && run.notice.starts_with("Seed invalida");
        if invalid {
            self.fitted(
                v,
                &run.notice,
                x,
                r[1] + 288.0,
                16.0,
                w,
                [1.0, 0.55, 0.4, 1.0],
            );
        } else {
            self.text(
                v,
                "La expedición actual se reiniciará desde la entrada",
                x,
                r[1] + 288.0,
                16.0,
                [0.95, 0.68, 0.4, 1.0],
            );
        }
        glass::segment(
            v,
            [x, r[1] + 338.0],
            [x + w, r[1] + 338.0],
            1.0,
            alpha(tone.accent, 0.3),
        );
        let cy = r[1] + 388.0;
        let mut kx = x;
        kx += self.key(v, "ENTER", kx, cy - 14.0) + 10.0;
        self.mid(v, "Generar", kx, cy, 17.0, IVORY);
        kx += self.width("Generar", 17.0) + 34.0;
        kx += self.key(v, "ESC", kx, cy - 14.0) + 10.0;
        self.mid(v, "Cancelar", kx, cy, 17.0, IVORY);
        let back = self.width("Borrar", 17.0);
        let bw = self.width("RETROCESO", 15.0) + 18.0;
        self.right(v, "Borrar", x + w, cy, 17.0, IVORY);
        self.key(v, "RETROCESO", x + w - back - 10.0 - bw, cy - 14.0);
    }
}

/// Looted treasure, cleared combat, lit sanctuary or aligned seal.
pub(super) fn room_resolved(run: &Dungeon, room: &Room) -> bool {
    let in_room = |kinds: &[SpawnKind]| {
        run.actors
            .iter()
            .filter(|a| {
                run.map.spawns[a.spawn].room == room.id
                    && kinds.contains(&run.map.spawns[a.spawn].kind)
            })
            .collect::<Vec<_>>()
    };
    match room.role {
        Role::Treasure | Role::Secret => {
            let chests = in_room(&[SpawnKind::Chest]);
            !chests.is_empty() && chests.iter().all(|a| a.used)
        }
        Role::Encounter | Role::Elite | Role::Miniboss | Role::Boss => {
            let foes = in_room(&[
                SpawnKind::Enemy,
                SpawnKind::Elite,
                SpawnKind::Miniboss,
                SpawnKind::Boss,
            ]);
            !foes.is_empty() && foes.iter().all(|a| a.health <= 0.0)
        }
        Role::Rest => in_room(&[SpawnKind::Rest]).iter().any(|a| a.used),
        Role::Puzzle => run.seal,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_icons_and_medallions_are_finite_and_inside_the_canvas() {
        let mut v = Vec::new();
        for (k, role) in [
            Role::Entrance,
            Role::Exit,
            Role::Normal,
            Role::Encounter,
            Role::Treasure,
            Role::Secret,
            Role::Puzzle,
            Role::Event,
            Role::Rest,
            Role::Elite,
            Role::Miniboss,
            Role::Boss,
        ]
        .into_iter()
        .enumerate()
        {
            medallion(
                &mut v,
                [100.0 + k as f32 * 60.0, 400.0],
                15.0,
                role,
                k % 2 == 0,
            );
        }
        assert!(v.len() > 1000);
        assert!(v
            .iter()
            .all(|v| v.p.into_iter().chain(v.color).all(f32::is_finite)));
        assert!(v
            .iter()
            .all(|v| v.p[0] >= 0.0 && v.p[0] <= 1920.0 && v.p[1] >= 0.0 && v.p[1] <= 1080.0));
    }
}
