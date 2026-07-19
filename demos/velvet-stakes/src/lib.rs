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
/// SVG title wordmark (vector paths → raster via velvet-image).
pub mod wordmark_svg;

pub use live_dev::{
    load_rgb_buf, reload_stylesheet, ImageSlot, LiveDevApply, LiveDevSession, WatchKind,
};
pub use logo::{
    blit_rgba_bilinear, content_bounds, count_soft_alpha, crop_to_content, load_title_wordmark,
    probe_scaled_soft_alpha, RgbaBuf,
};
pub use render::{load_rgb, ArtBank, RgbImage};
pub use ui::{
    paint_collection, paint_options, paint_shop, paint_title_menu, theme, TITLE_ITEMS, WW, WH,
};
pub use wordmark_svg::{
    load_title_wordmark_svg, rasterize_svg_wordmark, rasterize_title_wordmark, resolve_title_wordmark,
    title_path_d, title_wordmark_svg_xml, TITLE_RASTER_H, TITLE_RASTER_W,
};
