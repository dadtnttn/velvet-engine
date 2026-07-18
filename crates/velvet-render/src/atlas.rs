//! Named texture atlas regions for sprite sheets and UI.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

use crate::texture::{TextureId, TextureRegion};

/// Simple texture atlas: named regions on one texture.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextureAtlas {
    /// Backing texture.
    pub texture: TextureId,
    /// Texture pixel size.
    pub size: Vec2,
    /// Named frames.
    frames: HashMap<String, TextureRegion>,
}

impl TextureAtlas {
    /// Create empty atlas.
    pub fn new(texture: TextureId, width: f32, height: f32) -> Self {
        Self {
            texture,
            size: Vec2::new(width, height),
            frames: HashMap::new(),
        }
    }

    /// Insert a named region (alias of [`Self::add_region`]).
    pub fn insert(&mut self, name: impl Into<String>, region: TextureRegion) {
        self.add_region(name, region);
    }

    /// Add or replace a named region.
    pub fn add_region(&mut self, name: impl Into<String>, region: TextureRegion) {
        self.frames.insert(name.into(), region);
    }

    /// Lookup region by name.
    pub fn get(&self, name: &str) -> Option<TextureRegion> {
        self.frames.get(name).copied()
    }

    /// Whether a region name exists.
    pub fn contains(&self, name: &str) -> bool {
        self.frames.contains_key(name)
    }

    /// Remove a region; returns previous value.
    pub fn remove(&mut self, name: &str) -> Option<TextureRegion> {
        self.frames.remove(name)
    }

    /// UV for named frame.
    pub fn uv(&self, name: &str) -> Option<Rect> {
        self.get(name).map(|r| r.to_uv(self.size.x, self.size.y))
    }

    /// Frame count.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Sorted region names.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.frames.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Iterate name/region pairs (unordered).
    pub fn iter(&self) -> impl Iterator<Item = (&str, TextureRegion)> + '_ {
        self.frames.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Fill atlas with a regular grid of regions named `{prefix}{i}`.
    pub fn add_grid(&mut self, prefix: &str, frame_w: f32, frame_h: f32, count: usize) {
        let cols = (self.size.x / frame_w.max(1.0)).floor().max(1.0) as usize;
        for i in 0..count {
            let col = i % cols;
            let row = i / cols;
            self.add_region(
                format!("{prefix}{i}"),
                TextureRegion {
                    x: col as f32 * frame_w,
                    y: row as f32 * frame_h,
                    width: frame_w,
                    height: frame_h,
                },
            );
        }
    }

    /// Add a grid from JSON-like list of (name, x, y, w, h).
    pub fn add_regions<I, S>(&mut self, regions: I)
    where
        I: IntoIterator<Item = (S, TextureRegion)>,
        S: Into<String>,
    {
        for (name, region) in regions {
            self.add_region(name, region);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_to_uv() {
        let mut atlas = TextureAtlas::new(TextureId::allocate(), 64.0, 16.0);
        atlas.add_region(
            "a",
            TextureRegion {
                x: 16.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
            },
        );
        let uv = atlas.uv("a").unwrap();
        assert!((uv.min.x - 0.25).abs() < 1e-5);
        assert!((uv.max.x - 0.5).abs() < 1e-5);
    }

    #[test]
    fn atlas_lookup_and_names() {
        let mut atlas = TextureAtlas::new(TextureId::allocate(), 32.0, 32.0);
        atlas.insert("idle", TextureRegion::full(16.0, 16.0));
        atlas.add_region(
            "walk",
            TextureRegion {
                x: 16.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
            },
        );
        assert!(atlas.contains("idle"));
        assert!(atlas.get("walk").is_some());
        assert_eq!(atlas.len(), 2);
        assert_eq!(atlas.names(), vec!["idle", "walk"]);
    }

    #[test]
    fn grid_fill() {
        let mut atlas = TextureAtlas::new(TextureId::allocate(), 32.0, 16.0);
        atlas.add_grid("f", 16.0, 16.0, 2);
        assert_eq!(atlas.len(), 2);
        assert!(atlas.get("f0").is_some());
        assert!(atlas.get("f1").is_some());
    }
}
