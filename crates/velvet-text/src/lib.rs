//! # velvet-text
//!
//! Rich text markup, layout measurement, and typewriter reveal for narrative UI.

#![deny(missing_docs)]

mod gpu_text;
mod icon;
mod layout;
mod markup;
mod measure;
mod rtl;
mod ruby;
mod shape;
mod style;
mod typewriter;

pub mod prelude;

pub use gpu_text::{
    flatten_glyph_quads, layout_product_text_items, GlyphAtlas, GpuGlyphQuad, GpuTextRasterizer,
    GpuTextRun,
};
pub use icon::{format_icon_markup, parse_icon_attrs, IconGlyphMap, IconSpan};
pub use layout::{AlignedLine, TextAlign, TextLayout};
pub use markup::{parse_rich_text, MarkupError, RichSpan, RichText};
pub use measure::{measure_width, GlyphMetrics, TextMetrics};
pub use rtl::{
    is_mostly_rtl, is_rtl_char, prepare_line_for_display, reverse_display_order, strip_bidi_marks,
    wrap_for_base_direction, BidiMark,
};
pub use ruby::{format_ruby_markup, parse_ruby_tag, RubySpan};
pub use shape::{
    naive_codepoint_width, set_shape_font_bytes, shape_font_loaded, shape_measure_width,
    shape_text, ShapeResult, ShapedGlyph,
};
pub use style::{FontWeight, TextEffect, TextStyle};
pub use typewriter::{Typewriter, TypewriterEvent};
