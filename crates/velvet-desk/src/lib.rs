//! # velvet-desk
//!
//! A desk of objects for 2D games — papers, cards, tools — and what the pointer does with them. It holds no pictures
//! and draws nothing: the game draws each object where the desk says (its place, turn and scale, bottom to top) and
//! hands the desk the pointer, in the desk's own coordinates.
//!
//! - **Stacking:** the objects are kept bottom to top; the one picked up goes on top.
//! - **Dragging:** an object follows the pointer from the point where it was grabbed, and stays on the desk (at least
//!   `keep_visible` of it inside the desk's bounds).
//! - **Clicks:** a press and release that barely moved is a click, not a drag; two clicks on the same object close in
//!   time are a double click (the game decides what they do: magnify, select…).
//! - **Zones:** named areas of the desk; a drop reports the zone under the pointer (the window to hand a paper back, a
//!   tray…).
//! - **Fields:** named rectangles inside an object (a name, a date, a seal), found under the pointer even on a turned
//!   or scaled object.
//! - **Marks:** a pair of marked fields to compare. A mark can also come from outside the desk (a rule of a book, a
//!   feature seen through a window): the game adds it with its own owner id.
//!
//! ```
//! use velvet_desk::{Desk, DeskEvent, Point, Rect};
//!
//! let mut desk = Desk::new(Rect::new(0.0, 300.0, 1400.0, 700.0));
//! desk.add_item(1, 400.0, 260.0, Point::new(100.0, 400.0), 0.0);
//! desk.add_field(1, "name", Rect::new(120.0, 60.0, 240.0, 30.0));
//! desk.pointer_down(Point::new(150.0, 450.0), 0.0);
//! desk.pointer_move(Point::new(350.0, 500.0));
//! assert!(matches!(desk.pointer_up(Point::new(350.0, 500.0), 0.1), Some(DeskEvent::Dropped { item: 1, .. })));
//! assert_eq!(desk.item(1).unwrap().pos, Point::new(300.0, 450.0));
//! ```

mod geometry;

pub use geometry::{Point, Rect};

/// The id the game gives an object.
pub type ItemId = u64;

/// An object on the desk.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: ItemId,
    /// Its own size, unscaled and unturned.
    pub w: f32,
    pub h: f32,
    /// Where its top-left corner is on the desk (the turn and the scale are around that corner).
    pub pos: Point,
    /// Its turn, in degrees (clockwise on screen).
    pub rotation: f32,
    /// Its scale (1 is its own size).
    pub scale: f32,
    /// False for an object that cannot be picked up (it is still found under the pointer).
    pub draggable: bool,
    fields: Vec<Field>,
}

/// A named rectangle inside an object, in the object's own coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub rect: Rect,
}

impl Item {
    /// The fields of the object.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// A point of the desk in the object's own coordinates.
    pub fn to_local(&self, p: Point) -> Point {
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        let (dx, dy) = (p.x - self.pos.x, p.y - self.pos.y);
        // Undo the turn, then the scale.
        let x = dx * cos + dy * sin;
        let y = -dx * sin + dy * cos;
        Point::new(x / self.scale, y / self.scale)
    }

    /// A point of the object (its own coordinates) on the desk.
    pub fn to_desk(&self, p: Point) -> Point {
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        let (x, y) = (p.x * self.scale, p.y * self.scale);
        Point::new(
            self.pos.x + x * cos - y * sin,
            self.pos.y + x * sin + y * cos,
        )
    }

    /// True when a point of the desk lies on the object.
    pub fn contains(&self, p: Point) -> bool {
        let local = self.to_local(p);
        local.x >= 0.0 && local.y >= 0.0 && local.x < self.w && local.y < self.h
    }

    /// The field under a point of the desk: the last added when they overlap.
    pub fn field_at(&self, p: Point) -> Option<&Field> {
        let local = self.to_local(p);
        self.fields.iter().rev().find(|f| f.rect.contains(local))
    }

    /// The smallest rectangle of the desk around the object.
    pub fn bounds(&self) -> Rect {
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(self.w, 0.0),
            Point::new(0.0, self.h),
            Point::new(self.w, self.h),
        ]
        .map(|c| self.to_desk(c));
        let x0 = corners.iter().map(|c| c.x).fold(f32::MAX, f32::min);
        let y0 = corners.iter().map(|c| c.y).fold(f32::MAX, f32::min);
        let x1 = corners.iter().map(|c| c.x).fold(f32::MIN, f32::max);
        let y1 = corners.iter().map(|c| c.y).fold(f32::MIN, f32::max);
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

/// A named area of the desk where objects are dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub name: String,
    pub rect: Rect,
}

/// A marked field: whose it is (an object of the desk, or any owner the game gives a mark from outside), its name, and
/// where on the desk to draw the mark (the field's centre).
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub owner: ItemId,
    pub field: String,
    pub at: Point,
}

/// What the pointer did.
#[derive(Debug, Clone, PartialEq)]
pub enum DeskEvent {
    /// An object was picked up (it is now on top).
    Picked { item: ItemId },
    /// A dragged object was put down: on a zone, or on the desk.
    Dropped { item: ItemId, zone: Option<String> },
    /// An object was clicked (pressed and released without moving): the point in its own coordinates and the field
    /// there, if any.
    Clicked {
        item: ItemId,
        local: Point,
        field: Option<String>,
    },
    /// The second click of a double click on the same object.
    DoubleClicked { item: ItemId, field: Option<String> },
    /// A press on no object.
    Missed { at: Point },
}

/// The drag in progress.
#[derive(Debug, Clone)]
struct Drag {
    item: ItemId,
    /// The grabbed point, in the object's own coordinates.
    grab: Point,
    start: Point,
    moved: bool,
}

/// The desk: its bounds, its objects bottom to top, its zones, the drag in progress and the marks.
#[derive(Debug, Clone)]
pub struct Desk {
    pub bounds: Rect,
    /// How much of an object stays inside the bounds while it is dragged (a part of its width and height, 0 to 1).
    pub keep_visible: f32,
    /// How far the pointer moves before a press becomes a drag.
    pub drag_threshold: f32,
    /// The longest time between two clicks of a double click, in seconds.
    pub double_click_time: f32,
    items: Vec<Item>,
    zones: Vec<Zone>,
    drag: Option<Drag>,
    last_click: Option<(ItemId, f32)>,
    marks: Vec<Mark>,
    magnified: Option<ItemId>,
}

impl Desk {
    pub fn new(bounds: Rect) -> Desk {
        Desk {
            bounds,
            keep_visible: 0.25,
            drag_threshold: 4.0,
            double_click_time: 0.35,
            items: Vec::new(),
            zones: Vec::new(),
            drag: None,
            last_click: None,
            marks: Vec::new(),
            magnified: None,
        }
    }

    // ---- objects ----------------------------------------------------------------------------------------------------

    /// Puts an object on top of the desk (replacing one with the same id).
    pub fn add_item(&mut self, id: ItemId, w: f32, h: f32, pos: Point, rotation: f32) {
        self.remove_item(id);
        self.items.push(Item {
            id,
            w,
            h,
            pos,
            rotation,
            scale: 1.0,
            draggable: true,
            fields: Vec::new(),
        });
    }

    /// Takes an object off the desk, with its marks; its drag and its magnification end.
    pub fn remove_item(&mut self, id: ItemId) -> Option<Item> {
        let index = self.items.iter().position(|i| i.id == id)?;
        if self.drag.as_ref().is_some_and(|d| d.item == id) {
            self.drag = None;
        }
        if self.magnified == Some(id) {
            self.magnified = None;
        }
        self.marks.retain(|m| m.owner != id);
        Some(self.items.remove(index))
    }

    /// Adds a named field to an object, in its own coordinates.
    pub fn add_field(&mut self, id: ItemId, name: &str, rect: Rect) {
        if let Some(item) = self.item_mut(id) {
            item.fields.push(Field {
                name: name.to_string(),
                rect,
            });
        }
    }

    pub fn item(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// The objects, bottom to top: the order to draw them in.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Puts an object on top.
    pub fn raise(&mut self, id: ItemId) {
        if let Some(index) = self.items.iter().position(|i| i.id == id) {
            let item = self.items.remove(index);
            self.items.push(item);
        }
    }

    /// The topmost object under a point of the desk.
    pub fn item_at(&self, p: Point) -> Option<ItemId> {
        self.items
            .iter()
            .rev()
            .find(|i| i.contains(p))
            .map(|i| i.id)
    }

    /// The field under a point of the desk, on the topmost object there.
    pub fn field_at(&self, p: Point) -> Option<(ItemId, String)> {
        let item = self.items.iter().rev().find(|i| i.contains(p))?;
        item.field_at(p).map(|f| (item.id, f.name.clone()))
    }

    // ---- zones ------------------------------------------------------------------------------------------------------

    pub fn add_zone(&mut self, name: &str, rect: Rect) {
        self.zones.retain(|z| z.name != name);
        self.zones.push(Zone {
            name: name.to_string(),
            rect,
        });
    }

    pub fn zones(&self) -> &[Zone] {
        &self.zones
    }

    /// The zone under a point (the last added when they overlap).
    pub fn zone_at(&self, p: Point) -> Option<&Zone> {
        self.zones.iter().rev().find(|z| z.rect.contains(p))
    }

    // ---- the pointer ------------------------------------------------------------------------------------------------

    /// A press of the pointer at a point of the desk, at a time in seconds. Picks up the topmost object there.
    pub fn pointer_down(&mut self, p: Point, _now: f32) -> Option<DeskEvent> {
        let Some(id) = self.item_at(p) else {
            self.drag = None;
            return Some(DeskEvent::Missed { at: p });
        };
        self.raise(id);
        let item = self.item(id)?;
        self.drag = Some(Drag {
            item: id,
            grab: item.to_local(p),
            start: p,
            moved: false,
        });
        Some(DeskEvent::Picked { item: id })
    }

    /// A move of the pointer: the picked-up object follows from where it was grabbed, once the pointer has moved past
    /// the drag threshold. Returns the object that moved.
    pub fn pointer_move(&mut self, p: Point) -> Option<ItemId> {
        let drag = self.drag.as_mut()?;
        if !drag.moved && distance(drag.start, p) < self.drag_threshold {
            return None;
        }
        drag.moved = true;
        let (id, grab) = (drag.item, drag.grab);
        let (bounds, keep) = (self.bounds, self.keep_visible);
        let item = self.items.iter_mut().find(|i| i.id == id)?;
        if !item.draggable {
            return None;
        }
        // The grabbed point goes under the pointer.
        let grabbed_now = item.to_desk(grab);
        item.pos = Point::new(
            item.pos.x + p.x - grabbed_now.x,
            item.pos.y + p.y - grabbed_now.y,
        );
        // And the object stays on the desk.
        let b = item.bounds();
        let (min_w, min_h) = (b.w * keep, b.h * keep);
        let dx = if b.right() < bounds.x + min_w {
            bounds.x + min_w - b.right()
        } else if b.x > bounds.right() - min_w {
            bounds.right() - min_w - b.x
        } else {
            0.0
        };
        let dy = if b.bottom() < bounds.y + min_h {
            bounds.y + min_h - b.bottom()
        } else if b.y > bounds.bottom() - min_h {
            bounds.bottom() - min_h - b.y
        } else {
            0.0
        };
        item.pos = Point::new(item.pos.x + dx, item.pos.y + dy);
        Some(id)
    }

    /// The pointer is released: a drag drops its object (on the zone under the pointer, if any), a press that did not
    /// move clicks it (twice in time: a double click).
    pub fn pointer_up(&mut self, p: Point, now: f32) -> Option<DeskEvent> {
        let drag = self.drag.take()?;
        if drag.moved {
            self.last_click = None;
            let zone = self.zone_at(p).map(|z| z.name.clone());
            return Some(DeskEvent::Dropped {
                item: drag.item,
                zone,
            });
        }
        let item = self.item(drag.item)?;
        let local = item.to_local(p);
        let field = item.field_at(p).map(|f| f.name.clone());
        let double = matches!(self.last_click, Some((id, at)) if id == drag.item && now - at <= self.double_click_time);
        if double {
            self.last_click = None;
            return Some(DeskEvent::DoubleClicked {
                item: drag.item,
                field,
            });
        }
        self.last_click = Some((drag.item, now));
        Some(DeskEvent::Clicked {
            item: drag.item,
            local,
            field,
        })
    }

    /// The object being dragged, if any.
    pub fn dragging(&self) -> Option<ItemId> {
        self.drag.as_ref().filter(|d| d.moved).map(|d| d.item)
    }

    /// Lets go of a drag without dropping (the pointer left the window, the game opened a menu…).
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }

    // ---- magnification ----------------------------------------------------------------------------------------------

    /// Marks an object as magnified (the game draws it at reading size); None shows none.
    pub fn set_magnified(&mut self, id: Option<ItemId>) {
        self.magnified = id.filter(|id| self.item(*id).is_some());
    }

    pub fn magnified(&self) -> Option<ItemId> {
        self.magnified
    }

    // ---- marks ------------------------------------------------------------------------------------------------------

    /// Marks the field of an object of the desk (its centre is where the mark is drawn). Marking a marked field takes
    /// the mark off; a third mark replaces the oldest. Returns the marks now.
    pub fn mark_field(&mut self, id: ItemId, field: &str) -> &[Mark] {
        let at = self.item(id).and_then(|i| {
            i.fields
                .iter()
                .find(|f| f.name == field)
                .map(|f| i.to_desk(f.rect.center()))
        });
        if let Some(at) = at {
            self.mark(Mark {
                owner: id,
                field: field.to_string(),
                at,
            });
        }
        &self.marks
    }

    /// Adds a mark from anywhere (a rule of a book, a feature seen through a window): same rules as `mark_field`.
    pub fn mark(&mut self, mark: Mark) -> &[Mark] {
        if let Some(index) = self
            .marks
            .iter()
            .position(|m| m.owner == mark.owner && m.field == mark.field)
        {
            self.marks.remove(index);
            return &self.marks;
        }
        if self.marks.len() == 2 {
            self.marks.remove(0);
        }
        self.marks.push(mark);
        &self.marks
    }

    /// The marks, oldest first (at most two).
    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }

    /// The two marks to compare, when there are two.
    pub fn pair(&self) -> Option<(&Mark, &Mark)> {
        match self.marks.as_slice() {
            [a, b] => Some((a, b)),
            _ => None,
        }
    }

    /// Where the marks of desk objects are now (they follow their objects when dragged).
    pub fn refresh_marks(&mut self) {
        for i in 0..self.marks.len() {
            let (owner, field) = (self.marks[i].owner, self.marks[i].field.clone());
            if let Some(at) = self.item(owner).and_then(|item| {
                item.fields
                    .iter()
                    .find(|f| f.name == field)
                    .map(|f| item.to_desk(f.rect.center()))
            }) {
                self.marks[i].at = at;
            }
        }
    }

    pub fn clear_marks(&mut self) {
        self.marks.clear();
    }
}

fn distance(a: Point, b: Point) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desk() -> Desk {
        let mut desk = Desk::new(Rect::new(0.0, 0.0, 1000.0, 600.0));
        desk.add_item(1, 200.0, 100.0, Point::new(100.0, 100.0), 0.0);
        desk.add_item(2, 200.0, 100.0, Point::new(150.0, 120.0), 0.0);
        desk
    }

    #[test]
    fn the_topmost_object_is_found_and_picking_one_raises_it() {
        let mut d = desk();
        assert_eq!(d.item_at(Point::new(160.0, 130.0)), Some(2));
        assert_eq!(d.item_at(Point::new(110.0, 110.0)), Some(1));
        d.pointer_down(Point::new(110.0, 110.0), 0.0);
        assert_eq!(d.items().last().unwrap().id, 1);
        assert_eq!(d.item_at(Point::new(160.0, 130.0)), Some(1));
    }

    #[test]
    fn a_dragged_object_keeps_the_grabbed_point_under_the_pointer() {
        let mut d = desk();
        d.pointer_down(Point::new(110.0, 110.0), 0.0);
        assert_eq!(
            d.pointer_move(Point::new(112.0, 110.0)),
            None,
            "under the threshold"
        );
        assert_eq!(d.pointer_move(Point::new(410.0, 310.0)), Some(1));
        assert_eq!(d.item(1).unwrap().pos, Point::new(400.0, 300.0));
        assert_eq!(d.dragging(), Some(1));
        assert_eq!(
            d.pointer_up(Point::new(410.0, 310.0), 0.2),
            Some(DeskEvent::Dropped {
                item: 1,
                zone: None
            })
        );
        assert_eq!(d.dragging(), None);
    }

    #[test]
    fn a_dragged_object_stays_on_the_desk() {
        let mut d = desk();
        d.pointer_down(Point::new(110.0, 110.0), 0.0);
        d.pointer_move(Point::new(-500.0, 5000.0));
        let b = d.item(1).unwrap().bounds();
        assert!(b.right() >= 0.0 + b.w * 0.25 - 0.01, "{b:?}");
        assert!(b.y <= 600.0 - b.h * 0.25 + 0.01, "{b:?}");
    }

    #[test]
    fn drops_report_the_zone_under_the_pointer() {
        let mut d = desk();
        d.add_zone("window", Rect::new(800.0, 0.0, 200.0, 200.0));
        d.pointer_down(Point::new(110.0, 110.0), 0.0);
        d.pointer_move(Point::new(850.0, 50.0));
        assert_eq!(
            d.pointer_up(Point::new(850.0, 50.0), 0.3),
            Some(DeskEvent::Dropped {
                item: 1,
                zone: Some("window".into())
            })
        );
    }

    #[test]
    fn clicks_and_double_clicks_name_the_field() {
        let mut d = desk();
        d.add_field(2, "name", Rect::new(10.0, 10.0, 100.0, 20.0));
        d.pointer_down(Point::new(170.0, 135.0), 0.0);
        assert_eq!(
            d.pointer_up(Point::new(170.0, 135.0), 0.05),
            Some(DeskEvent::Clicked {
                item: 2,
                local: Point::new(20.0, 15.0),
                field: Some("name".into())
            })
        );
        d.pointer_down(Point::new(170.0, 135.0), 0.2);
        assert_eq!(
            d.pointer_up(Point::new(170.0, 135.0), 0.25),
            Some(DeskEvent::DoubleClicked {
                item: 2,
                field: Some("name".into())
            })
        );
        d.pointer_down(Point::new(170.0, 135.0), 2.0);
        assert!(
            matches!(
                d.pointer_up(Point::new(170.0, 135.0), 2.05),
                Some(DeskEvent::Clicked { .. })
            ),
            "too late for a double click"
        );
    }

    #[test]
    fn fields_are_found_on_turned_and_scaled_objects() {
        let mut d = Desk::new(Rect::new(0.0, 0.0, 1000.0, 1000.0));
        d.add_item(7, 100.0, 50.0, Point::new(300.0, 300.0), 90.0);
        d.item_mut(7).unwrap().scale = 2.0;
        d.add_field(7, "seal", Rect::new(80.0, 30.0, 20.0, 20.0));
        // Turned 90° clockwise around its corner and doubled: local (90, 40) is at desk (300 - 80, 300 + 180).
        let at = d.item(7).unwrap().to_desk(Point::new(90.0, 40.0));
        assert!(
            (at.x - 220.0).abs() < 0.01 && (at.y - 480.0).abs() < 0.01,
            "{at:?}"
        );
        assert_eq!(
            d.field_at(Point::new(220.0, 480.0)),
            Some((7, "seal".into()))
        );
        assert_eq!(
            d.field_at(Point::new(330.0, 310.0)),
            None,
            "outside the turned object"
        );
    }

    #[test]
    fn marks_pair_up_toggle_and_follow_their_objects() {
        let mut d = desk();
        d.add_field(1, "species", Rect::new(0.0, 0.0, 40.0, 20.0));
        d.add_field(2, "species", Rect::new(0.0, 0.0, 40.0, 20.0));
        d.mark_field(1, "species");
        assert!(d.pair().is_none());
        d.mark_field(2, "species");
        assert!(d.pair().is_some());
        d.mark(Mark {
            owner: 900,
            field: "rule-3".into(),
            at: Point::new(0.0, 0.0),
        });
        assert_eq!(
            d.marks().iter().map(|m| m.owner).collect::<Vec<_>>(),
            vec![2, 900],
            "a third replaces the oldest"
        );
        d.mark_field(2, "species");
        assert_eq!(d.marks().len(), 1, "marking again takes it off");
        d.mark_field(1, "species");
        d.item_mut(1).unwrap().pos = Point::new(500.0, 500.0);
        d.refresh_marks();
        assert_eq!(
            d.marks().iter().find(|m| m.owner == 1).unwrap().at,
            Point::new(520.0, 510.0)
        );
        d.remove_item(1);
        assert!(
            d.marks().iter().all(|m| m.owner != 1),
            "an object taken away takes its marks"
        );
    }

    #[test]
    fn objects_that_cannot_be_dragged_still_click() {
        let mut d = desk();
        d.item_mut(2).unwrap().draggable = false;
        d.pointer_down(Point::new(170.0, 135.0), 0.0);
        assert_eq!(d.pointer_move(Point::new(400.0, 400.0)), None);
        assert_eq!(d.item(2).unwrap().pos, Point::new(150.0, 120.0));
    }

    #[test]
    fn a_press_on_nothing_is_missed_and_magnifying_follows_the_objects() {
        let mut d = desk();
        assert_eq!(
            d.pointer_down(Point::new(900.0, 500.0), 0.0),
            Some(DeskEvent::Missed {
                at: Point::new(900.0, 500.0)
            })
        );
        d.set_magnified(Some(1));
        assert_eq!(d.magnified(), Some(1));
        d.set_magnified(Some(42));
        assert_eq!(d.magnified(), None, "an object that is not on the desk");
        d.set_magnified(Some(2));
        d.remove_item(2);
        assert_eq!(d.magnified(), None);
    }
}
