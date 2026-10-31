//! The windowed shell: renders what the sim computes through `velvet-render`, on desktop and on
//! Android (`velvet-platform` gives the event loop, the GPU surface that survives the app going
//! to the background, the packaged scene file, the save folder and touch input).
//!
//! Desktop: WASD or arrows move, Space jumps, Esc quits. Touch: a finger on the left half of the
//! screen is a stick (its offset from where it landed steers), a tap on the right half jumps,
//! the back button quits. Progress is saved when the quest is done and whenever the app leaves
//! the screen, and loaded at the next start.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use velvet_math::{Mat4, Vec3};
use velvet_platform::gpu::SurfaceSlot;
use velvet_platform::touch::{TouchChange, Touches};
use velvet_platform::{Lifecycle, PlatformEvent};
use velvet_render::{
    scene_draws, upload_scene_meshes, Camera3D, Lighting3D, SceneMeshCache, Wgpu3DRenderer,
};
use velvet_scene3d::{flatten_scene, FlatNode3D, Scene3D, SceneSave3D};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Touch, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::sim::{QuestState, SimInput, ThirdPersonSim};

/// The folder (per user, per app) the save goes to.
const APP_DIR: &str = "velvet-third-person";

/// Runs the windowed slice until the window closes (desktop) or the back button (Android).
pub(crate) fn run(scene: Scene3D) -> Result<()> {
    let mut app = App::new(&scene);
    let event_loop = velvet_platform::event_loop().context("event loop")?;
    // Wait, not Poll: frames come from request_redraw while shown, and a backgrounded Android
    // app then sleeps instead of spinning the CPU.
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app).context("run app")?;
    Ok(())
}

/// The GPU objects, made when the window first appears and kept for the app's life (only the
/// surface goes away while the app is in the background).
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: SurfaceSlot,
    config: wgpu::SurfaceConfiguration,
    renderer: Wgpu3DRenderer,
}

impl Gpu {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let instance = wgpu::Instance::new(&velvet_platform::gpu::instance_descriptor());
        let mut surface = SurfaceSlot::new(instance, window.clone()).context("surface")?;
        let adapter = velvet_platform::gpu::request_adapter(
            surface.instance(),
            surface.surface().context("surface")?,
            wgpu::PowerPreference::HighPerformance,
        )
        .await
        .context("no graphics adapter for the window")?;
        eprintln!("[third-person] adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("third-person-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: velvet_platform::gpu::supported_limits(&adapter),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .context("device")?;
        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let config = velvet_platform::gpu::surface_config(&capabilities, size.width, size.height)
            .context("the surface offers no format")?;
        surface.configure(&device, &config);
        let renderer = Wgpu3DRenderer::new(
            &device,
            &queue,
            config.format,
            (config.width, config.height),
        );
        Ok(Self {
            device,
            queue,
            surface,
            config,
            renderer,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}

/// Touch controls: a finger landing on the left half is a stick, a tap on the right half jumps.
#[derive(Default)]
struct TouchControls {
    touches: Touches,
    stick: Option<u64>,
    jump: bool,
}

impl TouchControls {
    fn handle(&mut self, touch: &Touch, window_width: f64) {
        match self.touches.handle(touch) {
            Some(TouchChange::Pressed(finger)) => {
                if finger.start.x < window_width / 2.0 {
                    self.stick.get_or_insert(finger.id);
                } else {
                    self.jump = true;
                }
            }
            Some(TouchChange::Released(finger) | TouchChange::Cancelled(finger))
                if self.stick == Some(finger.id) =>
            {
                self.stick = None;
            }
            _ => {}
        }
    }

    /// The stick's direction on the screen (x right, y down), its length the speed (0..1).
    fn stick(&self, scale_factor: f64) -> Option<(f32, f32)> {
        let finger = self.touches.get(self.stick?)?;
        // Full speed at 60 dp from where the finger landed, nothing inside 15 % of that.
        let radius = 60.0 * scale_factor.max(0.5);
        let dx = (finger.position.x - finger.start.x) / radius;
        let dy = (finger.position.y - finger.start.y) / radius;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.15 {
            return None;
        }
        let k = len.min(1.0) / len;
        Some(((dx * k) as f32, (dy * k) as f32))
    }

    fn clear(&mut self) {
        self.touches.clear();
        self.stick = None;
        self.jump = false;
    }
}

struct App {
    sim: ThirdPersonSim,
    flat: Vec<FlatNode3D>,
    camera: Camera3D,
    lighting: Lighting3D,
    keys: HashSet<KeyCode>,
    prev_jump: bool,
    touch: TouchControls,
    lifecycle: Lifecycle,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    cache: SceneMeshCache,
    last: Instant,
    save_path: Option<PathBuf>,
    saved_once: bool,
}

impl App {
    fn new(scene: &Scene3D) -> Self {
        let mut sim = ThirdPersonSim::new(scene);
        let save_path = velvet_platform::files::data_dir(APP_DIR)
            .map(|dir| dir.join("slot.json"))
            .map_err(|e| eprintln!("[third-person] no save folder: {e}"))
            .ok();
        if let Some(path) = &save_path {
            if let Some(save) = std::fs::read_to_string(path)
                .ok()
                .and_then(|text| SceneSave3D::from_json(&text).ok())
            {
                sim.load(&save);
                eprintln!("[third-person] continued from {}", path.display());
            }
        }
        // Camera defaults come from the scene document.
        let camera = Camera3D {
            position: Vec3::new(
                scene.camera.position[0],
                scene.camera.position[1],
                scene.camera.position[2],
            ),
            target: Vec3::new(
                scene.camera.target[0],
                scene.camera.target[1],
                scene.camera.target[2],
            ),
            ..Camera3D::default()
        };
        Self {
            sim,
            flat: flatten_scene(scene),
            camera,
            lighting: Lighting3D::default(),
            keys: HashSet::new(),
            prev_jump: false,
            touch: TouchControls::default(),
            lifecycle: Lifecycle::new()
                .with_immersive(true)
                .with_keep_screen_on(true),
            window: None,
            gpu: None,
            cache: SceneMeshCache::default(),
            last: Instant::now(),
            save_path,
            saved_once: false,
        }
    }

    fn input(&mut self) -> SimInput {
        let mut dir = Vec3::ZERO;
        if self.keys.contains(&KeyCode::KeyW) || self.keys.contains(&KeyCode::ArrowUp) {
            dir.z -= 1.0;
        }
        if self.keys.contains(&KeyCode::KeyS) || self.keys.contains(&KeyCode::ArrowDown) {
            dir.z += 1.0;
        }
        if self.keys.contains(&KeyCode::KeyA) || self.keys.contains(&KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if self.keys.contains(&KeyCode::KeyD) || self.keys.contains(&KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        let mut speed = if dir.x != 0.0 || dir.z != 0.0 {
            1.0
        } else {
            0.0
        };
        let scale = self.window.as_ref().map_or(1.0, |w| w.scale_factor());
        if let Some((x, y)) = self.touch.stick(scale) {
            // Up the screen is forward (-z), right is +x.
            dir = Vec3::new(x, 0.0, y);
            speed = (x * x + y * y).sqrt();
        }
        let len = (dir.x * dir.x + dir.z * dir.z).sqrt();
        if len > 1e-6 {
            dir = Vec3::new(dir.x / len, 0.0, dir.z / len);
        }
        let held = self.keys.contains(&KeyCode::Space);
        // Edge-trigger jumps: holding Space must not bunny-hop.
        let jump = (held && !self.prev_jump) || std::mem::take(&mut self.touch.jump);
        self.prev_jump = held;
        SimInput {
            move_dir: dir,
            speed,
            jump,
        }
    }

    fn save(&self, why: &str) {
        let Some(path) = &self.save_path else {
            return;
        };
        let written = self
            .sim
            .save()
            .to_json()
            .map_err(anyhow::Error::msg)
            .and_then(|json| {
                velvet_platform::files::write_atomic(path, json.as_bytes())
                    .map_err(anyhow::Error::from)
            });
        match written {
            Ok(()) => eprintln!("[third-person] {why}: saved to {}", path.display()),
            Err(e) => eprintln!("[third-person] save failed: {e}"),
        }
    }

    fn step_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        let input = self.input();
        self.sim.tick(input, dt);

        // Chase camera follows the hero.
        let hero = self.sim.bodies["hero"].position;
        self.camera.target = Vec3::new(hero.x, hero.y + 1.2, hero.z);
        self.camera.position = Vec3::new(hero.x, hero.y + 9.0, hero.z + 12.0);

        if self.sim.quest == QuestState::Done && !self.saved_once {
            self.saved_once = true;
            self.save("quest done");
        }
        if let Some(window) = &self.window {
            window.set_title(&format!(
                "Third-person slice — hp={} quest={:?} particles={}",
                self.sim.hp,
                self.sim.quest,
                self.sim.particles.len()
            ));
        }
        self.redraw();
    }

    fn redraw(&mut self) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        let frame = match gpu.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                return;
            }
            Err(_) => return,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut overrides: HashMap<String, Mat4> = HashMap::new();
        overrides.insert("hero".into(), self.sim.hero_matrix());
        overrides.insert("enemy".into(), self.sim.body_matrix("enemy"));
        let draws = scene_draws(&self.cache, &self.flat, &overrides);
        let size = (gpu.config.width, gpu.config.height);
        gpu.renderer.render(
            &gpu.device,
            &gpu.queue,
            &view,
            size,
            self.camera,
            self.lighting,
            &draws,
            wgpu::Color {
                r: 0.05,
                g: 0.06,
                b: 0.10,
                a: 1.0,
            },
        );
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        frame.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.lifecycle.resumed();
        self.last = Instant::now();
        if let Some(gpu) = &mut self.gpu {
            // Back from the background (Android): a new surface for the same window.
            match gpu.surface.resume(&gpu.device) {
                Ok(_) => {
                    if let Some(window) = &self.window {
                        let size = window.inner_size();
                        gpu.resize(size.width, size.height);
                    }
                }
                Err(e) => eprintln!("[third-person] the surface could not be made again: {e}"),
            }
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Third-person slice")
                .with_inner_size(LogicalSize::new(1280, 720)),
        ) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                eprintln!("[third-person] no window: {e}");
                event_loop.exit();
                return;
            }
        };
        let mut gpu = match pollster::block_on(Gpu::new(window.clone())) {
            Ok(gpu) => gpu,
            Err(e) => {
                eprintln!("[third-person] graphics initialisation failed: {e:#}");
                event_loop.exit();
                return;
            }
        };
        // The demo arena is primitive-only, so this never touches disk.
        match upload_scene_meshes(
            &mut gpu.renderer,
            &gpu.device,
            std::path::Path::new("."),
            &self.flat,
        ) {
            Ok(cache) => self.cache = cache,
            Err(e) => eprintln!("[third-person] arena meshes: {e}"),
        }
        self.gpu = Some(gpu);
        self.window = Some(window);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.lifecycle.suspended();
        // Android destroys the window's surface now, and may end the process later without
        // another word: drop the surface, forget touches in flight, save.
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.suspend();
        }
        self.touch.clear();
        self.keys.clear();
        self.save("left the screen");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(PlatformEvent::Back) = self.lifecycle.window_event(&event) {
            event_loop.exit();
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Escape {
                        event_loop.exit();
                        return;
                    }
                    match event.state {
                        ElementState::Pressed => {
                            self.keys.insert(code);
                        }
                        ElementState::Released => {
                            self.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                let width = self
                    .window
                    .as_ref()
                    .map_or(1.0, |w| w.inner_size().width as f64);
                self.touch.handle(&touch, width);
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.step_and_render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
