//! Host hooks for translating and measuring text from scripts.
//!
//! Scripts call `tr(text)` right before showing text; the host installs a
//! [`Catalog`] for the active language with [`set_catalog`]. A catalog maps
//! source strings to translations. Source keys may contain numbered holes
//! (`"Le diste {0} a {1}."`) that match any text built at runtime; the
//! translation reorders them freely (`"You gave {1} {0}."`). Hole values are
//! translated too (a job title inside a sentence), and a translation can pick
//! a word by the raw hole value with `{0|o=Bienvenido|a=Bienvenida|*=...}`
//! (handy for gender endings produced by the source language). `{0!}` keeps
//! the raw value (names that happen to be words, like "Rosa"); a value that is
//! not a name but translates ("Tu mamá" said instead of a name) is translated.
//!
//! Translations can also pick by a global variable the game sets (the
//! player's gender, say): `{@g|m=Ты пошёл|f=Ты пошла}` — see [`set_var`].
//!
//! With `VELVET_I18N_MISSES=<file>` set, every distinct text that found no
//! translation is appended to that file (one per line) to find gaps.
//!
//! The host can also install a measurer so `str_measure`/`str_wrap` use real
//! font metrics (fallback fonts, shaping) instead of the per-character table.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Width of `text` in pixels for a font id and pixel size.
pub type Measurer = Rc<dyn Fn(&str, &str, f64) -> Option<f64>>;

thread_local! {
    static VARS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    static ACTIVE: RefCell<Option<Translator>> = const { RefCell::new(None) };
    static MEASURER: RefCell<Option<Measurer>> = const { RefCell::new(None) };
}

/// Install (or remove with `None`) the catalog used by `tr`.
pub fn set_catalog(catalog: Option<Catalog>) {
    ACTIVE.with(|a| *a.borrow_mut() = catalog.map(Translator::new));
}

/// Set a variable translations can select on (`{@name|value=...|*=...}`).
pub fn set_var(name: &str, value: &str) {
    let changed = VARS.with(|v| {
        let mut v = v.borrow_mut();
        if v.get(name).map(String::as_str) == Some(value) {
            false
        } else {
            v.insert(name.to_lowercase(), value.to_owned());
            true
        }
    });
    if changed {
        ACTIVE.with(|a| {
            if let Some(t) = a.borrow_mut().as_mut() {
                t.cache.clear();
                t.outputs.clear();
            }
        });
    }
}

fn var(name: &str) -> String {
    VARS.with(|v| {
        v.borrow()
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or_default()
    })
}

/// Resolve `{@var|k=v|...}` selects in a finished translation.
fn resolve_vars(text: &str) -> String {
    if !text.contains("{@") {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("{@") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let inner = &after[..close];
        match inner.split_once('|') {
            Some((name, options)) => out.push_str(&pick_option(options, &var(name))),
            None => out.push_str(&var(inner)),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Translate `text` with the active catalog (unchanged without one).
pub fn translate(text: &str) -> String {
    ACTIVE.with(|a| match a.borrow_mut().as_mut() {
        Some(t) => t.tr(text),
        None => text.to_owned(),
    })
}

static SYSTEM_LANG: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// The language of the player's system, as the host reports it ("es-MX").
pub fn set_system_lang(code: &str) {
    if let Ok(mut lang) = SYSTEM_LANG.write() {
        *lang = code.to_owned();
    }
}

/// The language of the player's system (`""` when the host did not say).
pub fn system_lang() -> String {
    SYSTEM_LANG.read().map(|l| l.clone()).unwrap_or_default()
}

/// Code of the active language (`""` when no catalog is installed).
pub fn active_lang() -> String {
    ACTIVE.with(|a| {
        a.borrow()
            .as_ref()
            .map(|t| t.cat.lang.clone())
            .unwrap_or_default()
    })
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
    pub fn new(
        lang: impl Into<String>,
        entries: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let mut cat = Catalog {
            lang: lang.into(),
            ..Default::default()
        };
        let mut entries: Vec<(String, String)> = entries.into_iter().collect();
        // " {0} se fue." or " Tienes el título." are glued after another
        // sentence: pieces of a line are looked up trimmed, so the trimmed
        // entry must exist too (unless the catalog already has its own).
        let known: HashSet<&str> = entries.iter().map(|(src, _)| src.as_str()).collect();
        let trimmed: Vec<(String, String)> = entries
            .iter()
            .filter(|(src, dst)| {
                let t = src.trim();
                t.len() != src.len() && !t.is_empty() && !dst.is_empty() && !known.contains(t)
            })
            .map(|(src, dst)| (src.trim().to_owned(), dst.trim().to_owned()))
            .collect();
        entries.extend(trimmed);
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
                            cat.by_prefix
                                .entry(prefix_key(first))
                                .or_default()
                                .push(idx);
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
                    // The first entry wins, except that a real translation
                    // replaces an identity one (a code id that is also a word).
                    for (map, k, v) in [
                        (&mut cat.upper, src.to_uppercase(), dst.to_uppercase()),
                        (
                            &mut cat.lower,
                            src.to_lowercase(),
                            lower_form(&cat.lang, &dst),
                        ),
                    ] {
                        let keep = map.get(&k).is_some_and(|old| *old != k || v == k);
                        if !keep {
                            map.insert(k, v);
                        }
                    }
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
    misses: Option<(std::path::PathBuf, HashSet<String>)>,
    /// Pieces already looked up while translating one line: templates,
    /// pieces and sentences try the same substrings over and over.
    memo: RefCell<HashMap<(String, usize), Option<String>>>,
    /// Lookups left for the current line, so a long line nothing matches
    /// cannot take forever.
    budget: std::cell::Cell<usize>,
}

const CACHE_LIMIT: usize = 60_000;
/// Nested lookups allowed per line (a normal line needs a handful).
const LOOKUP_BUDGET: usize = 1500;
/// Template readings tried per piece, heaviest template first.
const MAX_CANDIDATES: usize = 24;

/// One to three words, each starting with a capital letter: a proper name
/// ("Hanna", "Hanna Krawczyk") that no catalog is expected to translate.
/// A common word of the source makes it text ("Hace", "TU MAMÁ"), unless
/// `unknown` says the rest is foreign to the catalog: "Si Thu" and "Shwe
/// Sin" are names.
fn looks_like_name(v: &str, unknown: impl Fn(&str) -> bool) -> bool {
    let words: Vec<&str> = v.split_whitespace().collect();
    if words.is_empty()
        || words.len() > 3
        || !words
            .iter()
            .all(|w| w.chars().next().is_some_and(char::is_uppercase))
    {
        return false;
    }
    let cores: Vec<String> = words
        .iter()
        .map(|w| w.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase())
        .collect();
    let common = |w: &str| COMMON_WORDS.iter().any(|c| c.to_lowercase() == w);
    if !cores.iter().any(|w| common(w)) {
        return true;
    }
    let rest: Vec<&String> = cores.iter().filter(|w| !common(w)).collect();
    !rest.is_empty() && unknown(&v.to_lowercase()) && rest.iter().all(|w| unknown(w))
}

/// Capitalized words of the source language that are never names (a line
/// starting "Hace 3 años" or "Tu madre" is text, not a person).
const COMMON_WORDS: &[&str] = &[
    "Hace",
    "Tu",
    "Tus",
    "Te",
    "Mi",
    "Mis",
    "Su",
    "Sus",
    "El",
    "La",
    "Los",
    "Las",
    "Lo",
    "Un",
    "Una",
    "Unos",
    "Unas",
    "De",
    "Del",
    "Al",
    "Y",
    "O",
    "Con",
    "Sin",
    "Por",
    "Para",
    "Que",
    "Se",
    "No",
    "Sí",
    "Si",
    "Ya",
    "Muy",
    "Más",
    "Menos",
    "Hoy",
    "Ayer",
    "Ahora",
    "Nunca",
    "Siempre",
    "Cada",
    "Todo",
    "Toda",
    "Todos",
    "Todas",
    "Este",
    "Esta",
    "Ese",
    "Esa",
    "Es",
    "Eres",
    "Está",
    "Estás",
    "Son",
    "Hay",
    "Fue",
    "Fuiste",
    "Uno",
    "Dos",
    "Tres",
    "Cuatro",
    "Cinco",
    "Seis",
    "Siete",
    "Ocho",
    "Nueve",
    "Diez",
    "Once",
    "Doce",
    "Quince",
    "Veinte",
    "Treinta",
    "Cuarenta",
    "Cincuenta",
    "Cien",
    "Mil",
    "Primer",
    "Primera",
    "Primero",
    "Segunda",
    "Segundo",
];

fn has_letters(s: &str) -> bool {
    s.chars().any(char::is_alphabetic)
}

fn is_upper_text(s: &str) -> bool {
    s.chars().any(char::is_alphabetic) && !s.chars().any(char::is_lowercase)
}

/// Match `s` against `parts`; returns the hole values on success.
/// Ways to read `s` with a template, shortest holes first: "Requisitos:
/// {1}.{2}" over "Intel. 25 · Atractivo 50." can cut at either full stop, and
/// only the caller knows which split leaves holes that translate.
fn match_parts(parts: &[Part], s: &str) -> Vec<Vec<(usize, String)>> {
    let mut out = Vec::new();
    match_from(parts, s, &mut Vec::new(), 0, &mut out);
    out
}

/// Splits kept per template.
const MAX_SPLITS: usize = 6;

type Holes = Vec<(usize, String)>;

fn match_from(parts: &[Part], s: &str, holes: &mut Holes, depth: usize, out: &mut Vec<Holes>) {
    if depth > 64 || out.len() >= MAX_SPLITS {
        return;
    }
    match parts.first() {
        None => {
            if s.is_empty() {
                out.push(holes.clone());
            }
        }
        Some(Part::Lit(l)) => {
            if let Some(rest) = s.strip_prefix(l.as_str()) {
                match_from(&parts[1..], rest, holes, depth + 1, out);
            }
        }
        Some(Part::Hole(i)) => match parts.get(1) {
            // Last hole takes the rest.
            None => {
                holes.push((*i, s.to_owned()));
                out.push(holes.clone());
                holes.pop();
            }
            Some(Part::Lit(next)) => {
                // Try each occurrence of the next literal (shortest hole first).
                let mut start = 0;
                while let Some(pos) = s[start..].find(next.as_str()) {
                    let at = start + pos;
                    holes.push((*i, s[..at].to_owned()));
                    match_from(&parts[1..], &s[at..], holes, depth + 1, out);
                    holes.pop();
                    if out.len() >= MAX_SPLITS {
                        return;
                    }
                    // Advance by one character.
                    let step = s[at..].chars().next().map_or(1, char::len_utf8);
                    start = at + step;
                    if start > s.len() {
                        break;
                    }
                }
            }
            // Two holes in a row: split at every char boundary.
            Some(Part::Hole(_)) => {
                for (at, _) in s.char_indices().chain(std::iter::once((s.len(), ' '))) {
                    holes.push((*i, s[..at].to_owned()));
                    match_from(&parts[1..], &s[at..], holes, depth + 1, out);
                    holes.pop();
                    if out.len() >= MAX_SPLITS {
                        return;
                    }
                }
            }
        },
    }
}

/// A translation as `lower` keeps it, for lines in another case: all in
/// lowercase, except in German, whose nouns stay capitalized ("niedriges
/// Einkommen"); only its first letter then follows the line.
fn lower_form(lang: &str, dst: &str) -> String {
    if lang == "de" && !is_upper_text(dst) {
        dst.to_owned()
    } else {
        dst.to_lowercase()
    }
}

fn first_upper(s: &str) -> String {
    let mut cs = s.chars();
    cs.next()
        .map_or_else(String::new, |f| f.to_uppercase().chain(cs).collect())
}

fn first_lower(s: &str) -> String {
    let mut cs = s.chars();
    cs.next()
        .map_or_else(String::new, |f| f.to_lowercase().chain(cs).collect())
}

fn restore_case(src: &str, translated: String) -> String {
    if is_upper_text(src) {
        return translated.to_uppercase();
    }
    translated
}

impl Translator {
    /// A proper name, judged with this catalog: next to a common word
    /// ("Si Thu"), the other words must be unknown to it ("TU MAMÁ" is not).
    fn is_name(&self, v: &str) -> bool {
        looks_like_name(v, |w| !self.cat.lower.contains_key(w))
    }

    fn new(cat: Catalog) -> Self {
        let misses = std::env::var_os("VELVET_I18N_MISSES")
            .filter(|p| !p.is_empty())
            .map(|p| (std::path::PathBuf::from(p), HashSet::new()));
        Self {
            cat,
            cache: HashMap::new(),
            outputs: HashSet::new(),
            misses,
            memo: RefCell::new(HashMap::new()),
            budget: std::cell::Cell::new(0),
        }
    }

    fn note_miss(&mut self, s: &str) {
        let Some((path, seen)) = self.misses.as_mut() else {
            return;
        };
        if !seen.insert(s.to_owned()) {
            return;
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{}", s.replace('\n', "\\n"));
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
        self.memo.borrow_mut().clear();
        self.budget.set(LOOKUP_BUDGET);
        let result = match self.lookup(s, 0) {
            Some(t) => resolve_vars(&t),
            None => {
                self.note_miss(s);
                s.to_owned()
            }
        };
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
        let key = (s.to_owned(), depth);
        if let Some(hit) = self.memo.borrow().get(&key) {
            return hit.clone();
        }
        let left = self.budget.get();
        if left == 0 {
            return None;
        }
        self.budget.set(left - 1);
        let out = self.lookup_fresh(s, depth);
        self.memo.borrow_mut().insert(key, out.clone());
        out
    }

    fn lookup_fresh(&self, s: &str, depth: usize) -> Option<String> {
        if let Some(t) = self.cat.exact.get(s) {
            // "piojos" is kept as it is (an illness id), but shown as a word
            // it has a translation under "Piojos": that one wins.
            if t == s {
                if let Some(l) = self.cat.lower.get(&s.to_lowercase()) {
                    if l.to_lowercase() != s.to_lowercase()
                        && s.chars().next().is_some_and(char::is_lowercase)
                    {
                        return Some(first_lower(l));
                    }
                }
            }
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
        // Another case of an entry. One that stays the same is an id ("deprimido")
        // and only counts when nothing better fits: "Deprimido" is the
        // template "Deprimid{0}".
        let case_hit = if is_upper_text(s) {
            self.cat.upper.get(s).cloned()
        } else {
            self.cat
                .lower
                .get(s)
                .filter(|_| s.chars().next().is_some_and(char::is_lowercase))
                .map(|t| first_lower(t))
        }
        .or_else(|| {
            // A capitalized form of a lowercase entry (and vice versa).
            self.cat
                .lower
                .get(&s.to_lowercase())
                .map(|t| match s.chars().next() {
                    Some(c) if c.is_uppercase() => first_upper(t),
                    Some(c) if c.is_lowercase() => first_lower(t),
                    _ => t.clone(),
                })
        });
        if let Some(t) = case_hit.as_ref().filter(|t| *t != s) {
            return Some(t.clone());
        }
        if depth > 6 {
            return case_hit;
        }
        // Templates whose holes all translate come first; then lines built
        // from pieces; a template that keeps a hole as a name is the last
        // resort ("{0} años{1}" must not keep "Cinco" as if it were a name).
        self.match_template(s, depth, true)
            .or_else(|| self.split_list(s, depth))
            .or_else(|| self.paren_tail(s, depth))
            .or_else(|| self.sentences(s, depth))
            .or_else(|| self.decorated(s, depth))
            .or_else(|| case_hit.clone())
            .or_else(|| self.match_template(s, depth, false))
            .or_else(|| self.lowercase_start(s, depth))
    }

    /// A piece the game decorates: a bullet ("- Economía de Myanmar"), a
    /// full stop after a name ("Guerra de Gaza.") or a price after a label
    /// ("Casa con jardín $350,000"). The bare piece has its own entry.
    fn decorated(&self, s: &str, depth: usize) -> Option<String> {
        let inner = |body: &str| {
            has_letters(body)
                .then(|| self.lookup(body, depth + 1))
                .flatten()
                .filter(|t| t != body)
        };
        for bullet in ["- ", "• "] {
            if let Some(rest) = s.strip_prefix(bullet) {
                return inner(rest).map(|t| format!("{bullet}{t}"));
            }
        }
        if let Some(body) = s.strip_suffix('.').filter(|b| !b.ends_with('.')) {
            return inner(body).map(|t| {
                if t.ends_with(['.', '!', '?', '。', '！', '？', '।']) {
                    t
                } else {
                    format!("{t}{}", self.full_stop())
                }
            });
        }
        // A signed amount before a stat: "-8 contratación", "+40 % sueldo".
        if s.starts_with(['+', '-']) {
            let rest =
                s[1..].trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',');
            if rest.len() < s.len() - 1 {
                let (amount, words) = s.split_at(s.len() - rest.len());
                let (amount, words) = match words.strip_prefix(" %") {
                    Some(w) => (&s[..amount.len() + 2], w),
                    None => (amount, words),
                };
                if let Some(words) = words.strip_prefix(' ') {
                    if let Some(t) = inner(words) {
                        return Some(format!("{amount} {t}"));
                    }
                }
                // "+40 % sueldo": the entry is "% sueldo".
                if let Some(words) = rest.strip_prefix(' ') {
                    let amount = &s[..s.len() - rest.len()];
                    return inner(words).map(|t| format!("{amount} {t}"));
                }
            }
        }
        let (head, price) = s.rsplit_once(' ')?;
        let digits = price.trim_start_matches(['+', '-']).strip_prefix('$')?;
        let is_amount = !digits.is_empty()
            && digits.starts_with(|c: char| c.is_ascii_digit())
            && digits
                .chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == '.');
        if !is_amount {
            return None;
        }
        inner(head).map(|t| format!("{t} {price}"))
    }

    fn full_stop(&self) -> &'static str {
        match self.cat.lang.as_str() {
            "ja" | "zh-Hans" | "zh-Hant" => "。",
            "hi" => "।",
            _ => ".",
        }
    }

    /// "Te perdonó. ¡Era justo lo que Ana quería!": sentences the game glues
    /// together. The line is cut where sentences may end, into pieces that
    /// all translate (the longest ones first). Every piece is looked up at
    /// the same depth, so a summary of eight sentences still translates.
    fn sentences(&self, s: &str, depth: usize) -> Option<String> {
        let cs: Vec<(usize, char)> = s.char_indices().collect();
        let mut cuts = Vec::new();
        for w in cs.windows(3) {
            let [(_, end), (sp, ' '), (_, next)] = w else {
                continue;
            };
            if matches!(end, '.' | '!' | '?' | '»' | '"' | '…' | ')')
                && (next.is_uppercase() || matches!(next, '¡' | '¿' | '«' | '"' | '('))
            {
                cuts.push(*sp);
            }
        }
        if cuts.is_empty() {
            return None;
        }
        // Piece i..j runs from the start after cut i-1 to cut j-1 (or the
        // ends of the line); done[j] is the line up to cut j-1, translated.
        let start = |i: usize| if i == 0 { 0 } else { cuts[i - 1] + 1 };
        let end = |j: usize| cuts.get(j - 1).copied().unwrap_or(s.len());
        let last = cuts.len() + 1;
        // First sentence by sentence (the usual case, and cheap); runs of
        // sentences only when that fails: one entry spanning two of them
        // ("Fueron juntos {0}… Al despedirse…"), tried as exact text or a
        // template, never cut again (that is this same search).
        for runs in [false, true] {
            let mut done: Vec<Option<String>> = vec![None; last + 1];
            done[0] = Some(String::new());
            for j in 1..=last {
                for i in (0..j).rev() {
                    let single = j - i == 1;
                    if !single && !runs {
                        break;
                    }
                    let Some(before) = done[i].as_ref() else {
                        continue;
                    };
                    let piece = &s[start(i)..end(j)];
                    if (i == 0 && j == last) || !has_letters(piece) {
                        continue;
                    }
                    let t = if single {
                        self.lookup(piece, depth + 1)
                    } else {
                        self.cat
                            .exact
                            .get(piece)
                            .cloned()
                            .or_else(|| self.match_template(piece, depth + 1, true))
                    };
                    let Some(t) = t else {
                        continue;
                    };
                    // Chinese and Japanese put no space after a full stop.
                    let glue = if before.is_empty()
                        || before.ends_with(['。', '！', '？', '」', '』', '）'])
                    {
                        ""
                    } else {
                        " "
                    };
                    done[j] = Some(format!("{before}{glue}{t}"));
                    break;
                }
            }
            if let Some(out) = done[last].take() {
                return Some(out);
            }
        }
        None
    }

    /// "Cinco años juntos (Hanna)": a line with an aside in parentheses at
    /// the end. The head translates on its own and the aside too, or stays as
    /// it is when it is a name.
    fn paren_tail(&self, s: &str, depth: usize) -> Option<String> {
        let body = s.strip_suffix(')')?;
        let open = body.rfind(" (")?;
        let (head, inner) = (&body[..open], &body[open + 2..]);
        if !has_letters(head) {
            return None;
        }
        let head = self.lookup(head, depth + 1)?;
        let inner = if !has_letters(inner) {
            inner.to_owned()
        } else if let Some(t) = self.lookup(inner, depth + 1) {
            t
        } else if self.is_name(inner) {
            inner.to_owned()
        } else {
            return None;
        };
        let out = format!("{head} ({inner})");
        (out != s).then_some(out)
    }

    /// "hace 3 años" inside a longer line: templates are stored as
    /// "Hace {0} años", so try them capitalized and lower the result again.
    fn lowercase_start(&self, s: &str, depth: usize) -> Option<String> {
        let mut cs = s.chars();
        let first = cs.next().filter(|c| c.is_lowercase())?;
        let cap: String = first.to_uppercase().collect::<String>() + cs.as_str();
        let t = self
            .match_template(&cap, depth + 1, true)
            .or_else(|| self.match_template(&cap, depth + 1, false))?;
        let mut ts = t.chars();
        let f = ts.next()?;
        let out = f.to_lowercase().collect::<String>() + ts.as_str();
        (out != s).then_some(out)
    }

    /// Lines the game builds by joining pieces ("Madre · enojada · Hace 3
    /// años", "Gustos: A, B", "Ana: ¿Salimos?") that are not in the catalog
    /// as a whole: every piece is translated on its own. If a piece with
    /// letters neither translates nor looks like a name, the line is left as
    /// it is (no mixed languages).
    fn split_list(&self, s: &str, depth: usize) -> Option<String> {
        'seps: for sep in [" · ", " › ", ": ", ", ", " / "] {
            if !s.contains(sep) {
                continue;
            }
            let mut out = Vec::new();
            let mut any = false;
            for piece in s.split(sep) {
                let core = piece.trim();
                if core.is_empty() || !has_letters(core) {
                    out.push(piece.to_owned());
                    continue;
                }
                match self.lookup(piece, depth + 1) {
                    Some(t) => {
                        any |= t != piece;
                        out.push(t);
                    }
                    None if self.is_name(core) => out.push(piece.to_owned()),
                    None => continue 'seps,
                }
            }
            if any {
                return Some(out.join(sep));
            }
        }
        None
    }

    fn match_template(&self, s: &str, depth: usize, strict: bool) -> Option<String> {
        let upper = is_upper_text(s);
        // Every template that matches, heaviest first: if the best one would
        // leave untranslated words, the next one may not.
        let mut found: Vec<(usize, Vec<(usize, String)>)> = Vec::new();
        let mut tried = std::collections::HashSet::new();
        let mut consider = |idx: usize| {
            if !tried.insert(idx) {
                return;
            }
            let t = &self.cat.templates[idx];
            let parts = if upper { &t.upper } else { &t.parts };
            for h in match_parts(parts, s) {
                found.push((idx, h));
            }
        };
        let key = prefix_key(s);
        if let Some(list) = self.cat.by_prefix.get(&key) {
            for &idx in list {
                consider(idx);
            }
        }
        // Shorter leading literals (1-2 chars) are indexed by their own length.
        for n in 1..PREFIX_CHARS {
            let k: String = s.chars().take(n).collect();
            if k.chars().count() == n {
                if let Some(list) = self.cat.by_prefix.get(&k) {
                    for &idx in list {
                        consider(idx);
                    }
                }
            }
        }
        for (idx, lit) in &self.cat.floating {
            let hit = if upper {
                s.contains(&lit.to_uppercase())
            } else {
                s.contains(lit.as_str())
            };
            if hit {
                consider(*idx);
            }
        }
        found.sort_by(|a, b| {
            self.cat.templates[b.0]
                .weight
                .cmp(&self.cat.templates[a.0].weight)
        });
        // A result identical to the line (an identity template with names in
        // its holes) is not a translation: the next candidate may be.
        found
            .into_iter()
            .take(MAX_CANDIDATES)
            .filter(|(idx, holes)| self.holes_ok(*idx, holes, depth, strict))
            .map(|(idx, holes)| {
                restore_case(
                    s,
                    self.render(&self.cat.templates[idx].target, &holes, depth),
                )
            })
            .find(|out| out != s)
    }

    // A template whose holes hold untranslated text would only translate the
    // glue ("{1} y {2}" -> "…和…", "{0} años" -> "Hace 29 ans") and mix two
    // languages: better to leave the line as it is (the pieces may still
    // translate one by one, see `split_list`). A hole shown as text must
    // translate, or (when not `strict`) be a short name; holes the target
    // keeps raw ({0!}) or only uses to pick a form ({0|o=..}) are fine as
    // they are.
    fn holes_ok(&self, idx: usize, holes: &[(usize, String)], depth: usize, strict: bool) -> bool {
        let t = &self.cat.templates[idx];
        for (i, v) in holes {
            if !has_letters(v) {
                continue;
            }
            let plain = t.target.contains(&format!("{{{i}}}"));
            let raw = t.target.contains(&format!("{{{i}!"));
            if !plain && !raw {
                continue;
            }
            // A hole kept as it is must really be a name ("Hanna", not
            // "¿Salimos este finde?"), or something said instead of a name
            // that translates ("Tu mamá").
            if raw && !plain {
                if self.is_name(v) || self.lookup(v, depth + 1).is_some() {
                    continue;
                }
                return false;
            }
            // As it comes: " Tienes el título." may only exist with its space.
            if self.lookup(v, depth + 1).is_some() {
                continue;
            }
            let long = v.split_whitespace().count() > 3;
            if strict || long || !self.is_name(v) {
                return false;
            }
        }
        true
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
            if inner.starts_with('@') {
                out.push('{');
                out.push_str(inner);
                out.push('}');
                rest = &after[close + 1..];
                continue;
            }
            let (num, select) = match inner.find('|') {
                Some(bar) => (&inner[..bar], Some(&inner[bar + 1..])),
                None => (inner, None),
            };
            let (num, keep_raw) = match num.strip_suffix('!') {
                Some(n) => (n, true),
                None => (num, false),
            };
            match num.parse::<usize>() {
                Ok(i) => {
                    let raw = value(i).unwrap_or("");
                    match select {
                        Some(options) => out.push_str(&pick_plural(options, raw, &self.cat.lang)),
                        None if keep_raw => {
                            // Names stay; a role said instead ("Tu mamá")
                            // translates.
                            let shown = if self.is_name(raw) || !has_letters(raw) {
                                None
                            } else {
                                self.lookup(raw, depth + 1)
                            };
                            out.push_str(shown.as_deref().unwrap_or(raw));
                        }
                        None => {
                            let shown = if raw.trim().is_empty() || !has_letters(raw) {
                                raw.to_owned()
                            } else {
                                self.lookup(raw, depth + 1)
                                    .unwrap_or_else(|| raw.to_owned())
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

/// CLDR-style plural category of `n` for a language (the common cases).
fn plural_category(lang: &str, n: i64) -> &'static str {
    let base = lang.split('-').next().unwrap_or(lang);
    let (m10, m100) = (n.abs() % 10, n.abs() % 100);
    match base {
        "ru" | "uk" | "be" => {
            if m10 == 1 && m100 != 11 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "pl" => {
            if n == 1 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "zh" | "ja" | "ko" | "th" => "other",
        "fr" | "pt" | "hi" => {
            if n == 0 || n == 1 {
                "one"
            } else {
                "other"
            }
        }
        _ => {
            if n == 1 {
                "one"
            } else {
                "other"
            }
        }
    }
}

/// Like [`pick_option`], but `#one=`/`#few=`/`#many=`/`#other=` keys match the
/// plural category of a numeric value.
fn pick_plural(options: &str, raw: &str, lang: &str) -> String {
    if options.contains('#') {
        let digits: String = raw
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        if let Ok(n) = digits.parse::<i64>() {
            let cat = plural_category(lang, n);
            let mut fallback = None;
            for opt in options.split('|') {
                if let Some((k, v)) = opt.split_once('=') {
                    if k.strip_prefix('#') == Some(cat) {
                        return v.to_owned();
                    }
                    if k == "#other" || k == "*" {
                        fallback = Some(v.to_owned());
                    }
                }
            }
            if let Some(f) = fallback {
                return f;
            }
        }
    }
    pick_option(options, raw)
}

/// `o=Bienvenido|a=Bienvenida|*=...` → the option for `raw` (or `*`).
fn pick_option(options: &str, raw: &str) -> String {
    let mut fallback = String::new();
    for opt in options.split('|') {
        if let Some((k, v)) = opt.split_once('=') {
            if k == raw || k.to_lowercase() == raw.to_lowercase() {
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
        Catalog::new(
            "en",
            entries.iter().map(|(a, b)| (a.to_string(), b.to_string())),
        )
    }

    #[test]
    fn plural_forms() {
        let c = Catalog::new(
            "ru",
            vec![(
                "{0} años".to_string(),
                "{0} {0|#one=год|#few=года|#many=лет}".to_string(),
            )],
        );
        let mut t = Translator::new(c);
        assert_eq!(t.tr("1 años"), "1 год");
        assert_eq!(t.tr("3 años"), "3 года");
        assert_eq!(t.tr("21 años"), "21 год");
        assert_eq!(t.tr("12 años"), "12 лет");
    }

    #[test]
    fn a_line_in_another_case_keeps_german_nouns_capitalized() {
        let de = Catalog::new(
            "de",
            vec![
                (
                    "ingreso bajo".to_string(),
                    "niedriges Einkommen".to_string(),
                ),
                ("Ponerte en forma".to_string(), "In Form kommen".to_string()),
            ],
        );
        let mut t = Translator::new(de);
        assert_eq!(t.tr("Ingreso bajo"), "Niedriges Einkommen");
        assert_eq!(t.tr("ponerte en forma"), "in Form kommen");
        // Elsewhere a label used inside a sentence is all lowercase.
        let mut t = Translator::new(cat(&[
            ("INFANCIA", "INFANCY"),
            ("Ponerte en forma", "Get in Shape"),
        ]));
        assert_eq!(t.tr("infancia"), "infancy");
        assert_eq!(t.tr("ponerte en forma"), "get in shape");
    }

    #[test]
    fn glue_templates_do_not_mix_languages() {
        let c = cat(&[("{0} y {1}", "{0} and {1}"), ("Hola", "Hello")]);
        let mut t = Translator::new(c);
        // Names in the holes: the glue is translated.
        assert_eq!(t.tr("Hanna y Adam"), "Hanna and Adam");
        // Whole untranslated sentences in a hole: the line is left as it is.
        let s = "Se acabó tu juventud y no cumpliste tu propósito.";
        assert_eq!(t.tr(s), s);
        // A pure glue template with a common word that has no translation.
        assert_eq!(t.tr("Hola y elefante"), "Hola y elefante");
    }

    #[test]
    fn joined_pieces_translate_one_by_one() {
        let c = cat(&[
            ("Madre", "Mother"),
            ("enojada", "angry"),
            ("Hace {0} años", "{0} years ago"),
            ("Gustos", "Into"),
            ("Fan de los tríos", "Threesome fan"),
            ("Amante del BDSM", "BDSM lover"),
            ("¿Salimos este finde?", "Going out this weekend?"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr("Madre · enojada · Hace 3 años"),
            "Mother · angry · 3 years ago"
        );
        assert_eq!(
            t.tr("Gustos: Fan de los tríos, Amante del BDSM"),
            "Into: Threesome fan, BDSM lover"
        );
        assert_eq!(t.tr("Madre › Gustos"), "Mother › Into");
        // A name before the colon stays as it is.
        assert_eq!(
            t.tr("Filip: ¿Salimos este finde?"),
            "Filip: Going out this weekend?"
        );
        // One piece without translation: the whole line is left alone.
        let s = "Madre · algo que no existe";
        assert_eq!(t.tr(s), s);
    }

    #[test]
    fn asides_in_parentheses_keep_the_name() {
        let c = cat(&[
            ("Cinco años juntos", "Five years together"),
            ("Hito: {0}.", "Milestone: {0}."),
            ("Amigo", "Friend"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr("Hito: Cinco años juntos (Hanna)."),
            "Milestone: Five years together (Hanna)."
        );
        assert_eq!(
            t.tr("Cinco años juntos (Amigo)"),
            "Five years together (Friend)"
        );
        // An aside that is neither a name nor in the catalog: left alone.
        let s = "Cinco años juntos (algo raro)";
        assert_eq!(t.tr(s), s);
    }

    #[test]
    fn an_id_that_is_also_a_word_still_translates() {
        let c = cat(&[
            ("piojos", "piojos"),
            ("Piojos", "Lice"),
            (
                "Tu familia te llevó al médico por {0} y te curaron.",
                "Your family took you to the doctor for {0}.",
            ),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(t.tr("piojos"), "lice");
        assert_eq!(
            t.tr("Tu familia te llevó al médico por piojos y te curaron."),
            "Your family took you to the doctor for lice."
        );
    }

    #[test]
    fn glued_sentences_find_their_template() {
        let c = cat(&[
            (" {0} se fue.", " {0} left."),
            ("Cobraste {0}.{1}", "You earned {0}.{1}"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr("Cobraste $500. Hanna se fue."),
            "You earned $500. Hanna left."
        );
    }

    #[test]
    fn glued_sentences_translate_one_by_one() {
        let c = cat(&[
            (
                "Te perdonó. Se quitaron un peso de encima.",
                "They forgave you. A weight off both your shoulders.",
            ),
            (
                " ¡Era justo lo que {0} quería!",
                " It was exactly what {0!} wanted!",
            ),
            (
                "Te dio {0} «para tus dulces».",
                "They gave you {0} \"for candy.\"",
            ),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr("Te perdonó. Se quitaron un peso de encima. ¡Era justo lo que Ana quería!"),
            "They forgave you. A weight off both your shoulders. It was exactly what Ana wanted!"
        );
        assert_eq!(
            t.tr("Te dio $5 «para tus dulces». ¡Era justo lo que Ana quería!"),
            "They gave you $5 \"for candy.\" It was exactly what Ana wanted!"
        );
        // A sentence without translation keeps the whole line as it is.
        let s = "Algo que no existe. ¡Era justo lo que Ana quería!";
        assert_eq!(t.tr(s), s);
    }

    #[test]
    fn long_lines_nothing_matches_stay_fast() {
        let c = cat(&[
            ("{0} {1}", "{0} {1}"),
            ("{0}, {1}", "{0}, {1}"),
            ("{0}: {1}", "{0}: {1}"),
            ("{0}. {1}", "{0}. {1}"),
            ("{0} y {1}", "{0} and {1}"),
            ("{0} de {1}", "{1}'s {0}"),
            ("Hola", "Hello"),
        ]);
        let mut t = Translator::new(c);
        let line = (0..40)
            .map(|i| format!("Palabra{i} de cosa y otra, más: Fin{i}."))
            .collect::<Vec<_>>()
            .join(" ");
        let start = std::time::Instant::now();
        let _ = t.tr(&line);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn a_full_stop_inside_a_hole_is_not_the_split() {
        let c = cat(&[
            (
                "Requisitos: {0}.{1} Rangos: {2}.",
                "Requirements: {0}.{1} Ranks: {2}.",
            ),
            ("Intel. {0}", "Int. {0}"),
            ("Atractivo {0}", "Looks {0}"),
            (" Tienes el título.", " You have the degree."),
            ("Extra", "Extra"),
            ("Leyenda", "Legend"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr(
                "Requisitos: Intel. 25 · Atractivo 50. Tienes el título. Rangos: Extra › Leyenda."
            ),
            "Requirements: Int. 25 · Looks 50. You have the degree. Ranks: Extra › Legend."
        );
    }

    #[test]
    fn a_number_word_is_not_kept_as_a_name() {
        let c = cat(&[
            ("{0} años{1}", "Age {0}{1}"),
            ("Cinco años juntos", "Five years together"),
            ("juntos", "together"),
            ("Tu {0} murió.", "Your {0} died."),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr("Cinco años juntos (Hanna)"),
            "Five years together (Hanna)"
        );
        // A real name in a plain hole still works when nothing else fits.
        assert_eq!(t.tr("Tu Hanna murió."), "Your Hanna died.");
    }

    #[test]
    fn templates_never_leave_unknown_words_in_a_hole() {
        let c = cat(&[
            ("{0} · {1} años", "{0} · {1} years"),
            ("Tío", "Uncle"),
            ("cariñoso", "loving"),
            ("Hace {0} años", "{0} years ago"),
            ("Tu {0} te llama.", "Your {0} is calling."),
        ]);
        let mut t = Translator::new(c);
        // "{0} · {1} años" would give "Uncle · cariñoso · Hace 29 years":
        // the pieces translate one by one instead.
        assert_eq!(
            t.tr("Tío · cariñoso · Hace 29 años"),
            "Uncle · loving · 29 years ago"
        );
        // An unknown lowercase word in a hole: the line stays as it is.
        assert_eq!(t.tr("Tu perrito te llama."), "Tu perrito te llama.");
        // A name is fine.
        assert_eq!(t.tr("Tu Firulais te llama."), "Your Firulais is calling.");
        // Lowercase use of a capitalized template.
        assert_eq!(t.tr("Tío · hace 29 años"), "Uncle · 29 years ago");
    }

    #[test]
    fn common_words_are_not_names() {
        let c = cat(&[
            ("{0} · {1} años", "{0} · {1} years"),
            ("{0} años", "{0} years"),
            ("{0} · {1}", "{0} · {1}"),
            ("Tío", "Uncle"),
            ("cariñoso", "loving"),
            ("Hace {0} años", "{0} years ago"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(
            t.tr("Tío · cariñoso · Hace 29 años"),
            "Uncle · loving · 29 years ago"
        );
    }

    #[test]
    fn name_holes_translate_roles_said_instead_of_names() {
        let c = cat(&[
            ("{0} te saluda.", "{0!} says hi."),
            ("tu mamá", "your mom"),
            ("{0} TE BUSCA", "{0!} IS LOOKING FOR YOU"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(t.tr("Tu mamá te saluda."), "Your mom says hi.");
        assert_eq!(t.tr("Rosa te saluda."), "Rosa says hi.");
        assert_eq!(t.tr("TU MAMÁ TE BUSCA"), "YOUR MOM IS LOOKING FOR YOU");
        // Neither a name nor translatable: the template does not apply.
        assert_eq!(t.tr("Algo raro te saluda."), "Algo raro te saluda.");
    }

    #[test]
    fn names_with_a_spanish_looking_word_are_still_names() {
        let c = cat(&[
            ("{0} te saluda.", "{0!} says hi."),
            ("{0} TE BUSCA", "{0!} IS LOOKING FOR YOU"),
            ("Tu {0} {1} murió.", "Your {0} {1} died."),
            ("abuelo", "grandfather"),
            ("tu mamá", "your mom"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(t.tr("Shwe Sin te saluda."), "Shwe Sin says hi.");
        assert_eq!(t.tr("SI THU TE BUSCA"), "SI THU IS LOOKING FOR YOU");
        assert_eq!(
            t.tr("Tu abuelo Si Thu murió."),
            "Your grandfather Si Thu died."
        );
        // A common word next to a known one is still text, not a name.
        assert_eq!(t.tr("TU MAMÁ TE BUSCA"), "YOUR MOM IS LOOKING FOR YOU");
    }

    #[test]
    fn a_bullet_a_full_stop_or_a_price_around_a_known_piece() {
        let c = cat(&[
            ("Economía de {0}", "{0}'s economy"),
            ("Guerra de Gaza", "Gaza War"),
            ("Casa con jardín", "House with a yard"),
        ]);
        let mut t = Translator::new(c);
        assert_eq!(t.tr("- Economía de Myanmar"), "- Myanmar's economy");
        assert_eq!(t.tr("Guerra de Gaza."), "Gaza War.");
        assert_eq!(
            t.tr("Casa con jardín $350,000"),
            "House with a yard $350,000"
        );
        let mut u = Translator::new(cat(&[("contratación", "hiring"), ("sueldo", "salary")]));
        assert_eq!(
            u.tr("-8 contratación · +40 % sueldo"),
            "-8 hiring · +40 % salary"
        );
        let mut v = Translator::new(cat(&[
            ("deprimido", "deprimido"),
            ("Deprimid{0}", "Depressed"),
            ("% sueldo", "% salary"),
        ]));
        assert_eq!(v.tr("Deprimido"), "Depressed");
        assert_eq!(v.tr("+40 % sueldo"), "+40 % salary");
        // Nothing known inside: the line stays as it is.
        assert_eq!(t.tr("Algo raro."), "Algo raro.");
        let mut t = Translator::new(Catalog::new(
            "ja",
            vec![("Guerra de Gaza".to_string(), "ガザ戦争".to_string())],
        ));
        assert_eq!(t.tr("Guerra de Gaza."), "ガザ戦争。");
    }

    #[test]
    fn a_summary_of_many_glued_sentences_translates() {
        let c = cat(&[(
            "De los {0} a los {1}: {2} hitos.",
            "From {0} to {1}: {2} milestones.",
        )]);
        let mut t = Translator::new(c);
        let join = |f: &dyn Fn(usize) -> String| (0..8).map(f).collect::<Vec<_>>().join(" ");
        let line = join(&|d| format!("De los {} a los {}: {d} hitos.", d * 10, d * 10 + 9));
        let want = join(&|d| format!("From {} to {}: {d} milestones.", d * 10, d * 10 + 9));
        assert_eq!(t.tr(&line), want);
    }

    #[test]
    fn name_holes_only_take_names() {
        let c = cat(&[("{0}: {1}", "{0!}: {1!}"), ("¿Salimos?", "Going out?")]);
        let mut t = Translator::new(c);
        // The identity template would keep the Spanish line: the pieces
        // translate one by one instead.
        assert_eq!(t.tr("Filip: ¿Salimos?"), "Filip: Going out?");
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
            ("Rosa", "Pink"),
            ("{0} te saluda.", "{0!} says hi."),
            ("CONTENT{0}", "{0|o=HAPPY|a=HAPPY}"),
            ("Fuiste al cine.", "{@g|m=Ты пошёл|f=Ты пошла} в кино."),
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
        assert_eq!(t.tr("Rosa te saluda."), "Rosa says hi.");
        assert_eq!(t.tr("CONTENTA"), "HAPPY");
        set_var("g", "f");
        assert_eq!(t.tr("Fuiste al cine."), "Ты пошла в кино.");
        assert_eq!(t.tr("FUISTE AL CINE."), "ТЫ ПОШЛА В КИНО.");
        assert_eq!(t.tr("$1,200"), "$1,200");
        // Translated output is not translated again.
        assert_eq!(t.tr("Father"), "Father");
    }
}
