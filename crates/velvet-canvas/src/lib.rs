//! # velvet-canvas
//!
//! 2D drawing for Velvet Engine games, on the CPU: anti-aliased shapes, paths, gradients and soft shadows, text from
//! TrueType fonts, images (PNG, WebP, JPEG) and SVG, on a canvas of a **logical size** (the size a game is authored at,
//! such as 1920×1080) drawn at the canvas's **own pixels** (the screen's, when the game draws at native resolution).
//! Every coordinate is logical; `k` pixels stand for one logical pixel, so text and lines stay sharp at any size.
//!
//! What does not change from frame to frame (a document, a panel, a background) is drawn once into a **layer** — a
//! canvas of its own — and the layer is drawn into the frame with a transform, which costs a copy and not a redraw.
//!
//! ```ignore
//! use velvet_canvas::{Canvas, Color, Fill, Rect};
//!
//! let mut frame = Canvas::with_pixels(1920.0, 1080.0, 2560, 1440);
//! frame.clear(Color::hex(0x1b1530));
//! frame.fill_round_rect(Rect::new(100.0, 100.0, 400.0, 240.0), 18.0, &Fill::Solid(Color::hex(0xf3e6c8)));
//! frame.text("Permiso de entrada", 124.0, 128.0, &style);
//! let rgba = frame.pixels(); // premultiplied RGBA, row by row
//! ```
//!
//! The canvas does not open windows nor read input: a host presents its pixels (see the 2D host of the engine).

mod blur;
mod canvas;
mod color;
mod font;
mod geometry;
mod image;
mod path;
#[cfg(feature = "svg")]
mod svg;

pub use canvas::{Canvas, Fill, StrokeStyle};
pub use color::Color;
pub use font::{Align, Font, FontError, TextStyle};
pub use geometry::{Point, Rect};
pub use image::{Fit, Image, ImageError};
pub use path::{Path, PathBuilder};
#[cfg(feature = "svg")]
pub use svg::{Svg, SvgError};
