//! Live2D-compatible model attach/show on the product presentation path.
//!
//! Ships a testable surface without requiring the proprietary Cubism SDK:
//! model descriptors (model3.json / moc3-compatible paths + expressions) attach
//! to [`PresentationState`] and show as layered sprites.

use serde::{Deserialize, Serialize};

use crate::product::{LayeredSprite, PresentationState};

/// Live2D (or Live2D-compatible) model descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Live2dModel {
    /// Stable character/model id.
    pub id: String,
    /// Path to model3.json / moc3 package root (relative to project).
    pub model_path: String,
    /// Optional texture atlas path.
    pub texture: Option<String>,
    /// Active expression name.
    pub expression: String,
    /// Motion / idle name.
    pub motion: String,
    /// Placement tag: left | center | right.
    pub at: String,
    /// Z-order.
    pub z: i32,
    /// Visible.
    pub visible: bool,
}

impl Live2dModel {
    /// Create a model attached at center stage.
    pub fn new(id: impl Into<String>, model_path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            model_path: model_path.into(),
            texture: None,
            expression: "default".into(),
            motion: "idle".into(),
            at: "center".into(),
            z: 25,
            visible: false,
        }
    }
}

/// Registry of Live2D models on the product presentation path.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Live2dStage {
    /// Models by id.
    pub models: Vec<Live2dModel>,
}

impl Live2dStage {
    /// Attach (or replace) a model; does not show until [`Self::show`].
    pub fn attach(&mut self, model: Live2dModel) {
        if let Some(slot) = self.models.iter_mut().find(|m| m.id == model.id) {
            *slot = model;
        } else {
            self.models.push(model);
        }
    }

    /// Show model by id (marks visible). Returns false if missing.
    pub fn show(&mut self, id: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.visible = true;
            true
        } else {
            false
        }
    }

    /// Hide model.
    pub fn hide(&mut self, id: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.visible = false;
            true
        } else {
            false
        }
    }

    /// Set expression.
    pub fn set_expression(&mut self, id: &str, expression: &str) -> bool {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == id) {
            m.expression = expression.into();
            true
        } else {
            false
        }
    }

    /// Visible models.
    pub fn visible_models(&self) -> Vec<&Live2dModel> {
        self.models.iter().filter(|m| m.visible).collect()
    }

    /// Mirror visible Live2D models into presentation sprites (product show path).
    pub fn sync_presentation(&self, presentation: &mut PresentationState) {
        for m in self.models.iter().filter(|m| m.visible) {
            let sprite_id = format!("live2d:{}", m.id);
            presentation.sprites.insert(
                sprite_id.clone(),
                LayeredSprite {
                    id: sprite_id,
                    expression: Some(format!("{}@{}", m.expression, m.model_path)),
                    at: Some(m.at.clone()),
                    z: m.z,
                },
            );
        }
        let keep: std::collections::HashSet<String> = self
            .models
            .iter()
            .filter(|m| m.visible)
            .map(|m| format!("live2d:{}", m.id))
            .collect();
        presentation
            .sprites
            .retain(|k, _| !k.starts_with("live2d:") || keep.contains(k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_show_syncs_presentation() {
        let mut stage = Live2dStage::default();
        let mut model = Live2dModel::new("hero", "models/hero");
        model.texture = Some("models/hero/tex.png".into());
        stage.attach(model);
        assert!(!stage.show("missing"));
        assert!(stage.show("hero"));
        assert_eq!(stage.visible_models().len(), 1);
        stage.set_expression("hero", "smile");
        assert_eq!(stage.models[0].expression, "smile");

        let mut pres = PresentationState::default();
        stage.sync_presentation(&mut pres);
        assert!(pres.sprites.contains_key("live2d:hero"));
        let sp = &pres.sprites["live2d:hero"];
        assert!(sp
            .expression
            .as_ref()
            .map(|e| e.contains("smile"))
            .unwrap_or(false));
        stage.hide("hero");
        stage.sync_presentation(&mut pres);
        assert!(!pres.sprites.contains_key("live2d:hero"));
    }
}
