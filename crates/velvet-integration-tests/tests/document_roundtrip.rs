//! Cross-crate: visual document patches preserve advanced/protected script regions.
//!
//! This is the production-path test for Phase 2 acceptance:
//! open → visual mutation → save → reopen → advanced still present.

use std::fs;
use std::path::PathBuf;

use velvet_document::{
    apply_visual_patch, parse_document, render_document, round_trip_visual, DocumentError,
    PropertyValue, RegionId, RegionKind, VisualPatch, VisualPatchOp,
};

const MENU_DOC: &str = r#"// Velvet UI screen — shared by simplified and advanced Studio modes
screen main_menu {
    background: "assets/bg/menu.png"
    // @visual id=button.start
    button start {
        text: "Iniciar"
        position: (50%, 62%)
        image: "assets/ui/btn.png"
    // @advanced id=button.start
        on_pressed {
            analytics.track("new_game")
            game.new()
            scene.open(resolve_intro())
        }
    // @end
    }
    // @visual id=button.quit
    button quit {
        text: "Salir"
        position: (50%, 74%)
    // @advanced id=button.quit
        on_pressed {
            game.quit()
        }
    // @end
    }
    // @protected id=plugin.analytics
    // Do not edit from visual mode
    include "plugins/analytics.vel"
    // @end
}
"#;

#[test]
fn open_edit_save_reopen_preserves_advanced() {
    let patch = VisualPatch {
        region_id: RegionId::new("button.start"),
        ops: vec![
            VisualPatchOp::SetProperty {
                key: "text".into(),
                value: PropertyValue::String("Nueva partida".into()),
            },
            VisualPatchOp::SetProperty {
                key: "position".into(),
                value: PropertyValue::Raw("(52%, 60%)".into()),
            },
        ],
    };
    let (saved, reopened) = round_trip_visual(MENU_DOC, patch).unwrap();

    assert!(
        saved.contains("analytics.track(\"new_game\")"),
        "advanced handler lost: {saved}"
    );
    assert!(
        saved.contains("game.new()"),
        "advanced game.new lost: {saved}"
    );
    assert!(
        saved.contains("text: \"Nueva partida\""),
        "visual text not updated: {saved}"
    );
    assert!(
        saved.contains("include \"plugins/analytics.vel\""),
        "protected include lost: {saved}"
    );

    let adv = reopened
        .find(RegionKind::Advanced, "button.start")
        .expect("advanced region after reopen");
    assert!(adv.body.contains("resolve_intro()"));

    let vis = reopened
        .find(RegionKind::Visual, "button.start")
        .expect("visual region");
    assert!(vis
        .properties
        .iter()
        .any(|p| p.key == "text"
            && matches!(&p.value, PropertyValue::String(s) if s == "Nueva partida")));
}

#[test]
fn file_path_roundtrip_on_disk() {
    let dir = std::env::temp_dir().join(format!(
        "velvet_roundtrip_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main_menu.vel");
    fs::write(&path, MENU_DOC).unwrap();

    let source = fs::read_to_string(&path).unwrap();
    let mut doc = parse_document(&source).unwrap();
    apply_visual_patch(
        &mut doc,
        &VisualPatch {
            region_id: RegionId::new("button.quit"),
            ops: vec![VisualPatchOp::SetProperty {
                key: "text".into(),
                value: PropertyValue::String("Exit".into()),
            }],
        },
    )
    .unwrap();
    let out = render_document(&doc);
    fs::write(&path, &out).unwrap();

    let again = fs::read_to_string(&path).unwrap();
    assert!(again.contains("game.quit()"));
    assert!(again.contains("text: \"Exit\""));
    assert!(again.contains("analytics.track"));

    let _ = fs::remove_dir_all(&dir);
    let _: PathBuf = path;
}

#[test]
fn cannot_patch_protected_via_public_api() {
    let mut doc = parse_document(MENU_DOC).unwrap();
    let err = apply_visual_patch(
        &mut doc,
        &VisualPatch {
            region_id: RegionId::new("plugin.analytics"),
            ops: vec![VisualPatchOp::SetProperty {
                key: "x".into(),
                value: PropertyValue::Raw("1".into()),
            }],
        },
    )
    .unwrap_err();
    assert!(matches!(err, DocumentError::RegionNotVisual { .. }));
}
