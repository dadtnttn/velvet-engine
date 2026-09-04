//! Integration: 8 mejoras novelas en VnSession.

use velvet_story::{
    load_program_from_source, AccessibilityPrefs, AutoSavePolicy, BranchMap, CharacterVoice,
    MusicTrack, RichLine, SaveThumbnail, ScreenPreset, VnSession, VoiceCast,
};
use velvet_story::{parse_rich_line, parse_show_with_motion, GalleryEntry};

fn session() -> VnSession {
    let src = r#"
scene main {
    luna "Hola."
    choice {
        "A" { jump good }
        "B" { jump bad }
    }
}
scene good {
    end "good_end"
}
scene bad {
    end "bad_end"
}
"#;
    let prog = load_program_from_source(src, Some("t.vel"), "T").unwrap();
    VnSession::new(velvet_story::StoryPlayer::start(prog))
}

#[test]
fn screens_3_lines() {
    let mut s = session();
    assert!(s.apply_screen("nvl_page"));
    assert_eq!(s.screen.max_lines, 24);
    assert!(s.apply_screen("say_classic"));
    assert!(!ScreenPreset::from_id("say_classic")
        .unwrap()
        .summary()
        .is_empty());
    assert!(!s.apply_screen("nope"));
}

#[test]
fn mini_atl_one_line() {
    let mut s = session();
    let m = s
        .show_with_motion("luna happy at left with shake 0.4 10")
        .unwrap();
    assert_eq!(m.kind, velvet_story::MotionKind::Shake);
    // reduced motion => instant, still shows sprite
    s.accessibility = AccessibilityPrefs {
        reduced_motion: true,
        ..Default::default()
    };
    let m2 = s.show_with_motion("luna at right with move 0.3").unwrap();
    assert_eq!(m2.kind, velvet_story::MotionKind::Move);
    let (_, _, none) = parse_show_with_motion("luna");
    assert!(none.is_none());
}

#[test]
fn gallery_music_unlock() {
    let mut s = session();
    s.gallery
        .register(GalleryEntry::new("cg01", "Sol", "cg/sol.png"));
    assert!(s.unlock_gallery("cg01"));
    s.music_room
        .register(MusicTrack::new("bgm01", "Noche", "music/noche.ogg"));
    assert!(s.unlock_music("bgm01"));
    assert!(s.music_room.record_play("bgm01"));
    assert_eq!(s.music_room.plays_of("bgm01"), 1);
}

#[test]
fn texto_pro_ruby_cjk() {
    let line: RichLine = parse_rich_line("Hola {b}mundo{/b} {ruby=furi}漢{/ruby} 😀");
    assert!(line.has_style());
    assert!(line.display_width() > line.visible_len());
    let secs = velvet_story::estimate_voice_secs("Hola mundo largo para voz", 20.0, 1.0);
    assert!(secs > 0.5);
}

#[test]
fn saves_foto_autosave() {
    let mut s = session();
    let rgb = vec![10u8; 192 * 108 * 3];
    let thumb = SaveThumbnail::from_rgb(192, 108, &rgb, 96);
    assert_eq!(thumb.width, 96);
    assert!(!thumb.hash.is_empty());
    // autosave slot for current scene (main)
    let slot = s.autosave_slot().unwrap();
    assert!(slot.starts_with("auto_"));
    let p = AutoSavePolicy::default();
    assert!(p.slot_for_scene("Main").is_some());
}

#[test]
fn voz_por_personaje() {
    let mut s = session();
    let mut cast = VoiceCast::new();
    cast.register(CharacterVoice::new("luna", "luna"));
    s.voice_cast = cast;
    assert_eq!(s.voice_clip_for("luna", 7).as_deref(), Some("luna_0007"));
    assert!(s.voice_estimate("luna", "Hola, ¿vienes conmigo esta noche estrellada?") > 0.5);
}

#[test]
fn mapa_ramas() {
    let s = session();
    let map = s.branch_map();
    assert_eq!(map.broken_jumps.len(), 0);
    assert_eq!(map.endings.len(), 2);
    let text = map.to_text();
    assert!(text.contains("scenes="));
    // broken case
    let bad =
        load_program_from_source("scene main {\n jump nowhere\n}\n", Some("b.vel"), "B").unwrap();
    let m = BranchMap::analyze(&bad);
    assert!(!m.broken_jumps.is_empty());
}

#[test]
fn script_driven_motion_and_screen() {
    let src = r#"
scene main {
    screen nvl_page
    show luna.happy at left with shake 0.4 10
    luna "Hola, mira esto."
    "Ending: X"
}
"#;
    let prog = load_program_from_source(src, Some("m.vel"), "M").unwrap();
    let session = VnSession::new(velvet_story::StoryPlayer::start(prog));
    assert_eq!(session.screen_id, "nvl_page");
    assert!(session.presentation.motion_for("luna").is_some());
    assert!(session.player().snapshot_motions().contains_key("luna"));
    // hide clears motion
    let src2 = "scene main {\n show luna at left with shake 0.4\n hide luna\n \"x\"\n}\n";
    let prog2 = load_program_from_source(src2, Some("h.vel"), "H").unwrap();
    let s2 = VnSession::new(velvet_story::StoryPlayer::start(prog2));
    // advance past show+hide (both run before first line)
    assert!(!s2.player().snapshot_motions().contains_key("luna"));
    assert!(s2.presentation.motion_for("luna").is_none());
}

#[test]
fn project_catalog_loaders() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("music.json"),
        r#"{"tracks":[{"id":"bgm01","title":"Noche","path":"music/noche.ogg"}]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("voice_cast.json"),
        r#"{"voices":[{"speaker":"luna","clip_prefix":"luna","volume":0.9}]}"#,
    )
    .unwrap();
    let mut s = session().with_project_root(dir.path());
    assert_eq!(s.load_music_from_project().unwrap(), 1);
    assert_eq!(s.load_voice_cast_from_project().unwrap(), 1);
    assert_eq!(s.voice_clip_for("luna", 3).as_deref(), Some("luna_0003"));
}

#[test]
fn thumbnail_sidecar_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let store = velvet_story::SaveStore::new(dir.path());
    let rgb = vec![7u8; 96 * 54 * 3];
    let thumb = velvet_story::SaveThumbnail::from_rgb(96, 54, &rgb, 96);
    let hash = store.write_thumbnail("slot_1", &thumb).unwrap();
    assert_eq!(hash, thumb.hash);
    let back = store.read_thumbnail("slot_1").unwrap().unwrap();
    assert_eq!(back.hash, thumb.hash);
    assert!(store.read_thumbnail("missing").unwrap().is_none());
}

#[test]
fn unlock_script_unlocks_and_persists_in_save() {
    let src = r#"
scene main {
    "Intro."
    unlock gallery cg_rooftop
    unlock music soft
    "Fin."
}
"#;
    let prog = load_program_from_source(src, Some("u.vel"), "U").unwrap();
    let mut session = VnSession::new(velvet_story::StoryPlayer::start(prog));
    // real flow: catalogs load right after session start, unlocks fire mid-play
    session.gallery.register(velvet_story::GalleryEntry::new(
        "cg_rooftop",
        "Rooftop",
        "assets/cg/rooftop.png",
    ));
    session.music_room.register(velvet_story::MusicTrack::new(
        "soft",
        "Soft",
        "assets/music/soft.wav",
    ));
    // dismiss intro line; unlocks execute on the way to "Fin."
    session.say.reveal_all();
    session.advance();
    session.say.reveal_all();
    session.advance();
    assert!(session.gallery.is_unlocked("cg_rooftop"));
    assert!(session.music_room.is_unlocked("soft"));
    // save persists unlocks; fresh session restores them on load
    let dir = tempfile::tempdir().unwrap();
    let session = session.with_save_dir(dir.path());
    let save = session.save_slot("slot_1").unwrap();
    assert!(save.gallery_unlocked.contains("cg_rooftop"));
    assert!(save.music_unlocked.contains("soft"));
    let prog2 = load_program_from_source(src, Some("u.vel"), "U").unwrap();
    let mut fresh = VnSession::new(velvet_story::StoryPlayer::start(prog2));
    fresh.gallery.register(velvet_story::GalleryEntry::new(
        "cg_rooftop",
        "Rooftop",
        "assets/cg/rooftop.png",
    ));
    fresh.music_room.register(velvet_story::MusicTrack::new(
        "soft",
        "Soft",
        "assets/music/soft.wav",
    ));
    let fresh = fresh.with_save_dir(dir.path());
    let mut fresh = fresh;
    fresh.load_slot("slot_1").unwrap();
    assert!(fresh.gallery.is_unlocked("cg_rooftop"));
    assert!(fresh.music_room.is_unlocked("soft"));
}

fn movie_project() -> (tempfile::TempDir, VnSession) {
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("assets/video/opening");
    std::fs::create_dir_all(&clip).unwrap();
    for f in [
        "frame_0001.png",
        "frame_0002.png",
        "frame_0003.png",
        "frame_0004.png",
    ] {
        std::fs::write(clip.join(f), b"x").unwrap();
    }
    std::fs::write(clip.join("clip.json"), r#"{"fps": 4.0}"#).unwrap();
    let src = "scene main {\n movie \"assets/video/opening\"\n hero \"After.\"\n}\n";
    let prog = load_program_from_source(src, Some("m.vel"), "M").unwrap();
    let session = VnSession::new_with_root(
        velvet_story::StoryPlayer::start(prog),
        Some(dir.path().to_path_buf()),
    );
    (dir, session)
}

#[test]
fn movie_plays_blocks_auto_and_click_skips() {
    let (_dir, mut session) = movie_project();
    // movie op runs during start pump; ingest starts playback
    assert!(session.video.is_playing());
    assert_eq!(
        session.video.played(),
        &["assets/video/opening".to_string()]
    );
    let frame = velvet_story::build_product_ui_frame(&session);
    assert!(frame.video_playing);
    assert!((frame.video_progress - 0.0).abs() < 1e-5);
    session.tick(0.5);
    assert!(session.video.is_playing());
    assert!(velvet_story::build_product_ui_frame(&session).video_progress > 0.0);
    // auto-advance is gated while the clip plays
    session.set_auto(true);
    session.tick(0.25);
    assert!(matches!(
        session.player().wait(),
        velvet_story::StoryWait::Line
    ));
    // first click skips the cinema instead of advancing dialogue
    session.advance();
    assert!(!session.video.is_playing());
    assert!(matches!(
        session.player().wait(),
        velvet_story::StoryWait::Line
    ));
    // missing clip records a note and never blocks
    let prog = load_program_from_source(
        "scene main {\n movie \"assets/video/nope\"\n \"x\"\n}\n",
        Some("m.vel"),
        "M",
    )
    .unwrap();
    let mut missing = VnSession::new(velvet_story::StoryPlayer::start(prog));
    assert!(!missing.video.is_playing());
    assert!(missing.video.note.as_deref().unwrap().contains("nope"));
    missing.advance();
    assert!(matches!(
        missing.player().wait(),
        velvet_story::StoryWait::Line
    ));
}

#[test]
fn accesibilidad() {
    let mut s = session();
    s.accessibility = AccessibilityPrefs {
        font_scale: 1.5,
        high_contrast: true,
        reduced_motion: true,
        ..Default::default()
    }
    .validated();
    assert!(!s.accessibility.should_animate());
    assert_eq!(s.accessibility.font_family(), "default");
    assert_eq!(s.accessibility.transition_secs(0.4), 0.0);
    assert!(s.accessibility.text_px() > 18.0);
}
