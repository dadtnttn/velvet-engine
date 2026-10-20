//! A stage: pictures, the cards that show them (in the world or on the screen), the camera, the lights and the haze;
//! where the pointer lands, and where a point of the world shows on the screen.

use std::collections::HashMap;

use velvet_canvas::Canvas;
use velvet_math::{Mat4, Vec2, Vec3};

use crate::camera::{Camera, Ray};

/// A picture of the stage (a painted canvas).
pub type PictureId = u64;
/// A card of the stage.
pub type CardId = u64;

/// Where a card lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// In the 3D world, seen through the camera.
    World,
    /// On the screen, in logical pixels, over the world (the HUD).
    Screen,
}

/// How a card is laid over what is behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Over, by its opacity.
    Normal,
    /// Added (glows, light).
    Add,
}

/// A flat card showing a picture (or part of one).
///
/// In the world a card lies in its own XY plane, facing +Z; its picture's top is +Y. The point of the picture at
/// `anchor` (0 to 1 across and down) sits at `position`; the card is then turned (degrees around X, then Y... applied
/// as yaw · pitch · roll) and scaled. A card with a parent is placed in its parent's space: a point of the parent's
/// picture is found with [`Stage::on_card`].
///
/// On the screen, `position` is in logical pixels (y down), `size` too, and only `rotation.z` turns it.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub picture: PictureId,
    pub space: Space,
    /// Its size: world units, or logical pixels on the screen.
    pub size: Vec2,
    pub anchor: Vec2,
    pub position: Vec3,
    /// Degrees: x tilts it (pitch), y turns it (yaw), z rolls it.
    pub rotation: Vec3,
    pub scale: f32,
    pub parent: Option<CardId>,
    /// Multiplies the picture; its alpha is the card's opacity.
    pub tint: [f32; 4],
    /// Lights and haze act on it (world cards).
    pub lit: bool,
    pub blend: Blend,
    /// Drawn after the cards of lower order (in the world, then by distance: farther first).
    pub order: i32,
    pub visible: bool,
    /// Found by [`Stage::pick`].
    pub pickable: bool,
    /// The part of the picture shown: u0, v0, u1, v1.
    pub uv: [f32; 4],
}

impl Card {
    /// A card in the world, centred on the origin, lit, not pickable.
    pub fn world(picture: PictureId, width: f32, height: f32) -> Card {
        Card {
            picture,
            space: Space::World,
            size: Vec2::new(width, height),
            anchor: Vec2::new(0.5, 0.5),
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: 1.0,
            parent: None,
            tint: [1.0; 4],
            lit: true,
            blend: Blend::Normal,
            order: 0,
            visible: true,
            pickable: false,
            uv: [0.0, 0.0, 1.0, 1.0],
        }
    }

    /// A card on the screen with its top-left corner at a logical point.
    pub fn screen(picture: PictureId, x: f32, y: f32, width: f32, height: f32) -> Card {
        Card { space: Space::Screen, anchor: Vec2::new(0.0, 0.0), position: Vec3::new(x, y, 0.0), lit: false, ..Card::world(picture, width, height) }
    }

    pub fn at(mut self, position: Vec3) -> Card {
        self.position = position;
        self
    }

    pub fn turned(mut self, pitch: f32, yaw: f32, roll: f32) -> Card {
        self.rotation = Vec3::new(pitch, yaw, roll);
        self
    }

    pub fn anchored(mut self, x: f32, y: f32) -> Card {
        self.anchor = Vec2::new(x, y);
        self
    }

    pub fn ordered(mut self, order: i32) -> Card {
        self.order = order;
        self
    }

    pub fn pickable(mut self) -> Card {
        self.pickable = true;
        self
    }

    pub fn unlit(mut self) -> Card {
        self.lit = false;
        self
    }

    pub fn additive(mut self) -> Card {
        self.blend = Blend::Add;
        self.lit = false;
        self
    }

    pub fn tinted(mut self, tint: [f32; 4]) -> Card {
        self.tint = tint;
        self
    }

    pub fn child_of(mut self, parent: CardId) -> Card {
        self.parent = Some(parent);
        self
    }

    /// The card's own transform (without its parent's).
    pub fn local_matrix(&self) -> Mat4 {
        let r = self.rotation;
        Mat4::from_translation(self.position)
            * Mat4::from_rotation_y(r.y.to_radians())
            * Mat4::from_rotation_x(r.x.to_radians())
            * Mat4::from_rotation_z(r.z.to_radians())
            * Mat4::from_scale(Vec3::new(self.scale, self.scale, self.scale))
    }

    /// A point of the card's face (0 to 1 across and down) in the card's own space.
    pub fn face_point(&self, u: f32, v: f32) -> Vec3 {
        match self.space {
            Space::World => Vec3::new((u - self.anchor.x) * self.size.x, (self.anchor.y - v) * self.size.y, 0.0),
            Space::Screen => Vec3::new((u - self.anchor.x) * self.size.x, (v - self.anchor.y) * self.size.y, 0.0),
        }
    }
}

/// A light: a point that lights the world cards around it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub position: Vec3,
    /// Linear colour.
    pub color: [f32; 3],
    pub intensity: f32,
    /// Past this distance it lights nothing.
    pub range: f32,
}

/// The haze that fades far things towards a colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    pub color: [f32; 3],
    /// Where it starts, and where it hides everything.
    pub start: f32,
    pub end: f32,
}

/// The most lights a stage uses (the nearest to the camera's target win).
pub const MAX_LIGHTS: usize = 16;

/// A picture and how many times it has been painted.
#[derive(Debug)]
pub struct Picture {
    pub canvas: Canvas,
    pub version: u64,
}

/// Where the pointer landed on a card.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub card: CardId,
    /// On the card's face, 0 to 1 across and down (outside for [`Stage::pick_on`] off the card).
    pub uv: Vec2,
    /// The point of the card's picture, in the picture's logical units.
    pub point: Vec2,
    /// The point in the world (on the screen for a screen card).
    pub world: Vec3,
    /// How far from the eye (0 for a screen card).
    pub distance: f32,
}

/// The pictures, the cards, the camera and the lights.
#[derive(Debug)]
pub struct Stage {
    pub camera: Camera,
    /// The screen's logical size (the HUD's coordinates and the pointer's).
    pub screen: Vec2,
    /// The colour behind everything (sRGB).
    pub clear: [f32; 3],
    /// The light everywhere, before the lights (linear).
    pub ambient: [f32; 3],
    pub lights: Vec<Light>,
    pub fog: Option<Fog>,
    cards: Vec<(CardId, Card)>,
    pictures: HashMap<PictureId, Picture>,
    next_picture: PictureId,
    next_card: CardId,
    version: u64,
}

impl Stage {
    /// An empty stage for a screen of a logical size.
    pub fn new(width: f32, height: f32) -> Stage {
        Stage {
            camera: Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO),
            screen: Vec2::new(width, height),
            clear: [0.0, 0.0, 0.0],
            ambient: [1.0, 1.0, 1.0],
            lights: Vec::new(),
            fog: None,
            cards: Vec::new(),
            pictures: HashMap::new(),
            next_picture: 1,
            next_card: 1,
            version: 0,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.screen.x / self.screen.y.max(1.0)
    }

    // ---- pictures ---------------------------------------------------------------------------------------------------

    pub fn add_picture(&mut self, canvas: Canvas) -> PictureId {
        let id = self.next_picture;
        self.next_picture += 1;
        self.version += 1;
        self.pictures.insert(id, Picture { canvas, version: self.version });
        id
    }

    /// Paints a picture again (its cards show the new one).
    pub fn set_picture(&mut self, id: PictureId, canvas: Canvas) {
        self.version += 1;
        self.pictures.insert(id, Picture { canvas, version: self.version });
    }

    pub fn remove_picture(&mut self, id: PictureId) {
        self.pictures.remove(&id);
    }

    pub fn picture(&self, id: PictureId) -> Option<&Canvas> {
        self.pictures.get(&id).map(|p| &p.canvas)
    }

    pub fn pictures(&self) -> impl Iterator<Item = (PictureId, &Picture)> {
        self.pictures.iter().map(|(id, p)| (*id, p))
    }

    // ---- cards ------------------------------------------------------------------------------------------------------

    pub fn add(&mut self, card: Card) -> CardId {
        let id = self.next_card;
        self.next_card += 1;
        self.cards.push((id, card));
        id
    }

    pub fn card(&self, id: CardId) -> Option<&Card> {
        self.cards.iter().find(|(c, _)| *c == id).map(|(_, card)| card)
    }

    pub fn card_mut(&mut self, id: CardId) -> Option<&mut Card> {
        self.cards.iter_mut().find(|(c, _)| *c == id).map(|(_, card)| card)
    }

    /// Takes a card and its children off the stage.
    pub fn remove(&mut self, id: CardId) {
        let children: Vec<CardId> = self.cards.iter().filter(|(_, c)| c.parent == Some(id)).map(|(c, _)| *c).collect();
        for child in children {
            self.remove(child);
        }
        self.cards.retain(|(c, _)| *c != id);
    }

    /// The cards, in the order they were added.
    pub fn cards(&self) -> impl Iterator<Item = (CardId, &Card)> {
        self.cards.iter().map(|(id, c)| (*id, c))
    }

    /// The card's transform in its space (with its parents'), or None when a parent is missing.
    pub fn matrix(&self, id: CardId) -> Option<Mat4> {
        let card = self.card(id)?;
        let local = card.local_matrix();
        match card.parent {
            Some(parent) => Some(self.matrix(parent)? * local),
            None => Some(local),
        }
    }

    /// Shown: visible, and every parent visible.
    pub fn shown(&self, id: CardId) -> bool {
        match self.card(id) {
            Some(card) => card.visible && card.parent.is_none_or(|p| self.shown(p)),
            None => false,
        }
    }

    /// A point of a card's picture (in the picture's logical units) in the card's own space, lifted off its face by
    /// `lift` (in the card's units): where to put a child card.
    pub fn on_card(&self, id: CardId, point: Vec2, lift: f32) -> Option<Vec3> {
        let card = self.card(id)?;
        let canvas = self.picture(card.picture)?;
        let [u0, v0, u1, v1] = card.uv;
        let u = (point.x / canvas.width() - u0) / (u1 - u0).max(1e-6);
        let v = (point.y / canvas.height() - v0) / (v1 - v0).max(1e-6);
        let p = card.face_point(u, v);
        let lift = if card.space == Space::World { lift } else { 0.0 };
        Some(Vec3::new(p.x, p.y, lift))
    }

    /// A point of a card's picture in the world (or on the screen, for a screen card).
    pub fn card_to_world(&self, id: CardId, point: Vec2, lift: f32) -> Option<Vec3> {
        let local = self.on_card(id, point, lift)?;
        Some(self.matrix(id)?.transform_point3(local))
    }

    // ---- the camera and the pointer ---------------------------------------------------------------------------------

    pub fn view_projection(&self) -> Mat4 {
        self.camera.view_projection(self.aspect())
    }

    /// The ray from the eye through a logical point of the screen.
    pub fn ray(&self, point: Vec2) -> Ray {
        let ndc_x = point.x / self.screen.x * 2.0 - 1.0;
        let ndc_y = 1.0 - point.y / self.screen.y * 2.0;
        let inverse = self.view_projection().inverse().unwrap_or(Mat4::IDENTITY);
        let near = inverse.transform_point3(Vec3::new(ndc_x, ndc_y, 0.0));
        let far = inverse.transform_point3(Vec3::new(ndc_x, ndc_y, 1.0));
        Ray { origin: near, direction: (far - near).normalize_or_zero() }
    }

    /// Where a point of the world shows on the screen (logical pixels), and its depth from 0 (near) to 1 (far); None
    /// behind the eye.
    pub fn project(&self, world: Vec3) -> Option<(Vec2, f32)> {
        let m = self.view_projection();
        let x = m.x_axis[0] * world.x + m.y_axis[0] * world.y + m.z_axis[0] * world.z + m.w_axis[0];
        let y = m.x_axis[1] * world.x + m.y_axis[1] * world.y + m.z_axis[1] * world.z + m.w_axis[1];
        let z = m.x_axis[2] * world.x + m.y_axis[2] * world.y + m.z_axis[2] * world.z + m.w_axis[2];
        let w = m.x_axis[3] * world.x + m.y_axis[3] * world.y + m.z_axis[3] * world.z + m.w_axis[3];
        if w <= 1e-6 {
            return None;
        }
        let (nx, ny) = (x / w, y / w);
        Some((Vec2::new((nx + 1.0) * 0.5 * self.screen.x, (1.0 - ny) * 0.5 * self.screen.y), z / w))
    }

    /// The card under a logical point of the screen: the screen cards first (the highest order on top), then the
    /// nearest world card. Only pickable, shown cards count, and only where their picture is not transparent.
    pub fn pick(&self, point: Vec2) -> Option<Hit> {
        let mut screen: Vec<(i32, usize, Hit)> = Vec::new();
        let mut world: Option<Hit> = None;
        for (index, (id, card)) in self.cards.iter().enumerate() {
            if !card.pickable || !self.shown(*id) {
                continue;
            }
            let Some(hit) = self.pick_on(*id, point) else { continue };
            if !(0.0..=1.0).contains(&hit.uv.x) || !(0.0..=1.0).contains(&hit.uv.y) || !self.solid(card, hit.point) {
                continue;
            }
            match card.space {
                Space::Screen => screen.push((card.order, index, hit)),
                Space::World => {
                    if hit.distance > 0.0 && world.is_none_or(|w| hit.distance < w.distance) {
                        world = Some(hit);
                    }
                }
            }
        }
        if let Some((_, _, hit)) = screen.into_iter().max_by_key(|(order, index, _)| (*order, *index)) {
            return Some(hit);
        }
        world
    }

    /// Where a logical point of the screen falls on a card's plane, inside the card or not (to drag across a desk
    /// past its edges). None when the ray runs along the plane or away from it.
    pub fn pick_on(&self, id: CardId, point: Vec2) -> Option<Hit> {
        let card = self.card(id)?;
        let matrix = self.matrix(id)?;
        let inverse = matrix.inverse()?;
        let (local, distance, world) = match card.space {
            Space::Screen => {
                let local = inverse.transform_point3(Vec3::new(point.x, point.y, 0.0));
                (local, 0.0, Vec3::new(point.x, point.y, 0.0))
            }
            Space::World => {
                let ray = self.ray(point);
                let origin = inverse.transform_point3(ray.origin);
                let direction = inverse.transform_vector3(ray.direction);
                if direction.z.abs() < 1e-7 {
                    return None;
                }
                let t = -origin.z / direction.z;
                if t <= 0.0 {
                    return None;
                }
                let local = origin + direction * t;
                let world = matrix.transform_point3(local);
                (local, (world - self.camera.eye).length(), world)
            }
        };
        let u = local.x / card.size.x + card.anchor.x;
        let v = match card.space {
            Space::World => card.anchor.y - local.y / card.size.y,
            Space::Screen => local.y / card.size.y + card.anchor.y,
        };
        let canvas = self.picture(card.picture)?;
        let [u0, v0, u1, v1] = card.uv;
        let point = Vec2::new((u0 + (u1 - u0) * u) * canvas.width(), (v0 + (v1 - v0) * v) * canvas.height());
        Some(Hit { card: id, uv: Vec2::new(u, v), point, world, distance })
    }

    /// True where the card's picture is not transparent.
    fn solid(&self, card: &Card, point: Vec2) -> bool {
        let Some(canvas) = self.picture(card.picture) else { return false };
        let k = canvas.k();
        let x = (point.x * k).floor().clamp(0.0, canvas.pixel_width() as f32 - 1.0) as u32;
        let y = (point.y * k).floor().clamp(0.0, canvas.pixel_height() as f32 - 1.0) as u32;
        canvas.pixel(x, y).is_some_and(|p| p[3] > 8)
    }

    /// The lights that reach the stage's view, at most [`MAX_LIGHTS`], the nearest to where the camera looks first.
    pub fn active_lights(&self) -> Vec<Light> {
        let mut lights = self.lights.clone();
        let target = self.camera.target;
        lights.sort_by(|a, b| (a.position - target).length().total_cmp(&(b.position - target).length()));
        lights.truncate(MAX_LIGHTS);
        lights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage_with_floor() -> (Stage, CardId) {
        let mut stage = Stage::new(1920.0, 1080.0);
        stage.camera = Camera::new(Vec3::new(0.0, 100.0, 100.0), Vec3::new(0.0, 0.0, 0.0));
        let picture = stage.add_picture(Canvas::new(200.0, 100.0));
        // The canvas is empty (transparent): fill it so it can be picked.
        let mut canvas = Canvas::new(200.0, 100.0);
        canvas.clear(velvet_canvas::Color::WHITE);
        stage.set_picture(picture, canvas);
        // A floor lying flat, its top towards -Z.
        let floor = stage.add(Card::world(picture, 200.0, 100.0).turned(-90.0, 0.0, 0.0).pickable());
        (stage, floor)
    }

    #[test]
    fn the_centre_of_the_screen_hits_what_the_camera_looks_at() {
        let (stage, floor) = stage_with_floor();
        let hit = stage.pick(Vec2::new(960.0, 540.0)).expect("the floor");
        assert_eq!(hit.card, floor);
        assert!(hit.world.length() < 0.01, "{:?}", hit.world);
        assert!((hit.uv.x - 0.5).abs() < 1e-3 && (hit.uv.y - 0.5).abs() < 1e-3);
        assert!((hit.point.x - 100.0).abs() < 0.2 && (hit.point.y - 50.0).abs() < 0.2);
        assert!((hit.distance - (100.0f32 * 100.0 * 2.0).sqrt()).abs() < 0.1);
    }

    #[test]
    fn a_projected_point_picks_back_to_itself() {
        let (stage, floor) = stage_with_floor();
        for (px, py) in [(20.0, 10.0), (180.0, 90.0), (100.0, 5.0)] {
            let world = stage.card_to_world(floor, Vec2::new(px, py), 0.0).unwrap();
            let (screen, depth) = stage.project(world).unwrap();
            assert!((0.0..1.0).contains(&depth));
            let hit = stage.pick_on(floor, screen).unwrap();
            assert!((hit.point.x - px).abs() < 0.05 && (hit.point.y - py).abs() < 0.05, "{px},{py} → {:?}", hit.point);
        }
        // The top of the picture is the far side of the floor.
        let near = stage.card_to_world(floor, Vec2::new(100.0, 99.0), 0.0).unwrap();
        let far = stage.card_to_world(floor, Vec2::new(100.0, 1.0), 0.0).unwrap();
        assert!(far.z < near.z);
    }

    #[test]
    fn screen_cards_come_first_and_the_highest_on_top() {
        let (mut stage, _floor) = stage_with_floor();
        let mut solid = Canvas::new(100.0, 100.0);
        solid.clear(velvet_canvas::Color::BLACK);
        let picture = stage.add_picture(solid);
        let low = stage.add(Card::screen(picture, 900.0, 500.0, 100.0, 100.0).pickable());
        let high = stage.add(Card::screen(picture, 950.0, 520.0, 100.0, 100.0).ordered(5).pickable());
        assert_eq!(stage.pick(Vec2::new(960.0, 540.0)).unwrap().card, high);
        assert_eq!(stage.pick(Vec2::new(910.0, 510.0)).unwrap().card, low);
        let hit = stage.pick(Vec2::new(1000.0, 600.0)).unwrap();
        assert_eq!(hit.card, high);
        assert!((hit.point.x - 50.0).abs() < 1e-3 && (hit.point.y - 80.0).abs() < 1e-3);
    }

    #[test]
    fn children_follow_their_parent_and_transparent_parts_are_not_picked() {
        let (mut stage, floor) = stage_with_floor();
        let mut dot = Canvas::new(10.0, 10.0);
        dot.fill_circle(velvet_canvas::Point::new(5.0, 5.0), 2.0, &velvet_canvas::Color::BLACK.into());
        let picture = stage.add_picture(dot);
        let at = stage.on_card(floor, Vec2::new(150.0, 30.0), 0.5).unwrap();
        let child = stage.add(Card::world(picture, 10.0, 10.0).at(at).child_of(floor).pickable());
        let world = stage.card_to_world(child, Vec2::new(5.0, 5.0), 0.0).unwrap();
        let expected = stage.card_to_world(floor, Vec2::new(150.0, 30.0), 0.5).unwrap();
        assert!((world - expected).length() < 1e-3, "{world:?} vs {expected:?}");
        let (screen, _) = stage.project(world).unwrap();
        assert_eq!(stage.pick(screen).unwrap().card, child);
        // The corner of the dot's picture is transparent: the floor under it is picked.
        let corner = stage.card_to_world(child, Vec2::new(0.5, 0.5), 0.0).unwrap();
        let (screen, _) = stage.project(corner).unwrap();
        assert_eq!(stage.pick(screen).unwrap().card, floor);
        // Hidden with its parent, removed with its parent.
        stage.card_mut(floor).unwrap().visible = false;
        assert!(!stage.shown(child));
        stage.remove(floor);
        assert!(stage.card(child).is_none());
    }
}
