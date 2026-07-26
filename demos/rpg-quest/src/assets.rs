use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::GenericImageView;

#[derive(Debug, Clone)]
pub struct ImageAsset {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl ImageAsset {
    fn from_path(path: &Path) -> Result<Self> {
        let image =
            image::open(path).with_context(|| format!("no se pudo cargar {}", path.display()))?;
        let (width, height) = image.dimensions();
        let rgba = image.to_rgba8();
        let pixels = rgba
            .pixels()
            .map(|pixel| {
                let [r, g, b, a] = pixel.0;
                ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
            })
            .collect();
        Ok(Self {
            width: width as usize,
            height: height as usize,
            pixels,
        })
    }
}

pub struct AssetStore {
    root: PathBuf,
    cache: HashMap<String, ImageAsset>,
    missing: HashSet<String>,
}

impl AssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: HashMap::new(),
            missing: HashSet::new(),
        }
    }

    pub fn get(&mut self, relative: &str) -> Option<&ImageAsset> {
        if self.missing.contains(relative) {
            return None;
        }
        if !self.cache.contains_key(relative) {
            let path = self.root.join(relative);
            match ImageAsset::from_path(&path) {
                Ok(image) => {
                    self.cache.insert(relative.to_owned(), image);
                }
                Err(error) => {
                    eprintln!("[rpg-assets] {error:#}");
                    self.missing.insert(relative.to_owned());
                    return None;
                }
            }
        }
        self.cache.get(relative)
    }

    pub fn get_cloned(&mut self, relative: &str) -> Option<ImageAsset> {
        self.get(relative).cloned()
    }
}

pub fn locate_asset_root() -> PathBuf {
    // Artwork is intentionally local-only. Users can place their own licensed pack in
    // any candidate directory; the renderer keeps working with procedural fallbacks.
    [
        PathBuf::from("demos/rpg-quest/assets"),
        PathBuf::from("assets"),
        PathBuf::from("../rpg-quest/assets"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .unwrap_or_else(|| PathBuf::from("demos/rpg-quest/assets"))
}
