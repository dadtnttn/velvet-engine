//! Platform profile filters for asset selection (HD/SD, desktop/mobile, locale).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// High-level platform class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatformClass {
    /// Desktop (Windows/macOS/Linux).
    Desktop,
    /// Mobile phone/tablet.
    Mobile,
    /// Web / WASM.
    Web,
    /// Console-like restricted environment.
    Console,
}

impl PlatformClass {
    /// Detect roughly from target cfg (compile-time).
    pub fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            return Self::Web;
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            return Self::Mobile;
        }
        #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
        {
            Self::Desktop
        }
    }
}

/// Asset quality / density tier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum QualityTier {
    /// Lowest / mobile data saver.
    Low,
    /// Default.
    #[default]
    Medium,
    /// High-end.
    High,
    /// Ultra (desktop showcase).
    Ultra,
}

/// Runtime platform profile used to filter asset variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformProfile {
    /// Platform class.
    pub class: PlatformClass,
    /// Quality tier.
    pub quality: QualityTier,
    /// Preferred locale tags (BCP-47-ish), ordered.
    pub locales: Vec<String>,
    /// Feature tags enabled (e.g. "hdr", "touch").
    pub features: BTreeSet<String>,
    /// Max texture dimension hint.
    pub max_texture_size: u32,
}

impl Default for PlatformProfile {
    fn default() -> Self {
        Self {
            class: PlatformClass::current(),
            quality: QualityTier::Medium,
            locales: vec!["en".into()],
            features: BTreeSet::new(),
            max_texture_size: 4096,
        }
    }
}

impl PlatformProfile {
    /// Desktop high quality profile.
    pub fn desktop_high() -> Self {
        Self {
            class: PlatformClass::Desktop,
            quality: QualityTier::High,
            locales: vec!["en".into()],
            features: ["hdr", "mouse"].into_iter().map(str::to_string).collect(),
            max_texture_size: 8192,
        }
    }

    /// Mobile medium profile.
    pub fn mobile_medium() -> Self {
        Self {
            class: PlatformClass::Mobile,
            quality: QualityTier::Medium,
            locales: vec!["en".into()],
            features: ["touch"].into_iter().map(str::to_string).collect(),
            max_texture_size: 2048,
        }
    }

    /// Whether a feature tag is enabled.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(feature)
    }

    /// Add feature.
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.insert(feature.into());
        self
    }

    /// Set locales.
    pub fn with_locales(mut self, locales: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.locales = locales.into_iter().map(|s| s.into()).collect();
        self
    }
}

/// Variant descriptor attached to an asset path candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetVariant {
    /// Logical asset key (without variant suffixes).
    pub key: String,
    /// Concrete path to load.
    pub path: String,
    /// Minimum quality tier required (inclusive).
    pub min_quality: QualityTier,
    /// Maximum quality tier (inclusive).
    pub max_quality: QualityTier,
    /// Allowed platforms (empty = all).
    pub platforms: Vec<PlatformClass>,
    /// Required features (all must match).
    pub require_features: Vec<String>,
    /// Locale this variant targets (`None` = default/neutral).
    pub locale: Option<String>,
    /// Priority score (higher preferred when multiple match).
    pub priority: i32,
}

impl AssetVariant {
    /// Simple path variant with full quality range.
    pub fn simple(key: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            path: path.into(),
            min_quality: QualityTier::Low,
            max_quality: QualityTier::Ultra,
            platforms: Vec::new(),
            require_features: Vec::new(),
            locale: None,
            priority: 0,
        }
    }

    /// Whether this variant is eligible for the profile (ignoring locale preference rank).
    pub fn matches_profile(&self, profile: &PlatformProfile) -> bool {
        if profile.quality < self.min_quality || profile.quality > self.max_quality {
            return false;
        }
        if !self.platforms.is_empty() && !self.platforms.contains(&profile.class) {
            return false;
        }
        for f in &self.require_features {
            if !profile.has_feature(f) {
                return false;
            }
        }
        if let Some(loc) = &self.locale {
            // Eligible if profile lists this locale OR variant is neutral handled separately.
            if !profile
                .locales
                .iter()
                .any(|l| l == loc || l.starts_with(&format!("{loc}-")))
            {
                // Still allow if no locale match — ranked lower later; here we filter hard only
                // when profile has locales and none match — actually keep eligible for fallback.
                let _ = loc;
            }
        }
        if self.path_needs_texture_budget(profile) {
            return false;
        }
        true
    }

    fn path_needs_texture_budget(&self, profile: &PlatformProfile) -> bool {
        // Heuristic: paths containing `@4x` need high texture budget.
        if self.path.contains("@4x") && profile.max_texture_size < 4096 {
            return true;
        }
        if self.path.contains("@8k") && profile.max_texture_size < 8192 {
            return true;
        }
        false
    }

    /// Locale match score: higher is better; -1 = mismatch when locale set.
    pub fn locale_score(&self, profile: &PlatformProfile) -> i32 {
        match &self.locale {
            None => 0,
            Some(loc) => {
                for (i, pl) in profile.locales.iter().enumerate() {
                    if pl == loc || pl.starts_with(&format!("{loc}-")) || loc.starts_with(pl) {
                        return 100 - i as i32;
                    }
                }
                -1
            }
        }
    }
}

/// Catalog of variants for resolution.
#[derive(Debug, Clone, Default)]
pub struct VariantCatalog {
    variants: Vec<AssetVariant>,
}

impl VariantCatalog {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert variant.
    pub fn add(&mut self, variant: AssetVariant) {
        self.variants.push(variant);
    }

    /// All variants for a key.
    pub fn for_key<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a AssetVariant> + 'a {
        self.variants.iter().filter(move |v| v.key == key)
    }

    /// Resolve best path for key under profile.
    pub fn resolve<'a>(&'a self, key: &str, profile: &PlatformProfile) -> Option<&'a AssetVariant> {
        let mut best: Option<&'a AssetVariant> = None;
        let mut best_score = i32::MIN;
        for v in self.variants.iter().filter(|v| v.key == key) {
            if !v.matches_profile(profile) {
                continue;
            }
            let loc = v.locale_score(profile);
            if loc < 0 && v.locale.is_some() {
                // Prefer locale-matching; allow as weak fallback with low score.
            }
            let score = v.priority * 1000
                + loc.max(0) * 10
                + if loc >= 0 { 5 } else { 0 }
                + quality_closeness(v, profile);
            if score > best_score {
                best_score = score;
                best = Some(v);
            }
        }
        best
    }

    /// Resolve path string.
    pub fn resolve_path(&self, key: &str, profile: &PlatformProfile) -> Option<String> {
        self.resolve(key, profile).map(|v| v.path.clone())
    }

    /// Filter all variants that match.
    pub fn matching<'a>(
        &'a self,
        profile: &'a PlatformProfile,
    ) -> impl Iterator<Item = &'a AssetVariant> {
        self.variants.iter().filter(|v| v.matches_profile(profile))
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.variants.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }
}

fn quality_closeness(v: &AssetVariant, profile: &PlatformProfile) -> i32 {
    // Prefer variants whose max quality is closer to profile (not over-provisioned).
    let q = profile.quality as i32;
    let max = v.max_quality as i32;
    4 - (max - q).abs().min(4)
}

/// Parse a simple platform suffix from a filename (`sprite@mobile.png`, `bg@2x.webp`).
pub fn parse_path_tags(path: &str) -> PathTags {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name);
    let mut tags = PathTags::default();
    for part in stem.split('@').skip(1) {
        match part {
            "mobile" => tags.platform = Some(PlatformClass::Mobile),
            "desktop" => tags.platform = Some(PlatformClass::Desktop),
            "web" => tags.platform = Some(PlatformClass::Web),
            "low" => tags.quality = Some(QualityTier::Low),
            "mid" | "medium" => tags.quality = Some(QualityTier::Medium),
            "high" => tags.quality = Some(QualityTier::High),
            "ultra" => tags.quality = Some(QualityTier::Ultra),
            "2x" | "4x" | "8k" => tags.scale_tag = Some(part.into()),
            other => {
                if other.len() == 2 || other.contains('-') {
                    tags.locale = Some(other.into());
                } else {
                    tags.features.push(other.into());
                }
            }
        }
    }
    tags
}

/// Tags extracted from a path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathTags {
    /// Platform constraint.
    pub platform: Option<PlatformClass>,
    /// Quality.
    pub quality: Option<QualityTier>,
    /// Locale.
    pub locale: Option<String>,
    /// Scale tag string.
    pub scale_tag: Option<String>,
    /// Extra features.
    pub features: Vec<String>,
}

impl PathTags {
    /// Convert into a variant for `key`.
    pub fn into_variant(self, key: impl Into<String>, path: impl Into<String>) -> AssetVariant {
        let mut v = AssetVariant::simple(key, path);
        if let Some(p) = self.platform {
            v.platforms = vec![p];
        }
        if let Some(q) = self.quality {
            v.min_quality = q;
            v.max_quality = q;
        }
        v.locale = self.locale;
        v.require_features = self.features;
        if self.scale_tag.as_deref() == Some("4x") {
            v.min_quality = QualityTier::High;
            v.priority += 1;
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_locale_and_quality() {
        let mut cat = VariantCatalog::new();
        cat.add(AssetVariant {
            key: "ui.confirm".into(),
            path: "ui/confirm_en.png".into(),
            min_quality: QualityTier::Low,
            max_quality: QualityTier::Ultra,
            platforms: vec![],
            require_features: vec![],
            locale: Some("en".into()),
            priority: 0,
        });
        cat.add(AssetVariant {
            key: "ui.confirm".into(),
            path: "ui/confirm_ja.png".into(),
            min_quality: QualityTier::Low,
            max_quality: QualityTier::Ultra,
            platforms: vec![],
            require_features: vec![],
            locale: Some("ja".into()),
            priority: 0,
        });
        cat.add(AssetVariant {
            key: "ui.confirm".into(),
            path: "ui/confirm_en_hd.png".into(),
            min_quality: QualityTier::High,
            max_quality: QualityTier::Ultra,
            platforms: vec![PlatformClass::Desktop],
            require_features: vec![],
            locale: Some("en".into()),
            priority: 2,
        });

        let mobile = PlatformProfile::mobile_medium().with_locales(["ja"]);
        let path = cat.resolve_path("ui.confirm", &mobile).unwrap();
        assert!(path.contains("ja"));

        let desk = PlatformProfile::desktop_high();
        let path = cat.resolve_path("ui.confirm", &desk).unwrap();
        assert!(path.contains("hd"));
    }

    #[test]
    fn feature_gate() {
        let mut cat = VariantCatalog::new();
        cat.add(AssetVariant {
            key: "fx".into(),
            path: "fx_hdr.tex".into(),
            min_quality: QualityTier::Low,
            max_quality: QualityTier::Ultra,
            platforms: vec![],
            require_features: vec!["hdr".into()],
            locale: None,
            priority: 5,
        });
        cat.add(AssetVariant::simple("fx", "fx.tex"));
        let no_hdr = PlatformProfile::mobile_medium();
        assert_eq!(cat.resolve_path("fx", &no_hdr).unwrap(), "fx.tex");
        let hdr = PlatformProfile::desktop_high();
        assert_eq!(cat.resolve_path("fx", &hdr).unwrap(), "fx_hdr.tex");
    }

    #[test]
    fn parse_tags() {
        let t = parse_path_tags("sprites/hero@mobile@2x.png");
        assert_eq!(t.platform, Some(PlatformClass::Mobile));
        assert_eq!(t.scale_tag.as_deref(), Some("2x"));
    }

    #[test]
    fn texture_budget_filters_4x() {
        let mut cat = VariantCatalog::new();
        cat.add(AssetVariant::simple("bg", "bg@4x.png"));
        cat.add(AssetVariant::simple("bg", "bg.png"));
        let low = PlatformProfile {
            max_texture_size: 2048,
            ..PlatformProfile::mobile_medium()
        };
        assert_eq!(cat.resolve_path("bg", &low).unwrap(), "bg.png");
    }
}
