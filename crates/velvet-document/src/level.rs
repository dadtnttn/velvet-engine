//! 2D level document model for RPG / Action editors.
//!
//! Stores entities, tile layers, collisions, and cameras as serializable data
//! that Studio can edit and the runtime can load (JSON/RON-friendly).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Level document errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LevelError {
    /// Message.
    #[error("{0}")]
    Message(String),
}

/// 2D vector (f32).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vec2f {
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
}

impl Vec2f {
    /// Create.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Entity placed in a level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelEntity {
    /// Stable id.
    pub id: String,
    /// Prefab or type name (player, npc, enemy, door, …).
    pub kind: String,
    /// World position.
    pub position: Vec2f,
    /// Rotation degrees.
    pub rotation: f32,
    /// Scale.
    pub scale: Vec2f,
    /// Optional display name.
    pub name: Option<String>,
    /// Freeform properties (dialogue scene, weapon, …).
    #[serde(default)]
    pub props: std::collections::BTreeMap<String, String>,
    /// Collision layer bit.
    #[serde(default)]
    pub solid: bool,
    /// Trigger radius (0 = not a trigger).
    #[serde(default)]
    pub trigger_radius: f32,
}

impl LevelEntity {
    /// Create at position.
    pub fn new(id: impl Into<String>, kind: impl Into<String>, x: f32, y: f32) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            position: Vec2f::new(x, y),
            rotation: 0.0,
            scale: Vec2f::new(1.0, 1.0),
            name: None,
            props: Default::default(),
            solid: false,
            trigger_radius: 0.0,
        }
    }
}

/// One tile layer (row-major, width * height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileLayer {
    /// Layer name.
    pub name: String,
    /// Width in tiles.
    pub width: u32,
    /// Height in tiles.
    pub height: u32,
    /// Tile size in world units.
    pub tile_size: f32,
    /// Tile ids (0 = empty).
    pub tiles: Vec<u16>,
    /// Whether this layer contributes collision when tile != 0.
    #[serde(default)]
    pub collision: bool,
}

impl TileLayer {
    /// Create empty layer filled with zeros.
    pub fn empty(name: impl Into<String>, width: u32, height: u32, tile_size: f32) -> Self {
        let n = (width * height) as usize;
        Self {
            name: name.into(),
            width,
            height,
            tile_size,
            tiles: vec![0; n],
            collision: false,
        }
    }

    /// Index helper.
    pub fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    /// Paint a tile.
    pub fn paint(&mut self, x: u32, y: u32, tile: u16) -> Result<(), LevelError> {
        let i = self
            .index(x, y)
            .ok_or_else(|| LevelError::Message(format!("out of bounds {x},{y}")))?;
        self.tiles[i] = tile;
        Ok(())
    }

    /// Fill rectangle with tile id.
    pub fn fill_rect(
        &mut self,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        tile: u16,
    ) -> Result<(), LevelError> {
        for y in y0..=y1.min(self.height.saturating_sub(1)) {
            for x in x0..=x1.min(self.width.saturating_sub(1)) {
                self.paint(x, y, tile)?;
            }
        }
        Ok(())
    }

    /// Count non-empty tiles.
    pub fn filled_count(&self) -> usize {
        self.tiles.iter().filter(|&&t| t != 0).count()
    }
}

/// Axis-aligned collision rect in world space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionRect {
    /// Id.
    pub id: String,
    /// Position.
    pub position: Vec2f,
    /// Size.
    pub size: Vec2f,
    /// Solid vs trigger.
    #[serde(default = "default_true")]
    pub solid: bool,
}

fn default_true() -> bool {
    true
}

/// Camera definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelCamera {
    /// Id.
    pub id: String,
    /// Position.
    pub position: Vec2f,
    /// Zoom.
    #[serde(default = "default_zoom")]
    pub zoom: f32,
    /// Follow entity id if any.
    pub follow: Option<String>,
}

fn default_zoom() -> f32 {
    1.0
}

/// Full level document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelDocument {
    /// Format version.
    pub format_version: u32,
    /// Level name.
    pub name: String,
    /// Tile layers.
    pub layers: Vec<TileLayer>,
    /// Entities.
    pub entities: Vec<LevelEntity>,
    /// Extra collision rects.
    pub collisions: Vec<CollisionRect>,
    /// Cameras.
    pub cameras: Vec<LevelCamera>,
    /// Optional music path.
    pub music: Option<String>,
}

impl Default for LevelDocument {
    fn default() -> Self {
        Self {
            format_version: 1,
            name: "untitled".into(),
            layers: Vec::new(),
            entities: Vec::new(),
            collisions: Vec::new(),
            cameras: Vec::new(),
            music: None,
        }
    }
}

impl LevelDocument {
    /// Create a simple top-down level scaffold.
    pub fn top_down_scaffold(name: impl Into<String>, w: u32, h: u32) -> Self {
        let mut layer = TileLayer::empty("ground", w, h, 16.0);
        layer.collision = true;
        // Border walls
        for x in 0..w {
            let _ = layer.paint(x, 0, 1);
            let _ = layer.paint(x, h - 1, 1);
        }
        for y in 0..h {
            let _ = layer.paint(0, y, 1);
            let _ = layer.paint(w - 1, y, 1);
        }
        // Floor
        let _ = layer.fill_rect(1, 1, w.saturating_sub(2), h.saturating_sub(2), 2);

        let mut doc = Self {
            format_version: 1,
            name: name.into(),
            layers: vec![layer],
            entities: vec![LevelEntity::new("player", "player", 48.0, 48.0), {
                let mut n = LevelEntity::new("npc_1", "npc", 96.0, 64.0);
                n.props.insert("dialogue".into(), "talk_mira".into());
                n
            }],
            collisions: Vec::new(),
            cameras: vec![LevelCamera {
                id: "main".into(),
                position: Vec2f::new(0.0, 0.0),
                zoom: 1.0,
                follow: Some("player".into()),
            }],
            music: Some("assets/music/town.ogg".into()),
        };
        // Door entity
        let mut door = LevelEntity::new("door_east", "door", (w as f32 - 2.0) * 16.0, 64.0);
        door.solid = true;
        door.props.insert("requires".into(), "has_key".into());
        doc.entities.push(door);
        doc
    }

    /// Action arena scaffold with enemies.
    pub fn action_scaffold(name: impl Into<String>) -> Self {
        let mut doc = Self::top_down_scaffold(name, 20, 12);
        doc.entities
            .retain(|e| e.kind == "player" || e.kind == "door");
        for i in 0..5 {
            let mut e =
                LevelEntity::new(format!("enemy_{i}"), "enemy", 80.0 + i as f32 * 24.0, 80.0);
            e.props.insert("weapon".into(), "pistol".into());
            e.props.insert("ai".into(), "patrol".into());
            doc.entities.push(e);
        }
        doc.music = Some("assets/music/action.ogg".into());
        doc
    }

    /// Spawn / move entity.
    pub fn place_entity(&mut self, entity: LevelEntity) {
        if let Some(slot) = self.entities.iter_mut().find(|e| e.id == entity.id) {
            *slot = entity;
        } else {
            self.entities.push(entity);
        }
    }

    /// Move entity by id.
    pub fn move_entity(&mut self, id: &str, x: f32, y: f32) -> bool {
        if let Some(e) = self.entities.iter_mut().find(|e| e.id == id) {
            e.position = Vec2f::new(x, y);
            true
        } else {
            false
        }
    }

    /// Duplicate entity with new id.
    pub fn duplicate_entity(&mut self, id: &str, new_id: &str) -> Result<(), LevelError> {
        let src = self
            .entities
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or_else(|| LevelError::Message(format!("missing entity {id}")))?;
        let mut dup = src;
        dup.id = new_id.into();
        dup.position.x += 16.0;
        self.entities.push(dup);
        Ok(())
    }

    /// Remove entity.
    pub fn remove_entity(&mut self, id: &str) -> bool {
        let before = self.entities.len();
        self.entities.retain(|e| e.id != id);
        self.entities.len() < before
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, LevelError> {
        serde_json::to_string_pretty(self).map_err(|e| LevelError::Message(e.to_string()))
    }

    /// Parse from JSON.
    pub fn from_json(s: &str) -> Result<Self, LevelError> {
        serde_json::from_str(s).map_err(|e| LevelError::Message(e.to_string()))
    }

    /// Count entities of a kind.
    pub fn count_kind(&self, kind: &str) -> usize {
        self.entities.iter().filter(|e| e.kind == kind).count()
    }

    /// Validate basic integrity.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.entities.iter().filter(|e| e.kind == "player").count() != 1 {
            issues.push("level should have exactly one player".into());
        }
        let ids: HashSet<_> = self.entities.iter().map(|e| e.id.as_str()).collect();
        for c in &self.cameras {
            if let Some(f) = &c.follow {
                if !ids.contains(f.as_str()) {
                    issues.push(format!("camera {} follows missing entity {f}", c.id));
                }
            }
        }
        for layer in &self.layers {
            let expected = (layer.width * layer.height) as usize;
            if layer.tiles.len() != expected {
                issues.push(format!(
                    "layer {} size mismatch {} vs {}",
                    layer.name,
                    layer.tiles.len(),
                    expected
                ));
            }
        }
        issues
    }
}

use std::collections::HashSet;

/// Undoable level editor session.
#[derive(Debug, Clone)]
pub struct LevelEditor {
    doc: LevelDocument,
    undo: Vec<LevelDocument>,
    redo: Vec<LevelDocument>,
}

impl LevelEditor {
    /// Open document.
    pub fn open(doc: LevelDocument) -> Self {
        Self {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Current document.
    pub fn document(&self) -> &LevelDocument {
        &self.doc
    }

    fn snap(&mut self) {
        self.undo.push(self.doc.clone());
        self.redo.clear();
    }

    /// Paint tile with undo.
    pub fn paint(&mut self, layer: usize, x: u32, y: u32, tile: u16) -> Result<(), LevelError> {
        self.snap();
        self.doc
            .layers
            .get_mut(layer)
            .ok_or_else(|| LevelError::Message("bad layer".into()))?
            .paint(x, y, tile)
    }

    /// Place entity with undo.
    pub fn place(&mut self, entity: LevelEntity) {
        self.snap();
        self.doc.place_entity(entity);
    }

    /// Move entity with undo.
    pub fn move_entity(&mut self, id: &str, x: f32, y: f32) -> bool {
        self.snap();
        self.doc.move_entity(id, x, y)
    }

    /// Undo.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.doc.clone());
            self.doc = prev;
            true
        } else {
            false
        }
    }

    /// Redo.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.doc.clone());
            self.doc = next;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_rpg_level_from_editor() {
        let mut ed = LevelEditor::open(LevelDocument::top_down_scaffold("town", 16, 12));
        assert_eq!(ed.document().count_kind("player"), 1);
        assert!(ed.document().count_kind("npc") >= 1);

        ed.paint(0, 4, 4, 3).unwrap();
        ed.move_entity("player", 64.0, 64.0);
        ed.place(LevelEntity::new("chest_1", "chest", 120.0, 80.0));

        let issues = ed.document().validate();
        assert!(issues.is_empty(), "{issues:?}");

        let json = ed.document().to_json().unwrap();
        let mut again = LevelDocument::from_json(&json).unwrap();
        assert_eq!(again.name, "town");
        assert!(again.move_entity("player", 1.0, 1.0)); // exists

        assert!(ed.undo());
    }

    #[test]
    fn action_scaffold_has_five_enemies() {
        let doc = LevelDocument::action_scaffold("warehouse");
        assert_eq!(doc.count_kind("enemy"), 5);
        assert_eq!(doc.count_kind("player"), 1);
    }

    #[test]
    fn fill_and_collision_layer() {
        let mut layer = TileLayer::empty("c", 8, 8, 16.0);
        layer.collision = true;
        layer.fill_rect(2, 2, 5, 5, 9).unwrap();
        assert!(layer.filled_count() >= 16);
    }
}
