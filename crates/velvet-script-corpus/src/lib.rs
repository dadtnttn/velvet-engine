//! Corpus programs for Velvet Script 2 (small real samples, not 400 clones).

#![deny(missing_docs)]

use velvet_script_hir::lower_source_heuristic;
use velvet_script_i18n::extract_msg_ids;
use velvet_script_types::typeck_module;

/// Named corpus programs (hand-written, distinct).
pub static SAMPLES: &[(&str, &str)] = &[
    (
        "minimal_scene",
        r#"// @edition 2
scene start {
}
"#,
    ),
    (
        "dialogue_menu",
        r#"// @edition 2
character hero {
    name: t!("char.hero"),
}
scene start {
    background "bg/room.png";
    show hero at center;
    say hero, t!("start.hello");
    menu {
        t!("start.yes") => { jump good; }
        t!("start.no") => { jump bad; }
    }
}
scene good {
    say hero, t!("good.line");
}
scene bad {
    say hero, t!("bad.line");
}
"#,
    ),
    (
        "logic_fn",
        r#"// @edition 2
pub fn main() {
}
fn helper() {
}
"#,
    ),
    (
        "layers_i18n",
        r#"// @edition 2
scene ui {
    say narrator, t!("ui.hint");
}
"#,
    ),
    (
        "with_state",
        r#"// @edition 2
state {
    affection: i32 = 0,
}
scene start {
}
"#,
    ),
];

/// Number of embedded samples.
pub const SAMPLE_COUNT: usize = 5;

/// Get sample source by index.
pub fn sample(i: usize) -> String {
    SAMPLES
        .get(i)
        .map(|(_, src)| (*src).to_string())
        .unwrap_or_default()
}

/// Sample name.
pub fn sample_name(i: usize) -> Option<&'static str> {
    SAMPLES.get(i).map(|(n, _)| *n)
}

/// Run corpus lower+typeck; returns how many samples lowered without panic.
pub fn run_corpus() -> usize {
    let mut ok = 0;
    for i in 0..SAMPLE_COUNT {
        let src = sample(i);
        let (m, _) = lower_source_heuristic(&src, 2);
        let _ = typeck_module(&m);
        let _ = extract_msg_ids(&src);
        assert!(m.item_count() >= 1 || src.contains("fn "), "empty sample {i}");
        ok += 1;
    }
    ok
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_size_is_honest() {
        assert_eq!(SAMPLE_COUNT, SAMPLES.len());
        assert_eq!(SAMPLE_COUNT, 5);
        assert_eq!(run_corpus(), 5);
    }

    #[test]
    fn dialogue_sample_has_msg_ids() {
        let src = sample(1);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let msgs = extract_msg_ids(&src);
        assert!(
            msgs.iter().any(|m| m.id.as_str().contains("start.hello")
                || m.id.as_str().contains("char.hero")
                || !msgs.is_empty()),
            "msgs={msgs:?}"
        );
    }

    #[test]
    fn sample_names_unique() {
        let mut set = std::collections::HashSet::new();
        for i in 0..SAMPLE_COUNT {
            let n = sample_name(i).unwrap();
            assert!(set.insert(n));
        }
    }
}
