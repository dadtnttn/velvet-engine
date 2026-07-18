//! Visual patches applied without touching advanced/protected regions.

use crate::model::{Document, DocumentError, PropertyValue, RegionKind, VisualProperty};

/// A batch of visual edits targeting one region.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualPatch {
    /// Target visual region id.
    pub region_id: crate::model::RegionId,
    /// Operations.
    pub ops: Vec<VisualPatchOp>,
}

/// One visual mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualPatchOp {
    /// Set or insert a property.
    SetProperty {
        /// Key.
        key: String,
        /// Value.
        value: PropertyValue,
    },
    /// Remove a property key if present.
    RemoveProperty {
        /// Key.
        key: String,
    },
}

/// Apply a visual patch. Advanced/protected regions are never rewritten.
pub fn apply_visual_patch(doc: &mut Document, patch: &VisualPatch) -> Result<(), DocumentError> {
    let id = patch.region_id.as_str();
    // Ensure id is visual; if only advanced/protected exists, error.
    if let Some(r) = doc.regions.iter().find(|r| r.id.as_str() == id) {
        if r.kind != RegionKind::Visual {
            // Look for a visual twin
            if doc.find(RegionKind::Visual, id).is_none() {
                return Err(DocumentError::RegionNotVisual {
                    id: id.into(),
                    kind: r.kind,
                });
            }
        }
    } else {
        return Err(DocumentError::RegionNotFound(id.into()));
    }

    let region = doc
        .find_mut(RegionKind::Visual, id)
        .ok_or_else(|| DocumentError::RegionNotFound(id.into()))?;

    if region.kind != RegionKind::Visual {
        return Err(DocumentError::RegionNotVisual {
            id: id.into(),
            kind: region.kind,
        });
    }

    for op in &patch.ops {
        match op {
            VisualPatchOp::SetProperty { key, value } => {
                if let Some(p) = region.properties.iter_mut().find(|p| p.key == *key) {
                    p.value = value.clone();
                } else {
                    region.properties.push(VisualProperty {
                        key: key.clone(),
                        value: value.clone(),
                        indent: "        ".into(),
                        trailing_comment: None,
                    });
                }
            }
            VisualPatchOp::RemoveProperty { key } => {
                region.properties.retain(|p| p.key != *key);
            }
        }
    }

    // Rebuild visual body from properties + non-property raw lines
    region.body = rebuild_visual_body(region);
    Ok(())
}

fn rebuild_visual_body(region: &crate::model::Region) -> String {
    let mut out = String::new();
    // Emit structural raw lines first (opening braces etc.), then properties,
    // then remaining raw — keep simple: properties then raw_lines that are not props.
    for p in &region.properties {
        out.push_str(&p.indent);
        out.push_str(&p.key);
        out.push_str(": ");
        out.push_str(&p.value.render());
        if let Some(c) = &p.trailing_comment {
            out.push(' ');
            out.push_str(c);
        }
        out.push('\n');
    }
    for line in &region.raw_lines {
        // Avoid duplicating pure property lines already emitted
        let t = line.trim();
        if let Some((k, _)) = t.split_once(':') {
            let k = k.trim();
            if region.properties.iter().any(|p| p.key == k) {
                continue;
            }
        }
        out.push_str(line);
        if !line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RegionId;
    use crate::parse::parse_document;

    #[test]
    fn set_inserts_new_prop() {
        let src = r#"
// @visual id=b
button b {
    text: "A"
}
// @end
"#;
        let mut doc = parse_document(src).unwrap();
        apply_visual_patch(
            &mut doc,
            &VisualPatch {
                region_id: RegionId::new("b"),
                ops: vec![VisualPatchOp::SetProperty {
                    key: "color".into(),
                    value: PropertyValue::String("#fff".into()),
                }],
            },
        )
        .unwrap();
        let body = &doc.find(RegionKind::Visual, "b").unwrap().body;
        assert!(body.contains("color: \"#fff\""));
        assert!(body.contains("text: \"A\""));
    }
}
