use std::path::PathBuf;

use velvet_vs3_app::{validate_package, FrameView, ScriptRuntime};

fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

#[test]
#[ignore = "loads the art of demos/rpg-quest (not published) and Windows' segoeui.ttf; run with --ignored on a dev machine"]
fn official_app_contract_loads() {
    let manifest = validate_package(package()).unwrap();
    assert_eq!(manifest.app.canvas_width, 960);
    assert!(manifest.capabilities.contains("render2d"));
}

#[test]
fn all_five_worlds_remain_in_vs3() {
    let mut runtime = ScriptRuntime::load(package()).unwrap();
    let worlds = runtime.call("verify_all_worlds", &[]).unwrap();
    assert_eq!(worlds.list_items().unwrap().len(), 5);
}

#[test]
fn app_frame_is_declarative() {
    let mut runtime = ScriptRuntime::load(package()).unwrap();
    let frame = FrameView::parse(&runtime.initialize().unwrap()).unwrap();
    assert!(!frame.commands.is_empty());
}

#[test]
fn title_screen_has_no_world_transition_overlay() {
    let mut runtime = ScriptRuntime::load(package()).unwrap();
    let frame = FrameView::parse(&runtime.initialize().unwrap()).unwrap();
    assert!(
        frame
            .commands
            .iter()
            .all(|command| command.action != "confirm"),
        "the title frame must not contain the full-screen transition action"
    );
}

#[test]
fn new_game_contains_complete_world_scene() {
    let mut runtime = ScriptRuntime::load(package()).unwrap();
    let _ = runtime.initialize().unwrap();
    let frame = FrameView::parse(&runtime.action("new_game").unwrap()).unwrap();

    assert!(
        frame.commands.iter().any(|command| command.op == "clear"),
        "new_game must provide a world clear command"
    );
    assert!(
        frame
            .commands
            .iter()
            .any(|command| command.op == "tile_fill"),
        "new_game must provide the terrain tile layer"
    );
    assert!(
        frame
            .commands
            .iter()
            .any(|command| command.op == "sprite" && command.asset == "house"),
        "Mundo 1 must include its house prop"
    );
}
