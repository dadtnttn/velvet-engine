//! First-class layer stack for Velvet Script 2.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Kind of game layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerKind {
    /// Narrative presentation.
    Story,
    /// Menus / HUD.
    Ui,
    /// Play world.
    World,
    /// FX / transitions.
    Fx,
    /// Audio overlay.
    Audio,
}
impl LayerKind {
    /// String form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::Ui => "ui",
            Self::World => "world",
            Self::Fx => "fx",
            Self::Audio => "audio",
        }
    }
    /// Parse.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "story" => Some(Self::Story),
            "ui" => Some(Self::Ui),
            "world" => Some(Self::World),
            "fx" => Some(Self::Fx),
            "audio" => Some(Self::Audio),
            _ => None,
        }
    }
}

/// Layer id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub String);
impl LayerId {
    /// Construct.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// Borrow.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stack entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerEntry {
    /// Id.
    pub id: LayerId,
    /// Kind.
    pub kind: LayerKind,
    /// Z-order.
    pub z: i32,
    /// Visible.
    pub visible: bool,
    /// Exclusive within kind.
    pub exclusive: bool,
}

/// Errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LayerError {
    /// Unknown.
    #[error("unknown layer `{0}`")]
    Unknown(String),
    /// Empty.
    #[error("layer stack empty")]
    Empty,
    /// Dup.
    #[error("layer `{0}` already on stack")]
    AlreadyPushed(String),
}

/// Pure stack.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerStack {
    /// Entries.
    pub entries: Vec<LayerEntry>,
}
impl LayerStack {
    /// New.
    pub fn new() -> Self {
        Self::default()
    }
    /// Push.
    pub fn push(&mut self, entry: LayerEntry) -> Result<(), LayerError> {
        if self.entries.iter().any(|e| e.id == entry.id) {
            return Err(LayerError::AlreadyPushed(entry.id.0.clone()));
        }
        if entry.exclusive {
            for e in &mut self.entries {
                if e.kind == entry.kind {
                    e.visible = false;
                }
            }
        }
        self.entries.push(entry);
        self.entries.sort_by_key(|e| e.z);
        Ok(())
    }
    /// Pop.
    pub fn pop(&mut self) -> Result<LayerEntry, LayerError> {
        self.entries.pop().ok_or(LayerError::Empty)
    }
    /// Show.
    pub fn show(&mut self, id: &str) -> Result<(), LayerError> {
        self.find_mut(id)?.visible = true;
        Ok(())
    }
    /// Hide.
    pub fn hide(&mut self, id: &str) -> Result<(), LayerError> {
        self.find_mut(id)?.visible = false;
        Ok(())
    }
    /// Z.
    pub fn set_z(&mut self, id: &str, z: i32) -> Result<(), LayerError> {
        self.find_mut(id)?.z = z;
        self.entries.sort_by_key(|e| e.z);
        Ok(())
    }
    /// Visible ids.
    pub fn visible_ids(&self) -> Vec<&str> {
        let mut v: Vec<_> = self.entries.iter().filter(|e| e.visible).collect();
        v.sort_by_key(|e| e.z);
        v.into_iter().map(|e| e.id.as_str()).collect()
    }
    fn find_mut(&mut self, id: &str) -> Result<&mut LayerEntry, LayerError> {
        self.entries
            .iter_mut()
            .find(|e| e.id.as_str() == id)
            .ok_or_else(|| LayerError::Unknown(id.into()))
    }
}

/// Host trait.
pub trait LayerRuntime {
    /// Push.
    fn push_layer(
        &mut self,
        id: &str,
        kind: LayerKind,
        z: i32,
        exclusive: bool,
    ) -> Result<(), LayerError>;
    /// Pop.
    fn pop_layer(&mut self) -> Result<(), LayerError>;
    /// Show.
    fn show_layer(&mut self, id: &str) -> Result<(), LayerError>;
    /// Hide.
    fn hide_layer(&mut self, id: &str) -> Result<(), LayerError>;
}

impl LayerRuntime for LayerStack {
    fn push_layer(
        &mut self,
        id: &str,
        kind: LayerKind,
        z: i32,
        exclusive: bool,
    ) -> Result<(), LayerError> {
        self.push(LayerEntry {
            id: LayerId::new(id),
            kind,
            z,
            visible: true,
            exclusive,
        })
    }
    fn pop_layer(&mut self) -> Result<(), LayerError> {
        self.pop().map(|_| ())
    }
    fn show_layer(&mut self, id: &str) -> Result<(), LayerError> {
        self.show(id)
    }
    fn hide_layer(&mut self, id: &str) -> Result<(), LayerError> {
        self.hide(id)
    }
}

/// Well-known ids.
pub mod well_known {
    /// `dialogue`
    pub const DIALOGUE: &str = "dialogue";
    /// `namebox`
    pub const NAMEBOX: &str = "namebox";
    /// `choices`
    pub const CHOICES: &str = "choices";
    /// `history`
    pub const HISTORY: &str = "history";
    /// `save`
    pub const SAVE: &str = "save";
    /// `load`
    pub const LOAD: &str = "load";
    /// `prefs`
    pub const PREFS: &str = "prefs";
    /// `confirm`
    pub const CONFIRM: &str = "confirm";
    /// `title`
    pub const TITLE: &str = "title";
    /// `hud`
    pub const HUD: &str = "hud";
    /// `inventory`
    pub const INVENTORY: &str = "inventory";
    /// `map`
    pub const MAP: &str = "map";
    /// `battle`
    pub const BATTLE: &str = "battle";
    /// `pause`
    pub const PAUSE: &str = "pause";
    /// `credits`
    pub const CREDITS: &str = "credits";
    /// `gallery`
    pub const GALLERY: &str = "gallery";
    /// `settings`
    pub const SETTINGS: &str = "settings";
    /// `notify`
    pub const NOTIFY: &str = "notify";
    /// `tooltip`
    pub const TOOLTIP: &str = "tooltip";
    /// `modal`
    pub const MODAL: &str = "modal";
    /// `overlay`
    pub const OVERLAY: &str = "overlay";
    /// `cinematic`
    pub const CINEMATIC: &str = "cinematic";
    /// `minimap`
    pub const MINIMAP: &str = "minimap";
    /// `quest`
    pub const QUEST: &str = "quest";
    /// `shop`
    pub const SHOP: &str = "shop";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_push_real() {
        let mut s = LayerStack::new();
        s.push_layer("dialogue", LayerKind::Story, 10, false).unwrap();
        s.push_layer("settings", LayerKind::Ui, 100, true).unwrap();
        // exclusive UI should be on top and visible
        assert!(s.visible_ids().contains(&"settings"));
        s.pop().unwrap();
        assert!(s.visible_ids().contains(&"dialogue"));
    }

    #[test]
    fn stack_basic() {
        let mut s = LayerStack::new();
        s.push_layer("dialogue", LayerKind::Story, 10, false).unwrap();
        s.push_layer("settings", LayerKind::Ui, 100, true).unwrap();
        assert!(s.visible_ids().contains(&"settings"));
        s.hide("settings").unwrap();
        assert!(!s.visible_ids().contains(&"settings"));
    }
    #[test]
    fn well_known_ids_are_pushable() {
        let ids = [
            well_known::DIALOGUE,
            well_known::NAMEBOX,
            well_known::CHOICES,
            well_known::HISTORY,
            well_known::SAVE,
            well_known::LOAD,
            well_known::PREFS,
            well_known::CONFIRM,
            well_known::TITLE,
            well_known::HUD,
            well_known::INVENTORY,
            well_known::MAP,
            well_known::BATTLE,
            well_known::PAUSE,
            well_known::CREDITS,
            well_known::GALLERY,
            well_known::SETTINGS,
            well_known::NOTIFY,
            well_known::TOOLTIP,
            well_known::MODAL,
            well_known::OVERLAY,
            well_known::CINEMATIC,
            well_known::MINIMAP,
            well_known::QUEST,
            well_known::SHOP,
        ];
        let mut s = LayerStack::new();
        for (i, id) in ids.iter().enumerate() {
            s.push_layer(id, LayerKind::Ui, i as i32, false).unwrap();
            assert_eq!(s.entries.last().unwrap().id.as_str(), *id);
        }
        assert_eq!(s.entries.len(), ids.len());
    }
}
