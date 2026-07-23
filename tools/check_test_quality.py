#!/usr/bin/env python3
"""Reject high-confidence weak-test patterns in Rust sources.

This is intentionally conservative: it only fails on assertions that cannot
meaningfully fail, self-comparisons, and visual evidence tests accidentally
left in the default test suite.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOTS = (Path("crates"), Path("demos"), Path("examples"))


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    message: str
    source: str


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def compact(value: str) -> str:
    return re.sub(r"\s+", "", value)


def iter_rust_files() -> list[Path]:
    files: list[Path] = []
    for root in ROOTS:
        if root.exists():
            files.extend(root.rglob("*.rs"))
    return sorted(files)


def test_function_ranges(text: str) -> list[tuple[int, int, str, int]]:
    """Return approximate `(start, end, name, fn_line)` test function ranges."""
    ranges: list[tuple[int, int, str, int]] = []
    pattern = re.compile(
        r"#\s*\[\s*test\s*\](?P<attrs>(?:\s*#\s*\[[^\]]+\])*)\s*"
        r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)\s*\{",
        re.MULTILINE,
    )
    for match in pattern.finditer(text):
        depth = 1
        index = match.end()
        in_string = False
        escaped = False
        while index < len(text) and depth:
            char = text[index]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
            else:
                if char == '"':
                    in_string = True
                elif char == "{":
                    depth += 1
                elif char == "}":
                    depth -= 1
            index += 1
        ranges.append(
            (
                match.start(),
                index,
                match.group("name"),
                line_number(text, match.start("name")),
            )
        )
    return ranges


def scan_file(path: Path) -> list[Finding]:
    text = path.read_text(encoding="utf-8")
    findings: list[Finding] = []

    patterns = [
        (
            re.compile(r"assert!\s*\([^;]*?\|\|\s*true\b", re.DOTALL),
            "assertion contains `|| true` and therefore cannot protect behavior",
        ),
        (
            re.compile(
                r"assert!\s*\(\s*(?P<value>[A-Za-z_][A-Za-z0-9_\.]*)\.is_ok\(\)"
                r"\s*\|\|\s*(?P=value)\.is_err\(\)\s*\)",
                re.DOTALL,
            ),
            "assertion accepts both success and failure",
        ),
        (
            re.compile(
                r"assert!\s*\(\s*(?P<value>[A-Za-z_][A-Za-z0-9_\.()]*)\.is_empty\(\)"
                r"\s*\|\|\s*!\s*(?P=value)\.is_empty\(\)",
                re.DOTALL,
            ),
            "assertion accepts both empty and non-empty states",
        ),
        (
            re.compile(
                r"assert!\s*\(\s*!\s*(?P<value>[A-Za-z_][A-Za-z0-9_\.()]*)\.is_empty\(\)"
                r"\s*\|\|\s*(?P=value)\.is_empty\(\)",
                re.DOTALL,
            ),
            "assertion accepts both non-empty and empty states",
        ),
    ]
    for pattern, message in patterns:
        for match in pattern.finditer(text):
            line = line_number(text, match.start())
            source = text.splitlines()[line - 1].strip()
            findings.append(Finding(path, line, message, source))

    # High-confidence self comparisons on a single assertion line.
    for index, raw_line in enumerate(text.splitlines(), 1):
        line = raw_line.strip()
        if "assert_eq!" in line or "assert_ne!" in line:
            match = re.search(r"assert_(eq|ne)!\s*\(\s*(.+?)\s*,\s*(.+?)\s*\)\s*;?$", line)
            if match and compact(match.group(2)) == compact(match.group(3)):
                findings.append(
                    Finding(path, index, "assertion compares an expression with itself", line)
                )

    # Visual evidence is useful, but must not run as a default correctness test.
    lines = text.splitlines()
    for start, _end, name, fn_line in test_function_ranges(text):
        lower = name.lower()
        if "dump" not in lower or not ("png" in lower or "evidence" in lower):
            continue
        prefix = text[max(0, start - 300) : start]
        attrs_and_header = text[start : text.find("{", start) + 1]
        if "#[ignore" not in prefix + attrs_and_header:
            source = lines[fn_line - 1].strip()
            findings.append(
                Finding(
                    path,
                    fn_line,
                    "visual evidence test must be `#[ignore]` and run explicitly",
                    source,
                )
            )

    return findings


def main() -> int:
    findings: list[Finding] = []
    for path in iter_rust_files():
        findings.extend(scan_file(path))

    if findings:
        print("Weak-test quality gate failed:\n")
        for finding in findings:
            print(f"{finding.path}:{finding.line}: {finding.message}")
            print(f"    {finding.source}")
        print(f"\n{len(findings)} high-confidence issue(s) found.")
        return 1

    print(f"test-quality: ok ({len(iter_rust_files())} Rust files scanned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
