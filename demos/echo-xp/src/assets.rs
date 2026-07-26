use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use image::DynamicImage;

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>, // ARGB / 0xAARRGGBB software pixel buffer format
}

impl Texture {
    pub fn from_image(img: &DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut data = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let pixel = rgba.get_pixel(x, y);
                let a = pixel[3] as u32;
                let r = pixel[0] as u32;
                let g = pixel[1] as u32;
                let b = pixel[2] as u32;
                let argb = (a << 24) | (r << 16) | (g << 8) | b;
                data.push(argb);
            }
        }

        Self {
            width,
            height,
            data,
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize]
        } else {
            0
        }
    }
}

pub struct Assets {
    pub base_dir: PathBuf,
    pub textures: HashMap<String, Texture>,
}

impl Assets {
    pub fn load() -> Result<Self> {
        // Media packs are intentionally not versioned with this demo. Add compatible
        // files here from your own licensed collection; missing files use UI fallbacks.
        let base_dir = PathBuf::from("demos/echo-xp/data/assets");
        let mut textures = HashMap::new();

        let files_to_load = [
            ("bliss", "wallpapers/bliss.png"),
            ("boot_screen", "ui/boot_screen.png"),
            ("windows_logo_small", "ui/windows_logo_small.png"),
            ("xp_theme", "ui/xp_theme.png"),
            ("win7_wallpaper", "win7_wallpaper.png"),
            ("win7_start_orb", "win7_start_orb.png"),
            ("win7_glass_theme", "win7_glass_theme.png"),
            ("winicons_16", "icons/winicons_16.png"),
            ("winicons_32", "icons/winicons_32.png"),
            ("winicons_48", "winicons_48.png"),
            ("win7_cursor_arrow", "win7_cursor_arrow.png"),
            ("cursor_arrow", "cursors/arrow.png"),
            ("cursor_link", "cursors/link.png"),
            ("cursor_ibeam", "cursors/ibeam.png"),
            ("cursor_wait", "cursors/wait.png"),
            ("cursor_move", "cursors/move.png"),
            ("cursor_size_nwse", "cursors/size_nwse.png"),
            ("cursor_size_nesw", "cursors/size_nesw.png"),
            ("cursor_size_ns", "cursors/size_ns.png"),
            ("cursor_size_we", "cursors/size_we.png"),
            ("clip_idle", "clip/idle.png"),
            ("clip_think", "clip/think.png"),
            ("clip_read", "clip/read.png"),
            ("clip_weird", "clip/weird.png"),
            ("clip_eyes", "clip/eyes.png"),
            ("clip_listen", "clip/listen.png"),
            ("clip_noted", "clip/noted.png"),
        ];

        for (key, rel_path) in files_to_load {
            let full_path = base_dir.join(rel_path);
            if full_path.exists() {
                if let Ok(img) = image::open(&full_path) {
                    textures.insert(key.to_string(), Texture::from_image(&img));
                }
            }
        }

        Ok(Self { base_dir, textures })
    }

    pub fn get(&self, key: &str) -> Option<&Texture> {
        self.textures.get(key)
    }
}
