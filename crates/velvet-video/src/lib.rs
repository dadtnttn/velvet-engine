//! # velvet-video
//!
//! Frame-sequence cinema player for openings and cutscenes.
//!
//! A clip is a directory of numbered frames (`frame_0001.png`, …) plus an
//! optional `clip.json` manifest (`fps`, `loop`, `audio`). No codec
//! dependency: hosts decode pixels with their own pipeline (`velvet-image`
//! decodes PNG/JPEG today). Real container codecs (H.264/WebM) plug in
//! behind [`FrameSource`] without changing playback, saves, or script.
//!
//! ```text
//! assets/video/opening/
//!   clip.json            { "fps": 12, "loop": false }
//!   frame_0001.png … frame_0048.png
//! ```

#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default frames per second when the manifest omits `fps`.
pub const DEFAULT_FPS: f32 = 12.0;

/// Clip manifest (`clip.json` next to frames). All fields optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClipManifest {
    /// Frames per second.
    #[serde(default)]
    pub fps: Option<f32>,
    /// Loop instead of finishing (`loop` in JSON).
    #[serde(default, alias = "loop")]
    pub loop_clip: bool,
    /// Optional audio bed path (relative to project, host plays it).
    #[serde(default)]
    pub audio: Option<String>,
    /// Optional poster frame shown before play.
    #[serde(default)]
    pub poster: Option<String>,
}

impl ClipManifest {
    /// Effective fps.
    pub fn fps_or_default(&self) -> f32 {
        self.fps.unwrap_or(DEFAULT_FPS).clamp(1.0, 120.0)
    }
}

/// A resolved clip: ordered frame paths + timing.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoClip {
    /// Clip directory (or single-file marker root).
    pub root: PathBuf,
    /// Frame image paths in play order.
    pub frames: Vec<PathBuf>,
    /// Manifest (defaults when `clip.json` is absent).
    pub manifest: ClipManifest,
}

impl VideoClip {
    /// Scan `dir` for frames (`*.png|*.jpg|*.jpeg`, sorted) + `clip.json`.
    pub fn scan_dir(dir: &Path) -> Result<Self, VideoError> {
        if !dir.is_dir() {
            return Err(VideoError::NotFound(dir.display().to_string()));
        }
        let manifest = if dir.join("clip.json").is_file() {
            let text = std::fs::read_to_string(dir.join("clip.json"))
                .map_err(|e| VideoError::Manifest(format!("read clip.json: {e}")))?;
            serde_json::from_str(&text)
                .map_err(|e| VideoError::Manifest(format!("parse clip.json: {e}")))?
        } else {
            ClipManifest::default()
        };
        let mut frames: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| VideoError::Manifest(format!("scan {}: {e}", dir.display())))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && matches!(
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(str::to_ascii_lowercase)
                            .as_deref(),
                        Some("png") | Some("jpg") | Some("jpeg")
                    )
                    && p.file_name().and_then(|n| n.to_str()) != Some("clip.json")
            })
            .collect();
        frames.sort();
        if frames.is_empty() {
            return Err(VideoError::NoFrames(dir.display().to_string()));
        }
        Ok(Self {
            root: dir.to_path_buf(),
            frames,
            manifest,
        })
    }

    /// Frame count.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Empty (never true from `scan_dir`, useful for hosts).
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Duration seconds at manifest fps.
    pub fn duration_secs(&self) -> f32 {
        self.frames.len() as f32 / self.manifest.fps_or_default()
    }
}

/// Frame source abstraction: sequence scan today, container codecs tomorrow.
///
/// A real H.264/WebM decoder implements this trait (frame paths become
/// decoded-frame handles); [`VideoPlayer`] is unchanged.
pub trait FrameSource {
    /// Ordered frame identifiers.
    fn frames(&self) -> &[PathBuf];
    /// Frames per second.
    fn fps(&self) -> f32;
    /// Loop instead of finishing.
    fn loops(&self) -> bool;
}

impl FrameSource for VideoClip {
    fn frames(&self) -> &[PathBuf] {
        &self.frames
    }

    fn fps(&self) -> f32 {
        self.manifest.fps_or_default()
    }

    fn loops(&self) -> bool {
        self.manifest.loop_clip
    }
}

/// Playback state machine (host ticks it; renderer reads [`VideoPlayer::frame`]).
#[derive(Debug, Clone)]
pub struct VideoPlayer {
    clip: VideoClip,
    elapsed: f32,
    /// Playing (false = paused or finished).
    pub playing: bool,
    /// Finished (non-looping clip reached the end or was skipped).
    pub finished: bool,
    /// Completed loops (looping clips only).
    pub loops_done: u32,
}

impl VideoPlayer {
    /// New player, paused at frame 0. Call [`Self::play`] to start.
    pub fn new(clip: VideoClip) -> Self {
        Self {
            clip,
            elapsed: 0.0,
            playing: false,
            finished: false,
            loops_done: 0,
        }
    }

    /// Start (or restart after finish).
    pub fn play(&mut self) {
        if self.finished {
            self.elapsed = 0.0;
            self.finished = false;
        }
        self.playing = true;
    }

    /// Pause (keeps position).
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Skip to the end (finished, hosts treat as watched).
    pub fn skip(&mut self) {
        self.elapsed = self.clip.duration_secs();
        self.playing = false;
        self.finished = true;
    }

    /// Advance by `dt` seconds. Returns true on the frame it finishes.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.playing || self.finished {
            return false;
        }
        self.elapsed += dt.max(0.0);
        let dur = self.clip.duration_secs();
        if self.elapsed >= dur {
            if self.clip.manifest.loop_clip {
                self.loops_done += 1;
                self.elapsed %= dur.max(1e-4);
                return false;
            }
            self.elapsed = dur;
            self.playing = false;
            self.finished = true;
            return true;
        }
        false
    }

    /// Current frame path (last frame when finished).
    pub fn frame(&self) -> &Path {
        let n = self.clip.frames.len().max(1);
        let i = ((self.elapsed * self.clip.manifest.fps_or_default()).floor() as usize).min(n - 1);
        &self.clip.frames[i]
    }

    /// Current frame index.
    pub fn frame_index(&self) -> usize {
        let n = self.clip.frames.len().max(1);
        ((self.elapsed * self.clip.manifest.fps_or_default()).floor() as usize).min(n - 1)
    }

    /// Elapsed seconds.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Clip being played.
    pub fn clip(&self) -> &VideoClip {
        &self.clip
    }

    /// Progress 0..=1.
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.clip.duration_secs().max(1e-4)).clamp(0.0, 1.0)
    }
}

/// Video errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VideoError {
    /// Clip directory missing.
    #[error("video clip not found: {0}")]
    NotFound(String),
    /// No decodable frames inside.
    #[error("video clip has no frames: {0}")]
    NoFrames(String),
    /// Manifest unreadable.
    #[error("video manifest: {0}")]
    Manifest(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_frame(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"fake-png").unwrap();
    }

    fn clip_dir(frames: &[&str], manifest: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in frames {
            write_frame(dir.path(), f);
        }
        if let Some(m) = manifest {
            std::fs::write(dir.path().join("clip.json"), m).unwrap();
        }
        dir
    }

    #[test]
    fn scan_orders_frames_and_reads_manifest() {
        let dir = clip_dir(
            &[
                "frame_0003.png",
                "frame_0001.png",
                "frame_0002.png",
                "note.txt",
            ],
            Some(r#"{"fps": 24.0, "loop": true}"#),
        );
        let clip = VideoClip::scan_dir(dir.path()).unwrap();
        assert_eq!(clip.len(), 3);
        assert!(clip.frames[0].ends_with("frame_0001.png"));
        assert!((clip.manifest.fps_or_default() - 24.0).abs() < 1e-5);
        assert!(clip.manifest.loop_clip);
        assert!((clip.duration_secs() - 3.0 / 24.0).abs() < 1e-5);
    }

    #[test]
    fn scan_defaults_without_manifest() {
        let dir = clip_dir(&["a.jpg", "b.jpeg"], None);
        let clip = VideoClip::scan_dir(dir.path()).unwrap();
        assert!((clip.manifest.fps_or_default() - DEFAULT_FPS).abs() < 1e-5);
        assert!(!clip.manifest.loop_clip);
    }

    #[test]
    fn scan_errors_are_typed() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            VideoClip::scan_dir(&dir.path().join("nope")),
            Err(VideoError::NotFound(_))
        ));
        assert!(matches!(
            VideoClip::scan_dir(dir.path()),
            Err(VideoError::NoFrames(_))
        ));
        let bad = clip_dir(&["f.png"], Some("not json"));
        assert!(matches!(
            VideoClip::scan_dir(bad.path()),
            Err(VideoError::Manifest(_))
        ));
    }

    #[test]
    fn player_plays_finishes_and_skips() {
        let dir = clip_dir(&["f1.png", "f2.png"], Some(r#"{"fps": 2.0}"#));
        let clip = VideoClip::scan_dir(dir.path()).unwrap();
        let mut p = VideoPlayer::new(clip);
        assert!(!p.playing && !p.finished);
        assert_eq!(p.frame_index(), 0);
        p.play();
        assert!(!p.tick(0.25));
        assert_eq!(p.frame_index(), 0);
        assert!(!p.tick(0.25));
        assert_eq!(p.frame_index(), 1);
        assert!(p.tick(1.0));
        assert!(p.finished && !p.playing);
        assert_eq!(p.progress(), 1.0);
        // replay after finish restarts
        p.play();
        assert!(!p.finished && p.elapsed() == 0.0);
        p.skip();
        assert!(p.finished);
    }

    #[test]
    fn player_loops_forever() {
        let dir = clip_dir(&["f1.png", "f2.png"], Some(r#"{"fps": 2.0, "loop": true}"#));
        let clip = VideoClip::scan_dir(dir.path()).unwrap();
        let mut p = VideoPlayer::new(clip);
        p.play();
        assert!(!p.tick(10.0));
        assert!(!p.finished && p.loops_done >= 1);
    }
}
