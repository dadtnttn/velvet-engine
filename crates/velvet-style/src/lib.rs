//! # velvet-style
//!
//! **One stylesheet language** (`.vcss`) for visual style **and** motion.
//!
//! - UI: classes, ids, `:selected`, colors, layout numbers  
//! - Motion: `@keyframes` + `animation:` (replaces separate `.vanim`)  
//! - Legacy `.vanim` text can be converted with [`vanim_to_vcss`]
//!
//! ## Example
//!
//! ```css
//! .button { background: #0a0c16; color: #d2af64; height: 52; }
//! .button:selected { background: #501e78; color: #ffe496; glow: #dc50dc; }
//!
//! @keyframes deal {
//!   from { opacity: 0; y: -80; scale: 0.65; yaw: 0.9; }
//!   to   { opacity: 1; y: 0;   scale: 1;    yaw: 0; }
//! }
//! .card.deal { animation: deal 0.35s cubic_out; }
//! ```

#![deny(missing_docs)]

mod animation;
mod host;
mod parse;
mod resolve;
mod value;

pub use animation::{
    animation_spec_from_computed, plan_animation, plan_from_spec, vanim_to_vcss, AnimationSpec,
    ChannelPlan, KeyframeStop, Keyframes, TimelinePlan,
};
pub use host::StyleStoryHost;
pub use parse::{parse_stylesheet, StyleParseError, StyleRule, Stylesheet};
pub use resolve::{resolve, ComputedStyle, StyleQuery, StyleRegistry};
pub use value::{parse_color, parse_value, Color, StyleValue};
