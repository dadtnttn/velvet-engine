//! # velvet-host2d
//!
//! The host of a Velvet Engine 2D game. A game draws on a [`velvet_canvas::Canvas`] of its logical size (authored at,
//! say, 1920×1080) and reads an [`Input`] in those same coordinates; the host does the rest:
//!
//! - opens a resizable window and shows the canvas **letterboxed** at the logical aspect, never deformed, with bars
//!   that take no input;
//! - draws at the **window's own pixels** (`native`), so text and lines are sharp at any size, or at the logical size
//!   scaled by the graphics card (for weak machines). `VELVET_RENDER_SIZE=2560x1440` forces the canvas's pixels (QA);
//! - turns the pointer back into logical coordinates **exactly** (the inverse of the presentation);
//! - runs the game in **fixed steps** (60 a second by default) and draws once a frame;
//! - saves a **screenshot** of the canvas with F12 (into `captures_dir`);
//! - shows a game's **2.5D stage** ([`velvet_stage::Stage`]: painted cards in 3D, composited on the graphics card)
//!   when [`Game2D::stage`] gives one, with the canvas over it only if [`HostConfig::overlay`] says so;
//! - and runs the same game **without a window** ([`Headless`]) for tests, bots and captures.
//!
//! It also runs on **Android** through `velvet-platform` (see `docs/platforms/ANDROID.md`): the first finger is the
//! pointer, the back button arrives as Escape (and as [`Game2D::platform_event`]), the GPU surface follows the app's
//! suspend and resume, and the screen is kept full and on.
//!
//! ```ignore
//! use velvet_host2d::{Game2D, HostConfig, Input};
//! use velvet_host2d::velvet_canvas::{Canvas, Color};
//!
//! struct Hello;
//! impl Game2D for Hello {
//!     fn update(&mut self, _input: &Input, _dt: f32) {}
//!     fn draw(&mut self, canvas: &mut Canvas) { canvas.clear(Color::hex(0x1b1530)); }
//! }
//! velvet_host2d::run(HostConfig::new("Hello"), Hello).unwrap();
//! ```

mod headless;
mod input;
mod viewport;
mod window;

use std::path::PathBuf;

pub use headless::Headless;
pub use input::{Button, Input, InputEvent, KeyCode};
pub use velvet_canvas;
pub use velvet_platform;
pub use velvet_stage;
pub use viewport::Viewport;
pub use window::run;

use velvet_canvas::Canvas;

/// How the host shows a game.
#[derive(Debug, Clone)]
pub struct HostConfig {
    pub title: String,
    /// The size the game is authored at.
    pub logical_width: f32,
    pub logical_height: f32,
    /// How far the logical size may grow to fill a window of another shape: a taller window gets a taller picture, a
    /// wider one a wider picture, up to these (past them, bars). The game hears of it in
    /// [`Game2D::logical_size_changed`]. Left at the authored size, the picture keeps its shape, with bars.
    pub max_logical_width: f32,
    pub max_logical_height: f32,
    /// The window's first size and its smallest size, in pixels.
    pub window_width: u32,
    pub window_height: u32,
    pub min_width: u32,
    pub min_height: u32,
    /// Draw at the window's own pixels (true) or at the logical size, scaled by the graphics card (false).
    pub native: bool,
    /// The length of a fixed step, in seconds.
    pub step: f32,
    /// Where F12 saves screenshots (None: F12 does nothing).
    pub captures_dir: Option<PathBuf>,
    /// With a stage, whether the canvas is drawn over it (a game whose HUD is made of screen cards says no, and is
    /// spared painting and sending a whole canvas every frame).
    pub overlay: bool,
    /// Whether the window opens maximised (it can always go fullscreen with F11 or Alt+Enter).
    pub maximized: bool,
}

impl HostConfig {
    /// A 1920×1080 game in a 1600×900 window, at least 1280×720, drawn at native pixels, 60 steps a second.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            logical_width: 1920.0,
            logical_height: 1080.0,
            max_logical_width: 1920.0,
            max_logical_height: 1080.0,
            window_width: 1600,
            window_height: 900,
            min_width: 1280,
            min_height: 720,
            native: true,
            step: 1.0 / 60.0,
            captures_dir: Some(PathBuf::from("captures")),
            overlay: true,
            maximized: false,
        }
    }

    /// The logical size for a window of `width`×`height` pixels: the authored size, its height grown for a taller
    /// window or its width for a wider one, as far as the maxima allow (whole logical pixels).
    pub fn logical_for(&self, width: u32, height: u32) -> (f32, f32) {
        let (w, h) = (self.logical_width, self.logical_height);
        if width == 0 || height == 0 || w <= 0.0 || h <= 0.0 {
            return (w, h);
        }
        let aspect = width as f32 / height as f32;
        if aspect < w / h {
            (
                w,
                (w / aspect)
                    .round()
                    .clamp(h, self.max_logical_height.max(h)),
            )
        } else {
            (
                (h * aspect).round().clamp(w, self.max_logical_width.max(w)),
                h,
            )
        }
    }

    /// The canvas's pixels for a picture of `width`×`height` window pixels: the forced size of `VELVET_RENDER_SIZE`,
    /// the picture's own pixels when native, else the logical size.
    pub fn canvas_pixels(&self, width: u32, height: u32) -> (u32, u32) {
        if let Some(forced) = std::env::var("VELVET_RENDER_SIZE")
            .ok()
            .and_then(|v| parse_size(&v))
        {
            return forced;
        }
        if self.native {
            (width.max(1), height.max(1))
        } else {
            let (w, h) = self.logical_for(width, height);
            (w.round() as u32, h.round() as u32)
        }
    }
}

/// `2560x1440` → (2560, 1440).
fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.trim().split_once(['x', 'X', '×'])?;
    Some((
        w.trim().parse().ok().filter(|&w| w > 0)?,
        h.trim().parse().ok().filter(|&h| h > 0)?,
    ))
}

/// A 2D game, as the host runs it.
pub trait Game2D {
    /// One fixed step of the game, with what the player did since the last one.
    fn update(&mut self, input: &Input, dt: f32);

    /// Draws a frame. The canvas keeps the logical size; its pixels follow the window.
    fn draw(&mut self, canvas: &mut Canvas);

    /// The canvas's pixels changed (the window was resized): layers drawn at the old pixels are stale.
    fn pixels_changed(&mut self, _canvas: &Canvas) {}

    /// The platform did something: the app went to the background (`Suspended`: save and pause the
    /// sound, Android may end the process from here on), came back (`Resumed`), lost or regained focus,
    /// ran short of memory, or the back button was pressed (`Back`; the game also hears it as Escape).
    fn platform_event(&mut self, _event: velvet_platform::PlatformEvent) {}

    /// The game's 2.5D stage, if it has one: drawn every frame under the canvas (or alone, see
    /// [`HostConfig::overlay`]).
    fn stage(&self) -> Option<&velvet_stage::Stage> {
        None
    }

    /// The stage's pixels changed (the window was resized): pictures painted for the old size may be repainted.
    fn stage_pixels_changed(&mut self, _width: u32, _height: u32) {}

    /// True to close the game.
    fn quit(&self) -> bool {
        false
    }

    /// The logical size changed (the window took another shape, and [`HostConfig`]'s maxima let the picture follow):
    /// pointer coordinates come in the new size from now on, and what is laid out on the screen may be placed again.
    fn logical_size_changed(&mut self, _width: f32, _height: f32) {}

    /// False while the game draws its own pointer (a tool in hand that follows it): the system's arrow is then hidden
    /// over the picture.
    fn shows_pointer(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_read_with_any_x() {
        assert_eq!(parse_size("2560x1440"), Some((2560, 1440)));
        assert_eq!(parse_size(" 1280 X 720 "), Some((1280, 720)));
        assert_eq!(parse_size("0x720"), None);
        assert_eq!(parse_size("big"), None);
    }

    #[test]
    fn the_logical_size_grows_to_the_window_as_far_as_allowed() {
        let mut config = HostConfig::new("t");
        // At the authored size only, any window keeps 1920×1080 (with bars).
        assert_eq!(config.logical_for(2560, 1600), (1920.0, 1080.0));
        config.max_logical_height = 1440.0;
        config.max_logical_width = 2560.0;
        assert_eq!(config.logical_for(3840, 2160), (1920.0, 1080.0));
        assert_eq!(config.logical_for(2560, 1600), (1920.0, 1200.0));
        assert_eq!(config.logical_for(1024, 768), (1920.0, 1440.0));
        assert_eq!(config.logical_for(2560, 1080), (2560.0, 1080.0));
        assert_eq!(config.logical_for(2400, 1080), (2400.0, 1080.0));
        // Past the maxima, they hold (and the picture gets bars).
        assert_eq!(config.logical_for(1000, 1000), (1920.0, 1440.0));
        assert_eq!(config.logical_for(4000, 1000), (2560.0, 1080.0));
    }

    #[test]
    fn the_canvas_follows_the_picture_when_native() {
        let mut config = HostConfig::new("t");
        if std::env::var("VELVET_RENDER_SIZE").is_err() {
            assert_eq!(config.canvas_pixels(1366, 768), (1366, 768));
            config.native = false;
            assert_eq!(config.canvas_pixels(1366, 768), (1920, 1080));
        }
    }
}
