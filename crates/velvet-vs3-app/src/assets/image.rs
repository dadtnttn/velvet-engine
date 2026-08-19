use std::path::Path;

use anyhow::{Context, Result};
use image::GenericImageView;

#[derive(Debug, Clone)]
pub struct ImageAsset {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl ImageAsset {
    pub(crate) fn from_path(path: &Path, max_pixels: usize) -> Result<Self> {
        let image =
            image::open(path).with_context(|| format!("could not load {}", path.display()))?;
        let (width, height) = image.dimensions();
        let pixel_count = width as usize * height as usize;
        anyhow::ensure!(
            pixel_count <= max_pixels,
            "image {} has {pixel_count} pixels, limit is {max_pixels}",
            path.display()
        );
        let pixels = image
            .to_rgba8()
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
