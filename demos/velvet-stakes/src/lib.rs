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
/// Serif title wordmark (real font via fontdue).
pub mod title_font;
/// Title / lobby / modal UI paint + theme.
pub mod ui;

pub use live_dev::{
    load_rgb_buf, reload_screen, reload_stylesheet, ImageSlot, LiveDevApply, LiveDevSession,
    WatchKind,
};
pub use logo::{
    blit_rgba_bilinear, content_bounds, count_soft_alpha, crop_to_content, load_title_wordmark,
    probe_scaled_soft_alpha, RgbaBuf,
};
pub use render::{load_rgb, ArtBank, RgbImage};
pub use title_font::{paint_title_wordmark, title_font, TITLE_LINE1, TITLE_LINE2, TITLE_SUB};
pub use ui::{paint_collection, paint_options, paint_shop, paint_title_menu, theme, WH, WW};
