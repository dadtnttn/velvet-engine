use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use softbuffer::{Context as SoftContext, Surface};
use winit::dpi::LogicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window};

use crate::assets::{AppManifest, GameAssets};
use crate::graphics::{
    blit_letterboxed, render_scale, scale_letterboxed, Canvas, FontSystem, Rect, SvgCache,
    MAX_SVG_BYTES, MAX_SVG_SIDE,
};
use crate::input::InputState;
use crate::model::{FrameView, RuntimeEvent};
use crate::scene::{action_at, render_scene, Damage};
use crate::script::InputFrame;
use crate::script::ScriptRuntime;
use crate::storage::SaveStore;
use velvet_script_vs3::TaskStep;

#[derive(Debug, Clone)]
pub struct AppOptions {
    pub headless: bool,
    pub frames: u64,
    /// Save slot directory; `None` uses `VELVET_SAVE_DIR` or the per-user
    /// data directory.
    pub save_dir: Option<std::path::PathBuf>,
}
impl Default for AppOptions {
    fn default() -> Self {
        Self {
            headless: false,
            frames: 60,
            save_dir: None,
        }
    }
}

/// Where an app keeps its save slots.
#[derive(Debug, Clone, Default)]
pub enum SaveLocation {
    /// `VELVET_SAVE_DIR`, else the per-user data directory.
    #[default]
    Default,
    /// An explicit directory.
    Dir(std::path::PathBuf),
    /// Nothing touches the disk (captures, profiling, tests).
    Memory,
}
impl AppOptions {
    pub fn from_env_args() -> Result<Self> {
        let mut out = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--headless" => out.headless = true,
                "--frames" => {
                    out.frames = args.next().context("--frames requires a value")?.parse()?
                }
                "--save-dir" => {
                    out.save_dir = Some(args.next().context("--save-dir requires a value")?.into())
                }
                _ => {}
            }
        }
        Ok(out)
    }
}

pub fn run_package(path: impl AsRef<Path>, options: AppOptions) -> Result<()> {
    if options.headless {
        return run_headless(path, options.frames);
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let location = match options.save_dir {
        Some(dir) => SaveLocation::Dir(dir),
        None => SaveLocation::Default,
    };
    let mut app = Vs3App::with_saves(path, location)?;
    app.manifest.require_capability("window")?;
    event_loop.run_app(&mut app)?;
    app.runtime.shutdown()?;
    Ok(())
}
pub fn validate_package(path: impl AsRef<Path>) -> Result<AppManifest> {
    let mut runtime = ScriptRuntime::load(path)?;
    let manifest = runtime.manifest().clone();
    let asset_root = manifest.asset_root(runtime.package_root());
    let assets = GameAssets::load(&asset_root, &manifest)?;
    assets.validate_all()?;
    let fonts = FontSystem::load(&asset_root, &manifest)?;
    runtime.provide_fonts(fonts.metrics_value())?;
    let frame = FrameView::parse(&runtime.initialize()?)?;
    validate_frame(&frame, &manifest, &assets)?;
    runtime.shutdown()?;
    Ok(manifest)
}

pub fn capture_package(
    path: impl AsRef<Path>,
    output: impl AsRef<Path>,
    frames: u64,
) -> Result<()> {
    capture_package_with_action(path, output, frames, None)
}

pub fn capture_package_with_action(
    path: impl AsRef<Path>,
    output: impl AsRef<Path>,
    frames: u64,
    action: Option<&str>,
) -> Result<()> {
    let mut app = Vs3App::with_saves(path, SaveLocation::Memory)?;
    if let Some(action) = action {
        app.perform_action(action)?;
    }
    for _ in 0..frames {
        app.tick(1.0 / 60.0, Default::default())?;
    }
    app.render();
    save_canvas(&app.canvas, output.as_ref())?;
    app.runtime.shutdown()?;
    Ok(())
}
fn run_headless(path: impl AsRef<Path>, frames: u64) -> Result<()> {
    let mut app = Vs3App::with_saves(path, SaveLocation::Memory)?;
    for _ in 0..frames {
        app.tick(1.0 / 60.0, Default::default())?;
        if app.exit_requested {
            break;
        }
    }
    app.runtime.shutdown()?;
    println!(
        "VS3 APP HEADLESS OK frames={frames} commands={}",
        app.frame.commands.len()
    );
    Ok(())
}

pub struct Vs3App {
    pub(crate) runtime: ScriptRuntime,
    pub(crate) frame: FrameView,
    pub(crate) input: InputState,
    pub(crate) manifest: AppManifest,
    pub(crate) assets: GameAssets,
    pub(crate) fonts: FontSystem,
    pub(crate) svg: SvgCache,
    pub(crate) canvas: Canvas,
    /// What the last frame redrew, so the next one redraws only changes.
    pub(crate) damage: Damage,
    /// Canvas areas redrawn since the window last showed the canvas
    /// (`None`: all of it).
    unpresented: Option<Vec<Rect>>,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) early_window: EarlyWindow,
    pub(crate) context: Option<SoftContext<Arc<Window>>>,
    pub(crate) surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    pub(crate) last_frame: Instant,
    pub(crate) cursor: Option<[f32; 2]>,
    pub(crate) mouse_down: bool,
    pub(crate) exit_requested: bool,
    pub(crate) frame_count: u64,
    pub(crate) saves: SaveStore,
    pub(crate) audio: crate::audio::Audio,
    asset_root: std::path::PathBuf,
    storage_depth: u8,
    /// The running `app_task` is stopped at a `yield` and expects a reply.
    task_waiting: bool,
    /// A click made while a task runs, delivered with its next reply.
    task_action: Option<String>,
    /// Storage results produced while a task held the session.
    deferred_storage: Vec<velvet_script_vs3::Value>,
}
impl Vs3App {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let app = Self::with_saves(path, SaveLocation::Default)?;
        app.manifest.require_capability("window")?;
        Ok(app)
    }
    /// Load an app without opening a window (`window` is checked by the
    /// windowed entry points only).
    pub fn with_saves(path: impl AsRef<Path>, location: SaveLocation) -> Result<Self> {
        let mut runtime = ScriptRuntime::load(path)?;
        let manifest = runtime.manifest().clone();
        let saves = match location {
            SaveLocation::Default => SaveStore::for_app(&manifest, None),
            SaveLocation::Dir(dir) => SaveStore::for_app(&manifest, Some(dir)),
            SaveLocation::Memory => SaveStore::memory(),
        };
        let asset_root = manifest.asset_root(runtime.package_root());
        let assets = GameAssets::load(&asset_root, &manifest)?;
        let fonts = FontSystem::load(&asset_root, &manifest)?;
        runtime.provide_fonts(fonts.metrics_value())?;
        velvet_script_vs3::i18n::set_measurer(Some(fonts.measurer()));
        velvet_script_vs3::i18n::set_catalog(None);
        velvet_script_vs3::i18n::set_system_lang(&crate::locale::system_language());
        let audio = crate::audio::Audio::load(&asset_root, &manifest.sounds);
        let frame = FrameView::parse(&runtime.initialize()?).context("initial VS3 frame")?;
        validate_frame(&frame, &manifest, &assets)?;
        let canvas = Canvas::new(manifest.app.canvas_width, manifest.app.canvas_height);
        let mut svg = SvgCache::new(fonts.font_data());
        svg.set_fallbacks(&fonts.chain());
        let mut out = Self {
            runtime,
            frame,
            input: Default::default(),
            manifest,
            assets,
            fonts,
            svg,
            canvas,
            damage: Damage::default(),
            unpresented: None,
            window: None,
            early_window: EarlyWindow::default(),
            context: None,
            surface: None,
            last_frame: Instant::now(),
            cursor: None,
            mouse_down: false,
            exit_requested: false,
            frame_count: 0,
            saves,
            audio,
            asset_root,
            storage_depth: 0,
            task_waiting: false,
            task_action: None,
            deferred_storage: Vec::new(),
        };
        out.process_events();
        Ok(out)
    }
    /// Switch the UI language: translation catalog `i18n/<lang>.json` (if
    /// present), fallback fonts for its script and exact text measuring.
    pub fn set_language(&mut self, lang: &str) {
        let path = self.asset_root.join("i18n").join(format!("{lang}.json"));
        let catalog = match std::fs::read_to_string(&path) {
            Ok(text) => match velvet_script_vs3::json_decode(&text) {
                Ok(value) => {
                    let msgs = value
                        .map_get("msgs")
                        .ok()
                        .flatten()
                        .and_then(|m| m.map_entries().ok())
                        .unwrap_or_default();
                    let entries = msgs
                        .into_iter()
                        .filter_map(|(k, v)| v.as_str().map(|t| (k, t.to_owned())));
                    Some(velvet_script_vs3::i18n::Catalog::new(lang, entries))
                }
                Err(e) => {
                    eprintln!("[velvet-vs3-app] {}: {e}", path.display());
                    None
                }
            },
            Err(_) => None,
        };
        let catalog =
            catalog.unwrap_or_else(|| velvet_script_vs3::i18n::Catalog::new(lang, Vec::new()));
        velvet_script_vs3::i18n::set_catalog(Some(catalog));
        self.fonts.set_language(lang);
        self.svg.set_fallbacks(&self.fonts.chain());
        self.damage.invalidate();
        velvet_script_vs3::i18n::set_measurer(Some(self.fonts.measurer()));
    }
    pub(crate) fn update(&mut self, dt: f32) {
        let input = self.input.take_frame();
        if let Err(e) = self.tick(dt.clamp(0.0, 0.1) as f64, input) {
            eprintln!("[velvet-vs3-app] update failed: {e:#}");
        }
    }
    /// One frame: `app_update`, or one slice of the running `app_task`.
    pub fn tick(&mut self, dt: f64, input: InputFrame) -> Result<()> {
        if self.runtime.task_active() {
            return self.run_task_slice(dt, input);
        }
        let frame = FrameView::parse(&self.runtime.update(dt, input)?)?;
        self.accept_checked(frame)
    }
    /// A declarative action (click). While a task runs, it is handed to the
    /// task with its next `yield` reply instead of calling `app_action`.
    pub fn perform_action(&mut self, action: &str) -> Result<()> {
        if self.runtime.task_active() {
            self.task_action = Some(action.to_owned());
            return Ok(());
        }
        let frame = FrameView::parse(&self.runtime.action(action)?)?;
        self.accept_checked(frame)
    }
    fn run_task_slice(&mut self, dt: f64, input: InputFrame) -> Result<()> {
        use velvet_script_vs3::{float_val, map_val, string_val, Value};
        let reply = if self.task_waiting {
            let action = self.task_action.take().map_or(Value::Null, string_val);
            Some(map_val([
                ("dt".to_string(), float_val(dt)),
                ("input".to_string(), input.into_value()),
                ("action".to_string(), action),
            ]))
        } else {
            None
        };
        let step = match self.runtime.resume_task(reply) {
            Ok(step) => step,
            Err(error) => {
                self.task_waiting = false;
                self.task_action = None;
                self.flush_deferred_storage();
                return Err(error);
            }
        };
        match step {
            TaskStep::Yielded(value) => {
                self.task_waiting = true;
                if is_frame_packet(&value) {
                    self.accept_checked(FrameView::parse(&value)?)?;
                }
            }
            TaskStep::Paused => self.task_waiting = false,
            TaskStep::Complete(value) => {
                self.task_waiting = false;
                self.task_action = None;
                if is_frame_packet(&value) {
                    self.accept_checked(FrameView::parse(&value)?)?;
                }
                self.flush_deferred_storage();
            }
        }
        Ok(())
    }
    /// Draw the frame at the window's real resolution (the app keeps its
    /// logical canvas size; text and SVG are rasterized at the window scale,
    /// so nothing is stretched). Without a window, at the logical size.
    pub(crate) fn render(&mut self) {
        let (lw, lh) = (
            self.manifest.app.canvas_width,
            self.manifest.app.canvas_height,
        );
        let scale = match &self.window {
            Some(window) => {
                let size = window.inner_size();
                render_scale(size.width as usize, size.height as usize, lw, lh)
            }
            None => 1.0,
        };
        let w = ((lw as f32 * scale).round() as usize).max(1);
        let h = ((lh as f32 * scale).round() as usize).max(1);
        if self.canvas.width != w || self.canvas.height != h {
            self.canvas = Canvas::new(w, h);
        }
        self.svg.set_scale(scale);
        render_scene(
            &mut self.canvas,
            &self.fonts,
            &mut self.svg,
            &self.assets,
            &self.frame,
            self.cursor,
            self.mouse_down,
            scale,
            &mut self.damage,
        );
        self.unpresented = match (self.unpresented.take(), &self.damage.redrawn) {
            (Some(mut pending), Some(redrawn)) if pending.len() < 64 => {
                pending.extend_from_slice(redrawn);
                Some(pending)
            }
            _ => None,
        };
    }
    pub(crate) fn present(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let (width, height) = (size.width as usize, size.height as usize);
        let (lw, lh) = (
            self.manifest.app.canvas_width,
            self.manifest.app.canvas_height,
        );
        let scale = render_scale(width, height, lw, lh);
        let canvas = &self.canvas;
        let drawn_for_window = canvas.width == (lw as f32 * scale).round() as usize
            && canvas.height == (lh as f32 * scale).round() as usize
            && canvas.width <= width
            && canvas.height <= height;
        // The buffer still shows the last frame: copy only what changed (and
        // everything now and then, in case the window lost its contents).
        let pending = self.unpresented.take();
        let refresh = self.frame_count % 120 == 0;
        match pending {
            Some(regions) if drawn_for_window && buffer.age() == 1 && !refresh => {
                let x0 = (width - canvas.width) / 2;
                let y0 = (height - canvas.height) / 2;
                let mut damage = Vec::with_capacity(regions.len());
                for r in &regions {
                    let (x, y, w, h) = (r.x as usize, r.y as usize, r.w as usize, r.h as usize);
                    for row in y..y + h {
                        let from = row * canvas.width + x;
                        let at = (y0 + row) * width + x0 + x;
                        buffer[at..at + w].copy_from_slice(&canvas.pixels[from..from + w]);
                    }
                    if let (Some(w), Some(h)) =
                        (NonZeroU32::new(r.w as u32), NonZeroU32::new(r.h as u32))
                    {
                        damage.push(softbuffer::Rect {
                            x: (x0 + x) as u32,
                            y: (y0 + y) as u32,
                            width: w,
                            height: h,
                        });
                    }
                }
                // Even with nothing new, presenting validates the window (on
                // Windows an unvalidated window keeps asking to repaint).
                let _ = buffer.present_with_damage(&damage);
            }
            _ => {
                if drawn_for_window {
                    blit_letterboxed(
                        &canvas.pixels,
                        canvas.width,
                        canvas.height,
                        &mut buffer,
                        width,
                        height,
                    );
                } else {
                    // Past the largest render scale (or mid-resize): stretch.
                    scale_letterboxed(
                        &canvas.pixels,
                        canvas.width,
                        canvas.height,
                        &mut buffer,
                        width,
                        height,
                    );
                }
                let _ = buffer.present();
            }
        }
        self.unpresented = Some(Vec::new());
    }
    /// The window may have lost what it showed (uncovered, resized, back
    /// from the taskbar): copy the whole canvas next time.
    pub(crate) fn present_all(&mut self) {
        self.unpresented = None;
    }
    pub(crate) fn click(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some(action) = action_at(&self.frame.commands, cursor) else {
            return;
        };
        if let Err(e) = self.perform_action(&action) {
            eprintln!("[velvet-vs3-app] action `{action}` failed: {e:#}");
        }
    }
    fn accept(&mut self, frame: FrameView) {
        if let Err(error) = self.accept_checked(frame) {
            eprintln!("[velvet-vs3-app] rejected frame: {error:#}");
        }
    }

    fn accept_checked(&mut self, frame: FrameView) -> Result<()> {
        validate_frame(&frame, &self.manifest, &self.assets)?;
        self.frame = frame;
        self.process_events();
        Ok(())
    }
    fn process_events(&mut self) {
        let events = self.frame.events.clone();
        let allowed = self.manifest.capabilities.contains("storage");
        let mut results = Vec::new();
        for event in &events {
            match self.saves.handle(event, allowed) {
                Some(result) => results.push(result),
                None => self.process_event(event),
            }
        }
        for result in results {
            self.deliver_storage(result);
        }
    }
    fn flush_deferred_storage(&mut self) {
        for result in std::mem::take(&mut self.deferred_storage) {
            self.deliver_storage(result);
        }
    }
    /// Answer a storage request through `app_storage`; its frame replaces
    /// the current one (and may itself request storage, up to 8 deep).
    fn deliver_storage(&mut self, result: velvet_script_vs3::Value) {
        // A missing slot on load is normal (first run); other failures are
        // worth a line on stderr even when the app handles them.
        let failed = matches!(
            result.map_get("ok"),
            Ok(Some(velvet_script_vs3::Value::Bool(false)))
        );
        let op = result.map_get("op").ok().flatten();
        let is_load = op.as_ref().and_then(|op| op.as_str()) == Some("load");
        if failed && !is_load {
            if let Ok(Some(error)) = result.map_get("error") {
                eprintln!(
                    "[velvet-vs3-app] storage: {}",
                    error.as_str().unwrap_or_default()
                );
            }
        }
        if self.runtime.task_active() {
            // `app_storage` runs once the task finishes.
            self.deferred_storage.push(result);
            return;
        }
        if self.storage_depth >= 8 {
            eprintln!("[velvet-vs3-app] storage results nested too deeply; dropping one");
            return;
        }
        self.storage_depth += 1;
        match self
            .runtime
            .storage(result)
            .and_then(|value| value.map(|v| FrameView::parse(&v)).transpose())
        {
            Ok(Some(frame)) => self.accept(frame),
            Ok(None) => {}
            Err(e) => eprintln!("[velvet-vs3-app] app_storage failed: {e:#}"),
        }
        self.storage_depth -= 1;
    }
    fn process_event(&mut self, event: &RuntimeEvent) {
        match event.kind.as_str() {
            "app.exit" => self.exit_requested = true,
            "app.task" => {
                // `{payload: {name, arg}}` starts `app_task(name, arg)`; it
                // runs in slices from the next frame on.
                let name = payload_string(event, "name").unwrap_or_default();
                if self.runtime.task_active() {
                    eprintln!(
                        "[velvet-vs3-app] app.task `{name}` ignored: a task is already running"
                    );
                } else {
                    let arg = payload(event, "arg").unwrap_or(velvet_script_vs3::Value::Null);
                    match self.runtime.start_task(&name, arg) {
                        Ok(()) => {
                            self.task_waiting = false;
                            self.task_action = None;
                        }
                        Err(e) => eprintln!("[velvet-vs3-app] {e:#}"),
                    }
                }
            }
            "i18n.set" => {
                let lang = payload_string(event, "lang").unwrap_or_default();
                self.set_language(&lang);
            }
            "i18n.var" => {
                if let (Some(name), Some(value)) = (
                    payload_string(event, "name"),
                    payload_string(event, "value"),
                ) {
                    velvet_script_vs3::i18n::set_var(&name, &value);
                }
            }
            "audio.play" => {
                if let Some(id) = payload_string(event, "sound") {
                    let vol = payload_f32(event, "volume").unwrap_or(1.0);
                    let rate = payload_f32(event, "rate").unwrap_or(1.0);
                    self.audio.play(&id, vol, rate);
                }
            }
            "audio.music" => {
                let id = payload_string(event, "sound").unwrap_or_default();
                let vol = payload_f32(event, "volume").unwrap_or(1.0);
                let fade = payload_f32(event, "fade").unwrap_or(1.0);
                self.audio.music(&id, vol, fade);
            }
            "audio.volume" => {
                self.audio.volume(
                    payload_f32(event, "master"),
                    payload_f32(event, "sfx"),
                    payload_f32(event, "music"),
                );
            }
            "app.profile" => {
                // `{payload: {enabled: true}}` starts counting; `false` prints
                // the report to stderr and stops.
                if payload_bool(event, "enabled") == Some(true) {
                    self.runtime.set_profiling(true);
                } else {
                    for row in self.runtime.profile_report().iter().take(40) {
                        eprintln!(
                            "[profile] {:>12} instr {:>8} calls  {}",
                            row.instructions, row.calls, row.name
                        );
                    }
                    self.runtime.set_profiling(false);
                }
            }
            // Before the window opens (settings loaded at startup) the
            // request is kept and the window opens with it.
            "window.set_title" => {
                if let Some(title) = payload_string(event, "title") {
                    match &self.window {
                        Some(window) => window.set_title(&title),
                        None => self.early_window.title = Some(title),
                    }
                }
            }
            "window.set_size" => {
                if let (Some(w), Some(h)) =
                    (payload_i64(event, "width"), payload_i64(event, "height"))
                {
                    // Logical, like the window it opens with (screen scaling).
                    let size = LogicalSize::new(w.max(1) as u32, h.max(1) as u32);
                    match &self.window {
                        Some(window) => {
                            let _ = window.request_inner_size(size);
                        }
                        None => self.early_window.size = Some(size),
                    }
                }
            }
            "window.set_fullscreen" => {
                if let Some(on) = payload_bool(event, "enabled") {
                    match &self.window {
                        Some(window) => window.set_fullscreen(fullscreen(on)),
                        None => self.early_window.fullscreen = on,
                    }
                }
            }
            _ => {}
        }
    }
}
/// Window requests made before the window exists: the first frames run
/// while the app loads, before the event loop opens it.
#[derive(Debug, Default)]
pub(crate) struct EarlyWindow {
    pub(crate) title: Option<String>,
    pub(crate) size: Option<LogicalSize<u32>>,
    pub(crate) fullscreen: bool,
}
pub(crate) fn fullscreen(on: bool) -> Option<Fullscreen> {
    on.then_some(Fullscreen::Borderless(None))
}
fn is_frame_packet(value: &velvet_script_vs3::Value) -> bool {
    matches!(value.map_get("commands"), Ok(Some(_)))
}
fn payload(event: &RuntimeEvent, key: &str) -> Option<velvet_script_vs3::Value> {
    event.payload.as_ref()?.map_get(key).ok().flatten()
}
fn payload_string(event: &RuntimeEvent, key: &str) -> Option<String> {
    payload(event, key)?.as_str().map(str::to_owned)
}
fn payload_i64(event: &RuntimeEvent, key: &str) -> Option<i64> {
    payload(event, key)?.as_i64()
}
fn payload_f32(event: &RuntimeEvent, key: &str) -> Option<f32> {
    payload(event, key)?.as_f64().map(|v| v as f32)
}
fn payload_bool(event: &RuntimeEvent, key: &str) -> Option<bool> {
    match payload(event, key)? {
        velvet_script_vs3::Value::Bool(v) => Some(v),
        _ => None,
    }
}

fn validate_frame(frame: &FrameView, manifest: &AppManifest, assets: &GameAssets) -> Result<()> {
    anyhow::ensure!(
        frame.commands.len() <= manifest.limits.max_commands_per_frame,
        "frame has {} commands; limit is {}",
        frame.commands.len(),
        manifest.limits.max_commands_per_frame
    );
    let fonts = manifest
        .fonts
        .iter()
        .map(|font| font.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut clips = 0_i32;
    let mut transforms = 0_i32;
    let mut opacities = 0_i32;
    for (index, command) in frame.commands.iter().enumerate() {
        anyhow::ensure!(
            matches!(
                command.op.as_str(),
                "clear"
                    | "rect"
                    | "alpha_rect"
                    | "rounded_rect"
                    | "border"
                    | "circle"
                    | "ellipse"
                    | "alpha_ellipse"
                    | "line"
                    | "sprite"
                    | "sprite_frame"
                    | "tile_fill"
                    | "nine_slice"
                    | "text"
                    | "text_wrap"
                    | "panel"
                    | "bar"
                    | "button"
                    | "keycap"
                    | "svg"
                    | "clip_push"
                    | "clip_pop"
                    | "translate_push"
                    | "transform_pop"
                    | "opacity_push"
                    | "opacity_pop"
            ),
            "command {index} has unknown op `{}`",
            command.op
        );
        match command.op.as_str() {
            "clip_push" => clips += 1,
            "clip_pop" => {
                clips -= 1;
                anyhow::ensure!(clips >= 0, "command {index} pops an empty clip stack");
            }
            "translate_push" => transforms += 1,
            "transform_pop" => {
                transforms -= 1;
                anyhow::ensure!(
                    transforms >= 0,
                    "command {index} pops an empty transform stack"
                );
            }
            "opacity_push" => opacities += 1,
            "opacity_pop" => {
                opacities -= 1;
                anyhow::ensure!(
                    opacities >= 0,
                    "command {index} pops an empty opacity stack"
                );
            }
            _ => {}
        }
        if matches!(
            command.op.as_str(),
            "sprite" | "sprite_frame" | "tile_fill" | "nine_slice"
        ) {
            anyhow::ensure!(
                !command.asset.is_empty() && assets.contains(&command.asset),
                "command {index} references unknown asset `{}`",
                command.asset
            );
        }
        for asset in [
            command.asset.as_str(),
            command.hover_asset.as_str(),
            command.pressed_asset.as_str(),
            command.icon.as_str(),
        ] {
            anyhow::ensure!(
                asset.is_empty() || assets.contains(asset),
                "command {index} references unknown asset `{asset}`"
            );
        }
        if command.op == "svg" {
            anyhow::ensure!(
                !command.text.is_empty() && command.text.len() <= MAX_SVG_BYTES,
                "command {index} svg markup must be 1..={MAX_SVG_BYTES} bytes"
            );
            anyhow::ensure!(
                command.w >= 1.0
                    && command.h >= 1.0
                    && command.w <= MAX_SVG_SIDE as f32
                    && command.h <= MAX_SVG_SIDE as f32,
                "command {index} svg size must be 1..={MAX_SVG_SIDE} pixels per side"
            );
        }
        if matches!(
            command.op.as_str(),
            "text" | "text_wrap" | "button" | "keycap"
        ) {
            anyhow::ensure!(
                fonts.contains(command.font.as_str()),
                "command {index} references unknown font `{}`",
                command.font
            );
        }
    }
    anyhow::ensure!(clips == 0, "frame leaves {clips} clip scope(s) open");
    anyhow::ensure!(
        transforms == 0,
        "frame leaves {transforms} transform scope(s) open"
    );
    anyhow::ensure!(
        opacities == 0,
        "frame leaves {opacities} opacity scope(s) open"
    );
    Ok(())
}

fn save_canvas(canvas: &Canvas, output: &Path) -> Result<()> {
    let mut rgba = Vec::with_capacity(canvas.pixels.len() * 4);
    for pixel in &canvas.pixels {
        rgba.extend_from_slice(&[
            ((pixel >> 16) & 0xff) as u8,
            ((pixel >> 8) & 0xff) as u8,
            (pixel & 0xff) as u8,
            ((pixel >> 24) & 0xff) as u8,
        ]);
    }
    image::save_buffer(
        output,
        &rgba,
        canvas.width as u32,
        canvas.height as u32,
        image::ColorType::Rgba8,
    )
    .with_context(|| format!("could not save capture {}", output.display()))?;
    Ok(())
}

/// Profile an app headlessly: run `app_init`, then each action, then
/// `frames` updates, and return per-function instruction counts (most
/// expensive first). Saves stay in memory.
pub fn profile_package(
    path: impl AsRef<Path>,
    actions: &[String],
    frames: u64,
) -> Result<Vec<velvet_script_vs3::ProfileEntry>> {
    let mut app = Vs3App::with_saves(path, SaveLocation::Memory)?;
    app.runtime.set_profiling(true);
    for action in actions {
        app.perform_action(action)
            .with_context(|| format!("action `{action}`"))?;
        // Let a task started by the action finish before the next one.
        while app.runtime.task_active() {
            app.tick(1.0 / 60.0, Default::default())?;
        }
    }
    for _ in 0..frames {
        app.tick(1.0 / 60.0, Default::default())?;
    }
    let report = app.runtime.profile_report();
    app.runtime.shutdown()?;
    Ok(report)
}
