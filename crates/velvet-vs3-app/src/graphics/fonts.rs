use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use fontdue::{Font, FontSettings};

use velvet_script_vs3::Value;

use crate::assets::AppManifest;

fn round2(value: f32) -> f64 {
    (f64::from(value) * 100.0).round() / 100.0
}

pub struct FontSystem {
    fonts: HashMap<String, Font>,
    data: Vec<Vec<u8>>,
    default: String,
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
            fonts.insert(spec.id.clone(), font);
            data.push(bytes);
        }
        let default = manifest
            .fonts
            .first()
            .context("at least one font is required")?
            .id
            .clone();
        Ok(Self {
            fonts,
            data,
            default,
        })
    }
    /// Raw bytes of every declared font, in manifest order.
    pub fn font_data(&self) -> impl Iterator<Item = &[u8]> {
        self.data.iter().map(Vec::as_slice)
    }
    /// Advance widths at size 100 for every declared font, as
    /// `{font_id: {"a": 55.6, ..., "default": 55.6, "line_height": 117.2}}`,
    /// ready for `str_measure` / `str_wrap`.
    pub fn metrics_value(&self) -> Value {
        const EXTRA: &str = "¡¿ÁÉÍÓÚÜÑáéíóúüñÀÈÌÒÙàèìòùÂÊÎÔÛâêîôûÄËÏÖäëïöÇç«»€£°ºª…–—“”‘’·•×÷±";
        let mut ids: Vec<&String> = self.fonts.keys().collect();
        ids.sort();
        Value::map(ids.into_iter().map(|id| {
            let font = &self.fonts[id];
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
            (id.clone(), Value::map(entries))
        }))
    }
    pub fn named(&self, name: &str) -> &Font {
        self.fonts
            .get(name)
            .or_else(|| self.fonts.get(&self.default))
            .expect("font manifest validated")
    }
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
