//! # velvet-anim
//!
//! **Animation & VFX tools** for Velvet Engine: poses, tweens, named effects,
//! multi-target director, `.vanim` scripts, story host commands, and **3D-style
//! image FX** (perspective quads, card flip, **pack open** generators).
//!
//! Use for cards, UI, sprites, VN stage props — anything with an [`AnimPose`]
//! or [`Pose3D`] billboard.
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
//!
//! call anim.pack_open:
//!     x: 480
//!     y: 270
//!     cards: 5
//!     duration: 2.0
//! ```
//!
//! ## `.vanim` script
//!
//! ```text
//! spawn card0 0 0
//! fx card0 deal 200 360 0.35
//! pack_open 480 270 5 2.0
//! ```

#![deny(missing_docs)]

mod director;
mod effect;
mod fx3d;
mod host;
mod pose;
mod script;
mod tween;

pub use director::{AnimDirector, AnimTarget};
pub use effect::{build_effect, sample_tweens, tick_tweens, EffectKind, EffectParams};
pub use fx3d::{
    foil_phase, project_image, sample_card_flip, Fx3dCamera, PackLayer, PackLayerKind, PackOpenFx,
    PackOpenParams, PackPhase, Pose3D, ProjectedQuad,
};
pub use host::AnimStoryHost;
pub use pose::{AnimField, AnimPose};
pub use script::{
    apply_program_immediate, parse_anim_script, AnimOp, AnimProgram, AnimScriptError,
    AnimScriptRunner,
};
pub use tween::{apply_field, parse_ease, read_field, FloatTween};
