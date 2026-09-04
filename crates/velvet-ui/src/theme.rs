//! UI themes: named sets, registry, and density scaling.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use velvet_math::Color;

use crate::style::UiStyle;
use crate::tree::UiTree;

/// Named theme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Name.
    pub name: String,
    /// Default panel.
    pub panel: UiStyle,
    /// Button.
    pub button: UiStyle,
    /// Button hovered.
    pub button_hovered: UiStyle,
    /// Accent.
    pub accent: Color,
    /// Danger.
    pub danger: Color,
    /// Font.
    pub font: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::velvet_dark()
    }
}

impl Theme {
    /// Engine default dark theme.
    pub fn velvet_dark() -> Self {
        let mut button_hovered = UiStyle::button();
        button_hovered.background = Color::rgba(0.4, 0.3, 0.55, 1.0);
        Self {
            name: "velvet-dark".into(),
            panel: UiStyle::dialogue_panel(),
            button: UiStyle::button(),
            button_hovered,
            accent: Color::VELVET,
            danger: Color::rgb(0.9, 0.25, 0.3),
            font: "default".into(),
        }
    }

    /// Light theme.
    pub fn velvet_light() -> Self {
        let mut panel = UiStyle::dialogue_panel();
        panel.background = Color::rgba(0.95, 0.95, 0.97, 0.96);
        panel.text_color = Color::rgb(0.1, 0.1, 0.12);
        let mut button = UiStyle::button();
        button.background = Color::rgba(0.85, 0.8, 0.95, 1.0);
        button.text_color = Color::rgb(0.1, 0.1, 0.12);
        Self {
            name: "velvet-light".into(),
            panel,
            button: button.clone(),
            button_hovered: button,
            accent: Color::VELVET,
            danger: Color::rgb(0.8, 0.15, 0.2),
            font: "default".into(),
        }
    }
}

/// Density preset: scales spacing and type, never colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Density {
    /// Tighter spacing for dense HUDs (0.85×).
    Compact,
    /// Authored baseline (1.0×).
    #[default]
    Comfortable,
}

impl Density {
    /// Scale factor.
    pub fn factor(self) -> f32 {
        match self {
            Self::Compact => 0.85,
            Self::Comfortable => 1.0,
        }
    }
}

/// Scale the spacing/type fields of one style by a density factor.
pub fn scale_style_for_density(style: &UiStyle, factor: f32) -> UiStyle {
    let mut out = style.clone();
    let s = |v: f32| (v * factor).max(0.0);
    let t = |t: (f32, f32, f32, f32)| (s(t.0), s(t.1), s(t.2), s(t.3));
    let p = |p: (f32, f32)| (s(p.0), s(p.1));
    out.padding = t(out.padding);
    out.margin = t(out.margin);
    out.gap = s(out.gap);
    out.min_size = p(out.min_size);
    out.max_size = p(out.max_size);
    out.border_width = s(out.border_width);
    out.radius = s(out.radius);
    out.font_size = s(out.font_size).clamp(10.0, 96.0);
    out
}

/// Named theme registry with an active theme and density.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeRegistry {
    /// Themes by name (insertion order).
    pub themes: IndexMap<String, Theme>,
    /// Active theme name.
    pub active: String,
    /// Active density.
    pub density: Density,
}

impl ThemeRegistry {
    /// Registry with both shipped themes, dark active.
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        registry.register(Theme::velvet_dark());
        registry.register(Theme::velvet_light());
        registry.active = "velvet-dark".into();
        registry
    }

    /// Register or replace a theme.
    pub fn register(&mut self, theme: Theme) {
        if self.active.is_empty() {
            self.active = theme.name.clone();
        }
        self.themes.insert(theme.name.clone(), theme);
    }

    /// Active theme, if any.
    pub fn active(&self) -> Option<&Theme> {
        self.themes.get(&self.active)
    }

    /// Switch active theme (unknown names are ignored).
    pub fn set_active(&mut self, name: &str) {
        if self.themes.contains_key(name) {
            self.active = name.to_string();
        }
    }

    /// Active theme with density scaling applied.
    pub fn active_scaled(&self) -> Option<Theme> {
        self.active().map(|theme| {
            let mut scaled = theme.clone();
            let f = self.density.factor();
            scaled.panel = scale_style_for_density(&theme.panel, f);
            scaled.button = scale_style_for_density(&theme.button, f);
            scaled.button_hovered = scale_style_for_density(&theme.button_hovered, f);
            scaled
        })
    }

    /// Push the active (density-scaled) theme into a tree.
    pub fn apply_active(&self, tree: &mut UiTree) {
        if let Some(theme) = self.active_scaled() {
            tree.theme = theme;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_switches_and_scales() {
        let mut registry = ThemeRegistry::with_defaults();
        assert_eq!(
            registry.active().map(|t| t.name.as_str()),
            Some("velvet-dark")
        );
        registry.set_active("nope");
        assert_eq!(registry.active, "velvet-dark");
        registry.set_active("velvet-light");
        assert_eq!(registry.active, "velvet-light");
        registry.density = Density::Compact;
        let scaled = registry.active_scaled().unwrap();
        let base = Theme::velvet_light();
        assert!(scaled.button.font_size < base.button.font_size);
        assert!((scaled.button.font_size - base.button.font_size * 0.85).abs() < 1e-4);
        // Colors never scale.
        assert_eq!(scaled.button.background, base.button.background);
        // Compact never shrinks type below the floor.
        let mut tiny = UiStyle::button();
        tiny.font_size = 8.0;
        assert_eq!(scale_style_for_density(&tiny, 0.85).font_size, 10.0);
    }

    #[test]
    fn apply_active_pushes_theme_into_tree() {
        let mut registry = ThemeRegistry::with_defaults();
        registry.set_active("velvet-light");
        let mut tree = UiTree::with_root("root");
        registry.apply_active(&mut tree);
        assert_eq!(tree.theme.name, "velvet-light");
    }
}
