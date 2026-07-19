//! # velvet-anim
//!
//! **Tools** (not premade games) for motion and image VFX in Velvet Engine.
//!
//! | Layer | What you get |
//! |-------|----------------|
//! | Tweens / director | 2D pose fields, multi-target ids |
//! | [`Pose3D`] + [`project_image`] | Perspective billboards on **your** images |
//! | [`Timeline`] / [`ChannelTrack`] | Keyframes **you** author on any channel |
//! | Story `anim.pose3d` / `anim.track` | Drive tools from `.vstory` |
//! | [`recipes`] | Optional examples built *from* tools — skip if you prefer raw APIs |
//!
//! ## Build your own flip / pack-like motion
//!
//! ```ignore
//! use velvet_anim::{ChannelTrack, Timeline, Pose3D, Pose3DChannel, project_image, Fx3dCamera};
//! use velvet_math::{Ease, Vec2};
//!
//! let mut tl = Timeline::new().with_channel(
//!     ChannelTrack::new(Pose3DChannel::Yaw)
//!         .key(0.0, 0.0, Ease::Linear)
//!         .key(0.4, std::f32::consts::PI, Ease::CubicInOut),
//! );
//! tl.tick(dt);
//! let pose = tl.sample_pose(Pose3D::flat(Vec2::new(200.0, 300.0)));
//! let quad = project_image(&pose, 70.0, 100.0, &Fx3dCamera::default());
//! // draw your texture on quad corners
//! ```

#![deny(missing_docs)]

mod director;
mod effect;
mod fx3d;
mod host;
mod pose;
pub mod recipes;
mod script;
mod track;
mod tween;

pub use director::{AnimDirector, AnimTarget};
pub use effect::{build_effect, sample_tweens, tick_tweens, EffectKind, EffectParams};
pub use fx3d::{
    foil_phase, project_image, sort_projected, yaw_flip_amount, Fx3dCamera, ImageBillboard, Pose3D,
    Pose3DChannel, ProjectedQuad,
};
pub use host::AnimStoryHost;
pub use pose::{AnimField, AnimPose};
pub use script::{
    apply_program_immediate, parse_anim_script, AnimOp, AnimProgram, AnimScriptError,
    AnimScriptRunner,
};
pub use track::{parse_track_line, ChannelTrack, Keyframe, Timeline};
pub use tween::{apply_field, parse_ease, read_field, FloatTween};
