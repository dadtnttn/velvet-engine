//! Velvet FX Forge — BitFX Forge-like pixel VFX editor, 100% Velvet Engine.
//! Keyboard-driven: no external UI crate, softbuffer + pixel rendering.
//! R = Randomize (headline BitFX feature), S = export spritesheet, J = save JSON.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use std::sync::OnceLock;

use ab_glyph::{Font, FontArc, Point, PxScale, ScaleFont};
use anyhow::Result;
use softbuffer::{Context, Surface};
use velvet_fx::{FxPreset, FxSim, PaletteId, PathKind};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

// Preview window — muy grande, todo escala x1.5
const WIN_W: u32 = 1920;
const WIN_H: u32 = 1080;

fn pack(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn export_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output")
}

// ── TTF font for legible UI (normal sans, not 5×7 pixel) ──
static FONT: OnceLock<FontArc> = OnceLock::new();
fn font() -> &'static FontArc {
    FONT.get_or_init(|| {
        let windows = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:/Windows"));
        std::env::var_os("VELVET_FONT")
            .map(PathBuf::from)
            .into_iter()
            .chain([
                windows.join("Fonts/tahoma.ttf"),
                windows.join("Fonts/segoeui.ttf"),
                PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
                PathBuf::from("/System/Library/Fonts/Supplemental/Arial.ttf"),
            ])
            .find_map(|path| FontArc::try_from_vec(std::fs::read(path).ok()?).ok())
            .expect("No UI font found; set VELVET_FONT to a readable TTF font")
    })
}
fn draw_text_ttf(
    fb: &mut [u32],
    fb_w: u32,
    fb_h: u32,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    size: f32,
) {
    let scale = PxScale::from(size);
    let scaled = font().as_scaled(scale);
    let cr = ((color >> 16) & 0xFF) as u8;
    let cg = ((color >> 8) & 0xFF) as u8;
    let cb = (color & 0xFF) as u8;
    let mut caret = Point {
        x: x as f32,
        y: y as f32 + size * 0.85,
    };
    for ch in text.chars() {
        if ch == '\n' {
            caret.x = x as f32;
            caret.y += size * 1.22;
            continue;
        }
        let glyph_id = scaled.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(scale, caret);
        if let Some(outlined) = font().outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, v| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px >= 0 && py >= 0 && px < fb_w as i32 && py < fb_h as i32 {
                    let idx = (py as u32 * fb_w + px as u32) as usize;
                    if idx < fb.len() {
                        let a = (v * 255.0) as u32;
                        if a > 8 {
                            let bg = fb[idx];
                            let br = (bg >> 16) & 0xFF;
                            let bg2 = (bg >> 8) & 0xFF;
                            let bb = bg & 0xFF;
                            let rr = (br * (255 - a) + cr as u32 * a) / 255;
                            let gg = (bg2 * (255 - a) + cg as u32 * a) / 255;
                            let bb2 = (bb * (255 - a) + cb as u32 * a) / 255;
                            fb[idx] = (rr << 16) | (gg << 8) | bb2;
                        }
                    }
                }
            });
        }
        caret.x += scaled.h_advance(glyph_id);
        if caret.x > fb_w as f32 - 8.0 {
            break;
        }
    }
}
struct App {
    sim: FxSim,
    preset_idx: usize,
    examples: Vec<FxPreset>,
    /// Diseños populares (Fuego, Agua, etc.) para búsqueda guiada
    designs: Vec<(String, FxPreset)>,
    target_idx: Option<usize>,
    creativity: f32,
    time: f32,
    playing: bool,
    frame_idx: u32,
    preview_time: f32,
    show_help: bool,
    status: String,
    status_timer: f32,
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    fb: Vec<u32>,
    fb_w: u32,
    fb_h: u32,
    last: Instant,
    headless: bool,
    hframes: u32,
}

fn build_designs() -> Vec<(String, FxPreset)> {
    // Popular searches: Fuego, Agua, Humo, Magia, Rayo, Tóxico, Hielo, Explosión, Portal, Naturaleza
    let mut designs = Vec::new();
    designs.push(("Fuego".into(), FxPreset::fire()));
    // Agua — custom aqua preset
    let mut water = FxPreset::smoke();
    water.meta.name = "Agua".into();
    water.palette = PaletteId::Aqua;
    water.path = velvet_fx::PathConfig {
        kind: PathKind::Circle,
        scale_x: 0.55,
        scale_y: 0.35,
        ..Default::default()
    };
    water.particles.gravity = 8.0;
    water.particles.speed_min = 6.0;
    water.particles.speed_max = 18.0;
    water = water.validated();
    designs.push(("Agua".into(), water));
    designs.push(("Humo".into(), FxPreset::smoke()));
    designs.push(("Magia".into(), FxPreset::magic()));
    designs.push(("Rayo".into(), FxPreset::lightning()));
    let mut toxic = FxPreset::dna();
    toxic.meta.name = "Toxico".into();
    toxic.palette = PaletteId::Toxic;
    designs.push(("Toxico".into(), toxic));
    let mut frost = FxPreset::smoke();
    frost.meta.name = "Hielo".into();
    frost.palette = PaletteId::Frost;
    frost.particles.gravity = -4.0;
    frost = frost.validated();
    designs.push(("Hielo".into(), frost));
    designs.push(("Explosion".into(), FxPreset::embers()));
    designs.push(("Portal".into(), FxPreset::portal()));
    designs.push(("Naturaleza".into(), {
        let mut p = FxPreset::heal();
        p.meta.name = "Naturaleza".into();
        p.palette = PaletteId::Nature;
        p
    }));
    designs
}

impl App {
    fn new(headless: bool) -> Self {
        let examples = FxPreset::examples();
        let designs = build_designs();
        let preset = examples[0].clone();
        let sim = FxSim::new(preset);
        Self {
            sim,
            preset_idx: 0,
            examples,
            designs,
            target_idx: None,
            creativity: 0.6,
            time: 0.0,
            playing: true,
            frame_idx: 0,
            preview_time: 0.0,
            show_help: true,
            status: "R=Random  D=Diseno  C/V Creatividad  S=Export  1..8 presets  H=Ayuda".into(),
            status_timer: 5.0,
            window: None,
            context: None,
            surface: None,
            fb: vec![0; (WIN_W * WIN_H) as usize],
            fb_w: WIN_W,
            fb_h: WIN_H,
            last: Instant::now(),
            headless,
            hframes: 0,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_timer = 3.5;
        println!("[fx-forge] {}", self.status);
    }

    fn load_preset(&mut self, preset: FxPreset) {
        let name = preset.meta.name.clone();
        self.sim = FxSim::new(preset);
        self.preview_time = 0.0;
        self.frame_idx = 0;
        self.set_status(format!(
            "Preset: {name}  {} {} {}fps {} frames",
            self.sim.preset.palette.label(),
            self.sim.preset.path.kind.label(),
            self.sim.preset.fps,
            self.sim.preset.frames
        ));
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn randomize(&mut self) {
        let seed = (self.time * 1000.0) as u64 ^ 0xCAFE_1234 ^ (self.hframes as u64 * 0x9E37);
        let p = if let Some(idx) = self.target_idx {
            let base = self.designs[idx].1.clone();
            let label = self.designs[idx].0.clone();
            if self.creativity < 0.08 {
                base
            } else {
                let steps = (self.creativity * 4.0).round() as usize + 1;
                let mut cur = base;
                for i in 0..steps {
                    cur = velvet_fx::mutate_preset(&cur, seed.wrapping_add(i as u64 * 0x9E37));
                }
                // high creativity also randomizes palette 50% chance
                if self.creativity > 0.75 && (seed & 1) == 1 {
                    // blend random preset palette
                    let rnd = velvet_fx::random_preset(seed ^ 0x5A5A);
                    if (seed >> 2) & 1 == 1 {
                        cur.palette = rnd.palette;
                    }
                }
                let mut meta = cur.meta.clone();
                meta.name = format!("{} {:.0}%", label, self.creativity * 100.0);
                cur.meta = meta;
                cur
            }
        } else {
            velvet_fx::random_preset(seed)
        };
        self.load_preset(p);
        if self.target_idx.is_some() {
            self.set_status(format!(
                "Generado desde diseno '{}' creatividad {:.0}% — R / D / C/V",
                self.designs[self.target_idx.unwrap()].0,
                self.creativity * 100.0
            ));
        } else {
            self.set_status(format!(
                "Random total seed {seed:#x} — elige D para diseno guiado"
            ));
        }
    }

    fn mutate(&mut self) {
        let seed = (self.time * 1000.0) as u64 ^ 0xBEEF ^ self.sim.preset.seed;
        let p = velvet_fx::mutate_preset(&self.sim.preset, seed);
        self.load_preset(p);
    }

    fn cycle_target(&mut self) {
        if self.designs.is_empty() {
            return;
        }
        let next = match self.target_idx {
            None => 0,
            Some(i) => (i + 1) % self.designs.len(),
        };
        self.target_idx = Some(next);
        let (label, preset) = self.designs[next].clone();
        self.load_preset(preset);
        self.set_status(format!(
            "Diseno objetivo: {} — creatividad {:.0}% (C/V ajusta, R genera cercano, G quita)",
            label,
            self.creativity * 100.0
        ));
    }

    fn clear_target(&mut self) {
        self.target_idx = None;
        self.set_status("Diseno libre — Random total (D para elegir Fuego/Agua/...)");
    }

    fn adjust_creativity(&mut self, delta: f32) {
        self.creativity = (self.creativity + delta).clamp(0.0, 1.0);
        let label = if let Some(idx) = self.target_idx {
            self.designs[idx].0.clone()
        } else {
            "libre".into()
        };
        self.set_status(format!(
            "Creatividad {:.0}% — objetivo '{}' (bajo=fiel, alto=loco) R para generar",
            self.creativity * 100.0,
            label
        ));
    }

    fn next_palette(&mut self) {
        let all = PaletteId::all();
        let cur = self.sim.preset.palette;
        let idx = all.iter().position(|&p| p == cur).unwrap_or(0);
        let next = all[(idx + 1) % all.len()];
        let mut p = self.sim.preset.clone();
        p.palette = next;
        p.palette_override.clear();
        p.seed ^= 0x1111;
        self.load_preset(p.validated());
    }

    fn next_path(&mut self) {
        let all = PathKind::all();
        let cur = self.sim.preset.path.kind;
        let idx = all.iter().position(|&k| k == cur).unwrap_or(0);
        let next = all[(idx + 1) % all.len()];
        let mut p = self.sim.preset.clone();
        p.path.kind = next;
        // give dna a taller canvas
        if next == PathKind::Dna && p.dimensions.height < 80 {
            p.dimensions.height = 96;
        }
        p.seed ^= 0x2222;
        self.load_preset(p.validated());
    }

    fn adjust_scale(&mut self, dx: f32, dy: f32) {
        let mut p = self.sim.preset.clone();
        p.path.scale_x = (p.path.scale_x + dx).clamp(0.1, 3.0);
        p.path.scale_y = (p.path.scale_y + dy).clamp(0.1, 3.0);
        self.sim = FxSim::new(p);
        self.set_status(format!(
            "Scale x{:.2} y{:.2}  (+/- x  Shift+/- y)",
            self.sim.preset.path.scale_x, self.sim.preset.path.scale_y
        ));
    }

    fn cycle_dimensions(&mut self) {
        let mut p = self.sim.preset.clone();
        p.dimensions = match (p.dimensions.width, p.dimensions.height) {
            (32, 32) => velvet_fx::Dimensions {
                width: 48,
                height: 48,
            },
            (48, 48) => velvet_fx::Dimensions {
                width: 64,
                height: 64,
            },
            (64, 64) => velvet_fx::Dimensions {
                width: 96,
                height: 96,
            },
            (96, 96) => velvet_fx::Dimensions {
                width: 64,
                height: 96,
            },
            (64, 96) => velvet_fx::Dimensions {
                width: 48,
                height: 96,
            },
            (48, 96) => velvet_fx::Dimensions {
                width: 32,
                height: 32,
            },
            _ => velvet_fx::Dimensions {
                width: 64,
                height: 64,
            },
        };
        self.sim = FxSim::new(p);
        self.set_status(format!(
            "Canvas {}×{}",
            self.sim.preset.dimensions.width, self.sim.preset.dimensions.height
        ));
    }

    fn cycle_frames(&mut self) {
        let mut p = self.sim.preset.clone();
        p.frames = match p.frames {
            8 => 12,
            12 => 16,
            16 => 24,
            24 => 32,
            _ => 16,
        };
        self.sim = FxSim::new(p);
        self.set_status(format!(
            "Frames {} @ {}fps",
            self.sim.preset.frames, self.sim.preset.fps
        ));
    }

    fn toggle_additive(&mut self) {
        let mut p = self.sim.preset.clone();
        p.additive = !p.additive;
        let v = p.additive;
        self.sim = FxSim::new(p);
        self.set_status(if v {
            "Additive ON (glow)"
        } else {
            "Additive OFF"
        });
    }

    fn export_sheet(&mut self) {
        let out_dir = export_dir();
        let _ = std::fs::create_dir_all(&out_dir);
        let stem = self.sim.preset.file_stem();
        let out_png = out_dir.join(format!("{stem}.png"));
        match velvet_fx::export::export_spritesheet_to_files(&self.sim, &out_png, None) {
            Ok(meta) => self.set_status(format!(
                "Exported {} ({}×{}, {} cols) + json/preset",
                out_png.display(),
                meta.sheet_width,
                meta.sheet_height,
                meta.columns
            )),
            Err(e) => self.set_status(format!("Export failed: {e}")),
        }
    }

    fn save_json(&mut self) {
        let out_dir = export_dir();
        let _ = std::fs::create_dir_all(&out_dir);
        let stem = self.sim.preset.file_stem();
        let out = out_dir.join(format!("{stem}_preset.json"));
        match velvet_fx::export::export_preset_json(&self.sim.preset, &out) {
            Ok(()) => self.set_status(format!("Saved preset JSON {}", out.display())),
            Err(e) => self.set_status(format!("Save failed: {e}")),
        }
    }

    fn tick(&mut self, dt: f32) {
        self.time += dt;
        if self.playing {
            self.preview_time += dt;
        }
        // advance spritesheet frame index for strip preview
        // compute index for display
        let total = self.sim.preset.frames.max(1) as f32;
        let loop_dur = self.sim.preset.duration().max(0.2);
        let phase = (self.preview_time % loop_dur) / loop_dur;
        self.frame_idx = (phase * total).floor() as u32 % self.sim.preset.frames.max(1);
        if self.status_timer > 0.0 {
            self.status_timer -= dt;
        }
        if let Some(w) = &self.window {
            w.set_title(&format!(
                "Velvet FX Forge — {} | {} | {} | {}×{} {}f @ {}fps | {}",
                self.sim.preset.meta.name,
                self.sim.preset.palette.label(),
                self.sim.preset.path.kind.label(),
                self.sim.preset.dimensions.width,
                self.sim.preset.dimensions.height,
                self.sim.preset.frames,
                self.sim.preset.fps,
                if self.playing { "▶" } else { "⏸" }
            ));
            w.request_redraw();
        }
    }

    fn compose(&mut self) {
        let w = self.fb_w;
        let h = self.fb_h;
        // background: dark desaturated violet
        for px in self.fb.iter_mut() {
            *px = pack(18, 16, 26);
        }
        // top bar — mas alto y legible
        let bar_h: i32 = 42;
        for y in 0..bar_h {
            for x in 0..w as i32 {
                let bg = if y < 3 {
                    pack(70, 60, 95)
                } else {
                    pack(32, 30, 48)
                };
                put_fb(&mut self.fb, w, h, x, y, bg);
            }
        }
        // title in bar — escala 1 pero mas contrastado y centrado
        let title = format!(
            "VELVET FX FORGE  |  {}  |  {}  |  {}  |  {}x{}  {}f {}fps  {}",
            self.sim.preset.meta.name,
            self.sim.preset.palette.label(),
            self.sim.preset.path.kind.label(),
            self.sim.preset.dimensions.width,
            self.sim.preset.dimensions.height,
            self.sim.preset.frames,
            self.sim.preset.fps,
            if self.sim.preset.additive { "GLOW" } else { "" },
        );
        // doble pasada para negrita simulada
        draw_text(&mut self.fb, w, h, 12, 13, &title, pack(18, 16, 26), 1);
        draw_text(&mut self.fb, w, h, 11, 12, &title, pack(255, 240, 200), 1);
        let play_txt = if self.playing { "▶" } else { "||" };
        draw_text(
            &mut self.fb,
            w,
            h,
            w as i32 - 40,
            12,
            play_txt,
            pack(120, 255, 150),
            1,
        );

        // preview — enorme 8x para que llene
        let frame = self.sim.render_frame_at(self.preview_time);
        let fw = frame.width as i32;
        let fh = frame.height as i32;
        let scale = 8;
        let pw = fw * scale;
        let ph = fh * scale;
        let px = (w as i32 - pw) / 2;
        let py = (h as i32 - ph) / 2 + 24;

        // checker background
        for y in 0..ph {
            for x in 0..pw {
                let c = if ((x / 8) + (y / 8)) % 2 == 0 {
                    pack(32, 30, 42)
                } else {
                    pack(22, 20, 30)
                };
                put_fb(&mut self.fb, w, h, px + x, py + y, c);
            }
        }
        // border
        for i in 0..pw {
            put_fb(&mut self.fb, w, h, px + i, py - 1, pack(90, 85, 110));
            put_fb(&mut self.fb, w, h, px + i, py + ph, pack(90, 85, 110));
        }
        for i in 0..ph {
            put_fb(&mut self.fb, w, h, px - 1, py + i, pack(90, 85, 110));
            put_fb(&mut self.fb, w, h, px + pw, py + i, pack(90, 85, 110));
        }
        // blit scaled frame
        for y in 0..fh {
            for x in 0..fw {
                let si = ((y as u32 * frame.width + x as u32) * 4) as usize;
                if si + 3 >= frame.pixels.len() {
                    continue;
                }
                let r = frame.pixels[si];
                let g = frame.pixels[si + 1];
                let b = frame.pixels[si + 2];
                let a = frame.pixels[si + 3];
                if a == 0 {
                    continue;
                }
                let col = if a == 255 {
                    pack(r, g, b)
                } else {
                    blend_checker(r, g, b, a, x, y)
                };
                // replicate scale×scale
                for sy in 0..scale {
                    for sx in 0..scale {
                        put_fb(
                            &mut self.fb,
                            w,
                            h,
                            px + x * scale + sx,
                            py + y * scale + sy,
                            col,
                        );
                    }
                }
            }
        }
        // frame number badge over preview
        let badge = format!(
            " frame {}/{}  t={:.2}s ",
            self.frame_idx + 1,
            self.sim.preset.frames,
            self.preview_time % self.sim.preset.duration().max(0.2)
        );
        draw_text(
            &mut self.fb,
            w,
            h,
            px,
            py + ph + 6,
            &badge,
            pack(160, 155, 175),
            1,
        );

        // strip: thumbnails — tamaño adaptativo para que quepa
        let gap = 4;
        let mut thumb_scale = 3;
        let mut total_w = (fw * thumb_scale + gap) * self.sim.preset.frames as i32;
        if total_w > w as i32 - 32 {
            thumb_scale = 2;
            total_w = (fw * thumb_scale + gap) * self.sim.preset.frames as i32;
        }
        if total_w > w as i32 - 32 {
            thumb_scale = 1;
            total_w = (fw * thumb_scale + gap) * self.sim.preset.frames as i32;
        }
        let thumb_h = fh * thumb_scale;
        let strip_y = h as i32 - 32 - thumb_h - 14;
        let strip_x0 = (w as i32 - total_w) / 2;
        for idx in 0..self.sim.preset.frames {
            let f = self.sim.render_index(idx);
            let sx0 = strip_x0 + idx as i32 * (fw * thumb_scale + gap);
            let sy0 = strip_y;
            let is_current = idx == self.frame_idx;
            // highlight border for current
            if is_current {
                for oy in -1..=fh * thumb_scale {
                    for ox in -1..=fw * thumb_scale {
                        if ox == -1 || oy == -1 || ox == fw * thumb_scale || oy == fh * thumb_scale
                        {
                            put_fb(&mut self.fb, w, h, sx0 + ox, sy0 + oy, pack(255, 220, 90));
                        }
                    }
                }
            }
            // thumb bg
            for y in 0..fh * thumb_scale {
                for x in 0..fw * thumb_scale {
                    let c = if ((x / 4) + (y / 4)) % 2 == 0 {
                        pack(26, 24, 34)
                    } else {
                        pack(18, 16, 26)
                    };
                    put_fb(&mut self.fb, w, h, sx0 + x, sy0 + y, c);
                }
            }
            for y in 0..fh {
                for x in 0..fw {
                    let si = ((y as u32 * f.width + x as u32) * 4) as usize;
                    let a = f.pixels[si + 3];
                    if a == 0 {
                        continue;
                    }
                    let r = f.pixels[si];
                    let g = f.pixels[si + 1];
                    let b = f.pixels[si + 2];
                    let col = if a == 255 {
                        pack(r, g, b)
                    } else {
                        blend_checker(r, g, b, a, x, y)
                    };
                    for sy in 0..thumb_scale {
                        for sx in 0..thumb_scale {
                            put_fb(
                                &mut self.fb,
                                w,
                                h,
                                sx0 + x * thumb_scale + sx,
                                sy0 + y * thumb_scale + sy,
                                col,
                            );
                        }
                    }
                }
            }
        }
        // strip label
        draw_text(
            &mut self.fb,
            w,
            h,
            12,
            strip_y - 14,
            "Strip (all frames) — loop preview",
            pack(120, 115, 135),
            1,
        );

        // right panel — enorme
        let rx = w as i32 - 410;
        let ry = 64;
        panel(
            &mut self.fb,
            w,
            h,
            rx,
            ry,
            380,
            460,
            pack(28, 26, 38),
            pack(85, 75, 120),
        );
        let mut ly = ry + 12;
        draw_text(
            &mut self.fb,
            w,
            h,
            rx + 12,
            ly,
            "INSPECTOR",
            pack(255, 220, 120),
            1,
        );
        ly += 20;
        // palette swatches
        let pal = self.sim.preset.effective_palette();
        for (i, col) in pal.iter().take(7).enumerate() {
            let x = rx + 12 + i as i32 * 14;
            let y = ly;
            for oy in 0..10 {
                for ox in 0..10 {
                    put_fb(
                        &mut self.fb,
                        w,
                        h,
                        x + ox,
                        y + oy,
                        pack(col[0], col[1], col[2]),
                    );
                }
            }
            // border
            for k in 0..10 {
                put_fb(&mut self.fb, w, h, x + k, y, pack(0, 0, 0));
                put_fb(&mut self.fb, w, h, x + k, y + 9, pack(0, 0, 0));
                put_fb(&mut self.fb, w, h, x, y + k, pack(0, 0, 0));
                put_fb(&mut self.fb, w, h, x + 9, y + k, pack(0, 0, 0));
            }
        }
        ly += 16;
        let info = [
            format!("Paleta: {}", self.sim.preset.palette.label()),
            format!(
                "Path: {}  x{:.2} y{:.2}",
                self.sim.preset.path.kind.label(),
                self.sim.preset.path.scale_x,
                self.sim.preset.path.scale_y
            ),
            format!(
                "Freq {:.2}  Rate {:.0}",
                self.sim.preset.path.frequency, self.sim.preset.emitter.rate
            ),
            format!(
                "Gravity {:.1}  Drag {:.2}",
                self.sim.preset.particles.gravity, self.sim.preset.particles.drag
            ),
            format!(
                "Size {}  {}",
                self.sim.preset.particles.size,
                if self.sim.preset.additive {
                    "GLOW ON"
                } else {
                    "GLOW OFF"
                }
            ),
        ];
        for line in &info {
            draw_text(
                &mut self.fb,
                w,
                h,
                rx + 12,
                ly,
                line,
                pack(210, 205, 230),
                1,
            );
            ly += 15;
        }
        // separator
        for x in 0..208 {
            put_fb(&mut self.fb, w, h, rx + 12 + x, ly, pack(55, 50, 70));
        }
        ly += 8;
        // Objetivo + creatividad
        let target_label = if let Some(idx) = self.target_idx {
            self.designs[idx].0.clone()
        } else {
            "LIBRE".into()
        };
        draw_text(
            &mut self.fb,
            w,
            h,
            rx + 12,
            ly,
            &format!("OBJETIVO: {}", target_label),
            if self.target_idx.is_some() {
                pack(120, 255, 150)
            } else {
                pack(160, 155, 175)
            },
            1,
        );
        ly += 15;
        draw_text(
            &mut self.fb,
            w,
            h,
            rx + 12,
            ly,
            &format!("CREATIVIDAD {:.0}%", self.creativity * 100.0),
            pack(180, 175, 200),
            1,
        );
        ly += 13;
        // bar
        let bar_w = 180;
        let bar_h = 8;
        for oy in 0..bar_h {
            for ox in 0..bar_w {
                let bg = if ox as f32 / bar_w as f32 <= self.creativity {
                    pack(255, 180, 80)
                } else {
                    pack(40, 38, 55)
                };
                put_fb(&mut self.fb, w, h, rx + 12 + ox, ly + oy, bg);
            }
        }
        // ticks
        for t in [0.25, 0.5, 0.75] {
            let x = rx + 12 + (t * bar_w as f32) as i32;
            for oy in 0..bar_h {
                put_fb(&mut self.fb, w, h, x, ly + oy, pack(18, 16, 26));
            }
        }
        ly += 14;
        draw_text(
            &mut self.fb,
            w,
            h,
            rx + 12,
            ly,
            "C/V o Up/Down  D=diseno G=libre",
            pack(140, 135, 160),
            1,
        );
        ly += 14;
        draw_text(
            &mut self.fb,
            w,
            h,
            rx + 12,
            ly,
            "R genera cercano al objetivo",
            pack(140, 135, 160),
            1,
        );

        // left panel — enorme
        let lx = 14;
        let lw = 380;
        panel(
            &mut self.fb,
            w,
            h,
            lx,
            ry,
            lw,
            520,
            pack(28, 26, 38),
            pack(85, 75, 120),
        );
        ly = ry + 12;
        draw_text(
            &mut self.fb,
            w,
            h,
            lx + 12,
            ly,
            "CONTROLS",
            pack(255, 220, 120),
            1,
        );
        ly += 20;
        let ctrls = [
            "R Random (o cercano si hay objetivo)",
            "D Diseno  G Libre  M Mutar",
            "C/V  Creatividad  Up/Down",
            "P Paleta  K Path  +/- Scale",
            "[/] Freq  ,/. FPS  B Glow",
            "Space Play  F Canvas  N Frames",
            "S Export PNG  J Save JSON  H Ayuda",
        ];
        for line in &ctrls {
            draw_text(
                &mut self.fb,
                w,
                h,
                lx + 12,
                ly,
                line,
                pack(210, 205, 230),
                1,
            );
            ly += 15;
        }
        ly += 6;
        for x in 0..208 {
            put_fb(&mut self.fb, w, h, lx + 12 + x, ly, pack(55, 50, 70));
        }
        ly += 8;
        draw_text(
            &mut self.fb,
            w,
            h,
            lx + 12,
            ly,
            "DISENOS POPULARES (D para ciclar):",
            pack(160, 155, 175),
            1,
        );
        ly += 15;
        // show designs list with highlight for target
        for (i, (name, _)) in self.designs.iter().enumerate().take(8) {
            let is_target = self.target_idx == Some(i);
            let prefix = if is_target { ">" } else { " " };
            let txt = format!("{} {}: {}", prefix, i + 1, name);
            let col = if is_target {
                pack(120, 255, 150)
            } else {
                pack(180, 175, 200)
            };
            draw_text(&mut self.fb, w, h, lx + 12, ly, &txt, col, 1);
            ly += 13;
        }
        ly += 4;
        draw_text(
            &mut self.fb,
            w,
            h,
            lx + 12,
            ly,
            "EXAMPLES (1..8) presets base:",
            pack(160, 155, 175),
            1,
        );
        ly += 14;
        for (i, p) in self.examples.iter().enumerate() {
            let sel = if i == self.preset_idx { ">" } else { " " };
            let s = format!("{} {}:{}", sel, i + 1, p.meta.name);
            let col = if i == self.preset_idx {
                pack(255, 240, 130)
            } else {
                pack(140, 135, 160)
            };
            draw_text(&mut self.fb, w, h, lx + 12, ly, &s, col, 1);
            ly += 12;
        }

        // bottom status bar — mas alto para legibilidad
        let sh = 32;
        let sy0 = h as i32 - sh;
        for y in sy0..h as i32 {
            for x in 0..w as i32 {
                put_fb(&mut self.fb, w, h, x, y, pack(28, 26, 38));
            }
        }
        for x in 0..w as i32 {
            put_fb(&mut self.fb, w, h, x, sy0, pack(55, 50, 70));
        }
        draw_text(
            &mut self.fb,
            w,
            h,
            10,
            sy0 + 8,
            &self.status,
            pack(200, 195, 220),
            1,
        );
        // fps-ish
        let right = format!(
            "t {:.1}s  frame {}  {}×{}",
            self.preview_time,
            self.frame_idx + 1,
            self.sim.preset.dimensions.width,
            self.sim.preset.dimensions.height
        );
        let tw = right.len() as i32 * 6;
        draw_text(
            &mut self.fb,
            w,
            h,
            w as i32 - tw - 10,
            sy0 + 8,
            &right,
            pack(140, 135, 160),
            1,
        );

        // help overlay if toggled
        if self.show_help {
            let hx = (w as i32 - 560) / 2;
            let hy = 80;
            panel(
                &mut self.fb,
                w,
                h,
                hx,
                hy,
                560,
                420,
                pack(22, 20, 30),
                pack(90, 80, 110),
            );
            let lines = [
                "Velvet FX Forge — Forja VFX pixel-art 100% Velvet Engine",
                "",
                "Genera efectos loopables sin dibujar frame a frame.",
                "R = Randomize — boton estrella como en BitFX Forge.",
                "",
                "NOVEDAD: Diseno guiado + Creatividad",
                "  D = ciclo Fuego/Agua/Humo/Magia/Rayo...  G = libre",
                "  C/V o Up/Down = creatividad 0% fiel .. 100% loco",
                "  R genera cercano al objetivo si hay candado",
                "  Si no quieres guiado, deja en LIBRE y R = random total",
                "",
                "Flujo:  Elige objetivo D -> ajusta C/V -> R genera -> S exporta",
                "  Tweaks: P paleta K path +/- scale [/]freq ,/.fps B glow",
                "",
                "Exporta a output/ de FX Forge:  .png spritesheet + .json atlas + _preset.json",
                "Usa en juego: FxSim::new(preset) + render_frame_at(t)",
                "",
                "Teclas: R M D G C V B P K +/- Space F N S J H  1..8 Esc",
                "H oculta ayuda  —  fuente 5x7 legible, strip arriba de status",
            ];
            let mut yy = hy + 14;
            for line in &lines {
                draw_text(
                    &mut self.fb,
                    w,
                    h,
                    hx + 14,
                    yy,
                    line,
                    pack(210, 205, 230),
                    1,
                );
                yy += 14;
            }
        }
    }

    fn present(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let ww = size.width.max(1);
        let wh = size.height.max(1);
        if ww != self.fb_w || wh != self.fb_h {
            self.fb_w = ww;
            self.fb_h = wh;
            self.fb.resize((ww * wh) as usize, 0);
        }
        self.compose();
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(NonZeroU32::new(ww).unwrap(), NonZeroU32::new(wh).unwrap());
        if let Ok(mut buf) = surface.buffer_mut() {
            let n = buf.len().min(self.fb.len());
            buf[..n].copy_from_slice(&self.fb[..n]);
            let _ = buf.present();
        }
    }
}

#[inline]
fn put_fb(fb: &mut [u32], w: u32, h: u32, x: i32, y: i32, col: u32) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let i = (y as u32 * w + x as u32) as usize;
    if i < fb.len() {
        fb[i] = col;
    }
}

fn blend_checker(r: u8, g: u8, b: u8, a: u8, x: i32, y: i32) -> u32 {
    let bg = if (x + y) % 2 == 0 { 32u8 } else { 22u8 };
    let alpha = a as f32 / 255.0;
    let rr = (r as f32 * alpha + bg as f32 * (1.0 - alpha)) as u8;
    let gg = (g as f32 * alpha + bg as f32 * (1.0 - alpha)) as u8;
    let bb = (b as f32 * alpha + bg as f32 * (1.0 - alpha)) as u8;
    pack(rr, gg, bb)
}

fn panel(fb: &mut [u32], w: u32, h: u32, x: i32, y: i32, pw: i32, ph: i32, bg: u32, border: u32) {
    for oy in 0..ph {
        for ox in 0..pw {
            let is_border = ox == 0 || oy == 0 || ox == pw - 1 || oy == ph - 1;
            put_fb(
                fb,
                w,
                h,
                x + ox,
                y + oy,
                if is_border { border } else { bg },
            );
        }
    }
}

// Fuente TTF normal legible — 18px / 26px (antes 15/20)
fn draw_text(fb: &mut [u32], w: u32, h: u32, x: i32, y: i32, s: &str, col: u32, scale: i32) {
    let sz = if scale >= 2 { 26.0 } else { 18.0 };
    draw_text_ttf(fb, w, h, x, y, s, col, sz);
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Velvet FX Forge — BitFX-like Forge (R=randomize)")
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H))
            .with_min_inner_size(LogicalSize::new(960, 600));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        let ctx = Context::new(window.clone()).expect("ctx");
        let surf = Surface::new(&ctx, window.clone()).expect("surface");
        self.context = Some(ctx);
        self.surface = Some(surf);
        self.window = Some(window);
        self.last = Instant::now();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                match code {
                    KeyCode::Escape => el.exit(),
                    KeyCode::Space => {
                        self.playing = !self.playing;
                        self.set_status(if self.playing {
                            "Play ▶"
                        } else {
                            "Paused ⏸"
                        });
                    }
                    KeyCode::KeyH => self.show_help = !self.show_help,
                    KeyCode::KeyR => self.randomize(),
                    KeyCode::KeyM => self.mutate(),
                    KeyCode::KeyD => self.cycle_target(),
                    KeyCode::KeyG => self.clear_target(),
                    KeyCode::KeyC => self.adjust_creativity(-0.1),
                    KeyCode::KeyV => self.adjust_creativity(0.1),
                    KeyCode::KeyB => self.toggle_additive(),
                    KeyCode::KeyP => self.next_palette(),
                    KeyCode::KeyK => self.next_path(),
                    KeyCode::KeyF => self.cycle_dimensions(),
                    KeyCode::KeyN => self.cycle_frames(),
                    KeyCode::KeyS => self.export_sheet(),
                    KeyCode::KeyJ => self.save_json(),
                    KeyCode::ArrowUp => self.adjust_creativity(0.05),
                    KeyCode::ArrowDown => self.adjust_creativity(-0.05),
                    KeyCode::Equal | KeyCode::NumpadAdd => self.adjust_scale(0.12, 0.12),
                    KeyCode::Minus | KeyCode::NumpadSubtract => self.adjust_scale(-0.12, -0.12),
                    KeyCode::BracketLeft => {
                        let mut p = self.sim.preset.clone();
                        p.path.frequency = (p.path.frequency - 0.2).clamp(0.2, 4.0);
                        let f = p.path.frequency;
                        self.sim = FxSim::new(p);
                        self.set_status(format!("Freq {f:.2}"));
                    }
                    KeyCode::BracketRight => {
                        let mut p = self.sim.preset.clone();
                        p.path.frequency = (p.path.frequency + 0.2).clamp(0.2, 4.0);
                        let f = p.path.frequency;
                        self.sim = FxSim::new(p);
                        self.set_status(format!("Freq {f:.2}"));
                    }
                    KeyCode::Comma => {
                        let mut p = self.sim.preset.clone();
                        p.fps = (p.fps as i32 - 2).clamp(6, 30) as u32;
                        let fps = p.fps;
                        self.sim = FxSim::new(p);
                        self.set_status(format!("FPS {fps}"));
                    }
                    KeyCode::Period => {
                        let mut p = self.sim.preset.clone();
                        p.fps = (p.fps as i32 + 2).clamp(6, 30) as u32;
                        let fps = p.fps;
                        self.sim = FxSim::new(p);
                        self.set_status(format!("FPS {fps}"));
                    }
                    KeyCode::Digit1 => {
                        if let Some(p) = self.examples.first().cloned() {
                            self.preset_idx = 0;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit2 => {
                        if let Some(p) = self.examples.get(1).cloned() {
                            self.preset_idx = 1;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit3 => {
                        if let Some(p) = self.examples.get(2).cloned() {
                            self.preset_idx = 2;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit4 => {
                        if let Some(p) = self.examples.get(3).cloned() {
                            self.preset_idx = 3;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit5 => {
                        if let Some(p) = self.examples.get(4).cloned() {
                            self.preset_idx = 4;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit6 => {
                        if let Some(p) = self.examples.get(5).cloned() {
                            self.preset_idx = 5;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit7 => {
                        if let Some(p) = self.examples.get(6).cloned() {
                            self.preset_idx = 6;
                            self.load_preset(p);
                        }
                    }
                    KeyCode::Digit8 => {
                        if let Some(p) = self.examples.get(7).cloned() {
                            self.preset_idx = 7;
                            self.load_preset(p);
                        }
                    }
                    _ => {}
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.05);
                self.last = now;
                self.tick(dt);
                self.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.headless {
            self.hframes += 1;
            self.tick(1.0 / 60.0);
            if self.hframes == 10 {
                // headless: render all examples, export one, assert ok
                println!("=== Velvet FX Forge — headless check ===");
                for (i, preset) in self.examples.clone().iter().enumerate() {
                    let sim = FxSim::new(preset.clone());
                    let frame = sim.render_frame_at(0.15);
                    let filled = frame.pixels.chunks(4).filter(|c| c[3] > 20).count();
                    println!(
                        "preset {} {} {}×{} filled={} palette={:?}",
                        i,
                        preset.meta.name,
                        preset.dimensions.width,
                        preset.dimensions.height,
                        filled,
                        preset.palette
                    );
                    assert!(filled > 20, "preset should draw something");
                    let sheet = velvet_fx::export::export_spritesheet(&sim, None).expect("sheet");
                    assert!(sheet.0.width > 0 && sheet.0.height > 0);
                }
                let sim = FxSim::new(velvet_fx::random_preset(0x123456));
                let frame = sim.render_frame_at(0.2);
                assert!(frame.pixels.chunks(4).any(|c| c[3] > 0));
                let mut p = self.sim.preset.clone();
                p = velvet_fx::mutate_preset(&p, 0x999);
                let sim2 = FxSim::new(p);
                let _ = sim2.render_frame_at(0.1);
                // export temp
                let out_dir = export_dir();
                let out = out_dir.join("headless_test.png");
                let _ = std::fs::create_dir_all(out_dir);
                let sim3 = FxSim::new(self.sim.preset.clone());
                let _ = velvet_fx::export::export_spritesheet_to_files(&sim3, &out, None);
                println!("headless export {} exists={}", out.display(), out.exists());
                println!("ASSERT_OK fx_forge_headless");
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
    velvet_core::init_tracing_default("fx_forge=info,warn");
    let headless = std::env::args().any(|a| a == "--headless");
    println!("=== Velvet FX Forge — BitFX-like pixel VFX forge (Velvet Engine) ===");
    println!("R Randomize  M Mutate  P Palette  K Path  +/- Scale  Space Pause  S Export  J JSON  H Help  1..8 presets");
    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(headless);
    el.run_app(&mut app)?;
    Ok(())
}
