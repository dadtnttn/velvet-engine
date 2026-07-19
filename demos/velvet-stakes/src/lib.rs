//! Velvet Stakes library — live dev, logo, and **title UI paint** (shared with the bin).
//!
//! The playable bin is `velvet-stakes` (`src/main.rs`). This lib exposes the
//! shipped author hot-reload path and the real title/lobby paint entry used by
//! the game so unit tests exercise the same composition path.

#![deny(missing_docs)]

pub mod live_dev;
pub mod logo;
/// Softbuffer helpers shared with the demo bin.
pub mod render;
/// Title / lobby / modal UI paint + theme.
pub mod ui;

pub use live_dev::{
    load_rgb_buf, reload_stylesheet, ImageSlot, LiveDevApply, LiveDevSession, WatchKind,
};
pub use logo::{
    blit_rgba_bilinear, count_soft_alpha, load_title_wordmark, probe_scaled_soft_alpha, RgbaBuf,
};
pub use render::{load_rgb, ArtBank, RgbImage};
pub use ui::{
    paint_collection, paint_options, paint_shop, paint_title_menu, theme, TITLE_ITEMS, WW, WH,
};
