//! **Optional recipes** built *from* tools — not the product API.
//!
//! Prefer [`crate::track::Timeline`] + [`crate::fx3d::project_image`] for your own
//! effects. These functions only show how to compose the tools.

use velvet_math::{Ease, Vec2};

use crate::fx3d::{Pose3D, Pose3DChannel};
use crate::track::{ChannelTrack, Timeline};

/// Example: build a **custom** single-card flip timeline (you own the result).
///
/// This is a recipe, not a mandatory engine feature — copy/adapt freely.
pub fn recipe_card_flip(duration: f32) -> Timeline {
    let d = duration.max(0.05);
    Timeline::new()
        .with_channel(
            ChannelTrack::new(Pose3DChannel::Yaw)
                .key(0.0, 0.0, Ease::Linear)
                .key(d, std::f32::consts::PI, Ease::CubicInOut),
        )
        .with_channel(
            ChannelTrack::new(Pose3DChannel::Scale)
                .key(0.0, 1.0, Ease::Linear)
                .key(d * 0.5, 1.08, Ease::QuadOut)
                .key(d, 1.0, Ease::QuadIn),
        )
}

/// Example: one card “comes out of a pack” motion (position + flip + fade).
///
/// Compose more cards yourself with different delays/positions.
pub fn recipe_card_emerge(
    from: Vec2,
    to: Vec2,
    duration: f32,
    start_yaw: f32,
) -> Timeline {
    let d = duration.max(0.05);
    Timeline::new()
        .with_channel(
            ChannelTrack::new(Pose3DChannel::X)
                .key(0.0, from.x, Ease::Linear)
                .key(d, to.x, Ease::CubicOut),
        )
        .with_channel(
            ChannelTrack::new(Pose3DChannel::Y)
                .key(0.0, from.y, Ease::Linear)
                .key(d, to.y, Ease::BackOut),
        )
        .with_channel(
            ChannelTrack::new(Pose3DChannel::Yaw)
                .key(0.0, start_yaw, Ease::Linear)
                .key(d, 0.0, Ease::CubicOut),
        )
        .with_channel(
            ChannelTrack::new(Pose3DChannel::Opacity)
                .key(0.0, 0.0, Ease::Linear)
                .key(d * 0.25, 1.0, Ease::QuadOut),
        )
}

/// Apply a timeline onto a pose for one frame (recipe helper).
pub fn sample_recipe(tl: &Timeline, base: Pose3D) -> Pose3D {
    tl.sample_pose(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_is_just_timeline_tools() {
        let mut tl = recipe_card_flip(0.4);
        tl.elapsed = 0.4;
        let p = sample_recipe(&tl, Pose3D::default());
        assert!((p.yaw - std::f32::consts::PI).abs() < 0.05);
    }
}
