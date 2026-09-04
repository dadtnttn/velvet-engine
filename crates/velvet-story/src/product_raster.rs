//! CPU rasterizer: [`ProductPaintList`] → ARGB8888 buffer (softbuffer-friendly).
//!
//! Used by windowed product hosts so the same paint list as `velvet play` / GPU path
//! drives on-screen pixels without a separate ad-hoc UI stack.

use crate::product_paint::{ProductPaintCmd, ProductPaintList};

/// Pack R,G,B into softbuffer-style 0x00RRGGBB.
#[inline]
pub fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

#[inline]
fn f32_to_u8(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0) as u8
}

/// Fill a rectangle in the output buffer (clipped).
pub fn fill_rect(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let x0 = x0.clamp(0, out_w as i32);
    let y0 = y0.clamp(0, out_h as i32);
    let x1 = x1.clamp(0, out_w as i32);
    let y1 = y1.clamp(0, out_h as i32);
    for y in y0..y1 {
        let row = (y as u32 * out_w) as usize;
        for x in x0..x1 {
            buf[row + x as usize] = color;
        }
    }
}

/// 5×7 glyph rows (uppercase fold).
fn glyph(c: char) -> [u8; 7] {
    let c = match c {
        'á' | 'à' | 'ä' | 'â' | 'Á' => 'a',
        'é' | 'è' | 'ë' | 'ê' | 'É' => 'e',
        'í' | 'ì' | 'ï' | 'î' | 'Í' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'Ó' => 'o',
        'ú' | 'ù' | 'ü' | 'û' | 'Ú' => 'u',
        'ñ' | 'Ñ' => 'n',
        '¿' => '?',
        '¡' => '!',
        '…' => '.',
        '—' | '–' => '-',
        '“' | '”' | '„' => '"',
        '‘' | '’' => '\'',
        _ => c,
    };
    match c.to_ascii_uppercase() {
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        '3' => [0x1F, 0x01, 0x02, 0x06, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0C, 0x04, 0x08],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        ':' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00],
        ';' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x04, 0x08],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '\'' => [0x0C, 0x0C, 0x08, 0x00, 0x00, 0x00, 0x00],
        '"' => [0x1B, 0x1B, 0x12, 0x00, 0x00, 0x00, 0x00],
        '(' | '[' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' | ']' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        '>' => [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08],
        '<' => [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '=' => [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00],
        '%' => [0x19, 0x1A, 0x02, 0x04, 0x08, 0x0B, 0x13],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        '#' => [0x0A, 0x1F, 0x0A, 0x0A, 0x1F, 0x0A, 0x00],
        '*' => [0x00, 0x15, 0x0E, 0x1F, 0x0E, 0x15, 0x00],
        '|' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        _ => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F],
    }
}

fn draw_char(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    x: i32,
    y: i32,
    c: char,
    color: u32,
    scale: i32,
) {
    let rows = glyph(c);
    let s = scale.max(1);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                fill_rect(
                    buf,
                    out_w,
                    out_h,
                    x + col * s,
                    y + row as i32 * s,
                    x + col * s + s,
                    y + row as i32 * s + s,
                    color,
                );
            }
        }
    }
}

/// Parse `#rrggbb` / `#rgb` / a few named colors into RGB.
pub fn parse_text_color(spec: &str) -> Option<(u8, u8, u8)> {
    let s = spec.trim().to_ascii_lowercase();
    let had_hash = s.starts_with('#');
    let s = s.strip_prefix('#').unwrap_or(&s);
    let is_hex = |t: &str| t.chars().all(|c| c.is_ascii_hexdigit());
    if had_hash && s.len() == 6 && is_hex(s) {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        return Some((r, g, b));
    }
    if had_hash && s.len() == 3 && is_hex(s) {
        let r = u8::from_str_radix(&s[0..1], 16).ok()?;
        let g = u8::from_str_radix(&s[1..2], 16).ok()?;
        let b = u8::from_str_radix(&s[2..3], 16).ok()?;
        return Some((r * 17, g * 17, b * 17));
    }
    match s {
        "red" => Some((255, 70, 70)),
        "green" => Some((90, 220, 90)),
        "blue" => Some((110, 170, 255)),
        "yellow" => Some((255, 225, 90)),
        "white" => Some((255, 255, 255)),
        "black" => Some((0, 0, 0)),
        "gray" | "grey" => Some((150, 150, 160)),
        "pink" => Some((255, 120, 170)),
        _ => None,
    }
}

/// Draw rich markup (`{b}{i}{color=..}{ruby=..}`) with per-span color and
/// furigana above ruby spans. Unknown glyphs (emoji) use the box fallback
/// from the internal glyph atlas. Returns width px drawn.
pub fn draw_text_rich(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    x: i32,
    y: i32,
    markup: &str,
    base_color: u32,
    scale: i32,
) -> i32 {
    let line = crate::rich_text::parse_rich_line(markup);
    let s = scale.max(1);
    let adv = 6 * s;
    let mut cx = x;
    let start = x;
    for span in &line.spans {
        let col = span
            .color
            .as_deref()
            .and_then(parse_text_color)
            .map(|(r, g, b)| pack_rgb(r, g, b))
            .unwrap_or(base_color);
        // ruby reading above the span (smaller, dimmed toward white)
        if let Some(ruby) = &span.ruby {
            let shrink = (s - 1).max(1);
            let mut rx = cx;
            for ch in ruby.chars() {
                draw_char(buf, out_w, out_h, rx, y - 8 * shrink, ch, col, shrink);
                rx += 6 * shrink;
                if rx > out_w as i32 - 8 {
                    break;
                }
            }
        }
        for ch in span.text.chars() {
            if ch == '\n' {
                continue;
            }
            if span.style.italic {
                // slant: top rows shift right (5x7 hosts can't do TTF italics)
                draw_char_slanted(buf, out_w, out_h, cx, y, ch, col, s);
            } else {
                draw_char(buf, out_w, out_h, cx, y, ch, col, s);
            }
            if span.style.bold {
                // bold = second strike one px right
                draw_char(buf, out_w, out_h, cx + 1, y, ch, col, s);
            }
            cx += adv;
            if cx > out_w as i32 - 8 {
                break;
            }
        }
    }
    cx - start
}

/// Slanted glyph draw for `{i}` (top rows shift right).
fn draw_char_slanted(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    x: i32,
    y: i32,
    c: char,
    color: u32,
    scale: i32,
) {
    let rows = glyph(c);
    let s = scale.max(1);
    for (row, bits) in rows.iter().enumerate() {
        let slant = ((6 - row as i32) / 3).max(0);
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                fill_rect(
                    buf,
                    out_w,
                    out_h,
                    x + col * s + slant,
                    y + row as i32 * s,
                    x + col * s + slant + s,
                    y + row as i32 * s + s,
                    color,
                );
            }
        }
    }
}

/// Draw a single line of text (no wrap).
pub fn draw_text_line(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    mut x: i32,
    y: i32,
    text: &str,
    color: u32,
    scale: i32,
) {
    let advance = 6 * scale.max(1);
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        draw_char(buf, out_w, out_h, x, y, ch, color, scale);
        x += advance;
        if x > out_w as i32 - 8 {
            break;
        }
    }
}

/// Draw wrapped text within max_w pixels.
pub fn draw_text_wrapped(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    x0: i32,
    y0: i32,
    max_w: i32,
    text: &str,
    color: u32,
    scale: i32,
) {
    let advance = 6 * scale.max(1);
    let line_h = 9 * scale.max(1);
    let max_chars = (max_w / advance).max(4) as usize;
    let mut y = y0;
    let mut line = String::new();
    for word in text.split_whitespace() {
        let trial = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if trial.chars().count() > max_chars {
            if !line.is_empty() {
                draw_text_line(buf, out_w, out_h, x0, y, &line, color, scale);
                y += line_h;
                line.clear();
            }
            let mut rest = word;
            while rest.chars().count() > max_chars {
                let take: String = rest.chars().take(max_chars).collect();
                draw_text_line(buf, out_w, out_h, x0, y, &take, color, scale);
                y += line_h;
                rest = rest.get(take.len()..).unwrap_or("");
            }
            line = rest.to_string();
        } else {
            line = trial;
        }
        if y > out_h as i32 - 16 {
            break;
        }
    }
    if !line.is_empty() {
        draw_text_line(buf, out_w, out_h, x0, y, &line, color, scale);
    }
}

/// Rasterize a product paint list into an ARGB buffer (virtual → output scale).
///
/// Transparent quads (alpha ≈ 0) are skipped. Opaque/semi-opaque quads overwrite
/// (no full blend for host simplicity).
pub fn rasterize_product_paint(list: &ProductPaintList, buf: &mut [u32], out_w: u32, out_h: u32) {
    let need = (out_w as usize).saturating_mul(out_h as usize);
    assert!(
        buf.len() >= need,
        "buffer too small: len={} need={need} ({out_w}x{out_h})",
        buf.len()
    );
    // clear only the used region
    for p in buf.iter_mut().take(need) {
        *p = pack_rgb(8, 6, 14);
    }

    let sx = out_w as f32 / list.virtual_w.max(1.0);
    let sy = out_h as f32 / list.virtual_h.max(1.0);

    // Stable z order
    let mut cmds: Vec<&ProductPaintCmd> = list.commands.iter().collect();
    cmds.sort_by(|a, b| {
        let za = match a {
            ProductPaintCmd::Quad { z, .. } | ProductPaintCmd::Text { z, .. } => *z,
        };
        let zb = match b {
            ProductPaintCmd::Quad { z, .. } | ProductPaintCmd::Text { z, .. } => *z,
        };
        za.partial_cmp(&zb).unwrap_or(std::cmp::Ordering::Equal)
    });

    for c in cmds {
        match c {
            ProductPaintCmd::Quad {
                x, y, w, h, color, ..
            } => {
                if *w <= 0.0 || *h <= 0.0 || color[3] < 0.02 {
                    continue;
                }
                let r = f32_to_u8(color[0]);
                let g = f32_to_u8(color[1]);
                let b = f32_to_u8(color[2]);
                let px0 = (*x * sx) as i32;
                let py0 = (*y * sy) as i32;
                let px1 = ((*x + *w) * sx) as i32;
                let py1 = ((*y + *h) * sy) as i32;
                fill_rect(buf, out_w, out_h, px0, py0, px1, py1, pack_rgb(r, g, b));
            }
            ProductPaintCmd::Text {
                x,
                y,
                text,
                size,
                color,
                width,
                ..
            } => {
                if text.is_empty() {
                    continue;
                }
                let r = f32_to_u8(color[0]);
                let g = f32_to_u8(color[1]);
                let b = f32_to_u8(color[2]);
                let scale = ((*size * sy) / 8.0).round().max(1.0) as i32;
                let px = (*x * sx) as i32;
                let py = (*y * sy) as i32;
                let max_w = (*width * sx).max(40.0) as i32;
                draw_text_wrapped(
                    buf,
                    out_w,
                    out_h,
                    px,
                    py,
                    max_w,
                    text,
                    pack_rgb(r, g, b),
                    scale,
                );
            }
        }
    }
}

/// Count non-background pixels (any not equal to clear color).
pub fn count_painted_pixels(buf: &[u32], clear: u32) -> usize {
    buf.iter().filter(|&&p| p != clear).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::product::VnSession;
    use crate::product_paint::paint_product_session;
    use crate::runtime::{StoryPlayer, StoryWait};

    #[test]
    fn ui_symbols_have_distinct_glyphs() {
        for symbol in ['<', '+', '=', '%', '_', '#', '*'] {
            assert_ne!(glyph(symbol), glyph('@'), "{symbol} used fallback glyph");
        }
    }

    #[test]
    fn rich_text_paints_color_and_ruby() {
        let ww = 320u32;
        let wh = 60u32;
        let mut buf = vec![pack_rgb(8, 6, 14); (ww * wh) as usize];
        let w = draw_text_rich(
            &mut buf,
            ww,
            wh,
            10,
            20,
            "Hi {b}bold{/b} {color=#ff0000}red{ruby=furi}X{/ruby}",
            pack_rgb(220, 220, 230),
            2,
        );
        assert!(w > 40, "rich line should advance, got {w}");
        let painted = count_painted_pixels(&buf, pack_rgb(8, 6, 14));
        assert!(painted > 60, "rich spans should paint, got {painted}");
        // red span present
        let red = pack_rgb(255, 0, 0);
        assert!(buf.contains(&red), "color span should use exact red");
        assert_eq!(parse_text_color("#0f0"), Some((0, 255, 0)));
        assert_eq!(parse_text_color("red"), Some((255, 70, 70)));
        assert!(parse_text_color("nope").is_none());
    }

    #[test]
    fn rasterize_live_session_paints_say_panel_pixels() {
        let src = r#"
character hero { name: "Hero" }
scene main {
    background "bg/station"
    show hero at left
    hero "Raster path hello."
    choice {
        "A" { jump end }
        "B" { jump end }
    }
}
scene end { "Ending: Raster" }
"#;
        let program = load_program_from_source(src, Some("raster.vel"), "R").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 20 {
            session.advance();
            g += 1;
        }
        let list = paint_product_session(&session);
        assert!(list.has_say_geometry());
        assert!(
            list.commands
                .iter()
                .any(|c| matches!(c, ProductPaintCmd::Quad { id, .. } if id == "background")),
            "background cmd required"
        );
        assert!(
            list.commands.iter().any(
                |c| matches!(c, ProductPaintCmd::Quad { id, .. } if id.starts_with("sprite_"))
            ),
            "sprite stand from presentation"
        );

        let ww = 320u32;
        let wh = 180u32;
        let mut buf = vec![0u32; (ww * wh) as usize];
        rasterize_product_paint(&list, &mut buf, ww, wh);
        let painted = count_painted_pixels(&buf, pack_rgb(8, 6, 14));
        assert!(
            painted > 500,
            "expected substantial paint from product path, got {painted}"
        );

        session.say.reveal_all();
        session.advance();
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Choice) && g < 10 {
            session.advance();
            g += 1;
        }
        let list2 = paint_product_session(&session);
        assert!(list2.has_choice_geometry());
        rasterize_product_paint(&list2, &mut buf, ww, wh);
        assert!(list2.has_choice_geometry());
    }
}
