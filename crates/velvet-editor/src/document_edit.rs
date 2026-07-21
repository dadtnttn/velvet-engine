//! Visual document editing helpers (Phase 2 round-trip).

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use velvet_document::{
    apply_visual_patch, parse_document, render_document, PropertyValue, RegionId, UiDesigner,
    VisualPatch, VisualPatchOp,
};
// drag_region_on_disk uses velvet_document::drag_visual_region

/// Set a visual property on a marked region in a `.vel` file on disk.
///
/// Advanced and protected regions with the same or other ids are preserved.
pub fn set_visual_property(path: &Path, region_id: &str, key: &str, value: &str) -> Result<String> {
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut doc = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    let prop_value = if value.starts_with('"') {
        PropertyValue::String(value.trim_matches('"').to_string())
    } else if value.starts_with('(') || value.parse::<f64>().is_ok() {
        PropertyValue::Raw(value.to_string())
    } else {
        // Treat bare words as strings for UX (`Iniciar` → "Iniciar")
        PropertyValue::String(value.to_string())
    };
    apply_visual_patch(
        &mut doc,
        &VisualPatch {
            region_id: RegionId::new(region_id),
            ops: vec![VisualPatchOp::SetProperty {
                key: key.into(),
                value: prop_value,
            }],
        },
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let out = render_document(&doc);
    fs::write(path, &out).with_context(|| format!("write {}", path.display()))?;
    Ok(out)
}

/// List region ids of a given kind in a file.
pub fn list_regions(path: &Path) -> Result<Vec<(String, String)>> {
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let doc = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(doc
        .regions
        .iter()
        .filter(|r| r.marked)
        .map(|r| (r.kind.tag().to_string(), r.id.as_str().to_string()))
        .collect())
}

/// Ensure path exists and is a file.
pub fn require_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("not a file: {}", path.display());
    }
    Ok(())
}

/// Drag a visual region on disk by delta (Studio canvas / CLI shared path).
///
/// Returns the new position string and leaves advanced/protected regions intact.
pub fn drag_region_on_disk(path: &Path, region_id: &str, dx: f32, dy: f32) -> Result<String> {
    require_file(path)?;
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut doc = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    let rect = velvet_document::drag_visual_region(&mut doc, region_id, dx, dy)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let out = render_document(&doc);
    fs::write(path, &out).with_context(|| format!("write {}", path.display()))?;
    Ok(velvet_document::format_position(rect.pos))
}

/// Open a UI designer session, apply text/position edits, write back.
///
/// This is the real simplified-mode path: no hand-editing required.
pub fn design_set_button(
    path: &Path,
    region_id: &str,
    text: Option<&str>,
    position: Option<&str>,
) -> Result<()> {
    require_file(path)?;
    let source = fs::read_to_string(path)?;
    let mut designer = UiDesigner::open(source).map_err(|e| anyhow::anyhow!("{e}"))?;
    if let Some(t) = text {
        designer
            .set_text(region_id, t)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    if let Some(p) = position {
        designer
            .set_position(region_id, p)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    fs::write(path, designer.save_source())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn disk_patch_keeps_advanced() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("menu.vel");
        let mut f = fs::File::create(&p).unwrap();
        writeln!(
            f,
            r#"// @visual id=b
text: "A"
// @advanced id=b
on_pressed {{ foo() }}
// @end
"#
        )
        .unwrap();
        set_visual_property(&p, "b", "text", "B").unwrap();
        let out = fs::read_to_string(&p).unwrap();
        assert!(out.contains("foo()"));
        assert!(out.contains("text: \"B\""));
    }
}
