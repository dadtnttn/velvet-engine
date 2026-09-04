//! Export spritesheet / PNG / JSON atlas.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::preset::FxPreset;
use crate::sim::{FxSim, SimFrame};

/// Export error.
#[derive(Debug, Error)]
pub enum ExportError {
    /// Io.
    #[error("io: {0}")]
    Io(String),
    /// Image.
    #[error("image: {0}")]
    Image(String),
    /// Frames empty.
    #[error("no frames to export")]
    Empty,
}

/// Atlas metadata (for engine import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasMeta {
    /// Source preset file stem.
    pub name: String,
    /// Dimensions per frame.
    pub frame_width: u32,
    /// Frame height.
    pub frame_height: u32,
    /// Total frames.
    pub frames: u32,
    /// FPS virtual.
    pub fps: u32,
    /// Columns in sheet.
    pub columns: u32,
    /// Rows.
    pub rows: u32,
    /// Sheet width.
    pub sheet_width: u32,
    /// Sheet height.
    pub sheet_height: u32,
    /// Palette id string.
    pub palette: String,
    /// Seed.
    pub seed: u64,
    /// Duration seconds.
    pub duration: f32,
}

/// Encode a single frame to PNG bytes.
pub fn export_png(frame: &SimFrame) -> Result<Vec<u8>, ExportError> {
    let rgba = velvet_image::RgbaImage {
        width: frame.width,
        height: frame.height,
        pixels: frame.pixels.clone(),
    };
    velvet_image::encode(&rgba, velvet_image::ImageEncode::default())
        .map_err(|e| ExportError::Image(e.to_string()))
}

/// Build a spritesheet (all frames tiled) + atlas meta.
/// Columns = ceil(sqrt(frames)), rows = ceil(frames/columns) for roughly square sheet,
/// but we also support `columns = frames` for single-row sheet via param.
pub fn export_spritesheet(
    sim: &FxSim,
    columns: Option<u32>,
) -> Result<(SimFrame, AtlasMeta), ExportError> {
    let preset = &sim.preset;
    let frames = sim.render_all_frames();
    if frames.is_empty() {
        return Err(ExportError::Empty);
    }
    let fw = preset.dimensions.width;
    let fh = preset.dimensions.height;
    let n = frames.len() as u32;
    let cols = columns
        .unwrap_or_else(|| (n as f32).sqrt().ceil() as u32)
        .max(1)
        .min(n);
    let rows = n.div_ceil(cols);
    let sheet_w = fw * cols;
    let sheet_h = fh * rows;
    let mut sheet = SimFrame::new(sheet_w, sheet_h, [0, 0, 0, 0]);
    for (idx, frame) in frames.iter().enumerate() {
        let col = (idx as u32) % cols;
        let row = (idx as u32) / cols;
        let ox = (col * fw) as i32;
        let oy = (row * fh) as i32;
        for y in 0..fh as i32 {
            for x in 0..fw as i32 {
                let src_i = ((y as u32 * fw + x as u32) * 4) as usize;
                if src_i + 3 >= frame.pixels.len() {
                    continue;
                }
                let r = frame.pixels[src_i];
                let g = frame.pixels[src_i + 1];
                let b = frame.pixels[src_i + 2];
                let a = frame.pixels[src_i + 3];
                if a == 0 {
                    continue;
                }
                let dx = ox + x;
                let dy = oy + y;
                let di = ((dy as u32 * sheet_w + dx as u32) * 4) as usize;
                sheet.pixels[di] = r;
                sheet.pixels[di + 1] = g;
                sheet.pixels[di + 2] = b;
                sheet.pixels[di + 3] = a;
            }
        }
    }
    let meta = AtlasMeta {
        name: preset.file_stem(),
        frame_width: fw,
        frame_height: fh,
        frames: n,
        fps: preset.fps,
        columns: cols,
        rows,
        sheet_width: sheet_w,
        sheet_height: sheet_h,
        palette: format!("{:?}", preset.palette),
        seed: preset.seed,
        duration: preset.duration(),
    };
    Ok((sheet, meta))
}

/// Export sheet PNG to path, and atlas JSON sidecar.
pub fn export_spritesheet_to_files(
    sim: &FxSim,
    out_png: &Path,
    columns: Option<u32>,
) -> Result<AtlasMeta, ExportError> {
    let (sheet, meta) = export_spritesheet(sim, columns)?;
    let bytes = export_png(&sheet)?;
    std::fs::write(out_png, &bytes).map_err(|e| ExportError::Io(e.to_string()))?;
    let json = serde_json::to_string_pretty(&meta).map_err(|e| ExportError::Io(e.to_string()))?;
    let json_path = out_png.with_extension("json");
    std::fs::write(&json_path, json.as_bytes()).map_err(|e| ExportError::Io(e.to_string()))?;
    // also write preset json sidecar
    let preset_json = sim
        .preset
        .to_json()
        .map_err(|e| ExportError::Io(e.to_string()))?;
    let preset_path = out_png.with_file_name(format!("{}_preset.json", meta.name));
    std::fs::write(&preset_path, preset_json.as_bytes())
        .map_err(|e| ExportError::Io(e.to_string()))?;
    Ok(meta)
}

/// Export preset JSON alone.
pub fn export_preset_json(preset: &FxPreset, out: &Path) -> Result<(), ExportError> {
    let s = preset
        .to_json()
        .map_err(|e| ExportError::Io(e.to_string()))?;
    std::fs::write(out, s.as_bytes()).map_err(|e| ExportError::Io(e.to_string()))
}

/// Export single frame JSON atlas (for data-driven import).
pub fn export_json_atlas(meta: &AtlasMeta) -> Result<String, ExportError> {
    serde_json::to_string_pretty(meta).map_err(|e| ExportError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::FxPreset;

    #[test]
    fn sheet_dims_reasonable() {
        let sim = FxSim::new(FxPreset::fire());
        let (sheet, meta) = export_spritesheet(&sim, None).unwrap();
        assert_eq!(sheet.width, meta.sheet_width);
        assert!(meta.columns * meta.rows >= meta.frames);
    }

    #[test]
    fn png_roundtrips() {
        let sim = FxSim::new(FxPreset::smoke());
        let frame = sim.render_frame_at(0.1);
        let bytes = export_png(&frame).unwrap();
        assert!(!bytes.is_empty());
        let info = velvet_image::probe(&bytes).unwrap();
        assert_eq!(info.width, frame.width);
    }
}
