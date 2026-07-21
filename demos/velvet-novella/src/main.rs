//! Velvet Novella — **Luz de Estación**
//!
//! Title menu (softbuffer, resolution = window) → product VN host
//! (`VnSession` + hybrid presenter: menu softbuffer, play prefers wgpu descriptors).
//!
//! ## Language / pipeline (honest)
//! Story is `story/main.vel` loaded via `open_session_from_file` →
//! `velvet_script_parser` AST → lower to **`StoryProgram`** (product IR) →
//! `VnSession` / `StoryPlayer`.
//!
//! That is **not** the full VS2 stack (HIR → types → `OpVs2` bytecode → VM).
//! VS2 (`.vel` edition 2, typed VM) is **alpha / partial** and is **not** the
//! runtime driving this demo today. Same file extension (`.vel`), different depth.
//!
//! Title: ↑↓ Enter · Esc  
//! Play: Space/Click · ↑↓ choices · R menu · Esc quit  
//! `--headless`: title auto-start + auto-play to ending

mod menu;

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
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
    compose_size_for_window, font_status, letterbox_bilinear, load_rgb, move_sel, paint_novel_menu,
    paint_novel_menu_size, RgbImage, MAX_COMPOSE_EDGE, MENU_ITEMS, WH, WW,
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
    /// Compose framebuffer (window size, capped — not fixed 4K).
    pixels: Vec<u32>,
    /// Last compose width/height.
    compose_w: u32,
    compose_h: u32,
    /// Hybrid product presenter (title softbuffer / play wgpu IR).
    presenter: ProductPresenter,
    /// Dirty: need paint on next redraw.
    needs_paint: bool,
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

fn open_session(path: &Path) -> Result<VnSession> {
    open_session_from_file(
        path,
        "Luz de Estación",
        Some(PathBuf::from("saves/velvet-novella")),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
    .with_context(|| format!("load {}", path.display()))
}

impl App {
    fn new(headless: bool) -> Result<Self> {
        let story_path = story_path();
        let session = open_session(&story_path)?;
        let menu_bg = load_rgb(&ui_dir().join("menu_bg.jpg"));
        let mut presenter = ProductPresenter::hybrid();
        // Interactive: no headless GPU probe (avoids WARP + Vulkan layer spam).
        if headless {
            match velvet_render::GpuContext::headless() {
                Ok(g) => {
                    eprintln!("[presenter] wgpu probe: {}", g.adapter_info);
                    presenter.set_gpu_available(true, None::<String>);
                }
                Err(e) => {
                    eprintln!("[presenter] wgpu probe failed → softbuffer play: {e}");
                    presenter.set_gpu_available(false, Some(e.to_string()));
                }
            }
        } else {
            eprintln!(
                "[presenter] hybrid: menú softbuffer · juego paint IR wgpu · ventana softbuffer"
            );
        }
        presenter.set_phase_title();
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
            compose_w: WW,
            compose_h: WH,
            presenter,
            needs_paint: true,
        })
    }

    fn start_game(&mut self) {
        if let Ok(s) = open_session(&self.story_path) {
            self.session = s;
        }
        self.screen = Screen::Play;
        self.presenter.set_phase_play();
        self.needs_paint = true;
        eprintln!("[presenter] {}", self.presenter.status_line());
    }

    fn return_to_title(&mut self) {
        self.screen = Screen::Title;
        self.menu_sel = 0;
        self.presenter.set_phase_title();
        self.needs_paint = true;
    }

    fn confirm_menu(&mut self, el: &ActiveEventLoop) {
        match self.menu_sel {
            0 => self.start_game(),
            4 => el.exit(),
            _ => {}
        }
        self.needs_paint = true;
    }

    fn advance_or_choose(&mut self) {
        if self.session.choice.open {
            let _ = self.session.choose_selected();
        } else if matches!(self.session.player().wait(), StoryWait::Ended) {
            // stay on ending
        } else {
            self.session.advance();
        }
        self.needs_paint = true;
    }

    fn ensure_compose_buf(&mut self, cw: u32, ch: u32) {
        let n = (cw * ch) as usize;
        if self.pixels.len() != n {
            self.pixels.resize(n, 0);
        }
        self.compose_w = cw;
        self.compose_h = ch;
    }

    fn paint(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let dw = size.width.max(1);
        let dh = size.height.max(1);
        let (cw, ch) = compose_size_for_window(dw, dh);
        self.ensure_compose_buf(cw, ch);

        match self.screen {
            Screen::Title => {
                paint_novel_menu_size(
                    &mut self.pixels,
                    cw,
                    ch,
                    self.menu_bg.as_ref(),
                    self.menu_sel,
                );
                window.set_title(&format!(
                    "Luz de Estación — menú ({cw}×{ch} → ventana {dw}×{dh})"
                ));
            }
            Screen::Play => {
                let _list = self.presenter.present_session_softbuffer(
                    &self.session,
                    &mut self.pixels,
                    cw,
                    ch,
                );
                if matches!(self.session.player().wait(), StoryWait::Ended) {
                    let end = self
                        .session
                        .player()
                        .variables()
                        .get("ending")
                        .display_str();
                    let msg = format!("FIN  ending={end}  (R menu)");
                    let scale = (cw / 640).max(1) as i32;
                    velvet_story::draw_text_line(
                        &mut self.pixels,
                        cw,
                        ch,
                        40,
                        (ch / 3) as i32,
                        "=== FIN ===",
                        velvet_story::pack_rgb(255, 220, 120),
                        scale * 2,
                    );
                    velvet_story::draw_text_line(
                        &mut self.pixels,
                        cw,
                        ch,
                        40,
                        (ch / 3) as i32 + 28 * scale,
                        &msg,
                        velvet_story::pack_rgb(220, 220, 235),
                        scale,
                    );
                }
                let hint_scale = (cw / 960).max(1) as i32;
                velvet_story::draw_text_line(
                    &mut self.pixels,
                    cw,
                    ch,
                    16,
                    12,
                    "Space/Click  |  Up/Down  |  R menu  |  Esc quit",
                    velvet_story::pack_rgb(150, 145, 165),
                    hint_scale,
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

        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(NonZeroU32::new(dw).unwrap(), NonZeroU32::new(dh).unwrap());
        if let Ok(mut buf) = surface.buffer_mut() {
            // 1:1 → direct copy (no bilinear letterbox). Else scale once.
            if dw == cw && dh == ch {
                let n = self.pixels.len().min(buf.len());
                buf[..n].copy_from_slice(&self.pixels[..n]);
            } else {
                let present = letterbox_bilinear(
                    &self.pixels,
                    cw,
                    ch,
                    dw,
                    dh,
                    velvet_story::pack_rgb(8, 6, 14),
                );
                let n = present.len().min(buf.len());
                buf[..n].copy_from_slice(&present[..n]);
            }
            let _ = buf.present();
        }
        self.needs_paint = false;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // Prefer primary monitor size; start maximized so resolution = pantalla.
        let mut attrs = Window::default_attributes()
            .with_title("Luz de Estación — Velvet Novella")
            .with_inner_size(LogicalSize::new(1280, 720))
            .with_min_inner_size(LogicalSize::new(640, 360))
            .with_maximized(true);
        if let Some(monitor) = el.primary_monitor() {
            let size = monitor.size();
            // Logical size hint before maximize (some platforms use it for restore).
            attrs =
                attrs.with_inner_size(LogicalSize::new(size.width.max(800), size.height.max(600)));
        }
        let window = Arc::new(el.create_window(attrs).expect("window"));
        window.set_maximized(true);
        let context = SbContext::new(window.clone()).expect("ctx");
        let surface = Surface::new(&context, window.clone()).expect("surface");
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
        self.last = Instant::now();
        self.needs_paint = true;
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.needs_paint = true;
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
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
                            self.needs_paint = true;
                        }
                        KeyCode::ArrowDown | KeyCode::KeyS => {
                            self.menu_sel = move_sel(self.menu_sel, 1);
                            self.needs_paint = true;
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
                                self.needs_paint = true;
                            }
                        }
                        KeyCode::ArrowDown | KeyCode::KeyS => {
                            if self.session.choice.open {
                                self.session.choice.move_sel(1);
                                self.needs_paint = true;
                            }
                        }
                        KeyCode::Digit1 | KeyCode::Numpad1 => {
                            if self.session.choice.open {
                                let _ = self.session.choose_arm(0);
                                self.needs_paint = true;
                            }
                        }
                        KeyCode::Digit2 | KeyCode::Numpad2 => {
                            if self.session.choice.open {
                                let _ = self.session.choose_arm(1);
                                self.needs_paint = true;
                            }
                        }
                        KeyCode::Digit3 | KeyCode::Numpad3 => {
                            if self.session.choice.open {
                                let _ = self.session.choose_arm(2);
                                self.needs_paint = true;
                            }
                        }
                        KeyCode::Digit4 | KeyCode::Numpad4 => {
                            if self.session.choice.open {
                                let _ = self.session.choose_arm(3);
                                self.needs_paint = true;
                            }
                        }
                        KeyCode::KeyR => self.return_to_title(),
                        KeyCode::Escape => el.exit(),
                        _ => {}
                    },
                }
                if self.needs_paint {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
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
                    // Typewriter / auto may need continuous frames while playing.
                    self.needs_paint = true;
                }
                if self.needs_paint || self.screen == Screen::Play {
                    self.paint();
                }
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
                    if self.hframes == 3 {
                        self.ensure_compose_buf(WW, WH);
                        paint_novel_menu(&mut self.pixels, self.menu_bg.as_ref(), 0);
                        let fonts = font_status()
                            .map(|(t, u)| format!("title={t} ui={u}"))
                            .unwrap_or_else(|| "fonts=MISSING".into());
                        println!(
                            "headless title_menu {}x{} items={} bg={} {fonts}",
                            WW,
                            WH,
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
                        self.ensure_compose_buf(WW, WH);
                        let _ = self.presenter.present_session_softbuffer(
                            &self.session,
                            &mut self.pixels,
                            WW,
                            WH,
                        );
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

        // Title: sleep until input (no full-repaint storm while dragging).
        // Play: ~60 Hz for typewriter; still much cheaper than 4K×every poll.
        match self.screen {
            Screen::Title => {
                el.set_control_flow(ControlFlow::Wait);
            }
            Screen::Play => {
                el.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + Duration::from_millis(16),
                ));
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
    }
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default(
        "velvet_novella=info,wgpu_hal=error,wgpu_core=error,wgpu=error,naga=error,warn",
    );
    let headless = std::env::args().any(|a| a == "--headless");
    println!("=== Luz de Estación — novela visual ===");
    println!(
        "render: resolución = ventana (máx. arista {MAX_COMPOSE_EDGE}px) · arranque maximizado"
    );
    println!("menu: Nueva partida · Continuar · Galería · Opciones · Salir");
    if let Some((t, u)) = font_status() {
        println!("fonts: title={t}  ui={u}  (TrueType AA)");
    } else {
        println!("WARNING: no TTF fonts loaded — menu quality degraded");
    }
    println!("story: demos/velvet-novella/story/main.vel");
    println!("pipeline: .vel → parser AST → StoryProgram (product IR) → VnSession");
    println!("VS2 status: NOT the runtime of this demo (VS2 HIR/bytecode/VM is alpha)");

    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Wait);
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
        let mut pixels = vec![0u32; (1280 * 720) as usize];
        let list = paint_product_session(&session);
        rasterize_product_paint(&list, &mut pixels, 1280, 720);
        let painted = pixels.iter().filter(|&&p| p != 0).count();
        assert!(
            painted > 100,
            "rasterized product frame should paint pixels, got {painted}"
        );
    }
}
