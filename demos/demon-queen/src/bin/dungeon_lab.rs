//! Fast native entry point using the exact same run, controller, renderer and
//! HUD as the shipped game. No castle/village assets loaded for dungeon QA.
use demon_queen_dungeon as dungeon;
#[path = "../window_display.rs"]
#[allow(dead_code)]
mod window_display;
use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use velvet_dungeon::{controller::Input, Role, Shape};
use velvet_math::Vec3;
use velvet_render::*;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};
const SIZE: (u32, u32) = (1920, 1080);
fn texture(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::Texture {
    sized_texture(device, format, SIZE)
}
fn sized_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("dungeon-target"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    path: &Path,
) -> Result<()> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dungeon-readback"),
        size: (SIZE.0 * SIZE.1 * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE.0 * 4),
                rows_per_image: Some(SIZE.1),
            },
        },
        texture.size(),
    );
    queue.submit(Some(encoder.finish()));
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()??;
    let mut bytes = buffer.slice(..).get_mapped_range().to_vec();
    if matches!(
        texture.format(),
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for p in bytes.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    image::save_buffer(path, &bytes, SIZE.0, SIZE.1, image::ColorType::Rgba8)?;
    Ok(())
}
fn art_gallery(out: &Path) -> Result<()> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..Default::default()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .context("adapter")?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = texture(&device, format);
    let view = texture.create_view(&Default::default());
    let mut renderer = Wgpu3DRenderer::new_with_msaa(&device, &queue, format, SIZE, 4);
    let art = dungeon::assets::Assets::load(&mut renderer, &device, &queue);
    let chest_gold = renderer.upload_mesh(&device, &dungeon::visual::chest_gold())?;
    let floor = renderer.upload_mesh(
        &device,
        &Mesh3D::plane(14.0, Material3D::rgb(0.24, 0.29, 0.30)),
    )?;
    let healing_potion = renderer.upload_mesh(&device, &dungeon::visual::healing_potion())?;
    std::fs::create_dir_all(out)?;
    let biome = velvet_dungeon::Catalog::bundled().biomes[0].clone();
    let stone = biome.wall;
    let gallery = [
        (
            "crystal-cluster",
            dungeon::visual::crystal_cluster(stone, biome.accent),
            Vec3::new(1.5, 1.8, 1.5),
        ),
        (
            "runestone",
            dungeon::visual::runestone(stone),
            Vec3::new(1.8, 2.3, 1.3),
        ),
        (
            "rune-spiral",
            dungeon::visual::rune_spiral(),
            Vec3::new(1.6, 1.6, 1.0),
        ),
        (
            "exit-altar",
            dungeon::visual::exit_altar(stone, biome.accent),
            Vec3::new(2.2, 1.8, 1.8),
        ),
    ];
    for (name, model, scale) in gallery {
        let model = renderer.upload_mesh(&device, &model)?;
        let mut draws = vec![MeshDraw3D {
            mesh: floor,
            transform: velvet_math::Mat4::IDENTITY,
            material: Material3D::rgb(0.24, 0.29, 0.30),
            texture: None,
        }];
        draws.push(MeshDraw3D {
            mesh: model,
            transform: Transform3D {
                position: Vec3::new(0.0, 0.03, 0.0),
                scale,
                rotation: Vec3::ZERO,
            }
            .matrix(),
            material: Material3D::rgb(1.0, 1.0, 1.0),
            texture: None,
        });
        let camera = Camera3D {
            position: Vec3::new(3.1, 2.8, 5.8),
            target: Vec3::new(0.0, 1.3, 0.0),
            orthographic_height: Some(3.8),
            ..Default::default()
        };
        for (suffix, style) in [("toon", ShadingStyle3D::Toon), ("lit", ShadingStyle3D::Lit)] {
            renderer.set_shading_style(style);
            let mut environment = EnvironmentFrame3D::default();
            environment.clear_color = [0.07, 0.105, 0.13, 1.0];
            environment.lighting = Lighting3D::default();
            renderer.render_with_environment_and_lights(
                &device,
                &queue,
                &view,
                SIZE,
                camera,
                environment,
                &[],
                &draws,
            );
            capture(
                &device,
                &queue,
                &texture,
                &out.join(format!("{name}-{suffix}.png")),
            )?;
        }
    }

    // Keep a two-pose preview beside the dungeon kit gallery so hinged props
    // can be checked in their real renderer without adding placeholder meshes.
    for (pose, openness) in [("closed", 0.0), ("open", 1.0)] {
        let chest_position = Vec3::new(0.0, 0.03, 0.0);
        let preview_scale = 1.15 / 0.62;
        let mut draws = vec![MeshDraw3D {
            mesh: floor,
            transform: velvet_math::Mat4::IDENTITY,
            material: Material3D::rgb(0.24, 0.29, 0.30),
            texture: None,
        }];
        art.draw_hinged(
            "chest",
            &mut draws,
            chest_position,
            1.15,
            0.0,
            3.0,
            openness,
        )
        .context("chest hinge")?;
        if openness > 0.0 {
            draws.push(MeshDraw3D {
                mesh: chest_gold,
                transform: Transform3D {
                    position: chest_position + Vec3::new(-0.16, 0.195, -0.14) * preview_scale,
                    scale: Vec3::new(0.38, 0.46, 0.38) * preview_scale,
                    rotation: Vec3::ZERO,
                }
                .matrix(),
                material: Material3D::rgb(1.0, 0.96, 0.84),
                texture: None,
            });
            draws.push(MeshDraw3D {
                mesh: healing_potion,
                transform: Transform3D {
                    position: chest_position + Vec3::new(0.0, 0.19, -0.14) * preview_scale,
                    scale: Vec3::new(0.17, 0.18, 0.17) * preview_scale,
                    rotation: Vec3::ZERO,
                }
                .matrix(),
                material: Material3D::rgb(1.0, 1.0, 1.0),
                texture: None,
            });
            art.draw(
                "healing_herbs",
                &mut draws,
                chest_position + Vec3::new(0.17, 0.19, -0.14) * preview_scale,
                0.15 * preview_scale,
                -0.08,
            )
            .context("healing herbs")?;
        }
        renderer.set_shading_style(ShadingStyle3D::Toon);
        let mut environment = EnvironmentFrame3D::default();
        environment.clear_color = [0.07, 0.105, 0.13, 1.0];
        environment.lighting = Lighting3D::default();
        renderer.render_with_environment_and_lights(
            &device,
            &queue,
            &view,
            SIZE,
            Camera3D {
                position: Vec3::new(2.0, 1.55, 3.6),
                target: Vec3::new(0.0, 0.53, 0.0),
                orthographic_height: Some(2.3),
                ..Default::default()
            },
            environment,
            &[],
            &draws,
        );
        capture(
            &device,
            &queue,
            &texture,
            &out.join(format!("chest-{pose}-toon.png")),
        )?;
    }
    Ok(())
}

/// Screenshots of the expedition HUD and the 3D map only (fast art checks).
fn hud_preview(mut run: dungeon::Dungeon, out: &Path) -> Result<()> {
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
            .context("adapter")?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = texture(&device, format);
    let view = texture.create_view(&Default::default());
    let mut renderer = Wgpu3DRenderer::new_with_msaa(&device, &queue, format, SIZE, 4);
    let mut scene = dungeon::Scene::new(&mut renderer, &device, &queue, format)?;
    std::fs::create_dir_all(out)?;
    let seed = run.seed();
    let shot = |run: &dungeon::Dungeon,
                scene: &mut dungeon::Scene,
                renderer: &mut Wgpu3DRenderer,
                name: &str|
     -> Result<()> {
        for _ in 0..2 {
            scene.render(run, renderer, &device, &queue, &view, SIZE, run.camera());
            device.poll(wgpu::Maintain::Wait);
        }
        capture(
            &device,
            &queue,
            &texture,
            &out.join(format!("seed-{seed}-{name}.png")),
        )
    };
    // Play chrome in the entrance, hurt and holding a few things.
    run.elapsed = 10.0;
    run.health = 62.0;
    run.gold = 340;
    run.potions = 2;
    run.herbs = 3;
    run.notice = "Has encontrado un relicario sellado.".into();
    run.notice_until = 99.0;
    shot(&run, &mut scene, &mut renderer, "hud-entrance")?;
    // A champion's room.
    if let Some(room) = run
        .map
        .rooms
        .iter()
        .find(|r| r.role == Role::Miniboss)
        .or_else(|| run.map.rooms.iter().find(|r| r.role == Role::Boss))
    {
        let s = &room.sockets[0];
        run.controller.position = s.position - s.outward * 1.5;
        run.visited.insert(room.id);
        run.notice_until = 0.0;
        shot(&run, &mut scene, &mut renderer, "hud-boss")?;
        let id = room.id;
        let spawns = &run.map.spawns;
        if let Some(actor) = run
            .actors
            .iter_mut()
            .find(|a| spawns[a.spawn].room == id && spawns[a.spawn].health > 100.0)
        {
            actor.health *= 0.45;
            actor.enraged = true;
        }
        shot(&run, &mut scene, &mut renderer, "hud-boss-enraged")?;
    }
    // The map, charted through the first two thirds of the complex.
    for id in 0..run.map.rooms.len() * 2 / 3 {
        run.visited.insert(id);
    }
    run.map_open = true;
    for (name, yaw, pitch, zoom) in [
        ("map-a", 0.62, 0.92, 1.0),
        ("map-b", 2.4, 0.55, 1.0),
        ("map-c", 4.0, 1.2, 1.0),
        ("map-top", 0.0, 1.52, 1.0),
        ("map-zoom", 0.9, 0.7, 2.4),
        ("map-below", 0.9, -0.6, 1.0),
        ("map-below2", 3.5, -1.15, 1.0),
    ] {
        run.map_yaw = yaw;
        run.map_pitch = pitch;
        run.map_zoom = zoom;
        shot(&run, &mut scene, &mut renderer, name)?;
    }
    run.reset_map_view();
    run.map_open = false;
    run.seed_input = Some("8675309".into());
    shot(&run, &mut scene, &mut renderer, "seed")?;
    run.seed_input = None;
    run.debug = true;
    shot(&run, &mut scene, &mut renderer, "atlas")?;
    println!("HUD previews at {}", out.display());
    Ok(())
}

fn headless(mut run: dungeon::Dungeon, out: &Path, sweep: bool, rune_preview: bool) -> Result<()> {
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
            .context("adapter")?;
    eprintln!("Dungeon GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = texture(&device, format);
    let view = texture.create_view(&Default::default());
    let mut renderer = Wgpu3DRenderer::new_with_msaa(&device, &queue, format, SIZE, 4);
    renderer.frustum_culling = std::env::var_os("VELVET_NO_CULL").is_none();
    let mut scene = dungeon::Scene::new(&mut renderer, &device, &queue, format)?;
    let mut ids = vec![0];
    for role in [
        Role::Puzzle,
        Role::Event,
        Role::Rest,
        Role::Treasure,
        Role::Secret,
        Role::Encounter,
        Role::Elite,
        Role::Miniboss,
        Role::Boss,
        Role::Exit,
    ] {
        if let Some(r) = run.map.rooms.iter().find(|r| r.role == role) {
            ids.push(r.id);
        }
    }
    if let Some(r) = run
        .map
        .rooms
        .iter()
        .find(|r| r.biome == 1 && r.shape == Shape::Cavern)
    {
        ids.push(r.id);
    }
    for gallery in [
        dungeon::themes::Theme::Crypt,
        dungeon::themes::Theme::Mine,
        dungeon::themes::Theme::Fungal,
        dungeon::themes::Theme::Forge,
        dungeon::themes::Theme::Library,
        dungeon::themes::Theme::Overgrowth,
    ] {
        if let Some(r) = run
            .map
            .rooms
            .iter()
            .find(|r| dungeon::themes::of(&run.map, r) == Some(gallery))
        {
            if !ids.contains(&r.id) {
                ids.push(r.id);
            }
        }
    }
    if sweep {
        ids = (0..run.map.rooms.len()).collect();
    }
    // `--rooms 1,20` captures just those rooms (quick art checks).
    if let Some(list) = std::env::args().skip_while(|a| a != "--rooms").nth(1) {
        ids = list
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .filter(|&id: &usize| id < run.map.rooms.len())
            .collect();
    }
    std::fs::create_dir_all(out)?;
    let mut metrics = Vec::new();
    for id in ids {
        let r = &run.map.rooms[id];
        let s = &r.sockets[0];
        run.controller.position = s.position - s.outward * 1.5;
        let d = r.center + Vec3::Y * 1.25 - run.controller.eye();
        run.controller.yaw = (-d.x).atan2(-d.z);
        run.controller.pitch = (d.y / d.length()).asin();
        if id == 0 {
            run.controller.position = run.map.spawn();
            run.controller.yaw = run.spawn_yaw();
            run.controller.pitch = 0.0;
        }
        run.elapsed = 10.0;
        run.visited.insert(id);
        let cold_start = Instant::now();
        scene.render(
            &run,
            &mut renderer,
            &device,
            &queue,
            &view,
            SIZE,
            run.camera(),
        );
        device.poll(wgpu::Maintain::Wait);
        let cold_ms = cold_start.elapsed().as_secs_f64() * 1000.0;
        let cold_stats = scene.stats;
        for _ in 0..3 {
            scene.render(
                &run,
                &mut renderer,
                &device,
                &queue,
                &view,
                SIZE,
                run.camera(),
            );
            device.poll(wgpu::Maintain::Wait);
        }
        let mut times = Vec::new();
        for _ in 0..if sweep { 3 } else { 30 } {
            let start = Instant::now();
            scene.render(
                &run,
                &mut renderer,
                &device,
                &queue,
                &view,
                SIZE,
                run.camera(),
            );
            device.poll(wgpu::Maintain::Wait);
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            anyhow::ensure!(
                scene.stats.dropped_chunks == 0,
                "dropped visible geometry in room {id}"
            );
        }
        times.sort_by(f64::total_cmp);
        if !sweep || id == 0 || id == run.map.rooms.len() - 1 {
            capture(
                &device,
                &queue,
                &texture,
                &out.join(format!("seed-{}-room-{id}.png", run.seed())),
            )?;
        }
        if !sweep {
            if let Some(water) = dungeon::water::for_room(&run.map, r) {
                let saved = run.controller.clone();
                run.controller.position = r.center;
                let look = water.center + Vec3::Y * 0.08;
                let d = look - run.controller.eye();
                run.controller.yaw = (-d.x).atan2(-d.z);
                run.controller.pitch = (d.y / d.length()).asin();
                for _ in 0..3 {
                    scene.render(
                        &run,
                        &mut renderer,
                        &device,
                        &queue,
                        &view,
                        SIZE,
                        run.camera(),
                    );
                    device.poll(wgpu::Maintain::Wait);
                }
                capture(
                    &device,
                    &queue,
                    &texture,
                    &out.join(format!("seed-{}-water-room-{id}.png", run.seed())),
                )?;
                run.controller = saved;
            }
        }
        if id == run.map.entrance && !sweep {
            let mut preview = dungeon::Dungeon::try_generate(run.seed(), run.map.config.clone())
                .map_err(anyhow::Error::msg)?;
            if let Some(slime) = &preview.slime {
                let toward =
                    velvet_dungeon::horizontal(r.center - slime.position).normalize_or_zero();
                let at = slime.position + toward * 1.55;
                preview.controller.position = Vec3::new(at.x, r.floor_y(at.x, at.z), at.z);
                let mut captured = std::collections::BTreeSet::new();
                for _ in 0..260 {
                    preview.update(velvet_dungeon::controller::Input::default(), 0.02);
                    let Some(slime) = &preview.slime else { break };
                    let phase = match slime.mode {
                        dungeon::slime::Mode::Telegraph if slime.mode_time > 0.36 => Some("windup"),
                        dungeon::slime::Mode::Pounce if slime.mode_time > 0.18 => Some("pounce"),
                        dungeon::slime::Mode::Recover if slime.mode_time > 0.28 => Some("recovery"),
                        _ => None,
                    };
                    if let Some(name) = phase {
                        if captured.insert(name) {
                            let look = slime.position + Vec3::Y * 0.42;
                            let d = look - preview.controller.eye();
                            preview.controller.yaw = (-d.x).atan2(-d.z);
                            preview.controller.pitch = (d.y / d.length()).asin();
                            scene.render(
                                &preview,
                                &mut renderer,
                                &device,
                                &queue,
                                &view,
                                SIZE,
                                preview.camera(),
                            );
                            device.poll(wgpu::Maintain::Wait);
                            capture(
                                &device,
                                &queue,
                                &texture,
                                &out.join(format!("seed-{}-slime-{name}.png", run.seed())),
                            )?;
                        }
                    }
                    if captured.len() == 3 {
                        break;
                    }
                }
            }
        }
        // Signature rooms: also look back at the door from the chest, and
        // across the chamber from a side, to review the whole composition.
        if !sweep
            && matches!(r.role, Role::Treasure | Role::Secret | Role::Rest)
            && r.shape != Shape::Bridge
        {
            let saved = run.controller.clone();
            let entry = r.sockets[0].position;
            let inward = -velvet_dungeon::horizontal(r.sockets[0].outward).normalize_or_zero();
            let right = Vec3::new(inward.z, 0.0, -inward.x);
            let depth = inward.x.abs() * r.half[0] + inward.z.abs() * r.half[1];
            let width = right.x.abs() * r.half[0] + right.z.abs() * r.half[1];
            let chest = run
                .map
                .spawns
                .iter()
                .find(|s| s.room == id && s.kind == velvet_dungeon::gameplay::SpawnKind::Chest)
                .map(|s| s.position)
                .unwrap_or(r.center);
            let views = [
                (
                    "back",
                    chest + velvet_dungeon::horizontal(entry - chest).normalize_or_zero() * 1.3,
                    entry + Vec3::Y * 1.9,
                ),
                (
                    "side",
                    r.center - right * (width * 0.62) - inward * (depth * 0.25),
                    r.center + inward * (depth * 0.3) + Vec3::Y * 1.2,
                ),
            ];
            for (name, position, look) in views {
                run.controller.position = Vec3::new(position.x, r.center.y, position.z);
                let d = look - run.controller.eye();
                run.controller.yaw = (-d.x).atan2(-d.z);
                run.controller.pitch = (d.y / d.length()).asin();
                for _ in 0..3 {
                    scene.render(
                        &run,
                        &mut renderer,
                        &device,
                        &queue,
                        &view,
                        SIZE,
                        run.camera(),
                    );
                    device.poll(wgpu::Maintain::Wait);
                }
                capture(
                    &device,
                    &queue,
                    &texture,
                    &out.join(format!("seed-{}-room-{id}-{name}.png", run.seed())),
                )?;
            }
            run.controller = saved;
        }
        metrics.push(serde_json::json!({"room":id,"visible_rooms":scene.stats.visible_rooms,"resident_chunks":scene.stats.resident_chunks,"draws":scene.stats.draws,"static_and_imported_triangles":scene.stats.triangles,"first_view_render_wait_ms":cold_ms,"first_view_generation_ms":cold_stats.generation_ms,"first_view_upload_ms":cold_stats.upload_ms,"first_view_new_chunks":cold_stats.uploads,"warm_render_wait_median_ms":times[times.len()/2],"warm_render_wait_p95_ms":times[times.len()*95/100],"measurement":"CPU submission + synchronous GPU completion, 1920x1080 MSAA4; warm excludes screenshot and asset upload, not gameplay FPS; first_view includes newly visible chunk generation/upload"}));
    }
    if !sweep {
        let mut poses = Vec::new();
        // Oblique art review reveals wall/floor relief and the relationship of
        // authored arches to the procedural room, not just a centred hallway.
        let r = &run.map.rooms[0];
        let socket = &r.sockets[0];
        let side = Vec3::new(-socket.outward.z, 0.0, socket.outward.x);
        let position = r.center
            - socket.outward * (r.half[0].min(r.half[1]) * 0.50)
            - side * (r.half[0].min(r.half[1]) * 0.50);
        if r.contains(position.x, position.z, 0.6) {
            poses.push((
                "architecture",
                position,
                socket.position + Vec3::Y * 1.65 + side * 2.8,
                true,
            ));
        }
        if let Some(r) = run.map.rooms.iter().find(|r| r.shape == Shape::Cavern) {
            for i in 0..r.outline.len() {
                let a = r.center + Vec3::new(r.outline[i][0], 0.0, r.outline[i][1]);
                let b = r.outline[(i + 1) % r.outline.len()];
                let b = r.center + Vec3::new(b[0], 0.0, b[1]);
                let mid = (a + b) * 0.5;
                if r.sockets.iter().any(|s| (s.position - mid).length() < 3.0) {
                    continue;
                }
                let edge = (b - a).normalize_or_zero();
                let n = Vec3::new(-edge.z, 0.0, edge.x);
                poses.push(("wall-close", mid + n * 2.4, mid + Vec3::Y * 2.0, true));
                poses.push(("wall-torch-off", mid + n * 2.4, mid + Vec3::Y * 2.0, false));
                break;
            }
            poses.push((
                "ceiling",
                r.center,
                r.center + Vec3::new(1.0, r.height, 1.0),
                true,
            ));
        }
        if let Some(c) = run
            .map
            .connections
            .iter()
            .find(|c| (c.end.y - c.start.y).abs() > 1.0)
        {
            let mut p = c.start.lerp(c.end, 0.32);
            p.y = c.floor_y(0.32);
            poses.push(("stairs", p, c.start.lerp(c.end, 0.72) + Vec3::Y * 1.6, true));
        }
        for (name, position, target, torch) in poses {
            run.controller.position = position;
            let d = target - run.controller.eye();
            run.controller.yaw = (-d.x).atan2(-d.z);
            run.controller.pitch = (d.y / d.length()).asin();
            run.torch = torch;
            for _ in 0..4 {
                scene.render(
                    &run,
                    &mut renderer,
                    &device,
                    &queue,
                    &view,
                    SIZE,
                    run.camera(),
                );
                device.poll(wgpu::Maintain::Wait);
            }
            anyhow::ensure!(
                scene.stats.dropped_chunks == 0,
                "dropped chunks at surface review {name}"
            );
            capture(
                &device,
                &queue,
                &texture,
                &out.join(format!("seed-{}-{name}.png", run.seed())),
            )?;
        }
        run.torch = true;

        // Capture representative sword poses from the same viewmodel path
        // used by the game, making wind-up, impact and recovery easy to tune.
        run.controller.position = run.map.spawn();
        run.controller.yaw = run.spawn_yaw();
        run.controller.pitch = 0.0;
        run.elapsed = 10.0;
        for (pose, combo, remaining, hit_flash) in [
            (
                "sword-windup",
                0,
                dungeon::fp::SWINGS[0].duration * 0.94,
                0.0,
            ),
            (
                "sword-impact",
                0,
                dungeon::fp::SWINGS[0].duration - dungeon::fp::SWINGS[0].strike,
                0.16,
            ),
            ("sword-followthrough", 0, 0.13, 0.0),
            (
                "sword-backhand",
                1,
                dungeon::fp::SWINGS[1].duration - dungeon::fp::SWINGS[1].strike,
                0.16,
            ),
            (
                "sword-overhead",
                2,
                dungeon::fp::SWINGS[2].duration - dungeon::fp::SWINGS[2].strike,
                0.16,
            ),
            ("sword-idle", 0, 0.0, 0.0),
        ] {
            run.sword_combo = combo;
            run.sword_from = dungeon::fp::REST;
            run.sword_swing = remaining;
            run.sword_hit_flash = hit_flash;
            scene.render(
                &run,
                &mut renderer,
                &device,
                &queue,
                &view,
                SIZE,
                run.camera(),
            );
            device.poll(wgpu::Maintain::Wait);
            capture(
                &device,
                &queue,
                &texture,
                &out.join(format!("seed-{}-{pose}.png", run.seed())),
            )?;
        }
        run.sword_swing = 0.0;
        run.sword_hit_flash = 0.0;
    }
    if rune_preview {
        use velvet_dungeon::gameplay::SpawnKind;
        let stones = run
            .map
            .spawns
            .iter()
            .filter(|s| s.kind == SpawnKind::Seal)
            .cloned()
            .collect::<Vec<_>>();
        if stones.len() == 3 {
            let room = &run.map.rooms[stones[0].room];
            let toward_entry = (room.sockets[0].position - stones[0].position).normalize_or_zero();
            let mut position = stones[0].position + toward_entry * 2.0;
            position.y = room.floor_y(position.x, position.z);
            run.controller.position = position;
            let target = stones[0].position + Vec3::Y * 0.80;
            let d = target - run.controller.eye();
            run.controller.yaw = (-d.x).atan2(-d.z);
            run.controller.pitch = (d.y / d.length()).asin();
            run.elapsed = 10.0;
            run.notice_until = 0.0;
            run.torch = true;
            for phase in 0..=3 {
                run.sigils = [phase, 0, 0];
                run.rune_changed_at[0] = if phase == 0 { -100.0 } else { 9.78 };
                run.rune_start_angle[0] = if phase == 0 {
                    0.0
                } else {
                    (phase - 1) as f32 * std::f32::consts::FRAC_PI_2
                };
                run.rune_target_angle[0] = phase as f32 * std::f32::consts::FRAC_PI_2;
                scene.render(
                    &run,
                    &mut renderer,
                    &device,
                    &queue,
                    &view,
                    SIZE,
                    run.camera(),
                );
                device.poll(wgpu::Maintain::Wait);
                capture(
                    &device,
                    &queue,
                    &texture,
                    &out.join(format!("seed-{}-rune-{phase}.png", run.seed())),
                )?;
            }
            run.sigils = [1, 2, 3];
            run.rune_changed_at = [-100.0; 3];
            run.rune_start_angle = [
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
                std::f32::consts::FRAC_PI_2 * 3.0,
            ];
            run.rune_target_angle = run.rune_start_angle;
            let position = room.sockets[0].position - room.sockets[0].outward * 1.2;
            run.controller.position = position;
            let target = room.center + Vec3::Y * 0.82;
            let d = target - run.controller.eye();
            run.controller.yaw = (-d.x).atan2(-d.z);
            run.controller.pitch = (d.y / d.length()).asin();
            scene.render(
                &run,
                &mut renderer,
                &device,
                &queue,
                &view,
                SIZE,
                run.camera(),
            );
            device.poll(wgpu::Maintain::Wait);
            capture(
                &device,
                &queue,
                &texture,
                &out.join(format!("seed-{}-runes-solved.png", run.seed())),
            )?;
        }
    }
    // Player-facing expedition map with the rooms captured above charted.
    run.debug = false;
    run.map_open = true;
    scene.render(
        &run,
        &mut renderer,
        &device,
        &queue,
        &view,
        SIZE,
        run.camera(),
    );
    capture(
        &device,
        &queue,
        &texture,
        &out.join(format!("seed-{}-map.png", run.seed())),
    )?;
    run.map_open = false;
    run.seed_input = Some("8675309".into());
    scene.render(
        &run,
        &mut renderer,
        &device,
        &queue,
        &view,
        SIZE,
        run.camera(),
    );
    capture(
        &device,
        &queue,
        &texture,
        &out.join(format!("seed-{}-seed-dialog.png", run.seed())),
    )?;
    run.seed_input = None;
    run.debug = true;
    scene.render(
        &run,
        &mut renderer,
        &device,
        &queue,
        &view,
        SIZE,
        run.camera(),
    );
    capture(
        &device,
        &queue,
        &texture,
        &out.join(format!("seed-{}-diagnostics.png", run.seed())),
    )?;
    std::fs::write(
        out.join("render-metrics.json"),
        serde_json::to_vec_pretty(&metrics)?,
    )?;
    println!(
        "Captured seed {} at {}: {} views, max {} resident chunks",
        run.seed(),
        out.display(),
        metrics.len(),
        metrics
            .iter()
            .filter_map(|m| m["resident_chunks"].as_u64())
            .max()
            .unwrap_or(0)
    );
    Ok(())
}
struct Graphics {
    display: window_display::WindowDisplay,
    renderer: Wgpu3DRenderer,
    scene: dungeon::Scene,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}
struct App {
    run: dungeon::Dungeon,
    window: Option<Arc<Window>>,
    graphics: Option<Graphics>,
    keys: std::collections::BTreeSet<KeyCode>,
    last: Instant,
    next: Instant,
    paused: bool,
    frames: u64,
    frame_sum: f64,
}
impl App {
    fn cursor(&self) {
        if let Some(w) = &self.window {
            let free = self.paused || self.run.seed_input.is_some() || self.run.map_open;
            if free {
                let _ = w.set_cursor_grab(CursorGrabMode::None);
            } else {
                let _ = w
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| w.set_cursor_grab(CursorGrabMode::Confined));
            }
            w.set_cursor_visible(free);
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<()> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title(format!(
                            "Demon Queen - Cavernas v2 - seed {}",
                            self.run.seed()
                        ))
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0)),
                )?,
            );
            let display = pollster::block_on(window_display::WindowDisplay::new(window.clone()))?;
            let gpu = &display.gpu;
            let texture = texture(&gpu.device, gpu.format);
            let view = texture.create_view(&Default::default());
            let mut renderer = Wgpu3DRenderer::new_with_msaa(
                &gpu.device,
                &gpu.queue,
                gpu.format,
                SIZE,
                gpu.msaa_samples,
            );
            let scene = dungeon::Scene::new(&mut renderer, &gpu.device, &gpu.queue, gpu.format)?;
            self.graphics = Some(Graphics {
                display,
                renderer,
                scene,
                texture,
                view,
            });
            self.window = Some(window);
            self.last = Instant::now();
            self.cursor();
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("dungeon startup: {e:#}");
            event_loop.exit();
        }
    }
    fn device_event(&mut self, _: &ActiveEventLoop, _: winit::event::DeviceId, event: DeviceEvent) {
        if !self.paused
            && self.run.seed_input.is_none()
            && !self.run.map_open
            && self.window.as_ref().is_some_and(|w| w.has_focus())
        {
            if let DeviceEvent::MouseMotion { delta } = event {
                self.run.controller.look(delta.0 as f32, delta.1 as f32);
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => {
                if let Some(g) = &mut self.graphics {
                    g.display.resize(s.width, s.height);
                }
            }
            WindowEvent::Focused(focused) => {
                self.paused = !focused;
                self.run.auto_walk = false;
                self.keys.clear();
                self.last = Instant::now();
                self.cursor();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.paused {
                    self.paused = false;
                    self.cursor();
                } else {
                    self.run.swing_sword();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if code == KeyCode::Escape
                        && pressed
                        && self.run.seed_input.is_none()
                        && !self.run.map_open
                    {
                        self.paused = !self.paused;
                        self.run.auto_walk = false;
                        self.keys.clear();
                    }
                    if self.run.seed_input.is_none() {
                        if pressed {
                            self.keys.insert(code);
                        } else {
                            self.keys.remove(&code);
                        }
                    }
                    let modal = self.run.seed_input.is_some() || self.run.map_open;
                    self.run.key(
                        code,
                        pressed,
                        event.repeat,
                        event.text.as_ref().map(|s| s.as_str()),
                    );
                    if modal != (self.run.seed_input.is_some() || self.run.map_open) {
                        self.keys.clear();
                    }
                    self.cursor();
                    if let Some(window) = &self.window {
                        window.set_title(&format!(
                            "Demon Queen - Cavernas v2 - seed {}",
                            self.run.seed()
                        ));
                    }
                    if code == KeyCode::F10 && pressed {
                        if let Some(g) = &self.graphics {
                            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                                "../../temp/dungeon-live/seed-{}-{}.png",
                                self.run.seed(),
                                self.frames
                            ));
                            if let Err(e) = capture(
                                &g.display.gpu.device,
                                &g.display.gpu.queue,
                                &g.texture,
                                &path,
                            ) {
                                eprintln!("capture: {e}");
                            } else {
                                self.run
                                    .message(format!("Captura guardada: {}", path.display()));
                            }
                        }
                    }
                    if self.run.pending_exit {
                        event_loop.exit();
                    }
                } else if event.state == ElementState::Pressed {
                    if let Some(text) = event.text {
                        self.run.text_input(&text);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.05);
                self.last = now;
                if !self.paused {
                    let key = |k| self.keys.contains(&k) as u8 as f32;
                    let input = Input {
                        forward: key(KeyCode::KeyW) - key(KeyCode::KeyS),
                        strafe: key(KeyCode::KeyD) - key(KeyCode::KeyA),
                        sprint: self.keys.contains(&KeyCode::ShiftLeft),
                        jump: std::mem::take(&mut self.run.jump_requested),
                    };
                    self.run.update(input, dt);
                }
                if let Some(g) = &mut self.graphics {
                    let start = Instant::now();
                    let gpu = &g.display.gpu;
                    g.scene.render(
                        &self.run,
                        &mut g.renderer,
                        &gpu.device,
                        &gpu.queue,
                        &g.view,
                        SIZE,
                        self.run.camera(),
                    );
                    if let Err(e) = g.display.present_texture(&g.view, 1.0) {
                        eprintln!("present: {e}");
                        event_loop.exit();
                    }
                    self.frame_sum += start.elapsed().as_secs_f64() * 1000.0;
                    self.frames += 1;
                    if self.frames % 300 == 0 {
                        eprintln!("[dungeon] seed={} pos={:?} mean_frame={:.2}ms draws={} tris={} cache={} visible={}",self.run.seed(),self.run.controller.position,self.frame_sum/300.0,g.scene.stats.draws,g.scene.stats.triangles,g.scene.stats.resident_chunks,g.scene.stats.visible_rooms);
                        self.frame_sum = 0.0;
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            self.next = now + Duration::from_millis(14);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next));
    }
}
/// Fight in the Heart's chamber for `frames` frames (the player never dies),
/// rendering every frame: summons, blood rain, orbs and shattering all run.
/// Watch the process memory from outside to catch leaks.
fn soak(mut run: dungeon::Dungeon, frames: usize) -> Result<()> {
    use velvet_dungeon::gameplay::SpawnKind;
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
            .context("adapter")?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = texture(&device, format);
    let view = texture.create_view(&Default::default());
    let mut renderer = Wgpu3DRenderer::new_with_msaa(&device, &queue, format, SIZE, 4);
    let mut scene = dungeon::Scene::new(&mut renderer, &device, &queue, format)?;
    let boss = run
        .actors
        .iter()
        .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Boss)
        .context("seed has a boss")?;
    let room = run.map.spawns[run.actors[boss].spawn].room;
    let center = run.map.rooms[room].center;
    run.visited.insert(room);
    let start = Instant::now();
    for frame in 0..frames {
        // Circle the chamber so the boss keeps attacking and summoning.
        let a = frame as f32 * 0.004;
        run.controller.position = center + Vec3::new(a.cos() * 3.5, 0.0, a.sin() * 3.5);
        run.controller.velocity = Vec3::ZERO;
        let d = run.actors[boss].position + Vec3::Y - run.controller.eye();
        run.controller.yaw = (-d.x).atan2(-d.z);
        run.health = 100.0;
        run.actors[boss].health = run.actors[boss].health.max(1.0);
        if frame % 90 == 0 {
            run.swing_sword();
        }
        run.update(velvet_dungeon::controller::Input::default(), 1.0 / 60.0);
        scene.render(
            &run,
            &mut renderer,
            &device,
            &queue,
            &view,
            SIZE,
            run.camera(),
        );
        device.poll(wgpu::Maintain::Wait);
        if frame % 600 == 0 {
            eprintln!(
                "soak frame {frame}: actors {}, spawns {}, fx {}, rain {:.2}, {:.0}s",
                run.actors.len(),
                run.map.spawns.len(),
                run.fx.len(),
                run.heart.rain,
                start.elapsed().as_secs_f32()
            );
        }
    }
    Ok(())
}

/// Play the whole run as the replay bot does, rendering every frame on the
/// fast GPU at the game's settings (1080p, no MSAA), and report where the
/// frame time goes and which frames hitch.
fn perf(mut run: dungeon::Dungeon, limit: usize) -> Result<()> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: if cfg!(windows) {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::all()
        },
        ..Default::default()
    });
    // `--low-power` runs on the integrated GPU: a stand-in for modest PCs.
    let low_power = std::env::args().any(|a| a == "--low-power");
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: if low_power {
            wgpu::PowerPreference::LowPower
        } else {
            wgpu::PowerPreference::HighPerformance
        },
        ..Default::default()
    }))
    .context("adapter")?;
    eprintln!("perf GPU: {}", adapter.get_info().name);
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    // `--scale 0.5` renders at a fraction of 1080p: separates pixel cost
    // from geometry cost.
    let scale = std::env::args()
        .skip_while(|a| a != "--scale")
        .nth(1)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0);
    let size = (
        (SIZE.0 as f32 * scale) as u32,
        (SIZE.1 as f32 * scale) as u32,
    );
    let texture = sized_texture(&device, format, size);
    let view = texture.create_view(&Default::default());
    let mut renderer = Wgpu3DRenderer::new_with_msaa(&device, &queue, format, size, 1);
    let mut scene = dungeon::Scene::new(&mut renderer, &device, &queue, format)?;
    struct Frame {
        by_phase: [usize; 9],
        drawn: usize,
        sim: f64,
        cpu: f64,
        gpu: f64,
        stats: dungeon::scene::RenderStats,
        room: usize,
    }
    let mut frames: Vec<Frame> = Vec::new();
    let mut last = Instant::now();
    let _ = dungeon::replay::complete_observed(&mut run, &mut |run| {
        if frames.len() >= limit {
            return;
        }
        let sim = last.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        scene.render(
            run,
            &mut renderer,
            &device,
            &queue,
            &view,
            size,
            run.camera(),
        );
        let cpu = start.elapsed().as_secs_f64() * 1000.0;
        device.poll(wgpu::Maintain::Wait);
        let gpu = start.elapsed().as_secs_f64() * 1000.0 - cpu;
        let mut by_phase = [0usize; 9];
        let mut start = 0;
        for (i, &end) in scene.stats.phase_draws.iter().enumerate() {
            let end = end.min(renderer.last_draw_triangles.len());
            by_phase[i] = renderer.last_draw_triangles[start.min(end)..end]
                .iter()
                .map(|&t| t as usize)
                .sum();
            start = end.max(start);
        }
        frames.push(Frame {
            by_phase,
            drawn: renderer.last_drawn_triangles,
            sim,
            cpu,
            gpu,
            stats: scene.stats,
            room: run.map.region_at(run.controller.position),
        });
        last = Instant::now();
    });
    let pct = |mut v: Vec<f64>, p: f64| {
        v.sort_by(f64::total_cmp);
        v[((v.len() - 1) as f64 * p) as usize]
    };
    let skip = 30.min(frames.len());
    let frames = frames.split_off(skip);
    let series = |f: &dyn Fn(&Frame) -> f64| frames.iter().map(f).collect::<Vec<_>>();
    for (name, v) in [
        ("sim", series(&|f| f.sim)),
        ("render cpu", series(&|f| f.cpu)),
        ("gpu wait", series(&|f| f.gpu)),
        ("total", series(&|f| f.sim + f.cpu + f.gpu)),
    ] {
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        eprintln!(
            "{name:>10}: mean {mean:6.2} ms  p50 {:6.2}  p95 {:6.2}  p99 {:6.2}  max {:6.2}",
            pct(v.clone(), 0.5),
            pct(v.clone(), 0.95),
            pct(v.clone(), 0.99),
            pct(v, 1.0)
        );
    }
    let hitches = frames
        .iter()
        .filter(|f| f.sim + f.cpu + f.gpu > 8.33)
        .count();
    eprintln!("{} frames, {hitches} over 8.33 ms (120 fps)", frames.len());
    let mut worst: Vec<&Frame> = frames.iter().collect();
    worst.sort_by(|a, b| (b.sim + b.cpu + b.gpu).total_cmp(&(a.sim + a.cpu + a.gpu)));
    for f in worst.iter().take(12) {
        eprintln!(
            "  worst: sim {:5.1} cpu {:5.1} gpu {:5.1} | room {} draws {} tris {} uploads {} gen {:.1} up {:.1} | {:?}",
            f.sim, f.cpu, f.gpu, f.room, f.stats.draws, f.stats.triangles, f.stats.uploads, f.stats.generation_ms, f.stats.upload_ms,
            f.stats.phases.map(|p| (p * 10.0).round() / 10.0)
        );
    }
    let mut line = String::from("mean phases:");
    for (i, name) in dungeon::scene::PHASES.iter().enumerate() {
        line += &format!(
            " {name} {:.2}",
            frames.iter().map(|f| f.stats.phases[i] as f64).sum::<f64>()
                / frames.len().max(1) as f64
        );
    }
    eprintln!("{line}");
    let mean_draws = frames.iter().map(|f| f.stats.draws).sum::<usize>() / frames.len().max(1);
    let mean_tris = frames.iter().map(|f| f.stats.triangles).sum::<usize>() / frames.len().max(1);
    let mean_drawn = frames.iter().map(|f| f.drawn).sum::<usize>() / frames.len().max(1);
    eprintln!(
        "mean draws {mean_draws}, mean triangles {mean_tris}, drawn after culling {mean_drawn}"
    );
    let mut line = String::from("drawn triangles by stage:");
    for (i, name) in dungeon::scene::PHASES.iter().enumerate().take(6) {
        line += &format!(
            " {name} {}",
            frames.iter().map(|f| f.by_phase[i]).sum::<usize>() / frames.len().max(1)
        );
    }
    eprintln!("{line}");
    Ok(())
}

/// Stage each kind of foe in front of the camera, in its guard and through
/// its attacks, and save the frames: a quick review of weapons and poses.
fn enemy_review(mut run: dungeon::Dungeon, out: &Path) -> Result<()> {
    use velvet_dungeon::gameplay::SpawnKind;
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
            .context("adapter")?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = texture(&device, format);
    let view = texture.create_view(&Default::default());
    let mut renderer = Wgpu3DRenderer::new_with_msaa(&device, &queue, format, SIZE, 4);
    let mut scene = dungeon::Scene::new(&mut renderer, &device, &queue, format)?;
    std::fs::create_dir_all(out)?;
    let only = std::env::args().skip_while(|a| a != "--only").nth(1);

    // One ordinary foe is re-dressed for each case; the others are cleared.
    let actor = run
        .actors
        .iter()
        .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Enemy)
        .context("seed has an enemy")?;
    for (index, a) in run.actors.iter_mut().enumerate() {
        if index != actor {
            a.health = 0.0;
        }
    }
    let spawn = run.actors[actor].spawn;
    let room = run.map.spawns[spawn].room;
    let r = &run.map.rooms[room];
    let camera_at = r.center;
    let cases = [
        ("minion-blade", "fallen", SpawnKind::Enemy, 0usize),
        ("rogue-blade", "lurker", SpawnKind::Enemy, 1),
        ("scout-crossbow", "bone_scout", SpawnKind::Enemy, 0),
        ("guard-axe", "bone_guard", SpawnKind::Enemy, 0),
        ("mage-cast", "grave_mage", SpawnKind::Enemy, 0),
        ("boss-staff", "lurker", SpawnKind::Boss, 0),
        ("miniboss-axe", "sentinel", SpawnKind::Miniboss, 0),
    ];
    let mut elapsed = 20.0;
    for (name, definition, kind, id) in cases {
        if only.as_deref().is_some_and(|o| !name.contains(o)) {
            continue;
        }
        run.map.spawns[spawn].definition = definition.into();
        run.map.spawns[spawn].kind = kind;
        run.map.spawns[spawn].id = id;
        let moves = dungeon::combat::moves(definition, kind, id);
        let size = dungeon::combat::body_size(kind);
        for (view_name, angle) in [("side", 1.1f32), ("front", 0.25)] {
            // Foe 2.6 m ahead (more for big ones), turned to show its flank.
            let distance = 3.6 * size + 0.6;
            let mut foe = camera_at + Vec3::new(0.0, 0.0, -distance);
            foe.y = run.map.capsule_floor(foe, 0.4).unwrap_or(camera_at.y);
            run.controller.position = camera_at;
            let look = foe + Vec3::Y * (0.95 * size);
            let d = look - run.controller.eye();
            run.controller.yaw = (-d.x).atan2(-d.z);
            run.controller.pitch = (d.y / d.length()).asin();
            run.actors[actor].position = foe;
            run.actors[actor].health = run.map.spawns[spawn].health.max(1.0);
            run.actors[actor].yaw = dungeon::combat::yaw_to(camera_at - foe) + angle;
            let mut poses: Vec<(String, Option<(usize, f32)>)> = vec![("guard".into(), None)];
            for (index, mv) in moves.iter().enumerate() {
                for (label, t) in [
                    ("windup", mv.strike * 0.7),
                    ("strike", mv.strike),
                    ("follow", mv.strike + 0.12),
                ] {
                    poses.push((format!("{}-{label}", mv.name), Some((index, t))));
                }
            }
            for (pose, swing) in poses {
                run.actors[actor].gait = dungeon::combat::Gait::Guard;
                run.actors[actor].attack = swing.map(|(index, t)| dungeon::combat::Swing {
                    index,
                    t,
                    struck: false,
                    cast_point: camera_at,
                    struck_again: false,
                    origin: foe,
                });
                // Let the clip cross-fade settle before the capture.
                for _ in 0..3 {
                    elapsed += 0.5;
                    run.elapsed = elapsed;
                    scene.render(
                        &run,
                        &mut renderer,
                        &device,
                        &queue,
                        &view,
                        SIZE,
                        run.camera(),
                    );
                    device.poll(wgpu::Maintain::Wait);
                }
                capture(
                    &device,
                    &queue,
                    &texture,
                    &out.join(format!("{name}-{view_name}-{pose}.png")),
                )?;
            }
            // The fall: early, mid and sinking.
            run.actors[actor].attack = None;
            for death in [0.6f32, 1.4, 2.8] {
                run.actors[actor].health = 0.0;
                run.actors[actor].death = death;
                for _ in 0..3 {
                    elapsed += 0.5;
                    run.elapsed = elapsed;
                    scene.render(
                        &run,
                        &mut renderer,
                        &device,
                        &queue,
                        &view,
                        SIZE,
                        run.camera(),
                    );
                    device.poll(wgpu::Maintain::Wait);
                }
                capture(
                    &device,
                    &queue,
                    &texture,
                    &out.join(format!("{name}-{view_name}-death-{death:.1}.png")),
                )?;
            }
            run.actors[actor].health = run.map.spawns[spawn].health.max(1.0);
        }
        if kind == SpawnKind::Boss {
            // The Heart's blood rain over the same fight.
            run.heart.rain = 1.0;
            run.actors[actor].attack = None;
            for _ in 0..3 {
                elapsed += 0.5;
                run.elapsed = elapsed;
                scene.render(
                    &run,
                    &mut renderer,
                    &device,
                    &queue,
                    &view,
                    SIZE,
                    run.camera(),
                );
                device.poll(wgpu::Maintain::Wait);
            }
            capture(
                &device,
                &queue,
                &texture,
                &out.join(format!("{name}-blood-rain.png")),
            )?;
            run.heart.rain = 0.0;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
    };
    let seed = arg("--seed")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);
    if let Some(path) = arg("--art-gallery") {
        return art_gallery(Path::new(path));
    }
    let config = if let Some(path) = arg("--config") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        Default::default()
    };
    let run = dungeon::Dungeon::try_generate(seed, config).map_err(anyhow::Error::msg)?;
    if args.iter().any(|a| a == "--replay") {
        let count = arg("--count")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1);
        let mut reports = Vec::new();
        let mut failures = Vec::new();
        for seed in seed..seed.saturating_add(count) {
            let mut next = dungeon::Dungeon::try_generate(seed, run.map.config.clone())
                .map_err(anyhow::Error::msg)?;
            match dungeon::replay::complete(&mut next) {
                Ok(report) => reports.push(report),
                Err(e) => failures.push(e),
            };
        }
        let output = serde_json::json!({"completed":reports,"failures":failures});
        if let Some(path) = arg("--report") {
            std::fs::write(path, serde_json::to_vec_pretty(&output)?)?;
        }
        println!(
            "Replay: {} complete, {} failed",
            output["completed"].as_array().unwrap().len(),
            failures.len()
        );
        anyhow::ensure!(failures.is_empty(), "{}", failures.join("; "));
        return Ok(());
    }
    if let Some(frames) = arg("--soak").and_then(|s| s.parse::<usize>().ok()) {
        return soak(run, frames);
    }
    if let Some(frames) = arg("--perf").and_then(|s| s.parse::<usize>().ok()) {
        return perf(run, frames);
    }
    if let Some(path) = arg("--enemies") {
        return enemy_review(run, Path::new(path));
    }
    if let Some(path) = arg("--hud-preview") {
        return hud_preview(run, Path::new(path));
    }
    if let Some(path) = arg("--capture") {
        return headless(
            run,
            Path::new(path),
            args.iter().any(|a| a == "--sweep"),
            args.iter().any(|a| a == "--rune-preview"),
        );
    }
    let event_loop = EventLoop::new()?;
    let mut app = App {
        run,
        window: None,
        graphics: None,
        keys: Default::default(),
        last: Instant::now(),
        next: Instant::now(),
        paused: false,
        frames: 0,
        frame_sum: 0.0,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
