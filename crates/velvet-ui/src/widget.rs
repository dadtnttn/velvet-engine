//! Widget kinds.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Widget type payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WidgetKind {
    /// Empty panel.
    Panel,
    /// Text label.
    Label {
        /// Text.
        text: String,
    },
    /// Button.
    Button {
        /// Label.
        label: String,
        /// Pressed this frame.
        pressed: bool,
        /// Hovered.
        hovered: bool,
    },
    /// Image box.
    Image {
        /// Asset path / id.
        source: String,
        /// Preferred size.
        preferred: Vec2,
    },
    /// Progress 0..=1.
    ProgressBar {
        /// Value.
        value: f32,
    },
    /// Slider.
    Slider {
        /// Value 0..=1.
        value: f32,
        /// Dragging.
        dragging: bool,
    },
    /// Toggle.
    Toggle {
        /// On/off.
        value: bool,
        /// Label.
        label: String,
    },
    /// Text field.
    TextField {
        /// Content.
        text: String,
        /// Cursor.
        cursor: usize,
        /// Focused.
        focused: bool,
    },
}

/// Button builder helpers.
pub struct Button;
/// Label helper.
pub struct Label;
/// Panel helper.
pub struct Panel;
/// Image helper.
pub struct ImageBox;
/// Progress helper.
pub struct ProgressBar;
/// Slider helper.
pub struct Slider;
/// Toggle helper.
pub struct Toggle;

impl Button {
    /// Create widget.
    pub fn widget(label: impl Into<String>) -> WidgetKind {
        WidgetKind::Button {
            label: label.into(),
            pressed: false,
            hovered: false,
        }
    }
}

impl Label {
    /// Create widget.
    pub fn widget(text: impl Into<String>) -> WidgetKind {
        WidgetKind::Label { text: text.into() }
    }
}

impl Panel {
    /// Create panel widget.
    pub fn widget() -> WidgetKind {
        WidgetKind::Panel
    }
}

impl ImageBox {
    /// Create image widget.
    pub fn widget(source: impl Into<String>, preferred: Vec2) -> WidgetKind {
        WidgetKind::Image {
            source: source.into(),
            preferred,
        }
    }
}

impl ProgressBar {
    /// Create progress bar with value clamped to `0..=1`.
    pub fn widget(value: f32) -> WidgetKind {
        WidgetKind::ProgressBar {
            value: value.clamp(0.0, 1.0),
        }
    }
}

impl Slider {
    /// Create slider.
    pub fn widget(value: f32) -> WidgetKind {
        WidgetKind::Slider {
            value: value.clamp(0.0, 1.0),
            dragging: false,
        }
    }
}

impl Toggle {
    /// Create toggle.
    pub fn widget(label: impl Into<String>, value: bool) -> WidgetKind {
        WidgetKind::Toggle {
            label: label.into(),
            value,
        }
    }
}

impl WidgetKind {
    /// Progress / slider value if applicable.
    pub fn value01(&self) -> Option<f32> {
        match self {
            Self::ProgressBar { value } | Self::Slider { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Set progress / slider value.
    pub fn set_value01(&mut self, v: f32) {
        let v = v.clamp(0.0, 1.0);
        match self {
            Self::ProgressBar { value } => *value = v,
            Self::Slider { value, .. } => *value = v,
            _ => {}
        }
    }

    /// Label / button / toggle text if any.
    pub fn label_text(&self) -> Option<&str> {
        match self {
            Self::Label { text } => Some(text),
            Self::Button { label, .. } => Some(label),
            Self::Toggle { label, .. } => Some(label),
            Self::TextField { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Toggle value.
    pub fn toggle_value(&self) -> Option<bool> {
        match self {
            Self::Toggle { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Flip toggle if this is a toggle.
    pub fn flip_toggle(&mut self) {
        if let Self::Toggle { value, .. } = self {
            *value = !*value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_and_value() {
        let mut p = ProgressBar::widget(1.5);
        assert_eq!(p.value01(), Some(1.0));
        p.set_value01(0.25);
        assert_eq!(p.value01(), Some(0.25));
        let mut t = Toggle::widget("Mute", false);
        t.flip_toggle();
        assert_eq!(t.toggle_value(), Some(true));
        assert_eq!(Button::widget("OK").label_text(), Some("OK"));
        assert!(matches!(Panel::widget(), WidgetKind::Panel));
        let img = ImageBox::widget("a.png", Vec2::new(32.0, 32.0));
        assert!(matches!(img, WidgetKind::Image { .. }));
        let s = Slider::widget(-1.0);
        assert_eq!(s.value01(), Some(0.0));
    }
}
