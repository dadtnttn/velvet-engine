//! Basic RTL mark handling and display-order helpers.

/// Unicode bidirectional marks / controls we care about for narrative text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiMark {
    /// U+200E LEFT-TO-RIGHT MARK.
    Lrm,
    /// U+200F RIGHT-TO-LEFT MARK.
    Rlm,
    /// U+202A LEFT-TO-RIGHT EMBEDDING.
    Lre,
    /// U+202B RIGHT-TO-LEFT EMBEDDING.
    Rle,
    /// U+202C POP DIRECTIONAL FORMATTING.
    Pdf,
    /// U+2066 LEFT-TO-RIGHT ISOLATE.
    Lri,
    /// U+2067 RIGHT-TO-LEFT ISOLATE.
    Rli,
    /// U+2069 POP DIRECTIONAL ISOLATE.
    Pdi,
}

impl BidiMark {
    /// Codepoint.
    pub fn as_char(self) -> char {
        match self {
            Self::Lrm => '\u{200E}',
            Self::Rlm => '\u{200F}',
            Self::Lre => '\u{202A}',
            Self::Rle => '\u{202B}',
            Self::Pdf => '\u{202C}',
            Self::Lri => '\u{2066}',
            Self::Rli => '\u{2067}',
            Self::Pdi => '\u{2069}',
        }
    }

    /// Parse from char if it is a known mark.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '\u{200E}' => Some(Self::Lrm),
            '\u{200F}' => Some(Self::Rlm),
            '\u{202A}' => Some(Self::Lre),
            '\u{202B}' => Some(Self::Rle),
            '\u{202C}' => Some(Self::Pdf),
            '\u{2066}' => Some(Self::Lri),
            '\u{2067}' => Some(Self::Rli),
            '\u{2069}' => Some(Self::Pdi),
            _ => None,
        }
    }
}

/// Strip bidi control characters for measurement / plain storage.
pub fn strip_bidi_marks(input: &str) -> String {
    input
        .chars()
        .filter(|c| BidiMark::from_char(*c).is_none())
        .collect()
}

/// Whether a character is in a common RTL Unicode block (heuristic, not full UBA).
pub fn is_rtl_char(c: char) -> bool {
    matches!(
        c,
        '\u{0590}'..='\u{05FF}' // Hebrew
        | '\u{0600}'..='\u{06FF}' // Arabic
        | '\u{0700}'..='\u{074F}' // Syriac
        | '\u{0750}'..='\u{077F}'
        | '\u{08A0}'..='\u{08FF}'
        | '\u{FB50}'..='\u{FDFF}'
        | '\u{FE70}'..='\u{FEFF}'
    )
}

/// Detect if string is predominantly RTL by counting strong directional letters.
pub fn is_mostly_rtl(input: &str) -> bool {
    let mut rtl = 0usize;
    let mut ltr = 0usize;
    for c in input.chars() {
        if is_rtl_char(c) {
            rtl += 1;
        } else if c.is_ascii_alphabetic() {
            ltr += 1;
        }
    }
    rtl > ltr
}

/// Reverse display order of grapheme-ish clusters for simple RTL runs.
///
/// This is **not** a full Unicode Bidirectional Algorithm implementation.
/// It reverses the sequence of non-mark characters for single-run RTL UI labels.
pub fn reverse_display_order(input: &str) -> String {
    let cleaned = strip_bidi_marks(input);
    // Reverse by char (good enough for pure RTL letters without combining marks tests).
    cleaned.chars().rev().collect()
}

/// Wrap text with RLM/LRM so mixed snippets render more predictably in LTR UI.
pub fn wrap_for_base_direction(input: &str, base_rtl: bool) -> String {
    let mark = if base_rtl {
        BidiMark::Rlm.as_char()
    } else {
        BidiMark::Lrm.as_char()
    };
    format!("{mark}{input}{mark}")
}

/// Prepare a line for layout: if mostly RTL, reverse display order and tag with RLM.
pub fn prepare_line_for_display(input: &str) -> String {
    if is_mostly_rtl(input) {
        let rev = reverse_display_order(input);
        wrap_for_base_direction(&rev, true)
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_marks() {
        let s = format!("{}hi{}", BidiMark::Lrm.as_char(), BidiMark::Rlm.as_char());
        assert_eq!(strip_bidi_marks(&s), "hi");
    }

    #[test]
    fn hebrew_is_rtl() {
        assert!(is_rtl_char('ש'));
        assert!(is_mostly_rtl("שלום"));
        assert!(!is_mostly_rtl("hello"));
    }

    #[test]
    fn reverse_order() {
        assert_eq!(reverse_display_order("abc"), "cba");
    }

    #[test]
    fn prepare_ltr_unchanged() {
        assert_eq!(prepare_line_for_display("Hello"), "Hello");
    }
}
