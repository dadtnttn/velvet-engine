//! The art manifest, `art/manifest.json`. This module reads it and checks every rule of
//! `docs/formats/ART_MANIFEST.md`, the way `js/packages/assets/src/manifest.ts` does: a refused
//! document lists all of its problems, each with its path, and the first one gives the code.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use crate::error::{issue, ErrorCode, Issue, Refusal};

/// The format tag of an art manifest.
pub const FORMAT: &str = "velvet-art";

/// The version that this crate reads. Any other version is refused.
pub const VERSION: u32 = 1;

/// The largest width or height of a box, in logical pixels.
pub const MAX_SIDE: u32 = 7680;

/// The longest id, in characters.
pub const ID_LIMIT: usize = 128;

/// The longest variant name, in characters.
pub const VARIANT_LIMIT: usize = 32;

/// The kinds of art, in the order of the format. A kind names the folder of the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// A character's picture for dialogue or menus.
    Portrait,
    /// A whole character.
    Fullbody,
    /// A card face.
    Card,
    /// A small round icon.
    Icon,
    /// A scene behind the play area.
    Background,
    /// An object in a scene or on a board.
    Prop,
}

impl Kind {
    /// Every kind, in the order of the format.
    pub const ALL: [Kind; 6] = [
        Kind::Portrait,
        Kind::Fullbody,
        Kind::Card,
        Kind::Icon,
        Kind::Background,
        Kind::Prop,
    ];

    /// The name of the kind, which is also the name of its folder in `art/`.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Portrait => "portrait",
            Kind::Fullbody => "fullbody",
            Kind::Card => "card",
            Kind::Icon => "icon",
            Kind::Background => "background",
            Kind::Prop => "prop",
        }
    }

    /// The kind with this name, or None when the name is not a kind.
    pub fn from_name(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.as_str() == name)
    }
}

/// One art id of the manifest, with its defaults filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The id, unique in the manifest, such as `character.lyra.portrait`.
    pub id: String,
    /// The kind: the folder of the files.
    pub kind: Kind,
    /// The box the art is drawn in, in logical pixels: `[width, height]`.
    pub size: [u32; 2],
    /// The pivot inside the box, as fractions of the size. Defaults to `[0, 0]`.
    pub anchor: [f64; 2],
    /// The variant names of the id, unique within the entry. Defaults to none.
    pub variants: Vec<String>,
    /// Notes for art generation. The build does not read them.
    pub meta: Map<String, Value>,
}

impl Entry {
    /// The names of the entry's pictures, in the order of the manifest: the base picture first, then
    /// one per variant. Each name is `(variant, stem)`: the variant is None for the base picture,
    /// and the stem is the file name without folder or extension, such as `card.wood.red`.
    pub fn stems(&self) -> Vec<(Option<String>, String)> {
        let mut stems = vec![(None, self.id.clone())];
        for variant in &self.variants {
            stems.push((Some(variant.clone()), format!("{}.{variant}", self.id)));
        }
        stems
    }
}

/// A checked art manifest. The entries keep the order of the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// The entries, in the order of the file.
    pub entries: Vec<Entry>,
}

impl Manifest {
    /// The entry with this id, or None when the manifest does not declare it.
    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

/// Reads a manifest from the text of `art/manifest.json`.
///
/// # Errors
/// Returns a [`Refusal`] when the text is not JSON, when the format or the version is wrong, or when
/// any entry breaks a rule. The refusal lists every problem; its code is the code of the first one.
pub fn parse(text: &str) -> Result<Manifest, Refusal> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        Refusal::single(
            ErrorCode::Json,
            format!("the text is not valid JSON: {error}"),
            None,
        )
    })?;
    let Some(document) = value.as_object() else {
        return Err(Refusal::single(
            ErrorCode::Structure,
            "the manifest is not a JSON object",
            None,
        ));
    };
    if document.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(Refusal::single(
            ErrorCode::Format,
            format!("the format tag is not \"{FORMAT}\""),
            Some("format"),
        ));
    }
    if document.get("version").and_then(Value::as_f64) != Some(f64::from(VERSION)) {
        return Err(Refusal::single(
            ErrorCode::Version,
            format!("the version is not {VERSION}"),
            Some("version"),
        ));
    }
    let Some(raw_entries) = document.get("entries").and_then(Value::as_array) else {
        return Err(Refusal::single(
            ErrorCode::Structure,
            "entries must be an array",
            Some("entries"),
        ));
    };

    let mut issues = Vec::new();
    let parsed: Vec<Option<Entry>> = raw_entries
        .iter()
        .enumerate()
        .map(|(index, raw)| parse_entry(raw, &format!("entries[{index}]"), &mut issues))
        .collect();
    issues.extend(duplicate_issues(&parsed));
    if let Some(refusal) = Refusal::from_issues(issues) {
        return Err(refusal);
    }
    Ok(Manifest {
        entries: parsed.into_iter().flatten().collect(),
    })
}

/// True when `text` is a valid id: segments of `a`-`z`, `0`-`9`, `_` and `-`, joined by single dots,
/// at most [`ID_LIMIT`] characters.
pub fn valid_id(text: &str) -> bool {
    text.chars().count() <= ID_LIMIT
        && text
            .split('.')
            .all(|segment| !segment.is_empty() && segment.bytes().all(is_name_byte))
}

/// True when `text` is a valid variant name: `a`-`z`, `0`-`9`, `_` and `-`, at most
/// [`VARIANT_LIMIT`] characters.
pub fn valid_variant(text: &str) -> bool {
    !text.is_empty() && text.chars().count() <= VARIANT_LIMIT && text.bytes().all(is_name_byte)
}

/// The bytes that ids and variant names may use.
fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
}

/// Reads a whole number from 1 to [`MAX_SIDE`]. A number such as `128.0` counts as whole, as it
/// does in JavaScript.
fn whole_side(value: &Value) -> Option<u32> {
    let number = value.as_f64()?;
    let in_range = (1.0..=f64::from(MAX_SIDE)).contains(&number);
    (in_range && number.fract() == 0.0).then_some(number as u32)
}

/// Reads one entry. Returns None when any field is wrong; the issues say which.
fn parse_entry(raw: &Value, path: &str, issues: &mut Vec<Issue>) -> Option<Entry> {
    let Some(object) = raw.as_object() else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be an object"),
        ));
        return None;
    };
    let before = issues.len();
    let id = read_id(object.get("id"), &format!("{path}.id"), issues);
    let kind = read_kind(object.get("kind"), &format!("{path}.kind"), issues);
    let size = read_size(object.get("size"), &format!("{path}.size"), issues);
    let anchor = read_anchor(object.get("anchor"), &format!("{path}.anchor"), issues);
    let variants = read_variants(object.get("variants"), &format!("{path}.variants"), issues);
    let meta = read_meta(object.get("meta"), &format!("{path}.meta"), issues);
    if issues.len() > before {
        return None;
    }
    Some(Entry {
        id: id?,
        kind: kind?,
        size: size?,
        anchor: anchor?,
        variants: variants?,
        meta: meta?,
    })
}

/// Reads an id.
fn read_id(value: Option<&Value>, path: &str, issues: &mut Vec<Issue>) -> Option<String> {
    let Some(text) = value.and_then(Value::as_str) else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be a string"),
        ));
        return None;
    };
    if !valid_id(text) {
        issues.push(issue(
            ErrorCode::Value,
            path,
            format!("{path} \"{text}\" is not a valid art id: lowercase segments joined by dots, at most {ID_LIMIT} characters"),
        ));
        return None;
    }
    Some(text.to_string())
}

/// Reads a kind.
fn read_kind(value: Option<&Value>, path: &str, issues: &mut Vec<Issue>) -> Option<Kind> {
    let Some(text) = value.and_then(Value::as_str) else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be a string"),
        ));
        return None;
    };
    let kind = Kind::from_name(text);
    if kind.is_none() {
        let names: Vec<&str> = Kind::ALL.iter().map(|kind| kind.as_str()).collect();
        issues.push(issue(
            ErrorCode::Value,
            path,
            format!(
                "{path} \"{text}\" is not a kind; use one of {}",
                names.join(", ")
            ),
        ));
    }
    kind
}

/// Reads a size: two whole numbers from 1 to [`MAX_SIDE`].
fn read_size(value: Option<&Value>, path: &str, issues: &mut Vec<Issue>) -> Option<[u32; 2]> {
    let Some(items) = value.and_then(Value::as_array) else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be an array of two numbers"),
        ));
        return None;
    };
    let sides = match items.as_slice() {
        [width, height] => whole_side(width)
            .zip(whole_side(height))
            .map(|(w, h)| [w, h]),
        _ => None,
    };
    if sides.is_none() {
        issues.push(issue(
            ErrorCode::Value,
            path,
            format!("{path} must be two whole numbers from 1 to {MAX_SIDE}"),
        ));
    }
    sides
}

/// Reads an anchor: two numbers from 0 to 1. A missing anchor is the top-left corner.
fn read_anchor(value: Option<&Value>, path: &str, issues: &mut Vec<Issue>) -> Option<[f64; 2]> {
    let Some(value) = value else {
        return Some([0.0, 0.0]);
    };
    let Some(items) = value.as_array() else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be an array of two numbers"),
        ));
        return None;
    };
    let anchor = match items.as_slice() {
        [x, y] => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) => {
                Some([x, y])
            }
            _ => None,
        },
        _ => None,
    };
    if anchor.is_none() {
        issues.push(issue(
            ErrorCode::Value,
            path,
            format!("{path} must be two numbers from 0 to 1"),
        ));
    }
    anchor
}

/// Reads the variant names of an entry. A missing list is empty.
fn read_variants(
    value: Option<&Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) -> Option<Vec<String>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    let Some(items) = value.as_array() else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be an array of names"),
        ));
        return None;
    };
    let mut names: Vec<String> = Vec::new();
    let mut valid = true;
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        match item.as_str() {
            None => {
                issues.push(issue(
                    ErrorCode::Structure,
                    &item_path,
                    format!("{item_path} must be a string"),
                ));
                valid = false;
            }
            Some(name) if !valid_variant(name) => {
                issues.push(issue(
                    ErrorCode::Value,
                    &item_path,
                    format!("{item_path} \"{name}\" is not a valid variant name: lowercase letters, digits, \"_\" or \"-\""),
                ));
                valid = false;
            }
            Some(name) if names.iter().any(|known| known == name) => {
                issues.push(issue(
                    ErrorCode::Duplicate,
                    &item_path,
                    format!("{item_path} repeats the variant \"{name}\""),
                ));
                valid = false;
            }
            Some(name) => names.push(name.to_string()),
        }
    }
    valid.then_some(names)
}

/// Reads the notes of an entry. A missing block is an empty object.
fn read_meta(
    value: Option<&Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) -> Option<Map<String, Value>> {
    match value {
        None => Some(Map::new()),
        Some(Value::Object(object)) => Some(object.clone()),
        Some(_) => {
            issues.push(issue(
                ErrorCode::Structure,
                path,
                format!("{path} must be an object"),
            ));
            None
        }
    }
}

/// Finds the ids and file names that are used twice. Two entries share a file name when they have
/// the same kind and one's id or variant file name equals the other's: an entry `card.wood` with
/// variant `red`, and an entry `card.wood.red`. Refused entries are skipped, so the paths match the file.
fn duplicate_issues(entries: &[Option<Entry>]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut ids: HashSet<&str> = HashSet::new();
    let mut owners: HashMap<String, String> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(entry) = entry else { continue };
        let path = format!("entries[{index}]");
        if !ids.insert(entry.id.as_str()) {
            issues.push(issue(
                ErrorCode::Duplicate,
                &format!("{path}.id"),
                format!("id \"{}\" is already used by an earlier entry", entry.id),
            ));
            continue;
        }
        for (_, stem) in entry.stems() {
            let key = format!("{}/{stem}", entry.kind.as_str());
            match owners.get(&key) {
                None => {
                    owners.insert(key, path.clone());
                }
                Some(owner) => issues.push(issue(
                    ErrorCode::Duplicate,
                    &path,
                    format!("file name \"{key}\" would be used twice, by {owner} and {path}"),
                )),
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_defaults_for_an_entry_with_only_its_required_fields() {
        let manifest = parse(
            r#"{"format":"velvet-art","version":1,"entries":[{"id":"card.wood","kind":"card","size":[180,260]}]}"#,
        )
        .unwrap();
        let entry = &manifest.entries[0];
        assert_eq!(entry.anchor, [0.0, 0.0]);
        assert!(entry.variants.is_empty());
        assert!(entry.meta.is_empty());
        assert_eq!(entry.size, [180, 260]);
    }

    #[test]
    fn a_whole_number_written_with_a_fraction_is_still_whole() {
        let manifest = parse(
            r#"{"format":"velvet-art","version":1,"entries":[{"id":"card.wood","kind":"card","size":[180.0,260]}]}"#,
        )
        .unwrap();
        assert_eq!(manifest.entries[0].size, [180, 260]);
    }

    #[test]
    fn stems_list_the_base_then_each_variant() {
        let manifest = parse(
            r#"{"format":"velvet-art","version":1,"entries":[{"id":"bg.forest","kind":"background","size":[1920,1080],"variants":["day","night"]}]}"#,
        )
        .unwrap();
        let stems = manifest.entries[0].stems();
        assert_eq!(
            stems,
            vec![
                (None, "bg.forest".to_string()),
                (Some("day".to_string()), "bg.forest.day".to_string()),
                (Some("night".to_string()), "bg.forest.night".to_string()),
            ]
        );
    }

    #[test]
    fn the_first_problem_gives_the_code_and_all_are_listed() {
        let refusal = parse(
            r#"{"format":"velvet-art","version":1,"entries":[{"id":"Bad","kind":"card","size":[0,1]},{"kind":"card","size":[1,1]}]}"#,
        )
        .unwrap_err();
        assert_eq!(refusal.code, ErrorCode::Value);
        assert_eq!(refusal.issues.len(), 3);
        assert_eq!(refusal.issues[0].path.as_deref(), Some("entries[0].id"));
        assert_eq!(refusal.issues[2].path.as_deref(), Some("entries[1].id"));
    }

    #[test]
    fn ids_and_variants_follow_their_grammar() {
        assert!(valid_id("character.lyra.portrait"));
        assert!(valid_id("a-b_c.1"));
        assert!(!valid_id(""));
        assert!(!valid_id("a..b"));
        assert!(!valid_id("a.b."));
        assert!(!valid_id("A"));
        assert!(!valid_id(&"a".repeat(129)));
        assert!(valid_variant("happy"));
        assert!(!valid_variant("happy.png"));
        assert!(!valid_variant(&"a".repeat(33)));
    }
}
