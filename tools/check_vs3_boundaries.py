#!/usr/bin/env python3
"""VS3 boundary gate: the language core must not depend on engine subsystems.

VS3 is a portable language; VelvetEngine is one host among others
(docs/language/VS3_ROADMAP.md, phase 0). This check fails when:

1. A core language crate gains a dependency on an engine crate
   (velvet-math, velvet-ecs, velvet-render, ...). Only the designated
   adapter (velvet-script-vs3-engine, math only) and plain crates.io
   deps are allowed next to the script pipeline + velvet-crypto.
2. Engine module paths (velvet_math::, velvet_ecs::, ...) appear in core
   crate sources outside the adapter.
3. Legacy presentation natives (present_show, set_bg, ui_flag, ...)
   appear outside their confinement: the bytecode/VM definitions, the
   registrable compat module, its tests, and docs.

Usage: python3 tools/check_vs3_boundaries.py (exit 0 = ok, 1 = violations).
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Language-pipeline crates: no engine deps, no engine paths in src.
CORE_CRATES = [
    "velvet-script-vs3",
    "velvet-script-ast",
    "velvet-script-lexer",
    "velvet-script-parser",
    "velvet-script-hir",
    "velvet-script-compiler",
    "velvet-script-bytecode",
    "velvet-script-vm",
    "velvet-script-format",
    "velvet-script-resolve",
    "velvet-script-types",
    "velvet-script-syntax",
    "velvet-script-stdlib",
    "velvet-script-corpus",
    "velvet-script-i18n",
    "velvet-script-layers",
]

# velvet-* deps a core crate may keep (pipeline siblings + crypto utils).
ALLOWED_CORE_DEPS = {"velvet-crypto"}

# velvet-script-vs3-engine is the designated adapter: script stack + pure
# simulation math only (velvet-math conversions, velvet-terrain/velvet-liquid
# reference hosts), never gameplay subsystems (render/ecs/play/...).
ADAPTER_CRATE = "velvet-script-vs3-engine"
ADAPTER_ALLOWED_DEPS = {"velvet-math", "velvet-terrain", "velvet-liquid"}

# velvet-vs3-app is language-side runtime: script stack only for now.
APP_CRATE = "velvet-vs3-app"

# Engine module paths forbidden in core sources (adapter: math only).
ENGINE_PATHS = [
    "velvet_math::",
    "velvet_ecs::",
    "velvet_render::",
    "velvet_audio::",
    "velvet_play::",
    "velvet_rpg::",
    "velvet_app::",
    "velvet_core::",
    "velvet_ui::",
    "velvet_scene",
    "velvet_input::",
    "velvet_assets::",
]

# Legacy presentation natives: only here (definitions, compat module,
# CLI natives listing, tests, docs).
LEGACY_NATIVES = ["present_show", "present_hide", "set_bg", "ui_flag"]
LEGACY_ALLOW_RS = [
    "crates/velvet-script-bytecode/src/lib.rs",
    "crates/velvet-script-vm/src/stdlib.rs",
    "crates/velvet-script-vm/src/vm.rs",
    "crates/velvet-script-vm/src/present_host.rs",
    # Own Rust method of the VS2 Classic host, not the VS3 native string.
    "crates/velvet-script-vm/src/vs2_host.rs",
    "crates/velvet-script-vs3/src/lib.rs",
    "crates/velvet-script-vs3-engine/src/compat.rs",
    "crates/velvet-cli/src/vs3_cmd.rs",
    # Deprecation-warning tests name the legacy natives on purpose.
    "crates/velvet-script-compiler/src/compiler.rs",
    # Test exercising the legacy path on purpose.
    "crates/velvet-story/src/classic_vel_tests.rs",
]
LEGACY_ALLOW_DIRS = [
    "crates/velvet-script-vs3/tests",
    "crates/velvet-script-vm/tests",
    "crates/velvet-integration-tests",
    "docs",
]

DEP_LINE = re.compile(r"^(velvet-[a-z0-9-]+)\s*=")


@dataclass
class Finding:
    path: str
    line: int
    message: str


def workspace_dep_names(crate: str) -> list[tuple[str, int]]:
    """(dep name, line) for workspace velvet-* deps in a crate Cargo.toml."""
    path = ROOT / "crates" / crate / "Cargo.toml"
    out: list[tuple[str, int]] = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return out
    in_dev = False
    for i, raw in enumerate(lines, start=1):
        line = raw.strip()
        if line.startswith("["):
            in_dev = "dev-dependencies" in line
            continue
        if in_dev:
            continue
        m = DEP_LINE.match(line)
        if m and m.group(1).startswith("velvet-"):
            out.append((m.group(1), i))
    return out


def check_cargo_toml(findings: list[Finding]) -> None:
    for crate in CORE_CRATES:
        for dep, line in workspace_dep_names(crate):
            if dep.startswith("velvet-script-") or dep in ALLOWED_CORE_DEPS:
                continue
            findings.append(
                Finding(
                    f"crates/{crate}/Cargo.toml",
                    line,
                    f"core crate depends on engine crate `{dep}`",
                )
            )
    for dep, line in workspace_dep_names(ADAPTER_CRATE):
        if dep.startswith("velvet-script-") or dep in ADAPTER_ALLOWED_DEPS:
            continue
        findings.append(
            Finding(
                f"crates/{ADAPTER_CRATE}/Cargo.toml",
                line,
                f"adapter depends on `{dep}` (only pure sim math allowed)",
            )
        )
    for dep, line in workspace_dep_names(APP_CRATE):
        if dep.startswith("velvet-script-"):
            continue
        findings.append(
            Finding(
                f"crates/{APP_CRATE}/Cargo.toml",
                line,
                f"app runtime depends on `{dep}` (script stack only)",
            )
        )


def check_engine_paths(findings: list[Finding]) -> None:
    for crate in CORE_CRATES + [APP_CRATE]:
        src = ROOT / "crates" / crate / "src"
        if not src.is_dir():
            continue
        for path in sorted(src.rglob("*.rs")):
            rel = path.relative_to(ROOT).as_posix()
            for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                for banned in ENGINE_PATHS:
                    if banned in line:
                        findings.append(
                            Finding(rel, i, f"engine path `{banned}` in language core")
                        )
                        break
    # Adapter: math conversions only.
    src = ROOT / "crates" / ADAPTER_CRATE / "src"
    for path in sorted(src.rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            for banned in ENGINE_PATHS:
                if banned == "velvet_math::":
                    continue
                if banned in line:
                    findings.append(
                        Finding(rel, i, f"non-math engine path `{banned}` in adapter")
                    )
                    break


def legacy_allowed(rel: str) -> bool:
    if rel in LEGACY_ALLOW_RS:
        return True
    return any(rel.startswith(d + "/") or rel == d for d in LEGACY_ALLOW_DIRS)


def check_legacy_natives(findings: list[Finding]) -> None:
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        if legacy_allowed(rel):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError:
            continue
        for i, line in enumerate(lines, start=1):
            for native in LEGACY_NATIVES:
                # Note: `ui_flag` also matches `ui_flag_get`; one report
                # per line is enough, hence the break.
                if native in line:
                    findings.append(
                        Finding(rel, i, f"legacy native `{native}` outside confinement")
                    )
                    break


def main() -> int:
    findings: list[Finding] = []
    check_cargo_toml(findings)
    check_engine_paths(findings)
    check_legacy_natives(findings)
    if findings:
        print("vs3-boundary: FAILED")
        for f in findings:
            print(f"  {f.path}:{f.line}: {f.message}")
        print(f"{len(findings)} violation(s) found.")
        return 1
    print("vs3-boundary: ok (core clean, adapter math-only, natives confined)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
