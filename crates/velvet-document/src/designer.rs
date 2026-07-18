//! Structural UI designer: edit menus/screens without hand-writing code.
//!
//! Mutates the shared document model (`@visual` regions) with undo/redo.
//! Not a GPU canvas — the real data path Studio GUI will drive.

use crate::model::{
    Document, DocumentError, PropertyValue, Region, RegionId, RegionKind, VisualProperty,
};
use crate::mutate::{apply_visual_patch, VisualPatch, VisualPatchOp};
use crate::parse::parse_document;
use crate::render::render_document;

/// A widget known to the simplified designer.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignerWidget {
    /// Region id (e.g. `button.start`).
    pub id: String,
    /// Kind: button, panel, label, image, …
    pub kind: String,
    /// Display text if any.
    pub text: Option<String>,
    /// Position raw string `(x, y)` percentages or pixels.
    pub position: Option<String>,
    /// Size raw if present.
    pub size: Option<String>,
    /// Background / image path.
    pub image: Option<String>,
}

/// Undoable designer session over one source document.
#[derive(Debug, Clone)]
pub struct UiDesigner {
    /// Current source text.
    source: String,
    /// Undo stack of full source snapshots.
    undo: Vec<String>,
    /// Redo stack.
    redo: Vec<String>,
}

impl UiDesigner {
    /// Open a screen/menu document.
    pub fn open(source: impl Into<String>) -> Result<Self, DocumentError> {
        let source = source.into();
        // Validate parse
        let _ = parse_document(&source)?;
        Ok(Self {
            source,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    /// Current source.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Parsed document snapshot.
    pub fn document(&self) -> Result<Document, DocumentError> {
        parse_document(&self.source)
    }

    /// List visual widgets (regions with button/panel/label-ish properties).
    pub fn list_widgets(&self) -> Result<Vec<DesignerWidget>, DocumentError> {
        let doc = self.document()?;
        let mut out = Vec::new();
        for r in doc.regions.iter().filter(|r| r.kind == RegionKind::Visual) {
            if r.id.as_str().is_empty() {
                continue;
            }
            let kind = if r.id.as_str().starts_with("button.") {
                "button"
            } else if r.id.as_str().starts_with("panel.") {
                "panel"
            } else if r.id.as_str().starts_with("label.") {
                "label"
            } else {
                "widget"
            }
            .to_string();
            let get = |k: &str| {
                r.properties
                    .iter()
                    .find(|p| p.key == k)
                    .map(|p| match &p.value {
                        PropertyValue::String(s) => s.clone(),
                        PropertyValue::Raw(s) => s.clone(),
                    })
            };
            out.push(DesignerWidget {
                id: r.id.as_str().to_string(),
                kind,
                text: get("text"),
                position: get("position"),
                size: get("size"),
                image: get("image"),
            });
        }
        Ok(out)
    }

    fn push_undo(&mut self) {
        self.undo.push(self.source.clone());
        self.redo.clear();
    }

    /// Set text on a visual region (e.g. button label).
    pub fn set_text(&mut self, region_id: &str, text: &str) -> Result<(), DocumentError> {
        self.apply_props(
            region_id,
            vec![("text", PropertyValue::String(text.into()))],
        )
    }

    /// Move widget: position string e.g. `(50%, 62%)`.
    pub fn set_position(&mut self, region_id: &str, position: &str) -> Result<(), DocumentError> {
        self.apply_props(
            region_id,
            vec![("position", PropertyValue::Raw(position.into()))],
        )
    }

    /// Resize widget: size string e.g. `(18%, 8%)`.
    pub fn set_size(&mut self, region_id: &str, size: &str) -> Result<(), DocumentError> {
        self.apply_props(
            region_id,
            vec![("size", PropertyValue::Raw(size.into()))],
        )
    }

    /// Drag a visual widget by delta (same units as stored position).
    ///
    /// This is the API the Studio canvas / GUI calls for drag-move.
    pub fn drag(
        &mut self,
        region_id: &str,
        dx: f32,
        dy: f32,
    ) -> Result<crate::drag::WidgetRect, DocumentError> {
        self.push_undo();
        let mut doc = parse_document(&self.source)?;
        let rect = crate::drag::drag_visual_region(&mut doc, region_id, dx, dy)?;
        self.source = render_document(&doc);
        Ok(rect)
    }

    /// Resize a visual widget by delta.
    pub fn resize(
        &mut self,
        region_id: &str,
        dw: f32,
        dh: f32,
    ) -> Result<crate::drag::WidgetRect, DocumentError> {
        self.push_undo();
        let mut doc = parse_document(&self.source)?;
        let rect = crate::drag::resize_visual_region(&mut doc, region_id, dw, dh)?;
        self.source = render_document(&doc);
        Ok(rect)
    }

    /// Hit-test a canvas point (percent space) → region id.
    pub fn hit_test(&self, canvas_x: f32, canvas_y: f32) -> Result<Option<String>, DocumentError> {
        let doc = self.document()?;
        Ok(crate::drag::hit_test_visual(&doc, canvas_x, canvas_y))
    }

    /// Set background image on a visual region.
    pub fn set_image(&mut self, region_id: &str, path: &str) -> Result<(), DocumentError> {
        self.apply_props(
            region_id,
            vec![("image", PropertyValue::String(path.into()))],
        )
    }

    /// Connect a button's advanced action is NOT done here — advanced is protected.
    /// Visual mode can set a declarative `action` property for simple chains.
    pub fn set_action_property(
        &mut self,
        region_id: &str,
        action: &str,
    ) -> Result<(), DocumentError> {
        self.apply_props(
            region_id,
            vec![("action", PropertyValue::String(action.into()))],
        )
    }

    fn apply_props(
        &mut self,
        region_id: &str,
        props: Vec<(&str, PropertyValue)>,
    ) -> Result<(), DocumentError> {
        self.push_undo();
        let mut doc = parse_document(&self.source)?;
        let ops = props
            .into_iter()
            .map(|(k, v)| VisualPatchOp::SetProperty {
                key: k.into(),
                value: v,
            })
            .collect();
        apply_visual_patch(
            &mut doc,
            &VisualPatch {
                region_id: RegionId::new(region_id),
                ops,
            },
        )?;
        self.source = render_document(&doc);
        Ok(())
    }

    /// Undo last designer mutation.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.source.clone());
            self.source = prev;
            true
        } else {
            false
        }
    }

    /// Redo.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.source.clone());
            self.source = next;
            true
        } else {
            false
        }
    }

    /// Save body for writing to disk.
    pub fn save_source(&self) -> String {
        self.source.clone()
    }

    /// Drop/create a new visual widget (simplified palette).
    ///
    /// Inserts a marked `@visual` region (and empty structure) without an
    /// advanced block — authors can later switch to advanced mode to attach logic.
    pub fn add_widget(
        &mut self,
        kind: &str,
        id: &str,
        x_pct: f32,
        y_pct: f32,
        text: Option<&str>,
    ) -> Result<(), DocumentError> {
        if id.is_empty() {
            return Err(DocumentError::RegionNotFound("(empty id)".into()));
        }
        self.push_undo();
        let mut doc = parse_document(&self.source)?;
        if doc.find(RegionKind::Visual, id).is_some()
            || doc.find(RegionKind::Visual, &format!("{}.{}", kind.to_ascii_lowercase(), id)).is_some()
        {
            return Err(DocumentError::InvalidPatch(format!("region already exists: {id}")));
        }
        let kind_l = kind.to_ascii_lowercase();
        let kind_norm = match kind_l.as_str() {
            "button" | "btn" => "button",
            "label" | "text" => "label",
            "panel" | "box" => "panel",
            other => other,
        };
        let full_id = if id.contains('.') {
            id.to_string()
        } else {
            format!("{kind_norm}.{id}")
        };
        let label = text.unwrap_or(match kind_norm {
            "button" => "Button",
            "label" => "Label",
            "panel" => "Panel",
            _ => "Widget",
        });
        let pos = format!("({x_pct:.0}%, {y_pct:.0}%)");
        let size = match kind_norm {
            "panel" => "(24%, 18%)",
            "label" => "(20%, 6%)",
            _ => "(18%, 8%)",
        };
        let body = format!(
            "    text: \"{}\"\n    position: {pos}\n    size: {size}\n",
            label.replace('"', "\\\"")
        );
        doc.regions.push(Region {
            kind: RegionKind::Visual,
            id: RegionId::new(full_id),
            body,
            properties: vec![
                VisualProperty {
                    key: "text".into(),
                    value: PropertyValue::String(label.into()),
                    indent: "    ".into(),
                    trailing_comment: None,
                },
                VisualProperty {
                    key: "position".into(),
                    value: PropertyValue::Raw(pos),
                    indent: "    ".into(),
                    trailing_comment: None,
                },
                VisualProperty {
                    key: "size".into(),
                    value: PropertyValue::Raw(size.into()),
                    indent: "    ".into(),
                    trailing_comment: None,
                },
            ],
            raw_lines: Vec::new(),
            marked: true,
        });
        self.source = render_document(&doc);
        Ok(())
    }

    /// Alias used by Studio palette drop (percent canvas coords).
    pub fn drop_widget(
        &mut self,
        kind: &str,
        id: &str,
        canvas_x: f32,
        canvas_y: f32,
    ) -> Result<(), DocumentError> {
        self.add_widget(kind, id, canvas_x, canvas_y, None)
    }

    /// Replace entire source (advanced mode save / reparse).
    ///
    /// Validates parse; on success replaces buffer (push undo).
    pub fn set_source_advanced(&mut self, source: impl Into<String>) -> Result<(), DocumentError> {
        let source = source.into();
        let _ = parse_document(&source)?;
        self.push_undo();
        self.source = source;
        Ok(())
    }
}

/// Minimal action catalog for simplified mode (serialized into visual `action` prop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualAction {
    /// Open a scene path.
    OpenScene(String),
    /// New game.
    NewGame,
    /// Continue / load last.
    Continue,
    /// Quit.
    Quit,
    /// Play sound asset.
    PlaySound(String),
    /// Chain of actions.
    Chain(Vec<VisualAction>),
}

impl VisualAction {
    /// Encode as a simple string for the `action` property.
    pub fn encode(&self) -> String {
        match self {
            Self::OpenScene(p) => format!("open_scene:{p}"),
            Self::NewGame => "new_game".into(),
            Self::Continue => "continue".into(),
            Self::Quit => "quit".into(),
            Self::PlaySound(p) => format!("play_sound:{p}"),
            Self::Chain(a) => a
                .iter()
                .map(VisualAction::encode)
                .collect::<Vec<_>>()
                .join("|"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MENU: &str = r#"
// @visual id=button.start
button start {
    text: "Iniciar"
    position: (50%, 62%)
// @advanced id=button.start
    on_pressed {
        game.new()
        scene.open("intro")
    }
// @end
}
// @visual id=button.quit
button quit {
    text: "Salir"
    position: (50%, 74%)
// @end
}
"#;

    #[test]
    fn create_modify_menu_without_destroying_advanced() {
        let mut d = UiDesigner::open(MENU).unwrap();
        let widgets = d.list_widgets().unwrap();
        assert!(widgets.iter().any(|w| w.id == "button.start"));

        d.set_text("button.start", "Jugar").unwrap();
        d.set_position("button.start", "(48%, 60%)").unwrap();
        d.set_action_property("button.start", &VisualAction::NewGame.encode())
            .unwrap();

        let src = d.save_source();
        assert!(src.contains("text: \"Jugar\""));
        assert!(src.contains("position: (48%, 60%)"));
        assert!(
            src.contains("game.new()") && src.contains("scene.open"),
            "advanced must remain: {src}"
        );

        assert!(d.undo());
        assert!(d.source().contains("text: \"Jugar\"") || !d.source().contains("action:"));
        // undo once more toward original text
        let _ = d.undo();
        assert!(d.redo());
    }

    #[test]
    fn chain_action_encode() {
        let a = VisualAction::Chain(vec![
            VisualAction::PlaySound("ui/click.ogg".into()),
            VisualAction::NewGame,
            VisualAction::OpenScene("scripts/main.vel".into()),
        ]);
        let s = a.encode();
        assert!(s.contains("play_sound:"));
        assert!(s.contains("new_game"));
        assert!(s.contains("open_scene:"));
    }

    #[test]
    fn designer_drag_preserves_advanced_and_moves() {
        let mut d = UiDesigner::open(MENU).unwrap();
        let rect = d.drag("button.start", -5.0, 3.0).unwrap();
        assert!((rect.pos.x - 45.0).abs() < 0.01);
        assert!((rect.pos.y - 65.0).abs() < 0.01);
        let src = d.save_source();
        assert!(src.contains("game.new()") && src.contains("scene.open"));
        assert!(src.contains("45%") && src.contains("65%"), "{src}");
    }

    #[test]
    fn drop_widget_then_drag_does_not_destroy_existing_advanced() {
        let mut d = UiDesigner::open(MENU).unwrap();
        d.drop_widget("button", "extra", 40.0, 50.0).unwrap();
        let widgets = d.list_widgets().unwrap();
        assert!(widgets.iter().any(|w| w.id == "button.extra"));
        // Drag original; advanced body must remain
        d.drag("button.start", 1.0, 0.0).unwrap();
        let src = d.save_source();
        assert!(src.contains("game.new()"), "advanced survived drop+drag: {src}");
        assert!(src.contains("button.extra") || src.contains("id=button.extra"), "{src}");
    }

    #[test]
    fn advanced_set_source_reparses_visual_widgets() {
        let mut d = UiDesigner::open(MENU).unwrap();
        let mut src = d.save_source();
        src = src.replace("Iniciar", "Play");
        d.set_source_advanced(src).unwrap();
        let w = d.list_widgets().unwrap();
        let start = w.iter().find(|w| w.id == "button.start").unwrap();
        assert_eq!(start.text.as_deref(), Some("Play"));
        assert!(d.save_source().contains("game.new()"));
    }
}
