//! UI node tree elements.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

use crate::layout::LayoutType;
use crate::style::UiStyle;
use crate::widget::WidgetKind;

/// Node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

/// Computed rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UiRect {
    /// Position.
    pub pos: Vec2,
    /// Size.
    pub size: Vec2,
}

impl UiRect {
    /// Contains point.
    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.pos.x
            && p.y >= self.pos.y
            && p.x <= self.pos.x + self.size.x
            && p.y <= self.pos.y + self.size.y
    }

    /// Max corner.
    pub fn max(self) -> Vec2 {
        self.pos + self.size
    }
}

/// One UI node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiNode {
    /// Id.
    pub id: NodeId,
    /// Optional name.
    pub name: String,
    /// Widget.
    pub widget: WidgetKind,
    /// Style.
    pub style: UiStyle,
    /// Layout mode for children.
    pub layout: LayoutType,
    /// Children ids.
    pub children: Vec<NodeId>,
    /// Parent.
    pub parent: Option<NodeId>,
    /// Computed rect.
    pub rect: UiRect,
    /// Visible.
    pub visible: bool,
    /// Enabled.
    pub enabled: bool,
    /// Focusable.
    pub focusable: bool,
    /// User data bag.
    pub data: IndexMap<String, String>,
}

impl UiNode {
    /// Create.
    pub fn new(id: NodeId, name: impl Into<String>, widget: WidgetKind) -> Self {
        let focusable = matches!(
            &widget,
            WidgetKind::Button { .. }
                | WidgetKind::Slider { .. }
                | WidgetKind::Toggle { .. }
                | WidgetKind::TextField { .. }
        );
        Self {
            id,
            name: name.into(),
            widget,
            style: UiStyle::default(),
            layout: LayoutType::Stack,
            children: Vec::new(),
            parent: None,
            rect: UiRect::default(),
            visible: true,
            enabled: true,
            focusable,
            data: IndexMap::new(),
        }
    }
}
