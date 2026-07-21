//! Optional story host for `style.load` / `style.use` / `style.call` / `style.emit`.

use std::path::Path;
use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};

use crate::resolve::{StyleQuery, StyleRegistry};
use crate::script::JsValue;

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
                    return Err(StoryCommandError::new("style.load needs path= or body="));
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
            "style.play" | "anim.script" => {
                // Unified: body can be .vcss (CSS+JS) or legacy .vanim (auto-converted)
                let body = arg_str(args, "body")
                    .or_else(|| arg_str(args, "code"))
                    .unwrap_or_default();
                let vcss = if body.contains('{')
                    || body.contains("@keyframes")
                    || body.contains("@script")
                {
                    body
                } else {
                    crate::animation::vanim_to_vcss(&body).map_err(StoryCommandError::new)?
                };
                let sheet_name = arg_str(args, "name").unwrap_or_else(|| "anim".into());
                let mut reg = self
                    .registry
                    .lock()
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                reg.load_str(&sheet_name, &vcss)
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                vars.set("style.active", StoryValue::String(sheet_name));
                vars.set(
                    "style.keyframes",
                    StoryValue::Int(reg.sheets.values().map(|s| s.keyframes.len() as i64).sum()),
                );
                vars.set(
                    "style.fns",
                    StoryValue::Int(
                        reg.sheets
                            .values()
                            .map(|s| s.script.functions.len() as i64)
                            .sum(),
                    ),
                );
                Ok(CommandOutcome::Continue)
            }
            "style.call" => {
                // style.call: fn: dealHand, arg0: 5  (optional sheet=)
                let fn_name = arg_str(args, "fn")
                    .or_else(|| arg_str(args, "function"))
                    .ok_or_else(|| StoryCommandError::new("style.call needs fn="))?;
                let sheet_override = arg_str(args, "sheet");
                let mut js_args = Vec::new();
                for i in 0..8 {
                    let key = format!("arg{i}");
                    if let Some(v) = args.get(&key) {
                        js_args.push(story_to_js(v));
                    }
                }
                if js_args.is_empty() {
                    if let Some(c) = arg_str(args, "count").and_then(|s| s.parse::<f32>().ok()) {
                        js_args.push(JsValue::Number(c));
                    } else if let Some(t) = arg_str(args, "target") {
                        js_args.push(JsValue::String(t));
                    }
                }
                let reg = self
                    .registry
                    .lock()
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                let sheet_key = sheet_override
                    .or_else(|| reg.active.clone())
                    .ok_or_else(|| StoryCommandError::new("no active stylesheet"))?;
                let sheet = reg.sheets.get(&sheet_key).ok_or_else(|| {
                    StoryCommandError::new(format!("unknown stylesheet `{sheet_key}`"))
                })?;
                let run = crate::call_style_fn(sheet, &fn_name, &js_args)
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                vars.set("style.actions", StoryValue::Int(run.actions.len() as i64));
                vars.set(
                    "style.timelines",
                    StoryValue::Int(run.timelines.len() as i64),
                );
                Ok(CommandOutcome::Continue)
            }
            "style.emit" => {
                let event = arg_str(args, "event")
                    .or_else(|| arg_str(args, "name"))
                    .ok_or_else(|| StoryCommandError::new("style.emit needs event="))?;
                let mut js_args = Vec::new();
                if let Some(p) = arg_str(args, "payload") {
                    js_args.push(JsValue::String(p));
                }
                let reg = self
                    .registry
                    .lock()
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                let sheet_name = reg
                    .active
                    .clone()
                    .ok_or_else(|| StoryCommandError::new("no active stylesheet"))?;
                let sheet = reg.sheets.get(&sheet_name).ok_or_else(|| {
                    StoryCommandError::new(format!("unknown stylesheet `{sheet_name}`"))
                })?;
                let run = crate::emit_style_event(sheet, &event, &js_args)
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                vars.set("style.actions", StoryValue::Int(run.actions.len() as i64));
                Ok(CommandOutcome::Continue)
            }
            "style.set" => {
                // Records intent in story vars (host games apply to StyleRuntime).
                let target = arg_str(args, "target").unwrap_or_default();
                let prop = arg_str(args, "prop")
                    .or_else(|| arg_str(args, "property"))
                    .unwrap_or_else(|| "opacity".into());
                let value = arg_str(args, "value")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.0);
                vars.set("style.set.target", StoryValue::String(target));
                vars.set("style.set.prop", StoryValue::String(prop));
                vars.set("style.set.value", StoryValue::Float(value as f64));
                Ok(CommandOutcome::Continue)
            }
            "style.dump" => {
                let class = arg_str(args, "class").unwrap_or_else(|| "button".into());
                let state = arg_str(args, "state");
                let mut q = crate::StyleQuery::class(class);
                if let Some(s) = state {
                    q = q.with_state(s);
                }
                let reg = self
                    .registry
                    .lock()
                    .map_err(|e| StoryCommandError::new(e.to_string()))?;
                let computed = reg.resolve(&q);
                vars.set(
                    "style.dump.height",
                    StoryValue::Float(computed.number("height", 0.0) as f64),
                );
                vars.set(
                    "style.dump.props",
                    StoryValue::Int(computed.props.len() as i64),
                );
                Ok(CommandOutcome::Continue)
            }
            _ => Ok(CommandOutcome::Continue),
        }
    }
}

fn story_to_js(v: &StoryValue) -> JsValue {
    match v {
        StoryValue::Null => JsValue::Null,
        StoryValue::Int(i) => JsValue::Number(*i as f32),
        StoryValue::Float(f) => JsValue::Number(*f as f32),
        StoryValue::Bool(b) => JsValue::Bool(*b),
        StoryValue::String(s) => {
            if let Ok(n) = s.parse::<f32>() {
                JsValue::Number(n)
            } else {
                JsValue::String(s.clone())
            }
        }
    }
}

fn arg_str(args: &IndexMap<String, StoryValue>, key: &str) -> Option<String> {
    args.get(key).map(|v| v.display_str())
}
