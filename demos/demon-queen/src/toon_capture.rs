//! Explicit CLI-only render regression captures. No window, input injection,
//! save writes or changes to the player's live session.
use super::*;

/// How finely the maps' pictures are baked.
const MAP_PX_PER_M: f32 = 18.0;
/// The square cut from the middle of each frame (the frames are 1920 x 1080).
const MAP_TILE_PX: u32 = 1080;

/// The maps' pictures: every place seen from straight above at noon, north (-z) up, with no HUD,
/// no haze, no Queen and the castle's walls all standing; square tiles cut from the middle of
/// frames and stitched, then cut to the place. Writes `<name>.png` and `maps.json` (each
/// picture's world rectangle: x0, x1, z0, z1) into `out`, for `data/maps/`.
fn bake_maps(app: &mut App, out: &Path) -> Result<()> {
    let tile_m = MAP_TILE_PX as f32 / MAP_PX_PER_M;
    app.environment_cycle.set_phase(0.5);
    app.environment_cycle_enabled = false;
    if let Some(state) = app.state.as_mut() {
        state.renderer.no_fog = true;
        state.map_overlay.hidden = true;
    }
    let (clearing, radius) = village::clearing();
    let r = radius + 4.0;
    let [cx0, cx1, cz0, cz1] = castle3d::plan().bounds();
    let castle_rect = [cx0 - 1.5, cx1 + 1.5, cz0 - 1.5, cz1 + 1.5];
    type Setup = fn(&mut App);
    // (name, world rectangle, setup); the dark forest's picture is lifted (its noon is dim).
    let lift = |name: &str| if name == "bosque" { 1.7 } else { 1.0 };
    let [px0, px1, pz0, pz1] = penascal3d::plan().bounds();
    let places: [(&str, [f32; 4], Setup); 6] = [
        (
            "villa",
            [
                clearing.x - r,
                clearing.x + r,
                clearing.z - r,
                clearing.z + r,
            ],
            |app| {
                app.castle = None;
                app.baths = None;
                app.brothel = None;
                app.nature_mode = true;
                app.village_entered = true;
            },
        ),
        ("bosque", [-27.5, 27.5, -27.5, 27.5], |app| {
            app.castle = None;
            app.baths = None;
            app.brothel = None;
            app.nature_mode = true;
            app.village_entered = false;
        }),
        ("termas", [-22.0, 22.0, -20.0, 20.0], |app| {
            app.castle = None;
            app.brothel = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.baths = Some(baths::State::enter(Vec3::ZERO, 0.0));
        }),
        ("castillo-0", castle_rect, |app| {
            app.baths = None;
            app.brothel = None;
            app.nature_mode = false;
            app.village_entered = false;
            if let Some(state) = app.state.as_mut() {
                state.ensure_castle_loaded();
            }
            let mut inside = castle3d::State::restore(Vec3::new(0.0, 0.0, 1.0));
            inside.keep_walls = true;
            app.castle = Some(inside);
        }),
        ("castillo-1", castle_rect, |app| {
            app.baths = None;
            app.brothel = None;
            app.nature_mode = false;
            app.village_entered = false;
            if let Some(state) = app.state.as_mut() {
                state.ensure_castle_loaded();
            }
            let mut inside =
                castle3d::State::restore(Vec3::new(2.0, castle3d::plan().level_h, 13.6));
            inside.keep_walls = true;
            app.castle = Some(inside);
        }),
        (
            "penascal",
            [px0 - 3.0, px1 + 3.0, pz0 - 3.0, pz1 + 3.0],
            |app| {
                app.castle = None;
                app.baths = None;
                app.brothel = None;
                app.nature_mode = true;
                app.village_entered = false;
                if let Some(state) = app.state.as_mut() {
                    state.ensure_penascal_loaded();
                }
                let mut inside = penascal3d::State::way_in();
                inside.keep_walls = true;
                app.penascal = Some(inside);
            },
        ),
    ];
    let mut index = serde_json::Map::new();
    for (name, [x0, x1, z0, z1], setup) in places {
        setup(app);
        // The Queen far off (she is drawn where she stands; what she lights is not wanted).
        app.player = Vec3::new(5000.0, 0.0, 5000.0);
        let (nx, nz) = (
            ((x1 - x0) / tile_m).ceil() as u32,
            ((z1 - z0) / tile_m).ceil() as u32,
        );
        let (cw, ch) = (nx * MAP_TILE_PX, nz * MAP_TILE_PX);
        let mut canvas = vec![0u8; (cw * ch * 3) as usize];
        for j in 0..nz {
            for i in 0..nx {
                let centre = Vec3::new(
                    x0 + (i as f32 + 0.5) * tile_m,
                    0.0,
                    z0 + (j as f32 + 0.5) * tile_m,
                );
                app.camera_override = Some(Camera3D {
                    position: centre + Vec3::new(0.0, 150.0, 0.0),
                    target: centre,
                    up: Vec3::new(0.0, 0.0, -1.0),
                    fov_y_radians: 0.6,
                    orthographic_height: Some(tile_m),
                    near: 1.0,
                    far: 400.0,
                });
                // Settle: the shadows rebuilt for the new view, the bounced light gathered.
                for _ in 0..16 {
                    app.elapsed += 1.0 / 60.0;
                    app.render_frame();
                    if let Some(state) = app.state.as_ref() {
                        let _ = state.device.poll(wgpu::Maintain::Wait);
                    }
                }
                let frame = app.capture_pixels()?;
                let left = (WINDOW_WIDTH - MAP_TILE_PX) / 2;
                for y in 0..MAP_TILE_PX.min(WINDOW_HEIGHT) {
                    for x in 0..MAP_TILE_PX {
                        let from = ((y * WINDOW_WIDTH + left + x) * 4) as usize;
                        let to = (((j * MAP_TILE_PX + y) * cw + i * MAP_TILE_PX + x) * 3) as usize;
                        canvas[to..to + 3].copy_from_slice(&frame[from..from + 3]);
                    }
                }
            }
        }
        // Cut to the place itself.
        let (w, h) = (
            ((x1 - x0) * MAP_PX_PER_M).round() as u32,
            ((z1 - z0) * MAP_PX_PER_M).round() as u32,
        );
        let mut picture = Vec::with_capacity((w * h * 3) as usize);
        let k = lift(name);
        for y in 0..h {
            let row = ((y * cw) * 3) as usize;
            picture.extend(
                canvas[row..row + (w * 3) as usize]
                    .iter()
                    .map(|&c| (c as f32 * k).min(255.0) as u8),
            );
        }
        image::save_buffer(
            out.join(format!("{name}.png")),
            &picture,
            w,
            h,
            image::ColorType::Rgb8,
        )?;
        index.insert(
            name.to_owned(),
            serde_json::json!({ "x0": x0, "x1": x1, "z0": z0, "z1": z1 }),
        );
        eprintln!("[toon-qa] baked map {name}: {w} x {h} px ({nx} x {nz} tiles)");
    }
    std::fs::write(
        out.join("maps.json"),
        serde_json::to_vec_pretty(&serde_json::Value::Object(index))?,
    )?;
    app.camera_override = None;
    app.castle = None;
    app.baths = None;
    app.penascal = None;
    if let Some(state) = app.state.as_mut() {
        state.renderer.no_fog = false;
        state.map_overlay.hidden = false;
    }
    Ok(())
}

use velvet_qa::private_mb;

pub(super) fn capture(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: if cfg!(windows) {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::all()
        },
        ..Default::default()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .context("toon QA adapter")?;
    eprintln!("[toon-qa] {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
            ..Default::default()
        },
        None,
    ))?;
    let gpu = GpuContext {
        device,
        queue,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        msaa_samples: 4,
    };
    if let Some(rounds) = std::env::args()
        .skip_while(|a| a != "--soak-reload")
        .nth(1)
        .and_then(|n| n.parse::<usize>().ok())
    {
        // Leak hunt: the 3D world built and dropped again and again (as every trip to the visual
        // novel and back does), with the process's private memory after each round.
        // VELVET_RELOAD=gpu builds only the GPU side (no world state, no frames).
        // VELVET_RELOAD=renderer|overlay|dungeon|forest: only that piece, made and dropped.
        let piece = std::env::var("VELVET_RELOAD").unwrap_or_default();
        if [
            "renderer", "overlay", "dungeon", "forest", "textures", "meshes", "raw", "prep",
            "threads", "castle", "baths", "brothel",
        ]
        .contains(&piece.as_str())
        {
            for round in 0..rounds {
                let (device, queue) = (&gpu.device, &gpu.queue);
                let mut renderer = Wgpu3DRenderer::new_with_msaa(
                    device,
                    queue,
                    gpu.format,
                    (WINDOW_WIDTH, WINDOW_HEIGHT),
                    gpu.msaa_samples,
                );
                match piece.as_str() {
                    "overlay" => drop(map_overlay::MapOverlay::new(device, queue, gpu.format)),
                    "dungeon" => drop(dungeon::Scene::new(
                        &mut renderer,
                        device,
                        queue,
                        gpu.format,
                    )?),
                    "forest" => drop(build_dark_forest(
                        &mut renderer,
                        device,
                        queue,
                        &dark_forest_asset_root(),
                    )?),
                    "castle" => drop(castle3d::Scene::load(&mut renderer, device, queue)?),
                    "baths" => drop(baths::Scene::load(&mut renderer, device, queue)?),
                    "brothel" => drop(brothel::Scene::load(&mut renderer, device, queue)?),
                    // 40 textures of 1024² with normal maps through the renderer.
                    "textures" => {
                        for k in 0..40u32 {
                            let image = TextureData3D {
                                width: 1024,
                                height: 1024,
                                rgba: (0..1024 * 1024 * 4)
                                    .map(|i| ((i as u32 * 7 + k * 13) % 251) as u8)
                                    .collect(),
                            };
                            let _ = renderer.upload_texture_with_normal(
                                device,
                                queue,
                                &image,
                                Some(&image),
                            );
                        }
                    }
                    // The same 40 textures prepared (mips, BC7) but not uploaded.
                    "prep" => {
                        for k in 0..40u32 {
                            let image = TextureData3D {
                                width: 1024,
                                height: 1024,
                                rgba: (0..1024 * 1024 * 4)
                                    .map(|i| ((i as u32 * 7 + k * 13) % 251) as u8)
                                    .collect(),
                            };
                            drop(velvet_render::PreparedTexture3D::new(&image, Some(&image)));
                        }
                    }
                    // As many short threads as the BC7 encoding of those textures starts.
                    "threads" => {
                        for _ in 0..200 {
                            std::thread::scope(|scope| {
                                for k in 0..16usize {
                                    scope.spawn(move || {
                                        std::hint::black_box(vec![k as u8; 4096]).len()
                                    });
                                }
                            });
                        }
                    }
                    // 40 meshes of 20k vertices.
                    "meshes" => {
                        for _ in 0..40 {
                            let _ = renderer.upload_mesh(
                                device,
                                &Mesh3D::uv_sphere(1.0, 140, 140, Material3D::rgb(1.0, 1.0, 1.0)),
                            );
                        }
                    }
                    // 40 plain wgpu textures written from the CPU, the renderer not involved.
                    "raw" => {
                        let pixels: Vec<u8> =
                            (0..1024 * 1024 * 4).map(|i| (i % 251) as u8).collect();
                        let textures: Vec<wgpu::Texture> = (0..40)
                            .map(|_| {
                                let t = device.create_texture(&wgpu::TextureDescriptor {
                                    label: Some("soak-raw"),
                                    size: wgpu::Extent3d {
                                        width: 1024,
                                        height: 1024,
                                        depth_or_array_layers: 1,
                                    },
                                    mip_level_count: 1,
                                    sample_count: 1,
                                    dimension: wgpu::TextureDimension::D2,
                                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                                        | wgpu::TextureUsages::COPY_DST,
                                    view_formats: &[],
                                });
                                queue.write_texture(
                                    wgpu::TexelCopyTextureInfo {
                                        texture: &t,
                                        mip_level: 0,
                                        origin: wgpu::Origin3d::ZERO,
                                        aspect: wgpu::TextureAspect::All,
                                    },
                                    &pixels,
                                    wgpu::TexelCopyBufferLayout {
                                        offset: 0,
                                        bytes_per_row: Some(4096),
                                        rows_per_image: Some(1024),
                                    },
                                    wgpu::Extent3d {
                                        width: 1024,
                                        height: 1024,
                                        depth_or_array_layers: 1,
                                    },
                                );
                                t
                            })
                            .collect();
                        queue.submit(std::iter::empty());
                        drop(textures);
                    }
                    _ => {}
                }
                drop(renderer);
                queue.submit(std::iter::empty());
                let _ = device.poll(wgpu::Maintain::Wait);
                let before = private_mb();
                // VELVET_HEAP_COMPACT: hand the heap's free blocks back, to tell a leak from a
                // heap that only keeps what was freed.
                #[cfg(windows)]
                if std::env::var("VELVET_HEAP_COMPACT").is_ok() {
                    // SAFETY: compacting the process heap only coalesces and releases free blocks.
                    unsafe {
                        windows_sys::Win32::System::Memory::HeapCompact(
                            windows_sys::Win32::System::Memory::GetProcessHeap(),
                            0,
                        );
                    }
                }
                eprintln!(
                    "[soak-reload] round {round}: {} MB private ({before} before compacting)",
                    private_mb()
                );
            }
            return Ok(());
        }
        let gpu_only = piece == "gpu";
        // VELVET_RELOAD=app builds the world state too, but draws no frames.
        let no_frames = std::env::var("VELVET_RELOAD").is_ok_and(|v| v == "app");
        // VELVET_RELOAD_THREAD: each world built on a thread of its own, as the game loads them.
        let on_thread = std::env::var("VELVET_RELOAD_THREAD").is_ok();
        for round in 0..rounds {
            let state = if on_thread {
                let gpu = gpu.clone();
                std::thread::spawn(move || App::prepare(gpu, 42))
                    .join()
                    .map_err(|_| anyhow::anyhow!("the loading thread panicked"))??
            } else {
                App::prepare(gpu.clone(), 42)?
            };
            if gpu_only {
                drop(state);
                // (Uploads wait in the queue until something is submitted.)
                gpu.queue.submit(std::iter::empty());
            } else {
                let mut app = App::new(
                    model_path(),
                    "toon-qa".into(),
                    "bedroom_inside".into(),
                    out.join("unused-qa-session.json"),
                );
                app.state = Some(state);
                app.nature_mode = true;
                app.village_entered = true;
                for _ in 0..if no_frames { 0 } else { 60 } {
                    app.tick(1.0 / 60.0);
                    app.render_frame();
                    let _ = gpu.device.poll(wgpu::Maintain::Wait);
                }
                drop(app);
                gpu.queue.submit(std::iter::empty());
            }
            let _ = gpu.device.poll(wgpu::Maintain::Wait);
            eprintln!("[soak-reload] round {round}: {} MB private", private_mb());
        }
        return Ok(());
    }
    let mut app = App::new(
        model_path(),
        "toon-qa".into(),
        "bedroom_inside".into(),
        out.join("unused-qa-session.json"),
    );
    app.dungeon = dungeon::Dungeon::generate(42);
    let state = App::prepare(gpu, 42)?;
    app.state = Some(state);
    app.elapsed = 1.0;
    app.environment_cycle.set_phase(0.30);
    app.environment_cycle_enabled = false;
    app.iso.distance = 9.0;
    app.camera_focus = app.player;
    fn save(app: &mut App, out: &Path, name: &str) -> Result<()> {
        for _ in 0..4 {
            app.elapsed += 1.0 / 30.0;
            app.render_frame();
        }
        app.capture(&out.join(format!("{name}.png")))?;
        // VELVET_GI_DUMP: the bounced light of the frame too (×4, tone mapped), to look at it.
        if std::env::var("VELVET_GI_DUMP").is_ok() {
            if let Some(state) = app.state.as_ref() {
                if let Some((w, h, light)) = state
                    .renderer
                    .read_bounced_light(&state.device, &state.queue)
                {
                    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
                    let (mut sum, mut peak) = (0.0f64, 0.0f32);
                    for p in &light {
                        for c in &p[..3] {
                            let v = (c * 4.0).max(0.0);
                            rgb.push(((v / (1.0 + v)).powf(1.0 / 2.2) * 255.0) as u8);
                            sum += f64::from(*c);
                            peak = peak.max(*c);
                        }
                    }
                    image::save_buffer(
                        out.join(format!("{name}-gi.png")),
                        &rgb,
                        w,
                        h,
                        image::ColorType::Rgb8,
                    )?;
                    eprintln!(
                        "[toon-qa] {name}: bounced light mean {:.4} peak {:.3}",
                        sum / (light.len() as f64 * 3.0),
                        peak
                    );
                }
            }
        }
        eprintln!("[toon-qa] captured {name}");
        Ok(())
    }
    if let Some(frames) = std::env::args()
        .skip_while(|a| a != "--soak-world")
        .nth(1)
        .and_then(|n| n.parse::<usize>().ok())
    {
        // Leak hunt: walk the villa, the forest and the cave in turns,
        // rendering every frame; watch the process memory from outside.
        for frame in 0..frames {
            // (VELVET_SOAK=0,1,2 keeps to those phases: to find which one leaks.)
            let phases: Vec<usize> = std::env::var("VELVET_SOAK")
                .ok()
                .map(|v| v.split(',').filter_map(|p| p.parse().ok()).collect())
                .filter(|v: &Vec<usize>| !v.is_empty())
                .unwrap_or(vec![0, 1, 2]);
            let phase = phases[(frame / 600) % phases.len()];
            let t = frame as f32 / 60.0;
            // (The cave is entered once a turn and left at its end: re-entering it every frame
            // made the soak itself grow.)
            match phase {
                0 => {
                    app.nature_mode = true;
                    app.village_entered = true;
                    app.player = village::entrance()
                        + Vec3::new(
                            (t * 0.7).sin() * 1.0,
                            0.0,
                            -2.0 - (t * 0.3).sin().abs() * 12.0,
                        );
                }
                1 => {
                    app.nature_mode = true;
                    app.village_entered = false;
                    app.player = mine::entry_position()
                        - Vec3::new(2.0 + (t * 0.5).sin() * 1.5, 0.0, (t * 0.4).cos() * 3.0);
                }
                _ => {
                    if !app.dungeon_mode {
                        app.enter_dungeon_unchecked();
                    }
                    let mut input = velvet_dungeon::controller::Input::default();
                    input.forward = if (t * 0.2).sin() > -0.3 { 1.0 } else { 0.0 };
                    app.dungeon.controller.yaw += 0.01;
                    app.dungeon.update(input, 1.0 / 60.0);
                }
            }
            app.camera_focus = app.player;
            app.elapsed += 1.0 / 60.0;
            app.render_frame();
            // As the windowed game is held to its swap chain: wait for the GPU each frame (with
            // no window nothing else stops the CPU queueing frames without end, and what the
            // queued frames hold then looks like a leak).
            if let Some(state) = app.state.as_ref() {
                let _ = state.device.poll(wgpu::Maintain::Wait);
            }
            if phase != 2 && app.dungeon_mode {
                app.leave_dungeon();
            }
            if frame % 600 == 0 {
                eprintln!("[soak] frame {frame} phase {phase}");
            }
        }
        return Ok(());
    }
    if let Some(frames) = std::env::args()
        .skip_while(|a| a != "--frame-times")
        .nth(1)
        .and_then(|n| n.parse::<usize>().ok())
    {
        // Frame cost: walk the villa, then the forest trail, rendering and waiting for the GPU
        // every frame (so the CPU never runs ahead), and report the times.
        let report = |name: &str, times: &mut Vec<f32>| {
            times.sort_by(|a, b| a.total_cmp(b));
            let avg = times.iter().sum::<f32>() / times.len().max(1) as f32;
            let at = |q: f32| times[((times.len() as f32 * q) as usize).min(times.len() - 1)];
            eprintln!(
                "[frame-times] {name}: {} frames, avg {:.2} ms ({:.0} fps), p50 {:.2}, p95 {:.2}, max {:.2} ms",
                times.len(), avg, 1000.0 / avg.max(0.001), at(0.5), at(0.95), times.last().copied().unwrap_or(0.0)
            );
        };
        for (name, in_village) in [("villa", true), ("forest", false)] {
            app.nature_mode = true;
            app.village_entered = in_village;
            app.iso.distance = 12.0;
            let mut times = Vec::with_capacity(frames);
            let mut cpu = Vec::with_capacity(frames);
            for frame in 0..frames + 30 {
                let t = frame as f32 / 60.0;
                app.player = if in_village {
                    // Up the road from the gate, over the bridge, round the plaza.
                    let (plaza, _) = village::clearing();
                    let (spawn, _) = village::spawn();
                    let k = (t * 0.05).fract();
                    if k < 0.5 {
                        spawn + (plaza - spawn) * (k * 2.0)
                    } else {
                        let a = (k - 0.5) * std::f32::consts::TAU * 2.0;
                        plaza + Vec3::new(a.cos() * 4.0, 0.0, a.sin() * 4.0)
                    }
                } else {
                    mine::entry_position()
                        - Vec3::new(2.0 + (t * 0.5).sin() * 1.5, 0.0, (t * 0.4).cos() * 3.0)
                };
                app.camera_focus = app.player;
                app.elapsed += 1.0 / 60.0;
                let started = std::time::Instant::now();
                app.render_frame();
                let submitted = started.elapsed().as_secs_f32() * 1000.0;
                app.wait_for_gpu();
                // The first frames load and upload the scene: not counted.
                if frame >= 30 {
                    times.push(started.elapsed().as_secs_f32() * 1000.0);
                    cpu.push(submitted);
                }
            }
            report(name, &mut times);
            report(&format!("{name} (cpu)"), &mut cpu);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--doctor") {
        // Nyxara on her round in the infirmary upstairs: a few moments of it, then a word with her.
        app.nature_mode = false;
        app.village_entered = false;
        app.baths = None;
        app.brothel = None;
        if let Some(state) = app.state.as_mut() {
            state.ensure_castle_loaded();
        }
        let inside = castle3d::State::restore(Vec3::new(-17.5, castle3d::plan().level_h, 13.5));
        app.player = inside.position();
        app.camera_focus = inside.focus();
        app.castle = Some(inside);
        for _ in 0..90 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        for (k, secs) in [1.0f32, 5.0, 6.0, 7.0, 9.0].iter().enumerate() {
            for _ in 0..(secs * 30.0) as usize {
                app.tick(1.0 / 30.0);
                app.elapsed += 1.0 / 30.0;
            }
            if let Some(c) = app.castle.as_ref() {
                eprintln!(
                    "[doctor-qa] {k}: at {:.1},{:.1} clip {} facing {:.2}",
                    c.doctor.at.x, c.doctor.at.z, c.doctor.clip, c.doctor.facing
                );
            }
            save(&mut app, out, &format!("doctor-{k}"))?;
        }
        // Up to her, and F.
        if let Some(c) = app.castle.as_mut() {
            let d = c.doctor.at;
            *c = castle3d::State::restore(Vec3::new(d.x + 1.3, castle3d::plan().level_h, d.z));
            c.doctor_qa_hold(d);
        }
        if let Some(c) = app.castle.as_ref() {
            app.player = c.position();
        }
        app.interact_castle();
        for _ in 0..40 {
            app.tick(1.0 / 30.0);
            app.elapsed += 1.0 / 30.0;
        }
        save(&mut app, out, "doctor-talk")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mage-menu") {
        // The hypnotist's stall in the wood: his greeting, the shop (salts bought), a talk and its
        // question, «no, para nada», another talk, «no demasiado», his ambush, and his game in the
        // wood's light.
        app.nature_mode = true;
        app.village_entered = false;
        app.castle = None;
        app.environment_cycle.set_phase(0.35);
        let mage = mage_position();
        app.player = Vec3::new(mage.x + 1.4, 0.0, mage.z + 1.2);
        app.camera_focus = app.player;
        app.gold = 120;
        app.story = guion::Vars::default();
        for _ in 0..90 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let wait = |app: &mut App, seconds: f32| {
            for _ in 0..(seconds * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.elapsed += 1.0 / 30.0;
            }
        };
        let key = |app: &mut App, code: KeyCode| app.mage_key(code);
        app.interact_forest();
        wait(&mut app, 1.4);
        save(&mut app, out, "mage-main")?;
        key(&mut app, KeyCode::ArrowDown);
        key(&mut app, KeyCode::Enter);
        wait(&mut app, 1.0);
        save(&mut app, out, "mage-shop")?;
        key(&mut app, KeyCode::Enter);
        wait(&mut app, 0.8);
        save(&mut app, out, "mage-bought")?;
        key(&mut app, KeyCode::ArrowDown);
        wait(&mut app, 0.3);
        save(&mut app, out, "mage-shop-amulet")?;
        key(&mut app, KeyCode::Escape);
        wait(&mut app, 0.3);
        key(&mut app, KeyCode::Digit1);
        wait(&mut app, 1.6);
        save(&mut app, out, "mage-talk")?;
        // On through the talk, a shot of her line, up to his question.
        for k in 0..8 {
            if app.mage_asking() {
                break;
            }
            key(&mut app, KeyCode::KeyF);
            wait(&mut app, 2.2);
            if k == 0 {
                save(&mut app, out, "mage-talk-queen")?;
            }
        }
        wait(&mut app, 0.8);
        save(&mut app, out, "mage-question")?;
        // «No, para nada»: his two lines, back to his stall.
        key(&mut app, KeyCode::Digit2);
        wait(&mut app, 1.4);
        save(&mut app, out, "mage-refusing")?;
        for _ in 0..6 {
            if app
                .mage
                .as_ref()
                .is_some_and(|m| m.page == mage::Page::Main)
            {
                break;
            }
            key(&mut app, KeyCode::KeyF);
            wait(&mut app, 1.6);
        }
        wait(&mut app, 1.0);
        save(&mut app, out, "mage-refused")?;
        // Talk again: «Uhm… no demasiado».
        key(&mut app, KeyCode::Digit1);
        wait(&mut app, 0.5);
        for _ in 0..8 {
            if app.mage_asking() {
                break;
            }
            key(&mut app, KeyCode::KeyF);
            wait(&mut app, 2.2);
        }
        wait(&mut app, 0.8);
        key(&mut app, KeyCode::Digit1);
        wait(&mut app, 2.4);
        save(&mut app, out, "mage-ambush-line")?;
        for _ in 0..6 {
            if app
                .mage
                .as_ref()
                .is_some_and(|m| m.page == mage::Page::Ambush)
            {
                break;
            }
            key(&mut app, KeyCode::KeyF);
            wait(&mut app, 0.5);
        }
        for (k, at) in [0.3f32, 0.6, 0.6].iter().enumerate() {
            wait(&mut app, *at);
            save(&mut app, out, &format!("mage-ambush{k}"))?;
        }
        wait(&mut app, 2.0);
        for _ in 0..60 {
            app.tick(1.0 / 30.0);
            app.bed_scene_settle();
            app.elapsed += 1.0 / 30.0;
        }
        save(&mut app, out, "mage-game")?;
        eprintln!("[mage-qa] game open: {}", app.play_scene.is_some());
        return Ok(());
    }
    if std::env::args().any(|a| a == "--burdel-menu") {
        // Rosalía's book page by page: the girls' page (fresh, with the elixir offered, and with
        // it awake), the offer with its content warning, the cursor on the leaflet's button, the
        // leaflet coming in and read, and the way back to the offer.
        app.nature_mode = true;
        app.village_entered = true;
        app.player = village::entrance();
        app.enter_brothel();
        app.queen_thought = None;
        app.gold = 300;
        app.story = guion::Vars::default();
        let desk = brothel::plan()
            .hotspots
            .iter()
            .find(|h| h.id == "recepcionista")
            .map(|h| (h.x, h.z))
            .unwrap();
        if let Some(inside) = app.brothel.as_mut() {
            inside.walker = brothel::Walker {
                x: desk.0,
                z: desk.1 + 0.2,
                level: 0,
                on_stair: false,
            };
            inside.rope_open = true;
            app.player = inside.position();
            app.camera_focus = app.player;
        }
        app.player_yaw = std::f32::consts::PI;
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        wait(&mut app, 1.0);
        app.venue = venue::Menu::default();
        app.open_venue_book();
        wait(&mut app, 1.2);
        save(&mut app, out, "burdel-00-libro")?;
        app.venue_key(KeyCode::ArrowRight);
        app.venue.progress.elixir_offered = true;
        wait(&mut app, 0.3);
        save(&mut app, out, "burdel-01-libro-elixir")?;
        // After a first girl the offer opens the book; left from «Comprar» is the leaflet.
        app.close_shop();
        app.venue.progress.visits = 1;
        app.venue.progress.elixir_offered = false;
        app.open_venue_book();
        wait(&mut app, 1.3);
        save(&mut app, out, "burdel-02-oferta")?;
        app.venue_key(KeyCode::ArrowLeft);
        wait(&mut app, 0.2);
        save(&mut app, out, "burdel-03-oferta-prospecto")?;
        app.venue_key(KeyCode::Enter);
        wait(&mut app, 0.15);
        save(&mut app, out, "burdel-04-prospecto-entra")?;
        wait(&mut app, 1.2);
        save(&mut app, out, "burdel-05-prospecto")?;
        app.venue_key(KeyCode::KeyP);
        wait(&mut app, 0.8);
        save(&mut app, out, "burdel-06-vuelta-oferta")?;
        eprintln!(
            "[burdel-qa] page after the leaflet: {:?}, choice {}",
            app.venue.page, app.venue.choice
        );
        app.venue.progress.elixir_offered = true;
        app.venue.progress.elixir_active = true;
        let now = (app.elapsed * 1000.0) as u32;
        app.venue.show(venue::Page::Girls, now);
        app.venue.selected = 2;
        wait(&mut app, 1.2);
        save(&mut app, out, "burdel-07-libro-despierto")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--elixir") {
        // Rosalía's book and her elixir: the girls and what to ask of Nina before any visit, the
        // offer after the first, the Queen drinking it at the desk (the 3D close shot and its 2D
        // layer), the book with the seal awake, Alba's refusal, then Nina's scene through the
        // Queen's eyes: its questions, its postures, the climax over black and the card.
        app.nature_mode = true;
        app.village_entered = true;
        app.player = village::entrance();
        app.enter_brothel();
        app.queen_thought = None;
        app.gold = 300;
        app.story = guion::Vars::default();
        let desk = brothel::plan()
            .hotspots
            .iter()
            .find(|h| h.id == "recepcionista")
            .map(|h| (h.x, h.z))
            .unwrap();
        if let Some(inside) = app.brothel.as_mut() {
            inside.walker = brothel::Walker {
                x: desk.0,
                z: desk.1 + 0.2,
                level: 0,
                on_stair: false,
            };
            inside.rope_open = true;
            app.player = inside.position();
            app.camera_focus = app.player;
        }
        app.player_yaw = std::f32::consts::PI;
        for _ in 0..30 {
            app.tick(1.0 / 30.0);
            app.render_frame();
        }
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        app.venue = venue::Menu::default();
        app.open_venue_book();
        wait(&mut app, 1.0);
        save(&mut app, out, "elixir-00-girls")?;
        app.venue_key(KeyCode::Enter);
        wait(&mut app, 0.1);
        save(&mut app, out, "elixir-01-ask-entering")?;
        wait(&mut app, 1.0);
        save(&mut app, out, "elixir-02-ask")?;
        // After a first girl, the offer opens the book.
        app.close_shop();
        app.venue.progress.visits = 1;
        app.open_venue_book();
        wait(&mut app, 0.2);
        save(&mut app, out, "elixir-03-offer-entering")?;
        wait(&mut app, 1.2);
        save(&mut app, out, "elixir-04-offer")?;
        // Bought: she drinks it.
        app.venue_key(KeyCode::Enter);
        eprintln!("[elixir-qa] drinking: {}", app.drink.is_some());
        for (name, at) in [
            ("elixir-05-wipe", 0.24),
            ("elixir-06-raise", 1.2),
            ("elixir-07-pop", 1.62),
            ("elixir-08-lips", 2.3),
            ("elixir-09-gulp", 2.98),
            ("elixir-10-ah", 3.8),
            ("elixir-11-awake", 5.25),
            ("elixir-12-ready", 6.8),
        ] {
            while app.drink_clock().is_some_and(|c| c < at - 4.0 / 30.0) {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
            save(&mut app, out, name)?;
        }
        app.drink_key(KeyCode::KeyF, true, false);
        wait(&mut app, 1.0);
        save(&mut app, out, "elixir-13-book-awake")?;
        // Alba's page: her three asks, her game lit.
        app.venue.selected = 1;
        app.venue_key(KeyCode::Enter);
        app.venue.choice = 1;
        wait(&mut app, 1.0);
        save(&mut app, out, "elixir-14-alba-ask")?;
        // Noa's: the seal awake.
        app.venue.back(0);
        app.venue.selected = 2;
        app.venue_key(KeyCode::Enter);
        if let Some(i) = app.venue.asking().iter().position(|a| a.awake) {
            app.venue.choice = i;
        }
        wait(&mut app, 1.0);
        save(&mut app, out, "elixir-14b-noa-awake")?;
        // Nina, the seal awake: she takes the Queen up.
        app.venue.back(0);
        app.venue.selected = 0;
        app.venue_key(KeyCode::Enter);
        if let Some(i) = app.venue.asking().iter().position(|a| a.awake) {
            app.venue.choice = i;
        }
        wait(&mut app, 1.0);
        save(&mut app, out, "elixir-15-nina-awake")?;
        app.venue_key(KeyCode::Enter);
        app.skip_escort();
        for _ in 0..300 {
            if app.talk.is_some() {
                break;
            }
            app.tick(1.0 / 30.0);
        }
        let mut lines = 0;
        while app.talk.is_some() && lines < 12 {
            app.elapsed += 5.0;
            if lines == 0 {
                save(&mut app, out, "elixir-16-guion")?;
            }
            app.talk_confirm();
            lines += 1;
        }
        eprintln!(
            "[elixir-qa] scene open: {}, elixir spent: {}",
            app.bed_scene.is_some(),
            !app.venue.progress.elixir_active
        );
        let run = |app: &mut App, seconds: f32, hold: bool| {
            app.bed_scene_button(hold);
            for _ in 0..(seconds * 30.0) as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
        };
        run(&mut app, 2.2, false);
        save(&mut app, out, "elixir-17-asks-size")?;
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.answer(1);
        }
        run(&mut app, 0.6, false);
        save(&mut app, out, "elixir-18-reply")?;
        // Up to the body: its question, then each posture.
        let part_fast = |app: &App| app.bed_scene.as_ref().is_some_and(|s| s.heat >= 0.55);
        for _ in 0..500 {
            if part_fast(&app) {
                break;
            }
            run(&mut app, 0.1, true);
            if app.bed_scene.as_ref().is_some_and(|s| s.braked) {
                run(&mut app, 0.1, false);
            }
        }
        run(&mut app, 1.6, false);
        save(&mut app, out, "elixir-19-body-asks")?;
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.answer(1);
        }
        let hold_body = |app: &mut App, seconds: f32| {
            let mut left = seconds;
            while left > 0.0 {
                let fast = app.bed_scene.as_ref().is_some_and(|s| s.heat >= 0.6);
                run(app, 0.1, !fast);
                if app.bed_scene.as_ref().is_some_and(|s| s.braked) {
                    run(app, 0.1, false);
                }
                left -= 0.1;
            }
        };
        hold_body(&mut app, 1.4);
        save(&mut app, out, "elixir-20-misionero")?;
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.turn_pose(1);
        }
        hold_body(&mut app, 0.4);
        save(&mut app, out, "elixir-21-pose-fade")?;
        hold_body(&mut app, 1.6);
        save(&mut app, out, "elixir-22-perrito")?;
        for _ in 0..600 {
            if app.bed_scene.as_ref().map_or(true, |s| s.heat >= 1.0) {
                break;
            }
            run(&mut app, 0.1, true);
            if app.bed_scene.as_ref().is_some_and(|s| s.braked) {
                run(&mut app, 0.1, false);
            }
        }
        run(&mut app, 0.6, false);
        save(&mut app, out, "elixir-23-black")?;
        run(&mut app, 2.15, false);
        save(&mut app, out, "elixir-24-black-flash")?;
        run(&mut app, 3.2, false);
        save(&mut app, out, "elixir-25-relax")?;
        run(&mut app, 6.0, false);
        save(&mut app, out, "elixir-26-card")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--brothel-escort") {
        // Rosalía's book, then each girl coming for the Queen at the desk and taking her up.
        app.nature_mode = true;
        app.village_entered = true;
        app.player = village::entrance();
        app.enter_brothel();
        app.queen_thought = None;
        app.gold = app.gold.max(300);
        let plan = brothel::plan();
        let desk = plan
            .hotspots
            .iter()
            .find(|h| h.id == "recepcionista")
            .map(|h| (h.x, h.z))
            .unwrap();
        for (k, id) in ["nina", "alba", "noa"].into_iter().enumerate() {
            if let Some(inside) = app.brothel.as_mut() {
                inside.walker = brothel::Walker {
                    x: desk.0,
                    z: desk.1,
                    level: 0,
                    on_stair: false,
                };
                inside.talk = None;
                inside.rope_open = true;
                inside.escort = None;
                inside.staying = None;
                app.player = inside.position();
                app.camera_focus = app.player;
            }
            app.talk = None;
            app.player_yaw = std::f32::consts::PI;
            app.open_shop();
            app.venue.open = true;
            app.venue.select_tab(0);
            app.venue.selected = k;
            if k == 0 {
                save(&mut app, out, "escort-00-book")?;
                app.venue.selected = 1;
                save(&mut app, out, "escort-01-book-alba")?;
                app.venue.selected = 2;
                app.venue.talk();
                save(&mut app, out, "escort-02-book-noa-talk")?;
                app.venue.selected = 0;
                app.venue.notice.clear();
            }
            app.venue.act(&mut app.gold);
            app.after_venue_act();
            let mut shots: Vec<&str> = vec!["rise", "come", "greet", "stair", "corridor", "room"];
            let mut t = 0.0f32;
            let mut greet_at = None;
            while t < 70.0 && !shots.is_empty() {
                app.tick(1.0 / 30.0);
                t += 1.0 / 30.0;
                let (phase, walker) = match app.brothel.as_ref() {
                    Some(b) => (b.escort_phase(), b.walker),
                    None => (
                        None,
                        brothel::Walker {
                            x: 0.0,
                            z: 0.0,
                            level: 0,
                            on_stair: false,
                        },
                    ),
                };
                let due = match shots[0] {
                    "rise" => t >= 0.55,
                    "come" => t >= 2.4 && phase == Some("approach"),
                    "greet" => {
                        if phase == Some("greet") && greet_at.is_none() {
                            greet_at = Some(t);
                        }
                        greet_at.is_some_and(|g| t >= g + 0.5) || phase == Some("lead")
                    }
                    "stair" => walker.on_stair && plan.stair_t(walker.x) > 0.45,
                    "corridor" => walker.level == 1 && !walker.on_stair && walker.z > -1.4,
                    _ => app.talk.is_some(),
                };
                if due {
                    let name = format!("escort-{id}-{}-{}", 3 + 6 - shots.len(), shots[0]);
                    if shots[0] == "room" {
                        for _ in 0..30 {
                            app.tick(1.0 / 30.0);
                        }
                        app.elapsed += 3.0;
                    }
                    save(&mut app, out, &name)?;
                    // Close up: the girl standing in her room; and, once she has left it, the
                    // bar where Nina put her tray down.
                    let girl_at = app
                        .brothel
                        .as_ref()
                        .and_then(|b| b.girl_looks().first().map(|l| l.at));
                    if let (Some(at), "room" | "greet") = (girl_at, shots[0]) {
                        let (focus, distance) = (app.camera_focus, app.iso.distance);
                        if shots[0] == "room" {
                            app.camera_focus = at;
                            app.iso.distance = 3.2;
                            let talk = app.talk.take();
                            save(&mut app, out, &format!("escort-{id}-closeup"))?;
                            app.talk = talk;
                        } else if id == "nina" {
                            let home = plan.npcs.iter().find(|n| n.id == "nina").unwrap();
                            app.camera_focus = Vec3::new(home.x, 0.0, home.z + 0.4);
                            app.iso.distance = 3.4;
                            save(&mut app, out, "escort-nina-bar-tray")?;
                        }
                        app.camera_focus = focus;
                        app.iso.distance = distance;
                    }
                    shots.remove(0);
                    if id != "nina" && shots.first() == Some(&"greet") {
                        // For the others, rising and walking are enough besides the room.
                        shots.retain(|s| *s == "room");
                    }
                }
            }
            eprintln!("[escort-qa] {id}: left {shots:?} after {t:.1} s");
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--brothel-only") {
        // The brothel: walk in from the villa and look round both floors.
        app.nature_mode = true;
        app.village_entered = true;
        app.player = village::entrance();
        app.enter_brothel();
        app.queen_thought = None;
        let plan = brothel::plan();
        let spot = |id: &str| {
            plan.hotspots
                .iter()
                .find(|h| h.id == id)
                .map(|h| (h.x, h.z, h.level))
                .unwrap()
        };
        let stand = |app: &mut App, x: f32, z: f32, level: u8| {
            if let Some(inside) = app.brothel.as_mut() {
                inside.walker = brothel::Walker {
                    x,
                    z,
                    level,
                    on_stair: false,
                };
                inside.talk = None;
                app.player = inside.position();
                app.camera_focus = app.player;
            }
        };
        save(&mut app, out, "brothel-00-entrance")?;
        for (name, id) in [
            ("brothel-01-desk", "recepcionista"),
            ("brothel-02-salon", "alba"),
            ("brothel-03-bar", "servicio"),
            ("brothel-04-waiting", "noa"),
            ("brothel-05-stair", "escalera"),
        ] {
            let (x, z, level) = spot(id);
            stand(&mut app, x, z, level);
            save(&mut app, out, name)?;
        }
        // A line from Alba, the rope's refusal, then Rosalía's book.
        let (x, z, level) = spot("alba");
        stand(&mut app, x, z, level);
        if let Some(inside) = app.brothel.as_mut() {
            inside.interact();
        }
        save(&mut app, out, "brothel-06-talk")?;
        let (x, z, level) = spot("recepcionista");
        stand(&mut app, x, z, level);
        if let Some(inside) = app.brothel.as_mut() {
            inside.interact();
        }
        app.open_shop();
        app.venue.open = true;
        app.venue.select_tab(0);
        save(&mut app, out, "brothel-07-rosalia-book")?;
        app.close_shop();
        // Up the stair to the rooms.
        app.iso.distance = 11.0;
        let lane = (plan.stair.z0 + plan.stair.z1) * 0.5;
        stand(&mut app, plan.stair.x1 + 0.6, lane, 0);
        if let Some(inside) = app.brothel.as_mut() {
            for _ in 0..70 {
                inside.walk(-0.05, 0.0);
            }
            app.player = inside.position();
            app.camera_focus = app.player;
        }
        save(&mut app, out, "brothel-08-on-the-stair")?;
        let (x, z, _) = spot("hab2");
        stand(&mut app, x, z, 1);
        save(&mut app, out, "brothel-09-corridor")?;
        if let Some(inside) = app.brothel.as_mut() {
            for _ in 0..40 {
                inside.walk(0.0, -0.05);
                inside.update(1.0 / 30.0);
            }
            app.player = inside.position();
            app.camera_focus = app.player;
        }
        save(&mut app, out, "brothel-10-room")?;
        let (x, z, _) = spot("hab3");
        stand(&mut app, x, z, 1);
        if let Some(inside) = app.brothel.as_mut() {
            inside.interact();
        }
        save(&mut app, out, "brothel-11-locked")?;
        app.map_open = true;
        save(&mut app, out, "brothel-12-map-upstairs")?;
        // The full map downstairs, before Rosalía has lifted the rope.
        let (x, z, level) = spot("tablon");
        stand(&mut app, x, z, level);
        if let Some(inside) = app.brothel.as_mut() {
            inside.rope_open = false;
            inside.talk = None;
        }
        save(&mut app, out, "brothel-13-map-ground")?;
        app.map_open = false;
        save(&mut app, out, "brothel-14-minimap-ground")?;
        // The fireplace close up, a few moments apart (the fire moves).
        if let Some(fire) = plan.lamps.iter().find(|l| l.fire) {
            stand(&mut app, fire.x + 0.6, fire.z + 1.6, 0);
            app.iso.distance = 4.2;
            for k in 0..3 {
                app.elapsed += 0.37;
                save(&mut app, out, &format!("brothel-15-fire-{k}"))?;
            }
        }
        // Each girl close up, two moments apart (her idle moves her hands).
        for npc in plan
            .npcs
            .iter()
            .filter(|n| n.model.is_some() && n.level == 0)
        {
            stand(&mut app, npc.x + 0.8, npc.z - 0.8, npc.level);
            app.iso.distance = 2.6;
            for k in 0..2 {
                app.elapsed += 2.3;
                save(&mut app, out, &format!("brothel-16-{}-{k}", npc.id))?;
            }
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--prompts-only") {
        // Every F prompt and the speech bubbles, each from within reach.
        app.iso.distance = 9.0;
        let near = |anchor: Vec3, away: Vec3| anchor + away * 1.8;
        // Castle interior: the wardrobe and the way out, each from a free spot beside it.
        app.nature_mode = false;
        app.village_entered = false;
        if let Some(state) = app.state.as_mut() {
            state.ensure_castle_loaded();
        }
        for (name, id) in [
            ("prompt-wardrobe", "vestidor"),
            ("prompt-castle-exit", "porton"),
        ] {
            let plan = castle3d::plan();
            let spot = plan
                .hotspots
                .iter()
                .find(|h| h.id == id)
                .expect("castle hotspot");
            let free = (1..=8)
                .flat_map(|ring| {
                    (0..12)
                        .map(move |k| (ring as f32 * 0.3, k as f32 * std::f32::consts::TAU / 12.0))
                })
                .map(|(r, a)| (spot.x + r * a.cos(), spot.z + r * a.sin()))
                .find(|&(x, z)| !plan.blocked(x, z, 0.3, spot.level))
                .unwrap_or((spot.x, spot.z));
            let inside = castle3d::State::restore(Vec3::new(free.0, 0.0, free.1));
            app.player = inside.position();
            app.camera_focus = inside.focus();
            app.castle = Some(inside);
            save(&mut app, out, name)?;
        }
        app.castle = None;
        // Outdoors: the mage (with a line), the castle gate, the mine, the villa.
        app.nature_mode = true;
        let spoken = (0..600)
            .map(|k| k as f32 * 0.25)
            .find(|&t| active_npc_comment(t).is_some_and(|(_, alpha)| alpha > 0.95));
        app.elapsed = spoken.unwrap_or(1.0);
        for (name, anchor) in [
            ("prompt-mage", mage_position()),
            ("prompt-castle-entry", castle_entry_position()),
            ("prompt-mine", mine::entry_position()),
            ("prompt-village-entry", village_entry_position()),
        ] {
            app.player = near(anchor, Vec3::new(0.6, 0.0, 0.8));
            app.camera_focus = app.player;
            save(&mut app, out, name)?;
        }
        // Without a weapon the Queen will not go down: her thought over her head.
        app.player = near(mine::entry_position(), Vec3::new(0.6, 0.0, 0.8));
        app.camera_focus = app.player;
        app.queen_thought = Some(("Necesito algo para defenderme primero.", app.elapsed + 30.0));
        save(&mut app, out, "gate-no-weapon")?;
        app.queen_thought = None;
        // Inside the villa: the shop, the tavern, the baths and the way back.
        app.village_entered = true;
        for id in ["villa_tienda", "villa_taberna", "villa_banos_termales"] {
            if let Some(anchor) = village::building_anchors().iter().find(|a| a.id == id) {
                if let Some((door, out_dir)) = village::door_anchor(anchor.footprint_index) {
                    app.player = door + out_dir * 1.4;
                    app.camera_focus = app.player;
                    save(&mut app, out, &format!("prompt-{id}"))?;
                }
            }
        }
        app.player = village::entrance() + Vec3::new(0.0, 0.0, -1.2);
        app.camera_focus = app.player;
        save(&mut app, out, "prompt-village-exit")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--maps-only") {
        // The corner minimap and the expanded map (M) of each place.
        let places: [(&str, bool, bool, Vec3); 5] = [
            ("castle", false, false, app.player),
            // (upstairs: her height picks the castle's floor on the map)
            (
                "castle-up",
                false,
                false,
                Vec3::new(2.0, castle3d::plan().level_h, 13.6),
            ),
            (
                "forest",
                true,
                false,
                Vec3::new(nature_trail_center(-2.0), 0.0, -2.0),
            ),
            (
                "village",
                true,
                true,
                village::entrance() + Vec3::new(0.0, 0.0, -6.0),
            ),
            ("baths", true, true, Vec3::new(-1.2, 0.0, -4.0)),
        ];
        for (name, nature, village_in, at) in places {
            app.nature_mode = nature;
            app.village_entered = village_in;
            app.baths = (name == "baths").then(|| baths::State::enter(Vec3::ZERO, 0.0));
            app.player = at;
            app.camera_focus = app.player;
            app.map_open = false;
            save(&mut app, out, &format!("map-mini-{name}"))?;
            app.map_open = true;
            save(&mut app, out, &format!("map-full-{name}"))?;
        }
        app.baths = None;
        // The wardrobe, with the green dress worn and the crimson one picked.
        app.nature_mode = false;
        app.village_entered = false;
        app.map_open = false;
        app.wardrobe_open = true;
        app.outfit = 1;
        for (name, selection) in [("wardrobe-a", 1u8), ("wardrobe-b", 0u8)] {
            app.wardrobe_selection = selection;
            save(&mut app, out, name)?;
        }
        app.wardrobe_open = false;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--shop-favor") {
        // «Otro trato»: short of gold, he hints at other ways to pay and the favour's tile comes up;
        // lit, his ask; played on the stage, sealed, its card; and back in the shop with its cut.
        // Then a day later with three done: the new one and the old ones to choose from.
        app.nature_mode = true;
        app.village_entered = true;
        app.shop_open = true;
        app.shop_opened_at = app.elapsed - 10.0;
        app.shop_tab = shop::Tab::Buy;
        app.shop_category = shop::Category::All;
        app.shop_selection = 0;
        app.story = guion::Vars::default();
        app.gold = 10;
        app.start_haggle();
        app.shop_page_at = app.elapsed - 10.0;
        if let Some(h) = app.haggle.as_mut() {
            h.offer_at = app.elapsed - 2.0;
        }
        save(&mut app, out, "favor-offer")?;
        app.haggle_key(KeyCode::KeyW);
        save(&mut app, out, "favor-lit")?;
        let step = |app: &mut App, seconds: f32| {
            for _ in 0..(seconds * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
        };
        app.haggle_key(KeyCode::Digit6);
        eprintln!("[favor-qa] scene open: {}", app.play_scene.is_some());
        step(&mut app, 3.0);
        save(&mut app, out, "favor-scene")?;
        app.bed_scene_key(KeyCode::Space, true, false);
        for _ in 0..400 {
            if app.play_scene_qa().is_some_and(|q| q.ended) {
                break;
            }
            step(&mut app, 0.1);
        }
        app.bed_scene_key(KeyCode::Space, false, false);
        step(&mut app, 1.6);
        save(&mut app, out, "favor-card")?;
        app.bed_scene_key(KeyCode::Enter, true, false);
        app.bed_scene_key(KeyCode::Enter, false, false);
        step(&mut app, 0.3);
        eprintln!(
            "[favor-qa] back: scene {} haggle {:?} ladder {} today {}",
            app.play_scene.is_some(),
            app.haggle
                .as_ref()
                .map(|h| (h.percent, h.price(), h.favor_done)),
            app.story.get(play_scene::tienda::LADDER),
            app.story.get(play_scene::tienda::TODAY),
        );
        save(&mut app, out, "favor-back")?;
        // The next day, three done: the fourth is new, the three old ones can be picked again.
        app.haggle = None;
        app.story
            .values
            .insert(play_scene::tienda::LADDER.to_owned(), 3);
        app.story
            .values
            .insert(play_scene::tienda::TODAY.to_owned(), 0);
        app.gold = 500;
        app.start_haggle();
        app.shop_page_at = app.elapsed - 10.0;
        if let Some(h) = app.haggle.as_mut() {
            h.offer_at = app.elapsed - 2.0;
        }
        app.haggle_key(KeyCode::KeyW);
        save(&mut app, out, "favor-new")?;
        app.haggle_key(KeyCode::KeyA);
        save(&mut app, out, "favor-again")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--shop-only") {
        // The shop's detail pane for a handful of items: models and icons.
        app.nature_mode = true;
        app.village_entered = true;
        app.shop_open = true;
        for id in [
            "pocion_pequena",
            "espada_de_madera",
            "anillo_misterioso",
            "tela_resistente",
            "llave_boveda",
            "cristal_memoria",
            "pocion_misteriosa",
            "tinta_runica",
            "hueso_viejo",
            "amuleto_ocultacion",
        ] {
            app.shop_tab = shop::Tab::Buy;
            app.shop_category = shop::Category::All;
            let visible = app.shop_product_indices();
            if let Some(pos) = visible
                .iter()
                .position(|i| shop::catalog().products[*i].id == id)
            {
                app.shop_selection = pos;
            } else {
                app.shop_tab = shop::Tab::Sell;
                app.inventory.insert(id.to_string(), 3);
                let visible = app.shop_product_indices();
                app.shop_selection = visible
                    .iter()
                    .position(|i| shop::catalog().products[*i].id == id)
                    .unwrap_or(0);
            }
            save(&mut app, out, &format!("shop-{id}"))?;
        }
        // Long open: the entrances are over in the pictures.
        app.shop_opened_at = app.elapsed - 10.0;
        app.shop_page_at = app.elapsed - 10.0;
        app.shop_tab = shop::Tab::Buy;
        app.shop_category = shop::Category::All;
        app.shop_selection = 0;
        save(&mut app, out, "shop-shelves")?;
        // A haggle under way: two good tries, one patience lost, a try just stamped.
        app.start_haggle();
        app.shop_page_at = app.elapsed - 10.0;
        if let Some(h) = app.haggle.as_mut() {
            h.percent = 15;
            h.patience = 2;
            h.rounds = 3;
            h.tactic = 1;
            h.needle_from = app.elapsed - 0.4;
            h.result = Some((shop_games::Outcome::Good, app.elapsed - 0.3));
        }
        app.shop_said = Some((
            "—Puedo hacer un esfuerzo, Majestad.".to_owned(),
            app.elapsed - 2.0,
        ));
        save(&mut app, out, "shop-haggle")?;
        // The altar: herbs and a potion in its sockets, and the same mid-fusion.
        app.haggle = None;
        app.inventory.insert("hierbas_curativas".to_owned(), 3);
        app.inventory.insert("pocion_pequena".to_owned(), 2);
        app.gold = app.gold.max(100);
        app.open_fusion();
        app.shop_page_at = app.elapsed - 10.0;
        app.fuse.slots = [
            Some("hierbas_curativas".to_owned()),
            Some("pocion_pequena".to_owned()),
        ];
        save(&mut app, out, "shop-fuse")?;
        app.fuse_go();
        if let Some((at, _)) = app.fuse.fusing.as_mut() {
            *at = app.elapsed - 0.45;
        }
        save(&mut app, out, "shop-fuse-spiral")?;
        if let Some((at, _)) = app.fuse.fusing.as_mut() {
            *at = app.elapsed - 1.1;
        }
        save(&mut app, out, "shop-fuse-burst")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--forest-only") {
        // The dark forest on its own: the trail from the castle gate, the
        // middle of the wood, the clearing and the mine, by day and by night.
        app.nature_mode = true;
        app.village_entered = false;
        let stops: [(&str, Vec3, f32); 7] = [
            (
                "forest-gate",
                Vec3::new(nature_trail_center(-11.0), 0.0, -11.0),
                16.0,
            ),
            (
                "forest-castle",
                Vec3::new(nature_trail_center(-10.5), 0.0, -10.5),
                24.0,
            ),
            (
                "forest-middle",
                Vec3::new(nature_trail_center(-2.0), 0.0, -2.0),
                18.0,
            ),
            (
                "forest-mage",
                mage_position() + Vec3::new(1.6, 0.0, 1.2),
                12.0,
            ),
            (
                "forest-mine",
                mine::entry_position() - Vec3::new(1.0, 0.0, 0.4),
                14.0,
            ),
            (
                "forest-village",
                Vec3::new(nature_trail_center(17.0), 0.0, 17.0),
                18.0,
            ),
            (
                "forest-close",
                Vec3::new(nature_trail_center(4.0), 0.0, 4.0),
                9.0,
            ),
        ];
        for (phase, tag) in [(0.5f32, "day"), (0.74, "dusk"), (0.0, "night")] {
            app.environment_cycle.set_phase(phase);
            for (name, at, distance) in stops {
                app.player = at;
                app.camera_focus = app.player;
                app.iso.distance = distance;
                save(&mut app, out, &format!("{name}-{tag}"))?;
            }
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--water-only") {
        // The simulated water: let it run a few seconds at each place, then look.
        let run = |app: &mut App, seconds: f32| {
            for _ in 0..(seconds * 60.0) as usize {
                app.elapsed += 1.0 / 60.0;
                app.render_frame();
            }
        };
        let center = fantasy_village_center();
        let villa = |x: f32, z: f32| Vec3::new(x + center.x, 0.0, z + center.z);
        let stops: [(&str, bool, Vec3, f32); 8] = [
            ("water-spout", true, villa(16.6, 16.9), 7.0),
            ("water-mill", true, villa(7.0, 16.2), 9.0),
            ("water-brook", true, villa(-4.0, 16.6), 12.0),
            ("water-pools", true, villa(18.0, 10.5), 9.0),
            ("water-well", true, villa(0.0, 1.75), 6.0),
            ("water-forest-pool", false, Vec3::new(-8.6, 0.0, 10.4), 9.0),
            ("water-wade-pool", false, Vec3::new(-8.2, 0.0, 13.0), 8.0),
            ("water-wade-brook", true, villa(-7.6, 17.4), 8.0),
        ];
        for (phase, tag) in [(0.5f32, "day"), (0.0, "night")] {
            app.environment_cycle.set_phase(phase);
            for (name, in_villa, at, distance) in stops {
                app.nature_mode = true;
                app.village_entered = in_villa;
                app.player = at;
                app.camera_focus = at;
                app.iso.distance = distance;
                run(&mut app, 4.0);
                save(&mut app, out, &format!("{name}-{tag}"))?;
            }
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--castle-only") {
        // The castle from the gate, the camera turned all the way round it, day and night.
        app.nature_mode = true;
        app.village_entered = false;
        for (phase, tag) in [(0.5f32, "day"), (0.0, "night")] {
            app.environment_cycle.set_phase(phase);
            for k in 0..4 {
                let yaw = std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::FRAC_PI_2;
                app.iso.yaw = yaw;
                app.iso.yaw_target = yaw;
                app.player = Vec3::new(nature_trail_center(-11.0), 0.0, -11.0);
                app.camera_focus = app.player;
                app.iso.distance = 18.0;
                save(&mut app, out, &format!("castle-{k}-{tag}"))?;
            }
        }
        app.iso.yaw = std::f32::consts::FRAC_PI_4;
        app.iso.yaw_target = app.iso.yaw;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--achievements-only") {
        // The achievements screen as it is (the shipped catalog), then with a sample of what it
        // will hold, and the wardrobe with a locked outfit.
        app.nature_mode = true;
        app.village_entered = true;
        app.achievements.record("visit:bosque");
        app.achievements.record("visit:villa");
        app.achievements.record("visit:termas");
        app.achievements_open = true;
        save(&mut app, out, "achievements-shipped")?;
        let sample = map_overlay::AchievementsView {
            rows: vec![
                map_overlay::AchievementRow {
                    title: "Agua caliente".into(),
                    detail: "Visita los baños termales.".into(),
                    reward: "Atuendo: Bikini · +50 oro".into(),
                    done: true,
                    icon: Some(Icon::Bath),
                },
                map_overlay::AchievementRow {
                    title: "Viajera".into(),
                    detail: "Visita cinco lugares distintos.".into(),
                    reward: "Poción pequeña ×2".into(),
                    done: false,
                    icon: Some(Icon::Forest),
                },
                map_overlay::AchievementRow {
                    title: "Clienta fiel".into(),
                    detail: "Compra diez cosas en la tienda.".into(),
                    reward: "+100 oro".into(),
                    done: false,
                    icon: Some(Icon::Shop),
                },
                map_overlay::AchievementRow {
                    title: "???".into(),
                    detail: "Logro oculto".into(),
                    reward: String::new(),
                    done: false,
                    icon: None,
                },
            ],
            stats: vec![
                ("Lugares visitados".into(), "3".into()),
                ("Compras".into(), "0".into()),
                ("Oro".into(), "120".into()),
            ],
        };
        app.achievements_preview = Some(sample);
        save(&mut app, out, "achievements-sample")?;
        app.achievements_preview = None;
        app.achievements_open = false;
        app.wardrobe_open = true;
        app.wardrobe_selection = 1;
        save(&mut app, out, "wardrobe")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--qa-bot") {
        return super::qa_bot::run(&mut app, out);
    }
    if std::env::args().any(|a| a == "--baths-relax") {
        // The Queen soaking in the central pool, then on a lounger; both outfits.
        app.nature_mode = true;
        app.village_entered = true;
        app.environment_cycle.set_phase(0.55);
        for outfit in 0..wardrobe_outfit_count() {
            app.outfit = outfit;
            let pool = &baths::plan().pools[1];
            let mut state = baths::State::enter(Vec3::ZERO, 0.0);
            (state.x, state.z) = (pool.x + 1.2, pool.z + 0.6);
            app.baths = Some(state);
            app.iso.distance = 7.0;
            for _ in 0..240 {
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
            save(&mut app, out, &format!("relax-soak-{outfit}"))?;
            // Walk off to a lounger and lie down (at eight, before the girls take them).
            app.environment_cycle.set_phase(8.0 / 24.0);
            for _ in 0..30 {
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
            if let Some(&(x, z, ..)) = baths::loungers().first() {
                if let Some(b) = app.baths.as_mut() {
                    b.relax = baths::Relax::None;
                    b.relax_blend = 0.0;
                    (b.x, b.z) = (x + 0.9, z + 0.2);
                    let _ = b.interact();
                }
                for _ in 0..90 {
                    app.elapsed += 1.0 / 60.0;
                    app.tick(1.0 / 60.0);
                    app.render_frame();
                }
                save(&mut app, out, &format!("relax-lounger-{outfit}"))?;
            }
            app.environment_cycle.set_phase(0.55);
        }
        app.outfit = 0;
        app.baths = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--baths-day") {
        // The spa girls' day, in real time from a few moments: the first ones coming in through the
        // gate, the change of spots after lunch, the evening leaving. Hours run at the game's pace.
        app.nature_mode = true;
        app.village_entered = true;
        for (tag, hour, x, z, distance, seconds) in [
            ("arrive", 5.95f32, -5.0, 5.0, 24.0, 40.0),
            ("noon", 12.9, 6.0, -1.0, 26.0, 40.0),
            ("evening", 21.3, -3.0, 2.0, 26.0, 40.0),
        ] {
            app.environment_cycle.set_phase(hour / 24.0);
            let mut state = baths::State::enter(Vec3::ZERO, 0.0);
            // She stands off to one side, out of their way.
            (state.x, state.z) = (-11.0, 4.0);
            state.hour = hour;
            app.baths = Some(state);
            app.player = Vec3::new(x, 0.0, z);
            app.camera_focus = app.player;
            app.iso.distance = distance;
            let frames = (seconds * 60.0) as usize;
            for f in 0..frames {
                app.elapsed += 1.0 / 60.0;
                app.environment_cycle.advance(1.0 / 60.0);
                if let Some(b) = app.baths.as_mut() {
                    b.hour = app.environment_cycle.phase() * 24.0;
                }
                app.render_frame();
                if f % 600 == 599 {
                    save(&mut app, out, &format!("spa-{tag}-{:02}", f / 600))?;
                    let report = app
                        .state
                        .as_ref()
                        .and_then(|s| s.baths_scene.as_ref())
                        .and_then(|b| b.life())
                        .map(|l| l.report())
                        .unwrap_or_default();
                    eprintln!(
                        "[spa] {tag} {:.2}h {:?}",
                        app.environment_cycle.phase() * 24.0,
                        report
                    );
                    // Close on someone walking (or else settling, rising, swimming).
                    let pick = ["walking", "settling", "rising", "swimming", "settled"]
                        .iter()
                        .find_map(|want| report.iter().find(|(_, phase, _)| phase == want));
                    if let Some((who, phase, at)) = pick {
                        let (focus, distance) = (app.camera_focus, app.iso.distance);
                        app.camera_focus = Vec3::new(at[0], app.player.y, at[1]);
                        app.iso.distance = 6.0;
                        app.render_frame();
                        app.capture(
                            &out.join(format!("spa-{tag}-{:02}-close-{who}-{phase}.png", f / 600)),
                        )?;
                        app.camera_focus = focus;
                        app.iso.distance = distance;
                    }
                }
            }
        }
        app.baths = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--bake-maps") {
        bake_maps(&mut app, out)?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--new-places") {
        // The places added for the first act, close and by day: the castle's courtyard (the arena,
        // the targets, the well) and the villa's corral.
        app.environment_cycle.set_phase(0.45);
        app.nature_mode = false;
        app.village_entered = false;
        for (name, x, z) in [
            ("patio-arena", -12.0f32, 29.4f32),
            ("patio-dianas", 15.4, 27.4),
            ("patio-pozo", 6.4, 30.2),
        ] {
            app.player = Vec3::new(x, 0.0, z);
            app.castle = None;
            app.iso.yaw = std::f32::consts::FRAC_PI_4;
            app.iso.yaw_target = app.iso.yaw;
            app.iso.distance = 14.0;
            for _ in 0..90 {
                app.tick(1.0 / 60.0);
                if let Some(inside) = &app.castle {
                    app.camera_focus = inside.focus();
                }
                app.render_frame();
            }
            save(&mut app, out, name)?;
        }
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = true;
        if let Some((at, _)) = village::spot("corral") {
            for (name, dz, distance) in [
                ("villa-corral", 2.4f32, 13.0f32),
                ("villa-corral-cerca", 1.2, 8.0),
            ] {
                app.player = at + Vec3::new(0.0, 0.0, dz);
                app.camera_focus = app.player;
                app.iso.distance = distance;
                for _ in 0..30 {
                    app.tick(1.0 / 60.0);
                    app.camera_focus = app.player;
                    app.render_frame();
                }
                save(&mut app, out, name)?;
            }
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--castle-inside") {
        // The castle from its prototype: every room, with the Queen standing in it, by day and night.
        app.nature_mode = false;
        app.village_entered = false;
        // (name, x, height, z): the height picks the floor (and puts her on the stair).
        let up = castle3d::plan().level_h;
        let stops: [(&str, f32, f32, f32); 17] = [
            ("vestibulo", 0.0, 0.0, 18.6),
            ("salon", -1.6, 0.0, 3.6),
            ("trono", 1.4, 0.0, -15.0),
            ("dormitorio", -17.6, 0.0, -12.6),
            ("balcon", -20.0, 0.0, -25.6),
            ("guerra", 18.0, 0.0, -10.6),
            ("biblioteca", -16.0, 0.0, 3.6),
            ("sello", -15.6, 0.0, 15.0),
            ("comedor", 20.0, 0.0, 0.4),
            ("servicio", 16.6, 0.0, 14.6),
            ("escalera", -8.5, up * 0.5, 15.5),
            ("galeria", 2.0, up, 13.6),
            ("enfermeria", -15.6, up, 12.0),
            ("almacen", -17.0, up, 2.0),
            ("armeria", 17.4, up, 11.0),
            ("barracones", 17.0, up, 18.8),
            ("trono-cerca", 0.0, 0.0, -16.6),
        ];
        for (phase, tag) in [(0.5f32, "day"), (0.0, "night")] {
            app.environment_cycle.set_phase(phase);
            for (name, x, y, z) in stops {
                app.player = Vec3::new(x, y, z);
                app.castle = None;
                app.iso.yaw = std::f32::consts::FRAC_PI_4;
                app.iso.yaw_target = app.iso.yaw;
                app.iso.distance = if name.ends_with("cerca") {
                    9.0
                } else {
                    ISO.max_distance
                };
                for _ in 0..90 {
                    app.tick(1.0 / 60.0);
                    if let Some(inside) = &app.castle {
                        app.camera_focus = if name.ends_with("cerca") {
                            inside.position()
                        } else {
                            inside.focus()
                        };
                    }
                    app.render_frame();
                }
                save(&mut app, out, &format!("castle-{name}-{tag}"))?;
            }
        }
        // A conversation in the box: Lira's line typing out, then her question.
        app.environment_cycle.set_phase(0.5);
        app.player = Vec3::new(20.0, 0.0, 2.8);
        app.castle = None;
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        app.story = guion::Vars::default();
        app.story.seen.insert("castillo.lira_primera".into());
        app.story.seen.insert("castillo.nemi_primera".into());
        if app.start_talk(Some("lira"), None) {
            for _ in 0..12 {
                app.elapsed += 1.0 / 60.0;
                app.render_frame();
            }
            save(&mut app, out, "castle-talk-typing")?;
            app.elapsed += 3.0;
            save(&mut app, out, "castle-talk-line")?;
            app.talk_confirm();
            app.talk_move(1);
            save(&mut app, out, "castle-talk-choice")?;
            app.end_talk();
        }
        app.castle = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--penascal") {
        // El Peñascal: the way in, the plaza with Varrak on his stone (and kneeling, awake), the
        // quarters round it, by day and by night; its map, small and full.
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.story = guion::Vars::default();
        app.enter_penascal_from_forest();
        anyhow::ensure!(app.penascal.is_some(), "el Peñascal did not load");
        let stops: [(&str, f32, f32); 8] = [
            ("paso", 0.0, 24.0),
            ("plaza", 1.5, 6.5),
            ("plaza-piedra", -2.0, 4.6),
            ("barrio", -22.0, 6.0),
            ("corrales", 21.0, 5.0),
            ("guarida", -1.0, -14.0),
            ("pozo", -35.0, 6.0),
            ("cubil", 33.5, 8.0),
        ];
        for (phase, tag) in [(0.5f32, "day"), (0.0, "night")] {
            app.environment_cycle.set_phase(phase);
            for (name, x, z) in stops {
                app.penascal = Some(penascal3d::State::restore(Vec3::new(x, 0.0, z)));
                if let Some(inside) = &app.penascal {
                    app.player = inside.position();
                    app.camera_focus = inside.focus();
                }
                app.iso.yaw = std::f32::consts::FRAC_PI_4;
                app.iso.yaw_target = app.iso.yaw;
                app.iso.distance = ISO.max_distance;
                for _ in 0..90 {
                    app.tick(1.0 / 60.0);
                    app.render_frame();
                }
                save(&mut app, out, &format!("penascal-{name}-{tag}"))?;
            }
        }
        // Varrak awake, on his knees by the stone; the map, small and full.
        app.environment_cycle.set_phase(0.5);
        app.story.values.insert("varrak_despierto".into(), 1);
        app.penascal = Some(penascal3d::State::restore(Vec3::new(1.5, 0.0, 6.5)));
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        save(&mut app, out, "penascal-varrak-despierto")?;
        app.map_open = true;
        for _ in 0..4 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        save(&mut app, out, "penascal-mapa")?;
        app.map_open = false;
        app.penascal = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--colmillo") {
        // «El Colmillo» from end to end: Varrak on his stone, the challenge, his arrival told, the
        // fight (won), his waking told by the stone, and the missions after it.
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.story = guion::Vars::default();
        for v in [
            "consejo_1_hecho",
            "lamento_visto",
            "corral_pedido",
            "gorou_cabeza_alta",
            "varrak_visto",
            "consejo_2_hecho",
        ] {
            app.story.values.insert(v.into(), 1);
        }
        app.environment_cycle.set_phase(0.8);
        app.enter_penascal_from_forest();
        app.penascal = Some(penascal3d::State::restore(Vec3::new(0.6, 0.0, 4.6)));
        fn run(app: &mut App, secs: f32) {
            for _ in 0..(secs * 60.0).max(1.0) as usize {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        }
        run(&mut app, 1.5);
        save(&mut app, out, "colmillo-01-varrak")?;
        app.interact_penascal();
        run(&mut app, 2.0);
        save(&mut app, out, "colmillo-02-llega")?;
        app.end_talk();
        run(&mut app, 0.3);
        anyhow::ensure!(
            app.boss.is_some(),
            "the fight did not start after his arrival"
        );
        save(&mut app, out, "colmillo-03-card")?;
        app.boss_input.attack = true;
        run(&mut app, 0.2);
        if let Some(f) = app.boss.as_mut() {
            f.qa("won");
        }
        app.boss_input.channel = true;
        for _ in 0..(60 * 12) {
            if let Some(f) = app.boss.as_mut() {
                f.qa_look();
            }
            run(&mut app, 1.0 / 60.0);
            if app.boss.as_ref().is_some_and(|f| f.card) {
                break;
            }
        }
        app.boss_input.channel = false;
        save(&mut app, out, "colmillo-04-won")?;
        app.boss_input.attack = true;
        run(&mut app, 1.0);
        anyhow::ensure!(app.boss.is_none(), "the fight did not end");
        save(&mut app, out, "colmillo-05-final")?;
        for _ in 0..200 {
            if app.talk.is_none() {
                break;
            }
            app.talk_confirm();
            run(&mut app, 0.05);
        }
        run(&mut app, 1.0);
        save(&mut app, out, "colmillo-06-despierto")?;
        let open: Vec<String> = missions::open(&app.story)
            .iter()
            .map(|(m, _)| m.title.clone())
            .collect();
        eprintln!(
            "[toon-qa] colmillo: varrak_despierto={} acto1_fin={} open={open:?}",
            app.story.get("varrak_despierto"),
            app.story.get("acto1_fin")
        );
        run(&mut app, 4.0);
        save(&mut app, out, "colmillo-07-ajedrez")?;
        app.penascal = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--boss") {
        // The fight with Varrak: the card, the pack at his roar, him rising with his cleaver, the
        // duel, the smash and its wave, on all fours, a spire broken by his head, the spark, a
        // memory, his kneeling and the card; and the Queen falling.
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.story = guion::Vars::default();
        app.story.values.insert("consejo_2_hecho".into(), 1);
        app.enter_penascal_from_forest();
        app.start_boss_fight();
        anyhow::ensure!(app.boss.is_some(), "the fight did not load");
        fn run(app: &mut App, secs: f32, look: bool) {
            for _ in 0..(secs * 60.0) as usize {
                if look {
                    if let Some(f) = app.boss.as_mut() {
                        f.qa_look();
                    }
                }
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        }
        let qa = |app: &mut App, what: &str| {
            if let Some(f) = app.boss.as_mut() {
                f.qa(what);
            }
        };
        run(&mut app, 0.5, false);
        save(&mut app, out, "boss-01-intro")?;
        app.boss_input.attack = true;
        run(&mut app, 3.4, false);
        save(&mut app, out, "boss-02-pack")?;
        qa(&mut app, "duel");
        run(&mut app, 2.0, true);
        save(&mut app, out, "boss-03-rise")?;
        run(&mut app, 2.4, true);
        save(&mut app, out, "boss-04-duel")?;
        qa(&mut app, "smash");
        run(&mut app, 0.9, true);
        save(&mut app, out, "boss-05-smash")?;
        run(&mut app, 0.35, true);
        save(&mut app, out, "boss-06-wave")?;
        qa(&mut app, "sweep");
        run(&mut app, 0.75, true);
        save(&mut app, out, "boss-07-sweep")?;
        qa(&mut app, "gallop");
        run(&mut app, 1.35, true);
        save(&mut app, out, "boss-08-fours")?;
        qa(&mut app, "spire");
        for _ in 0..240 {
            run(&mut app, 1.0 / 60.0, true);
            if app
                .boss
                .as_ref()
                .is_some_and(|f| f.varrak.state == varrak_fight::VState::Stunned)
            {
                break;
            }
        }
        run(&mut app, 0.25, true);
        save(&mut app, out, "boss-09-spire")?;
        qa(&mut app, "spark");
        app.boss_input.channel = true;
        run(&mut app, 1.0, true);
        save(&mut app, out, "boss-10-spark")?;
        qa(&mut app, "memory");
        run(&mut app, 0.9, true);
        save(&mut app, out, "boss-11-memory")?;
        qa(&mut app, "won");
        run(&mut app, 2.6, true);
        save(&mut app, out, "boss-12-kneel")?;
        app.boss_input.channel = false;
        run(&mut app, 3.0, false);
        save(&mut app, out, "boss-13-won")?;
        // Again, and this time she falls.
        app.boss = None;
        app.start_boss_fight();
        app.boss_input.attack = true;
        run(&mut app, 0.3, false);
        qa(&mut app, "lost");
        run(&mut app, 3.0, false);
        save(&mut app, out, "boss-14-lost")?;
        app.boss = None;
        app.penascal = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--missions") {
        // The missions and their guide, through the first act: the panel under the corner map,
        // the way on it, the arrows and the light in the world, the notices, the full map.
        let settle = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        };
        let in_castle = |app: &mut App, x: f32, z: f32| {
            app.penascal = None;
            app.nature_mode = false;
            app.village_entered = false;
            app.castle = None;
            app.player = Vec3::new(x, 0.0, z);
            app.iso.yaw = std::f32::consts::FRAC_PI_4;
            app.iso.yaw_target = app.iso.yaw;
            app.iso.distance = ISO.max_distance;
        };
        let outdoors = |app: &mut App, at: Vec3, village: bool| {
            app.penascal = None;
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = village;
            app.player = at;
            app.camera_focus = at;
            app.iso.yaw = std::f32::consts::FRAC_PI_4;
            app.iso.yaw_target = app.iso.yaw;
            app.iso.distance = ISO.max_distance;
        };
        let set = |app: &mut App, vars: &[&str]| {
            for v in vars {
                app.story.values.insert((*v).to_owned(), 1);
            }
        };
        app.environment_cycle.set_phase(0.42);
        app.story = guion::Vars::default();
        // The first council: from the bedroom to the war table (the reminder on top).
        in_castle(&mut app, -17.6, -12.6);
        settle(&mut app, 50);
        save(&mut app, out, "missions-01-consejo")?;
        settle(&mut app, 240);
        save(&mut app, out, "missions-02-consejo-sin-aviso")?;
        // Done: the notice of the next one; the balcony.
        set(&mut app, &["consejo_1_hecho"]);
        settle(&mut app, 40);
        save(&mut app, out, "missions-03-completada")?;
        settle(&mut app, 300);
        in_castle(&mut app, 18.0, -10.6);
        settle(&mut app, 60);
        save(&mut app, out, "missions-04-balcon")?;
        // Lira's errand: to wherever she walks.
        set(&mut app, &["lamento_visto"]);
        settle(&mut app, 400);
        save(&mut app, out, "missions-05-lira")?;
        // The corral is in the villa: to the castle's gate, the wood's path, the villa's corral.
        set(&mut app, &["corral_pedido"]);
        in_castle(&mut app, 0.0, 18.6);
        settle(&mut app, 360);
        save(&mut app, out, "missions-06-a-la-puerta")?;
        outdoors(
            &mut app,
            Vec3::new(nature_trail_center(-8.0), 0.0, -8.0),
            false,
        );
        settle(&mut app, 90);
        save(&mut app, out, "missions-07-bosque-a-la-villa")?;
        app.map_open = true;
        settle(&mut app, 4);
        save(&mut app, out, "missions-08-mapa-bosque")?;
        app.map_open = false;
        let (gate, _) = village::spawn();
        outdoors(&mut app, gate, true);
        settle(&mut app, 90);
        save(&mut app, out, "missions-09-villa-al-corral")?;
        // Then the night, the second council, and the Peñascal.
        set(
            &mut app,
            &["gorou_cabeza_alta", "varrak_visto", "consejo_2_hecho"],
        );
        settle(&mut app, 600);
        outdoors(
            &mut app,
            Vec3::new(nature_trail_center(0.0), 0.0, 0.0),
            false,
        );
        settle(&mut app, 90);
        save(&mut app, out, "missions-10-bosque-al-penascal")?;
        app.enter_penascal_from_forest();
        settle(&mut app, 120);
        save(&mut app, out, "missions-11-penascal-a-varrak")?;
        // J: the side errand instead (Nemi's gift is open since the first council).
        app.cycle_mission();
        in_castle(&mut app, 0.0, 3.6);
        settle(&mut app, 90);
        save(&mut app, out, "missions-12-secundaria")?;
        app.penascal = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mugo") {
        // Mugo at the springs: at his post, leading the Queen to the first spot (she follows a
        // step behind), pointing at it, the photo's prompt, and the photo on the stage.
        app.nature_mode = true;
        app.village_entered = true;
        app.castle = None;
        app.environment_cycle.set_phase(0.5);
        app.story = guion::Vars::default();
        app.story.values.insert("termas_bienvenida".into(), 1);
        app.enter_baths();
        let mugo = |app: &App| {
            app.state
                .as_ref()
                .and_then(|s| s.baths_scene.as_ref())
                .and_then(|scene| scene.mugo_report())
        };
        for _ in 0..180 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        if mugo(&app).is_none() {
            anyhow::bail!("no Mugo at the springs");
        }
        // Beside him at his post.
        let ([mx, mz], _) = mugo(&app).unwrap();
        if let Some(inside) = app.baths.as_mut() {
            (inside.x, inside.z) = (mx - 1.2, mz + 0.9);
        }
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        save(&mut app, out, "mugo-1-puesto")?;
        // Asked to show the spots: he leads, she follows a step and a half behind.
        app.story.values.insert("mugo_guia".into(), 1);
        let mut shot_walk = false;
        for frame in 0..(60 * 90) {
            app.tick(1.0 / 60.0);
            let Some(([mx, mz], there)) = mugo(&app) else {
                break;
            };
            if let Some(inside) = app.baths.as_mut() {
                let (dx, dz) = (mx - inside.x, mz - inside.z);
                let d = dx.hypot(dz);
                if d > 1.5 {
                    // Her walking pace, two metres a second at most.
                    let step = (d - 1.5).min(2.0 / 60.0);
                    inside.x += dx / d * step;
                    inside.z += dz / d * step;
                }
            }
            if let Some(inside) = &app.baths {
                app.player = inside.position();
                app.camera_focus = app.player;
            }
            if frame % 2 == 0 {
                app.render_frame();
            }
            if !shot_walk && frame > 90 {
                shot_walk = true;
                save(&mut app, out, "mugo-2-guia")?;
            }
            if there {
                break;
            }
        }
        let ready = mugo(&app).is_some_and(|(_, there)| there);
        save(&mut app, out, "mugo-3-senala")?;
        // At the spot: the photo's prompt, then the photo.
        if let Some(spot) = app.baths.as_ref().and_then(|b| b.guide) {
            let [sx, sz] = mugo::PHOTO_SPOTS[spot].stand();
            if let Some(inside) = app.baths.as_mut() {
                (inside.x, inside.z) = (sx, sz);
            }
            for _ in 0..20 {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
            save(&mut app, out, "mugo-4-aviso")?;
            let prompt = app.baths.as_ref().and_then(|b| b.photo_here());
            app.interact_baths();
            app.elapsed += 1.4;
            save(&mut app, out, "mugo-5-foto")?;
            eprintln!(
                "[mugo-qa] guide {} ({}), there {ready}, photo prompt {prompt:?}, stage {}",
                spot,
                mugo::PHOTO_SPOTS[spot].id,
                app.stage.is_some()
            );
        } else {
            eprintln!("[mugo-qa] no guide set (there {ready})");
        }
        return Ok(());
    }
    if let Some(which) = std::env::args().skip_while(|a| a != "--talk").nth(1) {
        // A conversation out in the world played through (`--talk castillo.lira_primera`): a shot
        // of each line and choice, the options taken from VELVET_STAGE_PICKS as for --stage.
        app.nature_mode = false;
        app.village_entered = true;
        app.castle = None;
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        app.story = guion::Vars::default();
        let picks: Vec<usize> = std::env::var("VELVET_STAGE_PICKS")
            .ok()
            .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
            .unwrap_or_default();
        let tag = which.replace('.', "-");
        // VELVET_OUTFIT=n: she wears the wardrobe's slot n (her face in the box follows it).
        if let Some(outfit) = std::env::var("VELVET_OUTFIT")
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
        {
            app.outfit = outfit;
        }
        if !app.start_talk_at(&which) {
            anyhow::bail!("no guion scene {which}");
        }
        let (mut shot, mut chosen) = (0, 0);
        while app.talk.is_some() && shot < 40 {
            app.elapsed += 5.0;
            let lib = guion::library();
            let shown = app.talk.as_ref().map(|t| t.shown(lib, &app.story));
            let choice = matches!(shown, Some(guion::Shown::Choice { .. }));
            let label = if choice { "eleccion" } else { "linea" };
            save(&mut app, out, &format!("talk-{tag}-{shot:02}-{label}"))?;
            shot += 1;
            if choice {
                let pick = picks.get(chosen).copied().unwrap_or(0);
                chosen += 1;
                app.talk_pick(pick);
            } else {
                app.talk_confirm();
            }
        }
        return Ok(());
    }
    if let Some(which) = std::env::args().skip_while(|a| a != "--stage").nth(1) {
        // A stage scene played through by a script: `--stage burdel.bienvenida`. The options
        // chosen in turn come from VELVET_STAGE_PICKS (`0,2,1`; the first one otherwise). A shot
        // of the title, of every few lines, of each flourish as it plays and of the outro.
        app.nature_mode = false;
        app.village_entered = true;
        app.castle = None;
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        app.story = guion::Vars::default();
        let picks: Vec<usize> = std::env::var("VELVET_STAGE_PICKS")
            .ok()
            .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
            .unwrap_or_default();
        let tag = which.replace('.', "-");
        if !app.start_talk_at(&which) {
            anyhow::bail!("no guion scene {which}");
        }
        let mut shot = 0;
        app.elapsed += 0.9;
        save(&mut app, out, &format!("stage-{tag}-{shot:02}-titulo"))?;
        shot += 1;
        let (mut steps, mut chosen) = (0, 0);
        while app.talk.is_some() && steps < 240 {
            steps += 1;
            app.elapsed += 2.6;
            let lib = guion::library();
            let shown = app.talk.as_ref().map(|t| t.shown(lib, &app.story));
            let choice = matches!(shown, Some(guion::Shown::Choice { .. }));
            if choice || steps % 5 == 0 {
                let label = if choice { "eleccion" } else { "linea" };
                save(&mut app, out, &format!("stage-{tag}-{shot:02}-{label}"))?;
                shot += 1;
            }
            let before = app.elapsed;
            if choice {
                let pick = picks.get(chosen).copied().unwrap_or(0);
                chosen += 1;
                app.talk_pick(pick);
            } else {
                app.talk_confirm();
            }
            // A flourish that just came on: caught while it plays.
            app.elapsed += 0.3;
            let fresh = |t: f32| t >= before - 0.01;
            if let Some(stage) = app.stage.as_ref() {
                let what = if stage.stamp.as_ref().is_some_and(|(_, _, t)| fresh(*t)) {
                    Some("sello")
                } else if stage.item.as_ref().is_some_and(|(_, _, _, t)| fresh(*t)) {
                    Some("objeto")
                } else if stage.title.as_ref().is_some_and(|(_, _, t)| fresh(*t)) {
                    Some("titulo")
                } else if fresh(stage.wipe_at) {
                    Some("barrido")
                } else if stage.reveal.as_ref().is_some_and(|(_, t)| fresh(*t)) {
                    Some("revela")
                } else if stage.panels.iter().flatten().any(|(_, t)| fresh(*t)) {
                    Some("panel")
                } else if stage
                    .actors
                    .iter()
                    .any(|a| a.spin.as_ref().is_some_and(|(_, t)| fresh(*t)))
                {
                    Some("giro")
                } else {
                    None
                };
                if let Some(what) = what {
                    save(&mut app, out, &format!("stage-{tag}-{shot:02}-{what}"))?;
                    shot += 1;
                }
            }
        }
        if app.stage.as_ref().is_some_and(|s| s.outro_at.is_some()) {
            app.elapsed += 1.6;
            save(&mut app, out, &format!("stage-{tag}-{shot:02}-fin"))?;
            if let Some(stage) = app.stage.as_mut() {
                stage.unlocks.clear();
            }
            app.end_talk();
        }
        eprintln!(
            "[stage-qa] {which}: {steps} steps, {shot} shots, stage down: {}, outfit {}",
            app.stage.is_none(),
            app.outfit
        );
        return Ok(());
    }
    if let Some(which) = std::env::args()
        .skip_while(|a| a != "--book-to-game")
        .nth(1)
    {
        // The whole way in: Rosalía's book, the girl paid for (the elixir drunk first when it is
        // asked for), her escort, her guion, and her game opening: `alba`, `alba-futa`, `noa`.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-17.6, 0.0, -12.6);
        app.castle = None;
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        app.story = guion::Vars::default();
        app.gold = app.gold.max(200);
        let (girl, ask) = match which.as_str() {
            "alba-futa" => (1, "sello"),
            "noa" => (2, "sello"),
            "noa-plain" => (2, "noche"),
            _ => (1, "pide"),
        };
        if ask == "sello" {
            app.venue.progress.elixir_active = true;
            app.venue.progress.elixir_offered = true;
            app.venue.progress.visits = 1;
        }
        app.venue.select_tab(0);
        app.venue.selected = girl;
        app.venue.act(&mut app.gold);
        app.venue.asked = Some(ask.to_owned());
        app.after_venue_act();
        app.skip_escort();
        for _ in 0..300 {
            if app.talk.is_some() {
                break;
            }
            app.tick(1.0 / 30.0);
        }
        let mut lines = 0;
        while app.talk.is_some() && lines < 16 {
            app.elapsed += 5.0;
            if lines == 1 {
                save(&mut app, out, &format!("book-{which}-guion"))?;
            }
            app.talk_confirm();
            app.tick(1.0 / 30.0);
            lines += 1;
        }
        eprintln!(
            "[book-qa] {which}: {lines} lines, game {:?}, elixir still on: {}",
            app.play_scene.as_ref().map(|s| s.game),
            app.venue.progress.elixir_active
        );
        for _ in 0..90 {
            app.tick(1.0 / 30.0);
            app.bed_scene_settle();
            app.elapsed += 1.0 / 30.0;
        }
        save(&mut app, out, &format!("book-{which}-game"))?;
        return Ok(());
    }
    if let Some(which) = std::env::args().skip_while(|a| a != "--play-scene").nth(1) {
        // A brothel girl's minigame played by a script, a shot at each moment worth seeing:
        // `--play-scene alba|alba-futa|noa`.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-17.6, 0.0, -12.6);
        app.castle = None;
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let game = match which.as_str() {
            "alba-futa" => play_scene::Game::AlbaFuta,
            "noa" => play_scene::Game::NoaFuta,
            "noa-plain" => play_scene::Game::Noa,
            w if w.starts_with("hipno") => play_scene::Game::Hipno,
            // `tienda-<n>`: the shopkeeper's favour n (0 the spank … 10 on his stool).
            w if w.starts_with("tienda") => play_scene::Game::Tienda(
                w.trim_start_matches("tienda")
                    .trim_start_matches('-')
                    .parse()
                    .unwrap_or(0),
            ),
            _ => play_scene::Game::Alba,
        };
        if game == play_scene::Game::Hipno {
            // Something from his stall in her bag: two sniffs of salts and the amulet.
            app.inventory.insert(play_scene::SALTS.to_owned(), 2);
            app.inventory.insert(play_scene::AMULET.to_owned(), 1);
        }
        app.open_play_scene(game);
        eprintln!("[play-qa] {which}: open {}", app.play_scene.is_some());
        let step = |app: &mut App, seconds: f32| {
            for _ in 0..(seconds * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
        };
        let key =
            |app: &mut App, code: KeyCode, pressed: bool| app.bed_scene_key(code, pressed, false);
        let qa = |app: &App| app.play_scene_qa();
        let tag = which.replace('-', "_");
        let shot = |app: &mut App, name: &str| save(app, out, &format!("{tag}-{name}"));
        step(&mut app, 2.2);
        shot(&mut app, "rules")?;
        if game == play_scene::Game::Hipno {
            hipno_qa(&mut app, out, &which, save)?;
        } else if matches!(game, play_scene::Game::Tienda(_)) {
            // Hurried with Space half way, then sealed.
            key(&mut app, KeyCode::Space, true);
            for _ in 0..300 {
                if qa(&app).is_some_and(|q| q.heat >= 0.5) {
                    break;
                }
                step(&mut app, 0.1);
            }
            shot(&mut app, "hurry")?;
            for _ in 0..300 {
                if qa(&app).is_some_and(|q| q.phase == "relax") {
                    break;
                }
                step(&mut app, 0.1);
            }
            key(&mut app, KeyCode::Space, false);
            step(&mut app, 0.25);
            shot(&mut app, "sealed")?;
        } else if matches!(game, play_scene::Game::NoaFuta | play_scene::Game::Noa) {
            // Through the intro, then held deep until her air runs low, and on until she coughs.
            for _ in 0..300 {
                if qa(&app).is_some_and(|q| q.phase == "play") {
                    break;
                }
                step(&mut app, 0.2);
            }
            key(&mut app, KeyCode::Space, true);
            step(&mut app, 2.4);
            shot(&mut app, "deep")?;
            for _ in 0..80 {
                if qa(&app).is_some_and(|q| q.air < 0.25) {
                    break;
                }
                step(&mut app, 0.1);
            }
            shot(&mut app, "air")?;
            for _ in 0..60 {
                if qa(&app).is_some_and(|q| q.air >= 0.3) {
                    break;
                }
                step(&mut app, 0.1);
            }
            step(&mut app, 0.25);
            shot(&mut app, "cough")?;
            key(&mut app, KeyCode::Space, false);
            // Played by feel: held while quiet and breathing, let go when loud; a new posture as
            // soon as one opens; one listening window caught on camera, one knock forced.
            let (mut listened, mut knocked, mut reves, mut pen, mut leash) =
                (false, false, false, false, false);
            for i in 0..4000 {
                let Some(q) = qa(&app) else { break };
                if q.phase != "play" {
                    break;
                }
                let limit = if q.listening { 0.5 } else { 0.9 };
                let force_knock = !knocked && reves && q.heat > 0.3;
                let hold = force_knock || (q.noise < limit * 0.75 && (q.pose >= 2 || q.air > 0.3));
                key(&mut app, KeyCode::Space, hold);
                if force_knock && q.knocks > 0 {
                    knocked = true;
                    step(&mut app, 0.1);
                    shot(&mut app, "knock")?;
                    key(&mut app, KeyCode::Space, false);
                    step(&mut app, 2.2);
                    shot(&mut app, "after-knock")?;
                }
                if q.listening && !listened {
                    listened = true;
                    step(&mut app, 1.0);
                    shot(&mut app, "listening")?;
                }
                if q.open >= 2 && !reves {
                    reves = true;
                    key(&mut app, KeyCode::KeyD, true);
                    key(&mut app, KeyCode::KeyD, false);
                    step(&mut app, 1.4);
                    shot(&mut app, "reves")?;
                }
                if q.open >= if game == play_scene::Game::Noa { 3 } else { 4 } && !pen {
                    pen = true;
                    key(&mut app, KeyCode::KeyD, true);
                    key(&mut app, KeyCode::KeyD, false);
                    step(&mut app, 1.4);
                    shot(&mut app, "misionero")?;
                }
                if pen && !leash && q.heat > 0.7 {
                    leash = true;
                    key(&mut app, KeyCode::KeyT, true);
                    key(&mut app, KeyCode::KeyT, false);
                    step(&mut app, 0.25);
                    shot(&mut app, "leash")?;
                }
                if i % 400 == 0 {
                    eprintln!(
                        "[play-qa] heat {:.2} noise {:.2} air {:.2} pose {} knocks {}",
                        q.heat, q.noise, q.air, q.pose, q.knocks
                    );
                }
                step(&mut app, 0.1);
            }
            key(&mut app, KeyCode::Space, false);
            if qa(&app).is_some_and(|q| q.phase == "cut") {
                step(&mut app, 1.0);
                shot(&mut app, "cut")?;
                step(&mut app, 1.1);
                shot(&mut app, "glass")?;
            }
            step(&mut app, 1.4);
            shot(&mut app, "climax")?;
        } else {
            // Her shouts answered: right mostly, one wrong, one let burn out.
            let (mut shout, mut right, mut wrong, mut late, mut streak, mut wild) =
                (false, false, false, false, false, false);
            let mut swoop = false;
            let mut n = 0;
            for _ in 0..3000 {
                let Some(q) = qa(&app) else { break };
                if q.phase != "play" && q.phase != "intro" {
                    break;
                }
                if let Some(r) = q.right {
                    if !shout {
                        shout = true;
                        step(&mut app, 0.6);
                        shot(&mut app, "shout")?;
                    }
                    if q.heat > 0.72 && !wild {
                        wild = true;
                        step(&mut app, 0.5);
                        shot(&mut app, "wild")?;
                    }
                    n += 1;
                    if n == 3 && !wrong {
                        wrong = true;
                        let k = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3][(r + 1) % 3];
                        key(&mut app, k, true);
                        key(&mut app, k, false);
                        step(&mut app, 0.3);
                        shot(&mut app, "wrong")?;
                    } else if n == 5 && !late {
                        late = true;
                        step(&mut app, 6.0);
                    } else {
                        step(&mut app, 0.5 + (n % 3) as f32 * 0.3);
                        let k = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3][r];
                        key(&mut app, k, true);
                        key(&mut app, k, false);
                        step(&mut app, 0.3);
                        if !right {
                            right = true;
                            shot(&mut app, "right")?;
                        }
                        if n == 9 && !streak {
                            streak = true;
                            shot(&mut app, "streak")?;
                        }
                        // Five in a row: Chiquita's loop in the air, seen on its way up and round.
                        if qa(&app).is_some_and(|q| q.streak >= 5) && !swoop {
                            swoop = true;
                            app.mascot_cue(mascot::Cue::Swoop);
                            for (k, at) in [0.05f32, 0.45, 0.45].iter().enumerate() {
                                step(&mut app, *at);
                                shot(&mut app, &format!("swoop-{k}"))?;
                            }
                        }
                    }
                }
                step(&mut app, 0.1);
            }
            step(&mut app, 2.6);
            shot(&mut app, "climax")?;
            for _ in 0..200 {
                if qa(&app).is_some_and(|q| q.phase == "verse") {
                    break;
                }
                step(&mut app, 0.1);
            }
            step(&mut app, 1.2);
            shot(&mut app, "verse")?;
            if let Some(r) = qa(&app).and_then(|q| q.right) {
                let k = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3][r];
                key(&mut app, k, true);
                key(&mut app, k, false);
            }
            step(&mut app, 0.4);
            shot(&mut app, "rhyme")?;
            step(&mut app, 3.0);
            shot(&mut app, "relax")?;
        }
        for _ in 0..300 {
            if qa(&app).is_some_and(|q| q.ended) {
                break;
            }
            step(&mut app, 0.1);
        }
        step(&mut app, 2.4);
        shot(&mut app, "card")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--nina-scene") {
        // Nina's way: the venue's purchase, her guion line by line, then her scene's four parts.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-17.6, 0.0, -12.6);
        app.castle = None;
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            if let Some(inside) = &app.castle {
                app.camera_focus = inside.focus();
            }
            app.render_frame();
        }
        app.story = guion::Vars::default();
        app.venue.select_tab(0);
        app.venue.selected = 0;
        app.gold = app.gold.max(100);
        app.venue.act(&mut app.gold);
        app.after_venue_act();
        let mut shot = 0;
        while app.talk.is_some() && shot < 12 {
            app.elapsed += 5.0;
            if shot == 0 {
                save(&mut app, out, "nina-line")?;
            }
            app.talk_confirm();
            shot += 1;
        }
        // `--nina-futa`: the second choice (the seal's own shape) instead of the first.
        if std::env::args().any(|a| a == "--nina-futa") {
            app.bed_scene = None;
            app.open_scene(&bed_scene::NINA_FUTA, false);
        }
        eprintln!("[nina-qa] scene open: {}", app.bed_scene.is_some());
        let run = |app: &mut App, seconds: f32, hold: bool| {
            app.bed_scene_button(hold);
            for _ in 0..(seconds * 30.0) as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
        };
        run(&mut app, 2.5, false);
        save(&mut app, out, "nina-intro")?;
        run(&mut app, 4.0, false);
        run(&mut app, 2.2, true);
        save(&mut app, out, "nina-hold")?;
        run(&mut app, 1.0, true);
        save(&mut app, out, "nina-brake")?;
        // Up to the eager loop: a frame as the loops cross-fade, then the eager loop itself.
        let heat = |app: &App| app.bed_scene.as_ref().map_or(0.0, |s| s.heat);
        let mut crossed = false;
        let mut topped = false;
        for _ in 0..400 {
            // Held until the bar is full: then the climax has begun.
            if heat(&app) >= 1.0 {
                break;
            }
            app.bed_scene_button(true);
            for _ in 0..3 {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
            if !crossed && heat(&app) >= 0.55 {
                crossed = true;
                run(&mut app, 0.2, true);
                save(&mut app, out, "nina-fade")?;
                run(&mut app, 1.2, true);
                save(&mut app, out, "nina-fast")?;
            }
            if !topped && heat(&app) >= 0.86 {
                topped = true;
                save(&mut app, out, "nina-top")?;
            }
            if app.bed_scene.as_ref().is_some_and(|s| s.braked) {
                run(&mut app, 0.1, false);
            }
        }
        // At the burst's peak (Spec::burst), so the room's flash is on the shot.
        run(&mut app, 3.55, false);
        save(&mut app, out, "nina-climax")?;
        run(&mut app, 5.0, false);
        save(&mut app, out, "nina-relax")?;
        run(&mut app, 6.0, false);
        save(&mut app, out, "nina-card")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mascot-poses") {
        // Her whole figure from her 3D self, for the visual novel (the prologue's lessons): a pose
        // held standing, read back with its transparency, cut from her horns to her feet.
        // Into data/assets/chiquita/pose_<name>.png.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-15.5, 0.0, -14.6);
        app.castle = None;
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let dir = crate::exploration3d_state::data_dir()
            .join("assets")
            .join("chiquita");
        std::fs::create_dir_all(&dir)?;
        // At the visual novel's sprite slot size (580 × 1010, painted 1:1 at x 1260, y 20): she
        // floats in front of the chalkboard's right edge, her feet just over the dialogue box.
        let tall = 600.0f32;
        let feet = [960.0f32, 735.0];
        let crop = [670u32, 20, 580, 1010];
        for (name, clip, time, yaw, face) in [
            ("pose_habla", 1u8, 0.5f32, 12.0f32, (0.0f32, 0.25f32)),
            ("pose_regana", 9, 0.35, 8.0, (0.35, 0.2)),
            ("pose_celosa", 2, 0.14, 0.0, (0.55, 0.0)),
            ("pose_animada", 5, 0.3, 6.0, (0.0, 0.4)),
            ("pose_avergonzada", 4, 0.4, 10.0, (0.2, 0.1)),
        ] {
            app.set_mascot_qa(Some(mascot::QaPose {
                clip,
                time,
                feet,
                height: tall,
                yaw,
                face,
            }));
            // Long enough for her hair to settle after the turn.
            for _ in 0..45 {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
            let state = app.state.as_ref().expect("gpu");
            let Some((w, h, rgba)) = state
                .map_overlay
                .mascot_snapshot(&state.device, &state.queue)
            else {
                eprintln!("[mascot-pose] no picture");
                continue;
            };
            let [cx, cy, cw, ch] = crop;
            let (cw, ch) = (cw.min(w - cx), ch.min(h - cy));
            let to_linear = |c: u8| {
                let c = c as f32 / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            let to_srgb = |c: f32| {
                let c = c.clamp(0.0, 1.0);
                let s = if c <= 0.003_130_8 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round() as u8
            };
            let mut out_px = Vec::with_capacity((cw * ch * 4) as usize);
            for y in cy..cy + ch {
                for x in cx..cx + cw {
                    let i = ((y * w + x) * 4) as usize;
                    let a = rgba[i + 3] as f32 / 255.0;
                    if a <= 0.002 {
                        out_px.extend_from_slice(&[0, 0, 0, 0]);
                        continue;
                    }
                    for c in 0..3 {
                        out_px.push(to_srgb(to_linear(rgba[i + c]) / a));
                    }
                    out_px.push(rgba[i + 3]);
                }
            }
            image::save_buffer(
                dir.join(format!("{name}.png")),
                &out_px,
                cw,
                ch,
                image::ColorType::Rgba8,
            )?;
            eprintln!("[mascot-pose] {name}");
        }
        app.set_mascot_qa(None);
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mascot-portraits") {
        // Her portraits from her 3D self: a pose held big with her head in the middle, read back
        // with its transparency, cut square from her shoulders to the tips of her horns.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-15.5, 0.0, -14.6);
        app.castle = None;
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let tall = 1900.0f32;
        let feet = [960.0f32, 540.0 + 0.80 * tall];
        let crop = [455u32, 70, 1010, 1010];
        for (name, clip, time, yaw, face) in [
            ("chiquita3d_normal", 1u8, 0.5f32, 10.0f32, (0.0f32, 0.25f32)),
            ("chiquita3d_celosa", 2, 0.14, 0.0, (0.55, 0.0)),
        ] {
            app.set_mascot_qa(Some(mascot::QaPose {
                clip,
                time,
                feet,
                height: tall,
                yaw,
                face,
            }));
            for _ in 0..12 {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
            let state = app.state.as_ref().expect("gpu");
            let Some((w, _h, rgba)) = state
                .map_overlay
                .mascot_snapshot(&state.device, &state.queue)
            else {
                eprintln!("[mascot-portrait] no picture");
                continue;
            };
            let [cx, cy, cw, ch] = crop;
            let to_linear = |c: u8| {
                let c = c as f32 / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            let to_srgb = |c: f32| {
                let c = c.clamp(0.0, 1.0);
                let s = if c <= 0.003_130_8 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round() as u8
            };
            let mut out_px = Vec::with_capacity((cw * ch * 4) as usize);
            for y in cy..cy + ch {
                for x in cx..cx + cw {
                    let i = ((y * w + x) * 4) as usize;
                    let a = rgba[i + 3] as f32 / 255.0;
                    if a <= 0.002 {
                        out_px.extend_from_slice(&[0, 0, 0, 0]);
                        continue;
                    }
                    // Her edges were resolved premultiplied (in linear light): straight colour again.
                    for c in 0..3 {
                        out_px.push(to_srgb(to_linear(rgba[i + c]) / a));
                    }
                    out_px.push(rgba[i + 3]);
                }
            }
            image::save_buffer(
                out.join(format!("{name}.png")),
                &out_px,
                cw,
                ch,
                image::ColorType::Rgba8,
            )?;
            eprintln!("[mascot-portrait] {name}");
        }
        app.set_mascot_qa(None);
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mascot-drag") {
        // Poking her, picking her up, carrying her across, throwing her, her landing; and her
        // balloons, one of each.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-15.5, 0.0, -14.6);
        app.castle = None;
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let step = |app: &mut App| {
            app.tick(1.0 / 30.0);
            app.render_frame();
        };
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        app.float_chiquita(
            "—¿Otra vez aquí? Esta alcoba huele a reina aburrida.",
            false,
        );
        wait(&mut app, 0.6);
        save(&mut app, out, "drag-00-talk-typing")?;
        wait(&mut app, 2.6);
        save(&mut app, out, "drag-01-talk")?;
        // A poke on her head.
        let head = [1790.0f32, 1078.0 - 290.0 * 0.8];
        eprintln!(
            "[drag-qa] hit at head: {}",
            app.mascot_hit(head[0], head[1])
        );
        app.mascot_press(head[0], head[1]);
        step(&mut app);
        app.mascot_release();
        wait(&mut app, 0.1);
        save(&mut app, out, "drag-02-poke")?;
        wait(&mut app, 1.4);
        save(&mut app, out, "drag-03-poke-line")?;
        // Picked up and carried across, up and to the left.
        let pressed = app.mascot_press(head[0], head[1]);
        eprintln!("[drag-qa] picked up: {pressed}");
        let mut at = head;
        for i in 0..24 {
            at = [at[0] - 26.0, at[1] - 14.0 + (i as f32 * 0.6).sin() * 6.0];
            app.mascot_drag_to(at[0], at[1]);
            step(&mut app);
            if i == 10 {
                save(&mut app, out, "drag-04-carried")?;
            }
        }
        save(&mut app, out, "drag-05-carried-more")?;
        // Held still a moment: she swings on.
        wait(&mut app, 0.25);
        save(&mut app, out, "drag-06-swing")?;
        // Thrown up and to the right.
        for _ in 0..4 {
            at = [at[0] + 90.0, at[1] - 70.0];
            app.mascot_drag_to(at[0], at[1]);
            step(&mut app);
        }
        app.mascot_release();
        wait(&mut app, 0.2);
        save(&mut app, out, "drag-07-thrown")?;
        wait(&mut app, 0.35);
        save(&mut app, out, "drag-08-flying")?;
        wait(&mut app, 1.6);
        save(&mut app, out, "drag-09-landed")?;
        wait(&mut app, 2.0);
        save(&mut app, out, "drag-10-placed")?;
        // Her balloons, one of each.
        for (name, line, cue) in [
            (
                "drag-11-cross",
                "—¿Celosa? ¿Yo? ¿De una doncella con cofia? …Un poco. Cállate.",
                mascot::Cue::Jealous,
            ),
            (
                "drag-12-mutter",
                "—La pared. Claro. Preciosa pared. Muy… pared.",
                mascot::Cue::Away,
            ),
            (
                "drag-13-flustered",
                "—…¿En mí? …Vale. Bien. Sigue. No me mires así.",
                mascot::Cue::Flustered,
            ),
            (
                "drag-14-pleased",
                "—…Buenas noches, tonta. Digo, Majestad.",
                mascot::Cue::Cheer,
            ),
        ] {
            app.float_chiquita_cue(line, cue);
            wait(&mut app, 2.2);
            save(&mut app, out, name)?;
        }
        eprintln!("[drag-qa] done");
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mascot-face") {
        // Chiquita close up: her eyes open, blinking, half shut, her mouth; every clip held big;
        // her hair as she springs up and as she whips round.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-15.5, 0.0, -14.6);
        app.castle = None;
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        let hold = |app: &mut App,
                    clip: u8,
                    time: f32,
                    feet: [f32; 2],
                    height: f32,
                    yaw: f32,
                    face: (f32, f32)| {
            app.set_mascot_qa(Some(mascot::QaPose {
                clip,
                time,
                feet,
                height,
                yaw,
                face,
            }));
        };
        for (name, face) in [
            ("chq-00-open", (0.0, 0.0)),
            ("chq-01-shut", (1.0, 0.0)),
            ("chq-02-half", (0.5, 0.0)),
            ("chq-03-lidded", (0.4, 0.0)),
            ("chq-04-mouth", (0.0, 1.0)),
            ("chq-05-mouth-half", (0.0, 0.5)),
        ] {
            hold(&mut app, 0, 0.0, [960.0, 2244.0], 2400.0, 0.0, face);
            wait(&mut app, 0.4);
            save(&mut app, out, name)?;
        }
        for (name, clip, time, yaw) in [
            ("chq-10-idle", 0u8, 1.0f32, 0.0f32),
            ("chq-11-talk", 1, 0.5, 0.0),
            ("chq-12-jealous", 2, 0.12, 0.0),
            ("chq-13-away-peek", 3, 1.65, 0.0),
            ("chq-14-flustered", 4, 0.4, 0.0),
            ("chq-15-cheer", 5, 0.44, 0.0),
            ("chq-16-splat", 6, 0.55, 0.0),
            ("chq-17-punch", 7, 0.44, 0.0),
            ("chq-18-enter", 8, 0.3, 0.0),
            ("chq-19-scold", 9, 0.3, 0.0),
            ("chq-20-side", 0, 1.0, 90.0),
            ("chq-21-back", 0, 1.0, 180.0),
        ] {
            hold(
                &mut app,
                clip,
                time,
                [960.0, 1050.0],
                900.0,
                yaw,
                (0.0, 0.0),
            );
            wait(&mut app, 0.5);
            save(&mut app, out, name)?;
        }
        app.set_mascot_qa(None);
        wait(&mut app, 8.0);
        // Springing up, her hair flying.
        app.float_chiquita(
            "—¿Otra vez aquí? Esta alcoba huele a reina aburrida.",
            false,
        );
        for (name, at) in [
            ("chq-30-up", 0.1),
            ("chq-31-up", 0.2),
            ("chq-32-up", 0.3),
            ("chq-33-up", 0.45),
            ("chq-34-up", 0.7),
            ("chq-35-up", 1.2),
        ] {
            let from = app.elapsed;
            let _ = from;
            wait(&mut app, if name == "chq-30-up" { 0.1 } else { 0.0 });
            let _ = at;
            save(&mut app, out, name)?;
            wait(
                &mut app,
                match name {
                    "chq-30-up" | "chq-31-up" => 0.1,
                    "chq-32-up" => 0.15,
                    "chq-33-up" => 0.25,
                    _ => 0.5,
                },
            );
        }
        // Whipping round to look at the wall.
        app.float_chiquita_cue("—Yo me voy a mirar la pared.", mascot::Cue::Away);
        for name in [
            "chq-40-turn",
            "chq-41-turn",
            "chq-42-turn",
            "chq-43-turn",
            "chq-44-turn",
        ] {
            wait(&mut app, 0.1);
            save(&mut app, out, name)?;
        }
        wait(&mut app, 0.6);
        save(&mut app, out, "chq-45-turned")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mascot") {
        // Chiquita out in 3D: talking in her corner, turning her back on the bed scene, flying at
        // the glass, and smashing the screen in front of the futa scene's black climax.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-15.5, 0.0, -14.6);
        app.castle = None;
        for _ in 0..90 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.render_frame();
            }
        };
        app.float_chiquita(
            "—¿Otra vez aquí? Esta alcoba huele a reina aburrida.",
            false,
        );
        wait(&mut app, 0.25);
        save(&mut app, out, "mascot-00-rising")?;
        wait(&mut app, 0.8);
        save(&mut app, out, "mascot-01-talk")?;
        app.float_chiquita("—Y no me mires así. Yo no he dicho nada.", true);
        wait(&mut app, 1.2);
        save(&mut app, out, "mascot-02-jealous")?;
        // The bed scene: "me voy a mirar la pared".
        app.story = guion::Vars::default();
        app.open_scene(&bed_scene::BED, false);
        let run = |app: &mut App, seconds: f32, hold: bool| {
            app.bed_scene_button(hold);
            for _ in 0..(seconds * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.render_frame();
            }
        };
        // Steps until `at` seconds have passed since `from` (the game's clock).
        let until = |app: &mut App, from: f32, at: f32| {
            while app.elapsed - from < at - 1e-3 {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.render_frame();
            }
        };
        run(&mut app, 2.4, false);
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.answer(1);
        }
        run(&mut app, 4.0, false);
        for _ in 0..40 {
            if app.bed_scene.as_ref().map_or(true, |s| s.heat >= 0.2) {
                break;
            }
            run(&mut app, 0.2, true);
        }
        run(&mut app, 0.6, false);
        save(&mut app, out, "mascot-03-wall")?;
        run(&mut app, 1.1, false);
        save(&mut app, out, "mascot-04-wall-peek")?;
        app.bed_scene_key(KeyCode::Escape, true, false);
        run(&mut app, 1.6, false);
        app.bed_scene_key(KeyCode::KeyF, true, false);
        wait(&mut app, 0.5);
        // Flying at the glass.
        app.float_chiquita_cue("—¡¿MIRAR?! …No pienso mirar.", mascot::Cue::Splat);
        let from = app.elapsed;
        for (name, at) in [
            ("mascot-05-leap", 0.2),
            ("mascot-06-splat", 0.45),
            ("mascot-07-stuck", 1.1),
            ("mascot-08-slide", 2.1),
            ("mascot-09-back", 4.2),
        ] {
            until(&mut app, from, at);
            save(&mut app, out, name)?;
        }
        // The futa scene's black climax: she breaks the screen.
        app.story = guion::Vars::default();
        app.open_scene(&bed_scene::NINA_FUTA, false);
        run(&mut app, 2.0, false);
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.answer(0);
        }
        for _ in 0..900 {
            if app.bed_scene.as_ref().map_or(true, |s| s.black_now()) {
                break;
            }
            let held = app.bed_scene.as_ref().is_some_and(|s| s.heat < 1.0);
            run(&mut app, 0.1, held);
            if app.bed_scene.as_ref().is_some_and(|s| s.braked) {
                run(&mut app, 0.1, false);
            }
        }
        eprintln!(
            "[mascot-qa] black: {}",
            app.bed_scene.as_ref().is_some_and(|s| s.black_now())
        );
        let from = app.elapsed;
        for (name, at) in [
            ("mascot-10-dash", 0.3),
            ("mascot-11-windup", 0.75),
            ("mascot-12-punch", 0.95),
            ("mascot-13-ouch", 1.5),
            ("mascot-14-guard", 3.0),
            ("mascot-15-shatter", 5.5),
            ("mascot-16-after", 6.4),
        ] {
            until(&mut app, from, at);
            save(&mut app, out, name)?;
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--alcove") {
        // Her own bed: the menu over her lying there, exploring, teasing Chiquita, the bed scene
        // with her in bed behind it, and a night's sleep.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-15.5, 0.0, -16.6);
        app.castle = None;
        for _ in 0..90 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        app.story = guion::Vars::default();
        app.story.values.insert("deseo".into(), 6);
        app.story.values.insert("chiquita_afecto".into(), 3);
        app.open_alcove();
        eprintln!("[alcove-qa] open: {}", app.alcove.is_some());
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.render_frame();
            }
        };
        wait(&mut app, 0.25);
        save(&mut app, out, "alcove-00-entering")?;
        wait(&mut app, 1.2);
        save(&mut app, out, "alcove-01-bed")?;
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::ArrowRight);
        wait(&mut app, 0.3);
        save(&mut app, out, "alcove-02-seal-lit")?;
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::Enter);
        wait(&mut app, 1.2);
        save(&mut app, out, "alcove-03-chiquita")?;
        app.alcove_key(KeyCode::Enter);
        wait(&mut app, 0.35);
        save(&mut app, out, "alcove-04-tease-queen")?;
        wait(&mut app, 1.4);
        save(&mut app, out, "alcove-05-tease-reply")?;
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::Enter);
        wait(&mut app, 1.8);
        save(&mut app, out, "alcove-06-goodnight")?;
        app.alcove_key(KeyCode::Escape);
        wait(&mut app, 0.6);
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::Enter);
        wait(&mut app, 1.2);
        save(&mut app, out, "alcove-07-explore")?;
        app.alcove_key(KeyCode::ArrowRight);
        app.alcove_key(KeyCode::Enter);
        let mut lines = 0;
        while app.talk.is_some() && lines < 10 {
            app.elapsed += 5.0;
            if lines == 0 {
                save(&mut app, out, "alcove-08-guion")?;
            }
            app.talk_confirm();
            lines += 1;
        }
        eprintln!("[alcove-qa] bed scene: {}", app.bed_scene.is_some());
        let run = |app: &mut App, seconds: f32, hold: bool| {
            app.bed_scene_button(hold);
            for _ in 0..(seconds * 30.0) as usize {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
        };
        run(&mut app, 2.5, false);
        save(&mut app, out, "alcove-09-scene-asks")?;
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.answer(0);
        }
        run(&mut app, 0.8, false);
        save(&mut app, out, "alcove-09b-scene-reply")?;
        run(&mut app, 6.0, true);
        save(&mut app, out, "alcove-10-scene-hold")?;
        if let Some(scene) = app.bed_scene.as_mut() {
            scene.answer(1);
        }
        run(&mut app, 1.0, true);
        save(&mut app, out, "alcove-10b-thinking-of-you")?;
        app.bed_scene_key(KeyCode::Escape, true, false);
        run(&mut app, 1.6, false);
        app.bed_scene_key(KeyCode::KeyF, true, false);
        wait(&mut app, 1.2);
        save(&mut app, out, "alcove-11-back")?;
        app.alcove_key(KeyCode::Digit1);
        wait(&mut app, 0.6);
        save(&mut app, out, "alcove-12-sleeping")?;
        wait(&mut app, 1.1);
        save(&mut app, out, "alcove-13-dawn")?;
        wait(&mut app, 1.6);
        save(&mut app, out, "alcove-14-up")?;
        eprintln!("[alcove-qa] closed: {}", app.alcove.is_none());
        return Ok(());
    }
    if std::env::args().any(|a| a == "--bed-scene") {
        // The bed: its menu and a line in the dialogue box, then the bed scene's panel through
        // its four parts (held all the way) and the closing card.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.player = Vec3::new(-17.6, 0.0, -12.6);
        app.castle = None;
        app.iso.yaw = std::f32::consts::FRAC_PI_4;
        app.iso.yaw_target = app.iso.yaw;
        app.iso.distance = ISO.max_distance;
        for _ in 0..90 {
            app.tick(1.0 / 60.0);
            if let Some(inside) = &app.castle {
                app.camera_focus = inside.focus();
            }
            app.render_frame();
        }
        app.story = guion::Vars::default();
        app.story.values.insert("deseo".into(), 3);
        if app.start_talk(None, Some("cama")) {
            app.elapsed += 3.0;
            save(&mut app, out, "bed-line")?;
            app.talk_confirm();
            app.elapsed += 3.0;
            save(&mut app, out, "bed-menu")?;
            // The way a player goes: «Explorar el cuerpo», then «Dejarse llevar del todo».
            app.talk_pick(1);
            for step in 0..40 {
                if app.talk.is_none() {
                    break;
                }
                let lib = guion::library();
                let shown = app.talk.as_ref().map(|t| t.shown(lib, &app.story));
                eprintln!("[bed-qa] step {step}: {shown:?}");
                if matches!(&shown, Some(guion::Shown::Line { who, .. }) if who == "chiquita") {
                    app.elapsed += 5.0;
                    save(&mut app, out, "bed-chiquita")?;
                }
                match shown {
                    Some(guion::Shown::Choice { .. }) => app.talk_pick(2),
                    _ => {
                        app.elapsed += 5.0;
                        app.talk_confirm();
                    }
                }
            }
            eprintln!(
                "[bed-qa] talk over: {}, scene open: {}",
                app.talk.is_none(),
                app.bed_scene.is_some()
            );
        }
        if app.bed_scene.is_none() {
            app.open_bed_scene(false);
        }
        let run = |app: &mut App, seconds: f32, hold: bool| {
            app.bed_scene_button(hold);
            let steps = (seconds * 30.0) as usize;
            for _ in 0..steps {
                app.tick(1.0 / 30.0);
                app.bed_scene_settle();
                app.elapsed += 1.0 / 30.0;
            }
        };
        run(&mut app, 2.5, false);
        save(&mut app, out, "bed-scene-intro")?;
        run(&mut app, 6.0, true);
        save(&mut app, out, "bed-scene-loop")?;
        run(&mut app, 8.0, true);
        save(&mut app, out, "bed-scene-climax")?;
        app.bed_scene_key(KeyCode::Escape, true, false);
        run(&mut app, 0.45, false);
        save(&mut app, out, "bed-scene-card-in")?;
        run(&mut app, 1.4, false);
        save(&mut app, out, "bed-scene-card")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--burdel-vn-spots") {
        // QA only: the brothel's door from the street at night, and inside, the places its VN
        // scenes happen (the desk, the fire, the bar, the waiting room, the door), seen at eye
        // height from round each one: references for the VN backgrounds. Run it with
        // VELVET_BROTHEL_EMPTY so the house is without its people; the Queen stands behind the
        // camera. The game's own camera is not touched outside this capture.
        app.nature_mode = true;
        app.village_entered = true;
        if let Some(state) = app.state.as_mut() {
            state.map_overlay.hidden = true;
        }
        app.environment_cycle.set_phase(0.92);
        let shoot = |app: &mut App, name: &str, eye: Vec3, target: Vec3| {
            app.camera_override = Some(Camera3D {
                position: eye,
                target,
                up: Vec3::Y,
                fov_y_radians: 62f32.to_radians(),
                orthographic_height: None,
                near: 0.05,
                far: 220.0,
            });
            for _ in 0..90 {
                app.elapsed += 1.0 / 60.0;
                app.render_frame();
            }
            if let Err(error) = save(app, out, name) {
                eprintln!("[burdel-vn] {name}: {error:#}");
            }
        };
        // The street: the door seen from across the way, on an arc in front of it.
        let door = village::building_anchors()
            .iter()
            .find(|a| a.id == "villa_burdel")
            .and_then(|a| village::door_anchor(a.footprint_index));
        if let Some((door, facing)) = door {
            let base = facing.z.atan2(facing.x);
            for (k, turn) in [-0.7f32, -0.35, 0.0, 0.35, 0.7].iter().enumerate() {
                let a = base + turn;
                let at = door + Vec3::new(a.cos(), 0.0, a.sin()) * 9.0;
                let eye = Vec3::new(at.x, village::surface_height(at.x, at.z) + 1.65, at.z);
                let away = (eye - door).normalize_or_zero();
                app.player = eye + away * 2.0;
                shoot(
                    &mut app,
                    &format!("burdel-calle-{k}"),
                    eye,
                    door + Vec3::Y * 2.2,
                );
            }
        }
        // Inside: round each place, the Queen behind the camera.
        app.player = village::entrance();
        app.enter_brothel();
        app.queen_thought = None;
        let plan = brothel::plan();
        for (name, id, distance, lift) in [
            ("recepcion", "recepcionista", 3.4f32, 1.3f32),
            ("salon", "lounge", 3.6, 1.1),
            ("barra", "servicio", 3.4, 1.2),
            ("espera", "espera", 3.0, 0.9),
            ("puerta", "salir", 3.5, 1.5),
        ] {
            let Some(h) = plan.hotspots.iter().find(|h| h.id == id) else {
                continue;
            };
            for (k, az) in [
                0.0f32, 0.7854, 1.5708, 2.3562, 3.1416, 3.927, 4.7124, 5.4978,
            ]
            .iter()
            .enumerate()
            {
                let (cx, cz) = (h.x + az.sin() * distance, h.z + az.cos() * distance);
                let (bx, bz) = (
                    h.x + az.sin() * (distance + 1.6),
                    h.z + az.cos() * (distance + 1.6),
                );
                let Some(inside) = app.brothel.as_mut() else {
                    break;
                };
                inside.walker = brothel::Walker {
                    x: bx,
                    z: bz,
                    level: h.level,
                    on_stair: false,
                };
                inside.talk = None;
                let floor = inside.position().y;
                app.player = inside.position();
                app.camera_focus = app.player;
                shoot(
                    &mut app,
                    &format!("burdel-{name}-{k}"),
                    Vec3::new(cx, floor + 1.65, cz),
                    Vec3::new(h.x, floor + lift, h.z),
                );
            }
        }
        app.camera_override = None;
        if let Some(state) = app.state.as_mut() {
            state.map_overlay.hidden = false;
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--selfie-spots") {
        // QA only: the baths seen from where a phone held at arm's length would be (eye height, a few
        // metres from a landmark, looking at it), the Queen out of the picture: references for the
        // backgrounds of the selfie CGs. The game's own camera is not touched outside this capture.
        app.nature_mode = true;
        app.village_entered = true;
        // No interface in these pictures (map, prompts): only the world.
        if let Some(state) = app.state.as_mut() {
            state.map_overlay.hidden = true;
        }
        let plan = baths::plan();
        let mut spots: Vec<(&str, Option<&str>, f32, f32, f32, f32)> = vec![
            // name, room, landmark x, z, distance, landmark height
            ("cascada", None, 0.2, -7.6, 5.5, 1.2),
            ("puente", None, 2.0, -8.62, 5.0, 1.0),
            ("pozas", None, 4.0, -3.2, 6.0, 0.6),
            ("tumbonas", None, 13.6, 0.2, 5.0, 0.8),
            ("torii", None, -8.2, 10.6, 6.0, 2.0),
            ("pabellon", None, 10.4, -4.0, 6.0, 1.6),
            ("recepcion", Some("recepcion"), 403.5, 0.4, 2.6, 1.3),
            ("privada", Some("privada"), 402.7, 3.0, 2.6, 1.1),
        ];
        if let Some((x, z)) = baths::capybara_focus() {
            spots.insert(0, ("capibaras", None, x, z, 3.2, 0.3));
        }
        for (phase, tag) in [(0.5f32, "dia"), (0.74, "atardecer")] {
            app.environment_cycle.set_phase(phase);
            for &(name, room, x, z, distance, lift) in &spots {
                if room.is_some() && tag != "atardecer" {
                    continue;
                }
                for (k, az) in [
                    0.0f32, 1.5708, 3.1416, 4.7124, 0.7854, 2.3562, 3.927, 5.4978,
                ]
                .iter()
                .enumerate()
                {
                    let (cx, cz) = (x + az.sin() * distance, z + az.cos() * distance);
                    let ground = |x: f32, z: f32| {
                        if room.is_some() {
                            0.0
                        } else {
                            plan.stand(x, z)
                        }
                    };
                    let mut state = baths::State::enter(Vec3::ZERO, 0.0);
                    state.room = room.map(str::to_owned);
                    (state.x, state.z) = (x, z);
                    app.baths = Some(state);
                    app.player = Vec3::new(5000.0, 0.0, 5000.0);
                    app.camera_focus = Vec3::new(x, ground(x, z), z);
                    app.camera_override = Some(Camera3D {
                        position: Vec3::new(cx, ground(cx, cz) + 1.65, cz),
                        target: Vec3::new(x, ground(x, z) + lift, z),
                        up: Vec3::Y,
                        fov_y_radians: 62f32.to_radians(),
                        orthographic_height: None,
                        near: 0.05,
                        far: 220.0,
                    });
                    for _ in 0..90 {
                        app.elapsed += 1.0 / 60.0;
                        app.render_frame();
                    }
                    // A view into the void outside a cut-away room is all black: skip it.
                    if let Err(error) = save(&mut app, out, &format!("selfie-{name}-{tag}-{k}")) {
                        eprintln!("[selfie] {name}-{tag}-{k}: {error:#}");
                    }
                }
            }
        }
        app.camera_override = None;
        app.baths = None;
        if let Some(state) = app.state.as_mut() {
            state.map_overlay.hidden = false;
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--baths-only") {
        // The hot springs: the gate, the pools (wading), the falls, by day, dusk and night; the rooms.
        app.nature_mode = true;
        app.village_entered = true;
        let stops: [(&str, Option<&str>, f32, f32, f32); 13] = [
            ("baths-gate", None, -8.2, 10.6, 18.0),
            ("baths-close", None, -1.2, -13.4, 6.0),
            ("baths-cascade", None, 0.2, -7.6, 9.0),
            ("baths-brook", None, -0.8, 2.4, 9.0),
            ("baths-bridge", None, 2.0, -8.62, 7.0),
            ("baths-deck", None, -0.9, -4.0, 6.0),
            ("baths-loungers", None, 13.6, 0.2, 8.0),
            ("baths-pools", None, 4.0, -3.2, 14.0),
            ("baths-upper", None, -1.5, -13.0, 14.0),
            ("baths-lower", None, -2.0, 6.8, 14.0),
            ("baths-pavilion", None, 10.4, -4.0, 14.0),
            ("baths-recepcion", Some("recepcion"), 403.5, 0.4, 11.0),
            ("baths-privada", Some("privada"), 402.7, 3.0, 11.0),
        ];
        for (phase, tag) in [(0.5f32, "day"), (0.74, "dusk"), (0.0, "night")] {
            app.environment_cycle.set_phase(phase);
            for (name, room, x, z, distance) in stops {
                if room.is_some() && tag != "dusk" {
                    continue;
                }
                let mut state = baths::State::enter(Vec3::ZERO, 0.0);
                state.room = room.map(str::to_owned);
                (state.x, state.z) = (x, z);
                app.player = state.position();
                app.camera_focus = app.player;
                app.iso.distance = distance;
                app.baths = Some(state);
                for _ in 0..120 {
                    app.elapsed += 1.0 / 60.0;
                    app.render_frame();
                }
                save(&mut app, out, &format!("{name}-{tag}"))?;
            }
            // The capybaras, close, from the shore side (the Queen a step away).
            if let Some((x, z)) = baths::capybara_focus() {
                let mut state = baths::State::enter(Vec3::ZERO, 0.0);
                (state.x, state.z) = (x + 2.2, z - 1.0);
                app.player = state.position();
                app.camera_focus = Vec3::new(x, app.player.y, z);
                app.iso.distance = 4.5;
                app.baths = Some(state);
                for _ in 0..120 {
                    app.elapsed += 1.0 / 60.0;
                    app.render_frame();
                }
                save(&mut app, out, &format!("baths-capy-{tag}"))?;
            }
        }
        app.baths = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mine-only") {
        // The mine mouth by the trail, from the Queen at its lip.
        app.nature_mode = true;
        app.village_entered = false;
        app.player = mine::entry_position() - Vec3::new(1.2, 0.0, 0.3);
        app.camera_focus = app.player;
        for (distance, name) in [(14.0, "mine-overview"), (8.0, "mine-close")] {
            app.iso.distance = distance;
            save(&mut app, out, name)?;
        }
        app.environment_cycle.set_phase(0.0);
        save(&mut app, out, "mine-night")?;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--village-hours") {
        // The villa through the evening, every 15 s of the 7-minute day: the light should melt
        // from one frame to the next, never jump.
        app.nature_mode = true;
        app.village_entered = true;
        let (plaza, _) = village::clearing();
        app.player = plaza + Vec3::new(2.4, 0.0, 2.0);
        app.camera_focus = app.player;
        app.iso.distance = 16.0;
        for k in 0..16 {
            let seconds = 250 + k * 15;
            app.environment_cycle.set_phase(seconds as f32 / 420.0);
            save(&mut app, out, &format!("village-hour-{seconds:03}"))?;
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--village-only") {
        // Quick villa review: the Queen on the main street, from above.
        app.nature_mode = true;
        app.village_entered = true;
        app.player = village::entrance() + Vec3::new(0.0, 0.0, -9.0);
        app.camera_focus = app.player;
        for (distance, name) in [(18.0, "village-overview"), (9.0, "village-close")] {
            app.iso.distance = distance;
            save(&mut app, out, name)?;
        }
        // At the tavern door: the medallion and the on-screen prompt.
        let tavern = village::building_anchors()
            .iter()
            .find(|a| a.id == "villa_taberna")
            .map(|a| a.footprint_index);
        if let Some((door, out_dir)) = tavern.and_then(village::door_anchor) {
            app.player = door + out_dir * 1.4;
            app.camera_focus = app.player;
            app.iso.distance = 9.0;
            save(&mut app, out, "village-prompt")?;
            app.player = door + out_dir * 4.2;
            app.camera_focus = app.player;
            save(&mut app, out, "village-prompt-far")?;
        }
        // Where she comes in, the plaza, and every door.
        let (spawn, _) = village::spawn();
        app.player = spawn;
        app.camera_focus = app.player;
        app.iso.distance = 16.0;
        save(&mut app, out, "village-spawn")?;
        let (plaza, _) = village::clearing();
        app.player = plaza + Vec3::new(2.4, 0.0, 2.0);
        app.camera_focus = app.player;
        save(&mut app, out, "village-plaza")?;
        for anchor in village::building_anchors() {
            if let Some((door, out_dir)) = village::door_anchor(anchor.footprint_index) {
                app.player = door + out_dir * 2.2;
                app.camera_focus = app.player;
                app.iso.distance = 13.0;
                save(&mut app, out, &format!("village-{}", anchor.id))?;
            }
        }
        app.player = plaza + Vec3::new(2.4, 0.0, 2.0);
        app.camera_focus = app.player;
        app.iso.distance = 16.0;
        // The same at dusk (the prototype's default hour), and at night.
        app.environment_cycle.set_phase(0.74);
        save(&mut app, out, "village-dusk")?;
        app.player = spawn;
        app.camera_focus = app.player;
        save(&mut app, out, "village-spawn-dusk")?;
        app.player = plaza + Vec3::new(2.4, 0.0, 2.0);
        app.camera_focus = app.player;
        app.environment_cycle.set_phase(0.0);
        save(&mut app, out, "village-night")?;
        return Ok(());
    }
    save(&mut app, out, "castle-toon")?;
    app.nature_mode = true;
    app.village_entered = true;
    app.player = village::entrance() + Vec3::new(0.0, 0.0, -5.0);
    app.camera_focus = app.player;
    app.iso.distance = 16.0;
    save(&mut app, out, "village-toon")?;
    app.environment_cycle.set_phase(0.0);
    save(&mut app, out, "village-night-toon")?;
    app.environment_cycle.set_phase(0.30);
    app.shop_open = true;
    for (id, name) in [
        ("pocion_pequena", "shop-potion-toon"),
        ("pocion_misteriosa", "shop-pink-toon"),
        ("anillo_misterioso", "shop-ring-toon"),
        ("espada_de_madera", "shop-sword-toon"),
    ] {
        app.shop_selection = shop::catalog()
            .products
            .iter()
            .position(|p| p.id == id)
            .with_context(|| format!("toon QA product missing: {id}"))?;
        app.shop_preview_rotation = Some((0.45, 0.24));
        save(&mut app, out, name)?;
    }
    Ok(())
}

/// Load and drop the whole 3D world `cycles` times on one device.
pub(super) fn load_cycle(cycles: usize) -> Result<()> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: if cfg!(windows) {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::all()
        },
        ..Default::default()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .context("load-cycle adapter")?;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
            ..Default::default()
        },
        None,
    ))?;
    for cycle in 0..cycles {
        let gpu = GpuContext {
            device: device.clone(),
            queue: queue.clone(),
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            msaa_samples: 4,
        };
        let started = std::time::Instant::now();
        let state = App::prepare(gpu, 42)?;
        drop(state);
        let _ = device.poll(wgpu::Maintain::Wait);
        eprintln!(
            "[load-cycle] {cycle}: loaded and released in {:.1}s",
            started.elapsed().as_secs_f32()
        );
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    Ok(())
}

/// The hypnotist's game played by a script (`--play-scene hipno|hipno-escape|hipno-fall`): resisting
/// until the ring has her or she wakes, answering his orders, Chiquita's numbers, the mouth, the end.
fn hipno_qa(
    app: &mut App,
    out: &Path,
    which: &str,
    save: fn(&mut App, &Path, &str) -> Result<()>,
) -> Result<()> {
    let tag = which.replace('-', "_");
    let step = |app: &mut App, seconds: f32| {
        for _ in 0..(seconds * 30.0).round() as usize {
            app.tick(1.0 / 30.0);
            app.bed_scene_settle();
            app.elapsed += 1.0 / 30.0;
        }
    };
    let key = |app: &mut App, code: KeyCode, pressed: bool| app.bed_scene_key(code, pressed, false);
    let tap = |app: &mut App, code: KeyCode| {
        app.bed_scene_key(code, true, false);
        app.bed_scene_key(code, false, false);
    };
    let qa = |app: &App| app.play_scene_qa();
    let shot = |app: &mut App, name: &str| save(app, out, &format!("{tag}-{name}"));
    let escape = which == "hipno-escape";
    let fall = which == "hipno-fall";
    step(app, 5.0);
    shot(app, "ring")?;
    // Resisting (escape): push the gaze away from the ring; otherwise stare at it on purpose.
    let mut answered = 0;
    let mut seen = [false; 5];
    let mut gags = 0;
    for i in 0..6000 {
        let Some(q) = qa(app) else { break };
        if q.phase != "play" || q.mouth {
            break;
        }
        if escape {
            let away_right = q.gaze >= q.ring;
            key(app, KeyCode::KeyD, away_right);
            key(app, KeyCode::KeyA, !away_right);
        } else {
            key(app, KeyCode::Space, i % 120 < 80);
        }
        if q.order {
            step(app, 1.2);
            if answered == 0 {
                shot(app, "order")?;
            }
            let pick = if escape {
                [KeyCode::Digit3, KeyCode::Digit2][answered % 2]
            } else if fall {
                KeyCode::Digit1
            } else {
                [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3][answered % 3]
            };
            tap(app, pick);
            step(app, 0.35);
            if answered < 3 {
                shot(app, &format!("answer{answered}"))?;
            }
            answered += 1;
        }
        if q.stage < 5 && !seen[q.stage] {
            seen[q.stage] = true;
            if q.stage > 0 {
                step(app, 1.0);
                shot(app, &format!("stage{}", q.stage))?;
            }
        }
        // Chiquita's two numbers, once each.
        if gags == 0 && (q.trance > 0.1 || q.awake > 0.15) {
            gags = 1;
            app.play_scene_force_gag(true);
            step(app, 0.8);
            shot(app, "cover")?;
        } else if gags == 1 && (q.trance > 0.3 || q.awake > 0.45) {
            gags = 2;
            app.play_scene_force_gag(false);
            step(app, 1.2);
            shot(app, "dazed")?;
        }
        if i % 300 == 0 {
            eprintln!(
                "[hipno-qa] stage {} trance {:.2} awake {:.2} gaze {:.2} ring {:.2} clip {}",
                q.stage, q.trance, q.awake, q.gaze, q.ring, q.clip
            );
        }
        step(app, 0.1);
    }
    for k in [KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space] {
        key(app, k, false);
    }
    if escape {
        // The punch, Chiquita's jab, her cheer.
        for k in 0..26 {
            step(app, 0.1);
            if let Some(v) = app.mascot_view() {
                eprintln!(
                    "[hipno-qa] jab {k}: feet {:?} clip {} h {}",
                    v.feet, v.clip, v.height
                );
            }
            if [3, 9, 24].contains(&k) {
                shot(app, &format!("escape{k}"))?;
            }
        }
    } else {
        // Into the mouth.
        step(app, 3.0);
        shot(app, "mouth")?;
        step(app, 4.0);
        if fall {
            // Hard, until it is all over.
            key(app, KeyCode::Space, true);
            for _ in 0..1200 {
                if qa(app).is_some_and(|q| q.phase != "play") {
                    break;
                }
                step(app, 0.1);
            }
            key(app, KeyCode::Space, false);
            step(app, 2.6);
            shot(app, "climax")?;
        } else {
            tap(app, KeyCode::KeyD);
            step(app, 2.0);
            shot(app, "deep")?;
            // Waking with the salts and E, until the bite.
            tap(app, KeyCode::KeyQ);
            step(app, 0.3);
            shot(app, "salts")?;
            for _ in 0..400 {
                if qa(app).is_some_and(|q| q.phase != "play") {
                    break;
                }
                tap(app, KeyCode::KeyE);
                step(app, 0.12);
            }
            step(app, 0.6);
            shot(app, "bite")?;
            step(app, 4.0);
            shot(app, "bite2")?;
        }
    }
    Ok(())
}
