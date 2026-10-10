//! An isometric view of a 3D world ([`IsoCamera3D`]): it orbits its focus at a fixed distance
//! and frames it orthographically, turns in steps and tilts to a top-down view, easing into each
//! new view over a few frames. What stands between the camera and the focus gets out of the way,
//! in one of two ways:
//!
//! - **A see-through window** ([`IsoSettings3D::cut_see_through`]): a piece (a house, a tree, a
//!   wall) that hides the focus has a round window dithered open in it, round the focus on
//!   screen, through the toon shader's [`Material3D::occlusion_window`](crate::Material3D).
//! - **Cutaway walls** ([`CutawayWalls3D`]): in a plan of rooms, the walls of the focus's room
//!   that stand between it and the camera ([`PlanRoom3D::wall_faces_camera`]) drop to a stub,
//!   so the room is seen whole and its neighbours stay closed.
use crate::{Camera3D, MeshDraw3D};
use std::f32::consts::{PI, TAU};
use velvet_math::Vec3;

/// How an isometric view behaves: its turns, its elevations, its zoom range, how far the eye
/// stands and how fast a new view settles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsoSettings3D {
    /// One turn of the view (radians).
    pub step: f32,
    /// The normal elevation (radians above the ground).
    pub elevation: f32,
    /// The top-down elevation (radians).
    pub top_down_elevation: f32,
    /// The closest zoom (the orthographic view's height, metres).
    pub min_distance: f32,
    /// The farthest zoom.
    pub max_distance: f32,
    /// How far the eye stands from the focus (metres). An orthographic view keeps what stands
    /// behind its plane, so this only has to clear what stands between.
    pub orbit: f32,
    /// How high above the focus's feet the view aims (about its chest), metres.
    pub focus_height: f32,
    /// How fast a turn or a tilt settles (per second; 16 settles in about a quarter second).
    pub ease: f32,
}

impl IsoSettings3D {
    /// A true isometric view (35.26° up) turning in 15° steps and zooming between 3.8 and 18 m.
    pub const DEFAULT: Self = Self {
        step: PI / 12.0,
        elevation: 0.615_479_7,
        top_down_elevation: 1.42,
        min_distance: 3.8,
        max_distance: 18.0,
        orbit: 18.0,
        focus_height: 0.78,
        ease: 16.0,
    };

    /// How big the see-through window is on screen (its radius, as a share of the view's
    /// height): bigger as the view zooms in, so it keeps hugging the focus, but kept compact when
    /// the view is pushed against a wall (a large window would take the whole lower wall away and
    /// leave only its top: the focus needs a readable halo, not a missing room).
    pub fn see_through_radius(&self, camera: &Camera3D) -> f32 {
        let zoom = camera
            .orthographic_height
            .unwrap_or(self.max_distance)
            .max(self.min_distance);
        let zoom_factor = (15.0 / zoom).clamp(0.75, 4.0);
        (0.11 * zoom_factor).clamp(0.12, 0.265)
    }

    /// Opens the see-through window in whatever hides the focus (its feet at `focus`, on ground
    /// at `ground_y`): `bounds[i]` are the world bounds of `draws[base + i]` (`None`: never cut).
    /// A piece hides the focus when it stands on its camera side, beside its line of sight, and
    /// rises above that line where it stands (the line climbs towards the camera).
    pub fn cut_see_through(
        &self,
        draws: &mut [MeshDraw3D],
        base: usize,
        bounds: &[Option<(Vec3, Vec3)>],
        camera: &Camera3D,
        focus: Vec3,
        ground_y: f32,
    ) {
        let sight = Sight::new(self, camera, focus, ground_y);
        for (index, bounds) in bounds.iter().enumerate() {
            let Some((lo, hi)) = bounds else { continue };
            if sight.hidden_by(*lo, *hi) {
                if let Some(draw) = draws.get_mut(base + index) {
                    let ao = draw.material.occlusion_window[1];
                    draw.material.occlusion_window = [0.0, ao, sight.radius, 0.985];
                }
            }
        }
    }
}

impl Default for IsoSettings3D {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The focus's line of sight to the camera, flattened, for telling what hides it.
struct Sight {
    toward_camera: Vec3,
    across: Vec3,
    /// How fast the line of sight climbs per metre towards the camera.
    climb: f32,
    /// The window's radius on screen, and on the ground in metres (with some room round it).
    radius: f32,
    reach: f32,
    chest: f32,
    flat: Vec3,
}

impl Sight {
    fn new(settings: &IsoSettings3D, camera: &Camera3D, focus: Vec3, ground_y: f32) -> Self {
        let view = camera.position - camera.target;
        let flat_len = Vec3::new(view.x, 0.0, view.z).length().max(1e-4);
        let toward_camera = Vec3::new(view.x / flat_len, 0.0, view.z / flat_len);
        let across = Vec3::new(toward_camera.z, 0.0, -toward_camera.x);
        let climb = view.y.max(0.0) / flat_len;
        let radius = settings.see_through_radius(camera);
        let reach =
            radius * camera.orthographic_height.unwrap_or(settings.max_distance) * 0.5 + 0.6;
        let chest = ground_y + settings.focus_height;
        let flat = Vec3::new(focus.x, 0.0, focus.z);
        Self {
            toward_camera,
            across,
            climb,
            radius,
            reach,
            chest,
            flat,
        }
    }

    /// Whether a piece standing in the box `lo..hi` hides the focus.
    fn hidden_by(&self, lo: Vec3, hi: Vec3) -> bool {
        let middle = Vec3::new((lo.x + hi.x) * 0.5, 0.0, (lo.z + hi.z) * 0.5);
        let corners = [
            Vec3::new(lo.x, 0.0, lo.z),
            Vec3::new(hi.x, 0.0, lo.z),
            Vec3::new(lo.x, 0.0, hi.z),
            Vec3::new(hi.x, 0.0, hi.z),
        ];
        let (near, far) = corners
            .iter()
            .map(|c| (*c - self.flat).dot(self.toward_camera))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| {
                (a.min(v), b.max(v))
            });
        let (side_lo, side_hi) = corners
            .iter()
            .map(|c| (*c - self.flat).dot(self.across))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| {
                (a.min(v), b.max(v))
            });
        let in_front = (middle - self.flat).dot(self.toward_camera) > -0.3 && far > 0.2;
        let beside = side_lo < self.reach && side_hi > -self.reach;
        let tall_enough = hi.y > self.chest + self.climb * near.max(0.0);
        in_front && beside && tall_enough
    }
}

/// Where an isometric view stands and where it is going (see [`IsoSettings3D`] for how it
/// behaves).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsoCamera3D {
    /// The view's bearing round the focus (radians; the eye is towards +z at 0).
    pub yaw: f32,
    /// The bearing it is turning to (the controls follow this one, so they answer at once).
    pub yaw_target: f32,
    /// How high it stands now (radians), easing to the normal or the top-down elevation.
    pub elevation: f32,
    /// Whether it is top-down (or tilting there).
    pub top_down: bool,
    /// The zoom: the orthographic view's height (metres).
    pub distance: f32,
    /// How it behaves.
    pub settings: IsoSettings3D,
}

impl IsoCamera3D {
    /// A view at bearing `yaw` and the normal elevation, zoomed all the way out.
    pub fn new(settings: IsoSettings3D, yaw: f32) -> Self {
        Self {
            yaw,
            yaw_target: yaw,
            elevation: settings.elevation,
            top_down: false,
            distance: settings.max_distance,
            settings,
        }
    }

    /// Faces bearing `yaw` at once, with no easing.
    pub fn face(&mut self, yaw: f32) {
        self.yaw = yaw;
        self.yaw_target = yaw;
    }

    /// Turns by `steps` steps (negative: the other way round); the view eases there.
    pub fn turn(&mut self, steps: i32) {
        self.yaw_target = (self.yaw_target + steps as f32 * self.settings.step).rem_euclid(TAU);
    }

    /// Zooms in by `by` metres of view height, no closer than the settings allow.
    pub fn zoom_in(&mut self, by: f32) {
        self.distance = (self.distance - by).max(self.settings.min_distance);
    }

    /// Zooms out by `by` metres of view height, no farther than the settings allow.
    pub fn zoom_out(&mut self, by: f32) {
        self.distance = (self.distance + by).min(self.settings.max_distance);
    }

    /// Tilts to the top-down view, or back.
    pub fn toggle_top_down(&mut self) {
        self.top_down = !self.top_down;
    }

    /// Eases the view towards the bearing it is turning to (the short way round) and the
    /// elevation it is tilting to. Once a frame, so a turn reads as a turn and not a jump.
    pub fn ease(&mut self, dt: f32) {
        let angle_delta = (self.yaw_target - self.yaw + PI).rem_euclid(TAU) - PI;
        let follow = (1.0 - (-dt * self.settings.ease).exp()).clamp(0.0, 1.0);
        self.yaw += angle_delta * follow;
        let elevation_target = if self.top_down {
            self.settings.top_down_elevation
        } else {
            self.settings.elevation
        };
        self.elevation += (elevation_target - self.elevation) * follow;
    }

    /// The view this frame of a focus whose feet are at `focus` (its near and far planes are
    /// the caller's to set).
    pub fn camera(&self, focus: Vec3) -> Camera3D {
        let mut camera = Camera3D::default();
        let target = focus + Vec3::new(0.0, self.settings.focus_height, 0.0);
        camera.orbit(target, self.yaw, self.elevation, self.settings.orbit);
        camera.orthographic_height = Some(self.distance);
        camera
    }

    /// The flat directions "up the screen" and "right" to walk in, from the bearing the view is
    /// turning to (so they change the moment the player turns it).
    pub fn walk_axes(&self) -> (Vec3, Vec3) {
        let (sin_yaw, cos_yaw) = self.yaw_target.sin_cos();
        (
            Vec3::new(-sin_yaw, 0.0, -cos_yaw),
            Vec3::new(cos_yaw, 0.0, -sin_yaw),
        )
    }
}

/// A straight wall of a plan of rooms: along x (lying at `z = at`) or along z (lying at
/// `x = at`), from `a` to `b` along its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanWall3D {
    /// Whether it runs along x (else along z).
    pub along_x: bool,
    /// Where it lies across its length.
    pub at: f32,
    /// Where it starts along its length.
    pub a: f32,
    /// Where it ends along its length.
    pub b: f32,
}

/// A room of a plan of rooms: `x0..x1` by `z0..z1`. An open room (a balcony) has no walls of
/// its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanRoom3D {
    /// West side.
    pub x0: f32,
    /// East side.
    pub x1: f32,
    /// North side.
    pub z0: f32,
    /// South side.
    pub z1: f32,
    /// Whether it is open (walled only by the room it hangs from).
    pub open: bool,
}

impl PlanRoom3D {
    /// Whether `wall` stands between this room and a camera looking from bearing `yaw`, so a
    /// cutaway view drops it: it lies on the room's side towards the camera. A room's own walls
    /// lie within its sides; for an open room, the wall of the room it hangs from is what would
    /// hide it.
    pub fn wall_faces_camera(&self, wall: &PlanWall3D, yaw: f32) -> bool {
        let (sx, sz) = yaw.sin_cos();
        let along = |a: f32, b: f32, lo: f32, hi: f32| {
            if self.open {
                b > lo + 0.01 && a < hi - 0.01
            } else {
                a >= lo - 0.01 && b <= hi + 0.01
            }
        };
        if wall.along_x && along(wall.a, wall.b, self.x0, self.x1) {
            return (wall.at == self.z1 && sz > 0.25) || (wall.at == self.z0 && sz < -0.25);
        }
        if !wall.along_x && along(wall.a, wall.b, self.z0, self.z1) {
            return (wall.at == self.x1 && sx > 0.25) || (wall.at == self.x0 && sx < -0.25);
        }
        false
    }
}

/// How far a wall drops each second towards where it is going (a share of the way left).
const DROP_RATE: f32 = 7.0;
/// How far down a wall may be and still wear what hangs on it.
const DRESSED_UNTIL: f32 = 0.35;

/// The walls of a cutaway view: each drops to a stub while it is in the way and rises again
/// when it is not, over a few frames.
#[derive(Clone, Debug, PartialEq)]
pub struct CutawayWalls3D {
    /// How far down each wall is (0 standing .. 1 dropped).
    lowered: Vec<f32>,
    /// How low a dropped wall goes (a share of its height).
    stub: f32,
}

impl CutawayWalls3D {
    /// `count` walls, all standing; dropped, they keep `stub` of their height.
    pub fn new(count: usize, stub: f32) -> Self {
        Self {
            lowered: vec![0.0; count],
            stub,
        }
    }

    /// Each wall drops a step further while `in_the_way(i)`, and rises a step otherwise.
    pub fn update(&mut self, dt: f32, mut in_the_way: impl FnMut(usize) -> bool) {
        for (i, lowered) in self.lowered.iter_mut().enumerate() {
            let target = if in_the_way(i) { 1.0 } else { 0.0 };
            *lowered += (target - *lowered) * (dt * DROP_RATE).min(1.0);
        }
    }

    /// How tall wall `i` stands now (1 whole .. the stub).
    pub fn scale(&self, i: usize) -> f32 {
        1.0 - (1.0 - self.stub) * self.lowered.get(i).copied().unwrap_or(0.0)
    }

    /// Whether what hangs on wall `i` shows (not once it is well on its way down).
    pub fn dressed(&self, i: usize) -> bool {
        self.lowered.get(i).copied().unwrap_or(0.0) < DRESSED_UNTIL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> IsoCamera3D {
        IsoCamera3D::new(IsoSettings3D::DEFAULT, std::f32::consts::FRAC_PI_4)
    }

    #[test]
    fn a_turn_eases_the_short_way_round() {
        let mut iso = IsoCamera3D::new(IsoSettings3D::DEFAULT, 0.1);
        iso.turn(-1);
        assert!(
            (iso.yaw_target - (0.1 - PI / 12.0).rem_euclid(TAU)).abs() < 1e-6,
            "turns wrap round"
        );
        assert!(iso.yaw_target > PI, "{}", iso.yaw_target);
        let mut last = iso.yaw;
        for _ in 0..30 {
            iso.ease(1.0 / 60.0);
            assert!(
                iso.yaw < last + 1e-6,
                "it goes down through zero, not up round the circle"
            );
            last = iso.yaw;
        }
        let settled = (iso.yaw - iso.yaw_target).rem_euclid(TAU);
        assert!(
            !(0.01..=TAU - 0.01).contains(&settled),
            "settled in half a second: {}",
            iso.yaw
        );
        iso.turn(1);
        iso.turn(1);
        assert!((iso.yaw_target - (0.1 + PI / 12.0)).abs() < 1e-5);
    }

    #[test]
    fn it_tilts_top_down_and_back_and_zooms_within_its_range() {
        let mut iso = view();
        iso.toggle_top_down();
        for _ in 0..60 {
            iso.ease(1.0 / 60.0);
        }
        assert!((iso.elevation - 1.42).abs() < 1e-3);
        iso.toggle_top_down();
        for _ in 0..60 {
            iso.ease(1.0 / 60.0);
        }
        assert!((iso.elevation - 0.615_479_7).abs() < 1e-3);
        for _ in 0..100 {
            iso.zoom_in(0.4);
        }
        assert_eq!(iso.distance, 3.8);
        for _ in 0..100 {
            iso.zoom_out(0.4);
        }
        assert_eq!(iso.distance, 18.0);
    }

    #[test]
    fn the_view_frames_the_focus_from_its_bearing() {
        let iso = view();
        let feet = Vec3::new(2.0, 1.0, -3.0);
        let camera = iso.camera(feet);
        assert_eq!(camera.target, feet + Vec3::new(0.0, 0.78, 0.0));
        assert!(((camera.position - camera.target).length() - 18.0).abs() < 1e-3);
        assert_eq!(camera.orthographic_height, Some(18.0));
        // Walking "up the screen" goes away from the eye; "right" is square to it.
        let (ahead, right) = iso.walk_axes();
        let toward_eye = camera.position - camera.target;
        assert!(ahead.dot(toward_eye) < 0.0);
        assert!(right.dot(ahead).abs() < 1e-6 && (right.length() - 1.0).abs() < 1e-6);
        assert!(
            ahead.cross(right).y < 0.0,
            "right is on the right seen from above the screen"
        );
    }

    #[test]
    fn the_window_opens_in_what_stands_between_and_not_behind() {
        let iso = view();
        let feet = Vec3::ZERO;
        let camera = iso.camera(feet);
        let toward_eye = {
            let v = camera.position - camera.target;
            Vec3::new(v.x, 0.0, v.z).normalize_or_zero()
        };
        let sight = Sight::new(&iso.settings, &camera, feet, 0.0);
        let box_at = |centre: Vec3, height: f32| {
            (
                centre - Vec3::new(1.0, 0.0, 1.0),
                centre + Vec3::new(1.0, height, 1.0),
            )
        };
        let (lo, hi) = box_at(toward_eye * 3.0, 4.0);
        assert!(sight.hidden_by(lo, hi), "a house between her and the eye");
        let (lo, hi) = box_at(toward_eye * -3.0, 4.0);
        assert!(!sight.hidden_by(lo, hi), "a house behind her");
        let (lo, hi) = box_at(toward_eye * 3.0, 0.4);
        assert!(!sight.hidden_by(lo, hi), "a low fence between");
        let across = Vec3::new(toward_eye.z, 0.0, -toward_eye.x);
        let (lo, hi) = box_at(toward_eye * 3.0 + across * 12.0, 4.0);
        assert!(!sight.hidden_by(lo, hi), "a house off to the side");
        // Zoomed in, the window is bigger on screen.
        let mut near = iso;
        near.distance = 4.0;
        assert!(
            iso.settings.see_through_radius(&near.camera(feet))
                > iso.settings.see_through_radius(&camera)
        );
    }

    #[test]
    fn the_walls_on_the_cameras_side_of_the_room_drop() {
        let room = PlanRoom3D {
            x0: 0.0,
            x1: 10.0,
            z0: 0.0,
            z1: 8.0,
            open: false,
        };
        let south = PlanWall3D {
            along_x: true,
            at: 8.0,
            a: 0.0,
            b: 10.0,
        };
        let north = PlanWall3D {
            along_x: true,
            at: 0.0,
            a: 0.0,
            b: 10.0,
        };
        let east = PlanWall3D {
            along_x: false,
            at: 10.0,
            a: 0.0,
            b: 8.0,
        };
        let next_door = PlanWall3D {
            along_x: true,
            at: 8.0,
            a: 10.0,
            b: 20.0,
        };
        // Looking from the south-east (the eye towards +x, +z).
        let yaw = std::f32::consts::FRAC_PI_4;
        assert!(room.wall_faces_camera(&south, yaw));
        assert!(room.wall_faces_camera(&east, yaw));
        assert!(!room.wall_faces_camera(&north, yaw));
        assert!(
            !room.wall_faces_camera(&next_door, yaw),
            "a neighbour's wall stays"
        );
        // A balcony hangs off the room's north side: from the south, the room's north wall hides it.
        let balcony = PlanRoom3D {
            x0: 2.0,
            x1: 6.0,
            z0: -3.0,
            z1: 0.0,
            open: true,
        };
        assert!(balcony.wall_faces_camera(
            &PlanWall3D {
                along_x: true,
                at: 0.0,
                a: 0.0,
                b: 10.0
            },
            yaw
        ));

        let mut walls = CutawayWalls3D::new(2, 0.12);
        for _ in 0..30 {
            walls.update(1.0 / 60.0, |i| i == 0);
        }
        assert!(
            walls.scale(0) < 0.2 && walls.scale(0) >= 0.12,
            "{}",
            walls.scale(0)
        );
        assert!(!walls.dressed(0));
        assert_eq!(walls.scale(1), 1.0);
        assert!(walls.dressed(1));
        for _ in 0..60 {
            walls.update(1.0 / 60.0, |_| false);
        }
        assert!(
            walls.scale(0) > 0.98 && walls.dressed(0),
            "it stands again once out of the way"
        );
    }
}
