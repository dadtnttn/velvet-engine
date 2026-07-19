//! Personalized Velvet Arcana UI modules (layout, theme, menu, HUD).

mod hud;
mod menu;
pub mod theme;

pub use menu::{paint_collection, paint_options, paint_shop, paint_title_menu};
pub use theme::{Theme, TITLE_ITEMS, WW, WH};
