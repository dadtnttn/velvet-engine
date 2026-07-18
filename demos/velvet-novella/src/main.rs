//! Velvet Novella — novela visual con ventana (VnSession + softbuffer).
//!
//! Click / Espacio / Enter: avanzar  
//! ↑↓ o W/S: elegir opción · Enter/Click: confirmar  
//! R: reiniciar · Esc: salir · `--headless` prueba automática

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use softbuffer::{Context as SbContext, Surface};
use velvet_story::prelude::*;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const WW: u32 = 960;
const WH: u32 = 540;

struct App {
    session: VnSession,
    story_path: PathBuf,
    window: Option<Arc<Window>>,
    context: Option<SbContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    ended_flash: f32,
}

fn story_path() -> PathBuf {
    let candidates = [
        PathBuf::from("demos/velvet-novella/story/main.vel"),
        PathBuf::from("story/main.vel"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("story/main.vel"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("story/main.vel"))
}

fn open_session(path: &PathBuf) -> Result<VnSession> {
    open_session_from_file(
        path,
        "Luz de Estación",
        Some(PathBuf::from("saves/velvet-novella")),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
    .with_context(|| format!("cargar {}", path.display()))
}

impl App {
    fn new(headless: bool) -> Result<Self> {
        let story_path = story_path();
        let session = open_session(&story_path)?;
        Ok(Self {
            session,
            story_path,
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            ended_flash: 0.0,
        })
    }

    fn restart(&mut self) {
        if let Ok(s) = open_session(&self.story_path) {
            self.session = s;
            self.ended_flash = 0.0;
        }
    }

    fn tick(&mut self, dt: f32) {
        self.session.tick(dt);
        if matches!(self.session.player().wait(), StoryWait::Ended) {
            self.ended_flash = (self.ended_flash + dt).min(3.0);
        }
        if let Some(w) = &self.window {
            let title = if matches!(self.session.player().wait(), StoryWait::Ended) {
                "Luz de Estación — FIN (R reinicia · Esc sale)".into()
            } else if self.session.choice.open {
                format!(
                    "Luz de Estación — elige ({}/{})",
                    self.session.choice.selected + 1,
                    self.session.choice.options.len().max(1)
                )
            } else {
                let name = if self.session.say.namebox.trim().is_empty() {
                    "…"
                } else {
                    self.session.say.namebox.as_str()
                };
                format!("Luz de Estación — {name}")
            };
            w.set_title(&title);
            w.request_redraw();
        }
    }

    fn advance_or_choose(&mut self) {
        if self.session.choice.open {
            let _ = self.session.choose_selected();
        } else if matches!(self.session.player().wait(), StoryWait::Ended) {
            // stay on ending
        } else {
            self.session.advance();
        }
    }

    fn paint(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let ww = size.width.max(1);
        let wh = size.height.max(1);
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(
            NonZeroU32::new(ww).unwrap(),
            NonZeroU32::new(wh).unwrap(),
        );
        let mut buf = surface.buffer_mut().unwrap();

        // Background from presentation path (color mood)
        let (br, bg, bb) = bg_color(self.session.presentation.background.as_deref());
        fill(&mut buf, ww, wh, 0, 0, ww as i32, wh as i32, br, bg, bb);

        // Soft gradient top (sky/light)
        for y in 0..(wh as i32 / 3) {
            let t = 1.0 - y as f32 / (wh as f32 / 3.0);
            let r = (br as f32 + 30.0 * t).min(255.0) as u8;
            let g = (bg as f32 + 20.0 * t).min(255.0) as u8;
            let b = (bb as f32 + 40.0 * t).min(255.0) as u8;
            fill(&mut buf, ww, wh, 0, y, ww as i32, y + 1, r, g, b);
        }

        // Character stands (simple silhouettes from presentation sprites)
        let sprites = self.session.presentation.sprites_by_z();
        let n = sprites.len().max(1) as f32;
        for (i, sp) in sprites.iter().enumerate() {
            let at = sp.at.as_deref().unwrap_or("center");
            let cx = match at {
                "left" => ww as f32 * 0.28,
                "right" => ww as f32 * 0.72,
                _ => ww as f32 * (0.35 + 0.15 * (i as f32 / n)),
            };
            let (cr, cg, cb) = char_color(&sp.id);
            let body_h = (wh as f32 * 0.42) as i32;
            let body_w = (ww as f32 * 0.12) as i32;
            let x0 = (cx as i32) - body_w / 2;
            let y0 = (wh as i32) - body_h - (wh as i32 / 5);
            // body
            fill(
                &mut buf, ww, wh, x0, y0, x0 + body_w, y0 + body_h, cr, cg, cb,
            );
            // head
            let hr = body_w / 3;
            fill(
                &mut buf,
                ww,
                wh,
                cx as i32 - hr,
                y0 - hr * 2,
                cx as i32 + hr,
                y0,
                (cr as u16 + 40).min(255) as u8,
                (cg as u16 + 30).min(255) as u8,
                (cb as u16 + 20).min(255) as u8,
            );
            // label under
            draw_text(
                &mut buf,
                ww,
                wh,
                x0,
                y0 + body_h + 6,
                &sp.id,
                220,
                220,
                230,
                2,
            );
        }

        // Dialogue box
        let box_h = (wh as f32 * 0.28) as i32;
        let box_y = wh as i32 - box_h - 16;
        let box_x = 24;
        let box_w = ww as i32 - 48;
        // panel
        fill(
            &mut buf, ww, wh, box_x, box_y, box_x + box_w, box_y + box_h, 18, 16, 28,
        );
        // border
        rect_border(
            &mut buf, ww, wh, box_x, box_y, box_x + box_w, box_y + box_h, 90, 80, 120, 2,
        );

        if self.session.say.visible {
            let name = self.session.say.namebox.trim();
            if !name.is_empty() {
                // nameplate
                let nw = (name.chars().count() as i32 * 10 + 24).max(80);
                fill(
                    &mut buf,
                    ww,
                    wh,
                    box_x + 16,
                    box_y - 22,
                    box_x + 16 + nw,
                    box_y + 2,
                    40,
                    30,
                    55,
                );
                draw_text(
                    &mut buf,
                    ww,
                    wh,
                    box_x + 24,
                    box_y - 16,
                    name,
                    255,
                    180,
                    210,
                    2,
                );
            }
            // body text (typewriter-aware)
            let text = &self.session.say.visible_text;
            draw_text_wrapped(
                &mut buf,
                ww,
                wh,
                box_x + 20,
                box_y + 20,
                box_w - 40,
                text,
                235,
                232,
                245,
                2,
            );
            if self.session.say.text_complete
                && !matches!(self.session.player().wait(), StoryWait::Choice)
            {
                draw_text(
                    &mut buf,
                    ww,
                    wh,
                    box_x + box_w - 100,
                    box_y + box_h - 22,
                    "[click]",
                    160,
                    150,
                    180,
                    1,
                );
            }
        }

        // Choices
        if self.session.choice.open {
            let opts = &self.session.choice.options;
            let start_y = box_y - 20 - (opts.len() as i32) * 36;
            for (i, o) in opts.iter().enumerate() {
                let y = start_y + i as i32 * 36;
                let sel = i == self.session.choice.selected;
                let (r, g, b) = if sel {
                    (70, 55, 100)
                } else {
                    (30, 28, 42)
                };
                fill(
                    &mut buf,
                    ww,
                    wh,
                    box_x + 40,
                    y,
                    box_x + box_w - 40,
                    y + 30,
                    r,
                    g,
                    b,
                );
                if sel {
                    rect_border(
                        &mut buf,
                        ww,
                        wh,
                        box_x + 40,
                        y,
                        box_x + box_w - 40,
                        y + 30,
                        255,
                        210,
                        100,
                        2,
                    );
                }
                let prefix = if sel { "> " } else { "  " };
                let line = format!("{}{}", prefix, o.text);
                draw_text(
                    &mut buf,
                    ww,
                    wh,
                    box_x + 52,
                    y + 8,
                    &line,
                    if sel { 255 } else { 200 },
                    if sel { 240 } else { 200 },
                    if sel { 200 } else { 220 },
                    2,
                );
            }
        }

        // Ending overlay
        if matches!(self.session.player().wait(), StoryWait::Ended) {
            let end = self
                .session
                .player()
                .variables()
                .get("ending")
                .display_str();
            fill(
                &mut buf,
                ww,
                wh,
                ww as i32 / 4,
                wh as i32 / 3,
                ww as i32 * 3 / 4,
                wh as i32 / 3 + 80,
                12,
                10,
                20,
            );
            draw_text(
                &mut buf,
                ww,
                wh,
                ww as i32 / 4 + 24,
                wh as i32 / 3 + 20,
                "=== FIN ===",
                255,
                220,
                120,
                3,
            );
            let msg = format!("ending: {end}  (R reinicia)");
            draw_text(
                &mut buf,
                ww,
                wh,
                ww as i32 / 4 + 24,
                wh as i32 / 3 + 50,
                &msg,
                200,
                200,
                220,
                2,
            );
        }

        // Hint bar
        draw_text(
            &mut buf,
            ww,
            wh,
            16,
            12,
            "Espacio/Click: avanzar  |  Arriba/Abajo: opciones  |  R: reiniciar  |  Esc: salir",
            140,
            135,
            160,
            1,
        );

        let _ = buf.present();
    }
}

fn bg_color(path: Option<&str>) -> (u8, u8, u8) {
    match path.unwrap_or("") {
        p if p.contains("rain") || p.contains("station") => (28, 32, 55),
        p if p.contains("platform") => (35, 38, 58),
        p if p.contains("tunnel") => (22, 20, 28),
        p if p.contains("train") => (40, 30, 45),
        p if p.contains("city") => (20, 25, 50),
        p if p.contains("street") => (25, 28, 40),
        _ => (30, 28, 48),
    }
}

fn char_color(id: &str) -> (u8, u8, u8) {
    match id {
        "nora" => (180, 70, 110),
        "june" => (50, 120, 180),
        "guard" => (160, 130, 70),
        "radio" => (100, 100, 110),
        _ => (90, 85, 110),
    }
}

fn pack(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn fill(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    r: u8,
    g: u8,
    b: u8,
) {
    let c = pack(r, g, b);
    let x0 = x0.clamp(0, ww as i32);
    let y0 = y0.clamp(0, wh as i32);
    let x1 = x1.clamp(0, ww as i32);
    let y1 = y1.clamp(0, wh as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            buf[(y as u32 * ww + x as u32) as usize] = c;
        }
    }
}

fn rect_border(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    r: u8,
    g: u8,
    b: u8,
    t: i32,
) {
    fill(buf, ww, wh, x0, y0, x1, y0 + t, r, g, b);
    fill(buf, ww, wh, x0, y1 - t, x1, y1, r, g, b);
    fill(buf, ww, wh, x0, y0, x0 + t, y1, r, g, b);
    fill(buf, ww, wh, x1 - t, y0, x1, y1, r, g, b);
}

/// Tiny 5×7 font (subset); accents fold to base letters.
fn glyph(c: char) -> [u8; 7] {
    // each row: 5 bits in low bits
    let c = match c {
        'á' | 'à' | 'ä' | 'â' | 'Á' => 'a',
        'é' | 'è' | 'ë' | 'ê' | 'É' => 'e',
        'í' | 'ì' | 'ï' | 'î' | 'Í' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'Ó' => 'o',
        'ú' | 'ù' | 'ü' | 'û' | 'Ú' => 'u',
        'ñ' | 'Ñ' => 'n',
        '¿' => '?',
        '¡' => '!',
        '…' => '.',
        '—' | '–' => '-',
        '“' | '”' | '„' => '"',
        '‘' | '’' => '\'',
        _ => c,
    };
    let cu = c.to_ascii_uppercase();
    match cu {
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        '3' => [0x1F, 0x01, 0x02, 0x06, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0C, 0x04, 0x08],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        ':' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00],
        ';' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x04, 0x08],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        '\'' => [0x0C, 0x0C, 0x08, 0x00, 0x00, 0x00, 0x00],
        '"' => [0x1B, 0x1B, 0x12, 0x00, 0x00, 0x00, 0x00],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        ']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        '\\' => [0x10, 0x10, 0x08, 0x04, 0x02, 0x01, 0x01],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '=' => [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00],
        '>' => [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08],
        '|' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        '{' => [0x06, 0x08, 0x08, 0x10, 0x08, 0x08, 0x06],
        '}' => [0x0C, 0x02, 0x02, 0x01, 0x02, 0x02, 0x0C],
        '&' => [0x0C, 0x12, 0x14, 0x08, 0x15, 0x12, 0x0D],
        '%' => [0x19, 0x1A, 0x02, 0x04, 0x08, 0x0B, 0x13],
        '*' => [0x00, 0x15, 0x0E, 0x1F, 0x0E, 0x15, 0x00],
        '#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        '@' => [0x0E, 0x11, 0x17, 0x15, 0x17, 0x10, 0x0E],
        _ => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F],
    }
}

fn draw_char(buf: &mut [u32], ww: u32, wh: u32, x: i32, y: i32, c: char, r: u8, g: u8, b: u8, s: i32) {
    let rows = glyph(c);
    let col = pack(r, g, b);
    for (row, bits) in rows.iter().enumerate() {
        for col_i in 0..5 {
            if bits & (1 << (4 - col_i)) != 0 {
                for dy in 0..s {
                    for dx in 0..s {
                        let px = x + col_i * s + dx;
                        let py = y + row as i32 * s + dy;
                        if px >= 0 && py >= 0 && px < ww as i32 && py < wh as i32 {
                            buf[(py as u32 * ww + px as u32) as usize] = col;
                        }
                    }
                }
            }
        }
    }
}

fn draw_text(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    mut x: i32,
    y: i32,
    text: &str,
    r: u8,
    g: u8,
    b: u8,
    s: i32,
) {
    let advance = 6 * s;
    for c in text.chars() {
        if c == '\n' {
            continue;
        }
        draw_char(buf, ww, wh, x, y, c, r, g, b, s);
        x += advance;
        if x > ww as i32 - 20 {
            break;
        }
    }
}

fn draw_text_wrapped(
    buf: &mut [u32],
    ww: u32,
    wh: u32,
    x0: i32,
    y0: i32,
    max_w: i32,
    text: &str,
    r: u8,
    g: u8,
    b: u8,
    s: i32,
) {
    let advance = 6 * s;
    let line_h = 9 * s;
    let max_chars = (max_w / advance).max(8) as usize;
    let mut y = y0;
    let mut line = String::new();
    for word in text.split_whitespace() {
        let trial = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if trial.chars().count() > max_chars {
            if !line.is_empty() {
                draw_text(buf, ww, wh, x0, y, &line, r, g, b, s);
                y += line_h;
                line.clear();
            }
            // hard split long word
            let mut rest = word;
            while rest.chars().count() > max_chars {
                let take: String = rest.chars().take(max_chars).collect();
                draw_text(buf, ww, wh, x0, y, &take, r, g, b, s);
                y += line_h;
                rest = &rest[take.len()..];
            }
            line = rest.to_string();
        } else {
            line = trial;
        }
        if y > wh as i32 - 40 {
            break;
        }
    }
    if !line.is_empty() {
        draw_text(buf, ww, wh, x0, y, &line, r, g, b, s);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Luz de Estación — Velvet Novella")
            .with_inner_size(LogicalSize::new(WW, WH));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        let context = SbContext::new(window.clone()).expect("ctx");
        let surface = Surface::new(&context, window.clone()).expect("surface");
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let PhysicalKey::Code(c) = event.physical_key else {
                    return;
                };
                match c {
                    KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter => {
                        self.advance_or_choose();
                    }
                    KeyCode::ArrowUp | KeyCode::KeyW => {
                        if self.session.choice.open {
                            self.session.choice.move_sel(-1);
                        }
                    }
                    KeyCode::ArrowDown | KeyCode::KeyS => {
                        if self.session.choice.open {
                            self.session.choice.move_sel(1);
                        }
                    }
                    KeyCode::Digit1 | KeyCode::Numpad1 => {
                        if self.session.choice.open {
                            let _ = self.session.choose_arm(0);
                        }
                    }
                    KeyCode::Digit2 | KeyCode::Numpad2 => {
                        if self.session.choice.open {
                            let _ = self.session.choose_arm(1);
                        }
                    }
                    KeyCode::Digit3 | KeyCode::Numpad3 => {
                        if self.session.choice.open {
                            let _ = self.session.choose_arm(2);
                        }
                    }
                    KeyCode::Digit4 | KeyCode::Numpad4 => {
                        if self.session.choice.open {
                            let _ = self.session.choose_arm(3);
                        }
                    }
                    KeyCode::KeyR => self.restart(),
                    KeyCode::Escape => el.exit(),
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.advance_or_choose();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.05);
                self.last = now;
                self.tick(dt);
                self.paint();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.headless {
            self.hframes += 1;
            self.session.tick(1.0 / 30.0);
            // auto-play: advance lines, pick first choice
            match self.session.player().wait().clone() {
                StoryWait::Line | StoryWait::Ready => self.session.advance(),
                StoryWait::Choice => {
                    let _ = self.session.choose_arm(0);
                }
                StoryWait::Ended => {
                    let end = self
                        .session
                        .player()
                        .variables()
                        .get("ending")
                        .display_str();
                    println!("headless ending={end} steps={}", self.hframes);
                    println!("ASSERT_OK velvet_novella");
                    el.exit();
                    return;
                }
            }
            if self.hframes > 5000 {
                println!("ASSERT_FAIL step_limit");
                el.exit();
            }
            return;
        }
        el.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("velvet_novella=info,info");
    let headless = std::env::args().any(|a| a == "--headless");
    println!("=== Luz de Estación — Velvet Novella ===");
    println!("Historia: demos/velvet-novella/story/main.vel");
    println!("Click/Espacio avanzar · ↑↓ opciones · R reiniciar · Esc salir");

    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(headless)?;
    el.run_app(&mut app)?;
    Ok(())
}
