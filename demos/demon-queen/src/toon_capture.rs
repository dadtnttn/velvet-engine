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
    // VELVET_I18N_DUMP=<file>: every text the guiones show (names, lines, prompts, options, what
    // the stage writes), for the translation catalogs (tools/i18n_extract.py reads it).
    if let Ok(path) = std::env::var("VELVET_I18N_DUMP") {
        let lib = guion::library();
        let mut texts = std::collections::BTreeSet::new();
        for g in &lib.guiones {
            for scene in &g.scenes {
                for step in &scene.steps {
                    match step {
                        guion::Step::Line { name, text, .. } => {
                            texts.insert(name.clone());
                            texts.insert(text.clone());
                        }
                        guion::Step::Choice {
                            prompt, options, ..
                        } => {
                            texts.extend(prompt.clone());
                            texts.extend(options.iter().map(|o| o.label.clone()));
                        }
                        guion::Step::Stage { arg, .. } => {
                            texts.insert(arg.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
        texts.retain(|t| !t.trim().is_empty());
        std::fs::write(&path, serde_json::to_string_pretty(&texts)?)?;
        eprintln!("[i18n] {} guion texts in {path}", texts.len());
        return Ok(());
    }
    // VELVET_LANG=<code>: the world drawn in that language.
    if let Ok(code) = std::env::var("VELVET_LANG") {
        demon_queen_dungeon::i18n::set_locale_dir(exploration3d_state::data_dir().join("locales"));
        demon_queen_dungeon::i18n::set_language(&code);
        velvet_guion::set_translator(demon_queen_dungeon::i18n::tr_string);
        velvet_render::set_image_override(demon_queen_dungeon::i18n::image_override);
    }
    fs::create_dir_all(out)?;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: if cfg!(windows) {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::all()
        },
        ..Default::default()
    });
    // `VELVET_ADAPTER=intel`: the graphics card whose name holds that (to measure a weak one).
    let wanted = std::env::var("VELVET_ADAPTER")
        .ok()
        .map(|n| n.to_lowercase());
    let adapter = match wanted.and_then(|n| {
        instance
            .enumerate_adapters(wgpu::Backends::all())
            .into_iter()
            .find(|a| a.get_info().name.to_lowercase().contains(&n))
    }) {
        Some(adapter) => adapter,
        None => {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .context("toon QA adapter")?
        }
    };
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
        // The game itself draws without MSAA; `VELVET_MSAA=1` measures it as it plays.
        msaa_samples: std::env::var("VELVET_MSAA")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4),
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
        .skip_while(|a| a != "--bench-all")
        .nth(1)
        .and_then(|n| n.parse::<usize>().ok())
    {
        // Frame cost in every place of the game: each zone entered, then `frames` frames of it
        // ticked and drawn (the GPU waited for each, so CPU and GPU add up), the player walking a
        // small circle where there is a player. One line per zone: avg/p95 frame, CPU split.
        // `--bench-only a,b` keeps some.
        use zone_war::WarPage;
        let only: Option<Vec<String>> = std::env::args()
            .skip_while(|a| a != "--bench-only")
            .nth(1)
            .map(|s| s.split(',').map(str::to_owned).collect());
        let take =
            |a: &std::sync::atomic::AtomicU64| a.swap(0, std::sync::atomic::Ordering::Relaxed);
        fn reset(app: &mut App) {
            if app.dungeon_mode {
                app.leave_dungeon();
            }
            if app.war.is_some() {
                app.leave_war();
            }
            app.boss = None;
            app.baths = None;
            app.brothel = None;
            app.penascal = None;
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = false;
        }
        let zones: [(&str, fn(&mut App) -> bool); 10] = [
            ("castillo", |app| {
                app.enter_castle_from_nature();
                true
            }),
            ("villa", |app| {
                let (spawn, _) = village::spawn();
                app.player = spawn;
                app.camera_focus = spawn;
                app.village_entered = true;
                true
            }),
            ("bosque", |app| {
                app.player = mine::entry_position() - Vec3::new(3.0, 0.0, 2.0);
                app.camera_focus = app.player;
                true
            }),
            ("termas", |app| {
                app.village_entered = true;
                app.enter_baths();
                true
            }),
            ("burdel", |app| {
                app.village_entered = true;
                app.enter_brothel();
                true
            }),
            ("penascal", |app| {
                app.enter_penascal_from_forest();
                true
            }),
            ("varrak", |app| {
                app.story.values.insert("consejo_2_hecho".into(), 1);
                app.enter_penascal_from_forest();
                app.start_boss_fight();
                if let Some(f) = app.boss.as_mut() {
                    f.player.god = true;
                    f.qa("duel");
                }
                app.boss.is_some()
            }),
            ("mina", |app| {
                app.enter_dungeon_unchecked();
                app.dungeon_mode
            }),
            ("campana", |app| {
                app.open_war_table();
                app.war_campaign = None;
                if let Some(w) = app.war.as_mut() {
                    w.page = WarPage::Table(0);
                }
                app.war_hit(map_overlay::WarHit::Table(1));
                for _ in 0..30 {
                    app.tick(1.0 / 30.0);
                    app.render_frame();
                }
                app.war_hit(map_overlay::WarHit::Modal(0));
                app.war.is_some()
            }),
            ("batalla", |app| {
                app.open_war_table();
                if let Some(w) = app.war.as_mut() {
                    w.page = WarPage::Table(1);
                    w.battle = None;
                }
                app.war_hit(map_overlay::WarHit::Quick(0));
                for _ in 0..20 {
                    app.tick(1.0 / 30.0);
                    app.render_frame();
                }
                app.war_hit(map_overlay::WarHit::Begin);
                if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                    run.b.auto_player = Some(demon_queen_war::ai::BattleAi::new(
                        0,
                        demon_queen_war::ai::Posture::Attack,
                    ));
                    run.b.speed = 2.0;
                }
                app.war.as_ref().is_some_and(|w| w.battle.is_some())
            }),
        ];
        let mut rows = Vec::new();
        for (name, enter) in zones {
            if only.as_ref().is_some_and(|o| !o.iter().any(|z| z == name)) {
                continue;
            }
            reset(&mut app);
            if !enter(&mut app) {
                eprintln!("[bench] {name}: could not enter");
                continue;
            }
            let walks = !app.dungeon_mode && app.war.is_none() && app.boss.is_none();
            let p0 = app.player;
            let mut times = Vec::with_capacity(frames);
            let mut cpu = Vec::with_capacity(frames);
            let (mut render_ns, mut overlay_ns) = (0u64, 0u64);
            let mut tick_ms = 0.0f32;
            for frame in 0..frames + 40 {
                let t = frame as f32 / 60.0;
                if walks {
                    app.player =
                        p0 + Vec3::new((t * 0.6).cos() * 2.0 - 2.0, 0.0, (t * 0.6).sin() * 2.0);
                    app.camera_focus = app.player;
                }
                app.elapsed += 1.0 / 60.0;
                let started = std::time::Instant::now();
                app.tick(1.0 / 60.0);
                let ticked = started.elapsed().as_secs_f32() * 1000.0;
                app.render_frame();
                let submitted = started.elapsed().as_secs_f32() * 1000.0;
                app.wait_for_gpu();
                let (r, o) = (
                    take(&velvet_render::RENDER3D_CPU_NS),
                    take(&map_overlay::OVERLAY_CPU_NS),
                );
                if frame >= 40 {
                    tick_ms += ticked;
                    times.push(started.elapsed().as_secs_f32() * 1000.0);
                    cpu.push(submitted);
                    render_ns += r;
                    overlay_ns += o;
                }
            }
            if std::env::args().any(|a| a == "--bench-shots") {
                save(&mut app, out, &format!("bench-{name}"))?;
            }
            times.sort_by(|a, b| a.total_cmp(b));
            let avg = times.iter().sum::<f32>() / times.len().max(1) as f32;
            let p95 = times[((times.len() as f32 * 0.95) as usize).min(times.len() - 1)];
            let n = frames.max(1) as f32 * 1e6;
            let cpu_avg = cpu.iter().sum::<f32>() / cpu.len().max(1) as f32;
            let (rend, ov) = (render_ns as f32 / n, overlay_ns as f32 / n);
            let tick = tick_ms / frames.max(1) as f32;
            eprintln!(
                "[bench] {name:9} frame avg {avg:6.2} ms p95 {p95:6.2} | cpu {cpu_avg:5.2} (render {rend:5.2}, overlay {ov:4.2}, tick {tick:4.2}, draws {:5.2}) | gpu~{:5.2}",
                cpu_avg - rend - ov - tick,
                (avg - cpu_avg).max(0.0)
            );
            rows.push((name, avg, p95, cpu_avg));
        }
        let total: f32 = rows.iter().map(|r| r.1).sum();
        eprintln!(
            "[bench] {} zones, mean frame {:.2} ms",
            rows.len(),
            total / rows.len().max(1) as f32
        );
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
            let take =
                |a: &std::sync::atomic::AtomicU64| a.swap(0, std::sync::atomic::Ordering::Relaxed);
            let (mut render_ns, mut overlay_ns) = (0u64, 0u64);
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
                let (r, o) = (
                    take(&velvet_render::RENDER3D_CPU_NS),
                    take(&map_overlay::OVERLAY_CPU_NS),
                );
                if frame >= 30 {
                    times.push(started.elapsed().as_secs_f32() * 1000.0);
                    cpu.push(submitted);
                    render_ns += r;
                    overlay_ns += o;
                }
            }
            let n = frames.max(1) as f64 * 1e6;
            let cpu_avg = cpu.iter().sum::<f32>() as f64 / cpu.len().max(1) as f64;
            eprintln!(
                "[frame-times] {name} cpu split: renderer {:.2} ms, overlay {:.2} ms, game {:.2} ms",
                render_ns as f64 / n,
                overlay_ns as f64 / n,
                cpu_avg - (render_ns + overlay_ns) as f64 / n
            );
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
            // In the royal dress the hypnotist only stammers.
            if name == "prompt-mage" {
                app.outfit = 0;
                app.interact_forest();
                for _ in 0..30 {
                    app.tick(1.0 / 30.0);
                    app.elapsed += 1.0 / 30.0;
                }
                save(&mut app, out, "mage-stammer")?;
            }
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
                    // The tavern has nothing behind its door yet: F says so.
                    if id == "villa_taberna" {
                        app.interact_village();
                        for _ in 0..24 {
                            app.tick(1.0 / 30.0);
                            app.elapsed += 1.0 / 30.0;
                        }
                        save(&mut app, out, "tavern-in-development")?;
                    }
                }
            }
        }
        // The brothel's door: by day a note; at night in the cloak she goes in, hood up.
        let settle = |app: &mut App| {
            for _ in 0..24 {
                app.tick(1.0 / 30.0);
                app.elapsed += 1.0 / 30.0;
            }
        };
        if let Some(anchor) = village::building_anchors()
            .iter()
            .find(|a| a.id == "villa_burdel")
        {
            if let Some((door, out_dir)) = village::door_anchor(anchor.footprint_index) {
                app.player = door + out_dir * 1.4;
                app.camera_focus = app.player;
                app.environment_cycle.set_phase(0.5);
                app.interact_village();
                settle(&mut app);
                save(&mut app, out, "brothel-day-note")?;
                app.environment_cycle.set_phase(0.95);
                app.inventory.insert(rules::CAPE_ITEM.to_owned(), 1);
                app.on_bought(rules::CAPE_ITEM);
                settle(&mut app);
                save(&mut app, out, "cape-bought")?;
                app.interact_village();
                app.talk = None;
                app.stage = None;
                settle(&mut app);
                save(&mut app, out, "brothel-in-cape")?;
                app.leave_brothel();
                app.environment_cycle.set_phase(0.5);
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
            // A place this run has not loaded (the castle when the world starts outdoors) shows
            // nothing: it is skipped, not the whole run.
            if let Err(error) = save(&mut app, out, &format!("map-mini-{name}")) {
                eprintln!("[toon-qa] map {name}: {error}");
                continue;
            }
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
        for _ in 0..300 {
            if app.play_scene_qa().is_some_and(|q| q.heat >= 0.55) {
                break;
            }
            step(&mut app, 0.1);
        }
        save(&mut app, out, "favor-hurry")?;
        for _ in 0..300 {
            if app.play_scene_qa().is_some_and(|q| q.phase == "relax") {
                break;
            }
            step(&mut app, 0.1);
        }
        step(&mut app, 0.25);
        save(&mut app, out, "favor-sealed")?;
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
        // The same day, another haggle: he offers the one done; the next waits for tomorrow.
        app.haggle = None;
        app.start_haggle();
        app.shop_page_at = app.elapsed - 10.0;
        if let Some(h) = app.haggle.as_mut() {
            h.offer_at = app.elapsed - 2.0;
        }
        save(&mut app, out, "favor-manana")?;
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
            "capa_reina",
            "espada_reliquia",
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
            // Its ticket: a flattery that worked, a press that missed, a press dead on.
            for (tile, outcome, moved) in [
                (0, shop_games::Outcome::Good, 4),
                (1, shop_games::Outcome::Miss, 0),
                (1, shop_games::Outcome::Perfect, 11),
            ] {
                h.log.push(shop_games::Try {
                    tile,
                    outcome,
                    moved,
                    favor: None,
                });
            }
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
        // VELVET_SWITCH_LANG=<code>: the world loaded in one language, then switched to this one
        // (its signs drawn again in place, as when the player changes it in the menu).
        if let Ok(code) = std::env::var("VELVET_SWITCH_LANG") {
            demon_queen_dungeon::i18n::set_language(&code);
            app.relocalize_signs();
        }
        let stops: [(&str, Vec3, f32); 8] = [
            (
                "forest-sign",
                Vec3::new(nature_trail_center(-2.2) - 1.2, 0.0, -1.2),
                6.0,
            ),
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
        // The cloak bought: its own card, next to the royal dress.
        app.achievements.grant_outfit("capa");
        for (name, selection) in [("wardrobe-royal", 0u8), ("wardrobe-cape", 3u8)] {
            app.wardrobe_selection = selection;
            save(&mut app, out, name)?;
        }
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
    if std::env::args().any(|a| a == "--war-map") {
        // QA: a battle on a war map 2.0 battle map: deployed, seen from high, the armies met.
        let args: Vec<String> = std::env::args().skip_while(|a| a != "--war-map").collect();
        let map = args.get(1).cloned().unwrap_or_else(|| "vado".into());
        let attack = args.get(2).is_none_or(|a| a != "defend");
        app.nature_mode = false;
        app.village_entered = false;
        app.castle = None;
        let step = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        app.open_war_table();
        step(&mut app, 10);
        app.start_map_battle(&map, attack);
        // the player's side builds what the AI would, so both sides' engines show
        if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
            let zone = run.b.zone;
            if let Some(mut s) = run.b.sim.siege.take() {
                s.ai_setup(&mut run.b.sim, 0, zone);
                for e in &s.engines {
                    eprintln!(
                        "[qa] engine {:?} side {} at ({:.0}, {:.0}) built {}",
                        e.kind, e.side, e.x, e.z, e.built
                    );
                }
                eprintln!(
                    "[qa] wood {:?}, defender {:?}, zone {:?}",
                    s.wood, s.defender, zone
                );
                run.b.sim.siege = Some(s);
            }
        }
        step(&mut app, 20);
        save(&mut app, out, &format!("map-{map}-deploy"))?;
        // the engines close to
        let first = app
            .war
            .as_ref()
            .and_then(|w| w.battle.as_ref())
            .and_then(|r| {
                let s = r.b.sim.siege.as_ref()?;
                s.engines
                    .iter()
                    .find(|e| e.side == 0)
                    .or(s.engines.first())
                    .map(|e| (e.x, e.z))
            });
        if let (Some((ex, ez)), Some(w)) = (first, app.war.as_mut()) {
            if let Some(run) = w.battle.as_ref() {
                let ground = |x: f32, z: f32| run.b.sim.field.walk_height(x, z);
                w.cam.frame(ex, ez, Some(38.0), Some(0.6), true, &ground);
            }
        }
        step(&mut app, 6);
        save(&mut app, out, &format!("map-{map}-engines"))?;
        // the build bar on: a catapult under the pointer, in the middle of the screen
        if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
            // (some wood back, for the look of it: the QA's side spent it all)
            if let Some(s) = run.b.sim.siege.as_mut() {
                s.wood[0] += 60.0;
            }
        }
        if let Some(w) = app.war.as_mut() {
            w.qa_pointer = Some((900.0, 640.0));
        }
        app.war_hit(map_overlay::WarHit::Build(0));
        step(&mut app, 8);
        save(&mut app, out, &format!("map-{map}-build"))?;
        app.war_hit(map_overlay::WarHit::Build(0));
        if let Some(w) = app.war.as_mut() {
            w.qa_pointer = None;
        }
        if let Some(w) = app.war.as_mut() {
            w.cam.zoom(900.0);
        }
        step(&mut app, 30);
        save(&mut app, out, &format!("map-{map}-high"))?;
        if let Some(w) = app.war.as_mut() {
            w.cam.zoom(-900.0);
        }
        app.war_hit(map_overlay::WarHit::Begin);
        if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
            run.b.auto_player = Some(demon_queen_war::ai::BattleAi::new(
                0,
                if attack {
                    demon_queen_war::ai::Posture::Attack
                } else {
                    demon_queen_war::ai::Posture::Defend
                },
            ));
            run.b.speed = 3.0;
        }
        // a siege: look at its gate from outside; frames as it goes
        let gate = app
            .war
            .as_ref()
            .and_then(|w| w.battle.as_ref())
            .and_then(|r| {
                let s = r.b.sim.siege.as_ref()?;
                s.works
                    .iter()
                    .find(|k| k.kind == demon_queen_war::baked::WorkKind::Gate)
                    .map(|g| (g.x, g.z))
            });
        let mut shots = 0;
        for k in 0..5400 {
            // (the war's scenes, if one comes up, read through: the war waits while they talk)
            if app.talk.is_some() {
                app.talk_pick(0);
                app.talk_confirm();
                app.talk_confirm();
            }
            app.tick(1.0 / 30.0);
            if k % 6 == 0 {
                if let (Some((gx, gz)), Some(w)) = (gate, app.war.as_mut()) {
                    if let Some(run) = w.battle.as_ref() {
                        let ground = |x: f32, z: f32| run.b.sim.field.walk_height(x, z);
                        let p = run.b.zone;
                        let (cx, cz) = ((p.x0 + p.x1) / 2.0, (p.z0 + p.z1) / 2.0);
                        let (dx, dz) = (gx - cx, gz - cz);
                        let l = dx.hypot(dz).max(1.0);
                        w.cam.frame(
                            gx - dx / l * 30.0,
                            gz - dz / l * 30.0,
                            Some(95.0),
                            Some((-dx).atan2(-dz)),
                            k == 0,
                            &ground,
                        );
                    }
                }
                app.render_frame();
            }
            let t = app
                .war
                .as_ref()
                .and_then(|w| w.battle.as_ref())
                .map_or(0.0, |r| r.b.sim.time);
            let over = app
                .war
                .as_ref()
                .and_then(|w| w.battle.as_ref())
                .is_none_or(|r| r.b.sim.over.is_some());
            for (n, at) in [(0, 25.0f32), (1, 70.0), (2, 140.0), (3, 240.0)] {
                if shots == n && t >= at {
                    step(&mut app, 2);
                    save(&mut app, out, &format!("map-{map}-t{}", at as u32))?;
                    shots += 1;
                }
            }
            if over || (gate.is_none() && t > 120.0) {
                break;
            }
        }
        step(&mut app, 4);
        save(&mut app, out, &format!("map-{map}-fight"))?;
        if let Some(s) = app
            .war
            .as_ref()
            .and_then(|w| w.battle.as_ref())
            .and_then(|r| r.b.sim.siege.as_ref())
        {
            eprintln!(
                "[qa] siege: {} engines ({} standing), broken {:?}, winner {:?}",
                s.engines.len(),
                s.engines.iter().filter(|e| e.alive).count(),
                s.works
                    .iter()
                    .filter(|w| w.broken)
                    .map(|w| format!("{:?}", w.kind))
                    .collect::<Vec<_>>(),
                app.war
                    .as_ref()
                    .and_then(|w| w.battle.as_ref())
                    .and_then(|r| r.b.sim.over.map(|o| o.winner))
            );
        }
        app.war = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--war-overview") {
        // QA: the campaign map seen whole and up close (castle, villa, the east, the capital), no fog,
        // like the prototype's views.
        app.nature_mode = false;
        app.village_entered = false;
        app.castle = None;
        let step = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        app.story.values.insert("varrak_despierto".into(), 1);
        app.open_war_table();
        step(&mut app, 10);
        app.war_campaign = None;
        app.war_hit(map_overlay::WarHit::Table(1));
        step(&mut app, 10);
        app.war_hit(map_overlay::WarHit::Modal(0));
        for _ in 0..60 {
            if app.talk.is_some() {
                app.end_talk();
            }
            step(&mut app, 2);
        }
        // the whole continent in sight (no fog: every region seen)
        if let Some(s) = app.war_campaign.as_mut() {
            for p in s.provinces.iter_mut() {
                p.lit = true;
            }
        }
        let views: [(&str, f32, f32, f32, f32); 7] = [
            ("overview", 0.0, 4.0, 250.0, 0.0),
            ("castle", -62.0, 2.0, 32.0, 0.4),
            ("villa", -42.0, 27.0, 32.0, -0.3),
            ("penascal", -22.0, -34.0, 38.0, 0.2),
            ("east", 55.0, 5.0, 90.0, 0.0),
            ("capital", 86.0, -42.0, 34.0, 0.5),
            ("valdesanto", 88.0, 8.0, 32.0, -0.4),
        ];
        let only = std::env::args()
            .skip_while(|a| a != "--war-overview")
            .nth(1)
            .filter(|a| !a.starts_with("--"));
        for (name, x, z, d, yaw) in views {
            if only.as_deref().is_some_and(|o| o != name) {
                continue;
            }
            if let Some(w) = app.war.as_mut() {
                let land = demon_queen_war::campaign_map::Land::new();
                w.cam
                    .frame(x, z, Some(d), Some(yaw), true, &|a: f32, b: f32| {
                        land.top_at(a, b)
                    });
            }
            step(&mut app, 30);
            save(&mut app, out, &format!("overview-{name}"))?;
        }
        app.war = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--war-act2") {
        // QA: the second act's scenes over the campaign, each at its first lines and at its
        // choice (or its last line).
        app.nature_mode = false;
        app.village_entered = false;
        app.castle = None;
        let step = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        for (k, v) in [("varrak_despierto", 1), ("consejo_1_hecho", 1), ("pista_merin", 1)] {
            app.story.values.insert(k.into(), v);
        }
        app.gold = 420;
        app.open_war_table();
        step(&mut app, 10);
        app.war_campaign = None;
        app.war_hit(map_overlay::WarHit::Table(1));
        step(&mut app, 10);
        app.war_hit(map_overlay::WarHit::Modal(0));
        step(&mut app, 10);
        // whatever the campaign opened with, through
        for _ in 0..200 {
            if app.talk.is_none() {
                step(&mut app, 4);
                if app.talk.is_none() {
                    break;
                }
            }
            app.talk_confirm();
            app.talk_confirm();
            app.talk_pick(0);
            step(&mut app, 2);
        }
        if let Some(w) = app.war.as_mut() {
            w.scenes.clear();
        }
        let only = std::env::var("VELVET_WAR_ACT2").ok();
        for id in [
            "consejo_traje",
            "el_odio",
            "merin",
            "reliquia_medianoche",
            "precio_chispa",
            "guardia",
            "noche_mesa",
        ] {
            if only.as_deref().is_some_and(|o| !o.split(',').any(|v| v == id)) {
                continue;
            }
            app.start_talk_at(&format!("guerra.{id}"));
            step(&mut app, 24);
            // VELVET_WAR_ACT2_STEPS: a picture after every line, to follow a scene through
            if std::env::var_os("VELVET_WAR_ACT2_STEPS").is_some() {
                for n in 0..40 {
                    if app.talk.is_none() {
                        break;
                    }
                    save(&mut app, out, &format!("act2-{id}-s{n:02}"))?;
                    app.talk_pick(0);
                    app.talk_confirm();
                    app.talk_confirm();
                    step(&mut app, 20);
                }
                continue;
            }
            for _ in 0..3 {
                app.talk_confirm();
                app.talk_confirm();
                step(&mut app, 6);
            }
            step(&mut app, 30);
            save(&mut app, out, &format!("act2-{id}-a"))?;
            // on to its choice, or its end
            for _ in 0..60 {
                let choice = matches!(
                    app.talk
                        .as_ref()
                        .map(|t| t.shown(guion::library(), &app.story)),
                    Some(guion::Shown::Choice { .. })
                );
                if choice || app.talk.is_none() {
                    break;
                }
                app.talk_confirm();
                app.talk_confirm();
                step(&mut app, 3);
            }
            step(&mut app, 30);
            save(&mut app, out, &format!("act2-{id}-b"))?;
            for _ in 0..80 {
                if app.talk.is_none() {
                    break;
                }
                app.talk_pick(0);
                app.talk_confirm();
                app.talk_confirm();
                step(&mut app, 3);
            }
            step(&mut app, 10);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--war-story") {
        // QA: the war and the story as one: one purse, the campaign's first scene in the dialogue
        // box, the river closed, the relics' page; a battle with its relic and her powers.
        use zone_war::{Sel, WarPage};
        app.nature_mode = false;
        app.village_entered = false;
        app.castle = None;
        let step = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        let purse = |app: &App| {
            app.war_campaign
                .as_ref()
                .map(|s| s.purse(demon_queen_war::data::Faction::Reina).gold)
        };
        // through a conversation: its lines on, its choices by the first option
        let talk_through = |app: &mut App, pick_first: bool| {
            for _ in 0..80 {
                if app.talk.is_none() {
                    step(app, 4);
                    if app.talk.is_none() {
                        break;
                    }
                }
                let choice = matches!(
                    app.talk
                        .as_ref()
                        .map(|t| t.shown(guion::library(), &app.story)),
                    Some(guion::Shown::Choice { .. })
                );
                if choice && pick_first {
                    app.talk_pick(0);
                } else {
                    app.talk_confirm();
                    app.talk_confirm();
                }
                step(app, 2);
            }
        };
        for (k, v) in [
            ("gorou_cabeza_alta", 1),
            ("afecto_nyxara", 3),
            ("consejo_1_hecho", 1),
            ("raciones_igual", 1),
            ("apagados_encierro", 1),
        ] {
            app.story.values.insert(k.into(), v);
        }
        for k in ["varrak_despierto", "varrak_vencido"] {
            app.story.values.remove(k);
        }
        app.gold = 420;
        app.open_war_table();
        step(&mut app, 10);
        app.war_campaign = None;
        app.war_hit(map_overlay::WarHit::Table(1));
        step(&mut app, 10);
        eprintln!("[qa] gold {} · treasury {:?}", app.gold, purse(&app));
        app.war_hit(map_overlay::WarHit::Modal(0));
        step(&mut app, 30);
        eprintln!("[qa] talk after the intro card: {}", app.talk.is_some());
        save(&mut app, out, "story-intro")?;
        for _ in 0..12 {
            if app.talk.is_none() {
                break;
            }
            let choice = matches!(
                app.talk
                    .as_ref()
                    .map(|t| t.shown(guion::library(), &app.story)),
                Some(guion::Shown::Choice { .. })
            );
            if choice {
                break;
            }
            app.talk_confirm();
            app.talk_confirm();
            step(&mut app, 2);
        }
        step(&mut app, 20);
        save(&mut app, out, "story-intro-choice")?;
        app.talk_pick(1);
        // the intro's end, then the powers' scenes queued at the start
        talk_through(&mut app, true);
        eprintln!(
            "[qa] seen: intro {} · grito {} · bendición {}",
            app.story.seen("guerra.intro"),
            app.story.seen("guerra.poder_grito"),
            app.story.seen("guerra.poder_bendicion")
        );
        // one purse: a regiment bought in the castle comes out of her gold
        let queen = app
            .war_campaign
            .as_ref()
            .and_then(|s| s.queen_army())
            .map(|a| a.id.clone());
        if let (Some(id), Some(c)) = (queen, app.war.as_mut().and_then(|w| w.camp.as_mut())) {
            c.sel = Some(Sel::Army(id));
            c.paint_version += 1;
        }
        step(&mut app, 4);
        let before = app.gold;
        app.war_hit(map_overlay::WarHit::Panel(0));
        step(&mut app, 4);
        eprintln!(
            "[qa] recruit: gold {} → {} · treasury {:?}",
            before,
            app.gold,
            purse(&app)
        );
        // the river: closed until Varrak is woken
        if let Some(p) = demon_queen_war::data::PROVINCES
            .iter()
            .position(|p| p.id == "capital")
        {
            app.campaign_go(p);
        }
        step(&mut app, 30);
        save(&mut app, out, "story-river")?;
        talk_through(&mut app, true);
        // a Libre region chosen: waking it with the spark
        let libre = demon_queen_war::data::PROVINCES.iter().position(|p| {
            app.war_campaign
                .as_ref()
                .is_some_and(|s| s.prov(p.id).owner == demon_queen_war::data::Faction::Libres)
        });
        if let Some(p) = libre {
            if let Some(c) = app.war.as_mut().and_then(|w| w.camp.as_mut()) {
                c.sel = Some(Sel::Province(p));
                c.paint_version += 1;
            }
            step(&mut app, 6);
            save(&mut app, out, "story-libres")?;
        }
        // one turn a day: the second end of turn waits for the night's sleep
        let turn = |app: &App| app.war_campaign.as_ref().map_or(0, |s| s.turn);
        let t0 = turn(&app);
        eprintln!(
            "[qa] before the turn: talk {} · phase {:?} · today {} · modal {} · busy {}",
            app.talk.is_some(),
            app.war_campaign.as_ref().map(|s| s.phase),
            app.story.get("guerra_turno_hoy"),
            app.war
                .as_ref()
                .and_then(|w| w.camp.as_ref())
                .is_some_and(|c| c.modal.is_some()),
            app.war
                .as_ref()
                .and_then(|w| w.camp.as_ref())
                .is_some_and(|c| c.busy),
        );
        app.war_hit(map_overlay::WarHit::EndTurn);
        for _ in 0..40 {
            step(&mut app, 30);
            if app
                .war_campaign
                .as_ref()
                .is_some_and(|s| s.phase == demon_queen_war::rules::Phase::Player)
                && turn(&app) > t0
            {
                break;
            }
        }
        eprintln!(
            "[qa] after the turn: phase {:?} · today {} · talk {} · modal {:?} · steps {} · busy {}",
            app.war_campaign.as_ref().map(|s| s.phase),
            app.story.get("guerra_turno_hoy"),
            app.talk.is_some(),
            app.war.as_ref().and_then(|w| w.camp.as_ref()).and_then(|c| c.modal.as_ref().map(|m| format!("{m:?}").chars().take(60).collect::<String>())),
            app.war.as_ref().and_then(|w| w.camp.as_ref()).map_or(0, |c| c.steps.len()),
            app.war.as_ref().and_then(|w| w.camp.as_ref()).is_some_and(|c| c.busy),
        );
        let t1 = turn(&app);
        app.war_hit(map_overlay::WarHit::EndTurn);
        step(&mut app, 240);
        eprintln!("[qa] turns: {t0} → {t1} → {} (one a day)", turn(&app));
        save(&mut app, out, "story-turn-tomorrow")?;
        // the relics' page
        if let Some(w) = app.war.as_mut() {
            w.page = WarPage::Table(0);
            w.camp = None;
        }
        app.war_hit(map_overlay::WarHit::Table(6));
        step(&mut app, 10);
        save(&mut app, out, "story-relics")?;
        // a battle on el Peñascal: its relic on the field, her powers
        app.start_map_battle("penascal", true);
        step(&mut app, 20);
        talk_through(&mut app, true);
        let rel = app
            .war
            .as_ref()
            .and_then(|w| w.battle.as_ref())
            .and_then(|r| r.relic.as_ref().map(|l| (l.x, l.z, l.name.clone())));
        eprintln!("[qa] relic on the field: {rel:?}");
        let powers = app
            .war
            .as_ref()
            .and_then(|w| w.battle.as_ref())
            .and_then(|r| r.b.sim.powers.as_ref().map(|p| p.unlocked.clone()));
        eprintln!("[qa] powers: {powers:?}");
        if let (Some((x, z, _)), Some(w)) = (rel.clone(), app.war.as_mut()) {
            if let Some(run) = w.battle.as_ref() {
                let ground = |a: f32, b: f32| run.b.sim.field.walk_height(a, b);
                w.cam.frame(x, z, Some(40.0), Some(0.5), true, &ground);
            }
        }
        step(&mut app, 10);
        save(&mut app, out, "story-battle-relic")?;
        app.war_hit(map_overlay::WarHit::Begin);
        // a regiment of hers walks onto it (put there, for the QA)
        if let (Some((x, z, _)), Some(run)) =
            (rel, app.war.as_mut().and_then(|w| w.battle.as_mut()))
        {
            let sim = &mut run.b.sim;
            if let Some(r) = (0..sim.regiments.len()).find(|&r| sim.regiments[r].side == 0) {
                let (dx, dz) = (x - sim.regiments[r].cx, z - sim.regiments[r].cz);
                for i in 0..sim.reg.len() {
                    if sim.reg[i] == r {
                        sim.px[i] += dx;
                        sim.pz[i] += dz;
                    }
                }
                sim.regiments[r].cx = x;
                sim.regiments[r].cz = z;
            }
        }
        step(&mut app, 8);
        save(&mut app, out, "story-battle-pickup")?;
        eprintln!(
            "[qa] relic taken: {:?} · brasas {} · gold {}",
            app.war
                .as_ref()
                .and_then(|w| w.battle.as_ref())
                .and_then(|r| r.relic.as_ref().map(|l| l.taken)),
            app.story.get("poder_brasas"),
            app.gold
        );
        // her powers, on the enemy
        let enemy = app
            .war
            .as_ref()
            .and_then(|w| w.battle.as_ref())
            .and_then(|r| {
                let s = &r.b.sim;
                let regs: Vec<_> = s
                    .regiments
                    .iter()
                    .filter(|g| g.side == 1 && g.alive > 0)
                    .collect();
                let n = regs.len().max(1) as f32;
                (!regs.is_empty()).then(|| {
                    (
                        regs.iter().map(|g| g.cx).sum::<f32>() / n,
                        regs.iter().map(|g| g.cz).sum::<f32>() / n,
                    )
                })
            });
        if let (Some((ex, ez)), Some(w)) = (enemy, app.war.as_mut()) {
            if let Some(run) = w.battle.as_ref() {
                let ground = |a: f32, b: f32| run.b.sim.field.walk_height(a, b);
                w.cam.frame(ex, ez, Some(70.0), Some(0.5), true, &ground);
            }
            w.qa_pointer = Some((960.0, 560.0));
        }
        app.war_hit(map_overlay::WarHit::Power(3));
        step(&mut app, 6);
        save(&mut app, out, "story-power-aim")?;
        if let Some(w) = app.war.as_mut() {
            w.qa_pointer = None;
            if let Some(run) = w.battle.as_mut() {
                run.power_aim = None;
                run.power_at = None;
            }
        }
        if let (Some((ex, ez)), Some(run)) =
            (enemy, app.war.as_mut().and_then(|w| w.battle.as_mut()))
        {
            if let Some(mut pw) = run.b.sim.powers.take() {
                use demon_queen_war::powers::Power;
                let a = pw.cast(&mut run.b.sim, 0, Power::Brasas, ex, ez);
                let b = pw.cast(&mut run.b.sim, 0, Power::Rugido, ex + 12.0, ez);
                eprintln!("[qa] cast brasas {a} · rugido {b}");
                run.b.sim.powers = Some(pw);
            }
        }
        step(&mut app, 24);
        save(&mut app, out, "story-power-embers")?;
        if let (Some((ex, ez)), Some(run)) =
            (enemy, app.war.as_mut().and_then(|w| w.battle.as_mut()))
        {
            if let Some(mut pw) = run.b.sim.powers.take() {
                use demon_queen_war::powers::Power;
                pw.unlocked.push(Power::Niebla);
                let a = pw.cast(&mut run.b.sim, 0, Power::Niebla, ex - 10.0, ez - 6.0);
                eprintln!("[qa] cast niebla {a}");
                run.b.sim.powers = Some(pw);
            }
        }
        step(&mut app, 60);
        save(&mut app, out, "story-power-fog")?;
        // the relic's scene, over the battle paused
        if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
            run.b.paused = true;
        }
        step(&mut app, 30);
        eprintln!("[qa] talk over the battle: {}", app.talk.is_some());
        save(&mut app, out, "story-battle-talk")?;
        app.war = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--war") {
        // QA: the war table's page, the campaign (its intro, the map, an army chosen, a battle
        // asked), a quick battle (deployed, fought by both generals, close, the spark, the end) and
        // el Peñascal with Varrak.
        use zone_war::{Sel, WarPage};
        app.nature_mode = false;
        app.village_entered = false;
        app.castle = None;
        let only = std::env::args()
            .skip_while(|a| a != "--war")
            .nth(1)
            .filter(|a| !a.starts_with("--"));
        let want = |name: &str| only.as_deref().is_none_or(|o| name.contains(o));
        let step = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        app.open_war_table();
        step(&mut app, 20);
        if want("table") {
            save(&mut app, out, "war-table")?;
            // the focus on the units' size (the keys move it), then the quick battles on the second
            if let Some(w) = app.war.as_mut() {
                w.focus = 2;
            }
            step(&mut app, 10);
            save(&mut app, out, "war-table-size")?;
            app.war_hit(map_overlay::WarHit::Table(2));
            if let Some(w) = app.war.as_mut() {
                w.focus = 1;
            }
            step(&mut app, 10);
            save(&mut app, out, "war-quick")?;
        }
        if want("campaign") {
            app.war_campaign = None;
            if let Some(w) = app.war.as_mut() {
                w.page = WarPage::Table(0);
            }
            app.war_hit(map_overlay::WarHit::Table(1));
            step(&mut app, 30);
            save(&mut app, out, "war-campaign-intro")?;
            app.war_hit(map_overlay::WarHit::Modal(0));
            step(&mut app, 30);
            save(&mut app, out, "war-campaign")?;
            let queen = app
                .war_campaign
                .as_ref()
                .and_then(|s| s.queen_army())
                .map(|a| a.id.clone());
            if let (Some(id), Some(c)) = (queen, app.war.as_mut().and_then(|w| w.camp.as_mut())) {
                c.sel = Some(Sel::Army(id));
                c.paint_version += 1;
            }
            step(&mut app, 10);
            save(&mut app, out, "war-campaign-army")?;
            // three regiments bought in the castle: they stand in the army's formation
            for _ in 0..3 {
                app.war_hit(map_overlay::WarHit::Panel(0));
                step(&mut app, 2);
            }
            if let Some(w) = app.war.as_mut() {
                w.cam.zoom(-900.0);
            }
            step(&mut app, 30);
            save(&mut app, out, "war-campaign-close")?;
            // a march to the fallen capital: it sets out at once, its route and turns on the map
            if let Some(w) = app.war.as_mut() {
                w.cam.zoom(900.0);
            }
            if let Some(p) = demon_queen_war::data::PROVINCES
                .iter()
                .position(|p| p.id == "capital")
            {
                app.campaign_go(p);
            }
            step(&mut app, 80);
            save(&mut app, out, "war-campaign-march")?;
            app.war_hit(map_overlay::WarHit::EndTurn);
            step(&mut app, 200);
            save(&mut app, out, "war-campaign-turn2")?;
        }
        for (index, name) in [
            (0usize, "vado"),
            (1, "penascal"),
            (2, "cienaga"),
            (3, "atalaya"),
            (4, "puerta"),
        ] {
            if !want(name) {
                continue;
            }
            if let Some(w) = app.war.as_mut() {
                w.page = WarPage::Table(1);
                w.battle = None;
            }
            app.war_hit(map_overlay::WarHit::Quick(index as u8));
            step(&mut app, 20);
            save(&mut app, out, &format!("war-{name}-deploy"))?;
            app.war_hit(map_overlay::WarHit::Begin);
            if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                run.b.auto_player = Some(demon_queen_war::ai::BattleAi::new(
                    0,
                    demon_queen_war::ai::Posture::Attack,
                ));
                run.b.speed = 3.0;
            }
            // the armies meet
            for k in 0..900 {
                app.tick(1.0 / 30.0);
                if k % 6 == 0 {
                    app.render_frame();
                }
                let fighting = app
                    .war
                    .as_ref()
                    .and_then(|w| w.battle.as_ref())
                    .is_some_and(|r| {
                        r.b.sim
                            .regiments
                            .iter()
                            .filter(|g| g.state == demon_queen_war::sim::RState::Melee)
                            .count()
                            >= 4
                    });
                if fighting && k > 200 {
                    break;
                }
            }
            let hot = |app: &App| {
                let run = app.war.as_ref()?.battle.as_ref()?;
                let regs: Vec<&demon_queen_war::sim::Regiment> =
                    run.b.sim.regiments.iter().filter(|r| r.alive > 0).collect();
                let mut hot: Vec<&&demon_queen_war::sim::Regiment> = regs
                    .iter()
                    .filter(|r| r.state == demon_queen_war::sim::RState::Melee)
                    .collect();
                if hot.is_empty() {
                    hot = regs.iter().collect();
                }
                let n: f32 = hot.iter().map(|r| r.alive as f32).sum::<f32>().max(1.0);
                Some((
                    hot.iter().map(|r| r.cx * r.alive as f32).sum::<f32>() / n,
                    hot.iter().map(|r| r.cz * r.alive as f32).sum::<f32>() / n,
                ))
            };
            if let Some((x, z)) = hot(&app) {
                if let Some(w) = app.war.as_mut() {
                    let run = w.battle.as_ref().expect("a battle");
                    let field = &run.b.sim.field;
                    w.cam
                        .frame(x, z, Some(110.0), Some(0.4), true, &|a: f32, b: f32| {
                            field.walk_height(a, b)
                        });
                }
            }
            step(&mut app, 4);
            save(&mut app, out, &format!("war-{name}-fight"))?;
            if let Some((x, z)) = hot(&app) {
                if let Some(w) = app.war.as_mut() {
                    let run = w.battle.as_ref().expect("a battle");
                    let field = &run.b.sim.field;
                    w.cam
                        .frame(x, z, Some(34.0), Some(-0.5), true, &|a: f32, b: f32| {
                            field.walk_height(a, b)
                        });
                }
            }
            step(&mut app, 4);
            save(&mut app, out, &format!("war-{name}-close"))?;
            // the Queen's spark, seen from near her
            let queen_at = app
                .war
                .as_ref()
                .and_then(|w| w.battle.as_ref())
                .and_then(|r| {
                    r.b.queen
                        .map(|q| (r.b.sim.regiments[q].cx, r.b.sim.regiments[q].cz))
                });
            if let Some((x, z)) = queen_at {
                if let Some(w) = app.war.as_mut() {
                    let run = w.battle.as_mut().expect("a battle");
                    if let Some(q) = run.b.queen {
                        if let Some(ab) = run.b.sim.regiments[q].ability.as_mut() {
                            ab.cd = 0.0;
                        }
                    }
                    let field = &run.b.sim.field;
                    w.cam
                        .frame(x, z, Some(60.0), Some(0.2), true, &|a: f32, b: f32| {
                            field.walk_height(a, b)
                        });
                }
                app.war_hit(map_overlay::WarHit::Order(4));
                if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                    run.b.speed = 1.0;
                }
                step(&mut app, 14);
                save(&mut app, out, &format!("war-{name}-spark"))?;
            }
            if name == "penascal" {
                let varrak = app
                    .war
                    .as_ref()
                    .and_then(|w| w.battle.as_ref())
                    .and_then(|r| {
                        r.b.sim
                            .regiments
                            .iter()
                            .find(|g| g.ty.model == "varrak" && g.alive > 0)
                            .map(|g| (g.cx, g.cz))
                    });
                if let Some((x, z)) = varrak {
                    if let Some(w) = app.war.as_mut() {
                        let run = w.battle.as_ref().expect("a battle");
                        let field = &run.b.sim.field;
                        w.cam
                            .frame(x, z, Some(24.0), Some(0.6), true, &|a: f32, b: f32| {
                                field.walk_height(a, b)
                            });
                    }
                    step(&mut app, 8);
                    save(&mut app, out, "war-penascal-varrak")?;
                }
            }
            if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                run.b.skip(900.0, true);
            }
            step(&mut app, 120);
            save(&mut app, out, &format!("war-{name}-end"))?;
        }
        app.leave_war();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--castle-life") {
        // The castle before and after the court's works and audiences: each room that changes,
        // first as she finds it and then with everything restored and decided.
        app.nature_mode = false;
        app.village_entered = false;
        let up = castle3d::plan().level_h;
        let stops: [(&str, f32, f32, f32); 9] = [
            ("patio", -4.0, 0.0, 29.0),
            ("patio-arena", -12.0, 0.0, 27.0),
            ("comedor", 20.0, 0.0, 0.4),
            ("biblioteca", -16.0, 0.0, 3.6),
            ("trono", 1.4, 0.0, -15.0),
            ("enfermeria", -15.6, up, 12.0),
            ("armeria", 17.4, up, 11.0),
            ("barracones", 17.0, up, 18.8),
            ("balcon", -20.0, 0.0, -25.6),
        ];
        let only = std::env::var("VELVET_CASTLE_LIFE").unwrap_or_default();
        app.environment_cycle.set_phase(0.5);
        for (tag, done) in [("antes", false), ("despues", true)] {
            let mut vars = guion::Vars::default();
            vars.values.insert("varrak_despierto".into(), 1);
            vars.values.insert("consejo_1_hecho".into(), 1);
            if done {
                for (k, v) in [
                    ("obra_enfermeria", 1),
                    ("obra_armeria", 1),
                    ("obra_cocina", 1),
                    ("obra_barracones", 1),
                    ("obra_bano", 1),
                    ("obra_biblioteca", 1),
                    ("aud_cabra_grubbo", 3),
                    ("aud_cabra", 2),
                    ("aud_cabra_trono", 2),
                    ("aud_refugiados_norte", 2),
                    ("aud_herrera_armaduras", 2),
                    ("aud_boda_orca", 2),
                    ("aud_dulces_villa", 2),
                    ("aud_lapida_capital", 1),
                    ("prisionera_n", 5),
                    ("prisionera_queda", 1),
                ] {
                    vars.values.insert(k.into(), v);
                }
            }
            for (name, x, y, z) in stops {
                if !only.is_empty() && !only.split(',').any(|o| name.starts_with(o)) {
                    continue;
                }
                app.story = vars.clone();
                app.player = Vec3::new(x, y, z);
                app.castle = None;
                app.iso.yaw = std::f32::consts::FRAC_PI_4;
                app.iso.yaw_target = app.iso.yaw;
                app.iso.distance = ISO.max_distance;
                for _ in 0..90 {
                    app.tick(1.0 / 60.0);
                    if let Some(inside) = &app.castle {
                        app.camera_focus = inside.focus();
                    }
                    app.guide = None;
                    app.render_frame();
                }
                save(&mut app, out, &format!("vida-{name}-{tag}"))?;
            }
        }
        app.castle = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--castle-vn-spots") {
        // QA only: each castle room seen from its four corners at eye height, every wall up and
        // the Queen behind the camera: references for the court's VN backgrounds. The game's own
        // camera is not touched outside this capture.
        app.nature_mode = false;
        app.village_entered = false;
        if let Some(state) = app.state.as_mut() {
            state.map_overlay.hidden = true;
        }
        let p = castle3d::plan();
        let rooms = [
            ("dormitorio", 0.0f32),
            ("dormitorio", 0.5),
            ("biblioteca", 0.5),
            ("barracones", 0.5),
            ("enfermeria", 0.5),
            ("armeria", 0.5),
            ("comedor", 0.5),
            ("almacen", 0.5),
        ];
        for (id, phase) in rooms {
            let Some(room) = p.rooms.iter().find(|r| r.id == id) else {
                continue;
            };
            app.environment_cycle.set_phase(phase);
            let floor = if room.level == 1 { p.level_h } else { 0.0 };
            let (cx, cz) = room.centre();
            let tag = if phase < 0.25 { "noche" } else { "dia" };
            let along = |a: f32, b: f32, t: f32| a + (b - a) * t;
            for (k, (fx, fz)) in [(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                .into_iter()
                .enumerate()
            {
                // She stands in the corner, the camera a step in front of her.
                // (Out of the furniture and the walls, or the castle puts her back in bed.)
                let (mut qx, mut qz) = (
                    along(room.x0, room.x1, 0.06 + fx * 0.88),
                    along(room.z0, room.z1, 0.06 + fz * 0.88),
                );
                for _ in 0..40 {
                    if !p.blocked(qx, qz, 0.45, room.level) {
                        break;
                    }
                    qx += (cx - qx) * 0.06;
                    qz += (cz - qz) * 0.06;
                }
                let (ex, ez) = (
                    along(room.x0, room.x1, 0.16 + fx * 0.68),
                    along(room.z0, room.z1, 0.16 + fz * 0.68),
                );
                app.player = Vec3::new(qx, floor, qz);
                app.castle = None;
                for _ in 0..3 {
                    app.tick(1.0 / 60.0);
                }
                if let Some(inside) = app.castle.as_mut() {
                    inside.keep_walls = true;
                }
                let eye = Vec3::new(ex, floor + 1.6, ez);
                let target = Vec3::new(cx + (cx - ex) * 0.5, floor + 1.35, cz + (cz - ez) * 0.5);
                app.camera_override = Some(Camera3D {
                    position: eye,
                    target,
                    up: Vec3::Y,
                    fov_y_radians: 64f32.to_radians(),
                    orthographic_height: None,
                    near: 0.05,
                    far: 220.0,
                });
                for _ in 0..90 {
                    app.tick(1.0 / 60.0);
                    // No mission arrows on the floor.
                    app.guide = None;
                    app.render_frame();
                }
                if let Err(error) = save(&mut app, out, &format!("castillo-{id}-{tag}-{k}")) {
                    eprintln!("[castle-vn] {id}: {error:#}");
                }
            }
        }
        app.camera_override = None;
        app.castle = None;
        if let Some(state) = app.state.as_mut() {
            state.map_overlay.hidden = false;
        }
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
    if std::env::args().any(|a| a == "--promo") {
        // Clips for the welcome video: the Queen walking through each place, filmed at 30 fps
        // (`<out>/<clip>/f0000.jpg`, 1280×720). `VELVET_PROMO=castillo,villa` films only those.
        let only: Option<Vec<String>> = std::env::var("VELVET_PROMO")
            .ok()
            .map(|v| v.split(',').map(str::to_owned).collect());
        let wanted = |clip: &str| only.as_ref().is_none_or(|o| o.iter().any(|c| c == clip));
        /// The keys that take her toward `to` as the camera stands (as a player would press them).
        fn steer(app: &mut App, to: Vec3) {
            let d = Vec3::new(to.x - app.player.x, 0.0, to.z - app.player.z);
            let len = d.length().max(1e-3);
            let (forward, right) = app.iso.walk_axes();
            let (f, r) = (d.dot(forward) / len, d.dot(right) / len);
            app.move_forward = f > 0.38;
            app.move_back = f < -0.38;
            app.move_right = r > 0.38;
            app.move_left = r < -0.38;
        }
        /// Films `secs` of the world, calling `step` before every tick (60 ticks a second, a
        /// frame every other one).
        fn film(
            app: &mut App,
            out: &Path,
            clip: &str,
            secs: f32,
            mut step: impl FnMut(&mut App, f32),
        ) -> Result<()> {
            let dir = out.join(clip);
            std::fs::create_dir_all(&dir)?;
            for i in 0..(secs * 30.0) as usize {
                for _ in 0..2 {
                    step(app, i as f32 / 30.0);
                    app.elapsed += 1.0 / 60.0;
                    app.tick(1.0 / 60.0);
                }
                app.render_frame();
                let pixels = app.capture_pixels()?;
                let frame = image::RgbaImage::from_raw(WINDOW_WIDTH, WINDOW_HEIGHT, pixels)
                    .context("frame")?;
                // The fight and the «coming soon» clips fill the screen in the videos: full size.
                let frame = if matches!(clip, "varrak" | "corte" | "citas" | "guerra" | "campana") {
                    frame
                } else {
                    image::imageops::resize(
                        &frame,
                        1280,
                        720,
                        image::imageops::FilterType::Triangle,
                    )
                };
                let file = std::io::BufWriter::new(std::fs::File::create(
                    dir.join(format!("f{i:04}.jpg")),
                )?);
                image::DynamicImage::ImageRgba8(frame)
                    .to_rgb8()
                    .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
                        file, 88,
                    ))?;
                if i % 45 == 0 {
                    eprintln!(
                        "[promo] {clip} {:.1}s at {:.2},{:.2}",
                        i as f32 / 30.0,
                        app.player.x,
                        app.player.z
                    );
                }
            }
            app.clear_input();
            eprintln!("[promo] {clip}: {} frames", (secs * 30.0) as usize);
            Ok(())
        }
        /// Walks the waypoints in turn (the next once she is within 0.5 m).
        fn walker(path: Vec<Vec3>) -> impl FnMut(&mut App, f32) {
            let mut at = 0usize;
            move |app: &mut App, _| {
                while at < path.len() {
                    let d = Vec3::new(path[at].x - app.player.x, 0.0, path[at].z - app.player.z);
                    if d.length() < 0.5 {
                        at += 1;
                    } else {
                        break;
                    }
                }
                match path.get(at) {
                    Some(&to) => steer(app, to),
                    None => app.clear_input(),
                }
            }
        }
        let settle = |app: &mut App| {
            for _ in 0..30 {
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        };
        app.story = guion::Vars::default();
        // (and Chiquita's castle tips said: they would float up over a later clip)
        for v in [
            "burdel_bienvenida",
            "termas_bienvenida",
            "consejo_1_hecho",
            "lamento_visto",
            "tutorial_castillo",
        ] {
            app.story.values.insert(v.to_owned(), 1);
        }
        app.environment_cycle_enabled = false;
        // Nothing over the world (the fight keeps its own HUD).
        let clean = |app: &mut App, on: bool| {
            if let Some(state) = app.state.as_mut() {
                state.map_overlay.hidden = on;
            }
        };
        clean(&mut app, true);
        super::guide::GUIDE_HIDDEN.store(true, std::sync::atomic::Ordering::Relaxed);

        if wanted("castillo") {
            app.penascal = None;
            app.nature_mode = false;
            app.village_entered = false;
            app.castle = None;
            app.outfit = 0;
            app.environment_cycle.set_phase(0.82);
            app.player = Vec3::new(0.0, 0.0, 18.6);
            app.camera_focus = app.player;
            app.iso.distance = 5.6;
            settle(&mut app);
            film(
                &mut app,
                out,
                "castillo",
                6.0,
                walker(vec![
                    Vec3::new(0.0, 0.0, 12.0),
                    Vec3::new(-0.6, 0.0, 4.0),
                    Vec3::new(0.0, 0.0, -4.0),
                    Vec3::new(1.0, 0.0, -12.0),
                ]),
            )?;
        }
        if wanted("misiones") {
            // The first mission's guide: its panel under the corner map, the arrows on the floor,
            // and she walks the way it draws (from the bedroom to the war table).
            let promo_story = std::mem::take(&mut app.story);
            app.story.values.insert("tutorial_castillo".into(), 1);
            app.penascal = None;
            app.nature_mode = false;
            app.village_entered = false;
            app.castle = None;
            app.outfit = 0;
            app.environment_cycle.set_phase(0.42);
            app.player = Vec3::new(-17.6, 0.0, -12.6);
            app.camera_focus = app.player;
            app.iso.yaw = std::f32::consts::FRAC_PI_4;
            app.iso.yaw_target = app.iso.yaw;
            app.iso.distance = 6.5;
            clean(&mut app, false);
            super::guide::GUIDE_HIDDEN.store(false, std::sync::atomic::Ordering::Relaxed);
            settle(&mut app);
            film(&mut app, out, "misiones", 6.0, |app, _| {
                // A point of the way a little ahead of her feet.
                let feet = Vec3::new(app.player.x, 0.0, app.player.z);
                let ahead = app.guide.as_ref().and_then(|g| {
                    g.route
                        .iter()
                        .copied()
                        .find(|p| (Vec3::new(p.x, 0.0, p.z) - feet).length() > 1.2)
                });
                match ahead {
                    Some(to) => steer(app, to),
                    None => app.clear_input(),
                }
            })?;
            super::guide::GUIDE_HIDDEN.store(true, std::sync::atomic::Ordering::Relaxed);
            clean(&mut app, true);
            app.story = promo_story;
        }
        if wanted("villa") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.outfit = 1;
            app.environment_cycle.set_phase(0.42);
            let (spawn, _) = village::spawn();
            let (plaza, _) = village::clearing();
            app.player = spawn;
            app.camera_focus = app.player;
            app.iso.distance = 8.5;
            settle(&mut app);
            film(&mut app, out, "villa", 6.0, walker(vec![plaza]))?;
        }
        if wanted("tienda") {
            // The shop: the shelves, the stone sword picked, and a haggle played as a good player
            // would (each try stopped with the needle in its zone) to a deal.
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.outfit = 1;
            app.environment_cycle.set_phase(0.42);
            // At the shop's door, as when she walks in.
            let door = village::building_anchors()
                .iter()
                .find(|b| b.id == "villa_tienda")
                .and_then(|b| village::door_anchor(b.footprint_index));
            app.player = match door {
                Some((at, out)) => at + out * 1.2,
                None => village::clearing().0,
            };
            app.camera_focus = app.player;
            app.iso.distance = 8.5;
            app.gold = 600;
            settle(&mut app);
            // The shop's pages are over the world (the HUD stays hidden under them).
            clean(&mut app, false);
            app.open_shop();
            let sword = app
                .shop_product_indices()
                .iter()
                .position(|i| shop::catalog().products[*i].id == "espada_reliquia")
                .unwrap_or(0);
            let mut done = 0usize;
            film(&mut app, out, "tienda", 7.0, move |app, t| {
                const TRIES: [KeyCode; 3] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3];
                match done {
                    0 if t > 0.6 => {
                        app.shop_selection = sword.saturating_sub(2);
                        done = 1;
                    }
                    1 if t > 0.95 => {
                        app.shop_selection = sword.saturating_sub(1);
                        done = 2;
                    }
                    2 if t > 1.3 => {
                        app.shop_selection = sword;
                        done = 3;
                    }
                    3 if t > 2.0 => {
                        app.start_haggle();
                        done = 4;
                    }
                    4..=6 if t > 2.7 + (done - 4) as f32 * 0.9 => {
                        let now = app.elapsed;
                        let on_zone = app
                            .haggle
                            .as_ref()
                            .is_some_and(|h| (h.needle(now) - h.zone).abs() < 0.02);
                        if on_zone {
                            app.haggle_key(TRIES[done - 4]);
                            done += 1;
                        }
                    }
                    7 if t > 5.6 => {
                        app.haggle_key(KeyCode::Digit4);
                        done = 8;
                    }
                    _ => {}
                }
            })?;
            app.haggle = None;
            app.close_shop();
            clean(&mut app, true);
        }
        if wanted("bosque") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = false;
            // The green dress, as at the mage's stall a little further on.
            app.outfit = 1;
            app.environment_cycle.set_phase(0.9);
            app.player = Vec3::new(nature_trail_center(-11.0), 0.0, -11.0);
            app.camera_focus = app.player;
            app.iso.distance = 8.0;
            settle(&mut app);
            let path = (0..8)
                .map(|k| {
                    let z = -9.5 + k as f32 * 1.6;
                    Vec3::new(nature_trail_center(z), 0.0, z)
                })
                .collect();
            film(&mut app, out, "bosque", 6.0, walker(path))?;
        }
        if wanted("mago") {
            // The hypnotist's stall in the wood (she goes in the green dress, the one he dares to
            // talk to): his greeting, then his wares.
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = false;
            app.outfit = 1;
            app.environment_cycle.set_phase(0.35);
            let mage = mage_position();
            app.player = Vec3::new(mage.x + 1.4, 0.0, mage.z + 1.2);
            app.camera_focus = app.player;
            app.iso.distance = 6.5;
            app.gold = 120;
            settle(&mut app);
            app.queen_thought = None;
            clean(&mut app, false);
            let mut done = 0usize;
            film(&mut app, out, "mago", 6.0, move |app, t| match done {
                0 if t > 0.3 => {
                    app.interact_forest();
                    done = 1;
                }
                1 if t > 2.6 => {
                    app.mage_key(KeyCode::ArrowDown);
                    done = 2;
                }
                2 if t > 3.0 => {
                    app.mage_key(KeyCode::Enter);
                    done = 3;
                }
                3 if t > 4.4 => {
                    app.mage_key(KeyCode::ArrowDown);
                    done = 4;
                }
                _ => {}
            })?;
            app.mage = None;
            clean(&mut app, true);
        }
        if wanted("termas") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.outfit = 2;
            app.environment_cycle.set_phase(0.74);
            let mut state = baths::State::enter(Vec3::ZERO, 0.0);
            (state.x, state.z) = (-8.2, 10.6);
            app.player = state.position();
            app.baths = Some(state);
            app.camera_focus = app.player;
            app.iso.distance = 7.0;
            settle(&mut app);
            film(
                &mut app,
                out,
                "termas",
                6.0,
                walker(vec![
                    Vec3::new(-6.0, 0.0, 8.0),
                    Vec3::new(-3.5, 0.0, 5.5),
                    Vec3::new(-0.8, 0.0, 2.4),
                ]),
            )?;
            app.baths = None;
        }
        if wanted("mugo") || wanted("fotos") {
            // Mugo leading her to a spot for a photo (she keeps a step and a half behind him), then
            // the photo on the stage.
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.environment_cycle.set_phase(0.5);
            app.story.values.remove("mugo_guia");
            app.enter_baths();
            app.outfit = 2;
            app.queen_thought = None;
            let mugo_at = |app: &App| {
                app.state
                    .as_ref()
                    .and_then(|s| s.baths_scene.as_ref())
                    .and_then(|scene| scene.mugo_report())
            };
            for _ in 0..180 {
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
            if let Some(([mx, mz], _)) = mugo_at(&app) {
                if let Some(inside) = app.baths.as_mut() {
                    (inside.x, inside.z) = (mx - 1.2, mz + 0.9);
                    app.player = inside.position();
                    app.camera_focus = app.player;
                }
            }
            app.iso.distance = 6.5;
            settle(&mut app);
            let follow = move |app: &mut App| match mugo_at(app) {
                Some(([mx, mz], _)) => {
                    let gap = Vec3::new(mx - app.player.x, 0.0, mz - app.player.z).length();
                    if gap > 1.6 {
                        steer(app, Vec3::new(mx, 0.0, mz));
                    } else {
                        app.clear_input();
                    }
                }
                None => app.clear_input(),
            };
            app.story.values.insert("mugo_guia".into(), 1);
            for _ in 0..50 {
                follow(&mut app);
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
            }
            if wanted("mugo") {
                film(&mut app, out, "mugo", 5.0, |app, _| follow(app))?;
            }
            // On to the spot unfilmed, then the photo.
            for _ in 0..(60 * 90) {
                if mugo_at(&app).is_none_or(|(_, there)| there) {
                    break;
                }
                follow(&mut app);
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
            }
            app.clear_input();
            if let Some(spot) = app.baths.as_ref().and_then(|b| b.guide) {
                let [sx, sz] = mugo::PHOTO_SPOTS[spot].stand();
                if let Some(inside) = app.baths.as_mut() {
                    (inside.x, inside.z) = (sx, sz);
                    app.player = inside.position();
                    app.camera_focus = app.player;
                }
                settle(&mut app);
                // He makes room for her and points again: then the photo can be taken.
                for _ in 0..300 {
                    if app.baths.as_ref().and_then(|b| b.photo_here()).is_some() {
                        break;
                    }
                    app.elapsed += 1.0 / 60.0;
                    app.tick(1.0 / 60.0);
                }
                if wanted("fotos") {
                    clean(&mut app, false);
                    let mut taken = false;
                    film(&mut app, out, "fotos", 5.0, move |app, t| {
                        // The photo, as F on the spot takes it (he may still be making room).
                        if !taken && t > 0.4 {
                            let hour = app.environment_cycle.phase() * 24.0;
                            app.story.values.insert(
                                "hora_tarde".to_owned(),
                                i32::from(!(6.5..16.5).contains(&hour)),
                            );
                            let scene = format!("termas_foto_{}", mugo::PHOTO_SPOTS[spot].id);
                            app.start_talk(None, Some(&scene));
                            taken = true;
                        }
                    })?;
                    clean(&mut app, true);
                }
            } else {
                eprintln!("[promo] Mugo set no spot: no photo");
            }
            app.stage = None;
            app.baths = None;
            app.outfit_before_baths = None;
        }
        if wanted("burdel") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.outfit = 0;
            app.environment_cycle.set_phase(0.95);
            app.enter_brothel();
            app.talk = None;
            let desk = brothel::plan()
                .hotspots
                .iter()
                .find(|h| h.id == "recepcionista")
                .map(|h| Vec3::new(h.x, 0.0, h.z));
            settle(&mut app);
            if let Some(desk) = desk {
                app.iso.distance = 9.0;
                let _ = desk;
                film(
                    &mut app,
                    out,
                    "burdel",
                    5.0,
                    walker(vec![
                        Vec3::new(5.6, 0.0, 1.2),
                        Vec3::new(4.4, 0.0, -1.4),
                        Vec3::new(3.6, 0.0, -3.6),
                    ]),
                )?;
            }
            app.leave_brothel();
        }
        if wanted("libro") {
            // Rosalía's book at her desk, its pages turning.
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.outfit = 0;
            app.environment_cycle.set_phase(0.95);
            app.player = village::entrance();
            app.enter_brothel();
            app.talk = None;
            app.queen_thought = None;
            app.gold = 300;
            let desk = brothel::plan()
                .hotspots
                .iter()
                .find(|h| h.id == "recepcionista")
                .map(|h| (h.x, h.z));
            if let (Some(desk), Some(inside)) = (desk, app.brothel.as_mut()) {
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
            app.iso.distance = 7.5;
            settle(&mut app);
            app.venue = venue::Menu::default();
            clean(&mut app, false);
            let mut done = 0usize;
            film(&mut app, out, "libro", 5.0, move |app, t| match done {
                0 if t > 0.2 => {
                    app.open_venue_book();
                    done = 1;
                }
                1 if t > 2.0 => {
                    app.venue_key(KeyCode::ArrowRight);
                    done = 2;
                }
                2 if t > 3.4 => {
                    app.venue_key(KeyCode::ArrowRight);
                    done = 3;
                }
                _ => {}
            })?;
            app.close_shop();
            clean(&mut app, true);
            app.leave_brothel();
        }
        if wanted("penascal") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = false;
            app.outfit = 0;
            app.environment_cycle.set_phase(0.8);
            app.enter_penascal_from_forest();
            app.penascal = Some(penascal3d::State::restore(Vec3::new(0.0, 0.0, 22.0)));
            if let Some(inside) = &app.penascal {
                app.player = inside.position();
                app.camera_focus = inside.focus();
            }
            app.iso.distance = 9.5;
            settle(&mut app);
            film(
                &mut app,
                out,
                "penascal",
                6.0,
                walker(vec![Vec3::new(0.5, 0.0, 15.0), Vec3::new(1.5, 0.0, 8.0)]),
            )?;
            app.penascal = None;
        }
        if wanted("mina") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = false;
            app.inventory.insert("espada_de_madera".into(), 1);
            app.enter_dungeon_unchecked();
            settle(&mut app);
            app.dungeon.auto_walk = true;
            film(&mut app, out, "mina", 6.0, |_, _| {})?;
            app.dungeon.auto_walk = false;
            app.leave_dungeon();
        }
        if wanted("varrak") {
            app.castle = None;
            app.nature_mode = true;
            app.village_entered = false;
            app.story.values.insert("consejo_2_hecho".into(), 1);
            app.enter_penascal_from_forest();
            app.start_boss_fight();
            clean(&mut app, false);
            if let Some(f) = app.boss.as_mut() {
                f.player.god = true;
                f.qa("run");
            }
            settle(&mut app);
            // He comes at a run, leaps, she swings; he drops on all fours and charges; the spark.
            let mut beat = 0;
            film(&mut app, out, "varrak", 9.0, |app, t| {
                if let Some(f) = app.boss.as_mut() {
                    f.qa_look();
                    let next = match t {
                        t if t > 6.2 => 4,
                        t if t > 3.6 => 3,
                        t if t > 1.2 => 2,
                        _ => 1,
                    };
                    if next != beat {
                        beat = next;
                        match next {
                            2 => f.qa("leap"),
                            3 => f.qa("gallop"),
                            4 => f.qa("spark"),
                            _ => {}
                        }
                    }
                }
                app.boss_input.channel = beat == 4;
                app.boss_input.attack = beat == 2 && (t * 2.0).fract() < 0.05;
            })?;
            app.boss = None;
            app.penascal = None;
        }
        // What 0.2.1 brings, for the «coming soon» part (full size, overlays on):
        if wanted("corte") {
            // An audience at the throne: the request types out, an answer lights, it is granted.
            app.penascal = None;
            app.nature_mode = false;
            app.village_entered = false;
            app.castle = None;
            app.outfit = 0;
            app.environment_cycle.set_phase(0.35);
            app.player = Vec3::new(0.0, 0.0, -14.0);
            app.camera_focus = app.player;
            settle(&mut app);
            let promo_story = std::mem::take(&mut app.story);
            for (k, v) in [
                ("consejo_1_hecho", 1),
                ("tutorial_castillo", 1),
                ("reputacion", 2),
                ("obra_biblioteca", 1),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.gold = 640;
            clean(&mut app, false);
            eprintln!("[promo] corte: audience {}", app.open_audience());
            let mut beat = 0;
            film(&mut app, out, "corte", 6.5, |app, t| {
                let next = match t {
                    t if t > 3.9 => 2,
                    t if t > 3.1 => 1,
                    _ => 0,
                };
                if next != beat {
                    beat = next;
                    app.court_key(if beat == 1 {
                        KeyCode::ArrowDown
                    } else {
                        KeyCode::Enter
                    });
                }
            })?;
            app.close_court();
            clean(&mut app, true);
            app.story = promo_story;
        }
        if wanted("citas") {
            // The vanity's dates page, then Lira's second date (the waltz): its lines, the
            // choice, the memory coming on with its ribbon.
            let promo_story = std::mem::take(&mut app.story);
            for (k, v) in [
                ("tutorial_castillo", 1),
                ("cita_lira", 1),
                ("afecto_lira", 3),
                ("afecto_nyxara", 2),
                ("cita_nyxara", 1),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.environment_cycle.set_phase(0.6);
            clean(&mut app, false);
            eprintln!("[promo] citas: page {}", app.open_dates());
            let (mut opened, mut next_at, mut memory_at) = (false, 0.0f32, None::<f32>);
            film(&mut app, out, "citas", 22.0, |app, t| {
                if !opened {
                    if t > 2.4 {
                        opened = true;
                        next_at = t + 1.6;
                        app.court_key(KeyCode::Enter);
                    }
                    return;
                }
                let memory = app
                    .stage
                    .as_ref()
                    .and_then(|s| s.bg.as_deref())
                    .is_some_and(|b| b.starts_with("citas/") && !b.contains("/fondo_"));
                if memory && memory_at.is_none() {
                    memory_at = Some(t);
                }
                // the memory stays on a while before the talk goes on
                if memory_at.is_some_and(|m| t - m < 3.2) || t < next_at || app.talk.is_none() {
                    return;
                }
                let lib = guion::library();
                match app.talk.as_ref().map(|talk| talk.shown(lib, &app.story)) {
                    Some(guion::Shown::Choice { .. }) => app.talk_pick(0),
                    _ => app.talk_confirm(),
                }
                next_at = t + 1.0;
            })?;
            eprintln!("[promo] citas: memory at {memory_at:?}");
            app.end_talk();
            app.close_court();
            clean(&mut app, true);
            app.story = promo_story;
        }
        if wanted("guerra") || wanted("campana") {
            use zone_war::{Sel, WarPage};
            app.penascal = None;
            app.nature_mode = false;
            app.village_entered = false;
            app.castle = None;
            clean(&mut app, false);
            let step = |app: &mut App, frames: usize| {
                for _ in 0..frames {
                    app.tick(1.0 / 30.0);
                    app.render_frame();
                }
            };
            app.open_war_table();
            step(&mut app, 20);
            if wanted("guerra") {
                // El Peñascal's battle: the armies meet; the camera circles the thick of it, then
                // flies to the Queen as she calls her spark.
                if let Some(w) = app.war.as_mut() {
                    w.page = WarPage::Table(1);
                    w.battle = None;
                }
                app.war_hit(map_overlay::WarHit::Quick(1));
                step(&mut app, 20);
                app.war_hit(map_overlay::WarHit::Begin);
                if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                    run.b.auto_player = Some(demon_queen_war::ai::BattleAi::new(
                        0,
                        demon_queen_war::ai::Posture::Attack,
                    ));
                    run.b.speed = 3.0;
                }
                for k in 0..900 {
                    app.tick(1.0 / 30.0);
                    if k % 6 == 0 {
                        app.render_frame();
                    }
                    let melee = app
                        .war
                        .as_ref()
                        .and_then(|w| w.battle.as_ref())
                        .map_or(0, |r| {
                            r.b.sim
                                .regiments
                                .iter()
                                .filter(|g| g.state == demon_queen_war::sim::RState::Melee)
                                .count()
                        });
                    if melee >= 4 && k > 200 {
                        break;
                    }
                }
                if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                    run.b.speed = 1.0;
                }
                let hot = |app: &App| -> Option<(f32, f32)> {
                    let run = app.war.as_ref()?.battle.as_ref()?;
                    let regs: Vec<&demon_queen_war::sim::Regiment> =
                        run.b.sim.regiments.iter().filter(|r| r.alive > 0).collect();
                    let melee: Vec<&&demon_queen_war::sim::Regiment> = regs
                        .iter()
                        .filter(|r| r.state == demon_queen_war::sim::RState::Melee)
                        .collect();
                    let pick: Vec<&&demon_queen_war::sim::Regiment> = if melee.is_empty() {
                        regs.iter().collect()
                    } else {
                        melee
                    };
                    let n = pick.iter().map(|r| r.alive as f32).sum::<f32>().max(1.0);
                    Some((
                        pick.iter().map(|r| r.cx * r.alive as f32).sum::<f32>() / n,
                        pick.iter().map(|r| r.cz * r.alive as f32).sum::<f32>() / n,
                    ))
                };
                let centre = hot(&app).unwrap_or((0.0, 0.0));
                let mut sparked = false;
                film(&mut app, out, "guerra", 9.0, |app, t| {
                    let queen = app
                        .war
                        .as_ref()
                        .and_then(|w| w.battle.as_ref())
                        .and_then(|r| {
                            r.b.queen
                                .map(|q| (r.b.sim.regiments[q].cx, r.b.sim.regiments[q].cz))
                        });
                    let (x, z, dist, turn) = if t < 5.6 {
                        let k = t / 5.6;
                        let s = k * k * (3.0 - 2.0 * k);
                        (centre.0, centre.1, 92.0 - 46.0 * s, 0.45 - 0.8 * s)
                    } else {
                        let (qx, qz) = queen.unwrap_or(centre);
                        let k = ((t - 5.6) / 0.8).min(1.0);
                        let s = k * k * (3.0 - 2.0 * k);
                        (
                            centre.0 + (qx - centre.0) * s,
                            centre.1 + (qz - centre.1) * s,
                            46.0 + 14.0 * s,
                            -0.35 + 0.5 * s,
                        )
                    };
                    if t > 6.6 && !sparked {
                        sparked = true;
                        if let Some(run) = app.war.as_mut().and_then(|w| w.battle.as_mut()) {
                            if let Some(q) = run.b.queen {
                                if let Some(ab) = run.b.sim.regiments[q].ability.as_mut() {
                                    ab.cd = 0.0;
                                }
                            }
                        }
                        app.war_hit(map_overlay::WarHit::Order(4));
                    }
                    if let Some(w) = app.war.as_mut() {
                        if let Some(run) = w.battle.as_ref() {
                            let field = &run.b.sim.field;
                            w.cam
                                .frame(x, z, Some(dist), Some(turn), true, &|a: f32, b: f32| {
                                    field.walk_height(a, b)
                                });
                        }
                    }
                })?;
                if let Some(w) = app.war.as_mut() {
                    w.battle = None;
                    w.page = WarPage::Table(0);
                }
            }
            if wanted("campana") {
                // The campaign: the map, the Queen's army chosen, a march drawn to the fallen
                // capital, the turn handed over and the armies on the move.
                app.war_campaign = None;
                if let Some(w) = app.war.as_mut() {
                    w.page = WarPage::Table(0);
                }
                app.war_hit(map_overlay::WarHit::Table(1));
                step(&mut app, 30);
                app.war_hit(map_overlay::WarHit::Modal(0));
                step(&mut app, 30);
                let queen = app
                    .war_campaign
                    .as_ref()
                    .and_then(|s| s.queen_army())
                    .map(|a| a.id.clone());
                if let (Some(id), Some(c)) = (queen, app.war.as_mut().and_then(|w| w.camp.as_mut()))
                {
                    c.sel = Some(Sel::Army(id));
                    c.paint_version += 1;
                }
                for _ in 0..3 {
                    app.war_hit(map_overlay::WarHit::Panel(0));
                    step(&mut app, 2);
                }
                let (mut marched, mut ended) = (false, false);
                film(&mut app, out, "campana", 8.0, |app, t| {
                    if let Some(w) = app.war.as_mut() {
                        w.cam.zoom(if t < 3.2 { -2.4 } else { 1.2 });
                    }
                    if t > 1.2 && !marched {
                        marched = true;
                        if let Some(p) = demon_queen_war::data::PROVINCES
                            .iter()
                            .position(|p| p.id == "capital")
                        {
                            app.campaign_go(p);
                        }
                    }
                    if t > 4.4 && !ended {
                        ended = true;
                        app.war_hit(map_overlay::WarHit::EndTurn);
                    }
                })?;
            }
            app.leave_war();
            clean(&mut app, true);
        }
        return Ok(());
    }
    if std::env::args().any(|a| a == "--boss-light") {
        // The fight's lighting from where she comes in, all round, and Varrak close to.
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.story = guion::Vars::default();
        app.story.values.insert("consejo_2_hecho".into(), 1);
        app.enter_penascal_from_forest();
        app.start_boss_fight();
        anyhow::ensure!(app.boss.is_some(), "the fight did not load");
        let run = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 60.0) as usize {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        };
        run(&mut app, 0.3);
        app.boss_input.attack = true;
        run(&mut app, 1.0);
        if let Some(f) = app.boss.as_mut() {
            f.player.god = true;
            f.qa("duel");
        }
        for (name, yaw, pitch) in [
            ("light-01-north", 0.0f32, 0.04f32),
            ("light-02-west", std::f32::consts::FRAC_PI_2, 0.0),
            ("light-03-east", -std::f32::consts::FRAC_PI_2, 0.0),
            ("light-04-south", std::f32::consts::PI, 0.12),
            ("light-05-ground", 0.0, -0.32),
        ] {
            if let Some(f) = app.boss.as_mut() {
                f.player.yaw = yaw;
                f.player.pitch = pitch;
            }
            run(&mut app, 0.4);
            save(&mut app, out, name)?;
        }
        if let Some(f) = app.boss.as_mut() {
            f.qa("smash");
            f.player.pitch = 0.1;
        }
        run(&mut app, 0.25);
        save(&mut app, out, "light-06-varrak")?;
        app.boss = None;
        app.penascal = None;
        return Ok(());
    }
    if std::env::args().any(|a| a == "--boss-hands") {
        // Her hands in the fight, for checking them: at rest, in a swing, holding the seal, and
        // up on his back (the sword in his shoulder).
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.story = guion::Vars::default();
        app.story.values.insert("consejo_2_hecho".into(), 1);
        app.enter_penascal_from_forest();
        app.start_boss_fight();
        anyhow::ensure!(app.boss.is_some(), "the fight did not load");
        if let Some(f) = app.boss.as_mut() {
            f.player.god = true;
        }
        let run = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 60.0) as usize {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        };
        run(&mut app, 0.3);
        app.boss_input.attack = true;
        run(&mut app, 1.5);
        if let Some(f) = app.boss.as_mut() {
            f.qa("duel");
            f.player.pitch = -0.25;
        }
        run(&mut app, 0.6);
        save(&mut app, out, "hands-01-rest")?;
        app.boss_input.attack = true;
        run(&mut app, 0.14);
        save(&mut app, out, "hands-02-swing")?;
        run(&mut app, 0.5);
        app.boss_button(winit::event::MouseButton::Right, true);
        run(&mut app, 0.5);
        save(&mut app, out, "hands-03-seal")?;
        app.boss_button(winit::event::MouseButton::Right, false);
        if let Some(f) = app.boss.as_mut() {
            f.qa("ride");
        }
        run(&mut app, 0.6);
        save(&mut app, out, "hands-04-ride")?;
        app.boss = None;
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
        // She takes every test's blow in turn: keep her standing (the last test drops her itself).
        if let Some(f) = app.boss.as_mut() {
            f.player.god = true;
        }
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
        // The way in behind her (the arch over the gate), and the plaza from its far end.
        if let Some(f) = app.boss.as_mut() {
            f.player.yaw = std::f32::consts::PI;
            f.player.pitch = 0.42;
        }
        run(&mut app, 0.05, false);
        save(&mut app, out, "boss-02c-gate")?;
        if let Some(f) = app.boss.as_mut() {
            f.player.yaw = 0.0;
            f.player.pitch = 0.0;
        }
        // The seal held (the right button) shows on its key; let go, it stops.
        app.boss_button(winit::event::MouseButton::Right, true);
        run(&mut app, 0.4, false);
        save(&mut app, out, "boss-02b-sello")?;
        app.boss_button(winit::event::MouseButton::Right, false);
        run(&mut app, 0.2, false);
        eprintln!(
            "[boss-qa] seal after release: {}",
            app.boss.as_ref().is_some_and(|f| f.player.channeling)
        );
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
        qa(&mut app, "run");
        run(&mut app, 1.1, true);
        save(&mut app, out, "boss-07b-run")?;
        qa(&mut app, "spin");
        run(&mut app, 0.55, true);
        save(&mut app, out, "boss-07c-spin")?;
        qa(&mut app, "leap");
        run(&mut app, 0.62, true);
        save(&mut app, out, "boss-07d-leap")?;
        run(&mut app, 0.36, true);
        save(&mut app, out, "boss-07e-land")?;
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
        // The quick events (VARRAK_COMBATE_2.md): the dodge, the ride, the fist, the net.
        let qte_up = |app: &App, kind: varrak_fight::QteKind| {
            app.boss
                .as_ref()
                .is_some_and(|f| f.qte.as_ref().is_some_and(|q| q.kind == kind))
        };
        qa(&mut app, "dodge");
        for _ in 0..300 {
            run(&mut app, 1.0 / 60.0, true);
            if qte_up(&app, varrak_fight::QteKind::Dodge) {
                break;
            }
        }
        run(&mut app, 0.35, true);
        save(&mut app, out, "boss-09b-dodge")?;
        app.boss_input.jump = true;
        run(&mut app, 0.5, true);
        save(&mut app, out, "boss-09c-dodged")?;
        qa(&mut app, "mount");
        run(&mut app, 0.4, true);
        save(&mut app, out, "boss-09d-mount")?;
        app.boss_input.attack = true;
        run(&mut app, 0.55, false);
        save(&mut app, out, "boss-09e-ride")?;
        // Ride him through: each click on its beat, then the seal on his nape.
        fn on_the_beat(app: &mut App) {
            for _ in 0..240 {
                let due = app.boss.as_ref().and_then(|f| f.qte.as_ref()).is_some_and(|q| {
                    matches!(q.ask, varrak_fight::QteAsk::Beat { beat, .. } if q.t >= beat - 0.01)
                });
                if due {
                    app.boss_input.attack = true;
                    run(app, 1.0 / 60.0, false);
                    return;
                }
                run(app, 1.0 / 60.0, false);
            }
        }
        run(&mut app, 0.2, false);
        save(&mut app, out, "boss-09f-ride-beat")?;
        on_the_beat(&mut app);
        run(&mut app, 0.15, false);
        save(&mut app, out, "boss-09f1-ride-stab")?;
        on_the_beat(&mut app);
        app.boss_button(winit::event::MouseButton::Right, true);
        run(&mut app, 0.8, false);
        save(&mut app, out, "boss-09f2-ride-seal")?;
        run(&mut app, 1.2, false);
        app.boss_button(winit::event::MouseButton::Right, false);
        run(&mut app, 0.3, false);
        save(&mut app, out, "boss-09f3-ride-done")?;
        eprintln!(
            "[boss-qa] after the ride: {:?}",
            app.boss
                .as_ref()
                .map(|f| (f.varrak.state, f.varrak.spark, f.ride.is_some()))
        );
        qa(&mut app, "grab");
        run(&mut app, 0.7, false);
        save(&mut app, out, "boss-09g-grab")?;
        for k in 0..60 {
            app.boss_input.attack = k % 3 == 0;
            run(&mut app, 1.0 / 60.0, false);
        }
        save(&mut app, out, "boss-09h-grab-mash")?;
        qa(&mut app, "rope");
        for _ in 0..360 {
            run(&mut app, 1.0 / 60.0, false);
            if qte_up(&app, varrak_fight::QteKind::Rope) {
                break;
            }
        }
        run(&mut app, 0.25, false);
        save(&mut app, out, "boss-09i-rope")?;
        app.boss_input.attack = true;
        run(&mut app, 0.9, false);
        save(&mut app, out, "boss-09j-net")?;
        eprintln!(
            "[boss-qa] after the net: {:?}",
            app.boss
                .as_ref()
                .map(|f| (f.varrak.state, f.varrak.why, f.rope_cut))
        );
        run(&mut app, 1.2, false);
        save(&mut app, out, "boss-09k-net-landed")?;
        qa(&mut app, "cauldron");
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
        run(&mut app, 0.8, true);
        save(&mut app, out, "boss-09l-cauldron")?;
        eprintln!(
            "[boss-qa] the cauldron: spill {:?}",
            app.boss.as_ref().map(|f| f.spill.is_some())
        );
        qa(&mut app, "rockfall");
        run(&mut app, 0.9, false);
        if let Some(f) = app.boss.as_mut() {
            f.player.pitch = -0.45;
            f.player.yaw = 0.0;
        }
        run(&mut app, 0.05, false);
        save(&mut app, out, "boss-09m-rockfall")?;
        run(&mut app, 1.1, false);
        save(&mut app, out, "boss-09n-rock-lands")?;
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
        // The awakening: the seal held while his memories come up, then his name.
        run(&mut app, 2.0, true);
        save(&mut app, out, "boss-12b-awaken")?;
        run(&mut app, 1.4, true);
        save(&mut app, out, "boss-12c-name")?;
        app.boss_input.attack = true;
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
    if std::env::args().any(|a| a == "--nemi-arena") {
        // Nemi waits in the arena once she has given the sword: the guide points there; a
        // training session brings the Queen to her (behind the scene).
        let settle = |app: &mut App, frames: usize| {
            for _ in 0..frames {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
        };
        app.penascal = None;
        app.nature_mode = false;
        app.village_entered = false;
        app.castle = None;
        app.player = Vec3::new(-17.6, 0.0, -12.6);
        app.environment_cycle.set_phase(0.42);
        // The gift first: Nemi already waits at the arena, and the scene brings the Queen there.
        app.story = guion::Vars::default();
        for v in ["consejo_1_hecho", "lamento_visto"] {
            app.story.values.insert(v.to_owned(), 1);
        }
        settle(&mut app, 60);
        let nemi = app.castle.as_ref().map(|c| c.maid(1).0);
        eprintln!(
            "[nemi-qa] gift due: nemi at {nemi:?}, queen at {:?}",
            app.player
        );
        anyhow::ensure!(app.start_talk(Some("nemi"), None), "no gift scene");
        settle(&mut app, 30);
        let scene = app
            .talk
            .as_ref()
            .map(|t| t.scene_id(guion::library()).to_owned());
        let d = nemi.map(|n| (n.x - app.player.x).hypot(n.z - app.player.z));
        eprintln!(
            "[nemi-qa] {scene:?}: queen at {:?}, {d:?} m from Nemi",
            app.player
        );
        app.talk = None;
        app.stage = None;
        settle(&mut app, 60);
        save(&mut app, out, "nemi-00-gift-arena")?;
        app.castle = None;
        app.player = Vec3::new(-17.6, 0.0, -12.6);
        app.story = guion::Vars::default();
        for v in ["consejo_1_hecho", "lamento_visto", "espada_regalo"] {
            app.story.values.insert(v.to_owned(), 1);
        }
        settle(&mut app, 60);
        save(&mut app, out, "nemi-01-guide-from-bedroom")?;
        let nemi = app.castle.as_ref().map(|c| c.maid(1).0);
        eprintln!("[nemi-qa] nemi at {nemi:?}");
        anyhow::ensure!(app.start_talk_at("acto1.entreno"), "no training scene");
        for _ in 0..12 {
            let shown = app
                .talk
                .as_ref()
                .map(|t| t.shown(guion::library(), &app.story));
            if matches!(shown, Some(guion::Shown::Choice { .. })) {
                break;
            }
            app.elapsed += 3.0;
            app.talk_confirm();
            settle(&mut app, 2);
        }
        app.talk_pick(0);
        for _ in 0..8 {
            settle(&mut app, 10);
        }
        let d = nemi.map(|n| (n.x - app.player.x).hypot(n.z - app.player.z));
        eprintln!(
            "[nemi-qa] after @llevar: queen at {:?}, {d:?} m from Nemi",
            app.player
        );
        while app.talk.is_some() {
            app.elapsed += 3.0;
            app.talk_confirm();
            settle(&mut app, 4);
            if app.stage.as_ref().is_some_and(|s| s.outro_at.is_some()) {
                app.talk_confirm();
                app.end_talk();
            }
        }
        app.stage = None;
        settle(&mut app, 90);
        save(&mut app, out, "nemi-02-arena")?;
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
            // The world runs on a while (the stage's people come and go, Chiquita springs up).
            for _ in 0..36 {
                app.tick(1.0 / 30.0);
                app.elapsed += 1.0 / 30.0;
            }
            app.elapsed += 3.0;
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
            // The world runs on a while (Chiquita springs up for her lines and ducks for the rest).
            for _ in 0..24 {
                app.tick(1.0 / 30.0);
                app.elapsed += 1.0 / 30.0;
            }
            app.elapsed += 1.8;
            let lib = guion::library();
            let shown = app.talk.as_ref().map(|t| t.shown(lib, &app.story));
            let choice = matches!(shown, Some(guion::Shown::Choice { .. }));
            if choice || steps % 5 == 0 {
                let label = if choice { "eleccion" } else { "linea" };
                save(&mut app, out, &format!("stage-{tag}-{shot:02}-{label}"))?;
                shot += 1;
            }
            let before = app.elapsed;
            // VELVET_STAGE_MOUSE=1: played with the mouse, a click on each option's card (and one
            // beside them first, which must do nothing) and on the screen for each line.
            let mouse = std::env::var("VELVET_STAGE_MOUSE").is_ok_and(|v| v == "1");
            if choice {
                let pick = picks.get(chosen).copied().unwrap_or(0);
                chosen += 1;
                if mouse {
                    app.talk_click_at(Some((40.0, 340.0)));
                    app.talk_click_at(Some((960.0, 330.0 + pick as f32 * 110.0 + 44.0)));
                } else {
                    app.talk_pick(pick);
                }
            } else if mouse {
                app.talk_click_at(Some((960.0, 900.0)));
            } else {
                app.talk_confirm();
            }
            // VELVET_STAGE_FILM=all (or `3,7,12`): those steps filmed, a frame every 1/15 s for
            // 1.1 s (3.3 s when a reveal or a turn plays), to see their animations. (A tick moves
            // the clock itself here, through the zone; a capture moves it 4/30 s, taken back.)
            let film = std::env::var("VELVET_STAGE_FILM")
                .ok()
                .is_some_and(|v| v == "all" || v.split(',').any(|n| n.trim().parse() == Ok(steps)));
            if film {
                let long = app.stage.as_ref().is_some_and(|s| {
                    s.reveal.as_ref().is_some_and(|(_, t)| *t >= before - 0.01)
                        || s.actors
                            .iter()
                            .any(|a| a.spin.as_ref().is_some_and(|(_, t, _)| *t >= before - 0.01))
                });
                for f in 0..if long { 50 } else { 17 } {
                    save(&mut app, out, &format!("film-{steps:03}-{f:02}"))?;
                    app.elapsed -= 4.0 / 30.0;
                    for _ in 0..2 {
                        app.tick(1.0 / 30.0);
                    }
                }
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
                } else if stage.panels.iter().flatten().any(|p| fresh(p.at)) {
                    Some("panel")
                } else if stage
                    .actors
                    .iter()
                    .any(|a| a.spin.as_ref().is_some_and(|(_, t, _)| fresh(*t)))
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
        // At the brothel's desk, as in the game.
        app.environment_cycle.set_phase(0.0);
        app.story = guion::Vars::default();
        into_brothel_room(&mut app, "");
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
        // `--play-scene alba|alba-futa|noa`. The girls' games in their rooms upstairs, as in the
        // game; the rest in the castle (its first walk's tips kept quiet).
        let girl = if which.starts_with("alba") {
            Some("alba")
        } else if which.starts_with("noa") {
            Some("noa")
        } else {
            None
        };
        app.environment_cycle.set_phase(0.0);
        if let Some(id) = girl {
            into_brothel_room(&mut app, id);
        } else {
            app.story.values.insert("tutorial_castillo".to_owned(), 1);
            app.nature_mode = false;
            app.village_entered = false;
            app.player = Vec3::new(-17.6, 0.0, -12.6);
            app.castle = None;
            for _ in 0..60 {
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
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
        if matches!(game, play_scene::Game::Tienda(_)) {
            // As in the game, a haggle under it: the crimson essence, a little haggled already,
            // and not gold enough for it.
            app.shop_open = true;
            app.shop_opened_at = app.elapsed - 10.0;
            app.shop_tab = shop::Tab::Buy;
            app.shop_category = shop::Category::All;
            app.shop_selection = app
                .shop_product_indices()
                .iter()
                .position(|i| shop::catalog().products[*i].id == "esencia_carmesi")
                .unwrap_or(0);
            app.gold = 40;
            app.start_haggle();
            if let Some(h) = app.haggle.as_mut() {
                h.percent = 15;
            }
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
    if let Some(girl) = std::env::args()
        .position(|a| a == "--caricias")
        .and_then(|i| std::env::args().nth(i + 1))
    {
        // A night's touch game: her coming in, the hand on her hair, a garment off, her warmed
        // up, the end with its clip and its card.
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.0);
        app.castle = None;
        // Her first walk of the castle (its keys from Chiquita) is long gone by her nights.
        app.story.values.insert("tutorial_castillo".to_owned(), 1);
        for _ in 0..60 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        eprintln!("[caricias-qa] open: {}", app.open_caricias(&girl));
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        let point = |app: &mut App, p: [f32; 2]| {
            if let Some(c) = app.caricias.as_mut() {
                c.pointer(p[0], p[1]);
            }
        };
        wait(&mut app, 0.6);
        save(&mut app, out, "caricias-01-enter")?;
        // The hand on her hair, slowly back and forth.
        let hair = caricias::from_art([if girl == "lira" { 850.0 } else { 890.0 }, 100.0]);
        point(&mut app, hair);
        if let Some(c) = app.caricias.as_mut() {
            c.press(false);
        }
        for k in 0..90 {
            point(
                &mut app,
                [hair[0] + (k as f32 * 0.25).sin() * 40.0, hair[1]],
            );
            wait(&mut app, 1.0 / 30.0);
        }
        save(&mut app, out, "caricias-02-hair")?;
        if let Some(c) = app.caricias.as_mut() {
            c.release();
            // Warmed up enough for the cap to come off (QA shortcut), then pulled down off her.
            c.deseo = 45.0;
        }
        let handle = app
            .caricias
            .as_ref()
            .map(|c| c.handle(0))
            .unwrap_or([0.0; 2]);
        point(&mut app, handle);
        if let Some(c) = app.caricias.as_mut() {
            c.press(false);
        }
        for k in 0..20 {
            point(
                &mut app,
                [handle[0] + k as f32 * 9.0, handle[1] - k as f32 * 3.0],
            );
            wait(&mut app, 1.0 / 30.0);
        }
        save(&mut app, out, "caricias-03-pulling")?;
        if let Some(c) = app.caricias.as_mut() {
            c.release();
        }
        wait(&mut app, 0.6);
        save(&mut app, out, "caricias-04-undressed")?;
        // A grab on her chest: her body answers.
        if let Some(c) = app.caricias.as_mut() {
            c.set_tool(2);
            c.deseo = 70.0;
        }
        let chest = caricias::from_art([if girl == "lira" { 850.0 } else { 888.0 }, 520.0]);
        point(&mut app, chest);
        if let Some(c) = app.caricias.as_mut() {
            c.press(false);
        }
        wait(&mut app, 0.35);
        save(&mut app, out, "caricias-05-grab")?;
        if let Some(c) = app.caricias.as_mut() {
            c.release();
            // Everything off, and to the end.
            for i in 0..c.worn.len() {
                c.worn[i] = false;
            }
            c.deseo = 99.5;
            c.set_tool(5);
        }
        let lips = caricias::from_art([if girl == "lira" { 846.0 } else { 885.0 }, 266.0]);
        point(&mut app, lips);
        if let Some(c) = app.caricias.as_mut() {
            c.press(true);
        }
        wait(&mut app, 0.3);
        save(&mut app, out, "caricias-06-climax")?;
        wait(&mut app, 2.0);
        save(&mut app, out, "caricias-07-clip")?;
        for _ in 0..40 {
            if app.caricias.as_ref().is_some_and(|c| c.card) {
                break;
            }
            wait(&mut app, 0.25);
        }
        wait(&mut app, 0.5);
        save(&mut app, out, "caricias-08-card")?;
        eprintln!(
            "[caricias-qa] ended: {:?}",
            app.caricias.as_ref().and_then(|c| c.ended).map(|(k, _)| k)
        );
        app.caricias_key(KeyCode::Enter);
        eprintln!(
            "[caricias-qa] closed: {} afecto_{girl} {}",
            app.caricias.is_none(),
            app.story.get(&format!("afecto_{girl}"))
        );
        return Ok(());
    }
    if std::env::args().any(|a| a == "--court") {
        // The court: an audience at the throne and its answer, the works and one restored, the
        // armoury, Nyxara's lessons (her words, the runes shown, repeated and won, the gift), and
        // Nemi's training (a blow coming, parries, the end).
        app.nature_mode = false;
        app.village_entered = false;
        app.environment_cycle.set_phase(0.35);
        app.player = Vec3::new(0.0, 0.0, -14.0);
        app.castle = None;
        for _ in 0..90 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        app.story = guion::Vars::default();
        for (k, v) in [
            ("consejo_1_hecho", 1),
            ("espada_regalo", 1),
            ("entreno", 3),
            ("maids_trust", 3),
            ("reputacion", 2),
            ("obra_biblioteca", 1),
        ] {
            app.story.values.insert(k.into(), v);
        }
        app.gold = 640;
        app.story.values.insert("tutorial_castillo".to_owned(), 1);
        let wait = |app: &mut App, secs: f32| {
            for _ in 0..(secs * 30.0).round() as usize {
                app.tick(1.0 / 30.0);
                app.render_frame();
            }
        };
        let shots: Vec<&str> = std::env::var("VELVET_COURT")
            .map(|v| {
                v.split(',')
                    .map(|s| Box::leak(s.to_owned().into_boxed_str()) as &str)
                    .collect()
            })
            .unwrap_or_else(|_| {
                vec![
                    "audiencia",
                    "obras",
                    "armeria",
                    "lecciones",
                    "entreno",
                    "manana",
                    "catalejo",
                    "codice",
                ]
            });
        if shots.contains(&"noche_maren") {
            // Maren's night, the way a player gets there: she stayed, she trusts the Queen, and
            // «Quédate esta noche» ends in her touch game.
            for (k, v) in [
                ("prisionera_queda", 1),
                ("prisionera_n", 1),
                ("maren_afecto", 2),
            ] {
                app.story.values.insert(k.into(), v);
            }
            let started = app.start_talk_at("corte.noche_maren");
            eprintln!("[court-qa] noche_maren: {started}");
            for step in 0..60 {
                if app.talk.is_none() {
                    break;
                }
                let lib = guion::library();
                let shown = app.talk.as_ref().map(|t| t.shown(lib, &app.story));
                match shown {
                    Some(guion::Shown::Choice { .. }) => {
                        wait(&mut app, 0.6);
                        save(&mut app, out, "court-40-maren-night")?;
                        app.talk_pick(0);
                    }
                    _ => {
                        app.elapsed += 5.0;
                        app.talk_confirm();
                    }
                }
                if step == 59 {
                    eprintln!("[court-qa] noche_maren: the talk never ended");
                }
            }
            wait(&mut app, 1.0);
            eprintln!(
                "[court-qa] noche_maren: talk over {}, caricias {}",
                app.talk.is_none(),
                app.caricias.as_ref().map(|c| c.girl.id).unwrap_or("-")
            );
            save(&mut app, out, "court-41-maren-caricias")?;
            app.caricias = None;
        }
        if shots.contains(&"citas") {
            // The vanity's dates: the page as it opens (Nyxara still waits for affection), Lira's
            // first date played through (its choice, the memory coming on, the end card), the page
            // after it, then the night: Nyxara's first night and both its memories.
            // (Run with DEMON_QUEEN_GALLERY pointing elsewhere: the memories go into the gallery.)
            let play = |app: &mut App, tag: &str| -> anyhow::Result<()> {
                let mut chose = false;
                let mut memories: Vec<String> = Vec::new();
                for _ in 0..160 {
                    let outro = app.stage.as_ref().is_some_and(|s| s.outro_at.is_some());
                    if app.talk.is_none() {
                        if outro {
                            wait(app, 1.2);
                            save(app, out, &format!("court-{tag}-outro"))?;
                            if let Some(stage) = app.stage.as_mut() {
                                stage.unlocks.clear();
                            }
                            app.end_talk();
                        }
                        break;
                    }
                    let lib = guion::library();
                    match app.talk.as_ref().map(|t| t.shown(lib, &app.story)) {
                        Some(guion::Shown::Choice { .. }) => {
                            wait(app, 0.8);
                            if !chose {
                                save(app, out, &format!("court-{tag}-choice"))?;
                                chose = true;
                            }
                            app.talk_pick(0);
                        }
                        _ => {
                            app.elapsed += 5.0;
                            app.talk_confirm();
                        }
                    }
                    wait(app, 0.05);
                    let shown = app
                        .stage
                        .as_ref()
                        .and_then(|s| s.bg.clone())
                        .filter(|b| b.starts_with("citas/") && !b.contains("/fondo_"));
                    if let Some(memory) = shown.filter(|m| !memories.contains(m)) {
                        wait(app, 0.7);
                        save(
                            app,
                            out,
                            &format!("court-{tag}-memory{}", memories.len() + 1),
                        )?;
                        wait(app, 1.5);
                        save(
                            app,
                            out,
                            &format!("court-{tag}-memory{}-line", memories.len() + 1),
                        )?;
                        memories.push(memory);
                    }
                }
                eprintln!("[court-qa] citas {tag}: memories {memories:?}");
                Ok(())
            };
            app.story.values.insert("afecto_nyxara".into(), 0);
            app.open_dates();
            wait(&mut app, 1.2);
            save(&mut app, out, "court-50-dates")?;
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.3);
            play(&mut app, "51-lira1")?;
            eprintln!(
                "[court-qa] citas: cita_lira {} afecto_lira {} cita_hoy {}",
                app.story.get("cita_lira"),
                app.story.get("afecto_lira"),
                app.story.get("cita_hoy")
            );
            app.open_dates();
            wait(&mut app, 1.2);
            save(&mut app, out, "court-52-dates-after")?;
            app.close_court();
            for (k, v) in [
                ("cita_nyxara", 4),
                ("afecto_nyxara", 10),
                ("cita_hoy", 0),
                ("noche_hoy", 0),
                ("cita_lira", 5),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.environment_cycle.set_phase(0.95);
            app.open_dates();
            wait(&mut app, 1.2);
            save(&mut app, out, "court-53-dates-night")?;
            if let Some(c) = app.court.as_mut() {
                c.choice = 1;
            }
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.3);
            play(&mut app, "54-nyx5")?;
            eprintln!(
                "[court-qa] citas: cita_nyxara {} noche_hoy {}",
                app.story.get("cita_nyxara"),
                app.story.get("noche_hoy")
            );
            // Nemi: her first date by day, then her first night
            app.environment_cycle.set_phase(0.35);
            for (k, v) in [("cita_hoy", 0), ("cita_nemi", 0), ("afecto_nemi", 0)] {
                app.story.values.insert(k.into(), v);
            }
            app.open_dates();
            if let Some(c) = app.court.as_mut() {
                c.choice = 2;
            }
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.3);
            play(&mut app, "55-nemi1")?;
            for (k, v) in [
                ("cita_hoy", 0),
                ("noche_hoy", 0),
                ("cita_nemi", 4),
                ("afecto_nemi", 10),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.environment_cycle.set_phase(0.95);
            app.open_dates();
            if let Some(c) = app.court.as_mut() {
                c.choice = 2;
            }
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.3);
            play(&mut app, "56-nemi5")?;
            app.open_dates();
            wait(&mut app, 1.2);
            save(&mut app, out, "court-57-dates-nemi-done")?;
            app.close_court();
            eprintln!("[court-qa] citas: cita_nemi {}", app.story.get("cita_nemi"));
            app.environment_cycle.set_phase(0.35);
        }
        if shots.contains(&"manana") {
            // A morning: yesterday's tasks done and told, today's to give.
            for (k, v) in [
                ("obra_cocina", 1),
                ("obra_enfermeria", 1),
                ("prisionera_queda", 1),
                ("tarea_lira", 2),
                ("tarea_nemi", 3),
                ("tarea_nyxara", 3),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.court_new_day(true);
            eprintln!("[court-qa] morning: {} reports", app.court_report.len());
            app.open_routine();
            wait(&mut app, 1.2);
            save(&mut app, out, "court-20-morning")?;
            app.court_key(KeyCode::ArrowDown);
            app.court_key(KeyCode::ArrowRight);
            wait(&mut app, 0.3);
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.4);
            save(&mut app, out, "court-21-morning-given")?;
            app.close_court();
        }
        if shots.contains(&"catalejo") {
            for (k, v) in [
                ("varrak_despierto", 1),
                ("consejo_2_hecho", 1),
                ("prisionera_queda", 1),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.open_telescope();
            wait(&mut app, 1.0);
            save(&mut app, out, "court-22-telescope")?;
            app.court_key(KeyCode::Enter);
            wait(&mut app, 1.2);
            save(&mut app, out, "court-23-telescope-north")?;
            app.court_key(KeyCode::Enter);
            app.court_key(KeyCode::KeyS);
            app.court_key(KeyCode::Enter);
            wait(&mut app, 1.2);
            save(&mut app, out, "court-24-telescope-south")?;
            app.close_court();
        }
        if shots.contains(&"codice") {
            for (k, v) in [
                ("codice_pista", 2),
                ("maren_informes", 2),
                ("varrak_despierto", 1),
            ] {
                app.story.values.insert(k.into(), v);
            }
            app.open_codex();
            wait(&mut app, 1.0);
            save(&mut app, out, "court-25-codex")?;
            app.court_key(KeyCode::Enter);
            app.court_key(KeyCode::ArrowDown);
            wait(&mut app, 0.12);
            save(&mut app, out, "court-26-codex-turning")?;
            wait(&mut app, 0.6);
            save(&mut app, out, "court-27-codex-page")?;
            app.close_court();
        }
        if shots.contains(&"audiencia") {
            let heard = app.open_audience();
            eprintln!(
                "[court-qa] audience: {heard} view: {}",
                app.court_view().is_some()
            );
            wait(&mut app, 0.07);
            save(&mut app, out, "court-00-opening")?;
            wait(&mut app, 0.83);
            save(&mut app, out, "court-01-audience")?;
            app.court_key(KeyCode::ArrowDown);
            wait(&mut app, 0.3);
            save(&mut app, out, "court-02-audience-lit")?;
            app.court_key(KeyCode::Enter);
            wait(&mut app, 1.0);
            save(&mut app, out, "court-03-answer")?;
            app.close_court();
        }
        if shots.contains(&"obras") {
            eprintln!("[court-qa] works: {}", app.open_works());
            wait(&mut app, 0.9);
            save(&mut app, out, "court-04-works")?;
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.5);
            save(&mut app, out, "court-05-restored")?;
            app.close_court();
        }
        if shots.contains(&"armeria") {
            app.story.values.insert("obra_armeria".into(), 1);
            app.open_armory();
            wait(&mut app, 0.9);
            save(&mut app, out, "court-06-armory")?;
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.3);
            save(&mut app, out, "court-06b-bought")?;
            app.close_court();
        }
        if shots.contains(&"lecciones") {
            eprintln!("[court-qa] lessons: {}", app.open_lessons());
            wait(&mut app, 0.9);
            save(&mut app, out, "court-07-lessons")?;
            app.court_key(KeyCode::Enter);
            wait(&mut app, 0.6);
            save(&mut app, out, "court-08-lesson-talk")?;
            for _ in 0..12 {
                if !matches!(
                    app.court.as_ref().map(|c| &c.page),
                    Some(court::Page::LessonTalk { .. })
                ) {
                    break;
                }
                app.court_key(KeyCode::Enter);
                wait(&mut app, 0.1);
            }
            wait(&mut app, 1.0);
            save(&mut app, out, "court-09-runes-showing")?;
            // Repeat each row as it is shown (reading it off the game).
            for _ in 0..3 {
                for _ in 0..240 {
                    let turn = matches!(
                        app.court.as_ref().map(|c| &c.page),
                        Some(court::Page::Runes { game, .. }) if game.showing.is_none() && game.next_round_at.is_none()
                    );
                    if turn {
                        break;
                    }
                    wait(&mut app, 1.0 / 30.0);
                }
                let row: Vec<u8> = match app.court.as_ref().map(|c| &c.page) {
                    Some(court::Page::Runes { game, .. }) => game.sequence.clone(),
                    _ => Vec::new(),
                };
                if row.is_empty() {
                    break;
                }
                save(&mut app, out, &format!("court-10-runes-turn-{}", row.len()))?;
                for r in row {
                    let key = [
                        KeyCode::Digit1,
                        KeyCode::Digit2,
                        KeyCode::Digit3,
                        KeyCode::Digit4,
                        KeyCode::Digit5,
                        KeyCode::Digit6,
                    ][r as usize % 6];
                    app.court_key(key);
                    wait(&mut app, 0.15);
                }
            }
            wait(&mut app, 0.3);
            save(&mut app, out, "court-11-runes-won")?;
            wait(&mut app, 1.4);
            save(&mut app, out, "court-12-lesson-end")?;
            eprintln!(
                "[court-qa] gifts: chispa {} afecto_nyxara {}",
                app.story.get(court::DON_CHISPA),
                app.story.get("afecto_nyxara")
            );
            app.close_court();
        }
        if shots.contains(&"entreno") {
            eprintln!("[court-qa] training: {}", app.open_training());
            wait(&mut app, 1.0);
            save(&mut app, out, "court-13-training")?;
            // Parry most blows on time (the first two let through), reading them off the game.
            let mut saved_coming = false;
            for step in 0..(60 * 30) {
                let now = app.elapsed;
                let (due, over) = match app.court.as_ref().map(|c| &c.page) {
                    Some(court::Page::Training(d)) => (
                        d.blows
                            .iter()
                            .enumerate()
                            .find(|(_, b)| b.result.is_none())
                            .map(|(i, b)| (i, b.from, b.at - (now - d.started))),
                        d.over.is_some(),
                    ),
                    _ => (None, true),
                };
                if over {
                    break;
                }
                if let Some((i, from, left)) = due {
                    if !saved_coming && left < 0.5 {
                        saved_coming = true;
                        save(&mut app, out, "court-14-blow-coming")?;
                    }
                    if i >= 2 && left.abs() < 0.05 {
                        app.court_key(match from {
                            0 => KeyCode::KeyA,
                            1 => KeyCode::KeyD,
                            _ => KeyCode::KeyW,
                        });
                        if i == 4 {
                            wait(&mut app, 0.15);
                            save(&mut app, out, "court-15-parried")?;
                        }
                    }
                }
                let _ = step;
                wait(&mut app, 1.0 / 30.0);
            }
            wait(&mut app, 0.6);
            save(&mut app, out, "court-16-training-end")?;
            eprintln!("[court-qa] skill: {}", app.story.get("destreza"));
            app.close_court();
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
        // VELVET_ALCOVE=invite: the night's «Invitar a alguien» page, every guest able to come.
        let invite = std::env::var("VELVET_ALCOVE").is_ok_and(|v| v == "invite");
        if invite {
            for (k, v) in [
                ("maids_trust", 3),
                ("afecto_nyxara", 2),
                ("prisionera_queda", 1),
                ("prisionera_n", 1),
                ("maren_afecto", 2),
            ] {
                app.story.values.insert(k.into(), v);
            }
        }
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
        if invite {
            for _ in 0..4 {
                app.alcove_key(KeyCode::ArrowRight);
            }
            app.alcove_key(KeyCode::Enter);
            wait(&mut app, 0.7);
            save(&mut app, out, "alcove-20-invite")?;
            app.alcove_key(KeyCode::ArrowRight);
            app.alcove_key(KeyCode::ArrowRight);
            wait(&mut app, 0.4);
            save(&mut app, out, "alcove-21-invite-maren")?;
            return Ok(());
        }
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
        // In the bikini, as she is there once the welcome has given it to her.
        app.nature_mode = true;
        app.village_entered = true;
        app.achievements.grant_outfit("bikini");
        app.change_into_bikini();
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
    if std::env::args().any(|a| a == "--mina2") {
        // La Mina 2.0 in the game: the prototype's views (feet, yaw and pitch in degrees, three.js'
        // frame), the Queen standing at each.
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.inventory.insert("espada_de_madera".into(), 1);
        app.dungeon = dungeon::Dungeon::mine().map_err(anyhow::Error::msg)?;
        app.enter_dungeon_unchecked();
        let views: &[(&str, [f32; 3], f32, f32)] = &[
            ("fuera", [0.0, 0.0, 15.0], 0.0, 6.0),
            ("boca", [0.6, 0.0, 3.5], 0.0, 2.0),
            ("norte", [0.4, -0.4, -9.0], 0.0, -2.0),
            ("carga", [-1.5, -3.0, -44.5], -8.0, 6.0),
            ("biblioteca", [3.5, -3.6, -74.2], 0.0, 6.0),
            ("cripta", [-18.4, -3.7, -73.8], 30.0, 4.0),
            ("forja", [27.4, -3.0, -64.0], 35.0, 4.0),
            ("gruta", [52.6, -13.5, -52.5], 0.0, 4.0),
            ("puente", [52.0, -14.0, -67.5], 0.0, -4.0),
            ("tesoro", [68.8, -14.0, -81.0], -90.0, 6.0),
            ("sello", [53.0, -14.4, -93.5], 0.0, 10.0),
            ("corazon", [53.2, -16.0, -118.8], 0.0, 12.0),
            ("tendero", [7.6, 0.0, 11.3], 0.0, -4.0),
            ("umbral", [53.0, -14.4, -104.5], 0.0, 4.0),
            ("umbral_izq", [50.0, -14.4, -105.5], -25.0, 4.0),
            ("umbral_atras", [53.0, -14.8, -112.5], 180.0, 4.0),
            // where the void finder (art/blender/voids.py) saw out of the mine
            ("hueco_cripta", [-28.9, -3.7, -84.1], -62.3, 39.1),
            ("hueco_tesoro", [77.1, -14.0, -84.1], -51.7, -18.6),
            ("hueco_brecha", [57.1, -14.4, -92.1], 118.3, -15.6),
            ("hueco_santuario", [-22.9, -3.7, -88.1], 25.6, -1.8),
            ("hueco_arcana", [-29.9, -3.5, -32.1], -17.5, 58.9),
            ("hueco_biblioteca", [3.1, -3.6, -80.1], 175.6, 6.8),
            ("techo_tesoro", [72.0, -14.0, -76.0], 0.0, 50.0),
            ("cupula_arcana", [-28.5, -3.5, -33.0], 0.0, 60.0),
            ("hueco_corazon", [47.1, -16.0, -121.1], 150.8, -11.9),
            ("hueco_hongos", [22.1, -14.1, -78.1], 140.2, 48.1),
            ("hueco_limos", [11.1, -14.4, -71.1], 127.8, 35.5),
            // the way into the library, from the loading hall to its door
            ("biblio_1", [4.0, -3.0, -56.0], 0.0, 0.0),
            ("biblio_2", [4.6, -3.0, -61.0], 0.0, 0.0),
            ("biblio_3", [4.3, -3.15, -66.0], 0.0, 0.0),
            ("biblio_4", [3.6, -3.45, -70.5], 0.0, 0.0),
        ];
        // VELVET_MINA_OPEN: the seal turned and the key in hand (the Heart's throat open)
        let open = std::env::var("VELVET_MINA_OPEN").is_ok();
        app.dungeon.seal = open;
        app.dungeon.crest = open;
        let only = std::env::var("VELVET_MINA_VIEW").ok();
        for (name, p, yaw, pitch) in views {
            if only
                .as_deref()
                .is_some_and(|o| !o.split(',').any(|v| v == *name))
            {
                continue;
            }
            let at = Vec3::new(p[0], p[1] + 0.3, p[2]);
            let floor = app.dungeon.map.capsule_floor(at, 0.28).unwrap_or(p[1]);
            app.dungeon.controller.position = Vec3::new(p[0], floor, p[2]);
            app.dungeon.controller.yaw = yaw.to_radians();
            app.dungeon.controller.pitch = pitch.to_radians();
            app.dungeon.notice_until = 0.0;
            app.player = app.dungeon.controller.position;
            save(&mut app, out, &format!("mina-{name}"))?;
        }
        // The Heart awake: its guardian before it, the blood rain, a ring racing out, a spit of orbs.
        if only
            .as_deref()
            .is_none_or(|o| o.split(',').any(|v| v == "pelea"))
        {
            if let Some((_, floor)) = app.dungeon.authored.as_ref().and_then(|m| m.heart) {
                let at = Vec3::new(floor.x - 3.0, floor.y, floor.z + 8.5);
                let y = app
                    .dungeon
                    .map
                    .capsule_floor(at + Vec3::Y * 0.4, 0.28)
                    .unwrap_or(floor.y);
                app.dungeon.controller.position = Vec3::new(at.x, y, at.z);
                app.dungeon.controller.yaw = 0.0;
                app.dungeon.controller.pitch = 0.2;
                app.player = app.dungeon.controller.position;
                app.dungeon.heart.awake = true;
                app.dungeon.heart.rain = 0.85;
                app.dungeon.heart.beat_in = 0.75;
                app.dungeon.heart.wave = Some((floor, 0.9));
                save(&mut app, out, "mina-pelea")?;
                app.dungeon.heart = Default::default();
            }
        }
        // One foe of each kind in a row before her in the loading hall, frozen in the clip
        // VELVET_FOE_POSE names (their look, their weapons, their stance).
        if only
            .as_deref()
            .is_some_and(|o| o.split(',').any(|v| v == "poses"))
        {
            let eye = Vec3::new(0.0, -3.0, -45.5);
            let y = app
                .dungeon
                .map
                .capsule_floor(eye + Vec3::Y * 0.4, 0.28)
                .unwrap_or(eye.y);
            app.dungeon.controller.position = Vec3::new(eye.x, y, eye.z);
            app.dungeon.controller.yaw = 0.0;
            app.dungeon.controller.pitch = -0.12;
            app.player = app.dungeon.controller.position;
            let kinds = ["lurker", "bone_scout", "bone_guard", "grave_mage"];
            for (k, kind) in kinds.iter().enumerate() {
                let Some(index) = app.dungeon.actors.iter().position(|a| {
                    let s = &app.dungeon.map.spawns[a.spawn];
                    s.definition == *kind && s.kind == velvet_dungeon::gameplay::SpawnKind::Enemy
                }) else {
                    continue;
                };
                let at = Vec3::new(-2.4 + k as f32 * 1.6, y, eye.z - 3.8);
                let y = app
                    .dungeon
                    .map
                    .capsule_floor(at + Vec3::Y * 0.4, 0.4)
                    .unwrap_or(y);
                let actor = &mut app.dungeon.actors[index];
                actor.position = Vec3::new(at.x, y, at.z);
                actor.yaw = 0.0;
                dungeon::combat::turn_toward(
                    &mut actor.yaw,
                    Vec3::new(eye.x - at.x, 0.0, eye.z - at.z),
                    10.0,
                    10.0,
                );
                actor.gait = dungeon::combat::Gait::Idle;
            }
            let tag = std::env::var("VELVET_FOE_POSE")
                .unwrap_or_else(|_| "Idle_A".into())
                .replace(['@', '.'], "_");
            save(&mut app, out, &format!("mina-poses-{tag}"))?;
        }
        // The Tendero in 3D by his stall, the winch over the miner's shaft beside it; his stall open.
        for (view, open) in [
            ("tendero3d", false),
            ("tienda_mina", true),
            ("tienda_mina_vender", true),
        ] {
            if !only
                .as_deref()
                .is_some_and(|o| o.split(',').any(|v| v == view))
            {
                continue;
            }
            let npc = app
                .dungeon
                .authored
                .as_ref()
                .and_then(|m| m.npcs.first().map(|n| n.position));
            if let Some(npc) = npc {
                let at = npc + Vec3::new(0.6, 0.0, 3.4);
                let y = app
                    .dungeon
                    .map
                    .capsule_floor(at + Vec3::Y * 0.5, 0.28)
                    .unwrap_or(npc.y);
                app.dungeon.controller.position = Vec3::new(at.x, y, at.z);
                let look = npc + Vec3::new(1.6, 0.0, 0.0) - app.dungeon.controller.position;
                app.dungeon.controller.yaw = (-look.x).atan2(-look.z);
                app.dungeon.controller.pitch = -0.12;
                app.player = app.dungeon.controller.position;
                if open {
                    app.dungeon.gold = 140;
                    app.dungeon.purse = 900;
                    app.dungeon.materials.insert("mineral_plata".into(), 7);
                    app.dungeon.materials.insert("gema_rubi".into(), 2);
                    app.dungeon.open_stall();
                    app.dungeon.stall.said_at -= 5.0;
                    // his counter: the shop's window, settled in
                    app.open_bocamina();
                    if view == "tienda_mina_vender" {
                        app.show_shelves(shop::Tab::Sell);
                    }
                    app.shop_opened_at -= 5.0;
                    app.shop_page_at -= 5.0;
                    if let Some((_, at)) = app.shop_said.as_mut() {
                        *at -= 5.0;
                    }
                }
                save(&mut app, out, &format!("mina-{view}"))?;
                if app.shop_open {
                    app.close_shop();
                }
                app.dungeon.close_stall();
            }
        }
        // The shaft's head-frame by the stall, from its front and from a corner.
        for (view, side) in [("castillete", 0.35f32), ("castillete_lado", -0.9)] {
            if !only
                .as_deref()
                .is_some_and(|o| o.split(',').any(|v| v == view))
            {
                continue;
            }
            let Some(shaft) = app.dungeon.shaft else {
                continue;
            };
            let yaw = app.dungeon.shaft_yaw + side;
            let at = shaft + Vec3::new(yaw.sin(), 0.0, yaw.cos()) * 8.0;
            let y = app
                .dungeon
                .map
                .capsule_floor(at + Vec3::Y * 1.0, 0.28)
                .unwrap_or(shaft.y);
            app.dungeon.controller.position = Vec3::new(at.x, y, at.z);
            let look = shaft + Vec3::Y * 2.6 - (app.dungeon.controller.position + Vec3::Y * 1.62);
            app.dungeon.controller.yaw = (-look.x).atan2(-look.z);
            app.dungeon.controller.pitch = (look.y / look.length()).asin();
            app.player = app.dungeon.controller.position;
            save(&mut app, out, &format!("mina-{view}"))?;
        }
        // Down the miner's shaft: its mouth, the bottom of a pit dug 26 m down (her pick raised at a
        // wall with ore), and the deep rock at 92 m.
        for view in [
            "pozo_boca",
            "pozo_fondo",
            "pozo_hondo",
            "pozo_sala",
            "pozo_arriba",
        ] {
            if !only
                .as_deref()
                .is_some_and(|o| o.split(',').any(|v| v == view))
            {
                continue;
            }
            app.dungeon.enter_pozo();
            let depth = match view {
                "pozo_boca" => 8,
                "pozo_fondo" | "pozo_arriba" | "pozo_sala" => 26,
                _ => 92,
            };
            if let Some(pozo) = app.dungeon.pozo.as_mut() {
                // a pit 4 blocks square down to `depth`, a stair of blocks cut down its side
                // (looking up: the whole shaft dug out, wall to wall, to show its timbers)
                let span = if view == "pozo_arriba" { 0..10 } else { 3..7 };
                let mut blocks = Vec::new();
                for k in 0..depth {
                    for x in span.clone() {
                        for z in span.clone() {
                            blocks.push([x, k, z]);
                        }
                    }
                }
                pozo.dig_out(blocks);
            }
            let (eye, yaw, pitch) = match view {
                "pozo_boca" => (Vec3::new(5.0, 0.0, -1.2), std::f32::consts::PI, -0.62),
                // the room from beside the cage, across the shaft's mouth
                "pozo_sala" => (Vec3::new(10.5, 0.0, 3.5), 0.75, 0.02),
                // the bottom of the pit, looking up its walls to the room
                "pozo_arriba" => (Vec3::new(4.5, -26.0, 4.5), 0.6, 1.05),
                "pozo_fondo" => (
                    Vec3::new(4.0, -depth as f32, 4.0),
                    std::f32::consts::PI * 0.75,
                    -0.05,
                ),
                _ => (Vec3::new(5.0, -depth as f32, 5.0), 0.4, -0.1),
            };
            app.dungeon.controller.position = eye;
            app.dungeon.controller.yaw = yaw;
            app.dungeon.controller.pitch = pitch;
            app.player = eye;
            if !matches!(view, "pozo_boca" | "pozo_sala" | "pozo_arriba") {
                app.dungeon.gear.pick = if view == "pozo_hondo" { 4 } else { 2 };
                if let Some(pozo) = app.dungeon.pozo.as_mut() {
                    pozo.swing = 0.2;
                }
                app.dungeon.mine_notes.push((
                    "+1 Hierro".into(),
                    [0.72, 0.6, 0.52],
                    app.dungeon.elapsed,
                ));
                app.dungeon.mine_notes.push((
                    "+1 Plata".into(),
                    [0.86, 0.88, 0.92],
                    app.dungeon.elapsed,
                ));
                app.dungeon.materials.insert("mineral_hierro".into(), 14);
            }
            app.dungeon
                .update(velvet_dungeon::controller::Input::default(), 1.0 / 60.0);
            if let Some(pozo) = app.dungeon.pozo.as_mut() {
                pozo.swing = 0.2;
            }
            save(&mut app, out, &format!("mina-{view}"))?;
            app.dungeon.leave_pozo();
        }
        // The mine's champions in a row before her in the loading hall, each in its own build and gear
        // (VELVET_FOE_POSE for one clip), the smith saying his first words.
        if only
            .as_deref()
            .is_some_and(|o| o.split(',').any(|v| v == "jefes"))
        {
            let eye = Vec3::new(1.0, -3.0, -44.0);
            let y = app
                .dungeon
                .map
                .capsule_floor(eye + Vec3::Y * 0.4, 0.28)
                .unwrap_or(eye.y);
            app.dungeon.controller.position = Vec3::new(eye.x, y, eye.z);
            app.dungeon.controller.yaw = 0.0;
            app.dungeon.controller.pitch = 0.05;
            app.player = app.dungeon.controller.position;
            let order = [
                "herrero",
                "bibliotecaria",
                "senor",
                "centinela",
                "custodio",
                "guardian",
            ];
            for (k, model) in order.iter().enumerate() {
                let Some(index) = app.dungeon.actors.iter().position(|a| {
                    let s = &app.dungeon.map.spawns[a.spawn];
                    dungeon::authored::champion_model(app.dungeon.authored.as_deref(), s)
                        == Some(*model)
                }) else {
                    continue;
                };
                let at = Vec3::new(
                    -6.6 + k as f32 * 2.9,
                    y,
                    eye.z - 8.5 - if k == 5 { 2.0 } else { 0.0 },
                );
                let y = app
                    .dungeon
                    .map
                    .capsule_floor(at + Vec3::Y * 0.4, 0.4)
                    .unwrap_or(y);
                let actor = &mut app.dungeon.actors[index];
                actor.position = Vec3::new(at.x, y, at.z);
                actor.yaw = 0.0;
                dungeon::combat::turn_toward(
                    &mut actor.yaw,
                    Vec3::new(eye.x - at.x, 0.0, eye.z - at.z),
                    10.0,
                    10.0,
                );
                actor.gait = dungeon::combat::Gait::Guard;
                if *model == "herrero" {
                    let s = app.dungeon.map.spawns[actor.spawn].clone();
                    let who = app.dungeon.champion_name(&s).unwrap_or_default();
                    app.dungeon.speech = Some(dungeon::Speech {
                        who,
                        line: dungeon::authored::champion_lines("herrero").unwrap().wake,
                        color: dungeon::authored::champion_color("herrero"),
                        at: app.dungeon.elapsed - 1.0,
                    });
                }
            }
            let tag = std::env::var("VELVET_FOE_POSE")
                .unwrap_or_else(|_| "guard".into())
                .replace(['@', '.'], "_");
            save(&mut app, out, &format!("mina-jefes-{tag}"))?;
        }
        // A fight in the guard room: she waits by the door, the room's foes come round the table
        // and strike — a frame every 0.4 s (their gait, their feet, their weapons, their way).
        if only
            .as_deref()
            .is_some_and(|o| o.split(',').any(|v| v == "horda"))
        {
            use velvet_dungeon::controller::Input;
            let at = Vec3::new(-0.4, -1.7, -26.1);
            let y = app
                .dungeon
                .map
                .capsule_floor(at + Vec3::Y * 0.4, 0.28)
                .unwrap_or(at.y);
            app.dungeon.controller.position = Vec3::new(at.x, y, at.z);
            let look = Vec3::new(-6.4, y, -26.1) - app.dungeon.controller.position;
            app.dungeon.controller.yaw = (-look.x).atan2(-look.z);
            app.dungeon.controller.pitch = -0.08;
            for frame in 0..9 {
                for _ in 0..24 {
                    app.dungeon.update(Input::default(), 1.0 / 60.0);
                    app.dungeon.health = 100.0;
                }
                app.player = app.dungeon.controller.position;
                save(&mut app, out, &format!("mina-horda-{frame}"))?;
            }
        }
        // What a fallen foe leaves on the floor, and a named champion with its bar.
        if only
            .as_deref()
            .is_none_or(|o| o.split(',').any(|v| v == "botin"))
        {
            use dungeon::pickups::{burst, update, Loot};
            let eye = Vec3::new(-1.5, -3.0, -48.0);
            let y = app
                .dungeon
                .map
                .capsule_floor(eye + Vec3::Y * 0.4, 0.28)
                .unwrap_or(-3.0);
            app.dungeon.controller.position = Vec3::new(eye.x, y, eye.z);
            app.dungeon.controller.yaw = 0.0;
            app.dungeon.controller.pitch = -0.35;
            app.player = app.dungeon.controller.position;
            let at = Vec3::new(eye.x, y, eye.z - 3.2);
            let loot = [
                Loot::Gold(6),
                Loot::Gold(5),
                Loot::Gold(20),
                Loot::Tonic,
                Loot::Herbs(2),
                Loot::Material("hueso_viejo", 2),
                Loot::Material("ceniza_arcana", 1),
                Loot::Material("esquirla_corazon", 1),
                Loot::Material("craneo_campeon", 1),
            ];
            let mut lying = burst(at, &loot, 7);
            for _ in 0..90 {
                update(
                    &mut lying,
                    &app.dungeon.map,
                    at + Vec3::new(0.0, 0.0, 40.0),
                    1.0 / 60.0,
                );
            }
            app.dungeon.pickups = lying;
            save(&mut app, out, "mina-botin")?;
            app.dungeon.pickups.clear();
        }
        if only
            .as_deref()
            .is_none_or(|o| o.split(',').any(|v| v == "herrero"))
        {
            let named = app.dungeon.authored.as_ref().and_then(|m| {
                m.champions
                    .iter()
                    .find(|(_, c)| c.power == dungeon::authored::Power::Sparks)
                    .map(|(id, _)| *id)
            });
            if let Some(index) =
                named.and_then(|id| app.dungeon.actors.iter().position(|a| a.spawn == id))
            {
                let home = app.dungeon.actors[index].position;
                let face = app
                    .dungeon
                    .authored
                    .as_ref()
                    .and_then(|m| m.face(app.dungeon.actors[index].spawn))
                    .unwrap_or(Vec3::Z);
                let eye = home + face * 5.5;
                let y = app
                    .dungeon
                    .map
                    .capsule_floor(eye + Vec3::Y * 0.4, 0.28)
                    .unwrap_or(home.y);
                app.dungeon.controller.position = Vec3::new(eye.x, y, eye.z);
                let d = home - eye;
                app.dungeon.controller.yaw = (-d.x).atan2(-d.z);
                app.dungeon.controller.pitch = 0.12;
                app.player = app.dungeon.controller.position;
                save(&mut app, out, "mina-herrero")?;
            }
        }
        // The chests 2.0: each kind, the treasure's open with its loot showing.
        for (tier, open) in [("comun", false), ("rico", false), ("tesoro", true)] {
            let Some(index) = app.dungeon.actors.iter().position(|a| {
                let s = &app.dungeon.map.spawns[a.spawn];
                s.kind == velvet_dungeon::gameplay::SpawnKind::Chest && s.definition == tier
            }) else {
                continue;
            };
            let chest = app.dungeon.actors[index].position;
            let face = app
                .dungeon
                .authored
                .as_ref()
                .and_then(|m| m.face(app.dungeon.actors[index].spawn))
                .unwrap_or(Vec3::Z);
            if open {
                app.dungeon.actors[index].chest_open_progress = 1.0;
            }
            let eye = chest + face * 2.1;
            let floor = app
                .dungeon
                .map
                .capsule_floor(eye + Vec3::Y * 0.3, 0.28)
                .unwrap_or(chest.y);
            app.dungeon.controller.position = Vec3::new(eye.x, floor, eye.z);
            app.dungeon.controller.yaw = (face.x).atan2(face.z);
            app.dungeon.controller.pitch = -0.32;
            app.player = app.dungeon.controller.position;
            save(&mut app, out, &format!("mina-cofre-{tier}"))?;
        }
        // The seal's stones (I and II set right, III unlit), the memory stone and the font.
        app.dungeon.sigils = [1, 2, 0];
        for (index, angle) in [(0usize, 1.0f32), (1, 2.0), (2, 0.0)] {
            app.dungeon.rune_start_angle[index] = angle * std::f32::consts::FRAC_PI_2;
            app.dungeon.rune_target_angle[index] = angle * std::f32::consts::FRAC_PI_2;
        }
        for view in ["runa_0", "runa_1", "runa_2", "memoria", "pila"] {
            if !only
                .as_deref()
                .is_some_and(|o| o.split(',').any(|v| v == view))
            {
                continue;
            }
            let found = app.dungeon.actors.iter().position(|a| {
                let s = &app.dungeon.map.spawns[a.spawn];
                match view {
                    "memoria" => s.kind == velvet_dungeon::gameplay::SpawnKind::Memory,
                    "pila" => s.kind == velvet_dungeon::gameplay::SpawnKind::Rest,
                    v => {
                        s.kind == velvet_dungeon::gameplay::SpawnKind::Seal
                            && s.definition == format!("sigil_{}", &v[5..])
                    }
                }
            });
            let Some(index) = found else {
                continue;
            };
            let at = app.dungeon.actors[index].position;
            let face = app
                .dungeon
                .authored
                .as_ref()
                .and_then(|m| m.face(app.dungeon.actors[index].spawn))
                .unwrap_or(Vec3::Z);
            let eye = at + face * 3.4;
            let floor = app
                .dungeon
                .map
                .capsule_floor(eye + Vec3::Y * 1.2, 0.28)
                .unwrap_or(at.y);
            app.dungeon.controller.position = Vec3::new(eye.x, floor, eye.z);
            app.dungeon.controller.yaw = (face.x).atan2(face.z);
            let target = if view == "pila" { 0.5 } else { 1.2 };
            let rise = at.y + target - (floor + 1.62);
            app.dungeon.controller.pitch = rise.atan2(3.4);
            app.player = app.dungeon.controller.position;
            save(&mut app, out, &format!("mina-{view}"))?;
        }
        // The chart, as if she had walked it all, from the loading hall.
        app.dungeon.visited = (0..app.dungeon.map.rooms.len()).collect();
        app.dungeon.controller.position = Vec3::new(0.0, -3.0, -50.0);
        app.dungeon.map_open = true;
        save(&mut app, out, "mina-mapa")?;
        app.dungeon.map_open = false;
        app.leave_dungeon();
        return Ok(());
    }
    if std::env::args().any(|a| a == "--mine-hud") {
        // The mine's own screen: the play chrome at the mouth (its F prompt), a notice with low
        // health and full pockets, a champion's bar, the map and the seed dialog.
        app.castle = None;
        app.nature_mode = true;
        app.village_entered = false;
        app.inventory.insert("espada_de_madera".into(), 1);
        app.enter_dungeon_unchecked();
        for _ in 0..60 {
            app.elapsed += 1.0 / 60.0;
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        save(&mut app, out, "minehud-play")?;
        // Each sword in her hand (none: the fist), slime and all, mid-swing and at rest.
        for (id, name) in [
            ("", "minehud-sword-none"),
            ("espada_de_madera", "minehud-sword-wood"),
            ("espada_pegajosa", "minehud-sword-wood-green"),
            ("espada_pegajosa_azul", "minehud-sword-wood-blue"),
            ("espada_reliquia", "minehud-sword-stone"),
            ("espada_piedra_pegajosa", "minehud-sword-stone-green"),
            ("espada_piedra_pegajosa_azul", "minehud-sword-stone-blue"),
        ] {
            app.backpack.equipment.weapon = (!id.is_empty()).then(|| id.to_owned());
            app.backpack.equipment.bare_hands = id.is_empty();
            if !id.is_empty() {
                app.inventory.insert(id.to_owned(), 1);
            }
            app.apply_equipment();
            for _ in 0..20 {
                app.elapsed += 1.0 / 60.0;
                app.tick(1.0 / 60.0);
                app.render_frame();
            }
            save(&mut app, out, name)?;
        }
        app.backpack.equipment.weapon = Some("espada_de_madera".into());
        app.backpack.equipment.bare_hands = false;
        app.apply_equipment();
        let d = &mut app.dungeon;
        d.notice = "Has encontrado un tónico".into();
        d.notice_until = d.elapsed + 60.0;
        d.health = 22.0;
        d.seal = true;
        d.potions = 2;
        d.herbs = 3;
        d.gold = 145;
        save(&mut app, out, "minehud-low")?;
        let boss = app.dungeon.map.spawns.iter().find(|s| {
            matches!(
                s.kind,
                velvet_dungeon::gameplay::SpawnKind::Miniboss
                    | velvet_dungeon::gameplay::SpawnKind::Boss
            )
        });
        if let Some(spawn) = boss.cloned() {
            let room = &app.dungeon.map.rooms[spawn.room];
            let at = room.center + (spawn.position - room.center) * 0.4;
            app.dungeon.controller.position = at;
            let to = spawn.position - at;
            app.dungeon.controller.yaw = to.x.atan2(-to.z);
            app.dungeon.notice_until = 0.0;
            save(&mut app, out, "minehud-boss")?;
        }
        app.dungeon.map_open = true;
        save(&mut app, out, "minehud-map")?;
        app.dungeon.map_open = false;
        app.dungeon.seed_input = Some("4242".into());
        save(&mut app, out, "minehud-seed")?;
        app.dungeon.seed_input = None;
        app.leave_dungeon();
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

/// Into the brothel, past Rosalía's first welcome: with a girl's id up in her room, the escort
/// over and her talk not begun (her game opens there); without one at the desk.
fn into_brothel_room(app: &mut App, id: &str) {
    app.story.values.insert("burdel_bienvenida".to_owned(), 1);
    app.story.values.insert("tutorial_castillo".to_owned(), 1);
    app.castle = None;
    app.nature_mode = true;
    app.village_entered = true;
    app.player = village::entrance();
    app.enter_brothel();
    app.queen_thought = None;
    app.talk = None;
    if let Some(inside) = app.brothel.as_mut() {
        inside.rope_open = true;
        if id.is_empty() {
            if let Some(desk) = brothel::plan()
                .hotspots
                .iter()
                .find(|h| h.id == "recepcionista")
            {
                inside.walker = brothel::Walker {
                    x: desk.x,
                    z: desk.z,
                    level: 0,
                    on_stair: false,
                };
            }
        } else {
            let room = venue::catalog()
                .company
                .iter()
                .find(|e| e.id == id)
                .map_or(1, |e| e.room);
            inside.start_escort(id, room, "—Sígame.");
        }
    }
    if !id.is_empty() {
        app.skip_escort();
    }
    if let Some(inside) = app.brothel.as_ref() {
        app.player = inside.position();
    }
    app.camera_focus = app.player;
    for _ in 0..60 {
        app.tick(1.0 / 60.0);
        app.render_frame();
    }
    if !id.is_empty() {
        app.talk = None;
        app.stage = None;
    }
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
