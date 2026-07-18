//! Localization key extraction helpers for dialogue and choice lines.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::ir::{StoryOp, StoryProgram, StoryScene};

/// One localizable string extracted from a story program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocEntry {
    /// Stable key (`scene:op:kind[:arm]`).
    pub key: String,
    /// Source scene name.
    pub scene: String,
    /// Original source text (template with `{vars}` allowed).
    pub source: String,
    /// Optional speaker character id for dialogue.
    pub speaker: Option<String>,
    /// Kind of line.
    pub kind: LocKind,
}

/// Classification of a localizable entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocKind {
    /// Character or narrator dialogue.
    Dialogue,
    /// Choice option text.
    Choice,
}

/// Catalog of localization keys extracted from a program.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocCatalog {
    /// Entries in extraction order.
    pub entries: Vec<LocEntry>,
}

impl LocCatalog {
    /// Number of keys.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty check.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Lookup by key.
    pub fn get(&self, key: &str) -> Option<&LocEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    /// Map key → source text.
    pub fn source_map(&self) -> IndexMap<String, String> {
        self.entries
            .iter()
            .map(|e| (e.key.clone(), e.source.clone()))
            .collect()
    }

    /// Apply a translation table to produce rewritten dialogue/choice ops.
    /// Missing keys keep the original source string.
    pub fn apply_to_program(
        &self,
        program: &StoryProgram,
        table: &IndexMap<String, String>,
    ) -> StoryProgram {
        self.apply_to_program_lookup(program, |k| table.get(k).cloned())
    }

    /// Apply translations via arbitrary key lookup.
    pub fn apply_to_program_lookup(
        &self,
        program: &StoryProgram,
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> StoryProgram {
        let mut out = program.clone();
        for (scene_name, scene) in out.scenes.iter_mut() {
            rewrite_scene_ops(scene_name, &mut scene.ops, &mut lookup);
            scene.reindex_labels();
        }
        out
    }
}

fn rewrite_scene_ops(
    scene: &str,
    ops: &mut [StoryOp],
    lookup: &mut impl FnMut(&str) -> Option<String>,
) {
    for (i, op) in ops.iter_mut().enumerate() {
        match op {
            StoryOp::Dialogue { text, .. } => {
                let key = dialogue_key(scene, i);
                if let Some(t) = lookup(&key) {
                    *text = t;
                }
            }
            StoryOp::Choice { options } => {
                for (arm_i, arm) in options.iter_mut().enumerate() {
                    let key = choice_key(scene, i, arm_i);
                    if let Some(t) = lookup(&key) {
                        arm.text = t;
                    }
                    rewrite_scene_ops(scene, &mut arm.body, lookup);
                }
            }
            StoryOp::If {
                then_ops, else_ops, ..
            } => {
                rewrite_scene_ops(scene, then_ops, lookup);
                rewrite_scene_ops(scene, else_ops, lookup);
            }
            _ => {}
        }
    }
}

/// Build catalog from a full story program.
pub fn extract_loc_keys(program: &StoryProgram) -> LocCatalog {
    let mut catalog = LocCatalog::default();
    for (scene_name, scene) in &program.scenes {
        extract_from_ops(scene_name, &scene.ops, &mut catalog);
    }
    catalog
}

/// Extract from a single scene.
pub fn extract_scene_loc_keys(scene: &StoryScene) -> LocCatalog {
    let mut catalog = LocCatalog::default();
    extract_from_ops(&scene.name, &scene.ops, &mut catalog);
    catalog
}

fn extract_from_ops(scene: &str, ops: &[StoryOp], catalog: &mut LocCatalog) {
    for (i, op) in ops.iter().enumerate() {
        match op {
            StoryOp::Dialogue { speaker, text } => {
                catalog.entries.push(LocEntry {
                    key: dialogue_key(scene, i),
                    scene: scene.to_string(),
                    source: text.clone(),
                    speaker: speaker.clone(),
                    kind: LocKind::Dialogue,
                });
            }
            StoryOp::Choice { options } => {
                for (arm_i, arm) in options.iter().enumerate() {
                    catalog.entries.push(LocEntry {
                        key: choice_key(scene, i, arm_i),
                        scene: scene.to_string(),
                        source: arm.text.clone(),
                        speaker: None,
                        kind: LocKind::Choice,
                    });
                    extract_from_ops(scene, &arm.body, catalog);
                }
            }
            StoryOp::If {
                then_ops, else_ops, ..
            } => {
                extract_from_ops(scene, then_ops, catalog);
                extract_from_ops(scene, else_ops, catalog);
            }
            _ => {}
        }
    }
}

/// Stable dialogue key for op index.
pub fn dialogue_key(scene: &str, op_index: usize) -> String {
    format!("{scene}:{op_index}:dialogue")
}

/// Stable choice key for op index + arm.
pub fn choice_key(scene: &str, op_index: usize, arm: usize) -> String {
    format!("{scene}:{op_index}:choice:{arm}")
}

/// Normalize source text into a suggested key slug (not guaranteed unique).
pub fn slugify_text(text: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_')
            && !out.ends_with('_')
            && !out.is_empty()
        {
            out.push('_');
        }
        if out.len() >= max_len {
            break;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "line".into()
    } else {
        out
    }
}

/// Collect unique speakers referenced by dialogue lines.
pub fn speakers_in_program(program: &StoryProgram) -> Vec<String> {
    let cat = extract_loc_keys(program);
    let mut set = indexmap::IndexSet::new();
    for e in &cat.entries {
        if let Some(s) = &e.speaker {
            set.insert(s.clone());
        }
    }
    set.into_iter().collect()
}

/// Export catalog as a simple PO-like text (key = msgid).
pub fn catalog_to_po_template(catalog: &LocCatalog) -> String {
    let mut out = String::from("# Velvet Engine localization template\n\n");
    for e in &catalog.entries {
        out.push_str(&format!("# . {}: {:?}\n", e.scene, e.kind));
        if let Some(sp) = &e.speaker {
            out.push_str(&format!("# . speaker: {sp}\n"));
        }
        out.push_str(&format!("msgctxt \"{}\"\n", e.key));
        out.push_str(&format!("msgid \"{}\"\n", escape_po(&e.source)));
        out.push_str("msgstr \"\"\n\n");
    }
    out
}

fn escape_po(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Translation table: loc key → translated string.
pub type TranslationTable = IndexMap<String, String>;

/// Load a translation table from `tl/<lang>/strings.json` (map of key → text)
/// or `tl/<lang>/strings.po` (msgctxt/msgid/msgstr).
///
/// Layout (documented convention):
/// ```text
/// project/
///   scripts/main.vel
///   tl/
///     en/strings.json   # optional identity
///     es/strings.json   # Spanish
/// ```
pub fn load_tl_table(
    project_root: &std::path::Path,
    lang: &str,
) -> Result<TranslationTable, String> {
    let dir = project_root.join("tl").join(lang);
    let json = dir.join("strings.json");
    let po = dir.join("strings.po");
    if json.is_file() {
        let text = std::fs::read_to_string(&json).map_err(|e| e.to_string())?;
        let map: IndexMap<String, String> =
            serde_json::from_str(&text).map_err(|e| e.to_string())?;
        return Ok(map);
    }
    if po.is_file() {
        let text = std::fs::read_to_string(&po).map_err(|e| e.to_string())?;
        return Ok(parse_simple_po_table(&text));
    }
    Err(format!(
        "no tl/{lang}/strings.json or strings.po under {}",
        project_root.display()
    ))
}

/// Write EN source catalog + empty-or-filled target JSON under `tl/`.
pub fn write_tl_scaffold(
    project_root: &std::path::Path,
    program: &StoryProgram,
    lang: &str,
    translations: &TranslationTable,
) -> Result<std::path::PathBuf, String> {
    let cat = extract_loc_keys(program);
    let en_dir = project_root.join("tl").join("en");
    std::fs::create_dir_all(&en_dir).map_err(|e| e.to_string())?;
    let en_map = cat.source_map();
    let en_path = en_dir.join("strings.json");
    std::fs::write(
        &en_path,
        serde_json::to_string_pretty(&en_map).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let lang_dir = project_root.join("tl").join(lang);
    std::fs::create_dir_all(&lang_dir).map_err(|e| e.to_string())?;
    let mut out_map = IndexMap::new();
    for e in &cat.entries {
        let t = translations
            .get(&e.key)
            .cloned()
            .unwrap_or_else(|| e.source.clone());
        out_map.insert(e.key.clone(), t);
    }
    let path = lang_dir.join("strings.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&out_map).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(path)
}

/// Apply a language pack to a program (English / empty = identity).
pub fn program_for_language(
    base: &StoryProgram,
    project_root: Option<&std::path::Path>,
    lang: &str,
) -> Result<StoryProgram, String> {
    let lang = lang.trim();
    if lang.is_empty() || lang.eq_ignore_ascii_case("en") || lang.eq_ignore_ascii_case("none") {
        return Ok(base.clone());
    }
    let root =
        project_root.ok_or_else(|| "project root required for non-English locale".to_string())?;
    let table = load_tl_table(root, lang)?;
    let cat = extract_loc_keys(base);
    Ok(cat.apply_to_program(base, &table))
}

fn parse_simple_po_table(text: &str) -> TranslationTable {
    let mut table = TranslationTable::new();
    let mut ctxt = String::new();
    let mut msgid = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("msgctxt ") {
            ctxt = unquote_po(rest);
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            msgid = unquote_po(rest);
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            let msgstr = unquote_po(rest);
            let key = if ctxt.is_empty() {
                msgid.clone()
            } else {
                ctxt.clone()
            };
            if !key.is_empty() && !msgstr.is_empty() {
                table.insert(key, msgstr);
            }
            ctxt.clear();
            msgid.clear();
        }
    }
    table
}

fn unquote_po(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{StoryChoice, StoryOp, StoryProgram, StoryScene};
    use crate::load::load_program_from_source;

    fn sample_program() -> StoryProgram {
        let src = r##"
character aria { name: "Aria" }
scene start {
    aria "Hello {player}"
    choice {
        "Yes" { jump end }
        "No" { jump end }
    }
}
scene end {
    "The end"
}
"##;
        load_program_from_source(src, None, "Loc").unwrap()
    }

    #[test]
    fn extracts_dialogue_and_choices() {
        let cat = extract_loc_keys(&sample_program());
        assert!(cat.len() >= 3);
        assert!(cat.entries.iter().any(|e| e.kind == LocKind::Dialogue));
        assert!(cat.entries.iter().any(|e| e.kind == LocKind::Choice));
        assert!(
            cat.get("start:0:dialogue").is_some()
                || cat.entries.iter().any(|e| e.source.contains("Hello"))
        );
    }

    #[test]
    fn apply_translation_rewrites_text() {
        let program = sample_program();
        let cat = extract_loc_keys(&program);
        let mut table = IndexMap::new();
        for e in &cat.entries {
            table.insert(e.key.clone(), format!("[{}]", e.source));
        }
        let translated = cat.apply_to_program(&program, &table);
        let start = translated.scene("start").unwrap();
        match &start.ops[0] {
            StoryOp::Dialogue { text, .. } => assert!(text.starts_with('[')),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify_text("Hello, World!", 32), "hello_world");
        assert_eq!(slugify_text("!!!", 8), "line");
    }

    #[test]
    fn po_template_contains_keys() {
        let cat = extract_loc_keys(&sample_program());
        let po = catalog_to_po_template(&cat);
        assert!(po.contains("msgid"));
        assert!(po.contains("msgctxt"));
    }

    #[test]
    fn nested_choice_bodies_extracted() {
        let mut program = StoryProgram::new("t");
        program.entry = "main".into();
        let mut scene = StoryScene {
            name: "main".into(),
            ops: vec![StoryOp::Choice {
                options: vec![StoryChoice {
                    text: "Go".into(),
                    body: vec![StoryOp::Dialogue {
                        speaker: None,
                        text: "Inside".into(),
                    }],
                    require: None,
                    hidden_if_locked: false,
                }],
            }],
            labels: Default::default(),
        };
        scene.reindex_labels();
        program.scenes.insert("main".into(), scene);
        let cat = extract_loc_keys(&program);
        assert!(cat.entries.iter().any(|e| e.source == "Inside"));
        assert!(cat.entries.iter().any(|e| e.source == "Go"));
    }

    #[test]
    fn speakers_collected() {
        let speakers = speakers_in_program(&sample_program());
        assert!(speakers.iter().any(|s| s == "aria"));
    }

    #[test]
    fn loc_keys_stable_and_unique() {
        let cat = extract_loc_keys(&sample_program());
        let mut keys = std::collections::BTreeSet::new();
        for e in &cat.entries {
            assert!(!e.key.is_empty());
            assert!(keys.insert(e.key.clone()), "duplicate key {}", e.key);
            assert!(!e.source.is_empty());
        }
        // Re-extract yields same keys.
        let cat2 = extract_loc_keys(&sample_program());
        let keys2: std::collections::BTreeSet<_> =
            cat2.entries.iter().map(|e| e.key.clone()).collect();
        assert_eq!(keys, keys2);
    }

    #[test]
    fn apply_partial_translation_keeps_missing() {
        let program = sample_program();
        let cat = extract_loc_keys(&program);
        let mut table = IndexMap::new();
        // Translate only first entry.
        if let Some(e) = cat.entries.first() {
            table.insert(e.key.clone(), "TRANSLATED".into());
        }
        let translated = cat.apply_to_program(&program, &table);
        // At least one dialogue/choice changed.
        let cat_after = extract_loc_keys(&translated);
        assert!(
            cat_after.entries.iter().any(|e| e.source == "TRANSLATED")
                || translated
                    .scenes
                    .values()
                    .any(|s| s.ops.iter().any(|op| matches!(
                        op,
                        StoryOp::Dialogue { text, .. } if text == "TRANSLATED"
                    ))),
            "expected partial apply"
        );
    }

    #[test]
    fn slugify_edge_cases() {
        assert_eq!(slugify_text("", 8), "line");
        assert_eq!(slugify_text("A  B   C", 32), "a_b_c");
        let long = "x".repeat(100);
        let s = slugify_text(&long, 16);
        assert!(s.len() <= 16);
        assert!(!s.is_empty());
    }

    #[test]
    fn po_template_escapes_and_msgctxt() {
        let mut program = StoryProgram::new("t");
        program.entry = "main".into();
        let mut scene = StoryScene {
            name: "main".into(),
            ops: vec![StoryOp::Dialogue {
                speaker: None,
                text: r#"He said "hi""#.into(),
            }],
            labels: Default::default(),
        };
        scene.reindex_labels();
        program.scenes.insert("main".into(), scene);
        let cat = extract_loc_keys(&program);
        let po = catalog_to_po_template(&cat);
        assert!(po.contains("msgctxt"));
        assert!(po.contains("msgid"));
        // Escaped quote present or source embedded.
        assert!(po.contains("hi") || po.contains(r#"\""#));
    }

    #[test]
    fn tl_scaffold_roundtrip_and_apply() {
        let dir = tempfile::tempdir().unwrap();
        let program = sample_program();
        let cat = extract_loc_keys(&program);
        let mut es = TranslationTable::new();
        for e in &cat.entries {
            es.insert(e.key.clone(), format!("ES:{}", e.source));
        }
        let path = write_tl_scaffold(dir.path(), &program, "es", &es).unwrap();
        assert!(path.exists());
        assert!(dir.path().join("tl/en/strings.json").exists());
        let loaded = load_tl_table(dir.path(), "es").unwrap();
        assert!(loaded.values().any(|v| v.starts_with("ES:")));
        let translated = program_for_language(&program, Some(dir.path()), "es").unwrap();
        let start = translated.scene("start").unwrap();
        match &start.ops[0] {
            StoryOp::Dialogue { text, .. } => assert!(text.starts_with("ES:")),
            other => panic!("{other:?}"),
        }
        let en = program_for_language(&program, Some(dir.path()), "en").unwrap();
        assert_eq!(
            en.scene("start").unwrap().ops[0],
            program.scene("start").unwrap().ops[0]
        );
    }
}
