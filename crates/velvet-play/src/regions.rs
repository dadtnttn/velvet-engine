//! Named map regions and trigger queries.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

/// A named rectangular (or polygonal AABB) region on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapRegion {
    /// Unique name.
    pub name: String,
    /// Axis-aligned bounds in world space.
    pub bounds: Rect,
    /// Optional tags for filtering (e.g. "safe", "combat", "indoors").
    pub tags: Vec<String>,
    /// Optional integer id for scripting.
    pub id: u32,
    /// Whether region is enabled.
    pub enabled: bool,
}

impl MapRegion {
    /// Create a region from min/max corners.
    pub fn from_minmax(name: impl Into<String>, min: Vec2, max: Vec2) -> Self {
        Self {
            name: name.into(),
            bounds: Rect {
                min: Vec2::new(min.x.min(max.x), min.y.min(max.y)),
                max: Vec2::new(min.x.max(max.x), min.y.max(max.y)),
            },
            tags: Vec::new(),
            id: 0,
            enabled: true,
        }
    }

    /// Create from position + size.
    pub fn from_pos_size(name: impl Into<String>, pos: Vec2, size: Vec2) -> Self {
        Self {
            name: name.into(),
            bounds: Rect::from_pos_size(pos, size),
            tags: Vec::new(),
            id: 0,
            enabled: true,
        }
    }

    /// Builder: add tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Builder: set id.
    pub fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    /// Contains world point.
    pub fn contains(&self, point: Vec2) -> bool {
        self.enabled && self.bounds.contains_point(point)
    }

    /// Overlaps another rect.
    pub fn overlaps(&self, other: Rect) -> bool {
        self.enabled && self.bounds.intersects(other)
    }
}

/// Event when an entity enters/exits a region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionEventKind {
    /// Entered.
    Enter,
    /// Exited.
    Exit,
    /// Stayed inside this frame (optional emit).
    Stay,
}

/// Region event for gameplay systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionEvent {
    /// Region name.
    pub region: String,
    /// Region id.
    pub region_id: u32,
    /// Tracker key (entity id / player id as string).
    pub actor: String,
    /// Kind.
    pub kind: RegionEventKind,
}

/// Registry of named regions + occupancy tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegionSet {
    regions: IndexMap<String, MapRegion>,
    /// actor → set of region names currently occupied.
    occupancy: IndexMap<String, Vec<String>>,
}

impl RegionSet {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a region.
    pub fn insert(&mut self, region: MapRegion) {
        self.regions.insert(region.name.clone(), region);
    }

    /// Remove by name.
    pub fn remove(&mut self, name: &str) -> Option<MapRegion> {
        self.regions.shift_remove(name)
    }

    /// Get region.
    pub fn get(&self, name: &str) -> Option<&MapRegion> {
        self.regions.get(name)
    }

    /// All regions.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &MapRegion)> {
        self.regions.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Regions containing a point.
    pub fn regions_at(&self, point: Vec2) -> Vec<&MapRegion> {
        self.regions
            .values()
            .filter(|r| r.contains(point))
            .collect()
    }

    /// First region with tag containing point.
    pub fn first_with_tag_at(&self, point: Vec2, tag: &str) -> Option<&MapRegion> {
        self.regions
            .values()
            .find(|r| r.contains(point) && r.tags.iter().any(|t| t == tag))
    }

    /// Whether point is inside any region with the given tag.
    pub fn in_tag(&self, point: Vec2, tag: &str) -> bool {
        self.first_with_tag_at(point, tag).is_some()
    }

    /// Update occupancy for one actor; returns enter/exit events.
    pub fn update_actor(&mut self, actor: impl Into<String>, position: Vec2) -> Vec<RegionEvent> {
        let actor = actor.into();
        let now: Vec<String> = self
            .regions
            .values()
            .filter(|r| r.contains(position))
            .map(|r| r.name.clone())
            .collect();
        let prev = self.occupancy.get(&actor).cloned().unwrap_or_default();
        let mut events = Vec::new();

        for name in &now {
            if !prev.iter().any(|p| p == name) {
                let id = self.regions.get(name).map(|r| r.id).unwrap_or(0);
                events.push(RegionEvent {
                    region: name.clone(),
                    region_id: id,
                    actor: actor.clone(),
                    kind: RegionEventKind::Enter,
                });
            }
        }
        for name in &prev {
            if !now.iter().any(|n| n == name) {
                let id = self.regions.get(name).map(|r| r.id).unwrap_or(0);
                events.push(RegionEvent {
                    region: name.clone(),
                    region_id: id,
                    actor: actor.clone(),
                    kind: RegionEventKind::Exit,
                });
            }
        }

        self.occupancy.insert(actor, now);
        events
    }

    /// Clear occupancy for actor.
    pub fn clear_actor(&mut self, actor: &str) {
        self.occupancy.shift_remove(actor);
    }

    /// Regions currently occupied by actor.
    pub fn occupied_by(&self, actor: &str) -> &[String] {
        self.occupancy
            .get(actor)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Build regions from tile-layer trigger cells (each distinct trigger_id becomes a region).
    pub fn from_tile_triggers(map: &crate::map::TileMap, prefix: &str) -> Self {
        let mut set = Self::new();
        let layer = map.main_layer();
        // Collect bounding boxes per trigger_id.
        let mut boxes: IndexMap<u16, (i32, i32, i32, i32)> = IndexMap::new();
        for y in 0..layer.height as i32 {
            for x in 0..layer.width as i32 {
                let tid = layer.get(x, y).flags.trigger_id;
                if tid == 0 {
                    continue;
                }
                boxes
                    .entry(tid)
                    .and_modify(|(minx, miny, maxx, maxy)| {
                        *minx = (*minx).min(x);
                        *miny = (*miny).min(y);
                        *maxx = (*maxx).max(x);
                        *maxy = (*maxy).max(y);
                    })
                    .or_insert((x, y, x, y));
            }
        }
        for (tid, (minx, miny, maxx, maxy)) in boxes {
            let min = Vec2::new(minx as f32 * map.tile_size, miny as f32 * map.tile_size);
            let max = Vec2::new(
                (maxx as f32 + 1.0) * map.tile_size,
                (maxy as f32 + 1.0) * map.tile_size,
            );
            let region =
                MapRegion::from_minmax(format!("{prefix}{tid}"), min, max).with_id(tid as u32);
            set.insert(region);
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::TileMap;

    #[test]
    fn enter_exit_events() {
        let mut set = RegionSet::new();
        set.insert(
            MapRegion::from_pos_size("town", Vec2::ZERO, Vec2::new(100.0, 100.0)).with_tag("safe"),
        );
        let e1 = set.update_actor("player", Vec2::new(10.0, 10.0));
        assert_eq!(e1.len(), 1);
        assert_eq!(e1[0].kind, RegionEventKind::Enter);
        let e2 = set.update_actor("player", Vec2::new(10.0, 10.0));
        assert!(e2.is_empty());
        let e3 = set.update_actor("player", Vec2::new(200.0, 200.0));
        assert_eq!(e3.len(), 1);
        assert_eq!(e3[0].kind, RegionEventKind::Exit);
    }

    #[test]
    fn tag_query() {
        let mut set = RegionSet::new();
        set.insert(MapRegion::from_pos_size("a", Vec2::ZERO, Vec2::splat(50.0)).with_tag("combat"));
        assert!(set.in_tag(Vec2::new(10.0, 10.0), "combat"));
        assert!(!set.in_tag(Vec2::new(10.0, 10.0), "safe"));
    }

    #[test]
    fn from_tile_triggers() {
        let map = TileMap::from_ascii(
            "\
###
#T#
###",
            16.0,
        )
        .unwrap();
        let set = RegionSet::from_tile_triggers(&map, "trig_");
        assert!(!set.is_empty());
        assert!(set.get("trig_1").is_some());
    }

    #[test]
    fn multi_region_overlap_enter_both() {
        let mut set = RegionSet::new();
        set.insert(MapRegion::from_pos_size("a", Vec2::ZERO, Vec2::splat(50.0)).with_tag("a"));
        set.insert(
            MapRegion::from_pos_size("b", Vec2::new(25.0, 25.0), Vec2::splat(50.0)).with_tag("b"),
        );
        let events = set.update_actor("p", Vec2::new(30.0, 30.0));
        let enters: Vec<_> = events
            .iter()
            .filter(|e| e.kind == RegionEventKind::Enter)
            .map(|e| e.region.as_str())
            .collect();
        assert!(enters.contains(&"a"));
        assert!(enters.contains(&"b"));
        // Stay: no events
        assert!(set.update_actor("p", Vec2::new(31.0, 31.0)).is_empty());
        // Exit only a
        let exit = set.update_actor("p", Vec2::new(80.0, 80.0));
        assert!(exit
            .iter()
            .any(|e| e.kind == RegionEventKind::Exit && e.region == "a"));
        assert!(
            exit.iter()
                .any(|e| e.kind == RegionEventKind::Exit && e.region == "b")
                || set.in_tag(Vec2::new(80.0, 80.0), "b")
        );
    }

    #[test]
    fn disabled_region_ignored() {
        let mut set = RegionSet::new();
        let mut r = MapRegion::from_pos_size("off", Vec2::ZERO, Vec2::splat(100.0));
        r.enabled = false;
        set.insert(r);
        let e = set.update_actor("x", Vec2::new(10.0, 10.0));
        assert!(e.is_empty());
        assert!(!set.in_tag(Vec2::new(10.0, 10.0), "anything"));
    }

    #[test]
    fn two_actors_independent_occupancy() {
        let mut set = RegionSet::new();
        set.insert(MapRegion::from_pos_size(
            "zone",
            Vec2::ZERO,
            Vec2::splat(20.0),
        ));
        let e1 = set.update_actor("a", Vec2::new(5.0, 5.0));
        let e2 = set.update_actor("b", Vec2::new(5.0, 5.0));
        assert_eq!(e1.len(), 1);
        assert_eq!(e2.len(), 1);
        let leave_a = set.update_actor("a", Vec2::new(100.0, 100.0));
        assert_eq!(leave_a[0].kind, RegionEventKind::Exit);
        // b still inside — no exit for b
        assert!(set.update_actor("b", Vec2::new(6.0, 6.0)).is_empty());
    }

    #[test]
    fn overlaps_rect_query() {
        let r = MapRegion::from_minmax("m", Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(r.overlaps(Rect::from_pos_size(Vec2::new(5.0, 5.0), Vec2::splat(2.0))));
        assert!(!r.overlaps(Rect::from_pos_size(Vec2::new(50.0, 50.0), Vec2::splat(2.0))));
        assert!(r.contains(Vec2::new(1.0, 1.0)));
        assert!(!r.contains(Vec2::new(-1.0, 0.0)));
    }
}
