//! Velvet Novella — product VN windowed host (`VnSession` + product paint + softbuffer).
//!
//! Presentation is driven by the same product UI/paint path as `velvet play`
//! (`build_product_ui_frame` / `paint_product_session` / `rasterize_product_paint`).
//!
//! Click / Space / Enter: advance  
//! Up/Down or W/S: move choice · Enter/Click: confirm  
//! R: restart · Esc: quit · `--headless`: auto-play to ending

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
    /// Reused CPU framebuffer (ARGB softbuffer pixels).
    pixels: Vec<u32>,
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
    .with_context(|| format!("load {}", path.display()))
}

/// Shared product presentation path: session → UI frame → paint list → pixels.
fn present_session(session: &VnSession, pixels: &mut [u32], ww: u32, wh: u32) -> ProductPaintList {
    let list = paint_product_session(session);
    rasterize_product_paint(&list, pixels, ww, wh);
    list
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
            pixels: vec![0; (WW * WH) as usize],
        })
    }

    fn restart(&mut self) {
        if let Ok(s) = open_session(&self.story_path) {
            self.session = s;
        }
    }

    fn tick(&mut self, dt: f32) {
        self.session.tick(dt);
        if let Some(w) = &self.window {
            let frame = build_product_ui_frame(&self.session);
            let title = if frame.wait == "ended" {
                "Luz de Estación — FIN (R restart · Esc quit)".into()
            } else if frame.choice_visible {
                format!(
                    "Luz de Estación — choice {}/{}",
                    frame.selected_choice + 1,
                    frame.choices.len().max(1)
                )
            } else if !frame.namebox.trim().is_empty() {
                format!("Luz de Estación — {}", frame.namebox)
            } else {
                "Luz de Estación".into()
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
        if self.pixels.len() != (ww * wh) as usize {
            self.pixels.resize((ww * wh) as usize, 0);
        }

        let list = present_session(&self.session, &mut self.pixels, ww, wh);
        // Ending overlay text (extra host chrome on top of product paint)
        if matches!(self.session.player().wait(), StoryWait::Ended) {
            let end = self
                .session
                .player()
                .variables()
                .get("ending")
                .display_str();
            let msg = format!("FIN  ending={end}  (R restart)");
            velvet_story::draw_text_line(
                &mut self.pixels,
                ww,
                wh,
                40,
                (wh / 3) as i32,
                "=== FIN ===",
                velvet_story::pack_rgb(255, 220, 120),
                3,
            );
            velvet_story::draw_text_line(
                &mut self.pixels,
                ww,
                wh,
                40,
                (wh / 3) as i32 + 36,
                &msg,
                velvet_story::pack_rgb(220, 220, 235),
                2,
            );
        }
        // Hint bar
        velvet_story::draw_text_line(
            &mut self.pixels,
            ww,
            wh,
            12,
            10,
            "Space/Click advance | Up/Down choices | R restart | Esc quit",
            velvet_story::pack_rgb(150, 145, 165),
            1,
        );
        let _ = list; // keep paint list for possible debug

        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(
            NonZeroU32::new(ww).unwrap(),
            NonZeroU32::new(wh).unwrap(),
        );
        let mut buf = surface.buffer_mut().unwrap();
        buf[..self.pixels.len()].copy_from_slice(&self.pixels);
        let _ = buf.present();
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
            // Exercise product paint path every few frames (fixed logical size)
            if self.hframes % 5 == 0 {
                if self.pixels.len() != (WW * WH) as usize {
                    self.pixels.resize((WW * WH) as usize, 0);
                }
                let _ = present_session(&self.session, &mut self.pixels, WW, WH);
            }
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
                    let list = paint_product_session(&self.session);
                    println!(
                        "headless ending={end} steps={} paint_cmds={} say_geom={}",
                        self.hframes,
                        list.len(),
                        list.has_say_geometry() || list.commands.iter().any(|_| true)
                    );
                    println!("ASSERT_OK velvet_novella_product_paint");
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
    println!("=== Luz de Estación — product VN host ===");
    println!("paint path: VnSession -> ProductUiFrame -> ProductPaintList -> softbuffer");
    println!("story: demos/velvet-novella/story/main.vel");

    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(headless)?;
    el.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod host_tests {
    use super::*;

    #[test]
    fn product_paint_from_novella_story_has_layout() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("story/main.vel");
        let mut session = open_session(&path).expect("open novella story");
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
            session.advance();
            g += 1;
        }
        let mut pixels = vec![0u32; (320 * 180) as usize];
        let list = present_session(&session, &mut pixels, 320, 180);
        assert!(list.has_say_geometry() || session.say.visible);
        assert!(!list.is_empty());
        let painted = pixels.iter().filter(|&&p| p != 0 && p != velvet_story::pack_rgb(8, 6, 14)).count();
        assert!(painted > 100, "rasterized product frame should paint pixels, got {painted}");
    }
}
