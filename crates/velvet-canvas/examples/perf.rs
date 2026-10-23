//! Times the canvas's main calls at 2560×1440 (a 1920×1080 canvas drawn at k = 1.33).
//!
//!     cargo run -p velvet-canvas --release --example perf

use std::time::Instant;

use velvet_canvas::{Canvas, Color, Fill, Point, Rect};

fn time(label: &str, runs: u32, mut f: impl FnMut()) {
    f();
    let start = Instant::now();
    for _ in 0..runs {
        f();
    }
    println!(
        "{label:<44} {:>8.3} ms",
        start.elapsed().as_secs_f64() * 1000.0 / runs as f64
    );
}

fn main() {
    let mut c = Canvas::with_pixels(1920.0, 1080.0, 2560, 1440);
    let gradient = Fill::Linear {
        from: Point::new(0.0, 0.0),
        to: Point::new(0.0, 1080.0),
        stops: vec![(0.0, Color::hex(0x2b1d45)), (1.0, Color::hex(0x1a1326))],
    };
    let radial = Fill::Radial {
        center: Point::new(960.0, 300.0),
        radius: 900.0,
        stops: vec![(0.0, Color::hex(0x5b3f86)), (1.0, Color::hex(0x231737))],
    };
    time("clear", 20, || c.clear(Color::BLACK));
    time("fill_rect solid, full screen", 20, || {
        c.fill_rect(
            Rect::new(0.0, 0.0, 1920.0, 1080.0),
            &Color::hex(0x221a30).into(),
        )
    });
    time("fill_rect linear gradient, full screen", 20, || {
        c.fill_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0), &gradient)
    });
    time("fill_rect radial gradient, full screen", 20, || {
        c.fill_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0), &radial)
    });
    let mut layer = c.layer(460.0, 300.0);
    layer.fill_round_rect(
        Rect::new(0.0, 0.0, 460.0, 300.0),
        12.0,
        &Color::hex(0xf4e7cb).into(),
    );
    time("draw_layer 460×300, only moved", 50, || {
        c.draw_layer(&layer, Point::new(300.5, 400.0), 1.0, 0.0, 1.0)
    });
    time("draw_layer 460×300, turned 3°", 50, || {
        c.draw_layer(&layer, Point::new(300.0, 400.0), 1.0, 3.0, 1.0)
    });
    time("shadow 460×300 blur 10 (cached)", 50, || {
        c.shadow(
            Rect::new(300.0, 400.0, 460.0, 300.0),
            12.0,
            10.0,
            Point::new(0.0, 7.0),
            Color::BLACK.fade(0.45),
        )
    });
    time("fill_round_rect 460×300", 50, || {
        c.fill_round_rect(
            Rect::new(300.0, 400.0, 460.0, 300.0),
            12.0,
            &Color::hex(0xf4e7cb).into(),
        )
    });
}
