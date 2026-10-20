//! # velvet-stage
//!
//! 2.5D for Velvet Engine: a stage of flat **cards**, each showing a **picture** painted with
//! [`velvet_canvas::Canvas`], placed in 3D and seen through a perspective [`Camera`]: a paper theatre. The pictures are
//! painted once (and again only when they change) and the graphics card composites them every frame, so a camera
//! that moves, tilts and leans costs nothing on the processor.
//!
//! - **World cards** stand, lie or lean anywhere; a card can be the **child** of another (a stamp on a document moves
//!   with it). They are lit by up to 16 point **lights** (candles) over an ambient light, and fade into a **haze** with
//!   distance, so layers far away read as far away.
//! - **Screen cards** sit on top in the screen's logical pixels (the HUD).
//! - [`Stage::pick`] finds the card under the pointer (the screen first, then the nearest in the world) and the point
//!   of its picture, ignoring the transparent parts; [`Stage::pick_on`] finds the point on one card's plane even past
//!   its edges (to drag across a desk); [`Stage::project`] goes the other way.
//! - [`StageRenderer`] draws a stage with wgpu (4× multisampling, mipmapped pictures, anisotropic filtering);
//!   [`Offscreen`] draws one without a window, for tests and captures.
//!
//! ```no_run
//! use velvet_canvas::{Canvas, Color};
//! use velvet_math::Vec3;
//! use velvet_stage::{Camera, Card, Offscreen, Stage};
//!
//! let mut stage = Stage::new(1920.0, 1080.0);
//! stage.camera = Camera::new(Vec3::new(0.0, 60.0, 90.0), Vec3::new(0.0, 0.0, 0.0));
//! let mut paper = Canvas::with_pixels(400.0, 300.0, 800, 600);
//! paper.clear(Color::hex(0xf4e7cb));
//! let picture = stage.add_picture(paper);
//! // A sheet lying on the table, its top away from the camera.
//! stage.add(Card::world(picture, 40.0, 30.0).turned(-90.0, 0.0, 5.0));
//! Offscreen::new().unwrap().save_png(&stage, 1920, 1080, "stage.png").unwrap();
//! ```

mod camera;
mod offscreen;
mod render;
mod stage;

pub use camera::{Camera, Ray};
pub use offscreen::{render_rgba, Offscreen};
pub use render::StageRenderer;
pub use stage::{Blend, Card, CardId, Fog, Hit, Light, Picture, PictureId, Space, Stage, MAX_LIGHTS};
pub use velvet_math::{Vec2, Vec3};
