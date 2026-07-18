#!/usr/bin/env python3
"""Strip path_parse_N and rewrite DiagCode catalog + syntax clone tests."""
from __future__ import annotations
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def clean_hir() -> None:
    p = ROOT / "crates/velvet-script-hir/src/lib.rs"
    t = p.read_text(encoding="utf-8")
    t2, n = re.subn(
        r"(?ms)^\s*#\[test\]\s*\n\s*fn path_parse_\d+\(\) \{.*?\n\s*\}\n",
        "",
        t,
    )
    print(f"hir: stripped {n} path_parse_N")
    if "path_parse_exact" not in t2:
        inject = """
    #[test]
    fn path_parse_exact() {
        let p = HirPath::parse("foo::bar::Baz");
        assert_eq!(p.segs.len(), 3);
        assert_eq!(p.display(), "foo::bar::Baz");
        let single = HirPath::parse("solo");
        assert_eq!(single.segs.len(), 1);
        assert_eq!(single.display(), "solo");
        let empty = HirPath::parse("");
        assert!(empty.segs.is_empty());
    }
"""
        m = re.search(r"fn ty_display\(\) \{.*?\n    \}\n", t2, re.S)
        if m:
            t2 = t2[: m.end()] + inject + t2[m.end() :]
            print("hir: injected path_parse_exact")
    p.write_text(t2, encoding="utf-8")
    print(f"hir lines: {t2.count(chr(10))}")


DIAG_REPLACEMENT = r'''
/// Stable diagnostic codes with real messages (not E0001..E0500 padding).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum DiagCode {
    /// Unexpected token.
    UnexpectedToken = 1,
    /// Unterminated string.
    UnterminatedString = 2,
    /// Invalid number literal.
    InvalidNumber = 3,
    /// Unknown keyword / reserved misuse.
    UnknownKeyword = 4,
    /// Unmatched brace / paren / bracket.
    UnmatchedDelimiter = 5,
    /// Expected expression.
    ExpectedExpr = 6,
    /// Expected type.
    ExpectedType = 7,
    /// Duplicate definition.
    DuplicateDefinition = 8,
    /// Unresolved name.
    UnresolvedName = 9,
    /// Type mismatch.
    TypeMismatch = 10,
    /// Invalid jump / scene target.
    InvalidJumpTarget = 11,
    /// Missing `main` or entry.
    MissingEntry = 12,
    /// Feature not yet lowered (struct/enum/field/mod…).
    UnsupportedHir = 13,
    /// Internal compiler bug placeholder.
    Internal = 99,
}

impl DiagCode {
    /// Numeric code.
    pub fn code(self) -> u16 {
        self as u16
    }

    /// Short label `VS####`.
    pub fn label(self) -> String {
        format!("VS{:04}", self.code() as u32)
    }

    /// Human-readable message (not just the label).
    pub fn message(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected token",
            Self::UnterminatedString => "unterminated string literal",
            Self::InvalidNumber => "invalid number literal",
            Self::UnknownKeyword => "unknown or misplaced keyword",
            Self::UnmatchedDelimiter => "unmatched delimiter",
            Self::ExpectedExpr => "expected expression",
            Self::ExpectedType => "expected type",
            Self::DuplicateDefinition => "duplicate definition",
            Self::UnresolvedName => "unresolved name",
            Self::TypeMismatch => "type mismatch",
            Self::InvalidJumpTarget => "invalid jump or scene target",
            Self::MissingEntry => "missing entry point",
            Self::UnsupportedHir => "construct not yet supported in lowering",
            Self::Internal => "internal compiler error",
        }
    }

    /// All real codes (dense catalog, not 500 placeholders).
    pub fn all() -> &'static [Self] {
        &[
            Self::UnexpectedToken,
            Self::UnterminatedString,
            Self::InvalidNumber,
            Self::UnknownKeyword,
            Self::UnmatchedDelimiter,
            Self::ExpectedExpr,
            Self::ExpectedType,
            Self::DuplicateDefinition,
            Self::UnresolvedName,
            Self::TypeMismatch,
            Self::InvalidJumpTarget,
            Self::MissingEntry,
            Self::UnsupportedHir,
            Self::Internal,
        ]
    }

    /// Lookup by numeric code.
    pub fn from_code(n: u16) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.code() == n)
    }
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_roundtrip() {
        for k in Keyword::all() {
            assert_eq!(Keyword::from_str(k.as_str()), Some(*k));
        }
        assert!(Keyword::all().len() > 20);
        assert!(Keyword::all().len() < 120);
    }

    #[test]
    fn edition_v2() {
        assert_eq!(Edition::from_u32(2), Some(Edition::V2));
        assert_eq!(Edition::latest(), Edition::V2);
    }

    #[test]
    fn diag_catalog_small_and_messaged() {
        assert!(DiagCode::all().len() < 40);
        assert!(DiagCode::all().len() >= 10);
        for d in DiagCode::all() {
            assert!(!d.message().is_empty());
            assert!(d.label().starts_with("VS"));
            assert_eq!(DiagCode::from_code(d.code()), Some(*d));
        }
        assert_eq!(DiagCode::TypeMismatch.message(), "type mismatch");
        assert_eq!(DiagCode::UnsupportedHir.code(), 13);
        // no 500-wide fake range
        assert!(DiagCode::from_code(500).is_none());
        assert!(DiagCode::from_code(1).is_some());
    }

    #[test]
    fn op_precedence() {
        assert!(Op::Mul.precedence() > Op::Add.precedence());
        assert!(Op::Add.precedence() > Op::Eq.precedence());
        assert_eq!(Op::Assign.as_str(), "=");
    }
}
'''


def clean_syntax() -> None:
    p = ROOT / "crates/velvet-script-syntax/src/lib.rs"
    t = p.read_text(encoding="utf-8")
    # keep everything before DiagCode enum
    idx = t.find("/// Stable diagnostic codes.")
    if idx < 0:
        idx = t.find("pub enum DiagCode")
    if idx < 0:
        raise SystemExit("DiagCode not found")
    head = t[:idx].rstrip() + "\n"
    out = head + "\n" + DIAG_REPLACEMENT
    p.write_text(out, encoding="utf-8")
    print(f"syntax lines: {out.count(chr(10))}")


def main() -> None:
    clean_hir()
    clean_syntax()
    print("done")


if __name__ == "__main__":
    main()
