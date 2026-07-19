//! Optional story host for `style.load` / `style.use`.

use std::path::Path;
use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};

use crate::resolve::{StyleQuery, StyleRegistry};

/// Shared style registry for narrative hosts.
pub struct StyleStoryHost {
    /// Registry.
    pub registry: Mutex<StyleRegistry>,
}

impl StyleStoryHost {
    /// Empty.
    pub fn new() -> Self {
        Self {
            registry: Mutex::new(StyleRegistry::new()),
        }
    }

    /// Load file into registry.
    pub fn load_path(&self, name: &str, path: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut reg = self.registry.lock().map_err(|e| e.to_string())?;
        reg.load_str(name, &text).map_err(|e| e.to_string())
    }
}

impl Default for StyleStoryHost {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryCommandHost for StyleStoryHost {
    fn call(
        &self,
        name: &str,
        args: &IndexMap<String, StoryValue>,
        vars: &mut StoryVariables,
    ) -> Result<CommandOutcome, StoryCommandError> {
        match name {
            "style.load" => {
                let sheet_name = arg_str(args, "name").unwrap_or_else(|| "default".into());
                if let Some(body) = arg_str(args, "body").or_else(|| arg_str(args, "code")) {
                    let mut reg = self
                        .registry
                        .lock()
                        .map_err(|e| StoryCommandError::new(e.to_string()))?;
                    reg.load_str(&sheet_name, &body)
                        .map_err(|e| StoryCommandError::new(e.to_string()))?;
                } else if let Some(path) = arg_str(args, "path") {
                    self.load_path(&sheet_name, Path::new(&path))
                        .map_err(StoryCommandError::new)?;
                } else {
                    return Err(StoryCommandError::new(
                        "style.load needs path= or body=",
                    ));
                }
                vars.set("style.active", StoryValue::String(sheet_name));
                Ok(CommandOutcome::Continue)
            }
            "style.use" => {
                let sheet_name = arg_str(args, "name").unwrap_or_else(|| "default".into());
                let mut reg = self
                    .registry
                    .lock()
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                if reg.sheets.contains_key(&sheet_name) {
                    reg.active = Some(sheet_name.clone());
                    vars.set("style.active", StoryValue::String(sheet_name));
                    Ok(CommandOutcome::Continue)
                } else {
                    Err(StoryCommandError::new(format!(
                        "unknown stylesheet `{sheet_name}`"
                    )))
                }
            }
            "style.resolve" => {
                // debug: set vars from a class resolve
                let class = arg_str(args, "class").unwrap_or_else(|| "button".into());
                let state = arg_str(args, "state");
                let mut q = StyleQuery::class(class);
                if let Some(s) = state {
                    q = q.with_state(s);
                }
                if let Some(id) = arg_str(args, "id") {
                    q = q.with_id(id);
                }
                let reg = self
                    .registry
                    .lock()
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                let computed = reg.resolve(&q);
                let bg = computed.background();
                vars.set(
                    "style.bg",
                    StoryValue::String(format!("#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b)),
                );
                let fg = computed.color_text();
                vars.set(
                    "style.color",
                    StoryValue::String(format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b)),
                );
                Ok(CommandOutcome::Continue)
            }
            _ => Ok(CommandOutcome::Continue),
        }
    }
}

fn arg_str(args: &IndexMap<String, StoryValue>, key: &str) -> Option<String> {
    args.get(key).map(|v| v.display_str())
}
