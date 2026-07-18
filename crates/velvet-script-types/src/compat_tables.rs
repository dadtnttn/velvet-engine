//! Real VS1 → VS2 name aliases (only documented renames).

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
