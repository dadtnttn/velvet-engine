//! Product play: `VnSession` (Say/Choice/…) headless, optional real WindowRunner attempt.
//! BGM intents are applied on the real [`AudioEngine`] / [`MusicPlayer`] path.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use softbuffer::{Context as SoftContext, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use anyhow::{bail, Context, Result};
use velvet_audio::prelude::*;
use velvet_story::prelude::*;
use velvet_story::{
    build_product_ui_frame, paint_product_frame, paint_product_frame_at,
    paint_to_render_descriptors, rasterize_product_paint, ProductPaintCmd, ProductPaintList,
    ProductUiFrame,
};

/// Host audio state: maps story music paths → clips and drives [`MusicPlayer`].
pub struct HostAudio {
    /// Engine.
    pub engine: AudioEngine,
    /// Music controller.
    pub music: MusicPlayer,
    /// Registered path → clip.
    pub clips: HashMap<String, ClipId>,
    /// Last effective music volume applied from prefs.
    pub last_music_volume: f32,
    /// Number of play intents applied.
    pub plays: u32,
    /// Number of stop/fade intents applied.
    pub stops: u32,
}

impl HostAudio {
    /// Create empty host audio (null mixer; no OS device required).
    pub fn new() -> Self {
        Self {
            engine: AudioEngine::new(),
            music: MusicPlayer::new(),
            clips: HashMap::new(),
            last_music_volume: 1.0,
            plays: 0,
            stops: 0,
        }
    }

    /// Ensure a clip exists for `path` (sine placeholder when asset file absent).
    fn clip_for(&mut self, path: &str) -> ClipId {
        if let Some(id) = self.clips.get(path) {
            return *id;
        }
        // Real path used by host: register named clip (file load optional later).
        let clip = AudioClip::sine(path, 220.0, 0.5, 22050);
        let id = self.engine.add_clip(clip);
        self.clips.insert(path.to_string(), id);
        id
    }

    /// Apply one BGM intent with prefs-derived volume.
    pub fn apply_intent(&mut self, intent: BgmIntent, prefs: &StoryPreferences) {
        self.music.volume = prefs.effective_music_volume();
        self.last_music_volume = self.music.volume;
        match intent {
            BgmIntent::Play { path, fade_in } => {
                let id = self.clip_for(&path);
                if fade_in > 0.0 {
                    let _ = self.music.crossfade_to(&mut self.engine, id, Some(fade_in));
                } else {
                    let _ = self.music.play(&mut self.engine, id);
                }
                self.plays += 1;
                println!(
                    "  [bgm-host] play path={path} fade_in={fade_in} volume={:.3} playing={}",
                    self.music.volume,
                    self.music.is_playing()
                );
            }
            BgmIntent::Stop { fade_out } => {
                self.music.stop(&mut self.engine, fade_out);
                self.stops += 1;
                println!(
                    "  [bgm-host] stop fade_out={fade_out} volume={:.3}",
                    self.music.volume
                );
            }
        }
        self.engine.tick(1.0 / 60.0);
        self.music.tick(1.0 / 60.0);
    }

    /// Drain session intents and apply all.
    pub fn apply_session_bgm(&mut self, session: &mut VnSession) {
        let prefs = session.prefs().clone();
        for intent in session.bgm.drain_intents() {
            self.apply_intent(intent, &prefs);
        }
        // Keep controller volume in sync when prefs change without new intents.
        self.music.volume = prefs.effective_music_volume();
        self.last_music_volume = self.music.volume;
    }
}

impl Default for HostAudio {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a `.vel` story through the product [`VnSession`] (Say + Choice + ending).
#[allow(dead_code)]
pub fn cmd_play_story(path: PathBuf, max_steps: u32, choice: Option<usize>) -> Result<()> {
    cmd_play_story_product(path, max_steps, choice, false, "en".into())
}

/// Product play with optional windowed host attempt and language.
pub fn cmd_play_story_product(
    path: PathBuf,
    max_steps: u32,
    choice: Option<usize>,
    windowed: bool,
    lang: String,
) -> Result<()> {
    let prefer = choice.unwrap_or(0);
    let save_dir = path
        .parent()
        .map(|p| p.join("saves"))
        .unwrap_or_else(|| PathBuf::from("saves"));

    // Unified boot: `.vstory` via story-lang pipeline; `.vel` via legacy load.
    let mut session = velvet_story_lang::open_session_from_story_path(&path, Some(save_dir))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if lang != "en" && !lang.is_empty() {
        session
            .set_language(&lang)
            .map_err(|e| anyhow::anyhow!("language `{lang}`: {e}"))?;
        println!(
            "language: {} (available: {})",
            session.language,
            session.available_languages().join(", ")
        );
    }

    let mut host_audio = HostAudio::new();
    // Apply music ops already emitted during session start/pump.
    host_audio.apply_session_bgm(&mut session);

    println!("playing product session {}", path.display());
    if windowed {
        println!("windowed: interactive product host owns playback until the window closes");
        return run_windowed_product_session(session, host_audio, &path);
    }

    let mut steps = 0u32;
    while steps < max_steps {
        steps += 1;
        // Apply any new BGM intents each step on the real audio path.
        host_audio.apply_session_bgm(&mut session);

        match session.player().wait().clone() {
            StoryWait::Ended => break,
            StoryWait::Choice => {
                let opts = session.player().choices();
                if opts.is_empty() {
                    bail!("choice wait but no options");
                }
                let frame = build_product_ui_frame(&session);
                println!(
                    "  [ui] wait=choice choices={} selected={}",
                    frame.choices.len(),
                    frame.selected_choice
                );
                println!(
                    "  [choice screen open={} opts={}]",
                    session.choice.open,
                    opts.len()
                );
                println!("  [choice] {}", frame.choices.join(" | "));
                // silence unused import warning if ProductUiFrame only used in type
                let _: &ProductUiFrame = &frame;
                let idx = prefer.min(opts.len().saturating_sub(1));
                let arm = opts[idx].index;
                session
                    .choose_arm(arm)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Line => {
                session.say.reveal_all();
                let frame = build_product_ui_frame(&session);
                let paint = paint_product_frame(&frame);
                let descs = paint_to_render_descriptors(&paint);
                println!(
                    "  [ui] wait={} font={} w={:.1} h={:.1} langs={}",
                    frame.wait,
                    frame.font.family,
                    frame.body_width,
                    frame.body_height,
                    frame.language_options.join(",")
                );
                println!(
                    "  [gpu-paint] cmds={} descs={} say={} scene={}",
                    paint.len(),
                    descs.len(),
                    paint.has_say_geometry(),
                    paint.scene
                );
                if frame.namebox.is_empty() {
                    println!("  [say] {}", frame.body);
                } else {
                    println!("  [say] {}: {}", frame.namebox, frame.body);
                }
                if let Some(bg) = frame.background.as_deref() {
                    println!("  [bg] {bg}");
                }
                for id in &frame.sprite_ids {
                    println!("  [sprite] {id}");
                }
                if frame.language_menu_visible {
                    println!(
                        "  [lang-menu] active={} options={}",
                        frame.language,
                        frame.language_options.join("|")
                    );
                }
                session.advance();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Ready => {
                session.advance();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Pause { .. } => {
                println!("  [pause] skip");
                session.player_mut().skip_pause();
                session.ingest_events();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Host { token } => {
                println!("  [host] auto-resume token={token}");
                session
                    .player_mut()
                    .resume_host(&token)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                session.ingest_events();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::ScreenCall { id } => {
                println!("  [screen call: {id}] returning null");
                session
                    .return_from_screen(velvet_story::StoryValue::Null)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                host_audio.apply_session_bgm(&mut session);
            }
        }
    }

    // Final BGM drain.
    host_audio.apply_session_bgm(&mut session);
    println!(
        "  [bgm-host] summary plays={} stops={} last_volume={:.3} is_playing={}",
        host_audio.plays,
        host_audio.stops,
        host_audio.last_music_volume,
        host_audio.music.is_playing()
    );

    if session.is_ended() {
        println!("ending reached after {steps} step(s)");
        if let Some(e) = session.ending() {
            println!("ending id: {e}");
        }
        if let Some(h) = session.history_entries().last() {
            if h.text.contains("Ending") {
                println!("{}", h.text);
            }
        }
        let cur = session.player().current_text();
        if cur.contains("Ending") {
            println!("{cur}");
        }
    } else {
        println!(
            "stopped after {steps} step(s) wait={:?}",
            session.player().wait()
        );
    }
    Ok(())
}

/// Play project's entry_scene from velvet.project.
#[allow(dead_code)]
pub fn cmd_play_project(root: PathBuf, max_steps: u32, choice: Option<usize>) -> Result<()> {
    cmd_play_project_opts(root, max_steps, choice, false, "en".into())
}

/// Project play with windowed flag and language.
pub fn cmd_play_project_opts(
    root: PathBuf,
    max_steps: u32,
    choice: Option<usize>,
    windowed: bool,
    lang: String,
) -> Result<()> {
    let proj_path = root.join("velvet.project");
    if !proj_path.exists() {
        bail!("no velvet.project in {}", root.display());
    }
    let text = std::fs::read_to_string(&proj_path)?;
    let project = velvet_project::VelvetProject::from_ron(&text)?;
    let entry = root.join(&project.entry_scene);
    if !entry.exists() {
        let fallback = root.join("scripts/main.vel");
        if fallback.exists() {
            println!(
                "playing project {} (entry_scene missing, fallback scripts/main.vel)",
                root.display()
            );
            return cmd_play_story_product(fallback, max_steps, choice, windowed, lang);
        }
        bail!("entry scene not found: {}", project.entry_scene);
    }
    println!(
        "playing project {} entry {}",
        root.display(),
        project.entry_scene
    );
    cmd_play_story_product(entry, max_steps, choice, windowed, lang)
}

/// Script check then play (re-check + re-play product path).
pub fn cmd_recheck_replay(
    path: PathBuf,
    max_steps: u32,
    choice: Option<usize>,
    lang: String,
) -> Result<()> {
    let story = if path.is_dir() || path.join("velvet.project").exists() {
        let proj_path = path.join("velvet.project");
        let text = std::fs::read_to_string(&proj_path)
            .with_context(|| format!("read {}", proj_path.display()))?;
        let project = velvet_project::VelvetProject::from_ron(&text)?;
        let entry = path.join(&project.entry_scene);
        if entry.exists() {
            entry
        } else {
            path.join("scripts/main.vel")
        }
    } else {
        path.clone()
    };

    println!("recheck: {}", story.display());
    crate::script_cmd::cmd_script_check(story.clone())?;
    println!("replay: product play");
    if path.is_dir() || path.join("velvet.project").exists() {
        cmd_play_project_opts(path, max_steps, choice, false, lang)
    } else {
        cmd_play_story_product(story, max_steps, choice, false, lang)
    }
}

/// Persistent interactive product host for `velvet play --windowed`.
///
/// Unlike the old smoke-test path, this host owns the live [`VnSession`],
/// renders every frame, and routes real pointer/keyboard input back into
/// dialogue, choices, and game-screen `Return(...)` buttons.
fn run_windowed_product_session(
    session: VnSession,
    host_audio: HostAudio,
    story: &Path,
) -> Result<()> {
    if std::env::var_os("VELVET_FORCE_NO_WINDOW").is_some() {
        bail!("VELVET_FORCE_NO_WINDOW set — interactive window disabled");
    }
    let (title, width, height) = project_window_profile(story);
    let event_loop = EventLoop::new().context("create interactive product event loop")?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut host = InteractiveProductHost::new(session, host_audio, title, width, height);
    event_loop
        .run_app(&mut host)
        .context("interactive product event loop")?;
    Ok(())
}

fn project_window_profile(story: &Path) -> (String, u32, u32) {
    for dir in story.ancestors() {
        let manifest = dir.join("velvet.project");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let display = json.get("display");
        let project = json.get("project");
        let title = display
            .and_then(|v| v.get("title"))
            .and_then(|v| v.as_str())
            .or_else(|| project.and_then(|v| v.get("name")).and_then(|v| v.as_str()))
            .unwrap_or("Velvet Engine")
            .to_string();
        let width = display
            .and_then(|v| v.get("width"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1280)
            .clamp(320, 7680) as u32;
        let height = display
            .and_then(|v| v.get("height"))
            .and_then(|v| v.as_u64())
            .unwrap_or(720)
            .clamp(240, 4320) as u32;
        return (title, width, height);
    }
    ("Velvet Engine".into(), 1280, 720)
}

struct InteractiveProductHost {
    session: VnSession,
    host_audio: HostAudio,
    window: Option<Arc<Window>>,
    context: Option<SoftContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    cursor: Option<(f64, f64)>,
    mouse_down: bool,
    last_frame: Instant,
    title: String,
    virtual_w: f32,
    virtual_h: f32,
}

impl InteractiveProductHost {
    fn new(
        session: VnSession,
        host_audio: HostAudio,
        title: String,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            session,
            host_audio,
            window: None,
            context: None,
            surface: None,
            cursor: None,
            mouse_down: false,
            last_frame: Instant::now(),
            title,
            virtual_w: width as f32,
            virtual_h: height as f32,
        }
    }

    fn paint(&self) -> ProductPaintList {
        let frame = build_product_ui_frame(&self.session);
        paint_product_frame_at(&frame, self.virtual_w, self.virtual_h)
    }

    fn render(&mut self) {
        let paint = self.paint();
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        rasterize_product_paint(&paint, &mut buffer, size.width, size.height);
        let _ = buffer.present();
    }

    fn cursor_virtual(&self) -> Option<(f32, f32)> {
        let window = self.window.as_ref()?;
        let (x, y) = self.cursor?;
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return None;
        }
        Some((
            (x as f32) * self.virtual_w / size.width as f32,
            (y as f32) * self.virtual_h / size.height as f32,
        ))
    }

    fn pointer_release(&mut self) {
        let Some((x, y)) = self.cursor_virtual() else {
            return;
        };
        let paint = self.paint();
        if let Some(id) = interactive_hit(&paint, x, y) {
            if self.activate_semantic(&id) {
                return;
            }
        }
        self.advance_or_reveal();
    }

    fn activate_semantic(&mut self, id: &str) -> bool {
        if let Some(index) = id
            .strip_prefix("choice_")
            .and_then(|v| v.parse::<usize>().ok())
        {
            if index < self.session.choice.options.len() {
                self.session.choice.selected = index;
                if let Err(error) = self.session.choose_selected() {
                    eprintln!("[velvet-play] choice activation failed: {error}");
                }
                self.host_audio.apply_session_bgm(&mut self.session);
                return true;
            }
        }
        if let Some(rest) = id.strip_prefix("gscreen_btn_") {
            if let Some((screen, index)) = rest.rsplit_once('_') {
                if let Ok(index) = index.parse::<usize>() {
                    if let Err(error) = self.session.activate_game_screen_button(screen, index) {
                        eprintln!("[velvet-play] screen button `{id}` failed: {error}");
                    }
                    self.host_audio.apply_session_bgm(&mut self.session);
                    return true;
                }
            }
        }
        false
    }

    fn advance_or_reveal(&mut self) {
        if let Some(screen) = self.session.pending_screen_call().map(str::to_string) {
            if let Some(frame) = self.session.resolved_screen_frame(&screen) {
                if !frame.text_complete {
                    self.session.reveal_game_screen(&screen);
                }
            }
            return;
        }
        match self.session.player().wait().clone() {
            StoryWait::Line => {
                if self.session.say.text_complete {
                    self.session.advance();
                } else {
                    self.session.say.reveal_all();
                }
            }
            StoryWait::Ready => self.session.advance(),
            StoryWait::Pause { .. } => {
                self.session.player_mut().skip_pause();
                self.session.ingest_events();
                self.session.sync_ui_from_wait();
            }
            StoryWait::Host { token } => {
                if let Err(error) = self.session.player_mut().resume_host(&token) {
                    eprintln!("[velvet-play] host resume failed: {error}");
                }
                self.session.ingest_events();
                self.session.sync_ui_from_wait();
            }
            StoryWait::Choice | StoryWait::ScreenCall { .. } | StoryWait::Ended => {}
        }
        self.host_audio.apply_session_bgm(&mut self.session);
    }

    fn keyboard(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter | KeyCode::Space => {
                if matches!(self.session.player().wait(), StoryWait::Choice) {
                    if let Err(error) = self.session.choose_selected() {
                        eprintln!("[velvet-play] choice activation failed: {error}");
                    }
                    self.host_audio.apply_session_bgm(&mut self.session);
                } else {
                    self.advance_or_reveal();
                }
            }
            KeyCode::ArrowUp if matches!(self.session.player().wait(), StoryWait::Choice) => {
                self.session.choice.move_sel(-1);
            }
            KeyCode::ArrowDown if matches!(self.session.player().wait(), StoryWait::Choice) => {
                self.session.choice.move_sel(1);
            }
            KeyCode::Backspace => {
                if let Err(error) = self.session.rollback_step() {
                    eprintln!("[velvet-play] rollback failed: {error}");
                }
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for InteractiveProductHost {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(PhysicalSize::new(
                self.virtual_w as u32,
                self.virtual_h as u32,
            ))
            .with_min_inner_size(PhysicalSize::new(360, 640))
            .with_resizable(true);
        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                match SoftContext::new(window.clone()).and_then(|context| {
                    Surface::new(&context, window.clone()).map(|surface| (context, surface))
                }) {
                    Ok((context, surface)) => {
                        self.window = Some(window.clone());
                        self.context = Some(context);
                        self.surface = Some(surface);
                        self.last_frame = Instant::now();
                        window.request_redraw();
                    }
                    Err(error) => {
                        eprintln!("[velvet-play] could not create softbuffer surface: {error}");
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                eprintln!("[velvet-play] could not create window: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => self.mouse_down = true,
                ElementState::Released => {
                    let was_down = self.mouse_down;
                    self.mouse_down = false;
                    if was_down {
                        self.pointer_release();
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
            },
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.keyboard(code);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now
                    .duration_since(self.last_frame)
                    .as_secs_f32()
                    .clamp(0.0, 0.1);
                self.last_frame = now;
                self.session.tick(dt);
                self.host_audio.apply_session_bgm(&mut self.session);
                self.render();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn interactive_hit(paint: &ProductPaintList, x: f32, y: f32) -> Option<String> {
    let mut best: Option<(f32, usize, String)> = None;
    for (order, command) in paint.commands.iter().enumerate() {
        let ProductPaintCmd::Quad {
            id,
            x: left,
            y: top,
            w,
            h,
            z,
            ..
        } = command
        else {
            continue;
        };
        let interactive = id.starts_with("choice_") || id.starts_with("gscreen_btn_");
        if !interactive || *w <= 0.0 || *h <= 0.0 {
            continue;
        }
        if x < *left || y < *top || x > *left + *w || y > *top + *h {
            continue;
        }
        let replace = best
            .as_ref()
            .map(|(best_z, best_order, _)| *z > *best_z || (*z == *best_z && order > *best_order))
            .unwrap_or(true);
        if replace {
            best = Some((*z, order, id.clone()));
        }
    }
    best.map(|(_, _, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn product_play_sample_reaches_ending_line() {
        let dir = tempdir().unwrap();
        let vel = dir.path().join("story.vel");
        std::fs::write(
            &vel,
            r#"
character hero { name: "Hero" }
scene main {
    background "bg.png"
    music "m.ogg" fade_in 0.5
    show hero at center
    hero "Hi"
    choice {
        "Go" { jump end }
    }
}
scene end {
    "Ending: Warm Lights"
}
"#,
        )
        .unwrap();
        cmd_play_story_product(vel, 40, Some(0), false, "en".into()).unwrap();
    }

    #[test]
    fn vn_session_bgm_intents_drive_real_audio_engine() {
        let src = r#"
character hero { name: "Hero" }
scene main {
    music "assets/music/soft.ogg" fade_in 1.0
    hero "Line one."
    "Ending: Done"
}
"#;
        let program = load_program_from_source(src, Some("bgm.vel"), "BGM").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        // Music op is pumped at start → intent or playing state.
        assert!(
            session.bgm.playing || session.bgm.path.is_some() || !session.bgm.intents.is_empty(),
            "expected BGM from music op: path={:?} playing={} intents={:?}",
            session.bgm.path,
            session.bgm.playing,
            session.bgm.intents
        );

        let mut prefs = session.prefs().clone();
        prefs.master_volume = 0.5;
        prefs.music_volume = 0.8;
        session.set_prefs(prefs);

        let mut host = HostAudio::new();
        host.apply_session_bgm(&mut session);
        assert!(
            host.plays >= 1,
            "host must apply at least one play intent, plays={}",
            host.plays
        );
        assert!(
            (host.last_music_volume - 0.4).abs() < 0.01,
            "volume from prefs master*music, got {}",
            host.last_music_volume
        );
        assert!(
            host.music.is_playing(),
            "MusicPlayer must be playing after intent apply"
        );

        // Fade stop path
        host.apply_intent(BgmIntent::Stop { fade_out: 0.1 }, session.prefs());
        assert!(host.stops >= 1);
    }
}
