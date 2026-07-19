//! Velvet Novella — **Luz de Estación**
//!
//! Title menu → product VN host (`VnSession` + product paint + softbuffer).
//!
//! Title: ↑↓ Enter · Esc  
//! Play: Space/Click advance · ↑↓ choices · R restart to menu · Esc quit  
//! `--headless`: title auto-start + auto-play to ending

mod menu;

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

use menu::{
    font_status, letterbox_bilinear, load_rgb, move_sel, paint_novel_menu,
    paint_novel_menu_size, RgbImage, MENU_ITEMS, WW, WH,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    Play,
}

struct App {
    screen: Screen,
    menu_sel: usize,
    menu_bg: Option<RgbImage>,
    session: VnSession,
    story_path: PathBuf,
    window: Option<Arc<Window>>,
    context: Option<SbContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    headless_done: bool,
    /// Logical framebuffer (title always 1280×720; play may letterbox into window).
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

fn ui_dir() -> PathBuf {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui"),
        PathBuf::from("demos/velvet-novella/data/ui"),
        PathBuf::from("data/ui"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("menu_bg.jpg").exists())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui"))
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

// presentation scale is bilinear — see menu::letterbox_bilinear

impl App {
    fn new(headless: bool) -> Result<Self> {
        let story_path = story_path();
        let session = open_session(&story_path)?;
        let menu_bg = load_rgb(&ui_dir().join("menu_bg.jpg"));
        Ok(Self {
            screen: Screen::Title,
            menu_sel: 0,
            menu_bg,
            session,
            story_path,
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            headless_done: false,
            pixels: vec![0; (WW * WH) as usize],
        })
    }

    fn start_game(&mut self) {
        if let Ok(s) = open_session(&self.story_path) {
            self.session = s;
        }
        self.screen = Screen::Play;
    }

    fn to_title(&mut self) {
        self.screen = Screen::Title;
        self.menu_sel = 0;
    }

    fn confirm_menu(&mut self, el: &ActiveEventLoop) {
        match self.menu_sel {
            0 => self.start_game(),
            4 => el.exit(),
            _ => {
                // stubs: Continuar / Galería / Opciones
            }
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
        let dw = size.width.max(1);
        let dh = size.height.max(1);

        match self.screen {
            Screen::Title => {
                // Paint at *native* window size so DPI never nearest-upsamples fonts
                if self.pixels.len() != (dw * dh) as usize {
                    self.pixels.resize((dw * dh) as usize, 0);
                }
                paint_novel_menu_size(
                    &mut self.pixels,
                    dw,
                    dh,
                    self.menu_bg.as_ref(),
                    self.menu_sel,
                );
                window.set_title("Luz de Estación — menú");
            }
            Screen::Play => {
                // Product path still uses design res; bilinear letterbox to window
                if self.pixels.len() != (WW * WH) as usize {
                    self.pixels.resize((WW * WH) as usize, 0);
                }
                let list = present_session(&self.session, &mut self.pixels, WW, WH);
                let _ = list;
                if matches!(self.session.player().wait(), StoryWait::Ended) {
                    let end = self
                        .session
                        .player()
                        .variables()
                        .get("ending")
                        .display_str();
                    let msg = format!("FIN  ending={end}  (R menu)");
                    velvet_story::draw_text_line(
                        &mut self.pixels,
                        WW,
                        WH,
                        40,
                        (WH / 3) as i32,
                        "=== FIN ===",
                        velvet_story::pack_rgb(255, 220, 120),
                        3,
                    );
                    velvet_story::draw_text_line(
                        &mut self.pixels,
                        WW,
                        WH,
                        40,
                        (WH / 3) as i32 + 36,
                        &msg,
                        velvet_story::pack_rgb(220, 220, 235),
                        2,
                    );
                }
                velvet_story::draw_text_line(
                    &mut self.pixels,
                    WW,
                    WH,
                    12,
                    10,
                    "Space/Click  |  Up/Down  |  R menu  |  Esc quit",
                    velvet_story::pack_rgb(150, 145, 165),
                    1,
                );
                let frame = build_product_ui_frame(&self.session);
                let title = if frame.wait == "ended" {
                    "Luz de Estación — FIN".into()
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
                window.set_title(&title);
            }
        }

        let present = match self.screen {
            Screen::Title => {
                // Already painted at native size
                self.pixels.clone()
            }
            Screen::Play => {
                if dw == WW && dh == WH {
                    self.pixels.clone()
                } else {
                    letterbox_bilinear(
                        &self.pixels,
                        WW,
                        WH,
                        dw,
                        dh,
                        velvet_story::pack_rgb(8, 6, 14),
                    )
                }
            }
        };

        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(NonZeroU32::new(dw).unwrap(), NonZeroU32::new(dh).unwrap());
        if let Ok(mut buf) = surface.buffer_mut() {
            let n = present.len().min(buf.len());
            buf[..n].copy_from_slice(&present[..n]);
            let _ = buf.present();
        }
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
                match self.screen {
                    Screen::Title => match c {
                        KeyCode::ArrowUp | KeyCode::KeyW => {
                            self.menu_sel = move_sel(self.menu_sel, -1);
                        }
                        KeyCode::ArrowDown | KeyCode::KeyS => {
                            self.menu_sel = move_sel(self.menu_sel, 1);
                        }
                        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                            self.confirm_menu(el);
                        }
                        KeyCode::Escape => el.exit(),
                        _ => {}
                    },
                    Screen::Play => match c {
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
                        KeyCode::KeyR => self.to_title(),
                        KeyCode::Escape => el.exit(),
                        _ => {}
                    },
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                match self.screen {
                    Screen::Title => self.confirm_menu(el),
                    Screen::Play => self.advance_or_choose(),
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.05);
                self.last = now;
                if self.screen == Screen::Play {
                    self.session.tick(dt);
                }
                self.paint();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.headless {
            if self.headless_done {
                el.exit();
                return;
            }
            self.hframes += 1;
            match self.screen {
                Screen::Title => {
                    // Auto-start after a few frames (paint title path first)
                    if self.hframes == 3 {
                        if self.pixels.len() != (WW * WH) as usize {
                            self.pixels.resize((WW * WH) as usize, 0);
                        }
                        paint_novel_menu(&mut self.pixels, self.menu_bg.as_ref(), 0);
                        let fonts = font_status()
                            .map(|(t, u)| format!("title={t} ui={u}"))
                            .unwrap_or_else(|| "fonts=MISSING".into());
                        println!(
                            "headless title_menu painted items={} bg={} {fonts}",
                            MENU_ITEMS.len(),
                            self.menu_bg.is_some()
                        );
                    }
                    if self.hframes >= 5 {
                        self.start_game();
                        println!("headless start Nueva partida");
                    }
                }
                Screen::Play => {
                    self.session.tick(1.0 / 30.0);
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
                                "headless ending={end} steps={} paint_cmds={}",
                                self.hframes,
                                list.len()
                            );
                            println!("ASSERT_OK velvet_novella_title_and_play");
                            self.headless_done = true;
                            el.exit();
                            return;
                        }
                        _ => {}
                    }
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
    println!("=== Luz de Estación — novela visual ===");
    println!("menu: Nueva partida · Continuar · Galería · Opciones · Salir");
    if let Some((t, u)) = font_status() {
        println!("fonts: title={t}  ui={u}  (TrueType AA, native DPI paint)");
    } else {
        println!("WARNING: no TTF fonts loaded — menu quality degraded");
    }
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
        let painted = pixels
            .iter()
            .filter(|&&p| p != 0 && p != velvet_story::pack_rgb(8, 6, 14))
            .count();
        assert!(
            painted > 100,
            "rasterized product frame should paint pixels, got {painted}"
        );
    }
}
