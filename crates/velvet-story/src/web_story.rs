//! Serialize a [`StoryProgram`] into the JSON shape used by the web Node player.

use serde_json::{json, Value};

use crate::ir::{StoryOp, StoryProgram};

/// Convert a story program into web-player JSON (ops simplified for Node runner).
pub fn program_to_web_json(program: &StoryProgram) -> Value {
    let mut scenes = serde_json::Map::new();
    for (name, scene) in &program.scenes {
        let ops: Vec<Value> = scene.ops.iter().map(op_to_json).collect();
        scenes.insert(name.clone(), json!({ "ops": ops }));
    }
    json!({
        "title": program.title,
        "entry": program.entry,
        "scenes": scenes,
    })
}

fn op_to_json(op: &StoryOp) -> Value {
    match op {
        StoryOp::Dialogue { speaker, text } => json!({
            "kind": "dialogue",
            "speaker": speaker,
            "text": text,
        }),
        StoryOp::Choice { options } => {
            let opts: Vec<Value> = options
                .iter()
                .map(|a| {
                    json!({
                        "text": a.text,
                        "body": a.body.iter().map(op_to_json).collect::<Vec<_>>(),
                    })
                })
                .collect();
            json!({ "kind": "choice", "options": opts })
        }
        StoryOp::Jump { target } => json!({ "kind": "jump", "target": target }),
        StoryOp::Call { target } => json!({ "kind": "jump", "target": target }),
        StoryOp::End { ending } => json!({ "kind": "end", "ending": ending }),
        StoryOp::Background { path } => json!({ "kind": "background", "path": path }),
        StoryOp::Music { path, fade_in } => json!({
            "kind": "music",
            "path": path,
            "fade_in": fade_in,
        }),
        StoryOp::Show { target, at } => json!({ "kind": "show", "target": target, "at": at }),
        StoryOp::Hide { target } => json!({ "kind": "hide", "target": target }),
        StoryOp::Label { name } => json!({ "kind": "label", "name": name }),
        StoryOp::Assign { name, .. } => json!({ "kind": "assign", "name": name }),
        StoryOp::If {
            cond_var,
            then_ops,
            else_ops,
        } => json!({
            "kind": "if",
            "cond": cond_var,
            "then": then_ops.iter().map(op_to_json).collect::<Vec<_>>(),
            "else": else_ops.iter().map(op_to_json).collect::<Vec<_>>(),
        }),
        StoryOp::HostCall { name, args } => json!({
            "kind": "host_call",
            "name": name,
            "args": args,
        }),
        StoryOp::Nop => json!({ "kind": "nop" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;

    #[test]
    fn web_json_has_entry_and_ending_line() {
        let src = r#"
character h { name: "H" }
scene main {
  h "Hi"
  choice { "Go" { jump end } }
}
scene end { "Ending: Web" }
"#;
        let p = load_program_from_source(src, Some("w.vel"), "W").unwrap();
        let v = program_to_web_json(&p);
        assert_eq!(v["entry"], "main");
        assert!(v["scenes"]["end"]["ops"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Ending"));
    }
}
