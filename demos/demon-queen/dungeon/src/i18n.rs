//! Text in the player's language. Every string the game draws goes through [`tr`], which looks it
//! up in the catalogs of `data/locales/text/<code>/*.json` (Spanish → that language). Spanish is
//! the source text and needs no catalog: a string with no entry is drawn as written.
//!
//! An entry may have holes, `{0}`, `{1}`…: «Guardia {0}%» translates «Guardia 45%», built at run
//! time, as «Guard 45%». What fills a hole is translated too when it is an entry of its own
//! («{0}: +{1} vitalidad» with an item's name in the first hole). A text of several lines with no
//! entry of its own is translated line by line.
//!
//! `VELVET_I18N_MISSING=<file>` appends every drawn text that looks Spanish and has no entry
//! (once each) to that file: play, then translate what it lists.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

/// A catalog entry with holes, split at them: `parts[0] {a} parts[1] {b} … parts[n]`.
struct Template {
    parts: Vec<String>,
    holes: Vec<usize>,
    out: String,
    /// Hardly any words of its own («{0} de {1}»): its holes may only hold numbers, or it would
    /// match half the language.
    short: bool,
}

struct Catalog {
    code: String,
    exact: HashMap<String, String>,
    templates: Vec<Template>,
    /// Every translation, so a text already in the language is not taken for a missing one.
    outputs: HashSet<String>,
    /// The entries in capitals (text the code upper-cases after the fact: «MISIÓN»).
    upper: HashMap<String, String>,
}

fn catalog_slot() -> &'static RwLock<Option<Arc<Catalog>>> {
    static SLOT: OnceLock<RwLock<Option<Arc<Catalog>>>> = OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(None))
}

fn cache() -> &'static Mutex<HashMap<String, Option<Arc<str>>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<str>>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn locale_dir() -> &'static Mutex<Option<PathBuf>> {
    static DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    DIR.get_or_init(|| Mutex::new(None))
}

/// Where the catalogs live: the folder holding `text/<code>/`.
pub fn set_locale_dir(dir: impl Into<PathBuf>) {
    *locale_dir().lock().unwrap() = Some(dir.into());
}

/// The language drawn from now on (`es`: the source text, no catalog).
pub fn set_language(code: &str) {
    let catalog = if code == "es" {
        None
    } else {
        // English under any other language: what it has not translated yet shows in English,
        // which more players read than the Spanish source.
        let dir = locale_dir().lock().unwrap().clone();
        dir.and_then(|d| {
            let mut folders = vec![d.join("text").join(code)];
            if code != "en" {
                folders.insert(0, d.join("text").join("en"));
            }
            load(&folders, code)
        })
        .map(Arc::new)
    };
    *catalog_slot().write().unwrap() = catalog;
    cache().lock().unwrap().clear();
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Goes up each time the language changes (font atlases rebuild their glyphs on it).
pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

/// Every text the active language's catalog can draw (so a font atlas knows the glyphs it needs).
pub fn catalog_texts() -> Vec<String> {
    catalog_slot()
        .read()
        .unwrap()
        .as_ref()
        .map(|c| {
            let mut all: Vec<String> = c.outputs.iter().cloned().collect();
            all.extend(c.upper.values().cloned());
            all
        })
        .unwrap_or_default()
}

/// What never opens a line in Japanese or Chinese (kinsoku): closing marks, stops, small kana, the
/// long vowel, the repeat marks.
const NO_LINE_START: &str = "、。，．,.!?！？）)」』】〕〉》〙〗〟’”…‥ーゝゞヽヾぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶ々〻・：；:;%％";
/// What never ends one: opening marks.
const NO_LINE_END: &str = "（(「『【〔〈《〘〖〝‘“";

/// `text` (a script written without spaces between words: Chinese, Japanese, Thai) broken between
/// its letters into lines no wider than `max` as `width` measures them, by the kinsoku rules: a
/// closing mark, a small kana or the long vowel never opens a line (it hangs at the end of the one
/// before), and an opening mark never ends one (it goes down with what it opens).
pub fn wrap_letters(text: &str, max: f32, width: impl Fn(&str) -> f32) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    let starts = |g: &str| g.chars().next().is_some_and(|c| NO_LINE_START.contains(c));
    let ends = |g: &str| g.chars().last().is_some_and(|c| NO_LINE_END.contains(c));
    let mut lines = Vec::new();
    let mut line = String::new();
    for g in text.graphemes(true) {
        if !line.is_empty() && !starts(g) && width(&format!("{line}{g}")) > max {
            let mut keep = line.len();
            for (i, h) in line.grapheme_indices(true).rev() {
                if !ends(h) {
                    break;
                }
                keep = i;
            }
            if keep > 0 {
                let carry = line.split_off(keep);
                lines.push(std::mem::replace(&mut line, carry));
            }
        }
        line.push_str(if line.is_empty() { g.trim_start() } else { g });
    }
    lines.push(line);
    lines
}

/// Whether the language is written right to left.
pub fn is_rtl_language(code: &str) -> bool {
    matches!(code, "he" | "ar" | "fa" | "ur")
}

/// Whether `c` is in a script whose letters combine and reorder when written (they need shaping,
/// not one glyph per character): Devanagari, Thai, Hebrew and Arabic marks.
pub fn needs_shaping(c: char) -> bool {
    matches!(c as u32, 0x0900..=0x097F | 0x0E00..=0x0E7F | 0x0590..=0x05FF | 0x0600..=0x06FF | 0xA8E0..=0xA8FF)
}

/// The language's own font (Windows' fonts): its bytes, its index in a collection, and whether its
/// text must be shaped (letters that join and reorder: Devanagari, Thai; Hebrew reads right to
/// left). Under the game's faces it draws what they lack.
pub fn script_font(language: &str) -> Option<(std::sync::Arc<Vec<u8>>, u32, bool)> {
    let names: &[&str] = match language {
        "zh-Hans" => &["msyh.ttc", "msyhbd.ttc", "simsun.ttc"],
        "zh-Hant" => &["msjh.ttc", "msjhbd.ttc", "mingliub.ttc"],
        "ja" => &["YuGothM.ttc", "YuGothR.ttc", "meiryo.ttc", "msgothic.ttc"],
        "ko" => &["malgun.ttf", "malgunbd.ttf", "gulim.ttc"],
        "hi" => &["Nirmala.ttc", "Nirmala.ttf", "mangal.ttf"],
        "th" => &["LeelawUI.ttf", "leelawui.ttf", "LeelUIsl.ttf", "tahoma.ttf"],
        _ => &["segoeuib.ttf", "segoeui.ttf", "arial.ttf"],
    };
    let shaped = matches!(language, "hi" | "th" | "he");
    let dir = PathBuf::from(std::env::var_os("WINDIR")?).join("Fonts");
    names.iter().find_map(|n| {
        let bytes = std::fs::read(dir.join(n)).ok()?;
        rustybuzz::Face::from_slice(&bytes, 0)?;
        Some((std::sync::Arc::new(bytes), 0, shaped))
    })
}

/// `line` shaped in a font: glyph ids with advance and offsets (font units × `k`, y down), in
/// display order (right-to-left runs reordered by the bidirectional algorithm, then shaped).
pub fn shape_line(bytes: &[u8], index: u32, k: f32, line: &str) -> Vec<(u16, f32, f32, f32)> {
    let Some(face) = rustybuzz::Face::from_slice(bytes, index) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (run, rtl) in visual_runs(line) {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(&run);
        buffer.guess_segment_properties();
        buffer.set_direction(if rtl {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
        let shaped = rustybuzz::shape(&face, &[], buffer);
        for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            out.push((
                info.glyph_id as u16,
                pos.x_advance as f32 * k,
                pos.x_offset as f32 * k,
                -(pos.y_offset as f32) * k,
            ));
        }
    }
    out
}

/// One line in display order: its runs left to right, each with whether it reads right to left
/// (the Unicode bidirectional algorithm; a line with no right-to-left text is one run).
pub fn visual_runs(line: &str) -> Vec<(String, bool)> {
    if !line
        .chars()
        .any(|c| matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF))
    {
        return vec![(line.to_owned(), false)];
    }
    let info = unicode_bidi::BidiInfo::new(line, None);
    let Some(para) = info.paragraphs.first() else {
        return vec![(line.to_owned(), false)];
    };
    let (levels, runs) = info.visual_runs(para, para.range.clone());
    runs.into_iter()
        .map(|r| (line[r.clone()].to_owned(), levels[r.start].is_rtl()))
        .collect()
}

/// The language being drawn.
pub fn language() -> String {
    catalog_slot()
        .read()
        .unwrap()
        .as_ref()
        .map_or_else(|| "es".to_owned(), |c| c.code.clone())
}

/// The catalogs of `folders`, each over the ones before it.
fn load(folders: &[PathBuf], code: &str) -> Option<Catalog> {
    let mut exact = HashMap::new();
    let mut templates = Vec::new();
    let mut files = Vec::new();
    for dir in folders {
        let mut these: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        these.sort();
        files.extend(these);
    }
    if files.is_empty() {
        return None;
    }
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(error) => {
                eprintln!("[i18n] {}: {error}", file.display());
                continue;
            }
        };
        let entries = value.get("entries").unwrap_or(&value);
        let Some(map) = entries.as_object() else {
            continue;
        };
        for (source, target) in map {
            let Some(target) = target.as_str() else {
                continue;
            };
            // Empty only for punctuation (Spanish's «¡» and «¿» have no English).
            if target.is_empty() && source.chars().any(char::is_alphabetic) {
                continue;
            }
            match template(source, target) {
                Some(t) => {
                    templates.retain(|old: &Template| old.parts != t.parts || old.holes != t.holes);
                    templates.push(t);
                }
                None => {
                    exact.insert(source.clone(), target.to_owned());
                }
            }
        }
    }
    // The most specific first: more literal text to match.
    templates.sort_by_key(|t| std::cmp::Reverse(t.parts.iter().map(|p| p.len()).sum::<usize>()));
    // A line shown a sentence at a time: its first sentences, when both sides have as many.
    let mut prefixes = Vec::new();
    for (source, target) in &exact {
        let (a, b) = (sentences(source), sentences(target));
        if a.len() > 1 && a.len() == b.len() {
            for k in 1..a.len() {
                prefixes.push((
                    a[..k].concat().trim_end().to_owned(),
                    b[..k].concat().trim_end().to_owned(),
                ));
            }
        }
    }
    for (a, b) in prefixes {
        exact.entry(a).or_insert(b);
    }
    let upper = exact
        .iter()
        .filter(|(k, _)| k.chars().any(char::is_lowercase))
        .map(|(k, v)| (k.to_uppercase(), v.to_uppercase()))
        .collect();
    let outputs = exact.values().cloned().collect();
    eprintln!(
        "[i18n] {code}: {} entries, {} with holes",
        exact.len(),
        templates.len()
    );
    Some(Catalog {
        code: code.to_owned(),
        exact,
        templates,
        outputs,
        upper,
    })
}

/// `s` cut after each sentence's end (`.`, `?`, `!`, `…` before a space), the space kept.
fn sentences(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end = matches!(c, '.' | '?' | '!' | '…');
        if end && chars.get(i + 1).is_some_and(|n| n.is_whitespace()) {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `source` split at its holes (`{0}`…); `None` without any.
fn template(source: &str, out: &str) -> Option<Template> {
    let mut parts = vec![String::new()];
    let mut holes = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '{' {
            let rest = &source[i + 1..];
            if let Some(end) = rest.find('}') {
                if let Ok(n) = rest[..end].parse::<usize>() {
                    holes.push(n);
                    parts.push(String::new());
                    for _ in 0..=end {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        parts.last_mut().unwrap().push(c);
    }
    let letters: usize = parts
        .iter()
        .map(|p| p.chars().filter(|c| c.is_alphabetic()).count())
        .sum();
    (!holes.is_empty()).then(|| Template {
        parts,
        holes,
        out: out.to_owned(),
        short: letters < 4,
    })
}

/// The text filling each hole of `t` in `s`, if `s` has its shape.
fn fill(t: &Template, s: &str) -> Option<Vec<String>> {
    let first = &t.parts[0];
    let last = t.parts.last().unwrap();
    if !s.starts_with(first.as_str())
        || !s.ends_with(last.as_str())
        || s.len() < first.len() + last.len()
    {
        return None;
    }
    let body = &s[first.len()..s.len() - last.len()];
    let mut caps = Vec::new();
    let mut rest = body;
    for part in &t.parts[1..t.parts.len() - 1] {
        // A hole holds at least one character.
        let start = rest.char_indices().nth(1).map_or(rest.len(), |(i, _)| i);
        let found = rest.get(start..)?.find(part.as_str())? + start;
        caps.push(rest[..found].to_owned());
        rest = &rest[found + part.len()..];
    }
    if rest.is_empty() {
        return None;
    }
    caps.push(rest.to_owned());
    Some(caps)
}

fn translate(catalog: &Catalog, s: &str, depth: u8) -> Option<String> {
    if let Some(t) = catalog.exact.get(s) {
        return Some(t.clone());
    }
    // Spaces round it are kept as they are.
    let trimmed = s.trim();
    if trimmed.len() != s.len() && !trimmed.is_empty() {
        if let Some(t) = catalog.exact.get(trimmed) {
            let lead = &s[..s.len() - s.trim_start().len()];
            let tail = &s[s.trim_end().len()..];
            return Some(format!("{lead}{t}{tail}"));
        }
    }
    if let Some(t) = catalog.upper.get(s) {
        return Some(t.clone());
    }
    for t in &catalog.templates {
        if let Some(caps) = fill(t, s) {
            if t.short && caps.iter().any(|c| c.chars().any(char::is_alphabetic)) {
                continue;
            }
            let mut out = t.out.clone();
            for (k, n) in t.holes.iter().enumerate() {
                let cap = &caps[k];
                let filled = if depth < 2 {
                    translate(catalog, cap, depth + 1).unwrap_or_else(|| cap.clone())
                } else {
                    cap.clone()
                };
                out = out.replace(&format!("{{{n}}}"), &filled);
            }
            return Some(out);
        }
    }
    if depth < 3 {
        // Quoted, in brackets, between dashes: the inside, decorated the same way.
        const WRAPS: [(&str, &str, &str, &str); 6] = [
            ("«", "»", "“", "”"),
            ("(", ")", "(", ")"),
            ("“", "”", "“", "”"),
            ("\"", "\"", "\"", "\""),
            ("— ", " —", "— ", " —"),
            ("—", "", "—", ""),
        ];
        for (open, close, o2, c2) in WRAPS {
            if s.len() > open.len() + close.len() && s.starts_with(open) && s.ends_with(close) {
                let inner = &s[open.len()..s.len() - close.len()];
                if let Some(t) = translate(catalog, inner, depth + 1) {
                    return Some(format!("{o2}{t}{c2}"));
                }
            }
        }
        // Sentences put together: the longest run of them that is an entry, then the rest.
        let parts = sentences(s);
        if parts.len() > 1 {
            let mut out = String::new();
            let mut any = false;
            let mut i = 0;
            while i < parts.len() {
                let mut taken = false;
                for j in (i + 1..=parts.len()).rev() {
                    let run = parts[i..j].concat();
                    // The space or line break before a sentence comes with it: keep it, or the
                    // sentences come out glued («One.Two.»). A space after a full-width stop is
                    // not written in Chinese or Japanese.
                    let lead = &run[..run.len() - run.trim_start().len()];
                    let tail = &run[run.trim_end().len()..];
                    if let Some(t) = translate(catalog, run.trim(), depth + 1) {
                        let cjk_stop = out.ends_with(['。', '！', '？', '」', '』']);
                        if lead.contains('\n') || !cjk_stop {
                            out.push_str(lead);
                        }
                        out.push_str(&t);
                        out.push_str(tail);
                        any = true;
                        taken = true;
                        i = j;
                        break;
                    }
                }
                if !taken {
                    out.push_str(&parts[i]);
                    i += 1;
                }
            }
            if any {
                return Some(out);
            }
        }
        // Pieces put together with a separator: each on its own.
        for sep in [" · ", " + ", " = ", " × ", " ×", " | ", ": ", " – "] {
            if s.contains(sep) {
                let mut any = false;
                let parts: Vec<String> = s
                    .split(sep)
                    .map(|p| match translate(catalog, p, depth + 1) {
                        Some(t) => {
                            any = true;
                            t
                        }
                        None => p.to_owned(),
                    })
                    .collect();
                if any {
                    return Some(parts.join(sep));
                }
            }
        }
    }
    // Several lines: each on its own.
    if depth == 0 && s.contains('\n') {
        let mut any = false;
        let lines: Vec<String> = s
            .split('\n')
            .map(|line| match translate(catalog, line, 1) {
                Some(t) => {
                    any = true;
                    t
                }
                None => line.to_owned(),
            })
            .collect();
        if any {
            return Some(lines.join("\n"));
        }
    }
    None
}

/// `s` in the language being drawn (as written when there is no entry for it).
pub fn tr(s: &str) -> Cow<'_, str> {
    if s.is_empty() {
        return Cow::Borrowed(s);
    }
    let Some(catalog) = catalog_slot().read().unwrap().clone() else {
        return Cow::Borrowed(s);
    };
    // Punctuation on its own («¡» opening a shout) only by an exact entry.
    if !s.chars().any(char::is_alphabetic) {
        return match catalog.exact.get(s) {
            Some(t) => Cow::Owned(t.clone()),
            None => Cow::Borrowed(s),
        };
    }
    if let Some(hit) = cache().lock().unwrap().get(s) {
        return match hit {
            Some(t) => Cow::Owned(t.to_string()),
            None => Cow::Borrowed(s),
        };
    }
    let found = translate(&catalog, s, 0);
    // In a language not written in Latin letters, any word in them with no entry is suspect too
    // («mirarlo» reads as nothing in particular, but it is no Russian).
    let latin_left = !matches!(
        catalog.code.as_str(),
        "es" | "en" | "pt-BR" | "de" | "fr" | "it" | "pl"
    ) && s
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| w.len() >= 3 && w.chars().any(|c| c.is_ascii_lowercase()));
    if found.is_none() && !catalog.outputs.contains(s) && (looks_spanish(s) || latin_left) {
        note_missing(s);
    }
    let mut cache = cache().lock().unwrap();
    if cache.len() > 40_000 {
        cache.clear();
    }
    cache.insert(s.to_owned(), found.as_deref().map(Arc::from));
    match found {
        Some(t) => Cow::Owned(t),
        None => Cow::Borrowed(s),
    }
}

/// The sign of a model in the player's language: pictures with words on them (a shop's board, a
/// map on a table) drawn again in it, `data/locales/textures/<code>/<model file>/<index>.png`
/// (made by `tools/i18n_textures.mjs`); English under any other language, as with the text.
pub fn image_override(
    path: &std::path::Path,
    index: usize,
    width: u32,
    height: u32,
) -> Option<image::RgbaImage> {
    let code = language();
    if code == "es" {
        return None;
    }
    let dir = locale_dir().lock().unwrap().clone()?;
    let file = path.file_name()?.to_str()?;
    let mut codes = vec![code.clone()];
    if code != "en" {
        codes.push("en".to_owned());
    }
    for code in codes {
        for ext in ["png", "jpg"] {
            let picture = dir
                .join("textures")
                .join(&code)
                .join(file)
                .join(format!("{index}.{ext}"));
            if let Ok(img) = velvet_avif::open_image(&picture) {
                let rgba = img.to_rgba8();
                if rgba.dimensions() == (width, height) {
                    return Some(rgba);
                }
            }
        }
    }
    None
}

/// A 2D picture with words on it (a background's sign…) drawn again in the player's language:
/// `data/locales/images/<code>/<its path under data/assets>`, English under any other language, as
/// with the text. `None`: the picture as it is.
pub fn localized_picture(relative: &std::path::Path) -> Option<PathBuf> {
    let code = language();
    if code == "es" {
        return None;
    }
    let dir = locale_dir().lock().unwrap().clone()?;
    let mut codes = vec![code.clone()];
    if code != "en" {
        codes.push("en".to_owned());
    }
    codes
        .into_iter()
        .map(|c| dir.join("images").join(c).join(relative))
        .find(|p| p.is_file())
}

/// Whether image `index` of model `path` is a sign drawn again in some language (so it changes
/// with the player's language).
pub fn is_sign(path: &std::path::Path, index: usize) -> bool {
    let Some(dir) = locale_dir().lock().unwrap().clone() else {
        return false;
    };
    let Some(file) = path.file_name().and_then(|f| f.to_str()) else {
        return false;
    };
    let Ok(languages) = std::fs::read_dir(dir.join("textures")) else {
        return false;
    };
    languages.flatten().any(|language| {
        ["png", "jpg"].iter().any(|ext| {
            language
                .path()
                .join(file)
                .join(format!("{index}.{ext}"))
                .is_file()
        })
    })
}

/// [`tr`] as an owned string.
pub fn tr_string(s: &str) -> String {
    tr(s).into_owned()
}

/// Whether `s` reads as Spanish (for the list of what is missing).
pub fn looks_spanish(s: &str) -> bool {
    if s.chars().any(|c| "áéíóúñ¿¡ÁÉÍÓÚÑ".contains(c)) {
        return true;
    }
    const WORDS: [&str; 44] = [
        "de", "la", "el", "los", "las", "que", "con", "por", "para", "una", "del", "al", "tu",
        "su", "sus", "es", "se", "lo", "le", "mi", "y", "ya", "muy", "pero", "un", "en", "te",
        "me", "sin", "hay", "esto", "eso", "aquí", "ahora", "hoy", "nada", "algo", "todo", "otra",
        "otro", "usted", "puedo", "oro", "señora",
    ];
    s.split(|c: char| !c.is_alphabetic())
        .any(|w| WORDS.contains(&w.to_lowercase().as_str()))
}

fn note_missing(s: &str) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let Ok(path) = std::env::var("VELVET_I18N_MISSING") else {
        return;
    };
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if !seen.lock().unwrap().insert(s.to_owned()) {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", serde_json::to_string(s).unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(entries: &[(&str, &str)]) -> Catalog {
        let mut exact = HashMap::new();
        let mut templates = Vec::new();
        for (s, t) in entries {
            match template(s, t) {
                Some(tp) => templates.push(tp),
                None => {
                    exact.insert((*s).to_owned(), (*t).to_owned());
                }
            }
        }
        Catalog {
            code: "en".into(),
            exact,
            templates,
            outputs: HashSet::new(),
            upper: HashMap::new(),
        }
    }

    #[test]
    fn letters_break_by_kinsoku() {
        let chars = |s: &str| s.chars().count() as f32;
        // A stop or a comma hangs at the end of the line instead of opening the next one.
        assert_eq!(
            wrap_letters("あいうえお、かきくけこ。", 5.0, chars),
            ["あいうえお、", "かきくけこ。"]
        );
        // An opening mark goes down with what it opens.
        assert_eq!(
            wrap_letters("あいうえ「かきく」", 5.0, chars),
            ["あいうえ", "「かきく」"]
        );
        // The small kana and the long vowel stay with the letter before them.
        assert_eq!(
            wrap_letters("あいうえおっー", 5.0, chars),
            ["あいうえおっー"]
        );
        assert_eq!(wrap_letters("短い", 5.0, chars), ["短い"]);
    }

    #[test]
    fn exact_holes_nested_and_lines() {
        let c = catalog(&[
            ("Hablar", "Talk"),
            ("Guardia {0}%", "Guard {0}%"),
            ("{0}: +{1} vitalidad.", "{0}: +{1} vitality."),
            ("Poción reforzada", "Strong potion"),
            ("Mañana", "Tomorrow"),
        ]);
        assert_eq!(translate(&c, "Hablar", 0).as_deref(), Some("Talk"));
        assert_eq!(translate(&c, "  Hablar ", 0).as_deref(), Some("  Talk "));
        assert_eq!(
            translate(&c, "Guardia 45%", 0).as_deref(),
            Some("Guard 45%")
        );
        assert_eq!(
            translate(&c, "Poción reforzada: +80 vitalidad.", 0).as_deref(),
            Some("Strong potion: +80 vitality.")
        );
        assert_eq!(
            translate(&c, "Hablar\nMañana", 0).as_deref(),
            Some("Talk\nTomorrow")
        );
        assert_eq!(translate(&c, "Nada que ver", 0), None);
        // Sentences translated one by one keep the space or the line break between them.
        let s = catalog(&[("Uno.", "One."), ("Dos.", "Two."), ("Tres.", "三。")]);
        assert_eq!(translate(&s, "Uno. Dos.", 0).as_deref(), Some("One. Two."));
        assert_eq!(
            translate(&s, "Uno.\nDos.", 0).as_deref(),
            Some("One.\nTwo.")
        );
        assert_eq!(translate(&s, "Tres. Tres.", 0).as_deref(), Some("三。三。"));
        // Holes of a template with hardly any words only take numbers.
        let c = catalog(&[("{0} de {1}", "{0} of {1}"), ("Hablar", "Talk")]);
        assert_eq!(translate(&c, "3 de 5", 0).as_deref(), Some("3 of 5"));
        assert_eq!(translate(&c, "nunca está de más", 0), None);
        // Decorations and pieces.
        assert_eq!(translate(&c, "«Hablar»", 0).as_deref(), Some("“Talk”"));
        assert_eq!(
            translate(&c, "Hablar · 3 de 5", 0).as_deref(),
            Some("Talk · 3 of 5")
        );
    }
}
