//! Cross-crate: render culling + audio engine + input resolution + typewriter UI text.

use velvet_audio::{AudioClip, AudioEngine, BusId, BusKind, PlayParams};
use velvet_input::{builtin, InputState, KeyCode};
use velvet_math::{Aabb2, Color, Rect, Vec2};
use velvet_render::{count_visible, cull_aabbs, Camera2D, CameraFrustum2D, CullResult};
use velvet_text::{Typewriter, TypewriterEvent};

// ---------------------------------------------------------------------------
// Render culling multi-object flow
// ---------------------------------------------------------------------------

#[test]
fn camera_frustum_culls_batch() {
    let cam = Camera2D::default();
    let frustum = CameraFrustum2D::from_camera_padded(&cam, 100.0);

    let near = Aabb2::from_center_extents(Vec2::new(10.0, 10.0), Vec2::splat(5.0));
    let far = Aabb2::from_center_extents(Vec2::new(50_000.0, 50_000.0), Vec2::splat(5.0));
    let edge = Aabb2::from_center_extents(frustum.bounds.center(), Vec2::splat(1.0));

    assert!(frustum.cull_aabb(near).is_visible());
    assert!(frustum.cull_point(Vec2::ZERO).is_visible());
    assert_eq!(frustum.cull_aabb(far), CullResult::Outside);
    assert!(frustum.cull_aabb(edge).is_visible());

    let boxes = [near, far, edge];
    let visible_idx = cull_aabbs(&frustum, &boxes);
    assert!(!visible_idx.is_empty());
    assert!(!visible_idx.contains(&1), "far box should be culled");
    let n = count_visible(&frustum, &boxes);
    assert_eq!(n, visible_idx.len());
    assert!(n < boxes.len());
}

#[test]
fn oriented_and_circle_cull() {
    let frustum = CameraFrustum2D::from_rect(Rect::from_pos_size(
        Vec2::new(-50.0, -50.0),
        Vec2::new(100.0, 100.0),
    ));
    assert_eq!(frustum.cull_circle(Vec2::ZERO, 10.0), CullResult::Inside);
    assert_eq!(
        frustum.cull_circle(Vec2::new(1000.0, 0.0), 5.0),
        CullResult::Outside
    );
    assert_eq!(frustum.cull_point(Vec2::new(0.0, 0.0)), CullResult::Inside);
    assert_eq!(
        frustum.cull_point(Vec2::new(500.0, 0.0)),
        CullResult::Outside
    );
}

// ---------------------------------------------------------------------------
// Audio engine: buses, play, tick, mute
// ---------------------------------------------------------------------------

#[test]
fn audio_play_tick_and_bus_gain() {
    let mut eng = AudioEngine::new();
    let clip = AudioClip::silent("beep", 1.0, 48_000);
    let id = eng.add_clip(clip);
    eng.set_bus_volume(BusKind::Effects, 0.5);
    let gain = eng.effective_gain(&BusId::from_kind(BusKind::Effects));
    assert!(gain > 0.0 && gain <= 1.0);

    let play_id = eng
        .play(
            id,
            PlayParams {
                volume: 1.0,
                fade_in: 0.1,
                ..PlayParams::default()
            },
        )
        .expect("play");
    assert!(play_id.raw() > 0);

    for _ in 0..5 {
        eng.tick(0.05);
    }

    eng.set_bus_muted(BusKind::Effects, true);
    let muted_gain = eng.effective_gain(&BusId::from_kind(BusKind::Effects));
    assert_eq!(muted_gain, 0.0);
    assert!(muted_gain < gain);
}

#[test]
fn audio_music_and_unknown_clip() {
    let mut eng = AudioEngine::new();
    let clip = AudioClip::sine("theme", 440.0, 0.5, 48_000);
    let id = eng.add_clip(clip);
    let pid = eng.play(id, PlayParams::music()).expect("music");
    assert!(pid.raw() > 0);

    // Unknown clip id (fresh allocate never registered).
    let missing = velvet_audio::ClipId::allocate();
    let bad = eng.play(missing, PlayParams::default());
    assert!(bad.is_err());
}

// ---------------------------------------------------------------------------
// Input: key press → action resolve
// ---------------------------------------------------------------------------

#[test]
fn input_key_resolves_move_and_confirm() {
    let mut input = InputState::with_defaults();
    input.begin_frame();
    input.key_down(KeyCode::W);
    input.key_down(KeyCode::Enter);
    input.end_frame();

    assert!(input.key_held(KeyCode::W));
    assert!(input.key_just_pressed(KeyCode::W));
    let movement = input.axis2(builtin::MOVE);
    assert!(movement.y > 0.99, "movement={movement:?}");
    assert!(movement.x.abs() < f32::EPSILON, "movement={movement:?}");
    assert!(input.pressed(builtin::CONFIRM));
    assert!(input.just_pressed(builtin::CONFIRM));

    input.begin_frame();
    input.key_up(KeyCode::W);
    input.end_frame();
    assert!(!input.key_held(KeyCode::W));
}

#[test]
fn input_frame_edges_clear() {
    let mut input = InputState::with_defaults();
    input.begin_frame();
    input.key_down(KeyCode::A);
    input.end_frame();
    assert!(input.key_just_pressed(KeyCode::A));
    assert!(input.key_held(KeyCode::A));
    input.begin_frame();
    // Pressed edge clears after begin_frame.
    assert!(!input.key_just_pressed(KeyCode::A));
    assert!(input.key_held(KeyCode::A));
    input.key_up(KeyCode::A);
    input.end_frame();
    assert!(!input.key_held(KeyCode::A));
}

// ---------------------------------------------------------------------------
// Typewriter progressive reveal (text → UI-facing events)
// ---------------------------------------------------------------------------

#[test]
fn typewriter_reveals_and_finishes() {
    let mut tw = Typewriter::new("Hello Velvet", 60.0);
    let mut chars = 0usize;
    let mut finished = false;
    for _ in 0..200 {
        let events = tw.tick(1.0 / 60.0);
        for e in events {
            match e {
                TypewriterEvent::Char(_) => chars += 1,
                TypewriterEvent::Finished => finished = true,
                TypewriterEvent::Skipped => finished = true,
                _ => {}
            }
        }
        if finished || tw.is_finished() {
            break;
        }
    }
    assert_eq!(chars, "Hello Velvet".chars().count());
    assert!(tw.is_finished());
    assert_eq!(tw.visible_text(), "Hello Velvet");
    assert!(
        !finished,
        "completion is represented by VM state after the last char"
    );
}

#[test]
fn typewriter_skip_finishes() {
    let mut tw = Typewriter::new("abcdefghij", 5.0);
    tw.skip();
    assert!(tw.is_finished());
}

// ---------------------------------------------------------------------------
// Combined: dialogue line typewriter + audio cue + input confirm
// ---------------------------------------------------------------------------

#[test]
fn dialogue_line_audio_and_input_advance() {
    let mut eng = AudioEngine::new();
    let clip = AudioClip::silent("blip", 0.1, 44_100);
    let clip_id = eng.add_clip(clip);

    let line = "Welcome, traveler.";
    let mut tw = Typewriter::new(line, 80.0);
    let mut input = InputState::with_defaults();

    let _ = eng.play(clip_id, PlayParams::ui());
    for _ in 0..10 {
        tw.tick(0.02);
        eng.tick(0.02);
    }

    input.begin_frame();
    input.key_down(KeyCode::Enter);
    input.end_frame();
    assert!(input.pressed(builtin::CONFIRM));
    assert!(input.just_pressed(builtin::CONFIRM));
    tw.skip();
    assert!(tw.is_finished());

    let frustum = CameraFrustum2D::from_rect(Rect::from_pos_size(
        Vec2::new(-200.0, -200.0),
        Vec2::splat(400.0),
    ));
    let speaker = Aabb2::from_center_extents(Vec2::new(40.0, 0.0), Vec2::new(16.0, 32.0));
    assert!(frustum.cull_aabb(speaker).is_visible());
    let _ = Color::VELVET;
}
