use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result};

use super::{AppManifest, ImageAsset};

struct AssetEntry {
    path: PathBuf,
    image: OnceLock<Option<ImageAsset>>,
}

pub struct GameAssets {
    entries: HashMap<String, AssetEntry>,
    max_image_pixels: usize,
}

impl GameAssets {
    pub fn load(root: &Path, manifest: &AppManifest) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("asset root {} does not exist", root.display()))?;
        let mut entries = HashMap::with_capacity(manifest.images.len());
        let mut ids = HashSet::new();
        for spec in &manifest.images {
            anyhow::ensure!(!spec.id.is_empty(), "asset id cannot be empty");
            anyhow::ensure!(
                ids.insert(spec.id.clone()),
                "duplicate asset id `{}`",
                spec.id
            );
            let path = safe_join(&root, &spec.path)?;
            entries.insert(
                spec.id.clone(),
                AssetEntry {
                    path,
                    image: OnceLock::new(),
                },
            );
        }
        let assets = Self {
            entries,
            max_image_pixels: manifest.limits.max_image_pixels,
        };
        for spec in &manifest.images {
            anyhow::ensure!(
                matches!(spec.mode.as_str(), "preload" | "lazy"),
                "asset `{}` has invalid mode `{}`",
                spec.id,
                spec.mode
            );
            if spec.mode == "preload" {
                assets.load_required(&spec.id)?;
            }
        }
        Ok(assets)
    }

    pub fn named(&self, name: &str) -> Option<&ImageAsset> {
        let entry = self.entries.get(name)?;
        entry
            .image
            .get_or_init(
                || match ImageAsset::from_path(&entry.path, self.max_image_pixels) {
                    Ok(image) => Some(image),
                    Err(error) => {
                        eprintln!("[velvet-vs3-app/assets] {name}: {error:#}");
                        None
                    }
                },
            )
            .as_ref()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn validate_all(&self) -> Result<()> {
        for name in self.entries.keys() {
            self.load_required(name)?;
        }
        Ok(())
    }

    fn load_required(&self, name: &str) -> Result<()> {
        let entry = self
            .entries
            .get(name)
            .with_context(|| format!("unknown asset `{name}`"))?;
        if entry.image.get().is_none() {
            let image = ImageAsset::from_path(&entry.path, self.max_image_pixels)
                .with_context(|| format!("asset `{name}`"))?;
            let _ = entry.image.set(Some(image));
        }
        anyhow::ensure!(
            entry.image.get().and_then(Option::as_ref).is_some(),
            "asset `{name}` failed to load"
        );
        Ok(())
    }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let requested = Path::new(relative);
    anyhow::ensure!(
        !requested.is_absolute(),
        "absolute asset paths are forbidden: {relative}"
    );
    anyhow::ensure!(
        !requested
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_))),
        "asset path escapes root: {relative}"
    );
    let joined = root.join(requested);
    let parent = joined
        .parent()
        .unwrap_or(root)
        .canonicalize()
        .with_context(|| format!("asset parent is missing for {relative}"))?;
    anyhow::ensure!(
        parent.starts_with(root),
        "asset path escapes root: {relative}"
    );
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_traversal_is_rejected() {
        let root = std::env::temp_dir();
        assert!(safe_join(&root, "../secret.png").is_err());
        assert!(safe_join(&root, r"C:\\secret.png").is_err());
    }
}
