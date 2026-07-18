//! Parse source text into a [`Document`].

use crate::model::{
    Document, DocumentError, PropertyValue, Region, RegionId, RegionKind, VisualProperty,
};

/// Parse a full source file into ordered regions.
pub fn parse_document(source: &str) -> Result<Document, DocumentError> {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut regions: Vec<Region> = Vec::new();
    let mut external_buf: Vec<String> = Vec::new();
    let mut i = 0usize;

    let flush_external = |buf: &mut Vec<String>, regions: &mut Vec<Region>| {
        if buf.is_empty() {
            return;
        }
        let body = buf.join("");
        buf.clear();
        if body.chars().all(|c| c.is_whitespace()) && body.is_empty() {
            return;
        }
        regions.push(Region {
            kind: RegionKind::External,
            id: RegionId::new(""),
            body,
            properties: Vec::new(),
            raw_lines: Vec::new(),
            marked: false,
        });
    };

    while i < lines.len() {
        let line = lines[i];
        if let Some((kind, id)) = parse_marker(line) {
            flush_external(&mut external_buf, &mut regions);
            i += 1;
            let mut body_lines: Vec<String> = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if is_end_marker(l) {
                    i += 1;
                    break;
                }
                if parse_marker(l).is_some() {
                    // Nested open: close current implicitly.
                    break;
                }
                body_lines.push(l.to_string());
                i += 1;
            }
            let body = body_lines.join("");
            let (properties, raw_lines) = if kind == RegionKind::Visual {
                parse_visual_body(&body)
            } else {
                (Vec::new(), Vec::new())
            };
            regions.push(Region {
                kind,
                id: RegionId::new(id),
                body,
                properties,
                raw_lines,
                marked: true,
            });
        } else {
            external_buf.push(line.to_string());
            i += 1;
        }
    }
    flush_external(&mut external_buf, &mut regions);

    Ok(Document {
        regions,
        path: None,
    })
}

fn parse_marker(line: &str) -> Option<(RegionKind, String)> {
    let t = line.trim();
    // // @visual id=foo  OR  // @visual
    let rest = t.strip_prefix("//")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('@')?;
    let (tag, after) = rest
        .split_once(|c: char| c.is_whitespace())
        .unwrap_or((rest, ""));
    let kind = match tag {
        "visual" => RegionKind::Visual,
        "advanced" => RegionKind::Advanced,
        "protected" => RegionKind::Protected,
        _ => return None,
    };
    let id = parse_id_attr(after).unwrap_or_default();
    Some((kind, id))
}

fn parse_id_attr(s: &str) -> Option<String> {
    for part in s.split_whitespace() {
        if let Some(v) = part.strip_prefix("id=") {
            return Some(v.trim_matches('"').to_string());
        }
    }
    None
}

fn is_end_marker(line: &str) -> bool {
    let t = line.trim();
    t == "// @end" || t.starts_with("// @end ")
}

fn parse_visual_body(body: &str) -> (Vec<VisualProperty>, Vec<String>) {
    let mut props = Vec::new();
    let mut raw = Vec::new();
    for line in body.lines() {
        let indent: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            raw.push(line.to_string());
            continue;
        }
        // Strip trailing comment
        let (code, comment) = split_trailing_comment(trimmed);
        if let Some((key, value)) = code.split_once(':') {
            let key = key.trim();
            // Skip structural lines like `button start {` — no pure identifier keys with braces
            if key.contains('{') || key.contains('}') || key.is_empty() {
                raw.push(line.to_string());
                continue;
            }
            // Function-like blocks `on_pressed {` handled as raw
            if value.trim().starts_with('{') {
                raw.push(line.to_string());
                continue;
            }
            let value = parse_value(value.trim());
            props.push(VisualProperty {
                key: key.to_string(),
                value,
                indent,
                trailing_comment: comment,
            });
        } else {
            raw.push(line.to_string());
        }
    }
    (props, raw)
}

fn split_trailing_comment(s: &str) -> (&str, Option<String>) {
    // naive: // outside quotes
    let mut in_str = false;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_str = !in_str;
        }
        if !in_str && c == '/' && bytes[i + 1] == b'/' {
            let code = s[..i].trim_end();
            let comment = s[i..].to_string();
            return (code, Some(comment));
        }
        i += 1;
    }
    (s, None)
}

fn parse_value(s: &str) -> PropertyValue {
    let s = s.trim().trim_end_matches(',');
    if let Some(inner) = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        PropertyValue::String(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
    } else {
        PropertyValue::Raw(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_parse() {
        let (k, id) = parse_marker("  // @visual id=button.start\n").unwrap();
        assert_eq!(k, RegionKind::Visual);
        assert_eq!(id, "button.start");
    }
}
