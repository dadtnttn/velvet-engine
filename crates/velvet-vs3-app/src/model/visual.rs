use anyhow::{anyhow, Result};
use velvet_script_vs3::Value;
#[derive(Debug, Clone, Default)]
pub struct VisualCommandView {
    pub op: String,
    pub layer: i64,
    pub color: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub x2: f32,
    pub y2: f32,
    pub rx: f32,
    pub ry: f32,
    pub radius: f32,
    pub thickness: f32,
    pub asset: String,
    pub hover_asset: String,
    pub pressed_asset: String,
    pub fit: String,
    pub text: String,
    pub font: String,
    pub align: String,
    pub action: String,
    pub icon: String,
    pub frame_width: i64,
    pub frame: i64,
    pub fps: f32,
    pub flip_x: bool,
    pub size: f32,
    pub line_height: f32,
    pub ratio: f32,
    pub stroke: i64,
    pub background: i64,
    pub fill: i64,
    pub source_x: i64,
    pub source_y: i64,
    pub source_w: i64,
    pub source_h: i64,
    pub tile_w: i64,
    pub tile_h: i64,
    pub source_border: i64,
    pub destination_border: i64,
    /// Text width limit in pixels (`text`, `button`); 0 = unlimited.
    pub max_width: f32,
    /// What to do when text exceeds its width: `ellipsis` (default),
    /// `shrink` (scale down, then ellipsis) or `clip`.
    pub overflow: String,
    /// Maximum lines for `text_wrap`; the last one ends in an ellipsis.
    pub max_lines: i64,
}
impl VisualCommandView {
    /// Parse one command map in a single pass over its entries (a frame can
    /// carry thousands of commands, so per-key lookups add up).
    pub fn parse(v: &Value) -> Result<Self> {
        let Value::Map(map) = v else {
            return Err(anyhow!(
                "visual command must be a map, got {}",
                v.type_name()
            ));
        };
        let map = map.borrow();
        let mut out = Self::default();
        let mut has_op = false;
        for (key, value) in map.iter() {
            match key.as_str() {
                "op" => {
                    out.op = value
                        .as_str()
                        .ok_or_else(|| anyhow!("`op` is not a string"))?
                        .to_owned();
                    has_op = true;
                }
                "layer" => out.layer = value.as_i64().unwrap_or_default(),
                "color" => out.color = value.as_i64().unwrap_or_default(),
                "x" => out.x = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "y" => out.y = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "w" => out.w = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "h" => out.h = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "x2" => out.x2 = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "y2" => out.y2 = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "rx" => out.rx = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "ry" => out.ry = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "radius" => out.radius = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "thickness" => out.thickness = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "asset" => out.asset = value.as_str().map(str::to_owned).unwrap_or_default(),
                "hover_asset" => {
                    out.hover_asset = value.as_str().map(str::to_owned).unwrap_or_default()
                }
                "pressed_asset" => {
                    out.pressed_asset = value.as_str().map(str::to_owned).unwrap_or_default()
                }
                "fit" => out.fit = value.as_str().map(str::to_owned).unwrap_or_default(),
                "text" => out.text = value.as_str().map(str::to_owned).unwrap_or_default(),
                "font" => out.font = value.as_str().map(str::to_owned).unwrap_or_default(),
                "align" => out.align = value.as_str().map(str::to_owned).unwrap_or_default(),
                "action" => out.action = value.as_str().map(str::to_owned).unwrap_or_default(),
                "icon" => out.icon = value.as_str().map(str::to_owned).unwrap_or_default(),
                "frame_width" => out.frame_width = value.as_i64().unwrap_or_default(),
                "frame" => out.frame = value.as_i64().unwrap_or_default(),
                "fps" => out.fps = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "flip_x" => out.flip_x = matches!(value, Value::Bool(true)),
                "size" => out.size = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "line_height" => {
                    out.line_height = value.as_f64().map(|n| n as f32).unwrap_or_default()
                }
                "ratio" => out.ratio = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "stroke" => out.stroke = value.as_i64().unwrap_or_default(),
                "background" => out.background = value.as_i64().unwrap_or_default(),
                "fill" => out.fill = value.as_i64().unwrap_or_default(),
                "source_x" => out.source_x = value.as_i64().unwrap_or_default(),
                "source_y" => out.source_y = value.as_i64().unwrap_or_default(),
                "source_w" => out.source_w = value.as_i64().unwrap_or_default(),
                "source_h" => out.source_h = value.as_i64().unwrap_or_default(),
                "tile_w" => out.tile_w = value.as_i64().unwrap_or_default(),
                "tile_h" => out.tile_h = value.as_i64().unwrap_or_default(),
                "source_border" => out.source_border = value.as_i64().unwrap_or_default(),
                "destination_border" => out.destination_border = value.as_i64().unwrap_or_default(),
                "max_width" => out.max_width = value.as_f64().map(|n| n as f32).unwrap_or_default(),
                "overflow" => out.overflow = value.as_str().map(str::to_owned).unwrap_or_default(),
                "max_lines" => out.max_lines = value.as_i64().unwrap_or_default(),
                _ => {}
            }
        }
        anyhow::ensure!(has_op, "missing VS3 field `op`");
        Ok(out)
    }
}
