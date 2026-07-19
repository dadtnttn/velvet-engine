//! # velvet-script-vs3
//!
//! **Official general game-logic language** (`// @edition 3`).
//!
//! - Classic product story (`.vel` without edition 3 → `StoryProgram`) stays separate.
//! - VS3 reuses the solid classic compile+VM path for **real** function execution.
//! - Edition gate, structured diagnostics, and a clean `compile` / `call` API.
//!
//! Not a genre prefab. Not Web3. Logics only.

#![deny(missing_docs)]

use thiserror::Error;
use velvet_script_ast::{Diagnostic, Severity, SourceLoc};
use velvet_script_lexer::Span;
use velvet_script_compiler::{compile_source, CompileError, CompileResult};
use velvet_script_vm::{Value, Vm, VmError, VmLimits};

/// Parsed source edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Classic / unspecified (not VS3).
    Classic,
    /// Historical VS2 marker (not official; rejected for VS3 API).
    Vs2,
    /// Official general logic language.
    Vs3,
}

impl Edition {
    /// Display name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Vs2 => "2",
            Self::Vs3 => "3",
        }
    }
}

/// Detect `// @edition N` (or `# @edition N`) from the first ~40 lines / 2 KiB.
pub fn detect_edition(source: &str) -> Edition {
    let head = source.lines().take(40);
    for line in head {
        let t = line.trim();
        // strip line comments
        let body = t
            .strip_prefix("//")
            .or_else(|| t.strip_prefix('#'))
            .unwrap_or(t)
            .trim();
        if let Some(rest) = body.strip_prefix("@edition") {
            let n = rest.trim();
            return match n {
                "3" | "vs3" | "VS3" => Edition::Vs3,
                "2" | "vs2" | "VS2" => Edition::Vs2,
                "1" | "classic" => Edition::Classic,
                _ => Edition::Classic,
            };
        }
    }
    Edition::Classic
}

/// VS3 diagnostic with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs3Diagnostic {
    /// Human message.
    pub message: String,
    /// Location (file:line:col when known).
    pub loc: SourceLoc,
}

impl Vs3Diagnostic {
    /// Format like other Velvet diagnostics.
    pub fn display(&self) -> String {
        format!("{}: {}", self.loc.display(), self.message)
    }
}

/// VS3 error (compile / runtime / edition).
#[derive(Debug, Error, Clone, PartialEq)]
pub enum Vs3Error {
    /// Wrong or missing edition for the VS3 API.
    #[error("{0}")]
    Edition(String),
    /// Compile / parse failure with structured diagnostics.
    #[error("{}", display_diags(.0))]
    Compile(Vec<Vs3Diagnostic>),
    /// Runtime failure.
    #[error("{loc}: {message}")]
    Runtime {
        /// Message.
        message: String,
        /// Location display.
        loc: String,
    },
}

fn display_diags(diags: &[Vs3Diagnostic]) -> String {
    if diags.is_empty() {
        return "compile failed".into();
    }
    diags
        .iter()
        .map(Vs3Diagnostic::display)
        .collect::<Vec<_>>()
        .join("\n")
}

impl Vs3Error {
    /// All structured diagnostics (empty for pure runtime/edition).
    pub fn diagnostics(&self) -> &[Vs3Diagnostic] {
        match self {
            Self::Compile(d) => d,
            _ => &[],
        }
    }

    /// True if any diagnostic carries a non-zero line number.
    pub fn has_located_diagnostic(&self) -> bool {
        self.diagnostics().iter().any(|d| d.loc.line > 0)
            || matches!(self, Self::Runtime { loc, .. } if loc.contains(':'))
    }
}

/// Compiled VS3 logic unit (callable functions).
#[derive(Debug, Clone)]
pub struct Vs3Module {
    /// Source edition (always Vs3 after successful compile).
    pub edition: Edition,
    /// Underlying bytecode module.
    pub bytecode: velvet_script_bytecode::BytecodeModule,
    /// Soft diagnostics (warnings).
    pub diagnostics: Vec<Vs3Diagnostic>,
    /// Source file name if known.
    pub file: Option<String>,
}

impl Vs3Module {
    /// Exported user function names (excludes the synthetic `<script>` entry).
    pub fn function_names(&self) -> Vec<String> {
        self.bytecode
            .exports
            .keys()
            .filter(|n| n.as_str() != "<script>")
            .cloned()
            .collect()
    }

    /// Count of callable user functions.
    pub fn user_function_count(&self) -> usize {
        self.function_names().len()
    }

    /// Call a pure logic function by name with arguments.
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Value, Vs3Error> {
        let mut vm = Vm::new(self.bytecode.clone(), VmLimits::default());
        // Bind natives into globals (Vm::new already does this)
        vm.call_name(name, args).map_err(map_vm_err)
    }
}

fn map_vm_err(e: VmError) -> Vs3Error {
    match e {
        VmError::Runtime {
            message,
            location,
            ..
        } => Vs3Error::Runtime {
            message,
            loc: location
                .as_ref()
                .map(|l| format!("{l}"))
                .unwrap_or_else(|| "<runtime>".into()),
        },
        other => Vs3Error::Runtime {
            message: other.to_string(),
            loc: "<runtime>".into(),
        },
    }
}

fn loc_at(file: Option<&str>, line: u32, column: u32) -> SourceLoc {
    SourceLoc {
        file: file.map(|s| s.to_string()),
        line,
        column,
        span: Span::default(),
    }
}

fn map_compile_err(e: CompileError, file: Option<&str>) -> Vs3Error {
    let mut diags = Vec::new();
    match e {
        CompileError::Parse(msg) => {
            // Try to pull line from "at L:C" patterns; else line 1
            let loc = parse_loc_from_message(&msg, file);
            diags.push(Vs3Diagnostic {
                message: msg,
                loc,
            });
        }
        CompileError::Codegen { message, loc } => {
            diags.push(Vs3Diagnostic {
                message,
                loc: loc_at(file, parse_line_from_loc(&loc), parse_col_from_loc(&loc)),
            });
        }
        CompileError::Many {
            diagnostics,
            first,
            ..
        } => {
            if diagnostics.is_empty() {
                diags.push(Vs3Diagnostic {
                    message: first,
                    loc: loc_at(file, 1, 1),
                });
            } else {
                for d in diagnostics {
                    diags.push(ast_diag_to_vs3(&d));
                }
            }
        }
    }
    Vs3Error::Compile(diags)
}

fn ast_diag_to_vs3(d: &Diagnostic) -> Vs3Diagnostic {
    Vs3Diagnostic {
        message: d.message.clone(),
        loc: d.loc.clone(),
    }
}

fn parse_loc_from_message(msg: &str, file: Option<&str>) -> SourceLoc {
    // e.g. "unexpected input at 3:5: ..."
    if let Some(idx) = msg.find(" at ") {
        let rest = &msg[idx + 4..];
        let nums: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == ':').collect();
        let mut parts = nums.split(':');
        let line = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        let column = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        return loc_at(file, line, column);
    }
    loc_at(file, 1, 1)
}

fn parse_line_from_loc(loc: &str) -> u32 {
    // "file:12:3" or "12:3"
    let parts: Vec<&str> = loc.rsplit(':').take(3).collect();
    if parts.len() >= 2 {
        if let Ok(l) = parts[1].parse() {
            return l;
        }
        if let Ok(l) = parts[0].parse() {
            return l;
        }
    }
    1
}

fn parse_col_from_loc(loc: &str) -> u32 {
    loc.rsplit(':')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

/// Compile VS3 source. **Requires** `// @edition 3`.
pub fn compile(source: &str, file: Option<&str>) -> Result<Vs3Module, Vs3Error> {
    let edition = detect_edition(source);
    match edition {
        Edition::Vs3 => {}
        Edition::Classic => {
            return Err(Vs3Error::Edition(
                "VS3 requires `// @edition 3` at the top of the file (classic product .vel uses StoryProgram instead)"
                    .into(),
            ));
        }
        Edition::Vs2 => {
            return Err(Vs3Error::Edition(
                "VS2 is not the official language line; use `// @edition 3` (see docs/language/VELVET_SCRIPT_3.md)"
                    .into(),
            ));
        }
    }

    let compiled: CompileResult = compile_source(source, file).map_err(|e| map_compile_err(e, file))?;

    // Surface hard diagnostics as failure
    let mut diags: Vec<Vs3Diagnostic> = compiled
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(ast_diag_to_vs3)
        .collect();

    // Reject empty logic unit (only synthetic <script>, no user fns)
    let user_fns = compiled
        .module
        .exports
        .keys()
        .filter(|n| n.as_str() != "<script>")
        .count();
    if user_fns == 0 {
        diags.push(Vs3Diagnostic {
            message: "VS3 logic unit has no callable functions".into(),
            loc: loc_at(file, 1, 1),
        });
    }

    // Disallow pure story-only units as the VS3 surface (character/scene without fn)
    // Soft: if there are functions we allow co-located story items for interop later.
    if !diags.is_empty() {
        return Err(Vs3Error::Compile(diags));
    }

    let soft: Vec<Vs3Diagnostic> = compiled
        .diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Error)
        .map(ast_diag_to_vs3)
        .collect();

    Ok(Vs3Module {
        edition: Edition::Vs3,
        bytecode: compiled.module,
        diagnostics: soft,
        file: file.map(|s| s.to_string()),
    })
}

/// Compile and call in one step (library entry for tests / hosts).
pub fn eval_call(source: &str, file: Option<&str>, name: &str, args: &[Value]) -> Result<Value, Vs3Error> {
    let module = compile(source, file)?;
    module.call(name, args)
}

/// List top-level function names from source without running (for tooling).
pub fn list_functions(source: &str, file: Option<&str>) -> Result<Vec<String>, Vs3Error> {
    let m = compile(source, file)?;
    Ok(m.function_names())
}

/// True if source is marked edition 3.
pub fn is_vs3_source(source: &str) -> bool {
    detect_edition(source) == Edition::Vs3
}

// ── Convenience constructors for tests / CLI ───────────────────────────────

/// Integer argument.
pub fn int(v: i64) -> Value {
    Value::Int(v)
}

/// Bool argument.
pub fn bool_val(v: bool) -> Value {
    Value::Bool(v)
}

/// String argument.
pub fn string_val(s: impl Into<String>) -> Value {
    Value::String(std::rc::Rc::from(s.into()))
}

/// Float argument.
pub fn float_val(v: f64) -> Value {
    Value::Float(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
// @edition 3
// VS3 logic sample — pure game rules

function can_play_card(hand_size, cost, energy) {
    return hand_size > 0 && energy >= cost
}

function apply_damage(hp, dmg) {
    if dmg >= hp {
        return 0
    }
    return hp - dmg
}

function clamp01(x) {
    if x < 0 {
        return 0
    }
    if x > 1 {
        return 1
    }
    return x
}
"#;

    #[test]
    fn detect_edition_3() {
        assert_eq!(detect_edition("// @edition 3\nfunction f() { return 1 }\n"), Edition::Vs3);
        assert_eq!(detect_edition("// @edition 2\nfunction f() { return 1 }\n"), Edition::Vs2);
        assert_eq!(detect_edition("function f() { return 1 }\n"), Edition::Classic);
    }

    #[test]
    fn classic_without_edition_rejected_by_vs3_api() {
        let err = compile("function f() { return 1 }\n", Some("c.vel")).unwrap_err();
        assert!(matches!(err, Vs3Error::Edition(_)));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn vs2_edition_rejected() {
        let err = compile("// @edition 2\nfunction f() { return 1 }\n", Some("x.vel")).unwrap_err();
        assert!(matches!(err, Vs3Error::Edition(_)));
        assert!(err.to_string().contains("edition 3") || err.to_string().contains("VS2"));
    }

    #[test]
    fn compile_edition_3_ok() {
        let m = compile(SAMPLE, Some("logic.vel")).unwrap();
        assert_eq!(m.edition, Edition::Vs3);
        let names = m.function_names();
        assert!(names.iter().any(|n| n == "can_play_card"));
        assert!(names.iter().any(|n| n == "apply_damage"));
    }

    // ── Phase 2: pure logic execution ───────────────────────────────────

    #[test]
    fn can_play_card_logic_returns_correct_bools() {
        // energy enough
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "can_play_card",
            &[int(5), int(3), int(3)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(true));
        // energy short
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "can_play_card",
            &[int(5), int(3), int(2)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(false));
        // empty hand
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "can_play_card",
            &[int(0), int(1), int(10)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(false));
    }

    #[test]
    fn apply_damage_clamps_to_zero() {
        let v = eval_call(SAMPLE, Some("logic.vel"), "apply_damage", &[int(10), int(3)]).unwrap();
        assert_eq!(v, Value::Int(7));
        let v = eval_call(SAMPLE, Some("logic.vel"), "apply_damage", &[int(5), int(5)]).unwrap();
        assert_eq!(v, Value::Int(0));
        let v = eval_call(SAMPLE, Some("logic.vel"), "apply_damage", &[int(5), int(9)]).unwrap();
        assert_eq!(v, Value::Int(0));
    }

    #[test]
    fn clamp01_control_flow() {
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(-2)]).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(0)]).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(1)]).unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(5)]).unwrap(),
            Value::Int(1)
        );
    }

    // ── Phase 3: host tool natives ──────────────────────────────────────

    const NATIVES: &str = r#"
// @edition 3

function half_turn_sin() {
    // sin(pi/2) ≈ 1
    return sin(1.5707963267948966)
}

function empty_sha() {
    return hash_sha256("")
}

function add_then_abs(a, b) {
    return abs(a + b)
}
"#;

    #[test]
    fn native_sin_matches_rust() {
        let v = eval_call(NATIVES, Some("nat.vel"), "half_turn_sin", &[]).unwrap();
        match v {
            Value::Float(f) => {
                let expected = 1.5707963267948966_f64.sin();
                assert!(
                    (f - expected).abs() < 1e-9,
                    "sin native {f} vs rust {expected}"
                );
            }
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn native_hash_sha256_empty_matches_crypto_tool() {
        let v = eval_call(NATIVES, Some("nat.vel"), "empty_sha", &[]).unwrap();
        let expected = velvet_crypto::hash_sha256_hex(b"").unwrap();
        match v {
            Value::String(s) => assert_eq!(&*s, expected.as_str()),
            other => panic!("expected string hex, got {other:?}"),
        }
    }

    #[test]
    fn native_abs_on_sum() {
        let v = eval_call(NATIVES, Some("nat.vel"), "add_then_abs", &[int(-3), int(-4)]).unwrap();
        assert_eq!(v, Value::Int(7));
    }

    // ── Phase 4: structured diagnostics ─────────────────────────────────

    #[test]
    fn parse_error_has_location() {
        let src = "// @edition 3\nfunction bad( {\n  return 1\n}\n";
        let err = compile(src, Some("bad.vel")).unwrap_err();
        match &err {
            Vs3Error::Compile(diags) => {
                assert!(!diags.is_empty());
                assert!(
                    diags.iter().any(|d| d.loc.line > 0),
                    "expected line > 0 in diags: {diags:?}"
                );
                assert!(diags.iter().any(|d| !d.message.is_empty()));
            }
            other => panic!("expected Compile diags, got {other:?}"),
        }
        assert!(err.has_located_diagnostic() || err.to_string().contains(':'));
    }

    #[test]
    fn empty_functions_is_error_with_location() {
        let src = "// @edition 3\n// no functions\n";
        let err = compile(src, Some("empty.vel")).unwrap_err();
        let diags = err.diagnostics();
        assert!(!diags.is_empty());
        assert!(diags[0].loc.line >= 1);
        assert!(diags[0].message.contains("no callable"));
    }

    #[test]
    fn unknown_function_runtime_fails() {
        let m = compile(SAMPLE, Some("logic.vel")).unwrap();
        let err = m.call("does_not_exist", &[]).unwrap_err();
        assert!(matches!(err, Vs3Error::Runtime { .. }));
        assert!(!err.to_string().is_empty());
    }
}
