//! Saves with photo: thumbnails + auto-save per scene.
//!
//! Thumbnails are tiny (default 96px wide) hashes + optional pixels kept
//! outside the checksum-critical path. Auto-save writes `auto_<scene>`
//! slots when the scene changes so players never lose progress.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Tiny save thumbnail (downscaled RGB, stored compact).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveThumbnail {
    /// Width px.
    pub width: u32,
    /// Height px.
    pub height: u32,
    /// sha256 hex of raw RGB bytes (for load-menu dedup / integrity hint).
    pub hash: String,
    /// Downscaled RGB bytes (row-major, 3 per pixel). Empty = hash only.
    #[serde(default)]
    pub rgb: Vec<u8>,
}

impl SaveThumbnail {
    /// Build from RGB bytes, downsampling by integer step to max_width.
    pub fn from_rgb(width: u32, height: u32, rgb: &[u8], max_width: u32) -> Self {
        if width == 0 || height == 0 || rgb.len() < (width * height * 3) as usize {
            return Self::default();
        }
        let step = (width.div_ceil(max_width.max(1))).max(1);
        let out_w = width.div_ceil(step);
        let out_h = height.div_ceil(step);
        let mut out = Vec::with_capacity((out_w * out_h * 3) as usize);
        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let i = ((y * width + x) * 3) as usize;
                out.push(rgb[i]);
                out.push(rgb[i + 1]);
                out.push(rgb[i + 2]);
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(&out);
        let hash = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Self {
            width: out_w,
            height: out_h,
            hash,
            rgb: out,
        }
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.rgb.is_empty()
    }

    /// Pixel count.
    pub fn len(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}

/// Auto-save policy: which scenes trigger `auto_<scene>` slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoSavePolicy {
    /// Enabled.
    pub enabled: bool,
    /// Max auto slots kept (oldest evicted by caller).
    pub max_slots: usize,
    /// Scene name prefix filter (empty = all scenes).
    pub scene_prefix: String,
}

impl Default for AutoSavePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_slots: 5,
            scene_prefix: String::new(),
        }
    }
}

impl AutoSavePolicy {
    /// Slot id for a scene, or None when filtered/disabled.
    pub fn slot_for_scene(&self, scene: &str) -> Option<String> {
        if !self.enabled || scene.trim().is_empty() {
            return None;
        }
        if !self.scene_prefix.is_empty() && !scene.starts_with(&self.scene_prefix) {
            return None;
        }
        let mut safe = String::with_capacity(scene.len() + 5);
        safe.push_str("auto_");
        for ch in scene.chars() {
            if ch.is_ascii_alphanumeric() {
                safe.push(ch.to_ascii_lowercase());
            } else if ch == '_' || ch == '-' {
                safe.push(ch);
            }
        }
        if safe == "auto_" {
            return None;
        }
        Some(safe)
    }

    /// Trim a slot list (oldest-first vec) to max_slots, returning evicted.
    pub fn trim_slots(&self, slots: &mut Vec<String>) -> Vec<String> {
        let mut evicted = Vec::new();
        while slots.len() > self.max_slots {
            evicted.push(slots.remove(0));
        }
        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                v.push((x % 256) as u8);
                v.push((y % 256) as u8);
                v.push(128);
            }
        }
        v
    }

    #[test]
    fn thumbnail_downsamples_and_hashes() {
        let full = rgb(192, 108);
        let t = SaveThumbnail::from_rgb(192, 108, &full, 96);
        assert_eq!(t.width, 96);
        assert!(!t.hash.is_empty());
        assert_eq!(t.rgb.len(), (96 * 54 * 3) as usize);
        // deterministic
        let t2 = SaveThumbnail::from_rgb(192, 108, &full, 96);
        assert_eq!(t.hash, t2.hash);
    }

    #[test]
    fn thumbnail_rejects_bad_input() {
        let t = SaveThumbnail::from_rgb(0, 0, &[], 96);
        assert!(t.is_empty());
    }

    #[test]
    fn autosave_slot_naming_and_trim() {
        let p = AutoSavePolicy::default();
        assert_eq!(
            p.slot_for_scene("Main Hall").as_deref(),
            Some("auto_mainhall")
        );
        assert_eq!(p.slot_for_scene(""), None);
        let mut slots = vec![
            "auto_a".to_string(),
            "auto_b".to_string(),
            "auto_c".to_string(),
            "auto_d".to_string(),
            "auto_e".to_string(),
            "auto_f".to_string(),
        ];
        let ev = p.trim_slots(&mut slots);
        assert_eq!(ev, vec!["auto_a".to_string()]);
        assert_eq!(slots.len(), 5);
    }

    #[test]
    fn prefix_filter() {
        let p = AutoSavePolicy {
            scene_prefix: "ch1".into(),
            ..Default::default()
        };
        assert!(p.slot_for_scene("ch1_intro").is_some());
        assert!(p.slot_for_scene("ch2_intro").is_none());
    }
}
