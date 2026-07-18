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
    fn stack_basic() {
        let mut s = LayerStack::new();
        s.push_layer("dialogue", LayerKind::Story, 10, false).unwrap();
        s.push_layer("settings", LayerKind::Ui, 100, true).unwrap();
        assert!(s.visible_ids().contains(&"settings"));
        s.hide("settings").unwrap();
        assert!(!s.visible_ids().contains(&"settings"));
    }
    #[test]
    fn well_known_dialogue() {
        assert_eq!(well_known::DIALOGUE, "dialogue");
        let mut s = LayerStack::new();
        s.push_layer("dialogue", LayerKind::Ui, 0, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "dialogue");
    }
    #[test]
    fn well_known_namebox() {
        assert_eq!(well_known::NAMEBOX, "namebox");
        let mut s = LayerStack::new();
        s.push_layer("namebox", LayerKind::Ui, 1, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "namebox");
    }
    #[test]
    fn well_known_choices() {
        assert_eq!(well_known::CHOICES, "choices");
        let mut s = LayerStack::new();
        s.push_layer("choices", LayerKind::Ui, 2, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "choices");
    }
    #[test]
    fn well_known_history() {
        assert_eq!(well_known::HISTORY, "history");
        let mut s = LayerStack::new();
        s.push_layer("history", LayerKind::Ui, 3, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "history");
    }
    #[test]
    fn well_known_save() {
        assert_eq!(well_known::SAVE, "save");
        let mut s = LayerStack::new();
        s.push_layer("save", LayerKind::Ui, 4, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "save");
    }
    #[test]
    fn well_known_load() {
        assert_eq!(well_known::LOAD, "load");
        let mut s = LayerStack::new();
        s.push_layer("load", LayerKind::Ui, 5, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "load");
    }
    #[test]
    fn well_known_prefs() {
        assert_eq!(well_known::PREFS, "prefs");
        let mut s = LayerStack::new();
        s.push_layer("prefs", LayerKind::Ui, 6, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "prefs");
    }
    #[test]
    fn well_known_confirm() {
        assert_eq!(well_known::CONFIRM, "confirm");
        let mut s = LayerStack::new();
        s.push_layer("confirm", LayerKind::Ui, 7, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "confirm");
    }
    #[test]
    fn well_known_title() {
        assert_eq!(well_known::TITLE, "title");
        let mut s = LayerStack::new();
        s.push_layer("title", LayerKind::Ui, 8, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "title");
    }
    #[test]
    fn well_known_hud() {
        assert_eq!(well_known::HUD, "hud");
        let mut s = LayerStack::new();
        s.push_layer("hud", LayerKind::Ui, 9, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "hud");
    }
    #[test]
    fn well_known_inventory() {
        assert_eq!(well_known::INVENTORY, "inventory");
        let mut s = LayerStack::new();
        s.push_layer("inventory", LayerKind::Ui, 10, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "inventory");
    }
    #[test]
    fn well_known_map() {
        assert_eq!(well_known::MAP, "map");
        let mut s = LayerStack::new();
        s.push_layer("map", LayerKind::Ui, 11, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "map");
    }
    #[test]
    fn well_known_battle() {
        assert_eq!(well_known::BATTLE, "battle");
        let mut s = LayerStack::new();
        s.push_layer("battle", LayerKind::Ui, 12, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "battle");
    }
    #[test]
    fn well_known_pause() {
        assert_eq!(well_known::PAUSE, "pause");
        let mut s = LayerStack::new();
        s.push_layer("pause", LayerKind::Ui, 13, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "pause");
    }
    #[test]
    fn well_known_credits() {
        assert_eq!(well_known::CREDITS, "credits");
        let mut s = LayerStack::new();
        s.push_layer("credits", LayerKind::Ui, 14, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "credits");
    }
    #[test]
    fn well_known_gallery() {
        assert_eq!(well_known::GALLERY, "gallery");
        let mut s = LayerStack::new();
        s.push_layer("gallery", LayerKind::Ui, 15, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "gallery");
    }
    #[test]
    fn well_known_settings() {
        assert_eq!(well_known::SETTINGS, "settings");
        let mut s = LayerStack::new();
        s.push_layer("settings", LayerKind::Ui, 16, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "settings");
    }
    #[test]
    fn well_known_notify() {
        assert_eq!(well_known::NOTIFY, "notify");
        let mut s = LayerStack::new();
        s.push_layer("notify", LayerKind::Ui, 17, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "notify");
    }
    #[test]
    fn well_known_tooltip() {
        assert_eq!(well_known::TOOLTIP, "tooltip");
        let mut s = LayerStack::new();
        s.push_layer("tooltip", LayerKind::Ui, 18, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "tooltip");
    }
    #[test]
    fn well_known_modal() {
        assert_eq!(well_known::MODAL, "modal");
        let mut s = LayerStack::new();
        s.push_layer("modal", LayerKind::Ui, 19, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "modal");
    }
    #[test]
    fn well_known_overlay() {
        assert_eq!(well_known::OVERLAY, "overlay");
        let mut s = LayerStack::new();
        s.push_layer("overlay", LayerKind::Ui, 20, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "overlay");
    }
    #[test]
    fn well_known_cinematic() {
        assert_eq!(well_known::CINEMATIC, "cinematic");
        let mut s = LayerStack::new();
        s.push_layer("cinematic", LayerKind::Ui, 21, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "cinematic");
    }
    #[test]
    fn well_known_minimap() {
        assert_eq!(well_known::MINIMAP, "minimap");
        let mut s = LayerStack::new();
        s.push_layer("minimap", LayerKind::Ui, 22, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "minimap");
    }
    #[test]
    fn well_known_quest() {
        assert_eq!(well_known::QUEST, "quest");
        let mut s = LayerStack::new();
        s.push_layer("quest", LayerKind::Ui, 23, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "quest");
    }
    #[test]
    fn well_known_shop() {
        assert_eq!(well_known::SHOP, "shop");
        let mut s = LayerStack::new();
        s.push_layer("shop", LayerKind::Ui, 24, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "shop");
    }
    #[test]
    fn exclusive_kind_0() {
        let mut s = LayerStack::new();
        s.push_layer("a0", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b0", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a0").unwrap().visible);
        s.show("a0").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a0").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_1() {
        let mut s = LayerStack::new();
        s.push_layer("a1", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b1", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a1").unwrap().visible);
        s.show("a1").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a1").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_2() {
        let mut s = LayerStack::new();
        s.push_layer("a2", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b2", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a2").unwrap().visible);
        s.show("a2").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a2").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_3() {
        let mut s = LayerStack::new();
        s.push_layer("a3", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b3", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a3").unwrap().visible);
        s.show("a3").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a3").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_4() {
        let mut s = LayerStack::new();
        s.push_layer("a4", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b4", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a4").unwrap().visible);
        s.show("a4").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a4").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_5() {
        let mut s = LayerStack::new();
        s.push_layer("a5", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b5", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a5").unwrap().visible);
        s.show("a5").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a5").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_6() {
        let mut s = LayerStack::new();
        s.push_layer("a6", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b6", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a6").unwrap().visible);
        s.show("a6").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a6").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_7() {
        let mut s = LayerStack::new();
        s.push_layer("a7", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b7", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a7").unwrap().visible);
        s.show("a7").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a7").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_8() {
        let mut s = LayerStack::new();
        s.push_layer("a8", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b8", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a8").unwrap().visible);
        s.show("a8").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a8").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_9() {
        let mut s = LayerStack::new();
        s.push_layer("a9", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b9", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a9").unwrap().visible);
        s.show("a9").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a9").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_10() {
        let mut s = LayerStack::new();
        s.push_layer("a10", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b10", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a10").unwrap().visible);
        s.show("a10").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a10").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_11() {
        let mut s = LayerStack::new();
        s.push_layer("a11", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b11", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a11").unwrap().visible);
        s.show("a11").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a11").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_12() {
        let mut s = LayerStack::new();
        s.push_layer("a12", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b12", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a12").unwrap().visible);
        s.show("a12").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a12").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_13() {
        let mut s = LayerStack::new();
        s.push_layer("a13", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b13", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a13").unwrap().visible);
        s.show("a13").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a13").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_14() {
        let mut s = LayerStack::new();
        s.push_layer("a14", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b14", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a14").unwrap().visible);
        s.show("a14").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a14").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_15() {
        let mut s = LayerStack::new();
        s.push_layer("a15", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b15", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a15").unwrap().visible);
        s.show("a15").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a15").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_16() {
        let mut s = LayerStack::new();
        s.push_layer("a16", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b16", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a16").unwrap().visible);
        s.show("a16").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a16").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_17() {
        let mut s = LayerStack::new();
        s.push_layer("a17", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b17", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a17").unwrap().visible);
        s.show("a17").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a17").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_18() {
        let mut s = LayerStack::new();
        s.push_layer("a18", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b18", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a18").unwrap().visible);
        s.show("a18").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a18").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_19() {
        let mut s = LayerStack::new();
        s.push_layer("a19", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b19", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a19").unwrap().visible);
        s.show("a19").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a19").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_20() {
        let mut s = LayerStack::new();
        s.push_layer("a20", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b20", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a20").unwrap().visible);
        s.show("a20").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a20").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_21() {
        let mut s = LayerStack::new();
        s.push_layer("a21", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b21", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a21").unwrap().visible);
        s.show("a21").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a21").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_22() {
        let mut s = LayerStack::new();
        s.push_layer("a22", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b22", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a22").unwrap().visible);
        s.show("a22").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a22").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_23() {
        let mut s = LayerStack::new();
        s.push_layer("a23", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b23", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a23").unwrap().visible);
        s.show("a23").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a23").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_24() {
        let mut s = LayerStack::new();
        s.push_layer("a24", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b24", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a24").unwrap().visible);
        s.show("a24").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a24").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_25() {
        let mut s = LayerStack::new();
        s.push_layer("a25", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b25", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a25").unwrap().visible);
        s.show("a25").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a25").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_26() {
        let mut s = LayerStack::new();
        s.push_layer("a26", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b26", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a26").unwrap().visible);
        s.show("a26").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a26").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_27() {
        let mut s = LayerStack::new();
        s.push_layer("a27", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b27", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a27").unwrap().visible);
        s.show("a27").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a27").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_28() {
        let mut s = LayerStack::new();
        s.push_layer("a28", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b28", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a28").unwrap().visible);
        s.show("a28").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a28").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_29() {
        let mut s = LayerStack::new();
        s.push_layer("a29", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b29", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a29").unwrap().visible);
        s.show("a29").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a29").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_30() {
        let mut s = LayerStack::new();
        s.push_layer("a30", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b30", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a30").unwrap().visible);
        s.show("a30").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a30").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_31() {
        let mut s = LayerStack::new();
        s.push_layer("a31", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b31", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a31").unwrap().visible);
        s.show("a31").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a31").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_32() {
        let mut s = LayerStack::new();
        s.push_layer("a32", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b32", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a32").unwrap().visible);
        s.show("a32").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a32").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_33() {
        let mut s = LayerStack::new();
        s.push_layer("a33", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b33", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a33").unwrap().visible);
        s.show("a33").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a33").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_34() {
        let mut s = LayerStack::new();
        s.push_layer("a34", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b34", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a34").unwrap().visible);
        s.show("a34").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a34").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_35() {
        let mut s = LayerStack::new();
        s.push_layer("a35", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b35", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a35").unwrap().visible);
        s.show("a35").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a35").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_36() {
        let mut s = LayerStack::new();
        s.push_layer("a36", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b36", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a36").unwrap().visible);
        s.show("a36").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a36").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_37() {
        let mut s = LayerStack::new();
        s.push_layer("a37", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b37", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a37").unwrap().visible);
        s.show("a37").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a37").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_38() {
        let mut s = LayerStack::new();
        s.push_layer("a38", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b38", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a38").unwrap().visible);
        s.show("a38").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a38").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_39() {
        let mut s = LayerStack::new();
        s.push_layer("a39", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b39", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a39").unwrap().visible);
        s.show("a39").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a39").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_40() {
        let mut s = LayerStack::new();
        s.push_layer("a40", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b40", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a40").unwrap().visible);
        s.show("a40").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a40").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_41() {
        let mut s = LayerStack::new();
        s.push_layer("a41", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b41", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a41").unwrap().visible);
        s.show("a41").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a41").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_42() {
        let mut s = LayerStack::new();
        s.push_layer("a42", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b42", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a42").unwrap().visible);
        s.show("a42").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a42").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_43() {
        let mut s = LayerStack::new();
        s.push_layer("a43", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b43", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a43").unwrap().visible);
        s.show("a43").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a43").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_44() {
        let mut s = LayerStack::new();
        s.push_layer("a44", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b44", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a44").unwrap().visible);
        s.show("a44").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a44").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_45() {
        let mut s = LayerStack::new();
        s.push_layer("a45", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b45", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a45").unwrap().visible);
        s.show("a45").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a45").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_46() {
        let mut s = LayerStack::new();
        s.push_layer("a46", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b46", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a46").unwrap().visible);
        s.show("a46").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a46").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_47() {
        let mut s = LayerStack::new();
        s.push_layer("a47", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b47", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a47").unwrap().visible);
        s.show("a47").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a47").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_48() {
        let mut s = LayerStack::new();
        s.push_layer("a48", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b48", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a48").unwrap().visible);
        s.show("a48").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a48").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_49() {
        let mut s = LayerStack::new();
        s.push_layer("a49", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b49", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a49").unwrap().visible);
        s.show("a49").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a49").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_50() {
        let mut s = LayerStack::new();
        s.push_layer("a50", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b50", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a50").unwrap().visible);
        s.show("a50").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a50").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_51() {
        let mut s = LayerStack::new();
        s.push_layer("a51", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b51", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a51").unwrap().visible);
        s.show("a51").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a51").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_52() {
        let mut s = LayerStack::new();
        s.push_layer("a52", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b52", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a52").unwrap().visible);
        s.show("a52").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a52").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_53() {
        let mut s = LayerStack::new();
        s.push_layer("a53", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b53", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a53").unwrap().visible);
        s.show("a53").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a53").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_54() {
        let mut s = LayerStack::new();
        s.push_layer("a54", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b54", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a54").unwrap().visible);
        s.show("a54").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a54").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_55() {
        let mut s = LayerStack::new();
        s.push_layer("a55", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b55", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a55").unwrap().visible);
        s.show("a55").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a55").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_56() {
        let mut s = LayerStack::new();
        s.push_layer("a56", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b56", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a56").unwrap().visible);
        s.show("a56").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a56").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_57() {
        let mut s = LayerStack::new();
        s.push_layer("a57", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b57", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a57").unwrap().visible);
        s.show("a57").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a57").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_58() {
        let mut s = LayerStack::new();
        s.push_layer("a58", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b58", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a58").unwrap().visible);
        s.show("a58").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a58").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_59() {
        let mut s = LayerStack::new();
        s.push_layer("a59", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b59", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a59").unwrap().visible);
        s.show("a59").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a59").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_60() {
        let mut s = LayerStack::new();
        s.push_layer("a60", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b60", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a60").unwrap().visible);
        s.show("a60").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a60").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_61() {
        let mut s = LayerStack::new();
        s.push_layer("a61", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b61", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a61").unwrap().visible);
        s.show("a61").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a61").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_62() {
        let mut s = LayerStack::new();
        s.push_layer("a62", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b62", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a62").unwrap().visible);
        s.show("a62").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a62").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_63() {
        let mut s = LayerStack::new();
        s.push_layer("a63", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b63", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a63").unwrap().visible);
        s.show("a63").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a63").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_64() {
        let mut s = LayerStack::new();
        s.push_layer("a64", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b64", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a64").unwrap().visible);
        s.show("a64").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a64").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_65() {
        let mut s = LayerStack::new();
        s.push_layer("a65", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b65", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a65").unwrap().visible);
        s.show("a65").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a65").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_66() {
        let mut s = LayerStack::new();
        s.push_layer("a66", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b66", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a66").unwrap().visible);
        s.show("a66").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a66").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_67() {
        let mut s = LayerStack::new();
        s.push_layer("a67", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b67", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a67").unwrap().visible);
        s.show("a67").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a67").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_68() {
        let mut s = LayerStack::new();
        s.push_layer("a68", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b68", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a68").unwrap().visible);
        s.show("a68").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a68").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_69() {
        let mut s = LayerStack::new();
        s.push_layer("a69", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b69", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a69").unwrap().visible);
        s.show("a69").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a69").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_70() {
        let mut s = LayerStack::new();
        s.push_layer("a70", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b70", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a70").unwrap().visible);
        s.show("a70").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a70").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_71() {
        let mut s = LayerStack::new();
        s.push_layer("a71", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b71", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a71").unwrap().visible);
        s.show("a71").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a71").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_72() {
        let mut s = LayerStack::new();
        s.push_layer("a72", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b72", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a72").unwrap().visible);
        s.show("a72").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a72").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_73() {
        let mut s = LayerStack::new();
        s.push_layer("a73", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b73", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a73").unwrap().visible);
        s.show("a73").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a73").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_74() {
        let mut s = LayerStack::new();
        s.push_layer("a74", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b74", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a74").unwrap().visible);
        s.show("a74").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a74").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_75() {
        let mut s = LayerStack::new();
        s.push_layer("a75", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b75", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a75").unwrap().visible);
        s.show("a75").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a75").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_76() {
        let mut s = LayerStack::new();
        s.push_layer("a76", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b76", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a76").unwrap().visible);
        s.show("a76").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a76").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_77() {
        let mut s = LayerStack::new();
        s.push_layer("a77", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b77", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a77").unwrap().visible);
        s.show("a77").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a77").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_78() {
        let mut s = LayerStack::new();
        s.push_layer("a78", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b78", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a78").unwrap().visible);
        s.show("a78").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a78").unwrap().visible);
    }
    #[test]
    fn exclusive_kind_79() {
        let mut s = LayerStack::new();
        s.push_layer("a79", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b79", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a79").unwrap().visible);
        s.show("a79").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a79").unwrap().visible);
    }
}
