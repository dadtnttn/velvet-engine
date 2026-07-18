//! Sprite frame animation (CPU); pairs with texture atlases for UVs.

use serde::{Deserialize, Serialize};
use velvet_math::Rect;

use crate::texture::{TextureId, TextureRegion};

/// One animation frame: pixel region or precomputed UV.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimFrame {
    /// Pixel region in the atlas texture.
    pub region: TextureRegion,
    /// Optional per-frame duration override (seconds). `None` = use clip fps.
    pub duration: Option<f32>,
}

impl AnimFrame {
    /// Frame from a pixel region.
    pub fn region(region: TextureRegion) -> Self {
        Self {
            region,
            duration: None,
        }
    }

    /// Frame with explicit duration seconds.
    pub fn with_duration(mut self, secs: f32) -> Self {
        self.duration = Some(secs.max(1e-4));
        self
    }

    /// UV rect given texture size.
    pub fn uv(&self, tex_w: f32, tex_h: f32) -> Rect {
        self.region.to_uv(tex_w, tex_h)
    }
}

/// Playback mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AnimLoop {
    /// Play once and hold last frame.
    Once,
    /// Loop from start.
    #[default]
    Loop,
    /// Ping-pong forward/back.
    PingPong,
}

/// Sprite animation clip: frames + timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    /// Optional debug name.
    pub name: String,
    /// Backing texture.
    pub texture: TextureId,
    /// Texture pixel size for UV conversion.
    pub texture_size: (f32, f32),
    /// Frames in order.
    pub frames: Vec<AnimFrame>,
    /// Frames per second when frame has no duration override.
    pub fps: f32,
    /// Loop mode.
    pub loop_mode: AnimLoop,
    /// Current frame index.
    frame_index: usize,
    /// Time accumulator within current frame.
    accum: f32,
    /// Playing.
    playing: bool,
    /// Ping-pong direction (+1 / -1).
    direction: i32,
    /// Finished (once mode).
    finished: bool,
}

impl SpriteAnimation {
    /// Create a clip from equal-sized grid cells (left-to-right, top-to-bottom).
    pub fn from_grid(
        texture: TextureId,
        tex_w: f32,
        tex_h: f32,
        frame_w: f32,
        frame_h: f32,
        frame_count: usize,
        fps: f32,
    ) -> Self {
        let cols = (tex_w / frame_w.max(1.0)).floor().max(1.0) as usize;
        let mut frames = Vec::with_capacity(frame_count);
        for i in 0..frame_count {
            let col = i % cols;
            let row = i / cols;
            frames.push(AnimFrame::region(TextureRegion {
                x: col as f32 * frame_w,
                y: row as f32 * frame_h,
                width: frame_w,
                height: frame_h,
            }));
        }
        Self::new("grid", texture, (tex_w, tex_h), frames, fps)
    }

    /// Create from explicit frames.
    pub fn new(
        name: impl Into<String>,
        texture: TextureId,
        texture_size: (f32, f32),
        frames: Vec<AnimFrame>,
        fps: f32,
    ) -> Self {
        Self {
            name: name.into(),
            texture,
            texture_size,
            frames,
            fps: fps.max(0.01),
            loop_mode: AnimLoop::Loop,
            frame_index: 0,
            accum: 0.0,
            playing: true,
            direction: 1,
            finished: false,
        }
    }

    /// Set loop mode (builder).
    pub fn with_loop(mut self, mode: AnimLoop) -> Self {
        self.loop_mode = mode;
        self
    }

    /// Whether playing.
    pub fn is_playing(&self) -> bool {
        self.playing && !self.finished
    }

    /// Finished (once mode reached end).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Current frame index.
    pub fn current_index(&self) -> usize {
        self.frame_index
    }

    /// Current frame, if any.
    pub fn current_frame(&self) -> Option<&AnimFrame> {
        self.frames.get(self.frame_index)
    }

    /// Current frame UV in `0..=1` texture space.
    pub fn current_uv(&self) -> Option<Rect> {
        self.current_frame()
            .map(|f| f.uv(self.texture_size.0, self.texture_size.1))
    }

    /// Current pixel region.
    pub fn current_region(&self) -> Option<TextureRegion> {
        self.current_frame().map(|f| f.region)
    }

    /// Pause.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resume.
    pub fn play(&mut self) {
        self.playing = true;
        if self.finished && self.loop_mode == AnimLoop::Once {
            self.restart();
        }
    }

    /// Restart from frame 0.
    pub fn restart(&mut self) {
        self.frame_index = 0;
        self.accum = 0.0;
        self.direction = 1;
        self.finished = false;
        self.playing = true;
    }

    /// Duration of the current frame in seconds.
    fn frame_duration(&self) -> f32 {
        if let Some(f) = self.current_frame() {
            if let Some(d) = f.duration {
                return d;
            }
        }
        1.0 / self.fps
    }

    /// Advance animation by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if !self.playing || self.finished || self.frames.is_empty() {
            return;
        }
        self.accum += dt.max(0.0);
        while self.accum >= self.frame_duration() {
            let dur = self.frame_duration();
            self.accum -= dur;
            if !self.advance_frame() {
                break;
            }
        }
    }

    fn advance_frame(&mut self) -> bool {
        let n = self.frames.len();
        if n == 0 {
            self.finished = true;
            return false;
        }
        if n == 1 {
            if self.loop_mode == AnimLoop::Once {
                self.finished = true;
                self.playing = false;
            }
            return false;
        }
        match self.loop_mode {
            AnimLoop::Loop => {
                self.frame_index = (self.frame_index + 1) % n;
                true
            }
            AnimLoop::Once => {
                if self.frame_index + 1 >= n {
                    self.finished = true;
                    self.playing = false;
                    false
                } else {
                    self.frame_index += 1;
                    true
                }
            }
            AnimLoop::PingPong => {
                let next = self.frame_index as i32 + self.direction;
                if next < 0 {
                    self.direction = 1;
                    self.frame_index = 1.min(n - 1);
                } else if next as usize >= n {
                    self.direction = -1;
                    self.frame_index = n.saturating_sub(2);
                } else {
                    self.frame_index = next as usize;
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_at_fps() {
        let tex = TextureId::allocate();
        let mut anim = SpriteAnimation::from_grid(tex, 64.0, 16.0, 16.0, 16.0, 4, 10.0);
        assert_eq!(anim.current_index(), 0);
        anim.update(0.11);
        assert_eq!(anim.current_index(), 1);
        let uv = anim.current_uv().unwrap();
        assert!((uv.min.x - 0.25).abs() < 1e-4);
    }

    #[test]
    fn once_finishes() {
        let tex = TextureId::allocate();
        let frames = vec![
            AnimFrame::region(TextureRegion::full(8.0, 8.0)),
            AnimFrame::region(TextureRegion {
                x: 8.0,
                y: 0.0,
                width: 8.0,
                height: 8.0,
            }),
        ];
        let mut anim =
            SpriteAnimation::new("once", tex, (16.0, 8.0), frames, 10.0).with_loop(AnimLoop::Once);
        anim.update(0.25);
        assert!(anim.is_finished());
        assert_eq!(anim.current_index(), 1);
    }

    #[test]
    fn loop_wraps() {
        let tex = TextureId::allocate();
        let mut anim = SpriteAnimation::from_grid(tex, 32.0, 16.0, 16.0, 16.0, 2, 10.0);
        anim.update(0.25);
        assert_eq!(anim.current_index(), 0);
    }

    #[test]
    fn sheet_controller_drives_sprite_region() {
        use crate::sprite::Sprite;
        use velvet_math::{Transform2D, Vec2};

        let tex = TextureId::allocate();
        let mut anim = SpriteAnimation::from_grid(tex, 64.0, 16.0, 16.0, 16.0, 4, 8.0);
        let mut sprite = Sprite {
            texture: tex,
            transform: Transform2D::from_translation(Vec2::ZERO),
            region: anim.current_region(),
            size: Some(Vec2::new(16.0, 16.0)),
            anchor: Vec2::new(0.5, 0.5),
            tint: Default::default(),
            z: 0.0,
            flip: Default::default(),
        };

        // Advance several frames and keep sprite.region in sync (controller integration).
        for _ in 0..3 {
            anim.update(0.15);
            sprite.region = anim.current_region();
        }
        assert!(anim.current_index() >= 1);
        assert_eq!(sprite.region, anim.current_region());
        assert!(sprite.region.unwrap().width > 0.0);
    }

    #[test]
    fn ping_pong_controller() {
        let tex = TextureId::allocate();
        let mut anim = SpriteAnimation::from_grid(tex, 48.0, 16.0, 16.0, 16.0, 3, 10.0)
            .with_loop(AnimLoop::PingPong);
        // Run long enough to reverse.
        for _ in 0..20 {
            anim.update(0.1);
        }
        assert!(anim.is_playing());
        assert!(anim.current_index() < 3);
    }

    #[test]
    fn multi_clip_switch() {
        let tex = TextureId::allocate();
        let mut idle = SpriteAnimation::from_grid(tex, 32.0, 16.0, 16.0, 16.0, 2, 5.0);
        let mut run = SpriteAnimation::from_grid(tex, 64.0, 16.0, 16.0, 16.0, 4, 12.0);
        idle.update(0.5);
        // Switch to run clip (game would swap controller).
        run.restart();
        run.update(0.1);
        assert_eq!(run.current_index(), 1);
        assert_ne!(idle.current_uv(), run.current_uv());
    }
}
