//! Text prelude.

pub use crate::icon::{IconGlyphMap, IconSpan};
pub use crate::layout::{AlignedLine, TextAlign, TextLayout};
pub use crate::markup::{parse_rich_text, RichSpan, RichText};
pub use crate::measure::{measure_width, TextMetrics};
pub use crate::rtl::{is_mostly_rtl, reverse_display_order};
pub use crate::ruby::RubySpan;
pub use crate::style::{FontWeight, TextEffect, TextStyle};
pub use crate::typewriter::{Typewriter, TypewriterEvent};
