use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use fontdue::{Font, FontSettings};

use velvet_script_vs3::Value;

use super::layout;
use crate::assets::{AppManifest, FallbackFontSpec};

fn round2(value: f32) -> f64 {
    (f64::from(value) * 100.0).round() / 100.0
}

static NEXT_FONT_KEY: AtomicUsize = AtomicUsize::new(1);

/// A parsed font plus its bytes (needed for shaping) and a stable identity
/// for glyph caches.
pub struct LoadedFont {
    pub font: Font,
    pub bytes: Arc<Vec<u8>>,
    pub key: usize,
}

impl LoadedFont {
    fn new(font: Font, bytes: Vec<u8>) -> Self {
        Self {
            font,
            bytes: Arc::new(bytes),
            key: NEXT_FONT_KEY.fetch_add(1, Ordering::Relaxed),
        }
    }
    /// Whether this font has a real glyph for `ch`.
    pub fn has(&self, ch: char) -> bool {
        self.font.lookup_glyph_index(ch) != 0
    }
}

/// The font a command asked for plus the active fallbacks, tried in order
/// for characters the main font lacks.
#[derive(Clone)]
pub struct FontChain {
    pub primary: Arc<LoadedFont>,
    pub fallbacks: Arc<Vec<Arc<LoadedFont>>>,
}

impl FontChain {
    /// Font that should draw `ch`.
    pub fn font_for(&self, ch: char) -> &Arc<LoadedFont> {
        if ch.is_whitespace() || ch.is_control() || self.primary.has(ch) {
            return &self.primary;
        }
        self.fallbacks
            .iter()
            .find(|f| f.has(ch))
            .unwrap_or(&self.primary)
    }
}

pub struct FontSystem {
    fonts: HashMap<String, Arc<LoadedFont>>,
    data: Vec<Arc<Vec<u8>>>,
    default: String,
    root: PathBuf,
    fallback_specs: Vec<FallbackFontSpec>,
    loaded_fallbacks: HashMap<String, Arc<LoadedFont>>,
    chain: Arc<Vec<Arc<LoadedFont>>>,
    lang: String,
}

impl FontSystem {
    pub fn load(asset_root: &Path, manifest: &AppManifest) -> Result<Self> {
        let root = asset_root
            .canonicalize()
            .with_context(|| format!("font root {} does not exist", asset_root.display()))?;
        let mut fonts = HashMap::new();
        let mut data = Vec::new();
        let mut ids = HashSet::new();
        for spec in &manifest.fonts {
            anyhow::ensure!(
                ids.insert(spec.id.clone()),
                "duplicate font id `{}`",
                spec.id
            );
            let paths = font_candidates(&root, &spec.path)?;
            let (font, bytes) = load_first(&paths)?;
            let loaded = Arc::new(LoadedFont::new(font, bytes));
            data.push(loaded.bytes.clone());
            fonts.insert(spec.id.clone(), loaded);
        }
        let default = manifest
            .fonts
            .first()
            .context("at least one font is required")?
            .id
            .clone();
        let mut system = Self {
            fonts,
            data,
            default,
            root,
            fallback_specs: manifest.fallback_fonts.clone(),
            loaded_fallbacks: HashMap::new(),
            chain: Arc::new(Vec::new()),
            lang: String::new(),
        };
        system.set_language("");
        Ok(system)
    }

    /// Raw bytes of every declared font, in manifest order.
    pub fn font_data(&self) -> impl Iterator<Item = &[u8]> {
        self.data.iter().map(|d| d.as_slice())
    }

    /// Activate the fallback fonts for `lang` (loading them on first use).
    /// Returns the bytes of fonts loaded now, so SVG text can use them too.
    pub fn set_language(&mut self, lang: &str) -> Vec<Arc<Vec<u8>>> {
        self.lang = lang.to_owned();
        let mut fresh = Vec::new();
        let mut chain = Vec::new();
        let wanted: Vec<FallbackFontSpec> = self
            .fallback_specs
            .iter()
            .filter(|s| {
                s.langs.is_empty()
                    || s.langs.iter().any(|l| l == "*" || l == lang || lang_prefix(l, lang))
            })
            .cloned()
            .collect();
        // Fonts specific to this language first, then the always-on ones.
        let (mut own, mut any): (Vec<_>, Vec<_>) =
            wanted.into_iter().partition(|s| s.langs.iter().any(|l| l != "*"));
        own.append(&mut any);
        for spec in own {
            if let Some(f) = self.loaded_fallbacks.get(&spec.id) {
                chain.push(f.clone());
                continue;
            }
            let loaded = font_candidates(&self.root, &spec.path)
                .and_then(|paths| load_first(&paths));
            match loaded {
                Ok((font, bytes)) => {
                    let f = Arc::new(LoadedFont::new(font, bytes));
                    fresh.push(f.bytes.clone());
                    self.loaded_fallbacks.insert(spec.id.clone(), f.clone());
                    chain.push(f);
                }
                Err(e) => eprintln!("[velvet-vs3-app] fallback font `{}`: {e:#}", spec.id),
            }
        }
        self.chain = Arc::new(chain);
        fresh
    }

    /// Advance widths at size 100 for every declared font, as
    /// `{font_id: {"a": 55.6, ..., "default": 55.6, "line_height": 117.2, "__font": id}}`,
    /// ready for `str_measure` / `str_wrap` (the `__font` entry lets the VM
    /// ask the host for exact widths).
    pub fn metrics_value(&self) -> Value {
        const EXTRA: &str = "¡¿ÁÉÍÓÚÜÑáéíóúüñÀÈÌÒÙàèìòùÂÊÎÔÛâêîôûÄËÏÖäëïöÇç«»€£°ºª…–—“”‘’·•×÷±";
        let mut ids: Vec<&String> = self.fonts.keys().collect();
        ids.sort();
        Value::map(ids.into_iter().map(|id| {
            let font = &self.fonts[id].font;
            let chars = (32u8..=126).map(char::from).chain(EXTRA.chars());
            let mut entries: Vec<(String, Value)> = chars
                .map(|ch| {
                    let advance = font.metrics(ch, 100.0).advance_width;
                    (ch.to_string(), Value::Float(round2(advance)))
                })
                .collect();
            let default = font.metrics('n', 100.0).advance_width;
            entries.push(("default".into(), Value::Float(round2(default))));
            let line = font
                .horizontal_line_metrics(100.0)
                .map_or(120.0, |m| m.new_line_size);
            entries.push(("line_height".into(), Value::Float(round2(line))));
            entries.push(("__font".into(), velvet_script_vs3::string_val(id.as_str())));
            (id.clone(), Value::map(entries))
        }))
    }

    pub fn named(&self, name: &str) -> FontChain {
        let primary = self
            .fonts
            .get(name)
            .or_else(|| self.fonts.get(&self.default))
            .expect("font manifest validated")
            .clone();
        FontChain {
            primary,
            fallbacks: self.chain.clone(),
        }
    }

    /// A measurer for `str_measure`/`str_wrap` with the current fonts.
    pub fn measurer(&self) -> velvet_script_vs3::i18n::Measurer {
        let fonts = self.fonts.clone();
        let default = self.default.clone();
        let chain = self.chain.clone();
        let cache: RefCell<HashMap<(String, String, u64), f64>> = RefCell::new(HashMap::new());
        Rc::new(move |font: &str, text: &str, size: f64| {
            let key = (font.to_owned(), text.to_owned(), size.to_bits());
            if let Some(w) = cache.borrow().get(&key) {
                return Some(*w);
            }
            let primary = fonts.get(font).or_else(|| fonts.get(&default))?.clone();
            let fc = FontChain {
                primary,
                fallbacks: chain.clone(),
            };
            let w = f64::from(layout::measure(&fc, text, size as f32));
            let mut c = cache.borrow_mut();
            if c.len() > 40_000 {
                c.clear();
            }
            c.insert(key, w);
            Some(w)
        })
    }
}

fn lang_prefix(spec: &str, lang: &str) -> bool {
    // "zh" matches "zh-Hans"; "pt" matches "pt-BR".
    !spec.is_empty() && lang.starts_with(spec) && lang[spec.len()..].starts_with('-')
}

fn font_candidates(root: &Path, spec: &str) -> Result<Vec<PathBuf>> {
    if let Some(name) = spec.strip_prefix("system:") {
        anyhow::ensure!(
            !name.is_empty() && !name.contains(['/', '\\']) && name != ".." && name != ".",
            "system font must be a bare file name: {spec}"
        );
        Ok(system_font_candidates(name))
    } else {
        let path = Path::new(spec);
        anyhow::ensure!(
            !path.is_absolute()
                && !path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_))),
            "font path escapes asset root: {spec}"
        );
        Ok(vec![root.join(path)])
    }
}
fn load_first(paths: &[PathBuf]) -> Result<(Font, Vec<u8>)> {
    let mut errors = Vec::new();
    for path in paths {
        match fs::read(path) {
            Ok(bytes) => {
                return Font::from_bytes(bytes.as_slice(), FontSettings::default())
                    .map(|font| (font, bytes))
                    .map_err(|e| anyhow::anyhow!("invalid font {}: {e}", path.display()))
            }
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    let shown = errors.len().min(3);
    anyhow::bail!(
        "could not load font ({} paths tried): {}",
        errors.len(),
        errors[..shown].join(" | ")
    )
}

/// Where `system:<file>` fonts are looked up, most specific first, ending
/// with a common sans-serif fallback for each platform.
fn system_font_candidates(name: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windows) = std::env::var_os("WINDIR").map(PathBuf::from) {
        dirs.push(windows.join("Fonts"));
    } else if cfg!(windows) {
        dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
        dirs.push(home.join("Library/Fonts"));
    }
    for dir in [
        "/Library/Fonts",
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/usr/share/fonts/truetype/msttcorefonts",
        "/usr/share/fonts/truetype/dejavu",
        "/usr/share/fonts/truetype/liberation",
        "/usr/share/fonts/TTF",
        "/usr/share/fonts/dejavu",
        "/usr/share/fonts/liberation",
        "/usr/local/share/fonts",
    ] {
        dirs.push(PathBuf::from(dir));
    }
    let mut out: Vec<PathBuf> = dirs.iter().map(|dir| dir.join(name)).collect();
    for fallback in [
        "arial.ttf",
        "Arial.ttf",
        "LiberationSans-Regular.ttf",
        "DejaVuSans.ttf",
        "Helvetica.ttc",
    ] {
        out.extend(dirs.iter().map(|dir| dir.join(fallback)));
    }
    out
}

#[cfg(test)]
pub(crate) fn test_chain(bytes: Vec<u8>) -> Option<FontChain> {
    let font = Font::from_bytes(bytes.as_slice(), FontSettings::default()).ok()?;
    Some(FontChain {
        primary: Arc::new(LoadedFont::new(font, bytes)),
        fallbacks: Arc::new(Vec::new()),
    })
}
