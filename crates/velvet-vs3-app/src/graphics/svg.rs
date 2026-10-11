//! Inline SVG rasterization for the `svg` primitive.
//!
//! Packages send SVG markup as text; the host rasterizes it with `resvg` at the
//! command's `w`×`h`, caches the result by content and size, and blends it like
//! any other image. External references are refused: `<image href="file">`
//! never touches the filesystem and text only sees the manifest's fonts.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use resvg::{tiny_skia, usvg};

use crate::assets::ImageAsset;

/// Largest side accepted for one rasterized SVG.
pub const MAX_SVG_SIDE: u32 = 4096;
/// Largest SVG document accepted in a command, in bytes.
pub const MAX_SVG_BYTES: usize = 256 * 1024;
/// Pixel budget kept in the raster cache before least-recently-used eviction.
const CACHE_PIXEL_BUDGET: usize = 24_000_000;

struct Entry {
    image: Option<ImageAsset>,
    last_used: u64,
}

/// Rasterizes and caches inline SVG documents.
pub struct SvgCache {
    fontdb: Arc<usvg::fontdb::Database>,
    entries: HashMap<u64, Entry>,
    pixels: usize,
    clock: u64,
}

impl SvgCache {
    /// Make an extra font (e.g. a fallback for the active language) visible
    /// to SVG text.
    pub fn add_font(&mut self, data: &[u8]) {
        std::sync::Arc::make_mut(&mut self.fontdb).load_font_data(data.to_vec());
    }
    /// Forget every cached raster (fonts changed).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pixels = 0;
    }
    /// Builds a cache whose text rendering only sees the given font files.
    pub fn new<'a>(fonts: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut db = usvg::fontdb::Database::new();
        for data in fonts {
            db.load_font_data(data.to_vec());
        }
        let family = db
            .faces()
            .next()
            .and_then(|face| face.families.first().map(|(name, _)| name.clone()));
        if let Some(family) = family {
            db.set_sans_serif_family(family.clone());
            db.set_serif_family(family.clone());
            db.set_monospace_family(family.clone());
            db.set_cursive_family(family.clone());
            db.set_fantasy_family(family);
        }
        Self {
            fontdb: Arc::new(db),
            entries: HashMap::new(),
            pixels: 0,
            clock: 0,
        }
    }

    /// Returns the raster for `svg` at `width`×`height`, or `None` if it is invalid.
    pub fn raster(&mut self, svg: &str, width: u32, height: u32) -> Option<&ImageAsset> {
        if width == 0 || height == 0 || width > MAX_SVG_SIDE || height > MAX_SVG_SIDE {
            return None;
        }
        self.clock += 1;
        let key = cache_key(svg, width, height);
        if !self.entries.contains_key(&key) {
            let image = rasterize(svg, width, height, &self.fontdb);
            if let Some(image) = &image {
                self.pixels += image.pixels.len();
            }
            self.entries.insert(
                key,
                Entry {
                    image,
                    last_used: self.clock,
                },
            );
            self.evict(key);
        }
        let entry = self.entries.get_mut(&key)?;
        entry.last_used = self.clock;
        entry.image.as_ref()
    }

    /// Number of cached rasters (including failed parses).
    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn evict(&mut self, keep: u64) {
        while self.pixels > CACHE_PIXEL_BUDGET {
            let oldest = self
                .entries
                .iter()
                .filter(|(key, _)| **key != keep)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| *key);
            let Some(oldest) = oldest else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                if let Some(image) = entry.image {
                    self.pixels -= image.pixels.len();
                }
            }
        }
    }
}

fn cache_key(svg: &str, width: u32, height: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    svg.hash(&mut hasher);
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    hasher.finish()
}

fn rasterize(
    svg: &str,
    width: u32,
    height: u32,
    fontdb: &Arc<usvg::fontdb::Database>,
) -> Option<ImageAsset> {
    let options = usvg::Options {
        fontdb: fontdb.clone(),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = match usvg::Tree::from_str(svg, &options) {
        Ok(tree) => tree,
        Err(error) => {
            eprintln!("[velvet-vs3-app] invalid svg: {error}");
            return None;
        }
    };
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let pixels = pixmap
        .pixels()
        .iter()
        .map(|pixel| {
            let color = pixel.demultiply();
            ((color.alpha() as u32) << 24)
                | ((color.red() as u32) << 16)
                | ((color.green() as u32) << 8)
                | color.blue() as u32
        })
        .collect();
    Some(ImageAsset {
        width: width as usize,
        height: height as usize,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED_SQUARE: &str = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'>\
        <rect width='10' height='10' fill='#ff0000'/></svg>";

    #[test]
    fn rasterizes_to_requested_size() {
        let mut cache = SvgCache::new(std::iter::empty());
        let image = cache.raster(RED_SQUARE, 20, 8).expect("valid svg");
        assert_eq!((image.width, image.height), (20, 8));
        assert_eq!(image.pixels[0], 0xffff0000);
        assert_eq!(image.pixels[20 * 8 - 1], 0xffff0000);
    }

    #[test]
    fn caches_by_content_and_size() {
        let mut cache = SvgCache::new(std::iter::empty());
        cache.raster(RED_SQUARE, 10, 10);
        cache.raster(RED_SQUARE, 10, 10);
        assert_eq!(cache.len(), 1);
        cache.raster(RED_SQUARE, 12, 10);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn rejects_invalid_markup_and_sizes() {
        let mut cache = SvgCache::new(std::iter::empty());
        assert!(cache.raster("<not svg", 10, 10).is_none());
        assert!(cache.raster(RED_SQUARE, 0, 10).is_none());
        assert!(cache.raster(RED_SQUARE, MAX_SVG_SIDE + 1, 10).is_none());
    }

    #[test]
    fn external_image_references_are_ignored() {
        let mut cache = SvgCache::new(std::iter::empty());
        let svg =
            "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' \
            viewBox='0 0 10 10'><image width='10' height='10' xlink:href='/etc/passwd'/></svg>";
        let image = cache.raster(svg, 10, 10).expect("document still parses");
        assert!(image.pixels.iter().all(|pixel| pixel >> 24 == 0));
    }

    #[test]
    fn evicts_least_recently_used_over_budget() {
        let mut cache = SvgCache::new(std::iter::empty());
        cache.raster(RED_SQUARE, 4000, 4000);
        cache.raster(RED_SQUARE, 4000, 3000);
        assert!(cache.pixels <= CACHE_PIXEL_BUDGET);
        assert!(cache.raster(RED_SQUARE, 4000, 3000).is_some());
    }
}
