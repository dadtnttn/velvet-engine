//! The window: winit for the window and its events, wgpu to show the canvas (or the stage) letterboxed.

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use velvet_canvas::{Canvas, Point};
use velvet_stage::{SharedRenderer, Stage, StageRenderer};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::Fullscreen;
use winit::window::{Window, WindowId};

use crate::input::{Button, Input, InputEvent};
use crate::viewport::Viewport;
use crate::{Game2D, HostConfig};

/// Opens the window and runs the game until it quits or the window is closed.
pub fn run<G: Game2D + 'static>(config: HostConfig, game: G) -> Result<(), Box<dyn Error>> {
    // The platform's event loop: on Android it is attached to the activity (see mobile.rs).
    let event_loop: EventLoop<()> = velvet_platform::event_loop()?;
    let canvas = Canvas::new(config.logical_width, config.logical_height);
    // QA: `VELVET_CAPTURE_AFTER=120:out.png` saves the canvas after that many frames and closes the window.
    let capture_after = std::env::var("VELVET_CAPTURE_AFTER").ok().and_then(|v| {
        let (frames, path) = v.split_once(':')?;
        Some((
            frames.trim().parse().ok()?,
            std::path::PathBuf::from(path.trim()),
        ))
    });
    let mut app = App {
        config,
        game,
        canvas,
        input: Input::new(),
        mobile: mobile::Mobile::new(),
        running: None,
        last: None,
        behind: 0.0,
        error: None,
        frames: 0,
        capture_after,
        modifiers: ModifiersState::empty(),
    };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

struct App<G: Game2D> {
    config: HostConfig,
    game: G,
    canvas: Canvas,
    input: Input,
    /// Touch as the pointer, the back button, suspend and resume (mobile.rs).
    mobile: mobile::Mobile,
    running: Option<Running>,
    last: Option<Instant>,
    /// Game time not yet stepped.
    behind: f32,
    error: Option<String>,
    /// Frames shown so far.
    frames: u64,
    /// After how many frames to save the canvas and close (QA).
    capture_after: Option<(u64, std::path::PathBuf)>,
    /// The keyboard's modifiers held now (Alt+Enter goes fullscreen).
    modifiers: ModifiersState,
}

struct Running {
    window: Arc<Window>,
    presenter: Presenter,
    viewport: Option<Viewport>,
    window_size: (u32, u32),
    safe_area: velvet_platform::screen::Insets,
    /// Where the pointer is, in window pixels.
    cursor: Option<(f64, f64)>,
    /// True while the pointer is on the picture.
    on_picture: bool,
    /// Whether the window is the one in use. One that is not takes no pointing: the pointer only crosses it on its
    /// way elsewhere (its place is still kept, for the click that comes back to the window).
    focused: bool,
    /// Whether the system's arrow is shown (the game hides it while it draws its own).
    pointer_shown: bool,
}

impl<G: Game2D> App<G> {
    /// The window has a new size: a new letterbox, and the canvas follows the picture's pixels.
    fn resized(&mut self, width: u32, height: u32) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        running.window_size = (width, height);
        running.presenter.resize(width, height);
        self.update_viewport();
    }

    /// Recomputes the letterbox viewport for the current window size and display-cutout safe area.
    fn update_viewport(&mut self) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        let (width, height) = running.window_size;
        let insets = velvet_platform::screen::safe_area();
        running.safe_area = insets;
        let (sx, sy, sw, sh) = insets.safe_rect(width, height);
        // The logical size follows the safe area's shape as far as the game allows.
        let (lw, lh) = self.config.logical_for(sw, sh);
        running.viewport = Viewport::letterbox_in(sx, sy, sw, sh, lw, lh)
            .map(|v| v.filling_in(sx, sy, sw, sh, self.config.fill_near_shape));
        if (lw, lh) != (self.canvas.width(), self.canvas.height()) {
            let (pw, ph) = (self.canvas.pixel_width(), self.canvas.pixel_height());
            self.canvas = Canvas::with_pixels(lw, lh, pw, ph);
            self.game.logical_size_changed(lw, lh);
        }
        if let Some(v) = running.viewport {
            let (pw, ph) = self.config.canvas_pixels(v.width, v.height);
            if pw != self.canvas.pixel_width() || ph != self.canvas.pixel_height() {
                self.canvas.resize_pixels(pw, ph);
                self.game.pixels_changed(&self.canvas);
            }
            if self.game.stage().is_some() {
                self.game.stage_pixels_changed(v.width, v.height);
            }
        }
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        if !self.mobile.is_visible() {
            return;
        }
        if let Some(running) = self.running.as_ref() {
            if velvet_platform::screen::safe_area() != running.safe_area {
                self.update_viewport();
            }
        }
        self.mobile_poll();
        let now = Instant::now();
        let dt = self
            .last
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        self.last = Some(now);
        // A long stall (a dragged window, a breakpoint) does not make the game race to catch up.
        self.behind += dt.min(0.25);
        let mut steps = 0;
        while self.behind >= self.config.step && steps < 8 {
            self.game.update(&self.input, self.config.step);
            self.input.end_step(self.config.step);
            self.behind -= self.config.step;
            steps += 1;
        }
        if self.game.quit() {
            event_loop.exit();
            return;
        }
        let Some(running) = self.running.as_mut() else {
            return;
        };
        // The game asked for fullscreen or a window (its settings); Android stays immersive.
        if let Some(full) = self.game.take_fullscreen_request() {
            if !cfg!(target_os = "android") && full != running.window.fullscreen().is_some() {
                running
                    .window
                    .set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
            }
            self.game.fullscreen_changed(if cfg!(target_os = "android") {
                true
            } else {
                full
            });
        }
        // The system's arrow hides while the game draws its own pointer, over the picture only (on the bars it is the
        // window's).
        if !cfg!(target_os = "android") {
            let shown = self.game.shows_pointer() || !running.on_picture;
            if shown != running.pointer_shown {
                running.window.set_cursor_visible(shown);
                running.pointer_shown = shown;
            }
        }
        // A minimised window shows nothing: nothing is drawn nor sent to the graphics card.
        let Some(viewport) = running.viewport else {
            return;
        };
        let staged = self.game.stage().is_some();
        if !staged || self.config.overlay {
            self.game.draw(&mut self.canvas);
        }
        let shown = match self.game.stage() {
            Some(stage) => running.presenter.present_stage(
                stage,
                self.config.overlay.then_some(&self.canvas),
                &viewport,
            ),
            None => running.presenter.present(&self.canvas, &viewport),
        };
        if let Err(e) = shown {
            eprintln!("[velvet-host2d] frame not shown: {e}");
        }
        self.game.frame_drawn();
        self.frames += 1;
        if let Some((after, path)) = self.capture_after.clone() {
            if self.frames >= after {
                match self.save_picture(&path) {
                    Ok((w, h)) => eprintln!(
                        "[velvet-host2d] {} after {} frames ({w}×{h} pixels)",
                        path.display(),
                        self.frames
                    ),
                    Err(e) => eprintln!("[velvet-host2d] the capture failed: {e}"),
                }
                event_loop.exit();
            }
        }
    }

    /// Saves what the window shows: the stage (drawn again at the picture's pixels, or those of
    /// VELVET_RENDER_SIZE), or the canvas.
    fn save_picture(&mut self, path: &std::path::Path) -> Result<(u32, u32), String> {
        if self.game.stage().is_none() {
            self.canvas.save_png(path)?;
            return Ok((self.canvas.pixel_width(), self.canvas.pixel_height()));
        }
        let running = self.running.as_mut().ok_or("no window")?;
        let viewport = running.viewport.ok_or("a minimised window")?;
        let (w, h) = self.config.canvas_pixels(viewport.width, viewport.height);
        let stage = self.game.stage().ok_or("no stage")?;
        let pixels = running.presenter.capture_stage(stage, w, h);
        image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8)
            .map_err(|e| e.to_string())?;
        Ok((w, h))
    }

    fn pointer_to(&mut self, x: f64, y: f64) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        running.cursor = Some((x, y));
        if !running.focused {
            return;
        }
        match running.viewport.and_then(|v| v.to_logical(x, y)) {
            Some((lx, ly)) => {
                running.on_picture = true;
                self.input
                    .push(InputEvent::PointerMoved(Point::new(lx, ly)));
            }
            None if running.on_picture => {
                running.on_picture = false;
                self.input.push(InputEvent::PointerLeft);
            }
            None => {}
        }
    }

    fn button(&mut self, state: ElementState, button: MouseButton) {
        let button = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
            _ => return,
        };
        let Some(running) = self.running.as_ref() else {
            return;
        };
        let (Some(viewport), Some((x, y))) = (running.viewport, running.cursor) else {
            return;
        };
        match state {
            // The bars take no input.
            ElementState::Pressed => {
                if let Some((lx, ly)) = viewport.to_logical(x, y) {
                    self.input.push(InputEvent::PointerDown {
                        button,
                        at: Point::new(lx, ly),
                    });
                }
            }
            // A release always arrives, so a drag that left the picture ends.
            ElementState::Released => {
                if self.input.button_down(button) {
                    let (lx, ly) = viewport.to_logical_clamped(x, y);
                    self.input.push(InputEvent::PointerUp {
                        button,
                        at: Point::new(lx, ly),
                    });
                }
            }
        }
    }

    fn screenshot(&mut self) {
        let Some(dir) = self.config.captures_dir.clone() else {
            return;
        };
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let path = dir.join(format!("captura-{stamp}.png"));
        match self.save_picture(&path) {
            Ok(_) => eprintln!("[velvet-host2d] {}", path.display()),
            Err(e) => eprintln!("[velvet-host2d] the screenshot failed: {e}"),
        }
    }
}

impl<G: Game2D> ApplicationHandler for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            // Back from the background (Android): a new surface for the same window.
            self.resume();
            return;
        }
        let mut attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(PhysicalSize::new(
                self.config.window_width,
                self.config.window_height,
            ))
            .with_min_inner_size(PhysicalSize::new(
                self.config.min_width,
                self.config.min_height,
            ))
            .with_maximized(self.config.maximized);
        if !cfg!(target_os = "android") {
            attributes = attributes.with_fullscreen(
                self.config
                    .fullscreen
                    .then_some(Fullscreen::Borderless(None)),
            );
        }
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(format!("the window could not be opened: {e}"));
                event_loop.exit();
                return;
            }
        };
        let mut presenter =
            match pollster::block_on(Presenter::new(window.clone(), self.config.frugal)) {
                Ok(p) => p,
                Err(e) => {
                    self.error = Some(e);
                    event_loop.exit();
                    return;
                }
            };
        if self.game.stage().is_some() {
            self.game.stage_renderer_ready(presenter.stage_renderer());
        }
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        self.running = Some(Running {
            window,
            presenter,
            viewport: None,
            window_size: (size.width, size.height),
            safe_area: velvet_platform::screen::Insets::ZERO,
            cursor: None,
            on_picture: false,
            focused: true,
            pointer_shown: true,
        });
        self.mobile.set_scale_factor(scale_factor);
        self.started();
        self.resized(size.width, size.height);
        self.last = Some(Instant::now());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.mobile_event(event_loop, _id, &event) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resized(size.width, size.height),
            WindowEvent::CursorMoved { position, .. } => self.pointer_to(position.x, position.y),
            WindowEvent::Focused(focused) => {
                if let Some(running) = self.running.as_mut() {
                    running.focused = focused;
                    if !focused && running.on_picture {
                        running.on_picture = false;
                        self.input.push(InputEvent::PointerLeft);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(running) = self.running.as_mut() {
                    running.cursor = None;
                    if running.on_picture {
                        running.on_picture = false;
                        self.input.push(InputEvent::PointerLeft);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => self.button(state, button),
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if lines != 0.0 {
                    self.input.push(InputEvent::Wheel(lines));
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if code == KeyCode::F12 && !event.repeat {
                                self.screenshot();
                            }
                            // F11 or Alt+Enter: fullscreen (borderless, on the window's own screen), or back.
                            let enter = matches!(code, KeyCode::Enter | KeyCode::NumpadEnter);
                            if !event.repeat
                                && (code == KeyCode::F11 || enter && self.modifiers.alt_key())
                            {
                                if let Some(running) = &self.running {
                                    let full = running.window.fullscreen().is_none();
                                    running.window.set_fullscreen(
                                        full.then_some(Fullscreen::Borderless(None)),
                                    );
                                    self.game.fullscreen_changed(full);
                                }
                            }
                            self.input.push(InputEvent::KeyDown(code));
                        }
                        ElementState::Released => self.input.push(InputEvent::KeyUp(code)),
                    }
                }
                if event.state == ElementState::Pressed {
                    if let Some(text) = event
                        .text
                        .as_ref()
                        .filter(|t| t.chars().all(|c| !c.is_control()))
                    {
                        self.input.push(InputEvent::Text(text.to_string()));
                    }
                }
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // A frame at every turn of the loop while visible; never spin while suspended in the background.
        if self.mobile.is_visible() {
            if let Some(running) = &self.running {
                running.window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.suspend();
    }

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {
        self.low_memory();
    }
}

/// Android's side of the window: touch, the back button, the surface's suspend and resume.
mod mobile;

const SHADER: &str = r#"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var filtering: sampler;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var v: Vertex;
    v.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    v.uv = uv;
    return v;
}
fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}
@fragment fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(source, filtering, v.uv).rgb, 1.0);
}
@fragment fn fs_main_srgb(v: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(to_srgb(textureSample(source, filtering, v.uv).rgb), 1.0);
}
@fragment fn fs_overlay(v: Vertex) -> @location(0) vec4<f32> {
    return textureSample(source, filtering, v.uv);
}
"#;

/// Shows a canvas in the window.
struct Presenter {
    /// The window's surface, dropped while Android has the app in the background.
    surface: velvet_platform::gpu::SurfaceSlot,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    source: Option<Source>,
    /// The canvas laid over a stage (premultiplied).
    overlay: wgpu::RenderPipeline,
    /// The stage's renderer for the window (also draws its captures).
    stage: Option<SharedRenderer>,
    /// Format used by the stage renderer (sRGB even when the window surface is linear).
    stage_format: wgpu::TextureFormat,
    /// Render the stage into an offscreen sRGB target and blit to the surface (needed on linear surfaces and OpenGL
    /// backends whose default framebuffer ignores external depth attachments).
    stage_indirect: bool,
    /// Offscreen sRGB target when `stage_indirect` is true.
    stage_target: Option<StageTarget>,
    /// Multisample count supported for the stage's colour and depth targets (4 or 1).
    stage_samples: u32,
    /// See [`crate::HostConfig::frugal`].
    frugal: bool,
}

/// Offscreen sRGB target for a stage before blitting to the window surface.
struct StageTarget {
    view: wgpu::TextureView,
    exact: wgpu::BindGroup,
    size: (u32, u32),
}

/// The canvas's texture, and its bindings with each sampler.
struct Source {
    texture: wgpu::Texture,
    smooth: wgpu::BindGroup,
    exact: wgpu::BindGroup,
    size: (u32, u32),
}

impl Presenter {
    async fn new(window: Arc<Window>, frugal: bool) -> Result<Presenter, String> {
        // As in Demon Queen: DX12 on Windows (Vulkan lost the device on the development machine). A frugal game tries
        // OpenGL first.
        let attempts: &[Option<wgpu::Backends>] = if !cfg!(target_os = "windows") {
            &[None]
        } else if frugal {
            &[Some(wgpu::Backends::GL), Some(wgpu::Backends::DX12)]
        } else {
            &[Some(wgpu::Backends::DX12)]
        };
        let mut failure = String::from("no graphics backend");
        for backends in attempts {
            match Self::open(window.clone(), *backends, frugal).await {
                Ok(presenter) => return Ok(presenter),
                Err(e) => failure = e,
            }
        }
        Err(failure)
    }

    async fn open(
        window: Arc<Window>,
        backends: Option<wgpu::Backends>,
        frugal: bool,
    ) -> Result<Presenter, String> {
        // One backend on Android (Vulkan or OpenGL ES, never both: see instance_descriptor).
        let mut descriptor = velvet_platform::gpu::instance_descriptor();
        if let Some(backends) = backends {
            descriptor.backends = backends;
            descriptor = descriptor.with_env();
        }
        let instance = wgpu::Instance::new(&descriptor);
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("the window's surface: {e}"))?;
        // On Android: Vulkan, else OpenGL ES; elsewhere wgpu's choice, as before.
        let adapter = velvet_platform::gpu::request_adapter(
            &instance,
            &surface,
            wgpu::PowerPreference::default(),
        )
        .await
        .ok_or("no graphics adapter for the window")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("velvet-host2d"),
                    required_features: wgpu::Features::empty(),
                    // `Limits::default()` where the GPU has it, less on phones and OpenGL ES.
                    required_limits: velvet_platform::gpu::supported_limits(&adapter),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .map_err(|e| format!("the graphics device: {e}"))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let info = adapter.get_info();
        let stage_indirect = !format.is_srgb() || info.backend == wgpu::Backend::Gl;
        let stage_format = if stage_indirect {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            format
        };
        // wgpu-hal's GLES backend reports MULTISAMPLE_X4 unconditionally without querying
        // per-format GL_SAMPLES; on Android GLES drivers (including SwiftShader / gfxstream)
        // Depth32Float + sRGB multisample renderbuffers fail with GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE.
        let msaa_ok = !(cfg!(target_os = "android") && info.backend == wgpu::Backend::Gl)
            && adapter
                .get_texture_format_features(stage_format)
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            && adapter
                .get_texture_format_features(wgpu::TextureFormat::Depth32Float)
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4);
        let stage_samples = if msaa_ok { 4 } else { 1 };
        eprintln!(
            "[velvet-host2d] GPU: {} ({:?}, {:?}), format={format:?}, stage={stage_format:?}, msaa={stage_samples}x",
            info.name, info.backend, info.device_type
        );
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-host2d"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-host2d"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-host2d"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let make = |entry: &str, target_format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("velvet-host2d"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let pipeline = make(
            if format.is_srgb() { "fs_main" } else { "fs_main_srgb" },
            format,
            None,
        );
        let overlay = make(
            "fs_overlay",
            stage_format,
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("velvet-host2d"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let linear = sampler(wgpu::FilterMode::Linear);
        let nearest = sampler(wgpu::FilterMode::Nearest);
        Ok(Presenter {
            surface: velvet_platform::gpu::SurfaceSlot::from_surface(instance, window, surface),
            device,
            queue,
            config,
            layout,
            pipeline,
            linear,
            nearest,
            source: None,
            overlay,
            stage: None,
            stage_format,
            stage_indirect,
            stage_target: None,
            stage_samples,
            frugal,
        })
    }

    /// The stage's renderer, made the first time.
    fn stage_renderer(&mut self) -> SharedRenderer {
        let (device, queue, format, samples, frugal) = (
            &self.device,
            &self.queue,
            self.stage_format,
            self.stage_samples,
            self.frugal,
        );
        self.stage
            .get_or_insert_with(|| SharedRenderer {
                device: device.clone(),
                queue: queue.clone(),
                renderer: Rc::new(RefCell::new(if frugal {
                    StageRenderer::frugal(device, queue, format, samples)
                } else {
                    StageRenderer::new(device, queue, format, samples)
                })),
                format,
            })
            .clone()
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// The texture for a canvas of a size, made again when the size changes.
    fn source(&mut self, size: (u32, u32)) -> &Source {
        if self.source.as_ref().map(|s| s.size) != Some(size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("velvet-host2d-canvas"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind = |sampler: &wgpu::Sampler| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("velvet-host2d"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            };
            let smooth = bind(&self.linear);
            let exact = bind(&self.nearest);
            self.source = Some(Source {
                texture,
                smooth,
                exact,
                size,
            });
        }
        self.source.as_ref().expect("made above")
    }

    fn present(&mut self, canvas: &Canvas, viewport: &Viewport) -> Result<(), String> {
        let size = (canvas.pixel_width(), canvas.pixel_height());
        self.source(size);
        let source = self.source.as_ref().expect("made above");
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &source.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            canvas.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.0 * 4),
                rows_per_image: Some(size.1),
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => return Err(e.to_string()),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velvet-host2d"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-host2d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // A canvas the picture's own size is copied as it is; another size is scaled smoothly.
            let same = size == (viewport.width, viewport.height);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, if same { &source.exact } else { &source.smooth }, &[]);
            pass.set_viewport(
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
                0.0,
                1.0,
            );
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        // The upload's staging memory is freed now rather than when the next frame comes.
        let _ = self.device.poll(wgpu::Maintain::Poll);
        Ok(())
    }

    /// Offscreen sRGB target for the stage, made again when the viewport's pixel size changes.
    fn stage_target(&mut self, size: (u32, u32)) -> &StageTarget {
        if self.stage_target.as_ref().map(|t| t.size) != Some(size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("velvet-host2d-stage"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.stage_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let exact = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("velvet-host2d-stage"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.nearest),
                    },
                ],
            });
            self.stage_target = Some(StageTarget { view, exact, size });
        }
        self.stage_target.as_ref().expect("made above")
    }

    /// Shows a stage, and the canvas over it if given.
    fn present_stage(
        &mut self,
        stage: &Stage,
        overlay: Option<&Canvas>,
        viewport: &Viewport,
    ) -> Result<(), String> {
        if let Some(canvas) = overlay {
            let size = (canvas.pixel_width(), canvas.pixel_height());
            self.source(size);
            let source = self.source.as_ref().expect("made above");
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &source.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                canvas.pixels(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.0 * 4),
                    rows_per_image: Some(size.1),
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
            );
        }
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => return Err(e.to_string()),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velvet-host2d stage"),
            });
        let stage_format = self.stage_format;
        let indirect = self.stage_indirect;
        let rect = [
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
        ];
        if indirect {
            self.stage_target((viewport.width, viewport.height));
        }
        let shared = self.stage_renderer();
        let mut renderer = shared.renderer.borrow_mut();
        if indirect {
            let st = self.stage_target.as_ref().expect("made above");
            let stage_rect = [0.0, 0.0, viewport.width as f32, viewport.height as f32];
            renderer.render(
                &self.device,
                &self.queue,
                &mut encoder,
                &st.view,
                stage_format,
                (viewport.width, viewport.height),
                stage_rect,
                stage,
            );
            if let (Some(canvas), Some(source)) = (overlay, self.source.as_ref()) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("velvet-host2d overlay"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &st.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let same = (canvas.pixel_width(), canvas.pixel_height())
                    == (viewport.width, viewport.height);
                pass.set_pipeline(&self.overlay);
                pass.set_bind_group(0, if same { &source.exact } else { &source.smooth }, &[]);
                pass.set_viewport(stage_rect[0], stage_rect[1], stage_rect[2], stage_rect[3], 0.0, 1.0);
                pass.draw(0..3, 0..1);
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-host2d blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &st.exact, &[]);
            pass.set_viewport(rect[0], rect[1], rect[2], rect[3], 0.0, 1.0);
            pass.draw(0..3, 0..1);
        } else {
            renderer.render(
                &self.device,
                &self.queue,
                &mut encoder,
                &view,
                stage_format,
                (self.config.width, self.config.height),
                rect,
                stage,
            );
            if let (Some(canvas), Some(source)) = (overlay, self.source.as_ref()) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("velvet-host2d overlay"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let same = (canvas.pixel_width(), canvas.pixel_height())
                    == (viewport.width, viewport.height);
                pass.set_pipeline(&self.overlay);
                pass.set_bind_group(0, if same { &source.exact } else { &source.smooth }, &[]);
                pass.set_viewport(rect[0], rect[1], rect[2], rect[3], 0.0, 1.0);
                pass.draw(0..3, 0..1);
            }
        }
        drop(renderer);
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        let _ = self.device.poll(wgpu::Maintain::Poll);
        Ok(())
    }

    /// The stage drawn again at a size and read back (sRGB RGBA), for screenshots and captures.
    fn capture_stage(&mut self, stage: &Stage, width: u32, height: u32) -> Vec<u8> {
        self.stage_renderer().render_rgba(stage, width, height)
    }
}
