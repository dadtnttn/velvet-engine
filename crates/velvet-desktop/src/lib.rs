//! # velvet-desktop
//!
//! Velvet Desktop runs a JavaScript game as a desktop program. A native window hosts the system
//! webview (WebView2 on Windows). The game's folder is served through a custom protocol, and a
//! small native bridge gives the game `window.velvet`: saves, settings, window control and logs.
//!
//! The modules:
//!
//! - [`origin`]: origins of addresses, for keeping the bridge to the game.
//! - [`manifest`]: the game folder's `velvet-game.json`.
//! - [`serve`]: safe file serving from the game folder.
//! - [`storage`]: the per-user data folder, save slots and settings.
//! - [`bridge`]: the `window.velvet` contract, its commands and the scripts it needs.
//! - [`cli`]: command-line parsing for the `velvet-desktop` binary.
//! - `app` (the `window` feature, on by default): the window, the webview and the event loop.
//! - [`log`]: the terminal and the log file in the data folder.

#![deny(missing_docs)]

#[cfg(feature = "window")]
pub mod app;
pub mod bridge;
pub mod cli;
pub mod log;
pub mod manifest;
pub mod origin;
pub mod serve;
pub mod storage;

pub use manifest::Manifest;
