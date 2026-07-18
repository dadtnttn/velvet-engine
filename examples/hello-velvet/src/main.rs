//! Hello Velvet — exercises core plugins: time, input, assets, audio, render batch (headless path).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use velvet_app::prelude::*;
use velvet_assets::prelude::*;
use velvet_audio::prelude::*;
use velvet_input::prelude::*;
use velvet_math::{Color, Transform2D, Vec2};
use velvet_render::prelude::*;

fn main() {
    velvet_core::init_tracing_default("hello_velvet=info,velvet=info,info");

    let frames = Arc::new(AtomicUsize::new(0));
    let frames_c = Arc::clone(&frames);

    let mut config = EngineConfig {
        name: "Hello Velvet".into(),
        ..Default::default()
    };
    config.window.title = "Hello Velvet".into();

    let mut app = App::with_config(config);
    app.add_plugin(InputPlugin);
    app.add_plugin(AssetsPlugin {
        memory: true,
        ..Default::default()
    });
    app.add_plugin(AudioPlugin);
    app.add_plugin(RenderPlugin {
        config: RenderConfig {
            profile: RenderProfile::VisualNovel,
            virtual_width: 1280.0,
            virtual_height: 720.0,
            clear: ClearColor::rgba(0.12, 0.08, 0.18, 1.0),
        },
    });

    app.add_system(ScheduleLabel::Startup, |app| {
        tracing::info!("Hello from Velvet Engine!");
        // Seed assets
        if let Some(assets) = app.resource_mut::<Assets>() {
            assets.insert(
                "hello/message.txt",
                TextAsset {
                    text: "Welcome to Velvet.".into(),
                },
            );
        }
        // Seed audio
        if let Some(audio) = app.resource_mut::<AudioEngine>() {
            let clip = audio.add_clip(AudioClip::sine("hello-tone", 440.0, 0.15, 22050));
            let _ = audio.play(
                clip,
                PlayParams {
                    bus: BusId::from_kind(BusKind::Ui),
                    volume: 0.5,
                    ..Default::default()
                },
            );
        }
        // Simulate confirm action once via raw key path
        if let Some(input) = app.resource_mut::<InputState>() {
            input.begin_frame();
            input.key_down(KeyCode::Enter);
            input.end_frame();
            if input.just_pressed(builtin::CONFIRM) {
                tracing::info!("confirm action ready");
            }
        }
    });

    app.add_system(ScheduleLabel::Update, move |app| {
        let n = frames_c.fetch_add(1, Ordering::SeqCst) + 1;

        // Queue a sprite into the render frame batch (CPU path; GPU present is optional).
        if let Some(frame) = app.resource_mut::<RenderFrame>() {
            let tex = TextureId::allocate(); // demo id; GPU would map white
            frame.batch.push_colored_quad(
                tex,
                Transform2D::from_translation(Vec2::new((n as f32 * 2.0).sin() * 40.0, 0.0)),
                Vec2::new(64.0, 64.0),
                Color::VELVET,
                0.0,
            );
            frame.stats.sprites_submitted = frame.batch.len() as u32;
        }

        if n == 1 {
            if let Some(assets) = app.resource::<Assets>() {
                // Demonstrate handle load from inserted asset path via get after insert already done.
                let _ = assets;
            }
            if let Some(t) = app.resource::<Time>() {
                tracing::info!(elapsed = t.elapsed_secs(), "first update tick");
            }
        }

        if n >= 30 {
            app.request_exit();
        }
    });

    app.set_runner(HeadlessRunner {
        max_frames: Some(120),
        delta_secs: 1.0 / 60.0,
    });

    let code = app.run();
    println!(
        "Hello Velvet finished: {} update(s), exit {}",
        frames.load(Ordering::SeqCst),
        code.0
    );
}
