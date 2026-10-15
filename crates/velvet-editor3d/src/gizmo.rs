//! What the viewport draws over the 3D image and how the transform handles answer the mouse:
//! the selection box, the move / rotate / scale gizmo, light and camera icons and the view axes.

use std::f32::consts::{PI, TAU};

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};
use velvet_math::{Mat4, Vec3};
use velvet_render::Camera3D;

use crate::theme;
use crate::viewport::{self, Aabb, BOX_EDGES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Move,
    Rotate,
    Scale,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Seleccionar",
            Tool::Move => "Mover",
            Tool::Rotate => "Rotar",
            Tool::Scale => "Escalar",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Tool::Select => "Q",
            Tool::Move => "W",
            Tool::Rotate => "E",
            Tool::Scale => "R",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Move along an axis.
    Axis(usize),
    /// Move in the plane whose normal is this axis.
    Plane(usize),
    /// Turn about an axis.
    Ring(usize),
    /// Stretch along an axis.
    ScaleAxis(usize),
    /// Grow or shrink evenly.
    ScaleAll,
}

/// The arms' length on screen.
pub const ARM_PX: f32 = 96.0;
const PICK_PX: f32 = 8.0;

/// The camera and where the viewport sits on screen.
pub struct View {
    pub cam: Camera3D,
    pub vp: Mat4,
    pub rect: Rect,
    forward: Vec3,
}

impl View {
    pub fn new(cam: Camera3D, rect: Rect) -> Self {
        let aspect = rect.width() / rect.height().max(1.0);
        let (forward, _, _) = viewport::basis(&cam);
        Self {
            cam,
            vp: cam.view_projection(aspect),
            rect,
            forward,
        }
    }
    pub fn project(&self, p: Vec3) -> Option<Pos2> {
        if (p - self.cam.position).dot(self.forward) < self.cam.near {
            return None;
        }
        viewport::project(&self.vp, p, self.rect)
    }
    /// A segment clipped to the space in front of the camera, on screen.
    pub fn segment(&self, a: Vec3, b: Vec3) -> Option<[Pos2; 2]> {
        let near = self.cam.near.max(0.01) * 1.5;
        let da = (a - self.cam.position).dot(self.forward) - near;
        let db = (b - self.cam.position).dot(self.forward) - near;
        if da < 0.0 && db < 0.0 {
            return None;
        }
        let cut = a + (b - a) * (da / (da - db));
        let (a, b) = if da < 0.0 {
            (cut, b)
        } else if db < 0.0 {
            (a, cut)
        } else {
            (a, b)
        };
        Some([
            viewport::project(&self.vp, a, self.rect)?,
            viewport::project(&self.vp, b, self.rect)?,
        ])
    }
    /// World length of a gizmo arm at `at`.
    pub fn arm(&self, at: Vec3) -> f32 {
        viewport::world_per_pixel(&self.cam, at, self.rect.height()) * ARM_PX
    }
    pub fn ray(&self, pos: Pos2) -> (Vec3, Vec3) {
        viewport::ray(&self.cam, pos, self.rect)
    }
    pub fn facing(&self, at: Vec3, dir: Vec3) -> bool {
        dir.dot(self.cam.position - at) > 0.0
    }
}

/// Where the gizmo stands and the directions of its three handles.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub center: Vec3,
    pub axes: [Vec3; 3],
    pub arm: f32,
}

// ------------------------------------------------------------------ geometry
fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

fn inside(p: Pos2, poly: &[Pos2]) -> bool {
    let mut odd = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
        j = i;
    }
    odd
}

fn plane_quad(view: &View, f: &Frame, i: usize) -> Option<[Pos2; 4]> {
    let (u, v) = (f.axes[(i + 1) % 3], f.axes[(i + 2) % 3]);
    let (a, b) = (0.26 * f.arm, 0.46 * f.arm);
    let c = f.center;
    Some([
        view.project(c + u * a + v * a)?,
        view.project(c + u * b + v * a)?,
        view.project(c + u * b + v * b)?,
        view.project(c + u * a + v * b)?,
    ])
}

/// The ring about axis `i` as world points (closed).
fn ring(f: &Frame, i: usize) -> Vec<Vec3> {
    let n = f.axes[i];
    let helper = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = n.cross(helper).normalize_or_zero();
    let v = n.cross(u).normalize_or_zero();
    (0..=72)
        .map(|k| {
            let a = k as f32 / 72.0 * TAU;
            f.center + (u * a.cos() + v * a.sin()) * (f.arm * 0.86)
        })
        .collect()
}

fn arm_end(view: &View, f: &Frame, i: usize) -> Option<(Pos2, Pos2)> {
    let a = view.project(f.center + f.axes[i] * (f.arm * 0.16))?;
    let b = view.project(f.center + f.axes[i] * f.arm)?;
    ((b - a).length() > 6.0).then_some((a, b))
}

/// The handle under the pointer, if any.
pub fn hit(view: &View, f: &Frame, tool: Tool, p: Pos2) -> Option<Handle> {
    match tool {
        Tool::Select => None,
        Tool::Move => {
            for i in 0..3 {
                if plane_quad(view, f, i).is_some_and(|q| inside(p, &q)) {
                    return Some(Handle::Plane(i));
                }
            }
            nearest_arm(view, f, p).map(Handle::Axis)
        }
        Tool::Scale => {
            let c = view.project(f.center)?;
            if (p - c).length() < 9.0 {
                return Some(Handle::ScaleAll);
            }
            nearest_arm(view, f, p).map(Handle::ScaleAxis)
        }
        Tool::Rotate => {
            let mut best = (PICK_PX, None);
            for i in 0..3 {
                let pts = ring(f, i);
                for w in pts.windows(2) {
                    if let Some([a, b]) = view.segment(w[0], w[1]) {
                        let d = dist_to_segment(p, a, b);
                        if d < best.0 {
                            best = (d, Some(Handle::Ring(i)));
                        }
                    }
                }
            }
            best.1
        }
    }
}

fn nearest_arm(view: &View, f: &Frame, p: Pos2) -> Option<usize> {
    let mut best = (PICK_PX, None);
    for i in 0..3 {
        if let Some((a, b)) = arm_end(view, f, i) {
            let d = dist_to_segment(p, a, b);
            if d < best.0 {
                best = (d, Some(i));
            }
        }
    }
    best.1
}

fn hot_color(c: Color32, hot: bool) -> Color32 {
    if hot {
        c.lerp_to_gamma(Color32::WHITE, 0.45)
    } else {
        c
    }
}

/// Draws the gizmo for `tool`, `hot` brighter.
pub fn draw(p: &Painter, view: &View, f: &Frame, tool: Tool, hot: Option<Handle>) {
    let shadow = Color32::from_black_alpha(110);
    match tool {
        Tool::Select => {}
        Tool::Move => {
            for i in 0..3 {
                if let Some(q) = plane_quad(view, f, i) {
                    let is_hot = hot == Some(Handle::Plane(i));
                    let c = theme::AXES[i].gamma_multiply(if is_hot { 0.85 } else { 0.42 });
                    p.add(Shape::convex_polygon(
                        q.to_vec(),
                        c,
                        Stroke::new(1.0_f32, theme::AXES[i]),
                    ));
                }
            }
            for i in order_by_depth(view, f) {
                if let Some((a, b)) = arm_end(view, f, i) {
                    let is_hot = hot == Some(Handle::Axis(i));
                    let c = hot_color(theme::AXES[i], is_hot);
                    let w = if is_hot { 4.0_f32 } else { 3.0_f32 };
                    p.line_segment([a, b], Stroke::new(w + 2.0_f32, shadow));
                    p.line_segment([a, b], Stroke::new(w, c));
                    let dir = (b - a).normalized();
                    let side = egui::vec2(-dir.y, dir.x);
                    let tip = b + dir * 14.0;
                    p.add(Shape::convex_polygon(
                        vec![tip, b + side * 6.5, b - side * 6.5],
                        c,
                        Stroke::new(1.0_f32, shadow),
                    ));
                }
            }
            if let Some(c) = view.project(f.center) {
                p.circle_filled(c, 4.0, Color32::WHITE);
            }
        }
        Tool::Rotate => {
            for i in order_by_depth(view, f) {
                let is_hot = hot == Some(Handle::Ring(i));
                let c = hot_color(theme::AXES[i], is_hot);
                let pts = ring(f, i);
                for w in pts.windows(2) {
                    let mid = (w[0] + w[1]) * 0.5;
                    let front = view.facing(f.center, mid - f.center);
                    if let Some([a, b]) = view.segment(w[0], w[1]) {
                        let (col, width) = if front {
                            (c, if is_hot { 4.0_f32 } else { 2.6_f32 })
                        } else {
                            (c.gamma_multiply(0.32), 1.6_f32)
                        };
                        p.line_segment([a, b], Stroke::new(width, col));
                    }
                }
            }
            if let Some(c) = view.project(f.center) {
                p.circle_stroke(c, 5.0, Stroke::new(1.5_f32, Color32::WHITE));
            }
        }
        Tool::Scale => {
            for i in order_by_depth(view, f) {
                if let Some((a, b)) = arm_end(view, f, i) {
                    let is_hot = hot == Some(Handle::ScaleAxis(i));
                    let c = hot_color(theme::AXES[i], is_hot);
                    let w = if is_hot { 4.0_f32 } else { 3.0_f32 };
                    p.line_segment([a, b], Stroke::new(w + 2.0_f32, shadow));
                    p.line_segment([a, b], Stroke::new(w, c));
                    let r = Rect::from_center_size(b, egui::vec2(11.0, 11.0));
                    p.rect_filled(r, 1.5, c);
                    p.rect_stroke(
                        r,
                        1.5,
                        Stroke::new(1.0_f32, shadow),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            if let Some(c) = view.project(f.center) {
                let is_hot = hot == Some(Handle::ScaleAll);
                let r = Rect::from_center_size(c, egui::vec2(12.0, 12.0));
                p.rect_filled(
                    r,
                    2.0,
                    if is_hot {
                        Color32::WHITE
                    } else {
                        Color32::from_gray(210)
                    },
                );
                p.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.0_f32, shadow),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }
}

/// Axis indices from the farthest to the nearest, so the near ones draw on top.
fn order_by_depth(view: &View, f: &Frame) -> [usize; 3] {
    let mut idx = [0usize, 1, 2];
    let depth = |i: usize| (f.center + f.axes[i] * f.arm - view.cam.position).length();
    idx.sort_by(|a, b| depth(*b).total_cmp(&depth(*a)));
    idx
}

// ------------------------------------------------------------------ drags
/// How far along `axis` (world units) a drag from `from` to `to` moves, measured on screen.
pub fn axis_drag(
    view: &View,
    center: Vec3,
    axis: Vec3,
    arm: f32,
    from: Pos2,
    to: Pos2,
) -> Option<f32> {
    let s0 = view.project(center)?;
    let s1 = view.project(center + axis * arm)?;
    let d = s1 - s0;
    let len = d.length();
    if len < 4.0 {
        return None;
    }
    Some((to - from).dot(d / len) / len * arm)
}

/// Where the ray through `pos` meets the plane through `point` with `normal`.
pub fn plane_hit(view: &View, pos: Pos2, point: Vec3, normal: Vec3) -> Option<Vec3> {
    let (o, d) = view.ray(pos);
    let denom = d.dot(normal);
    if denom.abs() < 1e-5 {
        return None;
    }
    let t = (point - o).dot(normal) / denom;
    (t > 0.0).then(|| o + d * t)
}

/// The angle of `pos` around the projected `center`, counter-clockwise as seen on screen.
pub fn screen_angle(view: &View, center: Vec3, pos: Pos2) -> Option<f32> {
    let c = view.project(center)?;
    Some((-(pos.y - c.y)).atan2(pos.x - c.x))
}

pub fn wrap_angle(a: f32) -> f32 {
    let mut a = a % TAU;
    if a > PI {
        a -= TAU;
    } else if a <= -PI {
        a += TAU;
    }
    a
}

pub fn snap(v: f32, step: f32) -> f32 {
    if step > 0.0 {
        (v / step).round() * step
    } else {
        v
    }
}

/// The three directions each Euler angle turns about (`local = T·Rz·Ry·Rx·S` under `parent`):
/// changing Z turns about the parent's Z, Y about Rz·Y and X about Rz·Ry·X.
pub fn euler_axes(parent: &Mat4, rotation_deg: [f32; 3]) -> [Vec3; 3] {
    let r = rotation_deg.map(f32::to_radians);
    let rz = Mat4::from_rotation_z(r[2]);
    let ry = Mat4::from_rotation_y(r[1]);
    let z = parent.transform_vector3(Vec3::Z).normalize_or_zero();
    let y = (*parent * rz)
        .transform_vector3(Vec3::Y)
        .normalize_or_zero();
    let x = (*parent * rz * ry)
        .transform_vector3(Vec3::X)
        .normalize_or_zero();
    [x, y, z]
}

/// The node's own axes in the world (its matrix columns).
pub fn local_axes(world: &Mat4) -> [Vec3; 3] {
    let col = |c: [f32; 4]| Vec3::new(c[0], c[1], c[2]).normalize_or_zero();
    let axes = [col(world.x_axis), col(world.y_axis), col(world.z_axis)];
    if axes.iter().any(|a| a.length() < 0.5) {
        [Vec3::X, Vec3::Y, Vec3::Z]
    } else {
        axes
    }
}

// ------------------------------------------------------------------ overlays
pub fn draw_box(p: &Painter, view: &View, b: &Aabb, stroke: Stroke) {
    if !b.is_valid() {
        return;
    }
    let c = b.corners();
    for (i, j) in BOX_EDGES {
        if let Some([a, bb]) = view.segment(c[i], c[j]) {
            p.line_segment([a, bb], stroke);
        }
    }
}

/// Little axes at an empty node's origin.
pub fn draw_origin(p: &Painter, view: &View, at: Vec3, axes: [Vec3; 3], len: f32) {
    for (i, a) in axes.iter().enumerate() {
        if let Some([s, e]) = view.segment(at, at + *a * len) {
            p.line_segment([s, e], Stroke::new(2.0_f32, theme::AXES[i]));
        }
    }
}

/// A bulb for a point light; returns where it was drawn.
pub fn draw_light(
    p: &Painter,
    view: &View,
    at: Vec3,
    color: [f32; 3],
    selected: bool,
) -> Option<Pos2> {
    let c = view.project(at)?;
    let col = Color32::from_rgb(
        (color[0].clamp(0.0, 1.0) * 255.0) as u8,
        (color[1].clamp(0.0, 1.0) * 255.0) as u8,
        (color[2].clamp(0.0, 1.0) * 255.0) as u8,
    );
    let ring = if selected {
        theme::SELECT
    } else {
        Color32::from_gray(235)
    };
    p.circle_filled(c, 9.0, Color32::from_black_alpha(140));
    p.circle_filled(c, 6.0, col);
    p.circle_stroke(c, 6.5, Stroke::new(1.5_f32, ring));
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let d = egui::vec2(a.cos(), a.sin());
        p.line_segment([c + d * 9.5, c + d * 13.0], Stroke::new(1.5_f32, ring));
    }
    Some(c)
}

/// A wire sphere: three circles of `radius` around `at`.
pub fn draw_sphere(p: &Painter, view: &View, at: Vec3, radius: f32, color: Color32) {
    for (u, v) in [(Vec3::X, Vec3::Z), (Vec3::X, Vec3::Y), (Vec3::Z, Vec3::Y)] {
        let pts: Vec<Vec3> = (0..=48)
            .map(|k| {
                let a = k as f32 / 48.0 * TAU;
                at + (u * a.cos() + v * a.sin()) * radius
            })
            .collect();
        for w in pts.windows(2) {
            if let Some([a, b]) = view.segment(w[0], w[1]) {
                p.line_segment([a, b], Stroke::new(1.0_f32, color));
            }
        }
    }
}

/// The scene camera as a small frustum; returns where its eye is on screen.
pub fn draw_camera(
    p: &Painter,
    view: &View,
    cam: &Camera3D,
    aspect: f32,
    selected: bool,
) -> Option<Pos2> {
    let (f, r, u) = viewport::basis(cam);
    let eye = cam.position;
    let d = 0.7;
    let h = (cam.fov_y_radians * 0.5).tan() * d;
    let w = h * aspect;
    let base = eye + f * d;
    let corners = [
        base + r * w + u * h,
        base - r * w + u * h,
        base - r * w - u * h,
        base + r * w - u * h,
    ];
    let col = if selected {
        theme::SELECT
    } else {
        Color32::from_gray(200)
    };
    let s = Stroke::new(if selected { 2.0_f32 } else { 1.3_f32 }, col);
    for (i, c) in corners.iter().enumerate() {
        if let Some(seg) = view.segment(eye, *c) {
            p.line_segment(seg, s);
        }
        if let Some(seg) = view.segment(*c, corners[(i + 1) % 4]) {
            p.line_segment(seg, s);
        }
    }
    // The "up" notch.
    let top = base + u * (h * 1.45);
    if let (Some(a), Some(b), Some(c)) = (
        view.project(corners[0] * 0.7 + corners[1] * 0.3),
        view.project(top),
        view.project(corners[0] * 0.3 + corners[1] * 0.7),
    ) {
        p.add(Shape::convex_polygon(
            vec![a, b, c],
            col.gamma_multiply(0.6),
            Stroke::NONE,
        ));
    }
    view.project(eye)
}

/// One bubble of the view-axes widget: where it is, its axis and which end.
pub struct AxisBubble {
    pub pos: Pos2,
    pub axis: usize,
    pub positive: bool,
}

/// The orientation widget in a corner: X/Y/Z as the camera sees them. Clicking a bubble looks
/// along that axis (the app handles it).
pub fn draw_view_axes(
    p: &Painter,
    cam: &Camera3D,
    center: Pos2,
    hover: Option<Pos2>,
) -> Vec<AxisBubble> {
    let (f, r, u) = viewport::basis(cam);
    let mut items = Vec::new();
    for (i, a) in [Vec3::X, Vec3::Y, Vec3::Z].iter().enumerate() {
        for positive in [true, false] {
            let a = if positive { *a } else { -*a };
            let off = egui::vec2(a.dot(r), -a.dot(u)) * 34.0;
            items.push((a.dot(f), center + off, i, positive));
        }
    }
    let near = hover.is_some_and(|h| (h - center).length() < 50.0);
    if near {
        p.circle_filled(center, 48.0, Color32::from_white_alpha(14));
    }
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out = Vec::new();
    for (_, pos, i, positive) in items {
        let c = theme::AXES[i];
        let hot = hover.is_some_and(|h| (h - pos).length() < 10.0);
        if positive {
            p.line_segment([center, pos], Stroke::new(2.0_f32, c));
            p.circle_filled(
                pos,
                9.0,
                if hot {
                    c.lerp_to_gamma(Color32::WHITE, 0.4)
                } else {
                    c
                },
            );
            p.text(
                pos,
                Align2::CENTER_CENTER,
                ["X", "Y", "Z"][i],
                FontId::proportional(11.0),
                Color32::from_gray(20),
            );
        } else {
            p.circle_filled(pos, 7.0, c.gamma_multiply(if hot { 0.8 } else { 0.38 }));
            p.circle_stroke(pos, 7.0, Stroke::new(1.0_f32, c.gamma_multiply(0.8)));
        }
        out.push(AxisBubble {
            pos,
            axis: i,
            positive,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euler_axes_match_the_rotation_order() {
        let axes = euler_axes(&Mat4::IDENTITY, [0.0, 0.0, 0.0]);
        assert!((axes[0] - Vec3::X).length() < 1e-5);
        // A quarter turn about Z swings the Y ring's axis onto -X.
        let axes = euler_axes(&Mat4::IDENTITY, [0.0, 0.0, 90.0]);
        assert!(
            (axes[1] - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-4,
            "{:?}",
            axes[1]
        );
        assert!(wrap_angle(3.5 * PI).abs() - PI < 1e-4);
        assert_eq!(snap(0.37, 0.25), 0.25);
    }
}
