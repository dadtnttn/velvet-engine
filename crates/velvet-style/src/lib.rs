//! # velvet-style
//!
//! **Tools** for CSS-like stylesheets (`.vcss`) you author and invoke at runtime.
//!
//! Not a full browser CSS engine — a focused subset for game UI:
//! classes, ids, pseudo-states (`:selected`), colors, lengths, cascade.
//!
//! ## Example
//!
//! ```css
//! .button {
//!   background: #0a0c16;
//!   border-color: #b9964b;
//!   color: #d2af64;
//!   height: 52;
//! }
//! .button:selected {
//!   background: #501e78;
//!   border-color: #ffdc96;
//!   glow: #dc50dc;
//!   color: #ffe496;
//! }
//! ```
//!
//! ```ignore
//! use velvet_style::{parse_stylesheet, resolve, StyleQuery};
//! let sheet = parse_stylesheet(src)?;
//! let style = resolve(&sheet, &StyleQuery::class("button").with_state("selected"));
//! let bg = style.background();
//! ```

#![deny(missing_docs)]

mod host;
mod parse;
mod resolve;
mod value;

pub use host::StyleStoryHost;
pub use parse::{parse_stylesheet, StyleParseError, StyleRule, Stylesheet};
pub use resolve::{resolve, ComputedStyle, StyleQuery, StyleRegistry};
pub use value::{parse_color, parse_value, Color, StyleValue};
