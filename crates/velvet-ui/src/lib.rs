//! # velvet-ui
//!
//! Immediate-mode-friendly retained UI tree: layout, widgets, focus, themes.

#![deny(missing_docs)]

mod animation;
mod dialogue_ui;
mod focus;
mod layout;
mod nine_patch;
mod node;
mod scroll;
mod style;
mod theme;
mod tree;
mod vcss;
mod widget;

pub mod prelude;

pub use animation::{Ease, TweenProperty, TweenSample, UiAnimator, UiTween};
pub use dialogue_ui::{DialogueAdvance, DialogueBox, DialogueBoxPhase};
pub use focus::{FocusDir, FocusState};
pub use layout::{LayoutConstraints, LayoutResult, LayoutType};
pub use nine_patch::{
    content_rect, layout_nine_patch, NinePatchCell, NinePatchMargins, NinePatchQuad,
};
pub use node::{NodeId, UiNode, UiRect};
pub use scroll::{clamp_scroll_offset, ScrollView};
pub use style::{UiColor, UiStyle};
pub use theme::Theme;
pub use tree::{HitResult, UiContext, UiTree};
pub use vcss::{apply_computed_style, resolve_node_style, style_query_for_node};
pub use widget::{Button, ImageBox, Label, Panel, ProgressBar, Slider, Toggle, WidgetKind};
