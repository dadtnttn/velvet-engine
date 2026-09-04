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
        /// Busy (spinner): blocks clicks silently, unlike disabled.
        #[serde(default)]
        busy: bool,
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
    /// Dropdown / select box.
    Dropdown {
        /// Options in order.
        options: Vec<String>,
        /// Selected option index.
        selected: Option<usize>,
        /// Keyboard highlight index.
        highlight: usize,
        /// Popup open.
        open: bool,
    },
    /// Tab strip (children map to tabs by position).
    Tabs {
        /// Tab labels in order (matches direct children by position).
        tabs: Vec<String>,
        /// Selected tab index.
        selected: usize,
    },
    /// Hover tooltip for another node (see `target`).
    Tooltip {
        /// Text.
        text: String,
        /// Node name this tooltip describes.
        target: String,
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
/// Dropdown helper.
pub struct Dropdown;
/// Tabs helper.
pub struct Tabs;
/// Tooltip helper.
pub struct Tooltip;

impl Button {
    /// Create widget.
    pub fn widget(label: impl Into<String>) -> WidgetKind {
        WidgetKind::Button {
            label: label.into(),
            pressed: false,
            hovered: false,
            busy: false,
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

impl Dropdown {
    /// Create dropdown (nothing selected, closed).
    pub fn widget(options: Vec<String>) -> WidgetKind {
        WidgetKind::Dropdown {
            options,
            selected: None,
            highlight: 0,
            open: false,
        }
    }
}

impl Tabs {
    /// Create tab strip (first tab selected).
    pub fn widget(tabs: Vec<String>) -> WidgetKind {
        WidgetKind::Tabs { tabs, selected: 0 }
    }
}

impl Tooltip {
    /// Create tooltip for the node named `target`.
    pub fn widget(text: impl Into<String>, target: impl Into<String>) -> WidgetKind {
        WidgetKind::Tooltip {
            text: text.into(),
            target: target.into(),
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

    /// Dropdown selected index.
    pub fn dropdown_selected(&self) -> Option<Option<usize>> {
        match self {
            Self::Dropdown { selected, .. } => Some(*selected),
            _ => None,
        }
    }

    /// Set dropdown selection (clamped; `None` clears).
    pub fn dropdown_select(&mut self, index: Option<usize>) {
        if let Self::Dropdown {
            options,
            selected,
            highlight,
            ..
        } = self
        {
            let valid = index.filter(|i| *i < options.len());
            *selected = valid;
            if let Some(i) = valid {
                *highlight = i;
            }
        }
    }

    /// Tabs selected index.
    pub fn tabs_selected(&self) -> Option<usize> {
        match self {
            Self::Tabs { selected, .. } => Some(*selected),
            _ => None,
        }
    }

    /// Set tabs selection (wraps around, ignored when empty).
    pub fn tabs_select(&mut self, index: usize) {
        if let Self::Tabs { tabs, selected } = self {
            if !tabs.is_empty() {
                *selected = index % tabs.len();
            }
        }
    }

    /// Tooltip text and target.
    pub fn tooltip(&self) -> Option<(&str, &str)> {
        match self {
            Self::Tooltip { text, target } => Some((text, target)),
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
    fn new_widgets_build_and_select() {
        let mut dd = Dropdown::widget(vec!["a".into(), "b".into()]);
        assert_eq!(dd.dropdown_selected(), Some(None));
        dd.dropdown_select(Some(5));
        assert_eq!(dd.dropdown_selected(), Some(None));
        dd.dropdown_select(Some(1));
        assert_eq!(dd.dropdown_selected(), Some(Some(1)));
        let mut tabs = Tabs::widget(vec!["x".into(), "y".into(), "z".into()]);
        assert_eq!(tabs.tabs_selected(), Some(0));
        tabs.tabs_select(5);
        assert_eq!(tabs.tabs_selected(), Some(2));
        let tip = Tooltip::widget("Help", "save");
        assert_eq!(tip.tooltip(), Some(("Help", "save")));
        assert_eq!(Button::widget("OK").dropdown_selected(), None);
    }

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
