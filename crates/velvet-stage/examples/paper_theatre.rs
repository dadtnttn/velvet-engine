//! A small paper theatre drawn without a window: a wall with a window, layers of hills behind it in the haze, a table
//! with two sheets of paper lit by a candle, and a label on the screen.
//!
//! ```text
//! cargo run -p velvet-stage --example paper_theatre --release -- out.png
//! ```

use velvet_canvas::{Align, Canvas, Color, Fill, Font, Point, Rect, StrokeStyle, TextStyle};
use velvet_stage::{Camera, Card, Fog, Light, Offscreen, Stage, Vec3};

const FONT: &[u8] = include_bytes!("../../velvet-canvas/tests/fixtures/Nunito-ExtraBold.ttf");

fn picture(w: f32, h: f32, density: f32, paint: impl FnOnce(&mut Canvas)) -> Canvas {
    let mut c = Canvas::with_pixels(w, h, (w * density) as u32, (h * density) as u32);
    paint(&mut c);
    c
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "paper_theatre.png".into());
    let font = Font::from_bytes(FONT.to_vec()).expect("font");
    let mut stage = Stage::new(1920.0, 1080.0);
    stage.clear = [0.07, 0.05, 0.12];
    stage.ambient = [0.35, 0.3, 0.45];
    stage.fog = Some(Fog { color: [0.16, 0.1, 0.3], start: 150.0, end: 900.0 });
    stage.camera = Camera::new(Vec3::new(0.0, 55.0, 70.0), Vec3::new(0.0, 12.0, -20.0));
    stage.camera.fov = 50.0;

    // Hills far away, in layers.
    for (i, (z, color)) in [(-700.0, 0x5a3f86), (-450.0, 0x46306c), (-250.0, 0x332252)].iter().enumerate() {
        let hills = picture(800.0, 200.0, 1.0, |c| {
            let mut points = vec![Point::new(0.0, 200.0)];
            for k in 0..=16 {
                let x = k as f32 * 50.0;
                points.push(Point::new(x, 80.0 + 50.0 * ((x * 0.013 + i as f32).sin() * 0.5 + 0.5)));
            }
            points.push(Point::new(800.0, 200.0));
            if let Some(path) = velvet_canvas::Path::polygon(&points) {
                c.fill_path(&path, &Color::hex(*color).into());
            }
        });
        let id = stage.add_picture(hills);
        stage.add(Card::world(id, 800.0 * (1.0 + i as f32 * 0.2), 200.0).at(Vec3::new(0.0, 40.0, *z)).anchored(0.5, 1.0).ordered(-10));
    }
    // The wall with its window.
    let wall = picture(400.0, 200.0, 2.0, |c| {
        c.fill_rect(Rect::new(0.0, 0.0, 400.0, 200.0), &Color::hex(0x2a1d3c).into());
        let window = velvet_canvas::Path::round_rect(Rect::new(110.0, 20.0, 180.0, 130.0), 60.0).expect("window");
        c.save();
        c.clip_path(&window);
        c.clear(Color::TRANSPARENT);
        c.restore();
        c.stroke_path(&window, &StrokeStyle::new(8.0, Color::hex(0xe08a3c)));
    });
    let wall = stage.add_picture(wall);
    stage.add(Card::world(wall, 200.0, 100.0).at(Vec3::new(0.0, 0.0, -60.0)).anchored(0.5, 1.0));
    // The table and two sheets.
    let table = picture(200.0, 100.0, 4.0, |c| {
        c.fill_rect(Rect::new(0.0, 0.0, 200.0, 100.0), &Fill::Linear { from: Point::new(0.0, 0.0), to: Point::new(0.0, 100.0), stops: vec![(0.0, Color::hex(0x5a3424)), (1.0, Color::hex(0x3a2016))] });
    });
    let table = stage.add_picture(table);
    let table = stage.add(Card::world(table, 200.0, 100.0).at(Vec3::new(0.0, 0.0, -10.0)).turned(-90.0, 0.0, 0.0));
    for (i, (title, x, roll)) in [("PASAPORTE", -25.0, 6.0), ("PERMISO", 25.0, -4.0)].iter().enumerate() {
        let sheet = picture(300.0, 200.0, 4.0, |c| {
            c.fill_round_rect(Rect::new(0.0, 0.0, 300.0, 200.0), 10.0, &Color::hex([0xe9dcf2, 0xf7d6dc][i]).into());
            c.stroke_round_rect(Rect::new(6.0, 6.0, 288.0, 188.0), 8.0, &StrokeStyle::new(3.0, Color::hex(0x6a3fa0)));
            c.text(title, 150.0, 20.0, &TextStyle::new(&font, 34.0, Color::hex(0x3a2550)).align(Align::Center));
            c.text("Vesper Ashlowe", 30.0, 90.0, &TextStyle::new(&font, 24.0, Color::hex(0x2a1b10)));
        });
        let sheet = stage.add_picture(sheet);
        let at = stage.on_card(table, velvet_stage::Vec2::new(100.0 + x, 60.0), 0.05 + i as f32 * 0.05).expect("table");
        stage.add(Card::world(sheet, 30.0, 20.0).at(at).turned(0.0, 0.0, *roll).child_of(table));
    }
    // A candle's light, and a label on the screen.
    stage.lights.push(Light { position: Vec3::new(-45.0, 15.0, 5.0), color: [1.0, 0.62, 0.3], intensity: 2.2, range: 90.0 });
    let label = picture(420.0, 70.0, 1.0, |c| {
        c.fill_round_rect(Rect::new(0.0, 0.0, 420.0, 70.0), 14.0, &Color::hex(0x1a1326).fade(0.85).into());
        c.text("Teatro de papel", 210.0, 14.0, &TextStyle::new(&font, 34.0, Color::hex(0xe8c27a)).align(Align::Center));
    });
    let label = stage.add_picture(label);
    stage.add(Card::screen(label, 40.0, 40.0, 420.0, 70.0));

    let mut offscreen = Offscreen::new().expect("graphics device");
    let started = std::time::Instant::now();
    offscreen.save_png(&stage, 1920, 1080, &path).expect("png");
    println!("{path} ({:.0} ms)", started.elapsed().as_secs_f64() * 1000.0);
}
