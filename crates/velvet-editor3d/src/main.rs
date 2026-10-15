//! Velvet Editor: the engine's visual editor for 3D scenes (`.scene3d.json`). A docked layout in
//! the style of Godot (scene tree, inspector, file system, script editor, output) around a
//! viewport drawn by the engine's own renderer, with move / rotate / scale handles.
//!
//! ```text
//! velvet-editor [project] [--scene file] [--open file]...
//! velvet-editor [project] --scene file --capture out.png [--size 1920x1080] [--frames 6]
//!     [--select node/path] [--tool select|move|rotate|scale] [--orbit yaw,pitch[,distance]]
//!     [--frame] [--scale 1.25] [--show-viewport]
//! ```
//!
//! `--capture` renders the whole editor window without opening one, to a PNG (QA and docs).

mod app;
mod code;
mod files;
mod gizmo;
mod scene_ops;
mod theme;
mod viewport;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use gizmo::Tool;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct Args {
    root: PathBuf,
    scene: Option<PathBuf>,
    open: Vec<PathBuf>,
    capture: Option<PathBuf>,
    size: (u32, u32),
    frames: u32,
    select: Option<String>,
    tool: Option<Tool>,
    orbit: Option<(f32, f32, Option<f32>)>,
    frame: bool,
    scale: f32,
    show_viewport: bool,
    demo: Option<PathBuf>,
}

const HELP: &str = "velvet-editor [carpeta del proyecto] [opciones]
  --scene <archivo>        abre esta escena .scene3d.json
  --open <archivo>         abre un script o texto en una pestaña (se puede repetir)
  --capture <salida.png>   dibuja el editor sin ventana y guarda la imagen
  --size <AnchoxAlto>      tamaño de la captura (1920x1080)
  --frames <n>             fotogramas antes de capturar (6)
  --select <ruta/nodo>     selecciona ese nodo
  --tool <select|move|rotate|scale>
  --orbit <yaw,pitch[,dist]>  vista del editor en grados
  --frame                  encuadra la selección
  --scale <factor>         escala de la interfaz en la captura (1.0)
  --show-viewport          deja el visor 3D al frente aunque se abran scripts
  --demo <carpeta>         prueba guiada sin ventana (clics, arrastres, teclas) con capturas";

fn resolve(root: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else if root.join(&p).exists() {
        root.join(p)
    } else {
        std::path::absolute(&p).unwrap_or(p)
    }
}

fn parse_args() -> Result<Args> {
    let mut a = Args {
        root: PathBuf::from("."),
        scene: None,
        open: Vec::new(),
        capture: None,
        size: (1920, 1080),
        frames: 6,
        select: None,
        tool: None,
        orbit: None,
        frame: false,
        scale: 1.0,
        show_viewport: false,
        demo: None,
    };
    let mut it = std::env::args().skip(1);
    let mut root_set = false;
    while let Some(arg) = it.next() {
        let mut val = |name: &str| {
            it.next()
                .with_context(|| format!("{name} necesita un valor"))
        };
        match arg.as_str() {
            "--scene" => a.scene = Some(PathBuf::from(val("--scene")?)),
            "--open" => a.open.push(PathBuf::from(val("--open")?)),
            "--capture" => a.capture = Some(PathBuf::from(val("--capture")?)),
            "--size" => {
                let v = val("--size")?;
                let (w, h) = v
                    .split_once('x')
                    .context("--size AnchoxAlto, p. ej. 1920x1080")?;
                a.size = (w.parse()?, h.parse()?);
            }
            "--frames" => a.frames = val("--frames")?.parse()?,
            "--select" => a.select = Some(val("--select")?),
            "--tool" => {
                a.tool = Some(match val("--tool")?.as_str() {
                    "select" => Tool::Select,
                    "move" => Tool::Move,
                    "rotate" => Tool::Rotate,
                    "scale" => Tool::Scale,
                    other => bail!("herramienta desconocida: {other}"),
                })
            }
            "--orbit" => {
                let v = val("--orbit")?;
                let parts = v
                    .split(',')
                    .map(|p| p.trim().parse::<f32>())
                    .collect::<Result<Vec<_>, _>>()?;
                if parts.len() < 2 {
                    bail!("--orbit yaw,pitch[,distancia]");
                }
                a.orbit = Some((parts[0], parts[1], parts.get(2).copied()));
            }
            "--frame" => a.frame = true,
            "--scale" => a.scale = val("--scale")?.parse()?,
            "--show-viewport" => a.show_viewport = true,
            "--demo" => a.demo = Some(PathBuf::from(val("--demo")?)),
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other if !other.starts_with("--") && !root_set => {
                a.root = PathBuf::from(other);
                root_set = true;
            }
            other => bail!("argumento desconocido: {other}\n\n{HELP}"),
        }
    }
    a.root = std::path::absolute(&a.root).unwrap_or(a.root);
    if !a.root.is_dir() {
        bail!("la carpeta del proyecto no existe: {}", a.root.display());
    }
    a.scene = a.scene.map(|p| resolve(&a.root, p));
    a.open = std::mem::take(&mut a.open)
        .into_iter()
        .map(|p| resolve(&a.root, p))
        .collect();
    a.scale = a.scale.clamp(0.5, 4.0);
    Ok(a)
}

// ------------------------------------------------------------------ GPU
fn instance() -> wgpu::Instance {
    let mut desc = wgpu::InstanceDescriptor::default();
    // Like the game: DX12 on Windows (Vulkan lost the device on the development RTX).
    if cfg!(windows) {
        desc.backends = wgpu::Backends::DX12;
    }
    wgpu::Instance::new(&desc.with_env())
}

fn pick_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter> {
    // `VELVET_ADAPTER=intel` picks the card whose name holds that.
    if let Ok(wanted) = std::env::var("VELVET_ADAPTER") {
        let wanted = wanted.to_lowercase();
        if let Some(a) = instance
            .enumerate_adapters(wgpu::Backends::all())
            .into_iter()
            .find(|a| {
                a.get_info().name.to_lowercase().contains(&wanted)
                    && surface.map_or(true, |s| a.is_surface_supported(s))
            })
        {
            return Ok(a);
        }
    }
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: surface,
        force_fallback_adapter: false,
    }))
    .context("no hay tarjeta gráfica compatible (hace falta DirectX 12)")
}

fn device_for(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue)> {
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("velvet-editor"),
            required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
        },
        None,
    ))
    .context("no se pudo abrir la tarjeta gráfica")?;
    device.on_uncaptured_error(Box::new(|e| eprintln!("[editor] error gráfico: {e}")));
    Ok((device, queue))
}

fn apply_args(editor: &mut app::Editor, a: &Args) {
    if let Some(scene) = &a.scene {
        editor.open_scene(scene);
    }
    for f in &a.open {
        editor.open_script(f);
    }
    if a.show_viewport {
        editor.focus_viewport();
    }
    if let Some(sel) = &a.select {
        editor.select(sel);
    }
    if let Some(t) = a.tool {
        editor.set_tool(t);
    }
    if let Some((yaw, pitch, dist)) = a.orbit {
        editor.set_orbit(yaw, pitch, dist);
    }
}

/// One egui frame drawn into `view` (the window or the capture target).
#[allow(clippy::too_many_arguments)]
fn paint(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut egui_wgpu::Renderer,
    ctx: &egui::Context,
    mut out: egui::FullOutput,
    view: &wgpu::TextureView,
    size: [u32; 2],
) -> egui::FullOutput {
    let ppp = out.pixels_per_point;
    let jobs = ctx.tessellate(std::mem::take(&mut out.shapes), ppp);
    for (id, delta) in &out.textures_delta.set {
        renderer.update_texture(device, queue, *id, delta);
    }
    let sd = egui_wgpu::ScreenDescriptor {
        size_in_pixels: size,
        pixels_per_point: ppp,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("velvet-editor-ui"),
    });
    let cmds = renderer.update_buffers(device, queue, &mut encoder, &jobs, &sd);
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("velvet-editor-ui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.086,
                        g: 0.094,
                        b: 0.118,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let mut pass = pass.forget_lifetime();
        renderer.render(&mut pass, &jobs, &sd);
    }
    queue.submit(cmds.into_iter().chain(std::iter::once(encoder.finish())));
    out
}

// ------------------------------------------------------------------ window
struct Running {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    egui_state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    editor: app::Editor,
    title: String,
}

impl Running {
    /// Draws one frame; how long until the next one is wanted.
    fn frame(&mut self) -> Duration {
        let raw = self.egui_state.take_egui_input(&self.window);
        let ctx = self.egui_state.egui_ctx().clone();
        let mut out = ctx.run(raw, |ctx| self.editor.ui(ctx));
        self.egui_state
            .handle_platform_output(&self.window, std::mem::take(&mut out.platform_output));
        let title = self.editor.title();
        if title != self.title {
            self.window.set_title(&title);
            self.title = title;
        }
        self.editor
            .render(&self.device, &self.queue, &mut self.renderer);
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Duration::ZERO;
            }
            Err(e) => {
                eprintln!("[editor] superficie: {e}");
                return Duration::from_millis(100);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let size = [self.config.width, self.config.height];
        let out = paint(
            &self.device,
            &self.queue,
            &mut self.renderer,
            &ctx,
            out,
            &view,
            size,
        );
        self.window.pre_present_notify();
        frame.present();
        for id in &out.textures_delta.free {
            self.renderer.free_texture(id);
        }
        let delay = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
        if self.editor.wants_repaint() {
            Duration::ZERO
        } else {
            delay
        }
    }
}

struct WinApp {
    args: Args,
    running: Option<Running>,
    error: Option<anyhow::Error>,
    next_repaint: Option<Instant>,
}

impl WinApp {
    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<Running> {
        let attrs = Window::default_attributes()
            .with_title("Velvet Editor")
            .with_inner_size(winit::dpi::LogicalSize::new(1600.0, 940.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(960.0, 600.0))
            .with_maximized(true);
        let window = Arc::new(event_loop.create_window(attrs)?);
        let instance = instance();
        let surface = instance.create_surface(window.clone())?;
        let adapter = pick_adapter(&instance, Some(&surface))?;
        let (device, queue) = device_for(&adapter)?;
        let caps = surface.get_capabilities(&adapter);
        let format =
            egui_wgpu::preferred_framebuffer_format(&caps.formats).unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
        let mut editor = app::Editor::new(&ctx, self.args.root.clone());
        editor.s.gpu_name = adapter.get_info().name;
        apply_args(&mut editor, &self.args);
        window.request_redraw();
        Ok(Running {
            window,
            surface,
            config,
            device,
            queue,
            egui_state,
            renderer,
            editor,
            title: String::new(),
        })
    }
}

impl ApplicationHandler for WinApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }
        match self.start(event_loop) {
            Ok(r) => self.running = Some(r),
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(r) = self.running.as_mut() else {
            return;
        };
        let response = r.egui_state.on_window_event(&r.window, &event);
        match event {
            WindowEvent::CloseRequested => {
                r.editor.request_quit();
                r.window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    r.config.width = size.width;
                    r.config.height = size.height;
                    r.surface.configure(&r.device, &r.config);
                }
                r.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let delay = r.frame();
                if r.editor.should_quit() {
                    event_loop.exit();
                    return;
                }
                self.next_repaint = if delay == Duration::ZERO {
                    r.window.request_redraw();
                    None
                } else {
                    Instant::now().checked_add(delay)
                };
            }
            _ => {}
        }
        if response.repaint {
            if let Some(r) = self.running.as_ref() {
                r.window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match (self.running.as_ref(), self.next_repaint) {
            (Some(r), Some(t)) if Instant::now() >= t => {
                self.next_repaint = None;
                r.window.request_redraw();
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            (Some(_), Some(t)) => event_loop.set_control_flow(ControlFlow::WaitUntil(t)),
            _ => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

// ------------------------------------------------------------------ headless runs
/// The editor drawn without a window: for captures and for the scripted QA run, which feeds it
/// the same pointer and key events a person would.
struct Headless {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: egui_wgpu::Renderer,
    ctx: egui::Context,
    editor: app::Editor,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
    ppp: f32,
    frame: u64,
    modifiers: egui::Modifiers,
}

impl Headless {
    fn new(a: &Args) -> Result<Self> {
        let instance = instance();
        let adapter = pick_adapter(&instance, None)?;
        let (device, queue) = device_for(&adapter)?;
        let (w, h) = a.size;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velvet-editor-capture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let ctx = egui::Context::default();
        let renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
        let mut editor = app::Editor::new(&ctx, a.root.clone());
        editor.s.gpu_name = adapter.get_info().name;
        apply_args(&mut editor, a);
        Ok(Self {
            device,
            queue,
            renderer,
            ctx,
            editor,
            target,
            view,
            size: a.size,
            ppp: a.scale,
            frame: 0,
            modifiers: egui::Modifiers::NONE,
        })
    }

    /// One frame with these input events.
    fn step(&mut self, events: Vec<egui::Event>) {
        let (w, h) = self.size;
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(w as f32 / self.ppp, h as f32 / self.ppp),
            )),
            time: Some(self.frame as f64 / 30.0),
            predicted_dt: 1.0 / 30.0,
            max_texture_side: Some(self.device.limits().max_texture_dimension_2d as usize),
            focused: true,
            modifiers: self.modifiers,
            events,
            ..Default::default()
        };
        raw.viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.ppp);
        let editor = &mut self.editor;
        let out = self.ctx.run(raw, |ctx| editor.ui(ctx));
        self.editor
            .render(&self.device, &self.queue, &mut self.renderer);
        let out = paint(
            &self.device,
            &self.queue,
            &mut self.renderer,
            &self.ctx,
            out,
            &self.view,
            [w, h],
        );
        for id in &out.textures_delta.free {
            self.renderer.free_texture(id);
        }
        self.frame += 1;
    }

    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.step(Vec::new());
        }
    }

    fn pointer(&mut self, p: egui::Pos2) {
        self.step(vec![egui::Event::PointerMoved(p)]);
    }

    fn button(&mut self, p: egui::Pos2, button: egui::PointerButton, pressed: bool) {
        let modifiers = self.modifiers;
        self.step(vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button,
                pressed,
                modifiers,
            },
        ]);
    }

    fn click(&mut self, p: egui::Pos2) {
        self.pointer(p);
        self.button(p, egui::PointerButton::Primary, true);
        self.button(p, egui::PointerButton::Primary, false);
        self.idle(2);
    }

    /// Presses at `from`, moves to `to` in small steps and lets go; `mid` saves a picture while
    /// still held (the live values show then).
    fn drag(&mut self, from: egui::Pos2, to: egui::Pos2, mid: Option<&Path>) -> Result<()> {
        self.pointer(from);
        self.button(from, egui::PointerButton::Primary, true);
        let steps = 14;
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            self.pointer(from + (to - from) * t);
        }
        if let Some(path) = mid {
            self.idle(1);
            self.save(path)?;
        }
        self.button(to, egui::PointerButton::Primary, false);
        self.idle(2);
        Ok(())
    }

    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        let before = self.modifiers;
        self.modifiers = modifiers;
        self.step(vec![event(true)]);
        self.step(vec![event(false)]);
        self.modifiers = before;
        self.idle(1);
    }

    fn text(&mut self, s: &str) {
        self.step(vec![egui::Event::Text(s.to_string())]);
        self.idle(1);
    }

    /// The last frame as a PNG.
    fn save(&mut self, out_path: &Path) -> Result<()> {
        let (w, h) = self.size;
        let bytes_per_row = (w * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-editor-readback"),
            size: (bytes_per_row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velvet-editor-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::Maintain::Wait);
        rx.recv()?.context("no se pudo leer la captura")?;
        let data = slice.get_mapped_range();
        let mut img = image::RgbaImage::new(w, h);
        let row_bytes = (w * 4) as usize;
        for y in 0..h as usize {
            let src = &data[y * bytes_per_row as usize..][..row_bytes];
            img.as_mut()[y * row_bytes..][..row_bytes].copy_from_slice(src);
        }
        drop(data);
        buffer.unmap();
        for p in img.pixels_mut() {
            p.0[3] = 255;
        }
        if let Some(dir) = out_path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        img.save(out_path)
            .with_context(|| format!("no se pudo guardar {}", out_path.display()))?;
        Ok(())
    }
}

fn capture(a: &Args, out_path: &Path) -> Result<()> {
    let started = Instant::now();
    let mut h = Headless::new(a)?;
    for frame in 0..a.frames.max(3) {
        if frame == 2 && a.frame {
            h.editor.frame_selection();
        }
        h.step(Vec::new());
    }
    h.save(out_path)?;
    eprintln!(
        "[editor] captura {} ({}×{}, {:.1} s)",
        out_path.display(),
        a.size.0,
        a.size.1,
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

/// The scripted QA run: clicks, drags and keys on the showcase scene, a picture per step and a
/// check of what each one changed. Needs `--scene` with `personajes/mugo`, `personajes/reina`
/// and `personajes/chiquita` in it.
fn demo(a: &Args, dir: &Path) -> Result<()> {
    use egui::{Key, Modifiers};
    use gizmo::Handle;
    std::fs::create_dir_all(dir)?;
    let mut h = Headless::new(a)?;
    let mut fails = 0;
    let mut check = |ok: bool, what: String| {
        eprintln!("[demo] {} {what}", if ok { "OK  " } else { "FALLA" });
        if !ok {
            fails += 1;
        }
    };
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    let shot = |name: &str| dir.join(name);
    h.editor.focus_viewport();
    h.idle(6);
    h.save(&shot("01_escena.png"))?;

    // Click a character in the viewport.
    let mugo = "personajes/mugo";
    let p = h
        .editor
        .node_screen(mugo)
        .context("Mugo no se ve en el visor")?;
    h.click(p);
    check(
        h.editor.selected().as_deref() == Some(mugo),
        format!(
            "clic en el visor selecciona a Mugo ({:?})",
            h.editor.selected()
        ),
    );
    h.save(&shot("02_seleccion.png"))?;

    // Move along X with the arrow.
    h.key(Key::W, Modifiers::NONE);
    let before = h.editor.transform_of(mugo).context("sin Mugo")?;
    let (p, c) = h
        .editor
        .handle_screen(Handle::Axis(0))
        .context("sin flecha X")?;
    let dir_x = (p - c).normalized();
    h.drag(p, p + dir_x * 130.0, Some(&shot("03_mover.png")))?;
    let moved = h.editor.transform_of(mugo).context("sin Mugo")?;
    check(
        moved.position[0] > before.position[0] + 0.2
            && (moved.position[2] - before.position[2]).abs() < 1e-3,
        format!(
            "la flecha X mueve en X: {:.2} → {:.2}",
            before.position[0], moved.position[0]
        ),
    );

    // Undo and redo.
    h.key(Key::Z, ctrl);
    let undone = h.editor.transform_of(mugo).context("sin Mugo")?;
    check(
        (undone.position[0] - before.position[0]).abs() < 1e-4,
        format!("Ctrl+Z la devuelve: {:.2}", undone.position[0]),
    );
    h.key(Key::Y, ctrl);
    let redone = h.editor.transform_of(mugo).context("sin Mugo")?;
    check(
        (redone.position[0] - moved.position[0]).abs() < 1e-4,
        format!("Ctrl+Y la rehace: {:.2}", redone.position[0]),
    );

    // Move on the ground plane (the XZ square).
    let before = redone.clone();
    let (p, _) = h
        .editor
        .handle_screen(Handle::Plane(1))
        .context("sin cuadro XZ")?;
    h.drag(p, p + egui::vec2(-60.0, 40.0), None)?;
    let slid = h.editor.transform_of(mugo).context("sin Mugo")?;
    check(
        (slid.position[1] - before.position[1]).abs() < 1e-4
            && ((slid.position[0] - before.position[0]).abs() > 0.05
                || (slid.position[2] - before.position[2]).abs() > 0.05),
        format!(
            "el cuadro XZ lo desliza por el suelo: ({:.2}, {:.2}) → ({:.2}, {:.2}), altura {:.2}",
            before.position[0],
            before.position[2],
            slid.position[0],
            slid.position[2],
            slid.position[1]
        ),
    );

    // Turn about Y with the ring.
    h.key(Key::E, Modifiers::NONE);
    let before = h.editor.transform_of(mugo).context("sin Mugo")?;
    let (p, c) = h
        .editor
        .handle_screen(Handle::Ring(1))
        .context("sin anillo Y")?;
    let r = p - c;
    let tangent = egui::vec2(-r.y, r.x).normalized();
    h.drag(p, p + tangent * 90.0, Some(&shot("04_rotar.png")))?;
    let turned = h.editor.transform_of(mugo).context("sin Mugo")?;
    check(
        (turned.rotation_deg[1] - before.rotation_deg[1]).abs() > 5.0,
        format!(
            "el anillo Y gira: {:.1}° → {:.1}°",
            before.rotation_deg[1], turned.rotation_deg[1]
        ),
    );

    // Grow evenly with the centre box.
    h.key(Key::R, Modifiers::NONE);
    let before = turned.clone();
    let (p, _) = h
        .editor
        .handle_screen(Handle::ScaleAll)
        .context("sin caja de escala")?;
    h.drag(
        p,
        p + egui::vec2(70.0, -40.0),
        Some(&shot("05_escalar.png")),
    )?;
    let grown = h.editor.transform_of(mugo).context("sin Mugo")?;
    check(
        grown.scale[0] > before.scale[0] * 1.2 && (grown.scale[0] - grown.scale[2]).abs() < 1e-4,
        format!(
            "la caja central escala igual en todo: {:.2} → {:.2}",
            before.scale[0], grown.scale[0]
        ),
    );

    // Reparent in the tree by dragging a row onto another, then undo it.
    let from = h
        .editor
        .tree_row_screen("personajes/chiquita")
        .context("sin fila de Chiquita")?;
    let to = h
        .editor
        .tree_row_screen("personajes/reina")
        .context("sin fila de la Reina")?;
    h.drag(from, to, None)?;
    h.idle(2);
    check(
        h.editor.has_node("personajes/reina/chiquita"),
        "arrastrar a Chiquita sobre la Reina la hace su hija".into(),
    );
    h.save(&shot("06_arbol.png"))?;
    h.key(Key::Z, ctrl);
    check(
        h.editor.has_node("personajes/chiquita"),
        "Ctrl+Z la devuelve a su sitio".into(),
    );

    // The Script workspace with nothing open: a picker; Enter opens the first match.
    let b = h
        .editor
        .script_button_screen()
        .context("sin botón Script")?;
    h.click(b);
    check(
        h.editor.dialog_open(),
        "el botón Script sin scripts abre el buscador".into(),
    );
    h.text("mugo.rs");
    h.save(&shot("07a_buscar_script.png"))?;
    h.key(Key::Enter, Modifiers::NONE);
    h.idle(2);
    let opened = h.editor.open_scripts();
    check(
        !h.editor.dialog_open() && opened.iter().any(|p| p.ends_with("mugo.rs")),
        format!("Intro abre el primero que coincide ({opened:?})"),
    );

    // Pictures open in their own tab: one of the game's SVG icons and a PNG.
    for rel in [
        "assets/ui/shop/category_equipment.svg",
        "assets/ui/shop/cart.png",
    ] {
        let pic = a.root.join(rel);
        if !pic.exists() {
            check(
                false,
                format!("falta {rel} para probar el visor de imágenes"),
            );
            continue;
        }
        h.editor.open_file(&pic);
        h.idle(3);
        check(
            h.editor.picture_shown(&pic) == Some(true),
            format!("abrir {rel} la muestra en su pestaña"),
        );
    }
    h.save(&shot("07b_imagen.png"))?;

    // A script: open, type, save.
    let script = dir.join("prueba_script.rs");
    std::fs::write(
        &script,
        "//! Un script de prueba.\n\nfn saludo(nombre: &str) -> String {\n    format!(\"Hola, {nombre}\")\n}\n",
    )?;
    h.editor.open_script(&script);
    h.idle(3);
    let area = h.editor.script_area().context("sin editor de scripts")?;
    h.click(area.left_top() + egui::vec2(70.0, 14.0));
    h.key(Key::Home, Modifiers::NONE);
    h.text("// Editado en Velvet Editor\n");
    h.save(&shot("07_script.png"))?;
    h.key(Key::S, ctrl);
    let saved = std::fs::read_to_string(&script)?;
    check(
        saved.contains("// Editado en Velvet Editor"),
        "Ctrl+S guarda el script con lo escrito".into(),
    );

    // The characters' animations.
    h.editor.focus_viewport();
    h.editor.select("personajes/reina");
    h.editor.play_animations(true);
    for _ in 0..20 {
        h.step(Vec::new());
        std::thread::sleep(Duration::from_millis(15));
    }
    h.save(&shot("08_animar.png"))?;
    h.editor.play_animations(false);

    // Save the scene somewhere else and read it back.
    let copy = dir.join("escena_guardada.scene3d.json");
    let saved = h.editor.save_scene_as(&copy);
    let back = velvet_scene3d::load_scene_from_path(&copy);
    check(
        saved
            && back
                .as_ref()
                .is_ok_and(|s| s.nodes.len() == h.editor.s.scene.nodes.len()),
        format!("guardar como y volver a leer ({})", copy.display()),
    );
    h.idle(3);
    h.save(&shot("09_final.png"))?;

    eprintln!("[demo] {} fallos; imágenes en {}", fails, dir.display());
    if fails > 0 {
        bail!("{fails} comprobaciones fallaron");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = parse_args()?;
    if let Some(dir) = args.demo.clone() {
        return demo(&args, &dir);
    }
    if let Some(out) = args.capture.clone() {
        return capture(&args, &out);
    }
    let event_loop = EventLoop::new()?;
    let mut app = WinApp {
        args,
        running: None,
        error: None,
        next_repaint: None,
    };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
