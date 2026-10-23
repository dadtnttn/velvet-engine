//! # velvet-ui
//!
//! Immediate-mode-friendly retained UI tree: layout, widgets, focus, themes.

#![deny(missing_docs)]

mod animation;
mod button_fx;
mod dialogue_ui;
mod focus;
mod layout;
mod list;
mod nine_patch;
mod node;
mod particles;
mod scroll;
mod sound;
mod style;
mod theme;
mod transition;
mod tree;
mod vcss;
mod widget;

pub mod prelude;
/// Labels anchored to things in the world, laid out off the HUD and off each other.
pub mod world_tags;

pub use animation::{Ease, TweenProperty, TweenSample, UiAnimator, UiTween};
pub use button_fx::{
    drag_glow, spinner_angle, AnimatedButton, AnimatedToggle, ButtonFxPreset, ButtonSample,
    ButtonVisual, PressRipple,
};
pub use dialogue_ui::{DialogueAdvance, DialogueBox, DialogueBoxPhase};
pub use focus::{FocusDir, FocusState};
pub use layout::{anchor_bottom, anchor_in, Anchor, LayoutConstraints, LayoutResult, LayoutType};
pub use list::VirtualList;
pub use nine_patch::{
    content_rect, layout_nine_patch, layout_nine_patch_with_destination_margins, NinePatchCell,
    NinePatchMargins, NinePatchQuad,
};
pub use node::{NodeId, UiNode, UiRect};
pub use particles::{UiBurst, UiParticle, UiParticlePoint, UiParticles};
pub use scroll::{clamp_scroll_offset, ScrollView};
pub use sound::{UiSound, UiSoundKind, UiSoundQueue};
pub use style::{UiBoxShadow, UiColor, UiGradient, UiImageSource, UiNineSlice, UiStyle};
pub use theme::{scale_style_for_density, Density, Theme, ThemeRegistry};
pub use transition::{capture_positions, flip_deltas, AppearTracker, FlipDelta, PositionSnapshot};
pub use tree::{HitResult, InputModality, UiContext, UiTree};
pub use vcss::{apply_computed_style, resolve_node_style, style_query_for_node};
pub use widget::{
    Button, Dropdown, ImageBox, Label, Panel, ProgressBar, Slider, Tabs, Toggle, Tooltip,
    WidgetKind,
};
