//! Velvet Script language server — **LSP over stdio** (Content-Length framing).
//!
//! Also supports a legacy line-delimited JSON mode when `VELVET_LSP_LEGACY=1`.
//!
//! VS Code: point `"velvet-script-lsp"` command at this binary with args empty;
//! transport is stdio.

use std::env;

fn main() {
    if env::var("VELVET_LSP_LEGACY").ok().as_deref() == Some("1") {
        legacy_ndjson();
        return;
    }
    if let Err(e) = velvet_script_lsp::stdio::run_stdio() {
        eprintln!("velvet-script-lsp error: {e}");
        std::process::exit(1);
    }
}

/// Legacy NDJSON protocol kept for older tooling.
fn legacy_ndjson() {
    use serde::{Deserialize, Serialize};
    use std::io::{self, BufRead, Write};
    use velvet_script_lsp::{analyze, completions};

    #[derive(Debug, Deserialize)]
    struct Request {
        cmd: String,
        #[serde(default)]
        source: String,
        #[serde(default)]
        file: Option<String>,
        #[serde(default)]
        line: u32,
        #[serde(default)]
        character: u32,
    }

    #[derive(Debug, Serialize)]
    struct Response {
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&Response {
                        ok: false,
                        result: None,
                        error: Some(e.to_string()),
                    })
                    .unwrap_or_default()
                );
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&Response {
                        ok: false,
                        result: None,
                        error: Some(e.to_string()),
                    })
                    .unwrap()
                );
                continue;
            }
        };
        if req.cmd == "exit" {
            break;
        }
        let resp = match req.cmd.as_str() {
            "analyze" => {
                let a = analyze(&req.source, req.file.as_deref());
                Response {
                    ok: true,
                    result: Some(serde_json::to_value(a).unwrap_or_default()),
                    error: None,
                }
            }
            "completions" => {
                let c = completions(&req.source, req.line, req.character);
                Response {
                    ok: true,
                    result: Some(serde_json::to_value(c).unwrap_or_default()),
                    error: None,
                }
            }
            other => Response {
                ok: false,
                result: None,
                error: Some(format!("unknown cmd {other}")),
            },
        };
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}
