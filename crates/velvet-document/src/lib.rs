//! # velvet-document
//!
//! Shared document model for Velvet Studio simplified and advanced modes.
//!
//! ## Goals
//!
//! * One source of truth (`.vel` / declarative text files).
//! * Visual edits never destroy advanced / protected code.
//! * Stable region ids and unknown properties survive round-trips.
//!
//! ## Region markers
//!
//! ```text
//! // @visual id=button.start
//! // @advanced id=button.start
//! // @protected id=plugin.analytics
//! // @end
//! ```
//!
//! Regions may also open with a bare `// @visual` and close at the next
//! `@visual`/`@advanced`/`@protected`/`@end` or block boundary.
//!
//! Visual property lines use simple `key: value` form and can be updated
//! without re-parsing the full language.

#![deny(missing_docs)]

mod designer;
mod drag;
mod graph;
mod level;
mod model;
mod mutate;
mod narrative;
mod parse;
mod render;

pub use designer::{DesignerWidget, UiDesigner, VisualAction};
pub use drag::{
    drag_visual_region, format_position, format_size, hit_test_visual, parse_position, parse_size,
    region_rect, resize_visual_region, WidgetPos, WidgetRect, WidgetSize,
};
pub use graph::{
    apply_graph_jump, GraphEdge, GraphEdgeKind, GraphNode, GraphNodeKind, GraphValidation,
    NarrativeGraph,
};
pub use level::{
    CollisionRect, LevelCamera, LevelDocument, LevelEditor, LevelEntity, LevelError, TileLayer,
    Vec2f,
};
pub use model::{
    Document, DocumentError, PropertyValue, Region, RegionId, RegionKind, VisualProperty,
};
pub use mutate::{apply_visual_patch, VisualPatch, VisualPatchOp};
pub use narrative::{
    DecisionArm, NarrativeBlock, NarrativeDocument, NarrativeError, NarrativeScene,
};
pub use parse::parse_document;
pub use render::render_document;

/// Open → apply visual mutation → save → reopen helper used by Studio and tests.
pub fn round_trip_visual(
    source: &str,
    patch: VisualPatch,
) -> Result<(String, Document), DocumentError> {
    let mut doc = parse_document(source)?;
    apply_visual_patch(&mut doc, &patch)?;
    let out = render_document(&doc);
    let again = parse_document(&out)?;
    Ok((out, again))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"// main menu screen
screen main_menu {
    // @visual id=button.start
    button start {
        text: "Iniciar"
        position: (50%, 62%)
        // @advanced id=button.start
        on_pressed {
            analytics.track("new_game")
            game.new()
            scene.open(resolve_intro())
        }
        // @end
    }
    // @protected id=plugin.analytics
    // plugin hooks — do not touch from visual mode
    include "plugins/analytics.vel"
    // @end
}
"#;

    #[test]
    fn parses_region_kinds_and_ids() {
        let doc = parse_document(SAMPLE).unwrap();
        assert!(doc
            .regions
            .iter()
            .any(|r| r.kind == RegionKind::Visual && r.id.as_str() == "button.start"));
        assert!(doc
            .regions
            .iter()
            .any(|r| r.kind == RegionKind::Advanced && r.id.as_str() == "button.start"));
        assert!(doc.regions.iter().any(|r| r.kind == RegionKind::Protected));
    }

    #[test]
    fn visual_edit_preserves_advanced_body() {
        let patch = VisualPatch {
            region_id: RegionId::new("button.start"),
            ops: vec![
                VisualPatchOp::SetProperty {
                    key: "text".into(),
                    value: PropertyValue::String("Comenzar".into()),
                },
                VisualPatchOp::SetProperty {
                    key: "position".into(),
                    value: PropertyValue::Raw("(48%, 70%)".into()),
                },
            ],
        };
        let (out, again) = round_trip_visual(SAMPLE, patch).unwrap();
        assert!(
            out.contains("analytics.track(\"new_game\")"),
            "advanced code must survive: {out}"
        );
        assert!(
            out.contains("text: \"Comenzar\""),
            "visual text must update: {out}"
        );
        assert!(
            out.contains("position: (48%, 70%)"),
            "visual position must update: {out}"
        );
        assert!(
            out.contains("include \"plugins/analytics.vel\""),
            "protected region must survive: {out}"
        );
        // Second open still sees advanced
        let adv = again
            .regions
            .iter()
            .find(|r| r.kind == RegionKind::Advanced && r.id.as_str() == "button.start")
            .expect("advanced region");
        assert!(adv.body.contains("analytics.track"));
    }

    #[test]
    fn open_edit_save_reopen_structure_stable() {
        let doc1 = parse_document(SAMPLE).unwrap();
        let patch = VisualPatch {
            region_id: RegionId::new("button.start"),
            ops: vec![VisualPatchOp::SetProperty {
                key: "text".into(),
                value: PropertyValue::String("Jugar".into()),
            }],
        };
        let (out, doc2) = round_trip_visual(SAMPLE, patch).unwrap();
        let _ = out;
        // Same region ids and kinds
        let kinds1: Vec<_> = doc1
            .regions
            .iter()
            .map(|r| (r.kind, r.id.as_str()))
            .collect();
        let kinds2: Vec<_> = doc2
            .regions
            .iter()
            .map(|r| (r.kind, r.id.as_str()))
            .collect();
        assert_eq!(kinds1, kinds2);
        // Comments in preamble preserved
        assert!(render_document(&doc2).contains("main menu screen"));
    }

    #[test]
    fn unknown_visual_props_preserved() {
        let src = r#"
// @visual id=panel.root
panel root {
    custom_shader: "fx/glow"
    opacity: 0.9
}
// @end
"#;
        let patch = VisualPatch {
            region_id: RegionId::new("panel.root"),
            ops: vec![VisualPatchOp::SetProperty {
                key: "opacity".into(),
                value: PropertyValue::Raw("0.5".into()),
            }],
        };
        let (out, _) = round_trip_visual(src, patch).unwrap();
        assert!(out.contains("custom_shader: \"fx/glow\""));
        assert!(out.contains("opacity: 0.5"));
    }

    #[test]
    fn protected_region_rejects_visual_patch() {
        let mut doc = parse_document(SAMPLE).unwrap();
        let patch = VisualPatch {
            region_id: RegionId::new("plugin.analytics"),
            ops: vec![VisualPatchOp::SetProperty {
                key: "x".into(),
                value: PropertyValue::Raw("1".into()),
            }],
        };
        let err = apply_visual_patch(&mut doc, &patch).unwrap_err();
        assert!(matches!(err, DocumentError::RegionNotVisual { .. }));
    }
}
