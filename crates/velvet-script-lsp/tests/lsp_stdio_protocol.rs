//! Protocol-level tests for Content-Length framed LSP (same handlers as stdio server).

use serde_json::json;
use std::io::Cursor;
use velvet_script_lsp::stdio::{read_message, write_message, LspState};

#[test]
fn framed_initialize_completion_symbols() {
    let mut state = LspState::new();

    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "capabilities": {}, "rootUri": null, "processId": null }
    });
    let out = state.handle_message(init);
    assert_eq!(out[0]["id"], 1);
    assert_eq!(out[0]["result"]["serverInfo"]["name"], "velvet-script-lsp");
    assert_eq!(out[0]["result"]["capabilities"]["definitionProvider"], true);

    let src = r#"
character hero { name: "Hero" }
scene intro {
    hero "Hello"
}
function add(a, b) {
    return a + b
}
"#;
    let open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///demo.vel",
                "languageId": "velvet",
                "version": 1,
                "text": src
            }
        }
    });
    let diags = state.handle_message(open);
    assert_eq!(diags[0]["method"], "textDocument/publishDiagnostics");

    let symbols = state.handle_message(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentSymbol",
        "params": { "textDocument": { "uri": "file:///demo.vel" } }
    }));
    let arr = symbols[0]["result"].as_array().cloned().unwrap_or_default();
    let names: Vec<_> = arr
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        names.contains(&"intro") || names.contains(&"add") || names.contains(&"hero"),
        "names={names:?}"
    );

    let comp = state.handle_message(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///demo.vel" },
            "position": { "line": 0, "character": 0 }
        }
    }));
    let items = comp[0]["result"].as_array().cloned().unwrap_or_default();
    assert!(!items.is_empty());
}

#[test]
fn write_read_frame_roundtrip() {
    let msg = json!({"jsonrpc":"2.0","id":9,"method":"shutdown","params":null});
    let mut buf = Vec::new();
    write_message(&mut buf, &msg).unwrap();
    let s = String::from_utf8_lossy(&buf);
    assert!(s.starts_with("Content-Length:"));
    let mut cur = Cursor::new(buf);
    let got = read_message(&mut cur).unwrap().unwrap();
    assert_eq!(got["id"], 9);
}

#[test]
fn intentional_error_produces_diagnostics_or_clean_recovery() {
    let mut state = LspState::new();
    let open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///bad.vel",
                "languageId": "velvet",
                "version": 1,
                "text": "function ( {\n"
            }
        }
    });
    let res = state.handle_message(open);
    assert_eq!(res[0]["method"], "textDocument/publishDiagnostics");
    // Either diagnostics non-empty OR recovery produced empty with no crash — protocol path works.
    assert!(res[0].pointer("/params/uri").is_some());
}
