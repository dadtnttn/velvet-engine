use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use softbuffer::{Context as SoftContext, Surface};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window};

use crate::assets::{AppManifest, GameAssets};
use crate::graphics::{
    scale_letterboxed, Canvas, FontSystem, SvgCache, MAX_SVG_BYTES, MAX_SVG_SIDE,
};
use crate::input::InputState;
use crate::model::{FrameView, RuntimeEvent};
use crate::scene::{action_at, render_scene};
use crate::script::ScriptRuntime;
use crate::storage::SaveStore;

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
        let frame = app
            .runtime
            .action(action)
            .and_then(|value| FrameView::parse(&value))?;
        app.accept_checked(frame)?;
    }
    for _ in 0..frames {
        let frame = app
            .runtime
            .update(1.0 / 60.0, Default::default())
            .and_then(|value| FrameView::parse(&value))?;
        app.accept_checked(frame)?;
    }
    app.render();
    save_canvas(&app.canvas, output.as_ref())?;
    app.runtime.shutdown()?;
    Ok(())
}
fn run_headless(path: impl AsRef<Path>, frames: u64) -> Result<()> {
    let mut runtime = ScriptRuntime::load(path)?;
    let manifest = runtime.manifest().clone();
    let fonts = FontSystem::load(&manifest.asset_root(runtime.package_root()), &manifest)?;
    runtime.provide_fonts(fonts.metrics_value())?;
    let mut frame = FrameView::parse(&runtime.initialize()?)?;
    for _ in 0..frames {
        anyhow::ensure!(
            frame.commands.len() <= manifest.limits.max_commands_per_frame,
            "frame exceeds command limit"
        );
        frame = FrameView::parse(&runtime.update(1.0 / 60.0, Default::default())?)?;
    }
    runtime.shutdown()?;
    println!(
        "VS3 APP HEADLESS OK frames={frames} commands={}",
        frame.commands.len()
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
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) context: Option<SoftContext<Arc<Window>>>,
    pub(crate) surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    pub(crate) last_frame: Instant,
    pub(crate) cursor: Option<[f32; 2]>,
    pub(crate) mouse_down: bool,
    pub(crate) exit_requested: bool,
    pub(crate) frame_count: u64,
    pub(crate) saves: SaveStore,
    storage_depth: u8,
}
impl Vs3App {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_saves(path, SaveLocation::Default)
    }
    pub fn with_saves(path: impl AsRef<Path>, location: SaveLocation) -> Result<Self> {
        let mut runtime = ScriptRuntime::load(path)?;
        let manifest = runtime.manifest().clone();
        manifest.require_capability("window")?;
        let saves = match location {
            SaveLocation::Default => SaveStore::for_app(&manifest, None),
            SaveLocation::Dir(dir) => SaveStore::for_app(&manifest, Some(dir)),
            SaveLocation::Memory => SaveStore::memory(),
        };
        let asset_root = manifest.asset_root(runtime.package_root());
        let assets = GameAssets::load(&asset_root, &manifest)?;
        let fonts = FontSystem::load(&asset_root, &manifest)?;
        runtime.provide_fonts(fonts.metrics_value())?;
        let frame = FrameView::parse(&runtime.initialize()?).context("initial VS3 frame")?;
        validate_frame(&frame, &manifest, &assets)?;
        let canvas = Canvas::new(manifest.app.canvas_width, manifest.app.canvas_height);
        let svg = SvgCache::new(fonts.font_data());
        let mut out = Self {
            runtime,
            frame,
            input: Default::default(),
            manifest,
            assets,
            fonts,
            svg,
            canvas,
            window: None,
            context: None,
            surface: None,
            last_frame: Instant::now(),
            cursor: None,
            mouse_down: false,
            exit_requested: false,
            frame_count: 0,
            saves,
            storage_depth: 0,
        };
        out.process_events();
        Ok(out)
    }
    pub(crate) fn update(&mut self, dt: f32) {
        let input = self.input.take_frame();
        match self
            .runtime
            .update(dt.clamp(0.0, 0.1) as f64, input)
            .and_then(|v| FrameView::parse(&v))
        {
            Ok(frame) => self.accept(frame),
            Err(e) => eprintln!("[velvet-vs3-app] update failed: {e:#}"),
        }
    }
    pub(crate) fn render(&mut self) {
        render_scene(
            &mut self.canvas,
            &self.fonts,
            &mut self.svg,
            &self.assets,
            &self.frame,
            self.cursor,
            self.mouse_down,
        )
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
        scale_letterboxed(
            &self.canvas.pixels,
            self.canvas.width,
            self.canvas.height,
            &mut buffer,
            size.width as usize,
            size.height as usize,
        );
        let _ = buffer.present();
    }
    pub(crate) fn click(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some(action) = action_at(&self.frame.commands, cursor) else {
            return;
        };
        match self
            .runtime
            .action(&action)
            .and_then(|v| FrameView::parse(&v))
        {
            Ok(frame) => self.accept(frame),
            Err(e) => eprintln!("[velvet-vs3-app] action `{action}` failed: {e:#}"),
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
            "window.set_title" => {
                if let (Some(window), Some(title)) = (&self.window, payload_string(event, "title"))
                {
                    window.set_title(&title)
                }
            }
            "window.set_size" => {
                if let (Some(window), Some(w), Some(h)) = (
                    &self.window,
                    payload_i64(event, "width"),
                    payload_i64(event, "height"),
                ) {
                    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(
                        w.max(1) as u32,
                        h.max(1) as u32,
                    ));
                }
            }
            "window.set_fullscreen" => {
                if let (Some(window), Some(on)) = (&self.window, payload_bool(event, "enabled")) {
                    window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
                }
            }
            _ => {}
        }
    }
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
        let frame = app
            .runtime
            .action(action)
            .and_then(|value| FrameView::parse(&value))
            .with_context(|| format!("action `{action}`"))?;
        app.accept_checked(frame)?;
    }
    for _ in 0..frames {
        let frame = app
            .runtime
            .update(1.0 / 60.0, Default::default())
            .and_then(|value| FrameView::parse(&value))?;
        app.accept_checked(frame)?;
    }
    let report = app.runtime.profile_report();
    app.runtime.shutdown()?;
    Ok(report)
}
