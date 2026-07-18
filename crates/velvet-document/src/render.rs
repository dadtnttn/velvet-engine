//! Re-emit a [`Document`] to source text.

use crate::model::{Document, RegionKind};

/// Render document regions in order. Marked regions get marker comments.
pub fn render_document(doc: &Document) -> String {
    let mut out = String::new();
    for region in &doc.regions {
        if region.marked {
            out.push_str(&format!(
                "// @{}{}\n",
                region.kind.tag(),
                if region.id.as_str().is_empty() {
                    String::new()
                } else {
                    format!(" id={}", region.id.as_str())
                }
            ));
            // body
            if !region.body.is_empty() {
                out.push_str(&region.body);
                if !region.body.ends_with('\n') {
                    out.push('\n');
                }
            }
            // Advanced/protected always close with @end for stability
            if matches!(
                region.kind,
                RegionKind::Advanced | RegionKind::Protected | RegionKind::Visual
            ) {
                out.push_str("// @end\n");
            }
        } else {
            out.push_str(&region.body);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    #[test]
    fn roundtrip_identity_without_patch() {
        let src = r#"// header
// @visual id=x
text: "hi"
// @end
// footer
"#;
        let doc = parse_document(src).unwrap();
        let out = render_document(&doc);
        let doc2 = parse_document(&out).unwrap();
        assert_eq!(doc.regions.len(), doc2.regions.len());
        assert!(out.contains("text: \"hi\""));
        assert!(out.contains("// header") || out.contains("header"));
    }
}
