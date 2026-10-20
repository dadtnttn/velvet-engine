//! Papers on a desk: the 2D pieces of the engine together.
//!
//! - Drag a paper; it comes on top and stays on the desk. Drop it on the window (top) to hand it back.
//! - Double-click a paper to read it big; click to put it down.
//! - Space (or the right button) turns the magnifier on: click two fields to compare them.
//! - A / R stamps the paper under the pointer (approved / refused). N brings every paper back. F12: screenshot.
//!
//! ```text
//! cargo run -p desk-demo --release
//! cargo run -p desk-demo --release -- --capture out.png     (no window: a scripted scene, saved to a PNG)
//! cargo run -p desk-demo --release -- --bench               (twelve papers at 2560×1440, timed)
//! ```

use velvet_canvas::{Align, Canvas, Color, Fill, Font, Point as CPoint, Rect as CRect, StrokeStyle, TextStyle};
use velvet_desk::{Desk, DeskEvent, Point, Rect};
use velvet_host2d::{Button, Game2D, Headless, HostConfig, Input, InputEvent, KeyCode};
use velvet_sfx::{presets, Player, Sound};

const FONT: &[u8] = include_bytes!("../../../crates/velvet-canvas/tests/fixtures/Nunito-ExtraBold.ttf");

const INK: Color = Color::hex(0x2a1b10);
const PAPER: Color = Color::hex(0xf4e7cb);
const PAPER_EDGE: Color = Color::hex(0xe2cb9b);
const NIGHT: Color = Color::hex(0x1a1326);
const GOLD: Color = Color::hex(0xe8c27a);
const GREEN: Color = Color::hex(0x2e7d4f);
const RED: Color = Color::hex(0xa3253a);

/// The window, where papers are handed back.
const WINDOW: Rect = Rect::new(720.0, 40.0, 480.0, 280.0);
/// The desk, under the window.
const DESK: Rect = Rect::new(0.0, 340.0, 1920.0, 740.0);

/// A paper of the demo: its title, its fields (label, value, place in the paper) and its stamp.
struct Paper {
    id: u64,
    title: &'static str,
    w: f32,
    h: f32,
    fields: Vec<(&'static str, &'static str, &'static str, Rect)>,
    stamp: Option<(bool, f32)>,
    /// The paper drawn straight (for reading it big).
    layer: Option<Canvas>,
    /// The paper drawn already turned, with its shadow, and where its corner sits in that layer: on the desk it is
    /// only copied (a turned copy costs a resampling every frame).
    baked: Option<(Canvas, CPoint, f32)>,
    /// The same, lifted in the hand: a bigger, softer shadow.
    lifted: Option<(Canvas, CPoint, f32)>,
}

/// `n` papers, the three kinds in turn.
fn many_papers(n: usize) -> Vec<Paper> {
    (0..n)
        .map(|i| {
            let mut p = papers().swap_remove(i % 3);
            p.id = i as u64 + 1;
            p
        })
        .collect()
}

fn papers() -> Vec<Paper> {
    let row = |i: f32| Rect::new(24.0, 74.0 + i * 52.0, 300.0, 44.0);
    vec![
        Paper {
            id: 1,
            title: "TARJETA DE BIBLIOTECA",
            w: 420.0,
            h: 300.0,
            fields: vec![
                ("name", "TITULAR", "Ada Fairbanks", row(0.0)),
                ("city", "CIUDAD", "Valcresta", row(1.0)),
                ("until", "VENCE", "12 · 05 · 1923", row(2.0)),
                ("number", "NÚMERO", "VC-2231-88", row(3.0)),
            ],
            stamp: None,
            layer: None,
            baked: None,
            lifted: None,
        },
        Paper {
            id: 2,
            title: "BILLETE DE TREN",
            w: 460.0,
            h: 260.0,
            fields: vec![
                ("name", "VIAJERA", "Ada Fairbanks", row(0.0)),
                ("city", "DESTINO", "Velmira", row(1.0)),
                ("until", "FECHA", "12 · 05 · 1923", row(2.0)),
            ],
            stamp: None,
            layer: None,
            baked: None,
            lifted: None,
        },
        Paper {
            id: 3,
            title: "CARTA DE PRESENTACIÓN",
            w: 400.0,
            h: 320.0,
            fields: vec![
                ("name", "FIRMA", "Ada Fairbank", row(0.0)),
                ("city", "DESDE", "Valcresta", row(1.0)),
                ("number", "REFERENCIA", "VC-2231-88", row(3.0)),
            ],
            stamp: None,
            layer: None,
            baked: None,
            lifted: None,
        },
    ]
}

struct Demo {
    font: Font,
    desk: Desk,
    papers: Vec<Paper>,
    magnifier: bool,
    notice: Option<(String, f32)>,
    time: f32,
    pointer: Option<Point>,
    /// The office (the night, the window, the desk), drawn once: its gradients are the slowest thing to draw.
    office: Option<Canvas>,
    /// The sounds, made by code, and where they play (none without a window).
    sounds: Sounds,
    player: Option<Player>,
}

struct Sounds {
    paper: Sound,
    stamp: Sound,
    chime: Sound,
    click: Sound,
}

impl Demo {
    fn new() -> Demo {
        let mut demo = Demo { font: Font::from_bytes(FONT.to_vec()).expect("font"), desk: Desk::new(DESK), papers: papers(), magnifier: false, notice: None, time: 0.0, pointer: None, office: None, sounds: Sounds { paper: presets::paper(), stamp: presets::stamp(), chime: presets::chime(), click: presets::click() }, player: None };
        demo.desk.add_zone("window", WINDOW);
        demo.lay_out();
        demo
    }

    /// Every paper on the desk, a little apart and a little turned.
    fn lay_out(&mut self) {
        for (i, paper) in self.papers.iter().enumerate() {
            let at = Point::new(220.0 + (i % 3) as f32 * 470.0 + (i / 3) as f32 * 60.0, 470.0 + (i % 2) as f32 * 120.0 + (i / 3) as f32 * 40.0);
            self.desk.add_item(paper.id, paper.w, paper.h, at, [-3.0, 2.0, -1.5][i % 3]);
            for (name, _, _, rect) in &paper.fields {
                self.desk.add_field(paper.id, name, *rect);
            }
        }
        self.desk.clear_marks();
    }

    fn paper(&self, id: u64) -> Option<&Paper> {
        self.papers.iter().find(|p| p.id == id)
    }

    fn sound(&mut self, which: fn(&Sounds) -> &Sound, name: &str) {
        if let Some(player) = &mut self.player {
            player.play(name, which(&self.sounds), 0.8, std::time::Duration::from_millis(60));
        }
    }

    fn say(&mut self, text: String) {
        self.notice = Some((text, self.time + 2.5));
    }

    /// Draws a paper into its layer (again when it was stamped or the pixels changed).
    fn draw_paper(font: &Font, frame: &Canvas, paper: &mut Paper) {
        let mut layer = frame.layer(paper.w, paper.h);
        let body = CRect::new(0.0, 0.0, paper.w, paper.h);
        layer.fill_round_rect(body, 12.0, &Fill::Linear { from: CPoint::new(0.0, 0.0), to: CPoint::new(0.0, paper.h), stops: vec![(0.0, PAPER), (1.0, PAPER_EDGE)] });
        layer.stroke_round_rect(body.inflate(-1.5), 11.0, &StrokeStyle::new(3.0, Color::hex(0x6b4a2b)));
        layer.stroke_round_rect(body.inflate(-10.0), 7.0, &StrokeStyle::new(1.5, Color::hex(0xb08a55)).dashed(&[7.0, 5.0]));
        let title = TextStyle::new(font, 21.0, Color::hex(0x4a2c58)).letter_spacing(1.2).align(Align::Center);
        layer.text(paper.title, paper.w / 2.0, 22.0, &title);
        let label = TextStyle::new(font, 12.5, Color::hex(0x8a6a44)).letter_spacing(0.8);
        let value = TextStyle::new(font, 19.0, INK);
        for (_, l, v, r) in &paper.fields {
            layer.text(l, r.x, r.y, &label);
            layer.text(v, r.x, r.y + 15.0, &value);
        }
        if let Some((approved, _)) = paper.stamp {
            Self::draw_stamp(font, &mut layer, paper.w - 92.0, paper.h - 78.0, approved, 1.0);
        }
        paper.layer = Some(layer);
        paper.baked = None;
        paper.lifted = None;
    }

    /// Draws the straight paper turned, with its shadow (resting, or lifted in the hand), into a layer of its own.
    fn bake(frame: &Canvas, paper: &Paper, rotation: f32, lifted: bool) -> Option<(Canvas, CPoint, f32)> {
        let flat = paper.layer.as_ref()?;
        let (sin, cos) = rotation.to_radians().sin_cos();
        let corners = [(0.0, 0.0), (paper.w, 0.0), (0.0, paper.h), (paper.w, paper.h)].map(|(x, y)| (x * cos - y * sin, x * sin + y * cos));
        let (x0, y0) = (corners.iter().map(|c| c.0).fold(f32::MAX, f32::min), corners.iter().map(|c| c.1).fold(f32::MAX, f32::min));
        let (x1, y1) = (corners.iter().map(|c| c.0).fold(f32::MIN, f32::max), corners.iter().map(|c| c.1).fold(f32::MIN, f32::max));
        let pad = if lifted { 60.0 } else { 32.0 };
        let mut baked = frame.layer(x1 - x0 + 2.0 * pad, y1 - y0 + 2.0 * pad);
        let origin = CPoint::new(pad - x0, pad - y0);
        baked.save();
        baked.translate(origin.x, origin.y);
        baked.rotate(rotation);
        if lifted {
            baked.shadow(CRect::new(0.0, 0.0, paper.w, paper.h), 12.0, 22.0, CPoint::new(0.0, 18.0), Color::BLACK.fade(0.5));
        } else {
            baked.shadow(CRect::new(0.0, 0.0, paper.w, paper.h), 12.0, 10.0, CPoint::new(0.0, 7.0), Color::BLACK.fade(0.45));
        }
        baked.restore();
        baked.draw_layer(flat, origin, 1.0, rotation, 1.0);
        Some((baked, origin, rotation))
    }

    fn draw_stamp(font: &Font, c: &mut Canvas, x: f32, y: f32, approved: bool, scale: f32) {
        let color = if approved { GREEN } else { RED }.fade(0.85);
        c.save();
        c.translate(x, y);
        c.rotate(-12.0);
        c.scale(scale);
        c.stroke_round_rect(CRect::new(-74.0, -26.0, 148.0, 52.0), 10.0, &StrokeStyle::new(5.0, color));
        let text = TextStyle::new(font, 22.0, color).letter_spacing(2.0).align(Align::Center);
        c.text(if approved { "APROBADO" } else { "RECHAZADO" }, 0.0, -14.0, &text);
        c.restore();
    }

    /// What the two marks say: the same, or not.
    fn comparison(&self) -> Option<(String, bool)> {
        let (a, b) = self.desk.pair()?;
        let value = |owner: u64, field: &str| self.paper(owner)?.fields.iter().find(|f| f.0 == field).map(|f| f.2);
        let (va, vb) = (value(a.owner, &a.field)?, value(b.owner, &b.field)?);
        let same = va == vb;
        Some((if same { format!("Coinciden: «{va}»") } else { format!("No coinciden: «{va}» / «{vb}»") }, same))
    }
}

impl Game2D for Demo {
    fn update(&mut self, input: &Input, dt: f32) {
        self.time = input.time;
        for event in input.events() {
            match event {
                InputEvent::PointerMoved(p) => {
                    self.pointer = Some(Point::new(p.x, p.y));
                    self.desk.pointer_move(Point::new(p.x, p.y));
                    self.desk.refresh_marks();
                }
                InputEvent::PointerLeft => self.pointer = None,
                InputEvent::PointerDown { button: Button::Left, at } => {
                    if self.desk.magnified().is_some() {
                        self.desk.set_magnified(None);
                        continue;
                    }
                    if let Some(DeskEvent::Picked { .. }) = self.desk.pointer_down(Point::new(at.x, at.y), self.time) {
                        self.sound(|s| &s.paper, "paper");
                    }
                }
                InputEvent::PointerDown { button: Button::Right, .. } => self.magnifier = !self.magnifier,
                InputEvent::PointerUp { button: Button::Left, at } => match self.desk.pointer_up(Point::new(at.x, at.y), self.time) {
                    Some(DeskEvent::Dropped { item, zone: Some(zone) }) if zone == "window" => {
                        let title = self.paper(item).map(|p| p.title).unwrap_or("");
                        self.desk.remove_item(item);
                        self.say(format!("Devuelto: {}", title.to_lowercase()));
                        self.sound(|s| &s.chime, "chime");
                    }
                    Some(DeskEvent::Clicked { item, field: Some(field), .. }) if self.magnifier => {
                        self.desk.mark_field(item, &field);
                        self.sound(|s| &s.click, "click");
                    }
                    Some(DeskEvent::DoubleClicked { item, .. }) if !self.magnifier => self.desk.set_magnified(Some(item)),
                    _ => {}
                },
                InputEvent::KeyDown(KeyCode::Space) => self.magnifier = !self.magnifier,
                InputEvent::KeyDown(key @ (KeyCode::KeyA | KeyCode::KeyR)) => {
                    let under = self.pointer.and_then(|p| self.desk.item_at(p));
                    if let Some(paper) = under.and_then(|id| self.papers.iter_mut().find(|p| p.id == id)) {
                        paper.stamp = Some((*key == KeyCode::KeyA, self.time));
                        paper.layer = None;
                        self.sound(|s| &s.stamp, "stamp");
                    }
                }
                InputEvent::KeyDown(KeyCode::KeyN) => {
                    for p in &mut self.papers {
                        p.stamp = None;
                        p.layer = None;
                    }
                    self.lay_out();
                }
                _ => {}
            }
        }
        let _ = dt;
    }

    fn pixels_changed(&mut self, _canvas: &Canvas) {
        for p in &mut self.papers {
            p.layer = None;
        }
        self.office = None;
    }

    fn draw(&mut self, c: &mut Canvas) {
        let font = self.font.clone();
        // The office: the night behind the window, the counter, the desk — drawn once into a layer.
        if self.office.is_none() {
            let mut o = c.layer(1920.0, 1080.0);
            o.fill_rect(CRect::new(0.0, 0.0, 1920.0, 1080.0), &Fill::Linear { from: CPoint::new(0.0, 0.0), to: CPoint::new(0.0, 340.0), stops: vec![(0.0, Color::hex(0x2b1d45)), (1.0, NIGHT)] });
            o.fill_round_rect(CRect::new(WINDOW.x, WINDOW.y, WINDOW.w, WINDOW.h), 18.0, &Fill::Radial { center: CPoint::new(960.0, 120.0), radius: 320.0, stops: vec![(0.0, Color::hex(0x5b3f86)), (1.0, Color::hex(0x231737))] });
            o.stroke_round_rect(CRect::new(WINDOW.x, WINDOW.y, WINDOW.w, WINDOW.h), 18.0, &StrokeStyle::new(6.0, GOLD));
            let hint = TextStyle::new(&font, 18.0, GOLD.fade(0.8)).align(Align::Center);
            o.text("Suelta aquí para devolver", 960.0, WINDOW.bottom() - 40.0, &hint);
            o.fill_rect(CRect::new(DESK.x, DESK.y, DESK.w, DESK.h), &Fill::Linear { from: CPoint::new(0.0, DESK.y), to: CPoint::new(0.0, 1080.0), stops: vec![(0.0, Color::hex(0x4a2a1c)), (1.0, Color::hex(0x2a160f))] });
            o.fill_rect(CRect::new(0.0, DESK.y - 6.0, 1920.0, 10.0), &Color::hex(0x6e4430).into());
            self.office = Some(o);
        }
        if let Some(office) = &self.office {
            c.draw_layer(office, CPoint::new(0.0, 0.0), 1.0, 0.0, 1.0);
        }

        // The papers, bottom to top, each from its layer.
        let ids: Vec<u64> = self.desk.items().iter().map(|i| i.id).collect();
        for id in ids {
            let Some(item) = self.desk.item(id).cloned() else { continue };
            let Some(index) = self.papers.iter().position(|p| p.id == id) else { continue };
            if self.papers[index].layer.is_none() {
                Self::draw_paper(&font, c, &mut self.papers[index]);
            }
            // A paper in the hand casts a bigger, softer shadow.
            let lifted = self.desk.dragging() == Some(id);
            let stale = |b: &Option<(Canvas, CPoint, f32)>| b.as_ref().map(|b| b.2) != Some(item.rotation);
            if lifted && stale(&self.papers[index].lifted) {
                self.papers[index].lifted = Self::bake(c, &self.papers[index], item.rotation, true);
            }
            if !lifted && stale(&self.papers[index].baked) {
                self.papers[index].baked = Self::bake(c, &self.papers[index], item.rotation, false);
            }
            let shown = if lifted { &self.papers[index].lifted } else { &self.papers[index].baked };
            if let Some((baked, origin, _)) = shown {
                c.draw_layer(baked, CPoint::new(item.pos.x - origin.x, item.pos.y - origin.y), 1.0, 0.0, 1.0);
            }

            // A fresh stamp lands with a bounce, drawn over the layer until it settles.
            if let Some((approved, at)) = self.papers[index].stamp {
                let t = ((self.time - at) / 0.25).clamp(0.0, 1.0);
                if t < 1.0 {
                    c.save();
                    c.translate(item.pos.x, item.pos.y);
                    c.rotate(item.rotation);
                    Self::draw_stamp(&font, c, item.w - 92.0, item.h - 78.0, approved, 1.0 + 0.5 * (1.0 - t) * (1.0 - t));
                    c.restore();
                }
            }
        }

        // The marks and what they say.
        let marks: Vec<CPoint> = self.desk.marks().iter().map(|m| CPoint::new(m.at.x, m.at.y)).collect();
        for m in &marks {
            c.stroke_circle(*m, 26.0, &StrokeStyle::new(4.0, GOLD));
        }
        if let [a, b] = marks.as_slice() {
            c.line(*a, *b, &StrokeStyle::new(3.0, GOLD).dashed(&[10.0, 7.0]));
            if let Some((text, same)) = self.comparison() {
                let style = TextStyle::new(&font, 26.0, if same { Color::hex(0x9fe0b4) } else { Color::hex(0xff9aa8) }).outline(4.0, NIGHT).align(Align::Center);
                c.text(&text, (a.x + b.x) / 2.0, (a.y + b.y) / 2.0 - 52.0, &style);
            }
        }

        // A paper to read, big, over a dimmed desk.
        if let Some(id) = self.desk.magnified() {
            c.fill_rect(CRect::new(0.0, 0.0, 1920.0, 1080.0), &Color::BLACK.fade(0.6).into());
            if let Some(layer) = self.paper(id).and_then(|p| p.layer.as_ref()) {
                let scale = (1500.0 / layer.width()).min(860.0 / layer.height());
                let at = CPoint::new(960.0 - layer.width() * scale / 2.0, 540.0 - layer.height() * scale / 2.0);
                c.draw_layer(layer, at, scale, 0.0, 1.0);
            }
        }

        // The top line: the magnifier, the keys, a notice.
        let top = TextStyle::new(&font, 20.0, Color::hex(0xd9c9ee));
        c.text(if self.magnifier { "LUPA: haz clic en dos campos para compararlos" } else { "Arrastra · doble clic: leer · Espacio: lupa · A/R: sellar · N: de nuevo" }, 28.0, 14.0, &top);
        if let Some((text, until)) = &self.notice {
            if self.time < *until {
                let style = TextStyle::new(&font, 24.0, Color::WHITE).outline(4.0, NIGHT).align(Align::Center);
                c.text(text, 960.0, WINDOW.bottom() + 6.0, &style);
            }
        }
        if self.magnifier {
            if let Some(p) = self.pointer {
                c.stroke_circle(CPoint::new(p.x, p.y), 30.0, &StrokeStyle::new(3.0, GOLD.fade(0.8)));
            }
        }
    }
}

/// Times frames of a desk with twelve papers at 2560×1440, one of them dragged (its shadow and place change every
/// frame; the papers' layers are drawn once).
fn bench() {
    let mut run = Headless::new(HostConfig::new("desk-demo"), 2560, 1440);
    let mut demo = Demo::new();
    for id in 1..=3 {
        demo.desk.remove_item(id);
    }
    demo.papers = many_papers(12);
    demo.lay_out();
    run.draw(&mut demo);
    let first = demo.desk.item(1).expect("paper 1").pos;
    run.push(InputEvent::PointerDown { button: Button::Left, at: CPoint::new(first.x + 20.0, first.y + 20.0) });
    run.step(&mut demo);
    let mut times = Vec::new();
    for k in 0..120 {
        run.move_to(first.x + 20.0 + (k as f32 * 0.1).sin() * 300.0, first.y + 20.0 + (k as f32 * 0.07).cos() * 150.0);
        run.step(&mut demo);
        let start = std::time::Instant::now();
        run.draw(&mut demo);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(|a, b| a.partial_cmp(b).expect("times"));
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    println!("12 papeles a 2560×1440: media {:.2} ms, p95 {:.2} ms, peor {:.2} ms", mean, times[times.len() * 95 / 100], times[times.len() - 1]);
}

/// A scripted scene without a window, saved to a PNG: a paper dragged, two fields compared, a stamp.
fn capture(path: &str) {
    let mut run = Headless::new(HostConfig::new("desk-demo"), 2560, 1440);
    let mut demo = Demo::new();
    let first = demo.desk.item(1).expect("paper 1").pos;
    run.push(InputEvent::PointerDown { button: Button::Left, at: CPoint::new(first.x + 40.0, first.y + 30.0) });
    run.step(&mut demo);
    for k in 1..=10 {
        run.move_to(first.x + 40.0 + k as f32 * 6.0, first.y + 30.0 + k as f32 * 20.0);
        run.step(&mut demo);
    }
    run.push(InputEvent::PointerUp { button: Button::Left, at: CPoint::new(first.x + 100.0, first.y + 230.0) });
    run.step(&mut demo);
    run.push(InputEvent::KeyDown(KeyCode::Space));
    run.step(&mut demo);
    for id in [2u64, 3] {
        let item = demo.desk.item(id).expect("paper").clone();
        let at = item.to_desk(Point::new(60.0, 100.0));
        run.click(at.x, at.y);
        run.step(&mut demo);
    }
    let paper2 = demo.desk.item(2).expect("paper 2").clone();
    let over = paper2.to_desk(Point::new(300.0, 200.0));
    run.move_to(over.x, over.y);
    run.push(InputEvent::KeyDown(KeyCode::KeyA));
    run.step(&mut demo);
    run.run_for(&mut demo, 0.5);
    run.draw(&mut demo).save_png(path).expect("png");
    println!("{path}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--bench") {
        bench();
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--capture") {
        capture(args.get(i + 1).map(String::as_str).unwrap_or("desk-demo.png"));
        return;
    }
    let mut config = HostConfig::new("Velvet · escritorio");
    config.captures_dir = Some("captures".into());
    let mut demo = Demo::new();
    demo.player = Some(Player::new());
    if let Err(e) = velvet_host2d::run(config, demo) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
