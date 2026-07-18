#!/usr/bin/env python3
"""Strip numbered filler functions from velvet-script sources; rewrite weak tests."""
from __future__ import annotations
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"


def strip_numbered_fns(text: str, names: list[str]) -> str:
    """Remove `pub fn name_N(...) { ... }` blocks for each base name."""
    for base in names:
        # multi-line function bodies (non-greedy, balanced-ish via brace count)
        pattern = re.compile(
            rf"(?m)^(?:///[^\n]*\n)*pub fn {re.escape(base)}_\d+[^\n]*\{{",
        )
        while True:
            m = pattern.search(text)
            if not m:
                break
            start = m.start()
            # find opening brace of signature
            brace = text.find("{", m.end() - 1)
            if brace < 0:
                break
            depth = 0
            i = brace
            while i < len(text):
                c = text[i]
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        # swallow trailing newline
                        if i < len(text) and text[i] == "\n":
                            i += 1
                        text = text[:start] + text[i:]
                        break
                i += 1
            else:
                break
    return text


def strip_static_array_padding(text: str, array_name: str, keep_real: bool = True) -> str:
    """Not used generically."""
    return text


def clean_vs2_format() -> None:
    p = CRATES / "velvet-script-format/src/vs2_format.rs"
    text = p.read_text(encoding="utf-8")
    text = strip_numbered_fns(text, ["format_fixture"])
    # replace weak test
    text = re.sub(
        r"fn fixture_0\(\)[^\n]*\{[^}]*\}",
        '''fn format_sample_fn() {
        let src = "fn main(){\\nlet x=1;\\n}\\n";
        let out = format_vs2(src, &Vs2FormatOptions::default());
        assert!(out.contains("fn main()"));
        assert!(out.contains('{'));
        let twice = format_vs2(&out, &Vs2FormatOptions::default());
        assert_eq!(out, twice);
    }''',
        text,
        count=1,
    )
    # add one real sample helper if missing
    if "pub fn format_sample" not in text:
        text = text.replace(
            "pub fn reject_python_style",
            '''/// Sample used by tests (not N clones).
pub fn format_sample_source() -> &'static str {
    "// @edition 2\\nfn main() {\\n    let x = 1;\\n    return x;\\n}\\n"
}

pub fn reject_python_style''',
        )
    p.write_text(text, encoding="utf-8")
    print(f"  format {p.stat().st_size}")


def clean_vs2_ide() -> None:
    p = CRATES / "velvet-script-lsp/src/vs2_ide.rs"
    text = p.read_text(encoding="utf-8")
    text = strip_numbered_fns(text, ["local_completions"])
    # add one real helper for module-local completions from names
    if "pub fn local_completions(" not in text:
        insert = '''
/// Completions for names discovered in a module (scenes, layers, msg keys).
pub fn local_completions(
    mod_name: &str,
    scenes: &[&str],
    layers: &[&str],
    msg_keys: &[&str],
) -> Vec<Vs2Completion> {
    let mut v = Vec::new();
    v.push(Vs2Completion::fn_item(&format!("{mod_name}_main"), "module entry"));
    for s in scenes {
        v.push(Vs2Completion {
            label: (*s).into(),
            kind: Vs2CompletionKind::Scene,
            detail: "scene".into(),
            insert: (*s).into(),
        });
    }
    for l in layers {
        v.push(Vs2Completion {
            label: (*l).into(),
            kind: Vs2CompletionKind::Layer,
            detail: "layer".into(),
            insert: format!("LayerId::new(\\\"{l}\\\")"),
        });
    }
    for k in msg_keys {
        v.push(Vs2Completion {
            label: (*k).into(),
            kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(),
            insert: format!("t!(\\\"{k}\\\")"),
        });
    }
    v
}

'''
        text = text.replace("pub fn classify_word", insert + "pub fn classify_word")
    p.write_text(text, encoding="utf-8")
    print("  ide cleaned")


def clean_vs2_codegen() -> None:
    p = CRATES / "velvet-script-compiler/src/vs2_codegen.rs"
    if not p.exists():
        return
    text = p.read_text(encoding="utf-8")
    text = strip_numbered_fns(
        text, ["pattern_say", "pattern_load_op", "pattern_layer", "emit_"]
    )
    # emit_* helpers are useful - oh no I stripped emit_ too!
    # Need to re-read and only strip pattern_*
    print("  codegen: redoing carefully")
    text = p.read_text(encoding="utf-8")
    # restore from git if we destroyed emit helpers - we'll re-read after careful strip
    # Actually first run already may have broken if we write - we haven't written yet.
    text = strip_numbered_fns(text, ["pattern_say", "pattern_load_op", "pattern_layer"])
    # keep one pattern helper
    if "pub fn pattern_say(" not in text:
        helper = '''
/// Emit load-msg + say (single helper, not N clones).
pub fn pattern_say(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Emit push + show layer.
pub fn pattern_layer(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

'''
        # insert before tests
        text = text.replace("#[cfg(test)]", helper + "#[cfg(test)]")
    # fix tests that call pattern_say_0
    text = text.replace("pattern_say_0", "pattern_say")
    text = text.replace("pattern_layer_0", "pattern_layer")
    p.write_text(text, encoding="utf-8")
    print("  codegen cleaned")


def clean_vs2_host() -> None:
    p = CRATES / "velvet-script-vm/src/vs2_host.rs"
    text = p.read_text(encoding="utf-8")
    text = strip_numbered_fns(text, ["scenario", "run_scenario"])
    if "pub fn scenario(" not in text:
        helper = '''
/// Build a tiny dialogue + layer scenario (one helper).
pub fn scenario(speaker: &str, msg_key: &str, layer: &str, line: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, line);
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario() -> Vs2Host {
    let mut vm = scenario("hero", "k0", "hud", "line-0");
    let _ = vm.run(32);
    vm.host
}

'''
        text = text.replace("#[cfg(test)]", helper + "#[cfg(test)]")
    text = text.replace("run_scenario_0", "run_scenario")
    text = text.replace("scenario_0", "scenario")
    p.write_text(text, encoding="utf-8")
    print("  host cleaned")


def clean_resolve() -> None:
    for rel in [
        "velvet-script-resolve/src/scope.rs",
        "velvet-script-resolve/src/symbols.rs",
        "velvet-script-resolve/src/imports.rs",
        "velvet-script-resolve/src/diagnostics.rs",
        "velvet-script-resolve/src/prelude_names.rs",
        "velvet-script-resolve/src/resolve.rs",
    ]:
        p = CRATES / rel
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        text = strip_numbered_fns(
            text,
            [
                "scope_kind_label",
                "make_sym",
                "chain_graph",
                "diag_ext",
                "prelude_batch",
                "resolve_smoke",
            ],
        )
        # strip prelude_ext_N entries from PRELUDE static
        text = re.sub(
            r'\s*PreludeEntry \{ name: "prelude_ext_\d+", kind: "fn", ty_hint: "fn" \},\n',
            "",
            text,
        )
        # strip E2xxx resolve_ext catalog spam, keep first 20 real codes
        text = re.sub(r'\s*"E2\d{3}_resolve_ext",\n', "", text)
        # add single factories where useful
        if rel.endswith("symbols.rs") and "pub fn make_sym(" not in text:
            text += '''
/// Construct a symbol (replaces numbered make_sym_N clones).
pub fn make_sym(name: &str, module: &str, kind: SymbolKind) -> Symbol {
    Symbol::new(SymbolId(0), name, kind, module)
}
'''
        if rel.endswith("imports.rs") and "pub fn chain_graph(" not in text:
            text += '''
/// Build a linear import chain graph of length `len`.
pub fn chain_graph(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let n = len.max(1);
    for i in 0..n.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None,
            glob: false,
        });
    }
    g
}
'''
        if rel.endswith("scope.rs") and "pub fn scope_kind_label(" not in text:
            text += '''
pub fn scope_kind_label(k: ScopeKind) -> &'static str {
    match k {
        ScopeKind::Module => "module",
        ScopeKind::Function => "function",
        ScopeKind::Block => "block",
        ScopeKind::Scene => "scene",
        ScopeKind::Screen => "screen",
        ScopeKind::Impl => "impl",
        ScopeKind::MatchArm => "match_arm",
        ScopeKind::Loop => "loop",
    }
}
'''
        p.write_text(text, encoding="utf-8")
        print(f"  resolve {rel}")


def clean_types_tests() -> None:
    p = CRATES / "velvet-script-types/src/lib.rs"
    text = p.read_text(encoding="utf-8")
    # remove typeck_module_N weak tests
    text = re.sub(
        r"(?ms)^\s*#\[test\]\s*\n\s*fn typeck_module_\d+\(\) \{.*?\n\s*\}\n",
        "",
        text,
    )
    # ensure strong tests exist after typeck_fn_scene
    if "typeck_scene_and_fn_exact" not in text:
        strong = '''
    #[test]
    fn typeck_scene_and_fn_exact() {
        let src = "scene intro {}\\npub fn main() {}\\n";
        let (m, _) = lower_source_heuristic(src, 2);
        let errs = typeck_module(&m);
        assert!(errs.is_empty(), "unexpected type errors: {errs:?}");
        assert_eq!(m.item_count(), 2);
    }

    #[test]
    fn typeck_empty_module_exact() {
        let m = HirModule::new(2);
        assert_eq!(typeck_module(&m).len(), 0);
    }
'''
        # insert before closing of tests mod - find last }
        # append before final brace of file's tests
        idx = text.rfind("#[test]\n    fn typeck_fn_scene")
        if idx < 0:
            idx = text.rfind("fn typeck_fn_scene")
        if idx >= 0:
            # find end of typeck_fn_scene function
            end = text.find("\n    #[test]", idx + 10)
            if end < 0:
                end = text.find("\n}", idx)
            text = text[:end] + "\n" + strong + text[end:]
        else:
            text = text.rstrip() + "\n" + strong + "\n"
    p.write_text(text, encoding="utf-8")
    print("  types tests cleaned")


def clean_stdlib() -> None:
    p = CRATES / "velvet-script-stdlib/src/lib.rs"
    if not p.exists():
        return
    text = p.read_text(encoding="utf-8")
    # remove abs_1 style clone entries
    text = re.sub(
        r"\s*StdFn \{ name: \"\w+_\d+\".*?\},\n",
        "",
        text,
    )
    p.write_text(text, encoding="utf-8")
    print("  stdlib cleaned")


def clean_i18n_layers_if_needed() -> None:
    for rel in [
        "velvet-script-i18n/src/lib.rs",
        "velvet-script-layers/src/lib.rs",
        "velvet-script-syntax/src/lib.rs",
        "velvet-script-hir/src/lib.rs",
        "velvet-script-corpus/src/lib.rs",
    ]:
        p = CRATES / rel
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        orig = text
        # generic numbered fn strip for known pad prefixes
        text = strip_numbered_fns(
            text,
            [
                "sample",
                "fixture",
                "marker",
                "pad",
                "doc_item",
                "keyword_batch",
                "diag_code",
            ],
        )
        # remove SAMPLE_COUNT inflated tests that only count
        if "SAMPLE_COUNT" in text and text.count("fn sample_") > 20:
            text = strip_numbered_fns(text, ["sample"])
        if text != orig:
            p.write_text(text, encoding="utf-8")
            print(f"  cleaned {rel}")


def clean_bytecode_tests() -> None:
    p = CRATES / "velvet-script-bytecode/src/opcodes_vs2.rs"
    if not p.exists():
        return
    text = p.read_text(encoding="utf-8")
    # leave enum but strip op_nop style mass tests if identical
    # only remove if there are hundreds of one-liners
    count = len(re.findall(r"fn op_\w+\(\)", text))
    if count > 40:
        # keep first few and a batch test
        # strip all fn op_* tests inside mod tests
        text2 = re.sub(
            r"(?ms)^\s*#\[test\]\s*\n\s*fn op_\w+\(\) \{.*?\n\s*\}\n",
            "",
            text,
        )
        if "fn opcodes_have_names" not in text2:
            text2 = text2.replace(
                "#[cfg(test)]\nmod tests {",
                '''#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opcodes_have_names() {
        assert_eq!(OpVs2::Nop.name(), "Nop");
        assert_eq!(OpVs2::Say.name(), "Say");
        assert_eq!(OpVs2::PushLayer.name(), "PushLayer");
        assert!(OpVs2::from_u16(0).is_some());
    }
''',
            )
        p.write_text(text2, encoding="utf-8")
        print(f"  bytecode tests reduced (was {count} op_* tests)")


def main() -> None:
    print("stripping filler…")
    clean_vs2_format()
    clean_vs2_ide()
    clean_vs2_codegen()
    clean_vs2_host()
    clean_resolve()
    clean_types_tests()
    clean_stdlib()
    clean_i18n_layers_if_needed()
    clean_bytecode_tests()
    # re-apply vs2_lower and compat if needed
    lower = CRATES / "velvet-script-compiler/src/vs2_lower.rs"
    if "story_marker_" in lower.read_text(encoding="utf-8"):
        print("  WARNING: story_marker still present — re-run phase1")
    print("done")


if __name__ == "__main__":
    main()
