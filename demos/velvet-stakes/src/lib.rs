//! Velvet Stakes library — live dev session + shared types for tests.
//!
//! The playable bin is `velvet-stakes` (`src/main.rs`). This lib exposes the
//! **shipped** author hot-reload path used by `--dev`.

#![deny(missing_docs)]

pub mod live_dev;
pub mod logo;

pub use live_dev::{
    load_rgb_buf, reload_stylesheet, ImageSlot, LiveDevApply, LiveDevSession, WatchKind,
};
pub use logo::{
    blit_rgba_bilinear, count_soft_alpha, load_title_wordmark, probe_scaled_soft_alpha, RgbaBuf,
};
