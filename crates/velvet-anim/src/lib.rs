//! # velvet-anim
//!
//! **Animation & VFX tools** for Velvet Engine: poses, tweens, named effects,
//! multi-target director, and a tiny author script (`.vanim`) plus story host
//! commands (`anim.fx`, `anim.move`, `anim.script`).
//!
//! Use for cards, UI, sprites, VN stage props — anything with an [`AnimPose`].
//!
//! ## Story language
//!
//! ```text
//! call anim.fx:
//!     target: card0
//!     effect: deal
//!     x: 200
//!     y: 360
//!     duration: 0.35
//! ```
//!
//! ## `.vanim` script
//!
//! ```text
//! spawn card0 0 0
//! fx card0 deal 200 360 0.35
//! wait 0.1
//! fx card0 punch strength 0.15
//! ```

#![deny(missing_docs)]

mod director;
mod effect;
mod host;
mod pose;
mod script;
mod tween;

pub use director::{AnimDirector, AnimTarget};
pub use effect::{build_effect, sample_tweens, tick_tweens, EffectKind, EffectParams};
pub use host::AnimStoryHost;
pub use pose::{AnimField, AnimPose};
pub use script::{
    apply_program_immediate, parse_anim_script, AnimOp, AnimProgram, AnimScriptError,
    AnimScriptRunner,
};
pub use tween::{apply_field, parse_ease, read_field, FloatTween};
