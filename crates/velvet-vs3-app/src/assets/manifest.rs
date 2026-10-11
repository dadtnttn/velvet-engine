use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use velvet_script_vs3::{Value, VmLimits};

#[derive(Debug, Clone)]
pub struct ImageSpec {
    pub id: String,
    pub path: String,
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct FontSpec {
    pub id: String,
    pub path: String,
}

/// A font used for characters the main fonts lack (CJK, Devanagari, Thai,
/// Hebrew...). Loaded on demand for the languages listed in `langs`
/// (`["*"]` = always).
#[derive(Debug, Clone)]
pub struct FallbackFontSpec {
    pub id: String,
    pub path: String,
    pub langs: Vec<String>,
}

/// A sound effect or music track (16-bit PCM WAV).
#[derive(Debug, Clone)]
pub struct SoundSpec {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct AppSpec {
    pub title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub minimum_width: u32,
    pub minimum_height: u32,
    pub canvas_width: usize,
    pub canvas_height: usize,
    pub resizable: bool,
}

#[derive(Debug, Clone)]
pub struct AppLimits {
    pub max_commands_per_frame: usize,
    pub max_assets: usize,
    pub max_image_pixels: usize,
    pub max_memory_units: usize,
    pub max_instructions: u64,
    /// Instructions a running `app_task` may use per frame.
    pub task_slice_instructions: u64,
    /// Instructions a whole `app_task` may use before it is stopped.
    pub max_task_instructions: u64,
}

impl Default for AppLimits {
    fn default() -> Self {
        Self {
            max_commands_per_frame: 16_384,
            max_assets: 2_048,
            max_image_pixels: 67_108_864,
            max_memory_units: 64_000_000,
            max_instructions: 5_000_000,
            task_slice_instructions: 2_000_000,
            max_task_instructions: 4_000_000_000,
        }
    }
}

impl AppLimits {
    pub fn vm_limits(&self) -> VmLimits {
        VmLimits {
            max_memory_units: self.max_memory_units,
            max_instructions: self.max_instructions,
            ..VmLimits::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppManifest {
    pub app: AppSpec,
    pub capabilities: BTreeSet<String>,
    pub limits: AppLimits,
    pub root: String,
    pub fonts: Vec<FontSpec>,
    pub fallback_fonts: Vec<FallbackFontSpec>,
    pub images: Vec<ImageSpec>,
    pub sounds: Vec<SoundSpec>,
}

impl AppManifest {
    pub fn parse(value: &Value) -> Result<Self> {
        let app_value = field(value, "app")?;
        let app = AppSpec {
            title: string(&app_value, "title")?,
            window_width: integer_or(&app_value, "window_width", 960)? as u32,
            window_height: integer_or(&app_value, "window_height", 640)? as u32,
            minimum_width: integer_or(&app_value, "minimum_width", 640)? as u32,
            minimum_height: integer_or(&app_value, "minimum_height", 360)? as u32,
            canvas_width: integer_or(&app_value, "canvas_width", 960)? as usize,
            canvas_height: integer_or(&app_value, "canvas_height", 640)? as usize,
            resizable: bool_or(&app_value, "resizable", true)?,
        };
        anyhow::ensure!(
            app.window_width > 0 && app.window_height > 0,
            "window size must be positive"
        );
        anyhow::ensure!(
            app.canvas_width > 0 && app.canvas_height > 0,
            "canvas size must be positive"
        );
        anyhow::ensure!(
            app.canvas_width <= 16_384 && app.canvas_height <= 16_384,
            "canvas exceeds 16384px limit"
        );

        let capabilities = optional_strings(value, "capabilities")?
            .into_iter()
            .collect();
        let limits = parse_limits(value)?;
        let root = string(value, "root")?;
        let fonts = list(value, "fonts")?
            .iter()
            .map(|entry| {
                Ok(FontSpec {
                    id: string(entry, "id")?,
                    path: string(entry, "path")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let images = list(value, "images")?
            .iter()
            .map(|entry| {
                Ok(ImageSpec {
                    id: string(entry, "id")?,
                    path: string(entry, "path")?,
                    mode: string_or(entry, "mode", "preload")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let fallback_fonts = match optional_field(value, "fallback_fonts")? {
            Some(items) => items
                .list_items()
                .map_err(|error| anyhow!(error))?
                .iter()
                .map(|entry| {
                    Ok(FallbackFontSpec {
                        id: string(entry, "id")?,
                        path: string(entry, "path")?,
                        langs: optional_strings(entry, "langs")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        let sounds = match optional_field(value, "sounds")? {
            Some(items) => items
                .list_items()
                .map_err(|error| anyhow!(error))?
                .iter()
                .map(|entry| {
                    Ok(SoundSpec {
                        id: string(entry, "id")?,
                        path: string(entry, "path")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        anyhow::ensure!(!fonts.is_empty(), "manifest requires at least one font");
        anyhow::ensure!(
            images.len() <= limits.max_assets,
            "manifest declares too many images"
        );
        Ok(Self {
            app,
            capabilities,
            limits,
            root,
            fonts,
            fallback_fonts,
            images,
            sounds,
        })
    }

    pub fn asset_root(&self, package_root: &Path) -> PathBuf {
        package_root.join(&self.root)
    }

    pub fn require_capability(&self, name: &str) -> Result<()> {
        anyhow::ensure!(
            self.capabilities.contains(name),
            "VS3 package did not declare `{name}` capability"
        );
        Ok(())
    }
}

fn parse_limits(value: &Value) -> Result<AppLimits> {
    let defaults = AppLimits::default();
    let Some(limits) = optional_field(value, "limits")? else {
        return Ok(defaults);
    };
    Ok(AppLimits {
        max_commands_per_frame: integer_or(
            &limits,
            "max_commands_per_frame",
            defaults.max_commands_per_frame as i64,
        )? as usize,
        max_assets: integer_or(&limits, "max_assets", defaults.max_assets as i64)? as usize,
        max_image_pixels: integer_or(
            &limits,
            "max_image_pixels",
            defaults.max_image_pixels as i64,
        )? as usize,
        max_memory_units: integer_or(
            &limits,
            "max_memory_units",
            defaults.max_memory_units as i64,
        )? as usize,
        max_instructions: integer_or(
            &limits,
            "max_instructions",
            defaults.max_instructions as i64,
        )? as u64,
        task_slice_instructions: integer_or(
            &limits,
            "task_slice_instructions",
            defaults.task_slice_instructions as i64,
        )?
        .max(1_000) as u64,
        max_task_instructions: integer_or(
            &limits,
            "max_task_instructions",
            defaults.max_task_instructions as i64,
        )?
        .max(1) as u64,
    })
}

fn field(value: &Value, key: &str) -> Result<Value> {
    value
        .map_get(key)
        .map_err(|error| anyhow!(error))?
        .ok_or_else(|| anyhow!("missing `{key}` in VS3 app manifest"))
}

fn optional_field(value: &Value, key: &str) -> Result<Option<Value>> {
    value.map_get(key).map_err(|error| anyhow!(error))
}

fn string(value: &Value, key: &str) -> Result<String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("`{key}` must be a string"))
}

fn string_or(value: &Value, key: &str, default: &str) -> Result<String> {
    match optional_field(value, key)? {
        None => Ok(default.to_owned()),
        Some(item) => item
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("`{key}` must be a string")),
    }
}

fn integer_or(value: &Value, key: &str, default: i64) -> Result<i64> {
    match optional_field(value, key)? {
        None => Ok(default),
        Some(item) => item
            .as_i64()
            .ok_or_else(|| anyhow!("`{key}` must be an integer")),
    }
}

fn bool_or(value: &Value, key: &str, default: bool) -> Result<bool> {
    match optional_field(value, key)? {
        None => Ok(default),
        Some(Value::Bool(flag)) => Ok(flag),
        Some(_) => Err(anyhow!("`{key}` must be a bool")),
    }
}

fn list(value: &Value, key: &str) -> Result<Vec<Value>> {
    field(value, key)?
        .list_items()
        .map_err(|error| anyhow!(error))
        .with_context(|| format!("manifest list `{key}`"))
}

fn optional_strings(value: &Value, key: &str) -> Result<Vec<String>> {
    let Some(items) = optional_field(value, key)? else {
        return Ok(Vec::new());
    };
    items
        .list_items()
        .map_err(|error| anyhow!(error))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("`{key}` must contain strings"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_vs3::{bool_val, int, list_val, map_val, string_val};

    #[test]
    fn manifest_parses_declarative_contract() {
        let value = map_val([
            (
                "app".into(),
                map_val([
                    ("title".into(), string_val("Test")),
                    ("window_width".into(), int(800)),
                    ("window_height".into(), int(600)),
                    ("canvas_width".into(), int(320)),
                    ("canvas_height".into(), int(180)),
                    ("resizable".into(), bool_val(true)),
                ]),
            ),
            (
                "capabilities".into(),
                list_val(vec![
                    string_val("window"),
                    string_val("render2d"),
                    string_val("input"),
                ]),
            ),
            ("root".into(), string_val("assets")),
            (
                "fonts".into(),
                list_val(vec![map_val([
                    ("id".into(), string_val("regular")),
                    ("path".into(), string_val("system:arial.ttf")),
                ])]),
            ),
            ("images".into(), list_val(vec![])),
        ]);
        let manifest = AppManifest::parse(&value).unwrap();
        assert_eq!(manifest.app.canvas_width, 320);
        assert_eq!(manifest.app.canvas_height, 180);
        assert!(manifest.capabilities.contains("render2d"));
        assert_eq!(manifest.limits.max_commands_per_frame, 16_384);
    }

    #[test]
    fn manifest_rejects_invalid_canvas() {
        let value = map_val([
            (
                "app".into(),
                map_val([
                    ("title".into(), string_val("Bad")),
                    ("canvas_width".into(), int(0)),
                    ("canvas_height".into(), int(180)),
                ]),
            ),
            ("root".into(), string_val("assets")),
            (
                "fonts".into(),
                list_val(vec![map_val([
                    ("id".into(), string_val("regular")),
                    ("path".into(), string_val("system:arial.ttf")),
                ])]),
            ),
            ("images".into(), list_val(vec![])),
        ]);
        assert!(AppManifest::parse(&value).is_err());
    }
}
