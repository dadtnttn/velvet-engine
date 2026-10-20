//! Draws a sample document on a desk to a PNG, at a logical 1920×1080 drawn at 2560×1440 pixels: a look at what the
//! canvas draws (shapes, gradients, shadows, text with an outline, an SVG figure).
//!
//!     cargo run -p velvet-canvas --example document -- out.png

use velvet_canvas::{Align, Canvas, Color, Fill, Font, Point, Rect, StrokeStyle, Svg, TextStyle};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "document.png".into());
    let font = Font::from_bytes(std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/Nunito-ExtraBold.ttf")).expect("font")).expect("font");
    let mut frame = Canvas::with_pixels(1920.0, 1080.0, 2560, 1440);

    // The desk: a dark wood with a soft light from above.
    frame.fill_rect(
        Rect::new(0.0, 0.0, 1920.0, 1080.0),
        &Fill::Radial { center: Point::new(960.0, 300.0), radius: 1300.0, stops: vec![(0.0, Color::hex(0x3a2342)), (1.0, Color::hex(0x120c1c))] },
    );

    // A document, drawn once into a layer, then placed on the desk twice (one a little turned).
    let mut doc = frame.layer(520.0, 340.0);
    let paper = Fill::Linear { from: Point::new(0.0, 0.0), to: Point::new(0.0, 340.0), stops: vec![(0.0, Color::hex(0xf6ead0)), (1.0, Color::hex(0xe8d3a8))] };
    doc.fill_round_rect(Rect::new(4.0, 4.0, 512.0, 332.0), 14.0, &paper);
    doc.stroke_round_rect(Rect::new(4.0, 4.0, 512.0, 332.0), 14.0, &StrokeStyle::new(3.0, Color::hex(0x6b4a2b)));
    doc.stroke_round_rect(Rect::new(14.0, 14.0, 492.0, 312.0), 9.0, &StrokeStyle::new(1.5, Color::hex(0xb08a55)).dashed(&[8.0, 5.0]));
    let title = TextStyle::new(&font, 26.0, Color::hex(0x4a2c58)).letter_spacing(1.5);
    doc.text("PERMISO DE ENTRADA", 260.0 - title.measure("PERMISO DE ENTRADA") / 2.0, 26.0, &title);
    let label = TextStyle::new(&font, 15.0, Color::hex(0x8a6a44));
    let value = TextStyle::new(&font, 21.0, Color::hex(0x2a1b10));
    for (i, (l, v)) in [("TITULAR", "Vesper Ashlowe"), ("PUEBLO", "Súcubo de Aszhar"), ("MOTIVO", "Abrir una perfumería"), ("VENCE", "14 · Luna de Brasas · 512")].iter().enumerate() {
        let y = 86.0 + i as f32 * 56.0;
        doc.text(l, 160.0, y, &label);
        doc.text(v, 160.0, y + 18.0, &value);
    }
    let figure = Svg::parse(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 140">
        <rect width="100" height="140" rx="8" fill="#d9c4e8"/>
        <circle cx="50" cy="42" r="18" fill="none" stroke="#2a1b2e" stroke-width="4"/>
        <path d="M36 28 L30 10 L42 24 M64 28 L70 10 L58 24" fill="#7a1f3d" stroke="#2a1b2e" stroke-width="3" stroke-linejoin="round"/>
        <circle cx="44" cy="40" r="2.6" fill="#2a1b2e"/><circle cx="56" cy="40" r="2.6" fill="#2a1b2e"/>
        <path d="M50 60 L50 100 M50 70 L30 88 M50 70 L70 88 M50 100 L36 130 M50 100 L64 130 M50 100 Q80 104 84 86" fill="none" stroke="#2a1b2e" stroke-width="4" stroke-linecap="round"/>
      </svg>"##).expect("svg");
    doc.draw_svg(&figure, Rect::new(30.0, 90.0, 110.0, 154.0), 1.0);
    // The stamp.
    doc.save();
    doc.translate(420.0, 260.0);
    doc.rotate(-12.0);
    doc.stroke_circle(Point::new(0.0, 0.0), 44.0, &StrokeStyle::new(5.0, Color::hex(0x2e7d4f).fade(0.85)));
    let stamp = TextStyle::new(&font, 18.0, Color::hex(0x2e7d4f).fade(0.85)).align(Align::Center);
    doc.text("APROBADO", 0.0, -12.0, &stamp);
    doc.restore();

    frame.shadow(Rect::new(300.0, 240.0, 520.0, 340.0), 14.0, 14.0, Point::new(0.0, 10.0), Color::BLACK.fade(0.45));
    frame.draw_layer(&doc, Point::new(300.0, 240.0), 1.0, 0.0, 1.0);
    frame.save();
    frame.translate(1000.0, 420.0);
    frame.rotate(4.0);
    frame.shadow(Rect::new(0.0, 0.0, 520.0, 340.0), 14.0, 14.0, Point::new(0.0, 10.0), Color::BLACK.fade(0.45));
    frame.draw_layer(&doc, Point::new(0.0, 0.0), 1.0, 0.0, 1.0);
    frame.restore();

    let headline = TextStyle::new(&font, 54.0, Color::hex(0xf2d38a)).outline(4.0, Color::hex(0x1a0f22)).shadow(Point::new(0.0, 5.0), Color::BLACK.fade(0.5)).align(Align::Center);
    frame.text("Garita 7 · Puerta del Crepúsculo", 960.0, 80.0, &headline);
    frame.save_png(&out).expect("png");
    println!("{out}");
}
