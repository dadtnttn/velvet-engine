//! The two loading minigames (ports of `banquet.js` and `infirmary.js`): each
//! is a small pixel-art scene of a character doing what they do, and mashing a
//! key makes them do it faster. The static rooms are baked by
//! `tools/export-loading.mjs` in the Three.js project; everything that moves
//! (the characters, flames, glows, particles, pops) is drawn here.
use std::rc::Rc;

use super::nyx::{draw_nyx, NyxPose, FLASK};
use super::palette::Palette;
use super::pixel::{clamp, glow, hash, jr, mix, ramp, PoseCache, Rgb, Surface, H, W};
use super::queen::{draw_queen, QueenPose, QUEEN_MOUTH};

/// Small deterministic RNG (xorshift64*) standing in for `Math.random`.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64) / (1u64 << 53) as f64
    }
}

pub trait Scene {
    fn title(&self) -> &'static str;
    fn subtitle(&self) -> &'static str;
    fn counter_label(&self) -> &'static str;
    fn counter(&self) -> i32;
    fn tips(&self) -> &'static [&'static str];
    fn update(&mut self, dt: f64, mash: bool);
    fn draw(&mut self, t: f64) -> &Surface;
}

// ---------------------------------------------------------------- scenery

const FLAME: [&str; 5] = ["#b82a0a", "#ff6a14", "#ffb13a", "#ffe78a", "#fffbe0"];

/// Animated candle flame (with inner core) centred on its base.
pub fn flame(s: &mut Surface, x: f64, y: f64, t: f64, size: f64, seed: f64) {
    let ramp = ramp(&FLAME);
    let flick = (t * 13.0 + seed * 7.0).sin() * 0.5 + (t * 23.0 + seed * 3.0).sin() * 0.3;
    let h = jr((5.0 + flick) * size) as i32;
    let lean = jr((t * 5.0 + seed).sin() * 0.8 * size);
    for j in 0..h {
        let f = j as f64 / h as f64;
        let w_half = (jr((1.0 - f) * 1.6 * size - if f < 0.2 { 0.6 } else { 0.0 })).max(0.0) as i32;
        let cx = x + jr(lean * f);
        for i in -w_half..=w_half {
            let core = (i.abs() as f64) < w_half as f64 * 0.5 && f < 0.6;
            let c = if core {
                ramp[4]
            } else if f > 0.75 {
                ramp[1]
            } else if i.abs() == w_half {
                ramp[2]
            } else {
                ramp[3]
            };
            s.set(cx + i as f64, y - j as f64, c, 1.0);
        }
    }
    s.set(x, y + 1.0, [40.0, 20.0, 20.0], 1.0);
}

/// Candle: wax stick with drips, flame and warm pool of glow.
pub fn candle(
    s: &mut Surface,
    x: f64,
    y: f64,
    t: f64,
    height: i32,
    width: i32,
    seed: f64,
    glow_r: f64,
) {
    let wax = ramp(&["#8a6a5a", "#c9ab94", "#eedcc8", "#fff6ea"]);
    for j in 0..height {
        for i in 0..width {
            let c = if i == 0 {
                wax[1]
            } else if i == width - 1 {
                wax[0]
            } else {
                wax[2]
            };
            s.set(x + i as f64 - (width / 2) as f64, y - j as f64, c, 1.0);
        }
    }
    s.set(x - (width / 2) as f64, y - height as f64 + 1.0, wax[3], 1.0);
    if hash(seed, 1.0, 0.0) > 0.4 {
        s.set(
            x + (width / 2) as f64,
            y - height as f64 + 3.0 + (hash(seed, 2.0, 0.0) * 3.0).floor(),
            wax[3],
            1.0,
        );
    }
    flame(
        s,
        x,
        y - height as f64 - 1.0,
        t,
        if width > 3 { 1.2 } else { 1.0 },
        seed,
    );
    glow(
        s,
        x,
        y - height as f64 - 3.0,
        glow_r,
        [255.0, 150.0, 60.0],
        0.32 + (t * 9.0 + seed).sin() * 0.03,
        5.0,
    );
}

// ------------------------------------------------------------- pixel text

fn glyph(ch: char) -> Option<&'static [&'static str]> {
    Some(match ch {
        'Ñ' => &[
            "0101", "1010", "0000", "1001", "1101", "1011", "1001", "1001",
        ],
        'A' => &["0110", "1001", "1111", "1001", "1001"],
        'M' => &["10001", "11011", "10101", "10001", "10001"],
        '!' => &["1", "1", "1", "0", "1"],
        '+' => &["000", "010", "111", "010", "000"],
        '1' => &["01", "11", "01", "01", "01"],
        _ => return None,
    })
}

/// A tiny pixel font for the pops over the scene.
pub fn pixel_text(
    s: &mut Surface,
    text: &str,
    x: i32,
    y: i32,
    color: Rgb,
    shadow: Rgb,
    scale: i32,
) {
    let mut cx = x;
    for ch in text.chars() {
        let Some(g) = glyph(ch) else {
            cx += 3 * scale;
            continue;
        };
        let top = y - (g.len() as i32 - 5) * scale;
        for pass in [true, false] {
            for (j, row) in g.iter().enumerate() {
                for (i, bit) in row.chars().enumerate() {
                    if bit != '1' {
                        continue;
                    }
                    for a in 0..scale {
                        for b in 0..scale {
                            let (px, py) = (cx + i as i32 * scale + a, top + j as i32 * scale + b);
                            if pass {
                                s.set_i(px + 1, py + 1, shadow, 1.0);
                            } else {
                                s.set_i(px, py, color, 1.0);
                            }
                        }
                    }
                }
            }
        }
        cx += (g[0].len() as i32 + 1) * scale;
    }
}

fn load_layer(bytes: &[u8]) -> Surface {
    let image = image::load_from_memory(bytes)
        .expect("baked loading layer")
        .into_rgba8();
    Surface::from_rgba(
        image.width() as usize,
        image.height() as usize,
        image.into_raw(),
    )
}

// ---------------------------------------------------------------- banquet

const QUEEN_AT: (f64, f64) = (140.0, 2.0);
const CANDELABRA: [(f64, f64); 2] = [(168.0, 170.0), (312.0, 170.0)];
const CHANDELIERS: [f64; 2] = [132.0, 348.0];
const BITES: i32 = 3;

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    color: Rgb,
    bone: bool,
}

struct Pop {
    text: &'static str,
    x: f64,
    y: f64,
    life: f64,
}

/// "Banquete real": the Queen devours a royal feast in the night hall, under
/// the two moons. Mash to make her eat faster; empty plates pile up.
pub struct Banquet {
    palette: Rc<Palette>,
    hall: Surface,
    table: Surface,
    plate: Surface,
    queen: PoseCache,
    frame: Surface,
    rng: Rng,
    t: f64,
    cycle: f64,
    bites: i32,
    meat: f64,
    plates: i32,
    eaten: i32,
    particles: Vec<Particle>,
    pops: Vec<Pop>,
    grabbing: f64,
    speed: f64,
}

impl Banquet {
    pub fn new(palette: Rc<Palette>, seed: u64) -> Self {
        Self {
            palette,
            hall: load_layer(include_bytes!("../../data/loading/banquet_hall.png")),
            table: load_layer(include_bytes!("../../data/loading/banquet_table.png")),
            plate: load_layer(include_bytes!("../../data/loading/banquet_plate.png")),
            queen: PoseCache::new(96),
            frame: Surface::new(W, H),
            rng: Rng::new(seed),
            t: 0.0,
            cycle: 0.0,
            bites: 0,
            meat: 1.0,
            plates: 3,
            eaten: 3,
            particles: Vec::new(),
            pops: Vec::new(),
            grabbing: 0.0,
            speed: 1.0,
        }
    }

    fn pose(&self) -> QueenPose {
        let c = self.cycle;
        let mut p = QueenPose {
            has_meat: self.grabbing <= 0.0 || self.grabbing < 0.5,
            meat: Some(self.meat),
            eyes: "open",
            mouth: "smile",
            puff: 0.0,
            hy: 0.0,
            ..Default::default()
        };
        // Resting, the forearm points toward us (foreshortened); raised to her
        // mouth it lies flat in the picture plane at full length.
        let rest = [85.0, 143.0];
        let mouth = [92.0, 88.0];
        let lerp =
            |a: [f64; 2], b: [f64; 2], t: f64| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        if self.grabbing > 0.0 {
            // Reach below the table edge for another drumstick.
            let g = self.grabbing;
            let k = if g > 0.5 { (1.0 - g) * 2.0 } else { g * 2.0 };
            p.hand = Some(lerp(rest, [80.0, 184.0], k));
            p.forearm = Some(21.0 + 8.0 * k);
            p.has_meat = g < 0.5;
            p.meat = Some(1.0);
            p.goblet_hand = Some([117.0, 145.0]);
            return p;
        }
        let reach = |k: f64, p: &mut QueenPose| {
            p.hand = Some(lerp(rest, mouth, k));
            p.forearm = Some(21.0 + 9.0 * k);
        };
        if c < 0.22 {
            let k = c / 0.22;
            reach(k * k * (3.0 - 2.0 * k), &mut p);
            p.mouth = if k > 0.6 { "chomp" } else { "smile" };
        } else if c < 0.34 {
            reach(1.0, &mut p);
            p.mouth = "chomp";
            p.eyes = "happy";
        } else {
            let k = clamp((c - 0.34) / 0.22, 0.0, 1.0);
            reach(1.0 - k * k * (3.0 - 2.0 * k), &mut p);
            p.eyes = "happy";
            let chew = ((c - 0.34) * 18.0).floor() as i64 % 2;
            p.mouth = if chew != 0 { "chewA" } else { "chewB" };
            p.puff = 1.0;
            p.hy = if chew != 0 { 1.0 } else { 0.0 };
        }
        p
    }

    fn bite(&mut self) {
        self.bites += 1;
        self.meat = (1.0 - self.bites as f64 / BITES as f64).max(0.0);
        let mx = QUEEN_AT.0 + QUEEN_MOUTH.0;
        let my = QUEEN_AT.1 + QUEEN_MOUTH.1;
        for _ in 0..7 {
            let color = if self.rng.next() < 0.5 {
                [206.0, 120.0, 40.0]
            } else {
                [250.0, 190.0, 90.0]
            };
            self.particles.push(Particle {
                x: mx + (self.rng.next() - 0.5) * 6.0,
                y: my,
                vx: (self.rng.next() - 0.5) * 70.0,
                vy: -30.0 - self.rng.next() * 50.0,
                life: 0.8,
                color,
                bone: false,
            });
        }
        // One pop at a time, so mashing never piles the letters up.
        if !self.pops.iter().any(|p| p.life > 0.35) {
            self.pops = vec![Pop {
                text: "ÑAM!",
                x: mx + 20.0 + self.rng.next() * 12.0,
                y: my - 26.0,
                life: 0.9,
            }];
        }
        if self.bites >= BITES {
            self.grabbing = 1.0;
            self.bites = 0;
            self.eaten += 1;
            // The bare bone is tossed onto the stack of empty plates.
            self.particles.push(Particle {
                x: mx - 10.0,
                y: my + 20.0,
                vx: 150.0,
                vy: -120.0,
                life: 1.2,
                color: [246.0, 238.0, 226.0],
                bone: true,
            });
        }
    }
}

impl Scene for Banquet {
    fn title(&self) -> &'static str {
        "Banquete real"
    }
    fn subtitle(&self) -> &'static str {
        "La Reina no deja ni las migas"
    }
    fn counter_label(&self) -> &'static str {
        "Platos"
    }
    fn counter(&self) -> i32 {
        self.eaten
    }
    fn tips(&self) -> &'static [&'static str] {
        &[
            "El cuerpo de la Reina recuerda el hambre de Gorou: el bufé fue lo último que vio en la Tierra.",
            "Nyxara recomienda no pasar de doce platos seguidos. La Reina nunca pasa de doce... por plato.",
            "Kuro sirve la cena a medianoche, bajo las dos lunas. Nadie se atreve a llegar tarde.",
            "Cuanto más rápido machacas, más rápido desaparece el pavo real.",
        ]
    }

    fn update(&mut self, dt: f64, mash: bool) {
        self.t += dt;
        self.speed += ((if mash { 2.6 } else { 1.0 }) - self.speed) * (dt * 4.0).min(1.0);
        if self.grabbing > 0.0 {
            self.grabbing -= dt * 1.8 * self.speed;
            if self.grabbing <= 0.0 {
                self.grabbing = 0.0;
                self.meat = 1.0;
                self.cycle = 0.0;
            }
        } else {
            let before = self.cycle;
            self.cycle += dt * 1.05 * self.speed;
            if before < 0.28 && self.cycle >= 0.28 {
                self.bite();
            }
            if self.cycle >= 1.0 {
                self.cycle = 0.0;
            }
        }
        for p in &mut self.particles {
            p.life -= dt;
            p.vy += 260.0 * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.bone && p.y > 150.0 && p.vy > 0.0 {
                p.life = 0.0;
                self.plates += 1;
                if self.plates > 22 {
                    self.plates = 4;
                }
            }
        }
        self.particles.retain(|p| p.life > 0.0);
        for p in &mut self.pops {
            p.life -= dt;
            p.y -= dt * 18.0;
        }
        self.pops.retain(|p| p.life > 0.0);
    }

    fn draw(&mut self, t: f64) -> &Surface {
        self.frame.copy_from(&self.hall);
        let iron = ramp(&["#0a080c", "#18141c", "#2a2430", "#403848"]);
        // Chandeliers: iron rings of candles hung from the vault.
        for cx in CHANDELIERS {
            for k in -24..=24 {
                let y = 18.0 + jr((1.0 - (k as f64 / 24.0).powi(2)).max(0.0).sqrt() * 3.0);
                self.frame.set(cx + k as f64, y, iron[2], 1.0);
                self.frame.set(cx + k as f64, y + 1.0, iron[1], 1.0);
            }
            for k in -2..=2 {
                let y = 18.0 + jr((1.0 - (k as f64 / 2.4).powi(2)).sqrt() * 3.0) - 1.0;
                candle(
                    &mut self.frame,
                    cx + k as f64 * 11.0,
                    y,
                    t,
                    5,
                    2,
                    cx + k as f64,
                    18.0,
                );
            }
        }
        let mut pose = self.pose();
        if let Some(h) = pose.hand {
            pose.hand = Some([jr(h[0]), jr(h[1])]);
        }
        if let Some(f) = pose.forearm {
            pose.forearm = Some(jr(f));
        }
        let key = format!("{pose:?}");
        let palette = self.palette.clone();
        let q = self.queen.get(key, || draw_queen(&palette, &pose));
        self.frame.blit(&q, QUEEN_AT.0, QUEEN_AT.1, false);
        self.frame.blit(&self.table, 0.0, 0.0, false);
        // The stack of empty plates beside her.
        for k in 0..self.plates {
            let jitter = jr((hash(k as f64, 3.0, 9.0) - 0.5) * 3.0);
            self.frame
                .blit(&self.plate, 318.0 + jitter, 172.0 - k as f64 * 3.0, false);
        }
        for (x, y) in CANDELABRA {
            for (dx, h) in [(-14.0, 48.0), (0.0, 52.0), (14.0, 48.0)] {
                flame(&mut self.frame, x + dx, y - h - 1.0, t, 1.0, x + dx);
                glow(
                    &mut self.frame,
                    x + dx,
                    y - h - 4.0,
                    30.0,
                    [255.0, 150.0, 60.0],
                    0.28,
                    5.0,
                );
            }
        }
        let bone = [246.0, 238.0, 226.0];
        for p in &self.particles {
            if p.bone {
                for k in -2..=2 {
                    self.frame.set(p.x + k as f64, p.y, bone, 1.0);
                }
                for (dx, dy) in [(-3.0, -1.0), (-3.0, 1.0), (3.0, -1.0), (3.0, 1.0)] {
                    self.frame.set(p.x + dx, p.y + dy, bone, 1.0);
                }
            } else {
                self.frame.set(p.x, p.y, p.color, 1.0);
            }
        }
        for p in &self.pops {
            pixel_text(
                &mut self.frame,
                p.text,
                jr(p.x) as i32,
                jr(p.y) as i32,
                [255.0, 226.0, 120.0],
                [90.0, 20.0, 40.0],
                2,
            );
        }
        // Motes of dust drifting through the moonlight.
        for k in 0..26 {
            let kf = k as f64;
            let x =
                (hash(kf, 1.0, 4.0) * W as f64 + t * (4.0 + hash(kf, 2.0, 4.0) * 5.0)) % W as f64;
            let y = (hash(kf, 5.0, 4.0) * 170.0 + (t * 0.7 + kf).sin() * 6.0 + 170.0) % 170.0;
            if (t * 2.0 + kf).sin() > 0.2 {
                self.frame.add(x, y, [140.0, 120.0, 200.0], 0.5);
            }
        }
        &self.frame
    }
}

// ------------------------------------------------------------- infirmary

const NYX_AT: (f64, f64) = (140.0, 14.0);
const WINDOW_CX: f64 = 92.0;
const DOSE_PER_DROP: f64 = 0.1;
const TONIC: [Rgb; 5] = [
    [90.0, 230.0, 160.0],
    [80.0, 200.0, 240.0],
    [140.0, 120.0, 255.0],
    [220.0, 100.0, 255.0],
    [255.0, 110.0, 190.0],
];

fn tonic_color(dose: f64) -> Rgb {
    let f = clamp(dose, 0.0, 0.999) * (TONIC.len() as f64 - 1.0);
    let i = f.floor() as usize;
    let c = mix(TONIC[i], TONIC[i + 1], f - i as f64);
    [jr(c[0]), jr(c[1]), jr(c[2])]
}

struct Drop {
    y: f64,
    v: f64,
}

struct Bubble {
    x: f64,
    y: f64,
    life: f64,
}

struct TonicPop {
    text: &'static str,
    life: f64,
    y: f64,
}

/// "Enfermería real": Nyxara doses a tonic drop by drop under the stained
/// glass, between shelves of remedies and the patient's bed. Mash to squeeze
/// the dropper; a full flask becomes a finished tonic.
pub struct Infirmary {
    palette: Rc<Palette>,
    room: Surface,
    bench: Surface,
    nyx: PoseCache,
    frame: Surface,
    rng: Rng,
    t: f64,
    dose: f64,
    drops: Vec<Drop>,
    timer: f64,
    squeeze: f64,
    tonics: i32,
    flash: f64,
    bubbles: Vec<Bubble>,
    blink: f64,
    pops: Vec<TonicPop>,
}

impl Infirmary {
    pub fn new(palette: Rc<Palette>, seed: u64) -> Self {
        Self {
            palette,
            room: load_layer(include_bytes!("../../data/loading/infirmary_room.png")),
            bench: load_layer(include_bytes!("../../data/loading/infirmary_bench.png")),
            nyx: PoseCache::new(96),
            frame: Surface::new(W, H),
            rng: Rng::new(seed),
            t: 0.0,
            dose: 0.1,
            drops: Vec::new(),
            timer: 0.4,
            squeeze: 0.0,
            tonics: 0,
            flash: 0.0,
            bubbles: Vec::new(),
            blink: 2.5,
            pops: Vec::new(),
        }
    }
}

impl Scene for Infirmary {
    fn title(&self) -> &'static str {
        "Enfermería real"
    }
    fn subtitle(&self) -> &'static str {
        "Nyxara prepara el tónico de la Reina"
    }
    fn counter_label(&self) -> &'static str {
        "Tónicos"
    }
    fn counter(&self) -> i32 {
        self.tonics
    }
    fn tips(&self) -> &'static [&'static str] {
        &[
            "Nyxara mide el sello con un disco de cristal tibio. Si se enfría, algo va mal.",
            "El tónico violeta calma el sello... y la indigestión de doce platos de pavo.",
            "Las gafas de Nyxara no son de adorno: lee etiquetas de un milímetro a la luz de la luna.",
            "Pulsa con ritmo: cada gota cuenta, pero el matraz no se llena solo.",
        ]
    }

    fn update(&mut self, dt: f64, mash: bool) {
        self.t += dt;
        self.timer -= dt * if mash { 3.2 } else { 1.0 };
        if self.timer <= 0.0 && self.flash <= 0.0 {
            self.drops.push(Drop { y: 0.0, v: 0.0 });
            self.squeeze = 1.0;
            self.timer = 1.0;
        }
        self.squeeze = (self.squeeze - dt * 5.0).max(0.0);
        for d in &mut self.drops {
            d.v += 180.0 * dt;
            d.y += d.v * dt;
        }
        let fall = FLASK.neck_top - FLASK.tip[1] + 6.0;
        let drops = std::mem::take(&mut self.drops);
        for d in drops {
            if d.y < fall {
                self.drops.push(d);
                continue;
            }
            self.dose = (self.dose + DOSE_PER_DROP).min(1.0);
            for _ in 0..3 {
                let x = (self.rng.next() - 0.5) * 12.0;
                let life = 0.8 + self.rng.next() * 0.5;
                self.bubbles.push(Bubble { x, y: 6.0, life });
            }
            if self.dose >= 1.0 && self.flash <= 0.0 {
                self.flash = 1.6;
                self.tonics += 1;
                self.pops.push(TonicPop {
                    text: "+1",
                    life: 1.2,
                    y: 0.0,
                });
            }
        }
        if self.flash > 0.0 {
            self.flash -= dt;
            if self.flash <= 0.0 {
                self.dose = 0.1;
            }
        }
        let t = self.t;
        for b in &mut self.bubbles {
            b.y -= dt * 14.0;
            b.life -= dt;
            b.x += (t * 8.0 + b.y).sin() * dt * 4.0;
        }
        self.bubbles.retain(|b| b.life > 0.0);
        if self.rng.next() < dt * 3.0 {
            let x = (self.rng.next() - 0.5) * 12.0;
            let life = 0.6 + self.rng.next() * 0.4;
            self.bubbles.push(Bubble { x, y: 6.0, life });
        }
        for p in &mut self.pops {
            p.life -= dt;
            p.y -= dt * 16.0;
        }
        self.pops.retain(|p| p.life > 0.0);
        self.blink -= dt;
        if self.blink < -0.12 {
            self.blink = 2.5 + self.rng.next() * 2.5;
        }
    }

    fn draw(&mut self, t: f64) -> &Surface {
        self.frame.copy_from(&self.room);
        // Candles on the shelves and the bed-side stool.
        for (x, y, h) in [(330.0, 50.0, 8), (466.0, 126.0, 7), (334.0, 164.0, 9)] {
            candle(&mut self.frame, x, y - 1.0, t, h, 3, x, 26.0);
        }
        let happy = self.flash > 0.0;
        let color = if happy {
            [255.0, 150.0, 220.0]
        } else {
            tonic_color(self.dose)
        };
        let phase = (t * 0.45) % 1.0;
        let pose = NyxPose {
            hy: jr((t * 1.6).sin() * 0.6),
            eyes: if happy {
                "shut"
            } else if self.blink < 0.0 {
                "blink"
            } else {
                "focus"
            },
            mouth: if happy { "smile" } else { "calm" },
            squeeze: jr(self.squeeze * 3.0) / 3.0,
            level: jr((0.15 + self.dose * 0.7) * 24.0) / 24.0,
            liquid: [
                jr(color[0] / 6.0) * 6.0,
                jr(color[1] / 6.0) * 6.0,
                jr(color[2] / 6.0) * 6.0,
            ],
            glint: if phase < 0.3 {
                jr((phase / 0.3) * 10.0) / 10.0
            } else {
                0.0
            },
        };
        let key = format!("{pose:?}");
        let palette = self.palette.clone();
        let nyx = self.nyx.get(key, || draw_nyx(&palette, &pose));
        self.frame.blit(&nyx, NYX_AT.0, NYX_AT.1, false);
        let (fx, fy) = (NYX_AT.0 + FLASK.x, NYX_AT.1 + FLASK.y);
        // The tonic lights her from below.
        glow(
            &mut self.frame,
            fx,
            fy,
            34.0 + if happy { 18.0 } else { 0.0 },
            color,
            if happy { 0.55 } else { 0.3 },
            5.0,
        );
        for b in &self.bubbles {
            let x = jr(fx + b.x * 0.7);
            let y = jr(fy + b.y - (1.0 - b.life) * 10.0);
            if (x - fx).powi(2) + (y - fy).powi(2) < 70.0 {
                self.frame.set(
                    x,
                    y,
                    [
                        (color[0] + 90.0).min(255.0),
                        (color[1] + 90.0).min(255.0),
                        (color[2] + 90.0).min(255.0),
                    ],
                    1.0,
                );
            }
        }
        for d in &self.drops {
            let x = NYX_AT.0 + FLASK.tip[0];
            let y = NYX_AT.1 + FLASK.tip[1] + 2.0 + d.y;
            self.frame.set(
                x,
                y,
                [
                    (color[0] + 60.0).min(255.0),
                    (color[1] + 60.0).min(255.0),
                    (color[2] + 60.0).min(255.0),
                ],
                1.0,
            );
            self.frame.set(x, y + 1.0, color, 1.0);
        }
        if happy {
            for k in 0..8 {
                let a = t * 2.0 + (k as f64 / 8.0) * std::f64::consts::PI * 2.0;
                let r = 16.0 + (1.6 - self.flash) * 14.0;
                self.frame.set(
                    fx + a.cos() * r,
                    fy + a.sin() * r * 0.8,
                    [255.0, 240.0, 250.0],
                    1.0,
                );
            }
        }
        self.frame.blit(&self.bench, 0.0, 0.0, false);
        // Spirit lamp under the alembic and its bubbling flask.
        flame(&mut self.frame, 112.0, 176.0, t, 1.0, 3.0);
        glow(
            &mut self.frame,
            112.0,
            174.0,
            20.0,
            [255.0, 140.0, 60.0],
            0.3,
            5.0,
        );
        for k in 0..3 {
            let phase = (t * 1.3 + k as f64 * 0.33) % 1.0;
            self.frame.set(
                110.0 + k as f64 * 3.0,
                166.0 - phase * 8.0,
                [180.0, 255.0, 210.0],
                1.0,
            );
        }
        glow(
            &mut self.frame,
            112.0,
            162.0,
            16.0,
            [90.0, 220.0, 150.0],
            0.18,
            5.0,
        );
        glow(
            &mut self.frame,
            285.0,
            177.0,
            18.0,
            [170.0, 110.0, 255.0],
            0.22 + (t * 2.2).sin() * 0.08,
            5.0,
        );
        for p in &self.pops {
            pixel_text(
                &mut self.frame,
                p.text,
                (fx + 16.0) as i32,
                jr(fy - 40.0 + p.y) as i32,
                [255.0, 200.0, 240.0],
                [70.0, 10.0, 50.0],
                2,
            );
        }
        // Dust turning in the moonbeams.
        for k in 0..30 {
            let kf = k as f64;
            let x = (hash(kf, 1.0, 8.0) * 240.0 + t * (3.0 + hash(kf, 2.0, 8.0) * 4.0)) % 240.0;
            let y = (hash(kf, 5.0, 8.0) * 200.0 + 20.0 + (t * 0.8 + kf).sin() * 5.0) % 220.0 + 20.0;
            let u = x - (WINDOW_CX - 30.0) - (y - 20.0) * 1.05;
            if u > 0.0 && u < 110.0 && (t * 1.7 + kf).sin() > 0.0 {
                self.frame.add(x, y, [150.0, 170.0, 255.0], 0.55);
            }
        }
        &self.frame
    }
}
