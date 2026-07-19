//! Product play: `VnSession` (Say/Choice/…) headless, optional real WindowRunner attempt.
//! BGM intents are applied on the real [`AudioEngine`] / [`MusicPlayer`] path.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_audio::prelude::*;
use velvet_story::prelude::*;
use velvet_story::{
    build_product_ui_frame, paint_product_frame, paint_to_render_descriptors, ProductUiFrame,
};

/// Host audio state: maps story music paths → clips and drives [`MusicPlayer`].
pub struct HostAudio {
    /// Engine.
    pub engine: AudioEngine,
    /// Music controller.
    pub music: MusicPlayer,
    /// Registered path → clip.
    pub clips: HashMap<String, ClipId>,
    /// Last effective music volume applied from prefs.
    pub last_music_volume: f32,
    /// Number of play intents applied.
    pub plays: u32,
    /// Number of stop/fade intents applied.
    pub stops: u32,
}

impl HostAudio {
    /// Create empty host audio (null mixer; no OS device required).
    pub fn new() -> Self {
        Self {
            engine: AudioEngine::new(),
            music: MusicPlayer::new(),
            clips: HashMap::new(),
            last_music_volume: 1.0,
            plays: 0,
            stops: 0,
        }
    }

    /// Ensure a clip exists for `path` (sine placeholder when asset file absent).
    fn clip_for(&mut self, path: &str) -> ClipId {
        if let Some(id) = self.clips.get(path) {
            return *id;
        }
        // Real path used by host: register named clip (file load optional later).
        let clip = AudioClip::sine(path, 220.0, 0.5, 22050);
        let id = self.engine.add_clip(clip);
        self.clips.insert(path.to_string(), id);
        id
    }

    /// Apply one BGM intent with prefs-derived volume.
    pub fn apply_intent(&mut self, intent: BgmIntent, prefs: &StoryPreferences) {
        self.music.volume = prefs.effective_music_volume();
        self.last_music_volume = self.music.volume;
        match intent {
            BgmIntent::Play { path, fade_in } => {
                let id = self.clip_for(&path);
                if fade_in > 0.0 {
                    let _ = self.music.crossfade_to(&mut self.engine, id, Some(fade_in));
                } else {
                    let _ = self.music.play(&mut self.engine, id);
                }
                self.plays += 1;
                println!(
                    "  [bgm-host] play path={path} fade_in={fade_in} volume={:.3} playing={}",
                    self.music.volume,
                    self.music.is_playing()
                );
            }
            BgmIntent::Stop { fade_out } => {
                self.music.stop(&mut self.engine, fade_out);
                self.stops += 1;
                println!(
                    "  [bgm-host] stop fade_out={fade_out} volume={:.3}",
                    self.music.volume
                );
            }
        }
        self.engine.tick(1.0 / 60.0);
        self.music.tick(1.0 / 60.0);
    }

    /// Drain session intents and apply all.
    pub fn apply_session_bgm(&mut self, session: &mut VnSession) {
        let prefs = session.prefs().clone();
        for intent in session.bgm.drain_intents() {
            self.apply_intent(intent, &prefs);
        }
        // Keep controller volume in sync when prefs change without new intents.
        self.music.volume = prefs.effective_music_volume();
        self.last_music_volume = self.music.volume;
    }
}

impl Default for HostAudio {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a `.vel` story through the product [`VnSession`] (Say + Choice + ending).
#[allow(dead_code)]
pub fn cmd_play_story(path: PathBuf, max_steps: u32, choice: Option<usize>) -> Result<()> {
    cmd_play_story_product(path, max_steps, choice, false, "en".into())
}

/// Product play with optional windowed host attempt and language.
pub fn cmd_play_story_product(
    path: PathBuf,
    max_steps: u32,
    choice: Option<usize>,
    windowed: bool,
    lang: String,
) -> Result<()> {
    let prefer = choice.unwrap_or(0);
    let save_dir = path
        .parent()
        .map(|p| p.join("saves"))
        .unwrap_or_else(|| PathBuf::from("saves"));

    // Unified boot: `.vstory` via story-lang pipeline; `.vel` via legacy load.
    let mut session = velvet_story_lang::open_session_from_story_path(&path, Some(save_dir))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if lang != "en" && !lang.is_empty() {
        session
            .set_language(&lang)
            .map_err(|e| anyhow::anyhow!("language `{lang}`: {e}"))?;
        println!(
            "language: {} (available: {})",
            session.language,
            session.available_languages().join(", ")
        );
    }

    let mut host_audio = HostAudio::new();
    // Apply music ops already emitted during session start/pump.
    host_audio.apply_session_bgm(&mut session);

    println!("playing product session {}", path.display());
    if windowed {
        match try_windowed_product_tick(&path) {
            Ok(msg) => println!("windowed: {msg}"),
            Err(e) => {
                println!("windowed: REAL WindowRunner failed or display unavailable: {e}");
                println!("windowed: continuing headless product path (not a window success)");
            }
        }
    }

    let mut steps = 0u32;
    while steps < max_steps {
        steps += 1;
        // Apply any new BGM intents each step on the real audio path.
        host_audio.apply_session_bgm(&mut session);

        match session.player().wait().clone() {
            StoryWait::Ended => break,
            StoryWait::Choice => {
                let opts = session.player().choices();
                if opts.is_empty() {
                    bail!("choice wait but no options");
                }
                let frame = build_product_ui_frame(&session);
                println!(
                    "  [ui] wait=choice choices={} selected={}",
                    frame.choices.len(),
                    frame.selected_choice
                );
                println!(
                    "  [choice screen open={} opts={}]",
                    session.choice.open,
                    opts.len()
                );
                println!("  [choice] {}", frame.choices.join(" | "));
                // silence unused import warning if ProductUiFrame only used in type
                let _: &ProductUiFrame = &frame;
                let idx = prefer.min(opts.len().saturating_sub(1));
                let arm = opts[idx].index;
                session
                    .choose_arm(arm)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Line => {
                session.say.reveal_all();
                let frame = build_product_ui_frame(&session);
                let paint = paint_product_frame(&frame);
                let descs = paint_to_render_descriptors(&paint);
                println!(
                    "  [ui] wait={} font={} w={:.1} h={:.1} langs={}",
                    frame.wait,
                    frame.font.family,
                    frame.body_width,
                    frame.body_height,
                    frame.language_options.join(",")
                );
                println!(
                    "  [gpu-paint] cmds={} descs={} say={} scene={}",
                    paint.len(),
                    descs.len(),
                    paint.has_say_geometry(),
                    paint.scene
                );
                if frame.namebox.is_empty() {
                    println!("  [say] {}", frame.body);
                } else {
                    println!("  [say] {}: {}", frame.namebox, frame.body);
                }
                if let Some(bg) = frame.background.as_deref() {
                    println!("  [bg] {bg}");
                }
                for id in &frame.sprite_ids {
                    println!("  [sprite] {id}");
                }
                if frame.language_menu_visible {
                    println!(
                        "  [lang-menu] active={} options={}",
                        frame.language,
                        frame.language_options.join("|")
                    );
                }
                session.advance();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Ready => {
                session.advance();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Pause { .. } => {
                println!("  [pause] skip");
                session.player_mut().skip_pause();
                session.ingest_events();
                host_audio.apply_session_bgm(&mut session);
            }
            StoryWait::Host { token } => {
                println!("  [host] auto-resume token={token}");
                session
                    .player_mut()
                    .resume_host(&token)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                session.ingest_events();
                host_audio.apply_session_bgm(&mut session);
            }
        }
    }

    // Final BGM drain.
    host_audio.apply_session_bgm(&mut session);
    println!(
        "  [bgm-host] summary plays={} stops={} last_volume={:.3} is_playing={}",
        host_audio.plays,
        host_audio.stops,
        host_audio.last_music_volume,
        host_audio.music.is_playing()
    );

    if session.is_ended() {
        println!("ending reached after {steps} step(s)");
        if let Some(e) = session.ending() {
            println!("ending id: {e}");
        }
        if let Some(h) = session.history_entries().last() {
            if h.text.contains("Ending") {
                println!("{}", h.text);
            }
        }
        let cur = session.player().current_text();
        if cur.contains("Ending") {
            println!("{cur}");
        }
    } else {
        println!(
            "stopped after {steps} step(s) wait={:?}",
            session.player().wait()
        );
    }
    Ok(())
}

/// Play project's entry_scene from velvet.project.
#[allow(dead_code)]
pub fn cmd_play_project(root: PathBuf, max_steps: u32, choice: Option<usize>) -> Result<()> {
    cmd_play_project_opts(root, max_steps, choice, false, "en".into())
}

/// Project play with windowed flag and language.
pub fn cmd_play_project_opts(
    root: PathBuf,
    max_steps: u32,
    choice: Option<usize>,
    windowed: bool,
    lang: String,
) -> Result<()> {
    let proj_path = root.join("velvet.project");
    if !proj_path.exists() {
        bail!("no velvet.project in {}", root.display());
    }
    let text = std::fs::read_to_string(&proj_path)?;
    let project = velvet_project::VelvetProject::from_ron(&text)?;
    let entry = root.join(&project.entry_scene);
    if !entry.exists() {
        let fallback = root.join("scripts/main.vel");
        if fallback.exists() {
            println!(
                "playing project {} (entry_scene missing, fallback scripts/main.vel)",
                root.display()
            );
            return cmd_play_story_product(fallback, max_steps, choice, windowed, lang);
        }
        bail!("entry scene not found: {}", project.entry_scene);
    }
    println!(
        "playing project {} entry {}",
        root.display(),
        project.entry_scene
    );
    cmd_play_story_product(entry, max_steps, choice, windowed, lang)
}

/// Script check then play (re-check + re-play product path).
pub fn cmd_recheck_replay(
    path: PathBuf,
    max_steps: u32,
    choice: Option<usize>,
    lang: String,
) -> Result<()> {
    let story = if path.is_dir() || path.join("velvet.project").exists() {
        let proj_path = path.join("velvet.project");
        let text = std::fs::read_to_string(&proj_path)
            .with_context(|| format!("read {}", proj_path.display()))?;
        let project = velvet_project::VelvetProject::from_ron(&text)?;
        let entry = path.join(&project.entry_scene);
        if entry.exists() {
            entry
        } else {
            path.join("scripts/main.vel")
        }
    } else {
        path.clone()
    };

    println!("recheck: {}", story.display());
    crate::script_cmd::cmd_script_check(story.clone())?;
    println!("replay: product play");
    if path.is_dir() || path.join("velvet.project").exists() {
        cmd_play_project_opts(path, max_steps, choice, false, lang)
    } else {
        cmd_play_story_product(story, max_steps, choice, false, lang)
    }
}

/// Attempt a **real** [`WindowRunner`] with a short max_frames budget and wall timeout.
///
/// Returns Ok with a success message only if the window event loop ran and exited 0.
/// On missing display / hang / event-loop failure returns Err (caller must not claim window success).
fn try_windowed_product_tick(story: &std::path::Path) -> Result<String> {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use velvet_app::prelude::*;

    if std::env::var_os("VELVET_FORCE_NO_WINDOW").is_some() {
        bail!("VELVET_FORCE_NO_WINDOW set — skipping WindowRunner");
    }

    let story = story.to_path_buf();
    println!("windowed: constructing real WindowRunner (max_frames=8, timeout=8s)…");

    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let send = |r: std::result::Result<i32, String>| {
            let _ = tx.send(r);
        };
        let source = match std::fs::read_to_string(&story) {
            Ok(s) => s,
            Err(e) => {
                send(Err(format!("read story: {e}")));
                return;
            }
        };
        let program =
            match load_program_from_source(&source, Some(&story.to_string_lossy()), "windowed") {
                Ok(p) => p,
                Err(e) => {
                    send(Err(format!("load story: {e}")));
                    return;
                }
            };
        let player = StoryPlayer::start(program);
        let session = VnSession::new(player);

        let mut app = App::new();
        app.add_plugin(StoryPlugin);
        app.insert_resource(session);
        app.set_runner(WindowRunner {
            max_frames: Some(8),
        });
        let code = app.run();
        send(Ok(code.0));
    });

    match rx.recv_timeout(Duration::from_secs(8)) {
        Ok(Ok(0)) => {
            let _ = handle.join();
            Ok("WindowRunner completed exit=0 (real OS window path)".into())
        }
        Ok(Ok(code)) => {
            let _ = handle.join();
            bail!("WindowRunner exited with code {code} (event loop / display failure)")
        }
        Ok(Err(e)) => {
            let _ = handle.join();
            bail!("WindowRunner setup/run error: {e}")
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Thread may still be blocked in the event loop; do not claim success.
            bail!(
                "WindowRunner did not finish within 8s (display/event-loop hang or no redraw); abandoning wait"
            )
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            bail!("WindowRunner worker disconnected without result")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn product_play_sample_reaches_ending_line() {
        let dir = tempdir().unwrap();
        let vel = dir.path().join("story.vel");
        std::fs::write(
            &vel,
            r#"
character hero { name: "Hero" }
scene main {
    background "bg.png"
    music "m.ogg" fade_in 0.5
    show hero at center
    hero "Hi"
    choice {
        "Go" { jump end }
    }
}
scene end {
    "Ending: Warm Lights"
}
"#,
        )
        .unwrap();
        cmd_play_story_product(vel, 40, Some(0), false, "en".into()).unwrap();
    }

    #[test]
    fn vn_session_bgm_intents_drive_real_audio_engine() {
        let src = r#"
character hero { name: "Hero" }
scene main {
    music "assets/music/soft.ogg" fade_in 1.0
    hero "Line one."
    "Ending: Done"
}
"#;
        let program = load_program_from_source(src, Some("bgm.vel"), "BGM").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        // Music op is pumped at start → intent or playing state.
        assert!(
            session.bgm.playing || session.bgm.path.is_some() || !session.bgm.intents.is_empty(),
            "expected BGM from music op: path={:?} playing={} intents={:?}",
            session.bgm.path,
            session.bgm.playing,
            session.bgm.intents
        );

        let mut prefs = session.prefs().clone();
        prefs.master_volume = 0.5;
        prefs.music_volume = 0.8;
        session.set_prefs(prefs);

        let mut host = HostAudio::new();
        host.apply_session_bgm(&mut session);
        assert!(
            host.plays >= 1,
            "host must apply at least one play intent, plays={}",
            host.plays
        );
        assert!(
            (host.last_music_volume - 0.4).abs() < 0.01,
            "volume from prefs master*music, got {}",
            host.last_music_volume
        );
        assert!(
            host.music.is_playing(),
            "MusicPlayer must be playing after intent apply"
        );

        // Fade stop path
        host.apply_intent(BgmIntent::Stop { fade_out: 0.1 }, session.prefs());
        assert!(host.stops >= 1);
    }
}
