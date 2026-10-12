use super::canvas::{alpha_table, Canvas};
use super::fonts::FontChain;
use super::layout;

impl Canvas {
    pub fn text_line(
        &mut self,
        chain: &FontChain,
        text: &str,
        x: i32,
        baseline: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        let (glyphs, width) = layout::layout(chain, text, size);
        let alpha = alpha_table(self.opacity());
        for g in &glyphs {
            self.glyph(
                &g.font,
                g.glyph,
                size,
                x as f32 + g.x,
                baseline - g.dy.round() as i32,
                color,
                &alpha,
            );
        }
        (x as f32 + width).round() as i32
    }
    /// `text_line` with an outline of `radius` pixels in `stroke` behind
    /// every glyph.
    #[allow(clippy::too_many_arguments)]
    pub fn text_line_outlined(
        &mut self,
        chain: &FontChain,
        text: &str,
        x: i32,
        baseline: i32,
        size: f32,
        color: u32,
        stroke: u32,
        radius: f32,
    ) -> i32 {
        let (glyphs, _) = layout::layout(chain, text, size);
        let alpha = alpha_table(self.opacity());
        for g in &glyphs {
            self.glyph_outline(
                &g.font,
                g.glyph,
                size,
                x as f32 + g.x,
                baseline - g.dy.round() as i32,
                stroke,
                radius,
                &alpha,
            );
        }
        self.text_line(chain, text, x, baseline, size, color)
    }
    pub fn measure_text(chain: &FontChain, text: &str, size: f32) -> f32 {
        layout::measure(chain, text, size)
    }
    /// Wrapped text; returns the y below the last line. `max_lines > 0`
    /// truncates with an ellipsis.
    #[allow(clippy::too_many_arguments)]
    pub fn text_wrapped(
        &mut self,
        chain: &FontChain,
        text: &str,
        x: i32,
        y: i32,
        max_width: i32,
        size: f32,
        line_height: i32,
        color: u32,
        max_lines: usize,
    ) -> i32 {
        let lines = wrap_text(chain, text, size, max_width as f32, max_lines);
        let mut cy = y;
        for line in &lines {
            if !line.is_empty() {
                self.text_line(chain, line, x, cy + size as i32, size, color);
            }
            cy += line_height;
        }
        cy
    }
    pub fn centered_text(
        &mut self,
        chain: &FontChain,
        text: &str,
        cx: i32,
        baseline: i32,
        size: f32,
        color: u32,
    ) {
        let w = Self::measure_text(chain, text, size).round() as i32;
        self.text_line(chain, text, cx - w / 2, baseline, size, color);
    }
}

/// Character used to mark truncated text (falls back to `...`).
fn ellipsis(chain: &FontChain) -> &'static str {
    if chain.primary.font.lookup_glyph_index('\u{2026}') != 0 {
        "\u{2026}"
    } else {
        "..."
    }
}

/// Longest prefix of `text` that fits `max_width` once `suffix` is appended.
fn truncate_to(chain: &FontChain, text: &str, size: f32, max_width: f32, suffix: &str) -> String {
    let suffix_width = Canvas::measure_text(chain, suffix, size);
    let mut out = String::new();
    for ch in text.chars() {
        let mut next = out.clone();
        next.push(ch);
        if Canvas::measure_text(chain, &next, size) + suffix_width > max_width {
            break;
        }
        out = next;
    }
    let trimmed = out.trim_end().len();
    out.truncate(trimmed);
    out.push_str(suffix);
    out
}

/// Fit one line into `max_width` according to `overflow` (`ellipsis`,
/// `shrink`, `clip`). Returns the text to draw and the font size to use.
pub fn fit_line(
    chain: &FontChain,
    text: &str,
    size: f32,
    max_width: f32,
    overflow: &str,
) -> (String, f32) {
    if max_width <= 0.0 || Canvas::measure_text(chain, text, size) <= max_width {
        return (text.to_owned(), size);
    }
    let mut size = size;
    if overflow == "shrink" {
        let measured = Canvas::measure_text(chain, text, size);
        let scaled = (size * max_width / measured).floor().max(8.0);
        size = scaled.min(size);
        if Canvas::measure_text(chain, text, size) <= max_width {
            return (text.to_owned(), size);
        }
    }
    let suffix = if overflow == "clip" {
        ""
    } else {
        ellipsis(chain)
    };
    (truncate_to(chain, text, size, max_width, suffix), size)
}

/// Greedy word wrap (long words break by character). With `max_lines > 0`
/// the output is cut to that many lines, the last one ending in an ellipsis.
pub fn wrap_text(
    chain: &FontChain,
    text: &str,
    size: f32,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = velvet_script_vs3::wrap_lines(text, f64::from(max_width), |piece| {
        f64::from(Canvas::measure_text(chain, piece, size))
    });
    if max_lines > 0 && lines.len() > max_lines {
        lines.truncate(max_lines);
        let last = lines.pop().unwrap_or_default();
        let mark = ellipsis(chain);
        let candidate = format!("{last}{mark}");
        lines.push(
            if Canvas::measure_text(chain, &candidate, size) <= max_width {
                candidate
            } else {
                truncate_to(chain, &last, size, max_width, mark)
            },
        );
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> Option<FontChain> {
        let candidates = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ];
        candidates.iter().find_map(|path| {
            let bytes = std::fs::read(path).ok()?;
            super::super::fonts::test_chain(bytes)
        })
    }

    #[test]
    fn fit_line_ellipsizes_shrinks_and_clips() {
        let Some(font) = font() else {
            return;
        };
        let text = "Una frase demasiado larga para el espacio";
        let (fitted, size) = fit_line(&font, text, 20.0, 120.0, "ellipsis");
        assert_eq!(size, 20.0);
        assert!(fitted.ends_with('\u{2026}') || fitted.ends_with("..."));
        assert!(Canvas::measure_text(&font, &fitted, size) <= 120.0);
        let (shrunk, smaller) = fit_line(&font, "Bastante largo", 20.0, 100.0, "shrink");
        assert!(smaller < 20.0);
        assert!(Canvas::measure_text(&font, &shrunk, smaller) <= 100.0);
        let (clipped, _) = fit_line(&font, text, 20.0, 120.0, "clip");
        assert!(text.starts_with(clipped.as_str()));
        let (short, _) = fit_line(&font, "ok", 20.0, 120.0, "ellipsis");
        assert_eq!(short, "ok");
    }

    #[test]
    fn wrap_text_respects_width_and_max_lines() {
        let Some(font) = font() else {
            return;
        };
        let text = "uno dos tres cuatro cinco seis siete ocho nueve diez once doce";
        let lines = wrap_text(&font, text, 16.0, 90.0, 0);
        assert!(lines.len() > 2);
        for line in &lines {
            assert!(Canvas::measure_text(&font, line, 16.0) <= 90.0, "{line}");
        }
        let capped = wrap_text(&font, text, 16.0, 90.0, 2);
        assert_eq!(capped.len(), 2);
        let long = wrap_text(&font, "supercalifragilisticoespialidoso", 16.0, 60.0, 0);
        assert!(long.len() > 1);
    }
}
