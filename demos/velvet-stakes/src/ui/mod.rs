//! Personalized Velvet Arcana UI modules (layout, theme, menu, HUD, buttons).

pub mod buttons;
/// Card archive, starter-deck editor, and pointer geometry.
pub mod collection;
/// Full gameplay HUD, arena, card-hand painter, and pointer geometry.
pub mod gameplay;
mod hud;
/// Playable Night Market painter, view model, and pointer geometry.
pub mod market;
mod menu;
/// Run-settlement painter and pointer geometry.
pub mod result;
pub mod theme;

pub use menu::{paint_collection, paint_options, paint_shop, paint_title_menu};
pub use theme::{Theme, WH, WW};
