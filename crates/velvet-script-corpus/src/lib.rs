//! Corpus programs for Velvet Script (small real samples, not cloned fixtures).

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

/// Distinct VS3 programs covering general logic, state, data, and tasks.
pub static VS3_SAMPLES: &[(&str, &str)] = &[
    (
        "typed_collections",
        r#"// @edition 3
function summarize(values: list) {
    let result: map = {"count": len(values), "last": null}
    if len(values) > 0 { result["last"] = values[len(values) - 1] }
    return result
}
"#,
    ),
    (
        "persistent_state",
        r#"// @edition 3
state { ticks: int = 0 }
function tick(step: int) {
    ticks += step
    return ticks
}
"#,
    ),
    (
        "cooperative_service",
        r#"// @edition 3
function save_profile(profile: map) {
    return yield(["storage.profile.save", profile])
}
"#,
    ),
    (
        "advanced_math",
        r#"// @edition 3
state { random: rng = rng_new(2026) }
function steer(position: vec3, target: vec3, speed: float) {
    return position + normalize(target - position) * speed
}
function sample_terrain(x: float, y: float) {
    return fbm2(x, y, 77, 5)
}
function summarize(values: list) {
    return [mean(values), stddev(values), quantile(values, 0.9)]
}
"#,
    ),
];

/// Number of embedded VS3 samples.
pub const VS3_SAMPLE_COUNT: usize = 4;

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
        assert!(
            m.item_count() >= 1 || src.contains("fn "),
            "empty sample {i}"
        );
        ok += 1;
    }
    ok
}

/// Compile every VS3 corpus program through the official semantic frontend.
pub fn run_vs3_corpus() -> usize {
    VS3_SAMPLES
        .iter()
        .map(|(name, source)| {
            velvet_script_vs3::compile(source, Some(name))
                .unwrap_or_else(|error| panic!("VS3 corpus `{name}` failed: {error}"));
            1usize
        })
        .sum()
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

    #[test]
    fn vs3_corpus_compiles_and_exercises_persistent_state() {
        assert_eq!(VS3_SAMPLE_COUNT, VS3_SAMPLES.len());
        assert_eq!(run_vs3_corpus(), VS3_SAMPLE_COUNT);

        let (_, source) = VS3_SAMPLES[1];
        let module = velvet_script_vs3::compile(source, Some("persistent_state")).unwrap();
        let mut session = module.session().unwrap();
        assert_eq!(
            session.call("tick", &[velvet_script_vs3::int(2)]).unwrap(),
            velvet_script_vs3::int(2)
        );
        assert_eq!(
            session.call("tick", &[velvet_script_vs3::int(3)]).unwrap(),
            velvet_script_vs3::int(5)
        );

        let (_, source) = VS3_SAMPLES[3];
        let module = velvet_script_vs3::compile(source, Some("advanced_math")).unwrap();
        assert_eq!(
            module
                .call(
                    "steer",
                    &[
                        velvet_script_vs3::Value::Vec3([0.0, 0.0, 0.0]),
                        velvet_script_vs3::Value::Vec3([3.0, 0.0, 0.0]),
                        velvet_script_vs3::Value::Float(2.0),
                    ],
                )
                .unwrap(),
            velvet_script_vs3::Value::Vec3([2.0, 0.0, 0.0])
        );
    }
}
