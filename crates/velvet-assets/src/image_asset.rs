//! Image / sprite / atlas assets for product presentation.
//!
//! Loaders decode PNG/JPEG into RGBA8 and parse simple atlas JSON. Hosts upload
//! pixels to `velvet-render` textures; this crate stays GPU-free.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::loader::{AssetLoader, LoadError};
use crate::path::{AssetPath, VirtualPath};

/// Decoded raster image (RGBA8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAsset {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA8.
    pub rgba: Vec<u8>,
    /// Logical role hint: `background`, `sprite`, `ui`, …
    pub role: ImageRole,
}

/// Semantic role for presentation lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImageRole {
    /// Full-screen / stage background.
    Background,
    /// Character or prop sprite.
    Sprite,
    /// UI chrome.
    Ui,
    /// Unknown / generic.
    #[default]
    Generic,
}

impl ImageAsset {
    /// Create from dimensions + RGBA buffer.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>, role: ImageRole) -> Result<Self, LoadError> {
        let expect = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        if rgba.len() != expect {
            return Err(LoadError::Decode(format!(
                "rgba len {} != {}x{}x4",
                rgba.len(),
                width,
                height
            )));
        }
        Ok(Self {
            width,
            height,
            rgba,
            role,
        })
    }

    /// Solid color utility image (tests / placeholders).
    pub fn solid(width: u32, height: u32, rgba: [u8; 4], role: ImageRole) -> Self {
        let n = (width * height) as usize;
        let mut buf = Vec::with_capacity(n * 4);
        for _ in 0..n {
            buf.extend_from_slice(&rgba);
        }
        Self {
            width,
            height,
            rgba: buf,
            role,
        }
    }

    /// Byte length of pixel buffer.
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }
}

/// One named frame inside a sprite atlas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasFrame {
    /// Frame name (e.g. `hero_idle`, `nora_happy`).
    pub name: String,
    /// Left in atlas pixels.
    pub x: u32,
    /// Top.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

impl AtlasFrame {
    /// Create a frame.
    pub fn new(name: impl Into<String>, x: u32, y: u32, w: u32, h: u32) -> Self {
        Self {
            name: name.into(),
            x,
            y,
            w,
            h,
        }
    }
}

/// Sprite sheet / texture atlas metadata (+ optional embedded sheet pixels).
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteAtlasAsset {
    /// Path or id of the backing image (logical).
    pub image_path: String,
    /// Atlas pixel size.
    pub width: u32,
    /// Atlas height.
    pub height: u32,
    /// Named frames.
    pub frames: HashMap<String, AtlasFrame>,
    /// Optional decoded sheet (when loaded together with image bytes).
    pub image: Option<ImageAsset>,
}

impl SpriteAtlasAsset {
    /// Empty atlas metadata.
    pub fn new(image_path: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            image_path: image_path.into(),
            width,
            height,
            frames: HashMap::new(),
            image: None,
        }
    }

    /// Insert a frame.
    pub fn insert_frame(&mut self, frame: AtlasFrame) {
        self.frames.insert(frame.name.clone(), frame);
    }

    /// Lookup frame by name.
    pub fn get(&self, name: &str) -> Option<&AtlasFrame> {
        self.frames.get(name)
    }

    /// Whether a frame exists.
    pub fn contains(&self, name: &str) -> bool {
        self.frames.contains_key(name)
    }

    /// Frame names sorted.
    pub fn frame_names(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.frames.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }
}

/// JSON atlas format (simple product schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasJson {
    /// Backing image path relative to atlas file / virtual root.
    #[serde(default)]
    pub image: String,
    /// Width.
    #[serde(default)]
    pub width: u32,
    /// Height.
    #[serde(default)]
    pub height: u32,
    /// Frames list or map.
    #[serde(default)]
    pub frames: Vec<AtlasFrameJson>,
}

/// One frame in JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasFrameJson {
    /// Name.
    pub name: String,
    /// x
    pub x: u32,
    /// y
    pub y: u32,
    /// w
    pub w: u32,
    /// h
    pub h: u32,
}

impl SpriteAtlasAsset {
    /// Parse atlas JSON bytes.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        let doc: AtlasJson = serde_json::from_slice(bytes)
            .map_err(|e| LoadError::Decode(format!("atlas json: {e}")))?;
        let mut atlas = Self::new(doc.image, doc.width, doc.height);
        for f in doc.frames {
            atlas.insert_frame(AtlasFrame::new(f.name, f.x, f.y, f.w, f.h));
        }
        Ok(atlas)
    }
}

/// Decode image bytes (PNG/JPEG) via `image` crate.
pub fn decode_image_bytes(bytes: &[u8], role: ImageRole) -> Result<ImageAsset, LoadError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| LoadError::Decode(format!("image: {e}")))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    ImageAsset::new(w, h, img.into_raw(), role)
}

/// Infer role from path segments.
pub fn role_from_path(path: &str) -> ImageRole {
    let p = path.replace('\\', "/").to_ascii_lowercase();
    if p.contains("/bg/")
        || p.contains("/background")
        || p.contains("bg_")
        || p.contains("menu_bg")
        || p.starts_with("bg/")
    {
        ImageRole::Background
    } else if p.contains("sprite")
        || p.contains("/char")
        || p.contains("stand_")
        || p.contains("/actors/")
        || p.contains("portrait")
    {
        ImageRole::Sprite
    } else if p.contains("/ui/") || p.contains("button") || p.contains("hud") {
        ImageRole::Ui
    } else {
        ImageRole::Generic
    }
}

/// PNG/JPEG → [`ImageAsset`] loader.
#[derive(Default)]
pub struct ImageLoader;

impl AssetLoader for ImageLoader {
    fn type_name(&self) -> &'static str {
        "ImageAsset"
    }
    fn value_type(&self) -> TypeId {
        TypeId::of::<ImageAsset>()
    }
    fn extensions(&self) -> &[&'static str] {
        &["png", "jpg", "jpeg", "webp", "bmp"]
    }
    fn load(
        &self,
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Box<dyn Any + Send + Sync>, LoadError> {
        let role = role_from_path(&path.to_string());
        let img = decode_image_bytes(bytes, role)?;
        Ok(Box::new(img))
    }
}

/// `.atlas.json` / `.atlas` → [`SpriteAtlasAsset`] loader.
#[derive(Default)]
pub struct AtlasLoader;

impl AssetLoader for AtlasLoader {
    fn type_name(&self) -> &'static str {
        "SpriteAtlasAsset"
    }
    fn value_type(&self) -> TypeId {
        TypeId::of::<SpriteAtlasAsset>()
    }
    fn extensions(&self) -> &[&'static str] {
        &["atlas", "atlas.json"]
    }
    fn load(
        &self,
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Box<dyn Any + Send + Sync>, LoadError> {
        let mut atlas = SpriteAtlasAsset::from_json_bytes(bytes)
            .map_err(|e| LoadError::Decode(format!("{path}: {e}")))?;
        if atlas.image_path.is_empty() {
            // default: same stem .png
            let s = path.to_string();
            atlas.image_path = s
                .trim_end_matches(".atlas.json")
                .trim_end_matches(".atlas")
                .to_string()
                + ".png";
        }
        Ok(Box::new(atlas))
    }
}

/// Product-facing catalog: backgrounds, sprites, atlases by logical key.
#[derive(Debug, Default, Clone)]
pub struct PresentAssetCatalog {
    /// Backgrounds keyed by logical id / path.
    backgrounds: HashMap<String, ImageAsset>,
    /// Full sprites keyed by id.
    sprites: HashMap<String, ImageAsset>,
    /// Atlases keyed by id.
    atlases: HashMap<String, SpriteAtlasAsset>,
}

impl PresentAssetCatalog {
    /// Empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a background image.
    pub fn insert_background(&mut self, key: impl Into<String>, image: ImageAsset) {
        let mut img = image;
        img.role = ImageRole::Background;
        self.backgrounds.insert(key.into(), img);
    }

    /// Insert a sprite image.
    pub fn insert_sprite(&mut self, key: impl Into<String>, image: ImageAsset) {
        let mut img = image;
        img.role = ImageRole::Sprite;
        self.sprites.insert(key.into(), img);
    }

    /// Insert an atlas.
    pub fn insert_atlas(&mut self, key: impl Into<String>, atlas: SpriteAtlasAsset) {
        self.atlases.insert(key.into(), atlas);
    }

    /// Lookup background by key (exact, then suffix match).
    pub fn background(&self, key: &str) -> Option<&ImageAsset> {
        if let Some(i) = self.backgrounds.get(key) {
            return Some(i);
        }
        self.backgrounds
            .iter()
            .find(|(k, _)| k.ends_with(key) || key.ends_with(k.as_str()))
            .map(|(_, v)| v)
    }

    /// Lookup sprite by key.
    pub fn sprite(&self, key: &str) -> Option<&ImageAsset> {
        if let Some(i) = self.sprites.get(key) {
            return Some(i);
        }
        self.sprites
            .iter()
            .find(|(k, _)| k == &key || k.ends_with(key) || key.ends_with(k.as_str()))
            .map(|(_, v)| v)
    }

    /// Lookup atlas by key.
    pub fn atlas(&self, key: &str) -> Option<&SpriteAtlasAsset> {
        self.atlases.get(key)
    }

    /// Lookup a frame across all atlases: `atlas_id/frame` or bare frame name.
    pub fn atlas_frame(&self, key: &str) -> Option<(&SpriteAtlasAsset, &AtlasFrame)> {
        if let Some((aid, fname)) = key.split_once('/') {
            if let Some(a) = self.atlases.get(aid) {
                if let Some(f) = a.get(fname) {
                    return Some((a, f));
                }
            }
        }
        for a in self.atlases.values() {
            if let Some(f) = a.get(key) {
                return Some((a, f));
            }
        }
        None
    }

    /// Background count.
    pub fn background_count(&self) -> usize {
        self.backgrounds.len()
    }

    /// Sprite count.
    pub fn sprite_count(&self) -> usize {
        self.sprites.len()
    }

    /// Atlas count.
    pub fn atlas_count(&self) -> usize {
        self.atlases.len()
    }

    /// Register defaults: tiny solid placeholders for missing keys (tests).
    pub fn with_placeholders() -> Self {
        let mut c = Self::new();
        c.insert_background(
            "bg/default",
            ImageAsset::solid(8, 8, [20, 18, 32, 255], ImageRole::Background),
        );
        c.insert_sprite(
            "sprite/default",
            ImageAsset::solid(4, 8, [180, 80, 120, 255], ImageRole::Sprite),
        );
        let mut atlas = SpriteAtlasAsset::new("sprites.png", 64, 64);
        atlas.insert_frame(AtlasFrame::new("hero_idle", 0, 0, 32, 64));
        atlas.insert_frame(AtlasFrame::new("hero_happy", 32, 0, 32, 64));
        c.insert_atlas("chars", atlas);
        c
    }
}

/// Register image + atlas loaders into a registry.
pub fn register_image_loaders(reg: &mut crate::loader::LoaderRegistry) {
    reg.register(ImageLoader);
    reg.register(AtlasLoader);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::LoaderRegistry;
    use crate::path::AssetPath;
    use crate::source::MemorySource;
    use crate::Assets;
    use std::sync::Arc;

    fn tiny_png_1x1() -> Vec<u8> {
        // Minimal valid 1×1 PNG (red pixel) — use image encode
        let mut img = image::RgbaImage::new(2, 2);
        for p in img.pixels_mut() {
            *p = image::Rgba([10, 20, 200, 255]);
        }
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut buf),
                image::ImageFormat::Png,
            )
            .expect("encode png");
        buf
    }

    #[test]
    fn image_loader_decodes_png() {
        let bytes = tiny_png_1x1();
        let loader = ImageLoader;
        let path = AssetPath::virtual_path(VirtualPath::new("bg/station.png"));
        let any = loader.load(&path, &bytes).unwrap();
        let img = any.downcast_ref::<ImageAsset>().unwrap();
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 2);
        assert_eq!(img.role, ImageRole::Background);
        assert_eq!(img.rgba.len(), 2 * 2 * 4);
    }

    #[test]
    fn atlas_json_frames_lookup() {
        let json = r#"{
            "image": "chars.png",
            "width": 64,
            "height": 64,
            "frames": [
                {"name": "hero_idle", "x": 0, "y": 0, "w": 32, "h": 64},
                {"name": "hero_happy", "x": 32, "y": 0, "w": 32, "h": 64}
            ]
        }"#;
        let atlas = SpriteAtlasAsset::from_json_bytes(json.as_bytes()).unwrap();
        assert!(atlas.contains("hero_idle"));
        assert_eq!(atlas.get("hero_happy").unwrap().x, 32);
        assert_eq!(atlas.frame_names().len(), 2);
    }

    #[test]
    fn present_catalog_bg_sprite_atlas_lookup() {
        let mut cat = PresentAssetCatalog::with_placeholders();
        cat.insert_background(
            "bg/station.png",
            ImageAsset::solid(16, 9, [30, 40, 80, 255], ImageRole::Background),
        );
        cat.insert_sprite(
            "nora",
            ImageAsset::solid(8, 16, [200, 100, 120, 255], ImageRole::Sprite),
        );

        assert!(cat.background("bg/station.png").is_some());
        assert!(cat.background("station.png").is_some());
        assert_eq!(cat.sprite("nora").unwrap().width, 8);
        let (a, f) = cat.atlas_frame("chars/hero_idle").unwrap();
        assert_eq!(a.width, 64);
        assert_eq!(f.w, 32);
        assert!(cat.atlas_frame("hero_happy").is_some());
        assert_eq!(cat.background_count(), 2); // default + station
        assert_eq!(cat.sprite_count(), 2);
        assert_eq!(cat.atlas_count(), 1);
    }

    #[test]
    fn assets_registry_loads_image_via_memory_source() {
        let mut mem = MemorySource::new();
        mem.insert("bg/rain.png", tiny_png_1x1());
        let mut assets = Assets::with_source(Arc::new(mem));
        register_image_loaders(assets.loaders_mut());
        let path = AssetPath::virtual_path(VirtualPath::new("bg/rain.png"));
        let handle = assets.load::<ImageAsset>(path);
        let img = assets.get(handle).expect("get image after load");
        assert_eq!(img.role, ImageRole::Background);
        assert!(img.width >= 1);
    }

    #[test]
    fn role_from_path_heuristics() {
        assert_eq!(role_from_path("bg/station.png"), ImageRole::Background);
        assert_eq!(role_from_path("data/ui/menu_bg.jpg"), ImageRole::Background);
        assert_eq!(role_from_path("sprites/hero.png"), ImageRole::Sprite);
        assert_eq!(role_from_path("ui/button.png"), ImageRole::Ui);
    }

    #[test]
    fn loader_registry_registers_image_and_atlas() {
        let mut reg = LoaderRegistry::with_defaults();
        let before = reg.type_count();
        register_image_loaders(&mut reg);
        assert!(reg.type_count() >= before + 2);
        assert!(reg.for_extension("png").is_some());
        assert!(reg.for_type::<ImageAsset>().is_some());
        assert!(reg.for_type::<SpriteAtlasAsset>().is_some());
    }
}
