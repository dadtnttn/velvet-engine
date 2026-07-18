//! Document region / visual patch commands (Phase 2 round-trip).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use velvet_document::{
    apply_visual_patch, parse_document, render_document, PropertyValue, RegionId, VisualPatch,
    VisualPatchOp,
};

/// List marked regions in a file.
pub fn cmd_document_regions(path: PathBuf) -> Result<()> {
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let doc = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    for r in doc.regions.iter().filter(|r| r.marked) {
        println!("{}\t{}", r.kind.tag(), r.id.as_str());
    }
    Ok(())
}

/// Patch a visual property in-place.
pub fn cmd_document_patch(path: PathBuf, region: String, key: String, value: String) -> Result<()> {
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut doc = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    let prop = if value.starts_with('(') || value.parse::<f64>().is_ok() {
        PropertyValue::Raw(value)
    } else {
        PropertyValue::String(value.trim_matches('"').to_string())
    };
    apply_visual_patch(
        &mut doc,
        &VisualPatch {
            region_id: RegionId::new(region.clone()),
            ops: vec![VisualPatchOp::SetProperty {
                key: key.clone(),
                value: prop,
            }],
        },
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let out = render_document(&doc);
    fs::write(&path, out).with_context(|| format!("write {}", path.display()))?;
    println!("ok: set {region}.{key} in {}", path.display());
    Ok(())
}
