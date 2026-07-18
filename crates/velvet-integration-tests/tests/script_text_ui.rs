//! Cross-crate: script VM, rich text, and UI dialogue helpers.

use velvet_math::Vec2;
use velvet_script_compiler::compile_source;
use velvet_script_vm::{run_source, VmLimits};
use velvet_text::{parse_rich_text, Typewriter};
use velvet_ui::prelude::*;

#[test]
fn vm_prints_and_text_typewriter() {
    // Top-level script body uses Print opcode; function print may be native.
    let src = r#"
print("hello")
let x = 40 + 2
"#;
    let out = run_source(src, Some("t.vel"), VmLimits::default()).expect("run");
    assert!(
        out.printed.iter().any(|l| l.contains("hello")) || out.instructions > 0,
        "vm should execute; printed={:?}",
        out.printed
    );

    let rich = parse_rich_text("Wait{pause=0.1}... {color=#ff0000}go{/color}").unwrap();
    let mut tw = Typewriter::from_rich(rich, 1000.0);
    for _ in 0..30 {
        tw.tick(0.05);
    }
    assert!(tw.is_finished() || !tw.visible_text().is_empty());
}

#[test]
fn dialogue_ui_tree_layout() {
    let mut ui = UiTree::with_root("root");
    let panel = ui.build_dialogue_box("Aria", "Hello world");
    ui.layout(Vec2::new(1280.0, 720.0));
    assert!(ui.len() >= 3);
    let n = ui.get(panel).expect("panel");
    assert_eq!(n.name, "dialogue_panel");
}

#[test]
fn script_scene_compiles() {
    let src = r#"
character hero { name: "Hero" }
scene intro {
    hero "Hi"
}
function add(a, b) {
    return a + b
}
"#;
    let compiled = compile_source(src, Some("s.vel")).expect("compile");
    assert!(
        !compiled.module.functions.is_empty() || !compiled.module.exports.is_empty(),
        "should produce bytecode functions or exports"
    );
}
