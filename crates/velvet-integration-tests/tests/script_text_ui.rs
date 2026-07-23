//! Cross-crate: script VM, rich text, and UI dialogue helpers.

use velvet_math::Vec2;
use velvet_script_compiler::compile_source;
use velvet_script_vm::{run_source, Value, Vm, VmLimits};
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
    assert_eq!(out.printed, vec!["hello".to_string()]);
    assert!(out.instructions > 0, "the VM must execute bytecode");

    let rich = parse_rich_text("Wait{pause=0.1}... {color=#ff0000}go{/color}").unwrap();
    let mut tw = Typewriter::from_rich(rich, 1000.0);
    for _ in 0..30 {
        tw.tick(0.05);
    }
    assert!(
        tw.is_finished(),
        "high-speed typewriter must finish within the budget"
    );
    assert_eq!(tw.visible_text(), "Wait... go");
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
    assert!(compiled.module.exports.contains_key("add"));
    let mut vm = Vm::new(compiled.module, VmLimits::default());
    assert_eq!(
        vm.call_name("add", &[Value::Int(2), Value::Int(3)])
            .unwrap(),
        Value::Int(5)
    );
}
