use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::GenericImageView;

use crate::model::SamLook;

#[derive(Debug, Clone)]
pub struct ImageAsset {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl ImageAsset {
    fn from_path(path: &Path) -> Result<Self> {
        let image = image::open(path)
            .with_context(|| format!("no se pudo cargar la imagen {}", path.display()))?;
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
    missing: HashMap<String, bool>,
}

impl AssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        // Character and background media is intentionally excluded from the repository.
        // Supply your own compatible assets; missing layers fall back to procedural UI.
        Self {
            root: root.into(),
            cache: HashMap::new(),
            missing: HashMap::new(),
        }
    }

    pub fn get(&mut self, relative: &str) -> Option<&ImageAsset> {
        if self.missing.contains_key(relative) {
            return None;
        }
        if !self.cache.contains_key(relative) {
            let path = self.root.join(relative);
            match ImageAsset::from_path(&path) {
                Ok(image) => {
                    self.cache.insert(relative.to_owned(), image);
                }
                Err(error) => {
                    eprintln!("[sam-assets] {error:#}");
                    self.missing.insert(relative.to_owned(), true);
                    return None;
                }
            }
        }
        self.cache.get(relative)
    }

    pub fn get_cloned(&mut self, relative: &str) -> Option<ImageAsset> {
        self.get(relative).cloned()
    }

    pub fn background(&mut self, name: &str) -> Option<ImageAsset> {
        self.get_cloned(&format!("backgrounds/png/{name}.png"))
    }

    fn shirt_path(pose: u8) -> String {
        if pose == 7 || pose == 8 {
            format!("tomboy/Clothes/sm_shirt_pose{pose}.png")
        } else {
            "tomboy/Clothes/sm_shirt.png".to_owned()
        }
    }

    fn casual_shirt_path(pose: u8) -> String {
        let shirt = match pose {
            1 | 4 => "sm_casualshirt_pose1-4.png",
            2 | 6 => "sm_casualshirt_pose2-6.png",
            3 => "sm_casualshirt_pose3.png",
            5 => "sm_casualshirt_pose5.png",
            7 => "sm_casualshirt_pose7.png",
            _ => "sm_casualshirt_pose8.png",
        };
        format!("tomboy/Outfit2/{shirt}")
    }

    fn work_shirt_path(pose: u8) -> String {
        let shirt = match pose {
            1 | 4 => "sm_workshirt2_pose1-4.png",
            2 | 6 => "sm_workshirt2_pose2-6.png",
            3 => "sm_workshirt2_pose3.png",
            5 => "sm_workshirt2_pose5.png",
            7 => "sm_workshirt2_pose7.png",
            _ => "sm_workshirt2_pose8.png",
        };
        format!("tomboy/Outfit3/{shirt}")
    }

    fn pose8_or_default(folder: &str, item: &str, pose: u8) -> String {
        if pose == 8 {
            format!("tomboy/{folder}/sm_{item}_pose8.png")
        } else {
            format!("tomboy/{folder}/sm_{item}.png")
        }
    }

    fn bikini_part_path(part: &str, pose: u8) -> String {
        if pose == 8 {
            format!("tomboy/Clothes/generated/sm_bikini_{part}_pose8.png")
        } else {
            format!("tomboy/Clothes/generated/sm_bikini_{part}.png")
        }
    }

    pub fn sam_layers(&self, look: SamLook) -> Vec<String> {
        let pose = look.pose.clamp(1, 8);
        let mut layers = vec![format!("tomboy/Poses/sm_pose{pose}.png")];

        match look.outfit {
            "shirt" => layers.push(Self::shirt_path(pose)),
            "hoodie" => layers.push(format!("tomboy/Clothes/sm_hoodie_pose{pose}.png")),
            "bikini" => layers.push(if pose == 8 {
                "tomboy/Clothes/sm_bikini_pose8.png".to_owned()
            } else {
                "tomboy/Clothes/sm_bikini.png".to_owned()
            }),
            "panties" => layers.push(Self::pose8_or_default("Clothes", "panties", pose)),
            "casual" => {
                layers.push(Self::casual_shirt_path(pose));
                for item in ["casualshorts", "casualsocks", "casualshoes"] {
                    layers.push(Self::pose8_or_default("Outfit2", item, pose));
                }
            }
            "work" => {
                layers.push(Self::work_shirt_path(pose));
                for item in ["workpants2", "workshoes2"] {
                    layers.push(Self::pose8_or_default("Outfit3", item, pose));
                }
            }
            _ => {}
        }

        layers.push(format!("tomboy/Expressions/sm_{}.png", look.expression));
        layers
    }

    pub fn dressup_layers(
        &mut self,
        pose: u8,
        top: &str,
        bottom: &str,
        shoes: &str,
        accessory: &str,
        expression: &str,
    ) -> Vec<ImageAsset> {
        let pose = pose.clamp(1, 8);
        let mut paths = vec![format!("tomboy/Poses/sm_pose{pose}.png")];

        match bottom {
            "panties" => paths.push(Self::pose8_or_default("Clothes", "panties", pose)),
            "pants" => paths.push(Self::pose8_or_default("Clothes", "pants", pose)),
            "casual" => paths.push(Self::pose8_or_default("Outfit2", "casualshorts", pose)),
            "work" => paths.push(Self::pose8_or_default("Outfit3", "workpants2", pose)),
            "bikini" => paths.push(Self::bikini_part_path("bottom", pose)),
            _ => {}
        }

        match top {
            "shirt" => paths.push(Self::shirt_path(pose)),
            "hoodie" => paths.push(format!("tomboy/Clothes/sm_hoodie_pose{pose}.png")),
            "casual" => paths.push(Self::casual_shirt_path(pose)),
            "work" => paths.push(Self::work_shirt_path(pose)),
            "bikini" => paths.push(Self::bikini_part_path("top", pose)),
            _ => {}
        }

        match shoes {
            "socks" => paths.push(Self::pose8_or_default("Outfit2", "casualsocks", pose)),
            "casual" => {
                paths.push(Self::pose8_or_default("Outfit2", "casualsocks", pose));
                paths.push(Self::pose8_or_default("Outfit2", "casualshoes", pose));
            }
            "work" => paths.push(Self::pose8_or_default("Outfit3", "workshoes2", pose)),
            _ => {}
        }

        // El rostro y el cabello deben dibujarse antes que los accesorios de cabeza.
        // Si la expresión se dibuja después, tapa casi toda la gorra y solo deja una franja.
        paths.push(format!("tomboy/Expressions/sm_{expression}.png"));

        match accessory {
            "collar" => paths.push(Self::pose8_or_default("Clothes", "collar", pose)),
            "work_hat" => paths.push(Self::pose8_or_default("Outfit3", "workhat2", pose)),
            _ => {}
        }

        paths
            .into_iter()
            .filter_map(|path| self.get_cloned(&path))
            .collect()
    }

    pub fn character_layers(&mut self, look: SamLook) -> Vec<ImageAsset> {
        self.sam_layers(look)
            .into_iter()
            .filter_map(|path| self.get_cloned(&path))
            .collect()
    }

    pub fn cg(&mut self, index: usize, alternate: bool) -> Option<ImageAsset> {
        let number = index % 3 + 1;
        let suffix = if alternate { "Cum" } else { "" };
        self.get_cloned(&format!("cg/Sam{number}{suffix}.png"))
    }

    pub fn animation_frame(&mut self, frame: usize) -> Option<ImageAsset> {
        let frame = frame.clamp(1, 30);
        self.get_cloned(&format!("anim/bjanim_{frame:04}.png"))
    }
}
