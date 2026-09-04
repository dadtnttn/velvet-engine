//! Session-side cinema state: one active [`VideoPlayer`], play history.
//!
//! Movies (`movie "assets/video/opening"`) start on ingest, tick in
//! [`crate::product::VnSession::tick`], block auto-advance while playing,
//! and are skipped (not story-advanced) by the first click — Ren'Py
//! `movie_cutscene` semantics. Movies are intentionally not persisted in
//! saves: loading replays from the scene cursor like any other op.

use velvet_video::VideoPlayer;

/// Active cinema playback for a product session.
#[derive(Debug, Default)]
pub struct VideoState {
    /// Currently loaded player (None = no movie yet).
    current: Option<VideoPlayer>,
    /// Clip roots played this session, in order.
    played: Vec<String>,
    /// Last load/playback note (missing clip dir, …). Cleared on play.
    pub note: Option<String>,
}

impl VideoState {
    /// Start (or restart) a clip, recording it in history.
    pub fn play(&mut self, player: VideoPlayer, root: String) {
        let mut player = player;
        player.play();
        self.current = Some(player);
        self.played.push(root);
        self.note = None;
    }

    /// Record a load failure (missing dir, bad manifest). Nothing plays.
    pub fn fail(&mut self, note: String) {
        self.current = None;
        self.note = Some(note);
    }

    /// Advance the active clip. No-op without one.
    pub fn tick(&mut self, dt: f32) {
        if let Some(p) = self.current.as_mut() {
            p.tick(dt);
        }
    }

    /// Skip the active clip (click-skips cinema). Returns true if one played.
    pub fn skip_current(&mut self) -> bool {
        match self.current.as_mut() {
            Some(p) if p.playing && !p.finished => {
                p.skip();
                true
            }
            _ => false,
        }
    }

    /// Whether a clip is currently playing (blocks auto-advance).
    pub fn is_playing(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|p| p.playing && !p.finished)
    }

    /// Current frame path for renderers.
    pub fn frame(&self) -> Option<&std::path::Path> {
        self.current.as_ref().map(|p| p.frame())
    }

    /// Current frame index.
    pub fn frame_index(&self) -> Option<usize> {
        self.current.as_ref().map(|p| p.frame_index())
    }

    /// Progress 0..=1 of the active clip.
    pub fn progress(&self) -> Option<f32> {
        self.current.as_ref().map(|p| p.progress())
    }

    /// Clips played this session.
    pub fn played(&self) -> &[String] {
        &self.played
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_video::VideoClip;

    fn player(frames: &[&str], manifest: Option<&str>) -> VideoPlayer {
        // NOTE: tempdir is deleted on return; only timing is asserted below
        // (frame paths are never re-read from disk).
        let dir = tempfile::tempdir().unwrap();
        for f in frames {
            std::fs::write(dir.path().join(f), b"x").unwrap();
        }
        if let Some(m) = manifest {
            std::fs::write(dir.path().join("clip.json"), m).unwrap();
        }
        VideoPlayer::new(VideoClip::scan_dir(dir.path()).unwrap())
    }

    #[test]
    fn play_tick_skip_flow() {
        let mut state = VideoState::default();
        assert!(!state.is_playing());
        assert!(state.frame().is_none());
        state.play(
            player(&["a.png", "b.png"], Some(r#"{"fps": 2.0}"#)),
            "op".into(),
        );
        assert!(state.is_playing());
        assert_eq!(state.frame_index(), Some(0));
        state.tick(0.5);
        assert_eq!(state.frame_index(), Some(1));
        assert!(state.skip_current());
        assert!(!state.is_playing());
        assert!(!state.skip_current());
        assert_eq!(state.played(), &["op".to_string()]);
    }

    #[test]
    fn finish_clears_playing() {
        let mut state = VideoState::default();
        state.play(player(&["a.png"], Some(r#"{"fps": 10.0}"#)), "op".into());
        state.tick(5.0);
        assert!(!state.is_playing());
        assert_eq!(state.progress(), Some(1.0));
    }

    #[test]
    fn fail_records_note() {
        let mut state = VideoState::default();
        state.fail("missing".into());
        assert!(!state.is_playing());
        assert_eq!(state.note.as_deref(), Some("missing"));
        state.play(player(&["a.png"], None), "op".into());
        assert!(state.note.is_none());
    }
}
