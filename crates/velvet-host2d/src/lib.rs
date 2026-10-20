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
//! - and runs the same game **without a window** ([`Headless`]) for tests, bots and captures.
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
}

impl HostConfig {
    /// A 1920×1080 game in a 1600×900 window, at least 1280×720, drawn at native pixels, 60 steps a second.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            logical_width: 1920.0,
            logical_height: 1080.0,
            window_width: 1600,
            window_height: 900,
            min_width: 1280,
            min_height: 720,
            native: true,
            step: 1.0 / 60.0,
            captures_dir: Some(PathBuf::from("captures")),
        }
    }

    /// The canvas's pixels for a picture of `width`×`height` window pixels: the forced size of `VELVET_RENDER_SIZE`,
    /// the picture's own pixels when native, else the logical size.
    pub fn canvas_pixels(&self, width: u32, height: u32) -> (u32, u32) {
        if let Some(forced) = std::env::var("VELVET_RENDER_SIZE").ok().and_then(|v| parse_size(&v)) {
            return forced;
        }
        if self.native {
            (width.max(1), height.max(1))
        } else {
            (self.logical_width.round() as u32, self.logical_height.round() as u32)
        }
    }
}

/// `2560x1440` → (2560, 1440).
fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.trim().split_once(['x', 'X', '×'])?;
    Some((w.trim().parse().ok().filter(|&w| w > 0)?, h.trim().parse().ok().filter(|&h| h > 0)?))
}

/// A 2D game, as the host runs it.
pub trait Game2D {
    /// One fixed step of the game, with what the player did since the last one.
    fn update(&mut self, input: &Input, dt: f32);

    /// Draws a frame. The canvas keeps the logical size; its pixels follow the window.
    fn draw(&mut self, canvas: &mut Canvas);

    /// The canvas's pixels changed (the window was resized): layers drawn at the old pixels are stale.
    fn pixels_changed(&mut self, _canvas: &Canvas) {}

    /// True to close the game.
    fn quit(&self) -> bool {
        false
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
    fn the_canvas_follows_the_picture_when_native() {
        let mut config = HostConfig::new("t");
        if std::env::var("VELVET_RENDER_SIZE").is_err() {
            assert_eq!(config.canvas_pixels(1366, 768), (1366, 768));
            config.native = false;
            assert_eq!(config.canvas_pixels(1366, 768), (1920, 1080));
        }
    }
}
