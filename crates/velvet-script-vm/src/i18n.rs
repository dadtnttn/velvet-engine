//! Host hooks for translating and measuring text from scripts.
//!
//! Scripts call `tr(text)` right before showing text; the host installs a
//! [`Catalog`] for the active language with [`set_catalog`]. A catalog maps
//! source strings to translations. Source keys may contain numbered holes
//! (`"Le diste {0} a {1}."`) that match any text built at runtime; the
//! translation reorders them freely (`"You gave {1} {0}."`). Hole values are
//! translated too (a job title inside a sentence), and a translation can pick
//! a word by the raw hole value with `{0|o=Bienvenido|a=Bienvenida|*=...}`
//! (handy for gender endings produced by the source language).
//!
//! The host can also install a measurer so `str_measure`/`str_wrap` use real
//! font metrics (fallback fonts, shaping) instead of the per-character table.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Width of `text` in pixels for a font id and pixel size.
pub type Measurer = Rc<dyn Fn(&str, &str, f64) -> Option<f64>>;

thread_local! {
    static ACTIVE: RefCell<Option<Translator>> = const { RefCell::new(None) };
    static MEASURER: RefCell<Option<Measurer>> = const { RefCell::new(None) };
}

/// Install (or remove with `None`) the catalog used by `tr`.
pub fn set_catalog(catalog: Option<Catalog>) {
    ACTIVE.with(|a| *a.borrow_mut() = catalog.map(Translator::new));
}

/// Translate `text` with the active catalog (unchanged without one).
pub fn translate(text: &str) -> String {
    ACTIVE.with(|a| match a.borrow_mut().as_mut() {
        Some(t) => t.tr(text),
        None => text.to_owned(),
    })
}

/// Code of the active language (`""` when no catalog is installed).
pub fn active_lang() -> String {
    ACTIVE.with(|a| a.borrow().as_ref().map(|t| t.cat.lang.clone()).unwrap_or_default())
}

/// Install (or remove) the host text measurer.
pub fn set_measurer(m: Option<Measurer>) {
    MEASURER.with(|slot| *slot.borrow_mut() = m);
}

pub(crate) fn host_measure(font: &str, text: &str, size: f64) -> Option<f64> {
    MEASURER.with(|slot| slot.borrow().as_ref().and_then(|m| m(font, text, size)))
}

// ---------------------------------------------------------------- catalog

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Lit(String),
    Hole(usize),
}

#[derive(Debug, Clone)]
struct Template {
    parts: Vec<Part>,
    /// Same literals, uppercased (for text shown with `str_upper`).
    upper: Vec<Part>,
    target: String,
    /// Total literal characters: more specific templates win.
    weight: usize,
}

/// A language catalog: exact strings plus templates with holes.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    /// Language code (`en`, `pt-BR`, ...).
    pub lang: String,
    exact: HashMap<String, String>,
    upper: HashMap<String, String>,
    lower: HashMap<String, String>,
    templates: Vec<Template>,
    /// Templates indexed by the first characters of their leading literal.
    by_prefix: HashMap<String, Vec<usize>>,
    /// Templates that start with a hole (checked by their longest literal).
    floating: Vec<(usize, String)>,
}

const PREFIX_CHARS: usize = 3;

fn prefix_key(s: &str) -> String {
    s.chars().take(PREFIX_CHARS).collect()
}

fn parse_template(src: &str) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    let mut chars = src.chars().peekable();
    let mut holes = 0;
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut num = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    num.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if !num.is_empty() && chars.peek() == Some(&'}') {
                chars.next();
                if !lit.is_empty() {
                    parts.push(Part::Lit(std::mem::take(&mut lit)));
                }
                parts.push(Part::Hole(num.parse().ok()?));
                holes += 1;
                continue;
            }
            lit.push('{');
            lit.push_str(&num);
            continue;
        }
        lit.push(c);
    }
    if !lit.is_empty() {
        parts.push(Part::Lit(lit));
    }
    (holes > 0).then_some(parts)
}

impl Catalog {
    /// Build from `(source, translation)` pairs.
    pub fn new(lang: impl Into<String>, entries: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut cat = Catalog {
            lang: lang.into(),
            ..Default::default()
        };
        for (src, dst) in entries {
            if dst.is_empty() {
                continue;
            }
            match parse_template(&src) {
                Some(parts) => {
                    let upper = parts
                        .iter()
                        .map(|p| match p {
                            Part::Lit(s) => Part::Lit(s.to_uppercase()),
                            Part::Hole(i) => Part::Hole(*i),
                        })
                        .collect();
                    let weight = parts
                        .iter()
                        .map(|p| match p {
                            Part::Lit(s) => s.chars().count(),
                            Part::Hole(_) => 0,
                        })
                        .sum();
                    let idx = cat.templates.len();
                    match parts.first() {
                        Some(Part::Lit(first)) => {
                            cat.by_prefix.entry(prefix_key(first)).or_default().push(idx);
                            cat.by_prefix
                                .entry(prefix_key(&first.to_uppercase()))
                                .or_default()
                                .push(idx);
                        }
                        _ => {
                            let longest = parts
                                .iter()
                                .filter_map(|p| match p {
                                    Part::Lit(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .max_by_key(|s| s.chars().count())
                                .unwrap_or_default();
                            cat.floating.push((idx, longest));
                        }
                    }
                    cat.templates.push(Template {
                        parts,
                        upper,
                        target: dst,
                        weight,
                    });
                }
                None => {
                    cat.upper.entry(src.to_uppercase()).or_insert_with(|| dst.to_uppercase());
                    cat.lower.entry(src.to_lowercase()).or_insert_with(|| dst.to_lowercase());
                    cat.exact.insert(src, dst);
                }
            }
        }
        for list in cat.by_prefix.values_mut() {
            list.sort_unstable();
            list.dedup();
        }
        cat
    }

    /// Number of exact entries and templates.
    pub fn len(&self) -> (usize, usize) {
        (self.exact.len(), self.templates.len())
    }

    /// Whether the catalog has no entries.
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.templates.is_empty()
    }
}

// ---------------------------------------------------------------- matching

struct Translator {
    cat: Catalog,
    cache: HashMap<String, String>,
    outputs: HashSet<String>,
}

const CACHE_LIMIT: usize = 60_000;

fn has_letters(s: &str) -> bool {
    s.chars().any(char::is_alphabetic)
}

fn is_upper_text(s: &str) -> bool {
    s.chars().any(char::is_alphabetic) && !s.chars().any(char::is_lowercase)
}

/// Match `s` against `parts`; returns the hole values on success.
fn match_parts(parts: &[Part], s: &str) -> Option<Vec<(usize, String)>> {
    let mut holes = Vec::new();
    if match_from(parts, s, &mut holes, 0) {
        Some(holes)
    } else {
        None
    }
}

fn match_from(parts: &[Part], s: &str, holes: &mut Vec<(usize, String)>, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match parts.first() {
        None => s.is_empty(),
        Some(Part::Lit(l)) => match s.strip_prefix(l.as_str()) {
            Some(rest) => match_from(&parts[1..], rest, holes, depth + 1),
            None => false,
        },
        Some(Part::Hole(i)) => {
            match parts.get(1) {
                // Last hole takes the rest.
                None => {
                    holes.push((*i, s.to_owned()));
                    true
                }
                Some(Part::Lit(next)) => {
                    // Try each occurrence of the next literal (shortest hole first).
                    let mut start = 0;
                    while let Some(pos) = s[start..].find(next.as_str()) {
                        let at = start + pos;
                        let mark = holes.len();
                        holes.push((*i, s[..at].to_owned()));
                        if match_from(&parts[1..], &s[at..], holes, depth + 1) {
                            return true;
                        }
                        holes.truncate(mark);
                        // Advance by one character.
                        let step = s[at..].chars().next().map_or(1, char::len_utf8);
                        start = at + step;
                        if start > s.len() {
                            break;
                        }
                    }
                    false
                }
                // Two holes in a row: split at every char boundary.
                Some(Part::Hole(_)) => {
                    for (at, _) in s.char_indices().chain(std::iter::once((s.len(), ' '))) {
                        let mark = holes.len();
                        holes.push((*i, s[..at].to_owned()));
                        if match_from(&parts[1..], &s[at..], holes, depth + 1) {
                            return true;
                        }
                        holes.truncate(mark);
                    }
                    false
                }
            }
        }
    }
}

fn restore_case(src: &str, translated: String) -> String {
    if is_upper_text(src) {
        return translated.to_uppercase();
    }
    translated
}

impl Translator {
    fn new(cat: Catalog) -> Self {
        Self {
            cat,
            cache: HashMap::new(),
            outputs: HashSet::new(),
        }
    }

    fn tr(&mut self, s: &str) -> String {
        if s.is_empty() || !has_letters(s) {
            return s.to_owned();
        }
        if let Some(hit) = self.cache.get(s) {
            return hit.clone();
        }
        if self.outputs.contains(s) {
            return s.to_owned();
        }
        let result = self.lookup(s, 0).unwrap_or_else(|| s.to_owned());
        if self.cache.len() > CACHE_LIMIT {
            self.cache.clear();
            self.outputs.clear();
        }
        if result != s {
            self.outputs.insert(result.clone());
        }
        self.cache.insert(s.to_owned(), result.clone());
        result
    }

    fn lookup(&self, s: &str, depth: usize) -> Option<String> {
        if let Some(t) = self.cat.exact.get(s) {
            return Some(t.clone());
        }
        // Keep surrounding spaces and a trailing colon/period shown by the UI.
        let trimmed = s.trim();
        if trimmed.len() != s.len() && !trimmed.is_empty() {
            if let Some(t) = self.lookup(trimmed, depth) {
                let lead = &s[..s.find(trimmed).unwrap_or(0)];
                let tail = &s[lead.len() + trimmed.len()..];
                return Some(format!("{lead}{t}{tail}"));
            }
        }
        if is_upper_text(s) {
            if let Some(t) = self.cat.upper.get(s) {
                return Some(t.clone());
            }
        } else if let Some(t) = self.cat.lower.get(s) {
            if s.chars().next().is_some_and(char::is_lowercase) {
                return Some(t.clone());
            }
        }
        // A capitalized form of a lowercase entry (and vice versa).
        if let Some(t) = self.cat.lower.get(&s.to_lowercase()) {
            let mut out = t.clone();
            if s.chars().next().is_some_and(char::is_uppercase) {
                let mut cs = out.chars();
                if let Some(f) = cs.next() {
                    out = f.to_uppercase().collect::<String>() + cs.as_str();
                }
            }
            return Some(out);
        }
        if depth > 3 {
            return None;
        }
        self.match_template(s, depth)
    }

    fn match_template(&self, s: &str, depth: usize) -> Option<String> {
        let upper = is_upper_text(s);
        let mut best: Option<(usize, Vec<(usize, String)>)> = None;
        let consider = |idx: usize, best: &mut Option<(usize, Vec<(usize, String)>)>| {
            let t = &self.cat.templates[idx];
            if best.as_ref().is_some_and(|(b, _)| self.cat.templates[*b].weight >= t.weight) {
                return;
            }
            let parts = if upper { &t.upper } else { &t.parts };
            if let Some(h) = match_parts(parts, s) {
                *best = Some((idx, h));
            }
        };
        let key = prefix_key(s);
        if let Some(list) = self.cat.by_prefix.get(&key) {
            for &idx in list {
                consider(idx, &mut best);
            }
        }
        // Shorter leading literals (1-2 chars) are indexed by their own length.
        for n in 1..PREFIX_CHARS {
            let k: String = s.chars().take(n).collect();
            if k.chars().count() == n {
                if let Some(list) = self.cat.by_prefix.get(&k) {
                    for &idx in list {
                        consider(idx, &mut best);
                    }
                }
            }
        }
        for (idx, lit) in &self.cat.floating {
            let hit = if upper { s.contains(&lit.to_uppercase()) } else { s.contains(lit.as_str()) };
            if hit {
                consider(*idx, &mut best);
            }
        }
        let (idx, holes) = best?;
        let t = &self.cat.templates[idx];
        let out = self.render(&t.target, &holes, depth);
        Some(restore_case(s, out))
    }

    fn render(&self, target: &str, holes: &[(usize, String)], depth: usize) -> String {
        let value = |i: usize| holes.iter().find(|(h, _)| *h == i).map(|(_, v)| v.as_str());
        let mut out = String::new();
        let mut rest = target;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                out.push_str(&rest[open..]);
                return out;
            };
            let inner = &after[..close];
            let (num, select) = match inner.find('|') {
                Some(bar) => (&inner[..bar], Some(&inner[bar + 1..])),
                None => (inner, None),
            };
            match num.parse::<usize>() {
                Ok(i) => {
                    let raw = value(i).unwrap_or("");
                    match select {
                        Some(options) => out.push_str(&pick_option(options, raw)),
                        None => {
                            let shown = if raw.trim().is_empty() || !has_letters(raw) {
                                raw.to_owned()
                            } else {
                                self.lookup(raw, depth + 1).unwrap_or_else(|| raw.to_owned())
                            };
                            out.push_str(&shown);
                        }
                    }
                }
                Err(_) => {
                    out.push('{');
                    out.push_str(inner);
                    out.push('}');
                }
            }
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

/// `o=Bienvenido|a=Bienvenida|*=...` → the option for `raw` (or `*`).
fn pick_option(options: &str, raw: &str) -> String {
    let mut fallback = String::new();
    for opt in options.split('|') {
        if let Some((k, v)) = opt.split_once('=') {
            if k == raw {
                return v.to_owned();
            }
            if k == "*" {
                fallback = v.to_owned();
            }
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(entries: &[(&str, &str)]) -> Catalog {
        Catalog::new("en", entries.iter().map(|(a, b)| (a.to_string(), b.to_string())))
    }

    #[test]
    fn exact_case_and_templates() {
        let c = cat(&[
            ("Padre", "Father"),
            ("Le diste {0} a {1}.", "You gave {1} {0}."),
            ("Bienvenid{0} al ejército.", "Welcome to the army."),
            ("Estás cansad{0}.", "{0|o=He|a=She|*=They} is tired."),
            ("Tu {0} te llama.", "Your {0} is calling."),
            ("{0} te dejó.", "{0} left you."),
            ("CITA CON {0}", "DATE WITH {0}"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(t.tr("Padre"), "Father");
        assert_eq!(t.tr("PADRE"), "FATHER");
        assert_eq!(t.tr("padre"), "father");
        assert_eq!(t.tr("Le diste $500 a Hugo."), "You gave Hugo $500.");
        assert_eq!(t.tr("Bienvenida al ejército."), "Welcome to the army.");
        assert_eq!(t.tr("Estás cansada."), "She is tired.");
        assert_eq!(t.tr("Tu padre te llama."), "Your father is calling.");
        assert_eq!(t.tr("Ana te dejó."), "Ana left you.");
        assert_eq!(t.tr("CITA CON JULIETA"), "DATE WITH JULIETA");
        assert_eq!(t.tr("Nada que ver"), "Nada que ver");
        assert_eq!(t.tr("$1,200"), "$1,200");
        // Translated output is not translated again.
        assert_eq!(t.tr("Father"), "Father");
    }
}
