// Some interactive panels are intentionally scaffolded beyond the current headless route.
#![allow(dead_code)]

mod app;
mod apps;
mod assets;
mod audio;
mod desktop;
mod input;
mod model;
mod render;
mod save;
mod windows;

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use app::App;
use model::AppKind;
use softbuffer::{Context as SoftContext, Surface};
use velvet_script_vs3::{float_val, string_val};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

const FRAME_TIME: Duration = Duration::from_micros(16_667);

struct WinitApp {
    app: App,
    window: Option<Arc<Window>>,
    context: Option<SoftContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last_tick: Instant,
    accumulator: Duration,
}

impl WinitApp {
    fn new(app: App) -> Self {
        Self {
            app,
            window: None,
            context: None,
            surface: None,
            last_tick: Instant::now(),
            accumulator: Duration::ZERO,
        }
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attr = Window::default_attributes()
                .with_title("ECHO//XP — Velvet Engine")
                .with_maximized(true)
                .with_inner_size(LogicalSize::new(800, 600));

            let window = match event_loop.create_window(attr) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    eprintln!("Failed to create window: {e}");
                    return;
                }
            };

            window.set_cursor_visible(false);

            let context = match SoftContext::new(window.clone()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed creating softbuffer context: {e}");
                    return;
                }
            };

            let surface = match Surface::new(&context, window.clone()) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed creating softbuffer surface: {e}");
                    return;
                }
            };

            self.window = Some(window);
            self.context = Some(context);
            self.surface = Some(surface);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(window)) = (&mut self.surface, &self.window) {
                    if let (Some(w), Some(h)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        let _ = surface.resize(w, h);
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    if size.width > 0 && size.height > 0 {
                        let vx = (position.x * 800.0 / size.width as f64) as i32;
                        let vy = (position.y * 600.0 / size.height as f64) as i32;
                        self.app
                            .mouse
                            .update_pos(vx.clamp(0, 799), vy.clamp(0, 599));
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.app.mouse.handle_button(button, state);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.app.keyboard.handle_key(code, event.state);
                    if self.app.keyboard.escape {
                        self.app.shell.start_menu_open = false;
                    }
                    if self.app.keyboard.alt_f4 {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = now.duration_since(self.last_tick);
                self.last_tick = now;
                self.accumulator += delta;

                while self.accumulator >= FRAME_TIME {
                    let _ = self.app.update(FRAME_TIME.as_secs_f32());
                    self.accumulator -= FRAME_TIME;
                }

                self.app.render();

                if let (Some(surface), Some(window)) = (&mut self.surface, &self.window) {
                    if let Ok(mut buffer) = surface.buffer_mut() {
                        let size = window.inner_size();
                        let src_w = render::WIDTH;
                        let src_h = render::HEIGHT;

                        for y in 0..size.height {
                            let sy = (y as usize * src_h) / size.height as usize;
                            for x in 0..size.width {
                                let sx = (x as usize * src_w) / size.width as usize;
                                buffer[(y * size.width + x) as usize] =
                                    self.app.renderer.buffer[sy * src_w + sx];
                            }
                        }
                        let _ = buffer.present();
                    }
                }
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--headless") {
        return run_headless();
    }

    if let Some(pos) = args.iter().position(|arg| arg == "--capture-screen") {
        if pos + 2 < args.len() {
            let stage = &args[pos + 1];
            let output_path = PathBuf::from(&args[pos + 2]);
            return run_capture_screen(stage, &output_path);
        } else {
            bail!("Usage: --capture-screen <stage> <output_path>");
        }
    }

    let app = App::new()?;
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut winit_app = WinitApp::new(app);
    event_loop.run_app(&mut winit_app)?;
    Ok(())
}

fn run_headless() -> Result<()> {
    println!("Running ECHO//XP headless acceptance suite...");
    let mut app = App::new()?;

    // 1. Boot phase check
    let frame = app.game.frame.as_ref().context("Missing initial frame")?;
    if frame.phase != "boot" {
        bail!("Expected phase 'boot', got '{}'", frame.phase);
    }
    println!("boot=ok");

    // 2. Complete boot -> login phase
    let frame = app.game.call("complete_boot", &[])?;
    if frame.phase != "login" {
        bail!("Expected phase 'login', got '{}'", frame.phase);
    }
    println!("login=ok");

    // 3. Login -> desktop phase
    let frame = app.game.call("login", &[])?;
    if frame.phase != "desktop" {
        bail!("Expected phase 'desktop', got '{}'", frame.phase);
    }

    // 4. Collect 4 pieces of evidence
    app.game.call("read_mail", &[string_val("mail_001")])?;
    app.game
        .call("read_case_file", &[string_val("CASE_001_MARA_V")])?;
    app.game
        .call("inspect_photo", &[string_val("photo_mara_01")])?;
    let frame = app
        .game
        .call("listen_tape", &[string_val("tape_c17"), float_val(0.8)])?;

    if frame.collected_evidence < 4 {
        bail!(
            "Expected 4 evidence items collected, got {}",
            frame.collected_evidence
        );
    }
    if !frame.can_classify {
        bail!("Expected can_classify = true after evidence collected");
    }
    println!("evidence=4/4");

    // 5. Submit wrong classification
    let frame = app
        .game
        .call("submit_classification", &[string_val("NO_ANOMALY")])?;
    if frame.case_complete {
        bail!("Case should not complete on wrong classification");
    }
    println!("wrong_classification=handled");

    // 6. Submit correct classification (MANDELA-CLASS INTRUSION)
    let frame = app.game.call(
        "submit_classification",
        &[string_val("MANDELA-CLASS INTRUSION")],
    )?;
    if !frame.case_complete || !frame.truth_unlocked {
        bail!("Case failed to complete on correct classification");
    }
    println!("correct_classification=accepted");
    println!("ending=unlocked");

    // 7. Save / Load roundtrip test
    let save = app.game.export_save()?;
    app.game.load_game(&save)?;
    let loaded_frame = app
        .game
        .frame
        .as_ref()
        .context("Missing frame after load")?;
    if !loaded_frame.case_complete {
        bail!("Save state roundtrip lost case_complete status");
    }
    println!("save_roundtrip=ok");

    println!("\nECHO//XP headless OK:");
    println!("boot=ok");
    println!("login=ok");
    println!("evidence=4/4");
    println!("wrong_classification=handled");
    println!("correct_classification=accepted");
    println!("ending=unlocked");
    println!("save_roundtrip=ok");

    Ok(())
}

fn run_capture_screen(stage: &str, output_path: &Path) -> Result<()> {
    println!(
        "Capturing ECHO//XP screen stage '{stage}' to '{:?}'...",
        output_path
    );
    let mut app = App::new()?;

    match stage {
        "boot" => {
            // Already in boot phase
        }
        "login" => {
            app.game.call("complete_boot", &[])?;
        }
        "desktop" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
        }
        "inbox" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
            app.open_app(AppKind::Inbox, "Inbox");
            app.game.call("read_mail", &[string_val("mail_001")])?;
        }
        "case-file" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
            app.open_app(AppKind::CaseFiles, "Case Files");
            app.game
                .call("read_case_file", &[string_val("CASE_001_MARA_V")])?;
        }
        "photo" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
            app.open_app(AppKind::PhotoViewer, "Evidence Photo");
            app.game
                .call("inspect_photo", &[string_val("photo_mara_01")])?;
        }
        "tape" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
            app.open_app(AppKind::TapePlayer, "Tape Player");
            app.game
                .call("listen_tape", &[string_val("tape_c17"), float_val(0.5)])?;
            app.tape_app.play();
            app.tape_app.tick(4.0);
        }
        "classifier" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
            app.game.call("read_mail", &[string_val("mail_001")])?;
            app.game
                .call("read_case_file", &[string_val("CASE_001_MARA_V")])?;
            app.game
                .call("inspect_photo", &[string_val("photo_mara_01")])?;
            app.game
                .call("listen_tape", &[string_val("tape_c17"), float_val(0.8)])?;
            app.open_app(AppKind::Classifier, "Classifier");
        }
        "ending" => {
            app.game.call("complete_boot", &[])?;
            app.game.call("login", &[])?;
            app.game.call("read_mail", &[string_val("mail_001")])?;
            app.game
                .call("read_case_file", &[string_val("CASE_001_MARA_V")])?;
            app.game
                .call("inspect_photo", &[string_val("photo_mara_01")])?;
            app.game
                .call("listen_tape", &[string_val("tape_c17"), float_val(0.8)])?;
            app.game.call(
                "submit_classification",
                &[string_val("MANDELA-CLASS INTRUSION")],
            )?;
            app.open_app(AppKind::OperatorRecord, "OPERATOR RECORD");
        }
        _ => bail!("Unknown stage '{stage}'"),
    }

    app.capture_screen(output_path)?;
    println!("Screen capture saved successfully.");
    Ok(())
}
