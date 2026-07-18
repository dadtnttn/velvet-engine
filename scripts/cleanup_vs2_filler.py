#!/usr/bin/env python3
"""Remove artificial LOC padding from velvet-script-* crates."""
from __future__ import annotations
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"


def write(rel: str, text: str) -> None:
    p = CRATES / rel
    p.write_text(text.replace("\r\n", "\n"), encoding="utf-8")
    print(f"  cleaned {rel} ({text.count(chr(10))+1} lines)")


# --- vs2_lower: keep only real helpers ---
write(
    "velvet-script-compiler/src/vs2_lower.rs",
    '''//! VS2 lowering helpers from HIR modules to story-ish ops.

#![allow(missing_docs)]

use velvet_script_hir::{HirItem, HirModule};

/// Count story-like items in module.
pub fn count_story_ops(m: &HirModule) -> usize {
    let mut n = 0;
    for it in &m.items {
        if let HirItem::Scene(sc) = it {
            n += sc.body.len() + 1;
        }
    }
    n
}

/// List scene names.
pub fn scene_names(m: &HirModule) -> Vec<String> {
    m.items
        .iter()
        .filter_map(|it| match it {
            HirItem::Scene(s) => Some(s.name.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the module has any narrative (scene) items.
pub fn is_story_module(m: &HirModule) -> bool {
    m.items.iter().any(|it| matches!(it, HirItem::Scene(_)))
}

/// Character names declared in the module.
pub fn character_names(m: &HirModule) -> Vec<String> {
    m.items
        .iter()
        .filter_map(|it| match it {
            HirItem::Character(c) => Some(c.name.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::{HirId, HirItem, HirModule, HirScene, HirSpan};

    #[test]
    fn counts_scenes() {
        let mut m = HirModule::new(2);
        m.items.push(HirItem::Scene(HirScene {
            id: HirId(1),
            name: "start".into(),
            body: vec![],
            span: HirSpan::unknown(),
        }));
        assert_eq!(scene_names(&m), vec!["start".to_string()]);
        assert!(is_story_module(&m));
        assert_eq!(count_story_ops(&m), 1);
    }
}
''',
)

# --- compat_tables: real VS1→VS2 aliases only ---
write(
    "velvet-script-types/src/compat_tables.rs",
    '''//! Real VS1 → VS2 name aliases (only documented renames).

#![allow(missing_docs)]

/// Documented keyword / API renames from Velvet Script edition 1 to 2.
/// Empty rows and numbered placeholders are intentionally **not** included.
pub static VS1_ALIASES: &[(&str, &str)] = &[
    // story surface
    ("label", "scene"),
    ("jump", "jump"),
    ("menu", "menu"),
    ("say", "say"),
    // types
    ("string", "str"),
    ("int", "i32"),
    ("float", "f64"),
    ("boolean", "bool"),
    // runtime helpers that changed names
    ("print_line", "print"),
    ("length", "len"),
];

/// Look up a VS1 name; returns VS2 name if renamed.
pub fn map_vs1(name: &str) -> Option<&'static str> {
    VS1_ALIASES
        .iter()
        .find(|(a, _)| *a == name)
        .map(|(_, b)| *b)
}

/// True if `name` is an unknown padded alias form (`alias_N`).
pub fn is_placeholder_alias(name: &str) -> bool {
    name.starts_with("alias_")
        && name
            .strip_prefix("alias_")
            .map(|s| s.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_aliases_only() {
        assert_eq!(map_vs1("string"), Some("str"));
        assert_eq!(map_vs1("label"), Some("scene"));
        assert_eq!(map_vs1("alias_0"), None);
        assert!(is_placeholder_alias("alias_0"));
        assert!(!is_placeholder_alias("string"));
        // no hundreds of fake rows
        assert!(VS1_ALIASES.len() < 50);
        assert!(!VS1_ALIASES.iter().any(|(a, _)| a.starts_with("alias_")));
    }
}
''',
)

print("phase1 written")
