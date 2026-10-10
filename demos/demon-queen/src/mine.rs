//! The mine mouth in the Dark Forest, east of the trail to the village: where
//! the Queen stands at its lip, and how near F takes her down into the
//! procedural dungeon. The forest's export models the mouth itself.
use super::*;

/// The mouth's footprint, x and z bounds in world units.
const PIT_X: [f32; 2] = [11.333_333, 14.0];
const PIT_Z: [f32; 2] = [5.333_333, 7.333_333];
/// The prompt shows from this far, and F works from this close.
pub(super) const SHOW_RADIUS: f32 = 6.0;
pub(super) const INTERACT_RADIUS: f32 = 2.4;

pub(super) fn centre() -> Vec3 {
    Vec3::new(
        (PIT_X[0] + PIT_X[1]) * 0.5,
        0.0,
        (PIT_Z[0] + PIT_Z[1]) * 0.5,
    )
}

/// Where the Queen stands to go down: the lip facing the trail.
pub(super) fn entry_position() -> Vec3 {
    let z = centre().z;
    let x = PIT_X[0] - 0.55;
    Vec3::new(x, nature_walk_surface_height(x, z), z)
}

pub(super) fn entry_distance(position: Vec3) -> f32 {
    let e = entry_position();
    Vec3::new(position.x - e.x, 0.0, position.z - e.z).length()
}
