//! `velvet image info|convert` — VS2-adjacent author tools for formats/compression.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_image::{decode_rgba, encode, probe, ImageEncode, ImageKind};

/// Print image metadata.
pub fn cmd_image_info(path: PathBuf) -> Result<()> {
    let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let info = probe(&bytes).map_err(|e| anyhow::anyhow!(e))?;
    println!(
        "{} kind={:?} {}x{} alpha={} bytes={}",
        path.display(),
        info.kind,
        info.width,
        info.height,
        info.has_alpha,
        info.bytes
    );
    Ok(())
}

/// Convert / re-encode with quality (compression, not encryption).
pub fn cmd_image_convert(
    input: PathBuf,
    output: PathBuf,
    quality: u8,
    png_level: u8,
) -> Result<()> {
    let bytes = std::fs::read(&input).with_context(|| format!("read {}", input.display()))?;
    let rgba = decode_rgba(&bytes).map_err(|e| anyhow::anyhow!(e))?;
    let kind = output
        .extension()
        .and_then(|e| e.to_str())
        .map(ImageKind::parse)
        .unwrap_or(ImageKind::Png);
    if matches!(kind, ImageKind::Unknown | ImageKind::Svg | ImageKind::WebP) {
        bail!("unsupported output kind for convert (use .png or .jpg)");
    }
    let out = encode(
        &rgba,
        ImageEncode {
            kind,
            quality,
            png_level,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    std::fs::write(&output, &out).with_context(|| format!("write {}", output.display()))?;
    println!(
        "wrote {} ({} bytes, kind={:?}, quality={})",
        output.display(),
        out.len(),
        kind,
        quality
    );
    Ok(())
}
