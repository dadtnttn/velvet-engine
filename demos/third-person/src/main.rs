//! Third-person vertical slice: declarative arena, physics hero, chasing
//! enemy, beacon quest, blended locomotion, bursts, and saves.
//!
//! `--headless` runs a scripted walk (CI-friendly, exit 0 on quest
//! completion). Without flags it opens a wgpu window (WASD/arrows move,
//! Space jumps, Esc quits); rendering reuses `velvet-render` through the
//! scene bridge, so the sim stays the source of truth in both modes.

mod sim;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use velvet_math::{Mat4, Vec3};
use velvet_render::{
    scene_draws, upload_scene_meshes, Camera3D, Lighting3D, SceneMeshCache, Wgpu3DRenderer,
};
use velvet_scene3d::{flatten_scene, load_scene_from_path, FlatNode3D};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use sim::{QuestState, SimInput, ThirdPersonSim};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scene_path = root.join("scenes/arena.scene3d.json");
    if args.iter().any(|a| a == "--headless") {
        return run_headless(&scene_path);
    }
    run_windowed(&scene_path)
}

/// Scripted CI walk: steer at the beacon until the quest completes.
fn run_headless(scene_path: &std::path::Path) -> Result<()> {
    let scene = load_scene_from_path(scene_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut sim = ThirdPersonSim::new(&scene);
    let mut steps = 0;
    while sim.quest != QuestState::Done && steps < 3600 {
        steps += 1;
        let h = sim.bodies["hero"].position;
        let d = Vec3::new(sim.beacon.x - h.x, 0.0, sim.beacon.z - h.z);
        let len = (d.x * d.x + d.z * d.z).sqrt().max(1e-6);
        sim.tick(
            SimInput {
                move_dir: Vec3::new(d.x / len, 0.0, d.z / len),
                speed: 1.0,
                jump: false,
            },
            1.0 / 60.0,
        );
        if steps % 600 == 0 {
            println!(
                "headless t={:.1}s hero=({:.1},{:.1},{:.1}) hp={} quest={:?} particles={}",
                sim.time,
                h.x,
                h.y,
                h.z,
                sim.hp,
                sim.quest,
                sim.particles.len()
            );
        }
    }
    anyhow::ensure!(
        sim.quest == QuestState::Done,
        "scripted walk did not reach the beacon"
    );
    // Save roundtrip through a temp file.
    let dir = std::env::temp_dir().join("third-person-headless");
    std::fs::create_dir_all(&dir)?;
    let save_path = dir.join("slot.json");
    sim.save_to(&save_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let back = velvet_scene3d::SceneSave3D::from_json(
        &std::fs::read_to_string(&save_path).context("read save")?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut fresh = ThirdPersonSim::new(&scene);
    fresh.load(&back);
    anyhow::ensure!(
        fresh.bodies["hero"].position == sim.bodies["hero"].position,
        "save roundtrip mismatch"
    );
    println!(
        "headless ok: quest done in {steps} steps, save roundtrip ok ({} bodies)",
        back.bodies.len()
    );
    Ok(())
}

struct App {
    sim: ThirdPersonSim,
    flat: Vec<FlatNode3D>,
    camera: Camera3D,
    lighting: Lighting3D,
    keys: HashSet<KeyCode>,
    prev_jump: bool,
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    renderer: Option<Wgpu3DRenderer>,
    cache: SceneMeshCache,
    root: PathBuf,
    last: Instant,
    save_path: PathBuf,
    saved_once: bool,
}

fn run_windowed(scene_path: &std::path::Path) -> Result<()> {
    let scene = load_scene_from_path(scene_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let flat = flatten_scene(&scene);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut app = App {
        camera: Camera3D::default(),
        lighting: Lighting3D::default(),
        sim: ThirdPersonSim::new(&scene),
        flat,
        keys: HashSet::new(),
        prev_jump: false,
        window: None,
        device: None,
        queue: None,
        surface: None,
        config: None,
        renderer: None,
        cache: SceneMeshCache::default(),
        root: root.clone(),
        last: Instant::now(),
        save_path: root.join("saves/slot.json"),
        saved_once: false,
    };
    // Camera defaults come from the scene document.
    app.camera.position = Vec3::new(
        scene.camera.position[0],
        scene.camera.position[1],
        scene.camera.position[2],
    );
    app.camera.target = Vec3::new(
        scene.camera.target[0],
        scene.camera.target[1],
        scene.camera.target[2],
    );
    let event_loop = EventLoop::new().context("event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).context("run app")?;
    Ok(())
}

impl App {
    fn input(&self) -> SimInput {
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
        let len = (dir.x * dir.x + dir.z * dir.z).sqrt();
        if len > 1e-6 {
            dir = Vec3::new(dir.x / len, 0.0, dir.z / len);
        }
        SimInput {
            move_dir: dir,
            speed: if len > 1e-6 { 1.0 } else { 0.0 },
            jump: self.keys.contains(&KeyCode::Space),
        }
    }

    fn step_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        let mut input = self.input();
        // Edge-trigger jumps: holding Space must not bunny-hop.
        input.jump = input.jump && !self.prev_jump;
        self.prev_jump = self.input().jump;
        self.sim.tick(input, dt);

        // Chase camera follows the hero.
        let hero = self.sim.bodies["hero"].position;
        self.camera.target = Vec3::new(hero.x, hero.y + 1.2, hero.z);
        self.camera.position = Vec3::new(hero.x, hero.y + 9.0, hero.z + 12.0);

        if self.sim.quest == QuestState::Done && !self.saved_once {
            self.saved_once = true;
            if let Some(parent) = self.save_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match self.sim.save_to(&self.save_path) {
                Ok(()) => println!("quest done — saved to {}", self.save_path.display()),
                Err(e) => eprintln!("save failed: {e}"),
            }
        }
        if let Some(window) = &self.window {
            window.set_title(&format!(
                "Third-person slice — hp={} quest={:?} particles={}",
                self.sim.hp,
                self.sim.quest,
                self.sim.particles.len()
            ));
            window.request_redraw();
        }
    }

    fn redraw(&mut self) {
        let (device, queue, surface, config, renderer) = match (
            &self.device,
            &self.queue,
            &self.surface,
            &self.config,
            &mut self.renderer,
        ) {
            (Some(d), Some(q), Some(s), Some(c), Some(r)) => (d, q, s, c, r),
            _ => return,
        };
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(device, config);
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
        let size = (config.width, config.height);
        renderer.render(
            device,
            queue,
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
        frame.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Third-person slice")
                        .with_inner_size(LogicalSize::new(1280, 720)),
                )
                .expect("window"),
        );
        let (device, queue, surface, config) = pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            let surface = instance.create_surface(window.clone()).expect("surface");
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("adapter");
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("third-person-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None,
                )
                .await
                .expect("device");
            let size = window.inner_size();
            let capabilities = surface.get_capabilities(&adapter);
            let format = capabilities
                .formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .unwrap_or(capabilities.formats[0]);
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
            (device, queue, surface, config)
        });
        let mut renderer = Wgpu3DRenderer::new(
            &device,
            &queue,
            config.format,
            (config.width, config.height),
        );
        // The demo arena is primitive-only, so this never touches disk.
        self.cache = upload_scene_meshes(&mut renderer, &device, &self.root, &self.flat)
            .expect("upload arena meshes");
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.config = Some(config);
        self.renderer = Some(renderer);
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
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
            WindowEvent::Resized(size) => {
                if let (Some(device), Some(surface), Some(config)) =
                    (&self.device, &self.surface, &mut self.config)
                {
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(device, config);
                }
            }
            WindowEvent::RedrawRequested => {
                self.step_and_render();
                self.redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
