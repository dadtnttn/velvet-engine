//! UI themes.

use serde::{Deserialize, Serialize};
use velvet_math::Color;

use crate::style::UiStyle;

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
