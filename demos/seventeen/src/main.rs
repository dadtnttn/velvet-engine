mod assets;
mod audio;
mod input;
mod model;
mod render;
mod save;

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use audio::Audio;
use input::{Controls, GameplayInput, UiInput};
use model::{FrameView, PlayerView, Vec2};
use render::{scale_nearest_letterbox, Renderer, HEIGHT, WIDTH};
use save::{SaveData, SaveStore};
use softbuffer::{Context as SoftContext, Surface};
use velvet_script_vs3::{
    bool_val, compile_bundle, float_val, int, map_val, vec2_val, Vs3Module, Vs3Session,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

const GAME_SOURCES: &[(&str, &str)] = &[
    ("game.vel", include_str!("../data/game.vel")),
    ("state.vel", include_str!("../data/state.vel")),
    ("core.vel", include_str!("../data/core.vel")),
    ("rooms.vel", include_str!("../data/rooms.vel")),
    ("combat.vel", include_str!("../data/combat.vel")),
    ("ai.vel", include_str!("../data/ai.vel")),
    ("interaction.vel", include_str!("../data/interaction.vel")),
    ("lifecycle.vel", include_str!("../data/lifecycle.vel")),
    ("acceptance.vel", include_str!("../data/acceptance.vel")),
];
const FRAME_TIME: Duration = Duration::from_micros(16_667);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Splash,
    Title,
    Help,
    Settings,
    DeleteConfirm,
    Playing,
    Pause,
    Memories,
}

struct Game {
    module: Vs3Module,
    session: Vs3Session,
    frame: Option<FrameView>,
    saved_revision: i64,
}

impl Game {
    fn compile() -> Result<Self> {
        let module = compile_bundle("game.vel", GAME_SOURCES.iter().copied())?;
        let session = module.session()?;
        Ok(Self {
            module,
            session,
            frame: None,
            saved_revision: -1,
        })
    }

    fn reset_session(&mut self) -> Result<()> {
        self.session = self.module.session()?;
        self.frame = None;
        self.saved_revision = -1;
        Ok(())
    }

    fn new_game(&mut self, seed: i64) -> Result<()> {
        self.reset_session()?;
        let value = self.session.call("new_game", &[int(seed)])?;
        self.frame = Some(FrameView::parse(&value)?);
        Ok(())
    }

    fn load_game(&mut self, save: &SaveData) -> Result<()> {
        self.reset_session()?;
        let value = self.session.call("load_game", &[save.to_vs3()])?;
        self.frame = Some(FrameView::parse(&value)?);
        Ok(())
    }

    fn tick(&mut self, dt: f32, input: GameplayInput) -> Result<()> {
        let input = map_val([
            (
                "move".into(),
                vec2_val(input.movement.x as f64, input.movement.y as f64),
            ),
            (
                "aim".into(),
                vec2_val(input.aim.x as f64, input.aim.y as f64),
            ),
            ("fire".into(), bool_val(input.fire)),
            ("dash".into(), bool_val(input.dash)),
            ("interact".into(), bool_val(input.interact)),
            ("reload".into(), bool_val(input.reload)),
            ("weapon".into(), int(input.weapon)),
        ]);
        let value = self.session.call("tick", &[float_val(dt as f64), input])?;
        self.frame = Some(FrameView::parse(&value)?);
        Ok(())
    }

    fn export_save(&mut self) -> Result<SaveData> {
        SaveData::from_vs3(&self.session.call("export_save", &[])?)
    }

    fn acceptance_call(
        &mut self,
        name: &str,
        args: &[velvet_script_vs3::Value],
    ) -> Result<FrameView> {
        let frame = FrameView::parse(&self.session.call(name, args)?)?;
        self.frame = Some(frame.clone());
        Ok(frame)
    }
}

struct App {
    screen: Screen,
    previous_screen: Screen,
    menu_selected: usize,
    pause_selected: usize,
    settings_selected: usize,
    delete_starts_new: bool,
    delete_return: Screen,
    splash_time: f32,
    status: String,
    game: Game,
    saves: SaveStore,
    controls: Controls,
    renderer: Renderer,
    audio: Audio,
    window: Option<Arc<Window>>,
    context: Option<SoftContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last_tick: Instant,
    accumulator: Duration,
}

impl App {
    fn new() -> Result<Self> {
        let saves = SaveStore::load();
        let status = saves.warning().unwrap_or_default().to_string();
        let audio = Audio::new(saves.settings());
        Ok(Self {
            screen: Screen::Splash,
            previous_screen: Screen::Title,
            menu_selected: 0,
            pause_selected: 0,
            settings_selected: 0,
            delete_starts_new: false,
            delete_return: Screen::Title,
            splash_time: 0.0,
            status,
            game: Game::compile()?,
            saves,
            controls: Controls::new(),
            renderer: Renderer::new(),
            audio,
            window: None,
            context: None,
            surface: None,
            last_tick: Instant::now(),
            accumulator: Duration::ZERO,
        })
    }

    fn start_new(&mut self) {
        let seed = chrono_free_seed();
        match self.game.new_game(seed) {
            Ok(()) => {
                self.screen = Screen::Playing;
                self.status.clear();
                self.autosave();
            }
            Err(error) => self.status = format!("No se pudo iniciar: {error:#}"),
        }
    }

    fn continue_game(&mut self) {
        let Some(save) = self.saves.save().cloned() else {
            self.status = "No hay una partida guardada.".into();
            return;
        };
        match self.game.load_game(&save) {
            Ok(()) => {
                self.screen = Screen::Playing;
                self.status.clear();
            }
            Err(error) => self.status = format!("No se pudo continuar: {error:#}"),
        }
    }

    fn autosave(&mut self) {
        let Some(frame) = &self.game.frame else {
            return;
        };
        if frame.save_revision == self.game.saved_revision || frame.phase == "dead" {
            return;
        }
        let revision = frame.save_revision;
        match self
            .game
            .export_save()
            .and_then(|save| self.saves.set_save(save))
        {
            Ok(()) => self.game.saved_revision = revision,
            Err(error) => self.status = format!("No se pudo guardar: {error:#}"),
        }
    }

    fn update(&mut self, event_loop: &ActiveEventLoop, dt: f32) {
        self.controls.poll();
        let ui = self.controls.ui();
        if self.screen != Screen::Playing {
            if ui.up || ui.down || ui.left || ui.right {
                self.audio.play_ui(false, self.saves.settings());
            }
            if ui.confirm {
                self.audio.play_ui(true, self.saves.settings());
            }
        }
        match self.screen {
            Screen::Splash => {
                self.splash_time += dt;
                if self.splash_time >= 1.8 || ui.confirm || ui.cancel {
                    self.screen = Screen::Title;
                }
            }
            Screen::Title => self.update_title(event_loop, ui),
            Screen::Help => {
                if ui.confirm || ui.cancel {
                    self.screen = self.previous_screen;
                }
            }
            Screen::Settings => self.update_settings(ui),
            Screen::DeleteConfirm => {
                if ui.confirm {
                    if let Err(error) = self.saves.clear_save() {
                        self.status = format!("No se pudo borrar: {error:#}");
                    }
                    if self.delete_starts_new {
                        self.start_new();
                    } else {
                        self.status = "Partida borrada.".into();
                        self.screen = self.delete_return;
                    }
                } else if ui.cancel {
                    self.screen = self.delete_return;
                }
            }
            Screen::Playing => self.update_gameplay(ui, dt),
            Screen::Pause => self.update_pause(ui),
            Screen::Memories => {
                if ui.confirm || ui.cancel {
                    self.screen = Screen::Pause;
                }
            }
        }
        self.controls.finish_frame();
    }

    fn update_title(&mut self, event_loop: &ActiveEventLoop, ui: UiInput) {
        if ui.up {
            self.menu_selected = self.menu_selected.saturating_sub(1);
        }
        if ui.down {
            self.menu_selected = (self.menu_selected + 1).min(4);
        }
        if ui.cancel {
            event_loop.exit();
        }
        if !ui.confirm {
            return;
        }
        match self.menu_selected {
            0 if self.saves.save().is_some() => {
                self.delete_starts_new = true;
                self.delete_return = Screen::Title;
                self.screen = Screen::DeleteConfirm;
            }
            0 => self.start_new(),
            1 => self.continue_game(),
            2 => {
                self.previous_screen = Screen::Title;
                self.screen = Screen::Help;
            }
            3 => {
                self.previous_screen = Screen::Title;
                self.settings_selected = 0;
                self.screen = Screen::Settings;
            }
            4 => event_loop.exit(),
            _ => {}
        }
    }

    fn update_settings(&mut self, ui: UiInput) {
        if ui.up {
            self.settings_selected = self.settings_selected.saturating_sub(1);
        }
        if ui.down {
            self.settings_selected = (self.settings_selected + 1).min(9);
        }
        let direction = i32::from(ui.right) - i32::from(ui.left);
        if direction != 0 {
            let settings = self.saves.settings_mut();
            match self.settings_selected {
                0 => settings.master_volume = step_volume(settings.master_volume, direction),
                1 => settings.music_volume = step_volume(settings.music_volume, direction),
                2 => settings.effects_volume = step_volume(settings.effects_volume, direction),
                3 => settings.screen_shake = !settings.screen_shake,
                4 => settings.distortion = !settings.distortion,
                5 => settings.flashes = !settings.flashes,
                6 => settings.high_contrast = !settings.high_contrast,
                7 => {
                    settings.fullscreen = !settings.fullscreen;
                    self.apply_fullscreen();
                }
                _ => {}
            }
            self.audio.update_settings(self.saves.settings());
            let _ = self.saves.flush();
        }
        if ui.confirm && self.settings_selected == 8 {
            self.delete_starts_new = false;
            self.delete_return = Screen::Settings;
            self.screen = Screen::DeleteConfirm;
            return;
        }
        if ui.cancel || ui.confirm && self.settings_selected == 9 {
            let _ = self.saves.flush();
            self.screen = self.previous_screen;
        }
    }

    fn update_gameplay(&mut self, ui: UiInput, dt: f32) {
        let Some(frame) = self.game.frame.as_ref() else {
            self.screen = Screen::Title;
            return;
        };
        if frame.phase == "complete" && ui.confirm {
            self.screen = Screen::Title;
            self.menu_selected = 0;
            return;
        }
        let size = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or(PhysicalSize::new(WIDTH, HEIGHT));
        let input = self.controls.gameplay(&frame.player, size);
        if input.pause {
            self.screen = Screen::Pause;
            self.pause_selected = 0;
            return;
        }
        match self.game.tick(dt, input) {
            Ok(()) => {
                let frame = self.game.frame.as_ref().expect("frame after tick");
                self.audio.play_events(&frame.events, self.saves.settings());
                self.renderer
                    .update(dt, &frame.events, self.saves.settings());
                self.autosave();
            }
            Err(error) => {
                self.status = format!("Error VS3: {error:#}");
                self.screen = Screen::Pause;
            }
        }
    }

    fn update_pause(&mut self, ui: UiInput) {
        if ui.up {
            self.pause_selected = self.pause_selected.saturating_sub(1);
        }
        if ui.down {
            self.pause_selected = (self.pause_selected + 1).min(3);
        }
        if ui.cancel || ui.pause {
            self.screen = Screen::Playing;
            return;
        }
        if !ui.confirm {
            return;
        }
        match self.pause_selected {
            0 => self.screen = Screen::Playing,
            1 => self.screen = Screen::Memories,
            2 => {
                self.previous_screen = Screen::Pause;
                self.settings_selected = 0;
                self.screen = Screen::Settings;
            }
            3 => {
                self.autosave();
                self.screen = Screen::Title;
                self.menu_selected = 0;
            }
            _ => {}
        }
    }

    fn apply_fullscreen(&self) {
        if let Some(window) = &self.window {
            window.set_fullscreen(if self.saves.settings().fullscreen {
                Some(Fullscreen::Borderless(None))
            } else {
                None
            });
        }
    }

    fn toggle_fullscreen(&mut self) {
        let fullscreen = !self.saves.settings().fullscreen;
        self.saves.settings_mut().fullscreen = fullscreen;
        self.apply_fullscreen();
        let _ = self.saves.flush();
    }

    fn paint(&mut self) {
        match self.screen {
            Screen::Splash => self
                .renderer
                .paint_splash((self.splash_time / 1.8).clamp(0.0, 1.0)),
            Screen::Title => self.renderer.paint_title(
                self.menu_selected,
                self.saves.save().is_some(),
                &self.status,
                self.audio.is_available(),
            ),
            Screen::Help => self.renderer.paint_help(),
            Screen::Settings => self
                .renderer
                .paint_settings(self.settings_selected, self.saves.settings()),
            Screen::DeleteConfirm => self.renderer.paint_delete_confirm(self.delete_starts_new),
            Screen::Playing => {
                if let Some(frame) = &self.game.frame {
                    self.renderer.paint_game(frame, self.saves.settings());
                }
            }
            Screen::Pause => {
                if let Some(frame) = &self.game.frame {
                    self.renderer.paint_game(frame, self.saves.settings());
                    self.renderer.paint_pause(self.pause_selected);
                }
            }
            Screen::Memories => {
                if let Some(frame) = &self.game.frame {
                    self.renderer.paint_memories(frame);
                }
            }
        }
        self.present();
    }

    fn present(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let present = scale_nearest_letterbox(&self.renderer.pixels, width, height);
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        if surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .is_err()
        {
            return;
        }
        if let Ok(mut buffer) = surface.buffer_mut() {
            buffer.copy_from_slice(&present);
            let _ = buffer.present();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("17 — VelvetEngine + VS3")
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
            .with_min_inner_size(LogicalSize::new(640, 360));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("create 17 window"),
        );
        let context = SoftContext::new(window.clone()).expect("create softbuffer context");
        let surface = Surface::new(&context, window.clone()).expect("create softbuffer surface");
        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
        self.last_tick = Instant::now();
        self.apply_fullscreen();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.paint(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(WinitKeyCode::F11)
                {
                    self.toggle_fullscreen();
                } else {
                    self.controls.keyboard(event.physical_key, event.state);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.controls.mouse_button(button, state)
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.controls.cursor(position.x, position.y)
            }
            WindowEvent::Focused(false) if self.screen == Screen::Playing => {
                self.screen = Screen::Pause;
                self.pause_selected = 0;
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_tick)
            .min(Duration::from_millis(100));
        self.last_tick = now;
        self.accumulator += elapsed;
        while self.accumulator >= FRAME_TIME {
            self.update(event_loop, FRAME_TIME.as_secs_f32());
            self.accumulator -= FRAME_TIME;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + FRAME_TIME));
    }
}

fn step_volume(value: f32, direction: i32) -> f32 {
    (value + direction as f32 * 0.05).clamp(0.0, 1.0)
}

fn chrono_free_seed() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| (duration.as_nanos() as u64 ^ 17) as i64)
        .unwrap_or(17)
}

fn run_headless(capture: Option<&Path>) -> Result<()> {
    let mut game = Game::compile()?;
    game.new_game(17)?;
    let intro = game.frame.as_ref().context("new game frame")?;
    if intro.phase != "intro" || intro.room != 1 {
        bail!("new_game did not enter the intro");
    }

    let dead = game.acceptance_call("verify_force_death", &[])?;
    if dead.phase != "dead" || dead.deaths != 1 {
        bail!("death/resurrection contract failed");
    }
    for _ in 0..30 {
        game.tick(0.05, GameplayInput::default())?;
    }
    if game.frame.as_ref().context("respawn frame")?.phase != "playing" {
        bail!("resurrection did not complete");
    }

    for room in 1..=5 {
        let frame = game.acceptance_call("verify_enter_room", &[int(room)])?;
        if frame.room != room {
            bail!("room {room} did not load");
        }
        for _ in 0..12 {
            game.tick(0.05, GameplayInput::default())?;
        }
    }
    game.acceptance_call("verify_collect_all_memories", &[])?;
    let ending_b = game.acceptance_call("verify_choose", &[bool_val(true)])?;
    if ending_b.phase != "ending_b" || ending_b.memory_count != 3 {
        bail!("memory ending failed");
    }

    game.new_game(17)?;
    game.acceptance_call("verify_enter_room", &[int(5)])?;
    let ending_a = game.acceptance_call("verify_choose", &[bool_val(false)])?;
    if ending_a.phase != "ending_a" {
        bail!("break-cycle ending failed");
    }

    let save = game.export_save()?;
    let round_trip = SaveData::from_vs3(&save.to_vs3())?;
    if round_trip.room != 5 {
        bail!("save round trip failed");
    }

    if let Some(path) = capture {
        game.acceptance_call("verify_collect_all_memories", &[])?;
        game.acceptance_call("verify_enter_room", &[int(4)])?;
        let frame = game.frame.as_ref().context("capture frame")?;
        let mut renderer = Renderer::new();
        renderer.paint_game(frame, &Default::default());
        let capture = scale_nearest_letterbox(&renderer.pixels, WIDTH, HEIGHT);
        write_capture(path, &capture)?;
    }

    println!(
        "17 headless OK: VS3 rooms=5 death=respawn memories=3 endings=2 save=ok instructions={}",
        game.session.instructions()
    );
    Ok(())
}

fn write_capture(path: &Path, pixels: &[u32]) -> Result<()> {
    let mut bytes = Vec::with_capacity((WIDTH * HEIGHT * 3) as usize);
    for pixel in pixels {
        bytes.push(((pixel >> 16) & 255) as u8);
        bytes.push(((pixel >> 8) & 255) as u8);
        bytes.push((pixel & 255) as u8);
    }
    image::save_buffer(path, &bytes, WIDTH, HEIGHT, image::ColorType::Rgb8)
        .with_context(|| format!("write capture {}", path.display()))
}

fn capture_argument() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|pair| pair[0] == "--capture")
        .map(|pair| PathBuf::from(&pair[1]))
}

fn title_capture_argument() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|pair| pair[0] == "--capture-title")
        .map(|pair| PathBuf::from(&pair[1]))
}

fn screen_capture_argument() -> Option<(String, PathBuf)> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(3)
        .find(|parts| parts[0] == "--capture-screen")
        .map(|parts| (parts[1].clone(), PathBuf::from(&parts[2])))
}

fn run_screen_capture(screen: &str, path: &Path) -> Result<()> {
    let mut renderer = Renderer::new();
    let settings = Default::default();
    let mut frame = FrameView {
        room: 4,
        room_name: "PURGA".into(),
        player: PlayerView {
            pos: Vec2 { x: 155.0, y: 205.0 },
            aim: Vec2 { x: 1.0, y: 0.0 },
            hp: 52.0,
            max_hp: 100.0,
            weapon: "blade".into(),
            ..Default::default()
        },
        memory_count: 3,
        memories: [true, true, true],
        deaths: 4,
        score: 3500,
        magazine: 7,
        phase: "playing".into(),
        ..Default::default()
    };

    match screen {
        "splash" => renderer.paint_splash(0.72),
        "title" => renderer.paint_title(0, true, "PARTIDA RECUPERADA", true),
        "help" => renderer.paint_help(),
        "settings" => renderer.paint_settings(8, &settings),
        "delete" => renderer.paint_delete_confirm(false),
        "pause" => {
            renderer.paint_game(&frame, &settings);
            renderer.paint_pause(2);
        }
        "memories" => renderer.paint_memories(&frame),
        "intro" => {
            frame.phase = "intro".into();
            renderer.paint_game(&frame, &settings);
        }
        "death" => {
            frame.phase = "dead".into();
            frame.death_timer = 0.7;
            renderer.paint_game(&frame, &settings);
        }
        "ending-a" => {
            frame.phase = "ending_a".into();
            frame.ending_variant = "liberacion".into();
            renderer.paint_game(&frame, &settings);
        }
        "ending-b" => {
            frame.phase = "ending_b".into();
            frame.ending_variant = "archivo completo".into();
            renderer.paint_game(&frame, &settings);
        }
        "credits" => {
            frame.phase = "credits".into();
            renderer.paint_game(&frame, &settings);
        }
        "complete" => {
            frame.phase = "complete".into();
            renderer.paint_game(&frame, &settings);
        }
        _ => bail!("unknown capture screen: {screen}"),
    }
    let capture = scale_nearest_letterbox(&renderer.pixels, WIDTH, HEIGHT);
    write_capture(path, &capture)
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("seventeen=info,info");
    if let Some((screen, path)) = screen_capture_argument() {
        return run_screen_capture(&screen, &path);
    }
    if let Some(path) = title_capture_argument() {
        let mut renderer = Renderer::new();
        renderer.paint_title(0, true, "", true);
        let capture = scale_nearest_letterbox(&renderer.pixels, WIDTH, HEIGHT);
        return write_capture(&path, &capture);
    }
    let capture = capture_argument();
    if std::env::args().any(|argument| argument == "--headless") || capture.is_some() {
        return run_headless(capture.as_deref());
    }
    let mut app = App::new()?;
    println!(
        "17 - VS3 gameplay loaded; save={}; {}",
        app.saves.path().display(),
        app.renderer.asset_status()
    );
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_vs3_acceptance_path() {
        run_headless(None).unwrap();
    }
}
