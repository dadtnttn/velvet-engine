use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use fontdue::{Font, FontSettings};

use crate::assets::AppManifest;

pub struct FontSystem {
    fonts: HashMap<String, Font>,
    default: String,
}

impl FontSystem {
    pub fn load(asset_root: &Path, manifest: &AppManifest) -> Result<Self> {
        let root = asset_root
            .canonicalize()
            .with_context(|| format!("font root {} does not exist", asset_root.display()))?;
        let mut fonts = HashMap::new();
        let mut ids = HashSet::new();
        for spec in &manifest.fonts {
            anyhow::ensure!(
                ids.insert(spec.id.clone()),
                "duplicate font id `{}`",
                spec.id
            );
            let paths = font_candidates(&root, &spec.path)?;
            fonts.insert(spec.id.clone(), load_first(&paths)?);
        }
        let default = manifest
            .fonts
            .first()
            .context("at least one font is required")?
            .id
            .clone();
        Ok(Self { fonts, default })
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
        let windows = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        Ok(vec![
            windows.join("Fonts").join(name),
            windows.join("Fonts").join("arial.ttf"),
        ])
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
fn load_first(paths: &[PathBuf]) -> Result<Font> {
    let mut errors = Vec::new();
    for path in paths {
        match fs::read(path) {
            Ok(bytes) => {
                return Font::from_bytes(bytes, FontSettings::default())
                    .map_err(|e| anyhow::anyhow!("invalid font {}: {e}", path.display()))
            }
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    anyhow::bail!("could not load font: {}", errors.join(" | "))
}
