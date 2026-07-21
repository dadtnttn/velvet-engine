//! Animation plans extracted from `.vcss` (`@keyframes` + `animation` props).
//!
//! This replaces the separate `.vanim` mini-language: motion lives in the same
//! stylesheet as visual style.

use indexmap::IndexMap;

use crate::parse::Stylesheet;
use crate::resolve::{resolve, ComputedStyle, StyleQuery};
use crate::value::{parse_time_seconds_token, StyleValue};

/// One stop inside `@keyframes`.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeStop {
    /// Progress 0.0 ..= 1.0
    pub offset: f32,
    /// Property bag at this stop.
    pub props: IndexMap<String, StyleValue>,
}

/// Named keyframe set.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Keyframes {
    /// Name (e.g. `deal`).
    pub name: String,
    /// Ordered stops.
    pub stops: Vec<KeyframeStop>,
}

/// Resolved animation instance from a style query.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSpec {
    /// Keyframe name.
    pub name: String,
    /// Duration seconds.
    pub duration: f32,
    /// Delay seconds.
    pub delay: f32,
    /// Timing function keyword (`cubic_out`, `linear`, …).
    pub easing: String,
    /// Iteration count (1 default; 0 = infinite not expanded here).
    pub iterations: f32,
    /// Fill mode: `none` | `forwards`
    pub fill_mode: String,
    /// Target id if provided via `animation-target` or query id.
    pub target: Option<String>,
}

impl Default for AnimationSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            duration: 0.35,
            delay: 0.0,
            easing: "cubic_out".into(),
            iterations: 1.0,
            fill_mode: "forwards".into(),
            target: None,
        }
    }
}

/// One channel of numeric keys ready for velvet-anim / any runner.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelPlan {
    /// Channel name: opacity, x, y, scale, yaw, pitch, roll, foil, depth, …
    pub channel: String,
    /// (time_secs, value)
    pub keys: Vec<(f32, f32)>,
    /// Ease applied between keys.
    pub ease: String,
}

/// Portable timeline plan (no dependency on velvet-anim).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimelinePlan {
    /// Channels.
    pub channels: Vec<ChannelPlan>,
    /// Total duration including delay padding at start of keys.
    pub duration: f32,
    /// Optional target id.
    pub target: Option<String>,
}

/// Read animation-* properties from computed style.
pub fn animation_spec_from_computed(style: &ComputedStyle) -> Option<AnimationSpec> {
    let name = style
        .props
        .get("animation")
        .and_then(|v| parse_animation_shorthand(v))
        .map(|(n, _, _, _)| n)
        .or_else(|| {
            style
                .props
                .get("animation-name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })?;
    if name.is_empty() || name == "none" {
        return None;
    }
    let mut spec = AnimationSpec {
        name,
        ..Default::default()
    };
    if let Some(v) = style
        .props
        .get("animation-duration")
        .and_then(|v| v.as_f32())
    {
        spec.duration = v;
    }
    if let Some(v) = style.props.get("animation-delay").and_then(|v| v.as_f32()) {
        spec.delay = v;
    }
    if let Some(v) = style
        .props
        .get("animation-timing-function")
        .or_else(|| style.props.get("animation-ease"))
        .and_then(|v| v.as_str())
    {
        spec.easing = v.to_string();
    }
    if let Some(v) = style
        .props
        .get("animation-iteration-count")
        .and_then(|v| v.as_f32())
    {
        spec.iterations = v;
    }
    if let Some(v) = style
        .props
        .get("animation-fill-mode")
        .and_then(|v| v.as_str())
    {
        spec.fill_mode = v.to_string();
    }
    if let Some(v) = style.props.get("animation-target").and_then(|v| v.as_str()) {
        spec.target = Some(v.to_string());
    }
    // shorthand may also set duration/ease
    if let Some(v) = style.props.get("animation") {
        if let Some(s) = v.as_str() {
            if let Some((_, dur, ease, delay)) = parse_animation_shorthand_str(s) {
                if dur > 0.0 {
                    spec.duration = dur;
                }
                if !ease.is_empty() {
                    spec.easing = ease;
                }
                if delay > 0.0 {
                    spec.delay = delay;
                }
            }
        }
    }
    Some(spec)
}

fn parse_animation_shorthand(v: &StyleValue) -> Option<(String, f32, String, f32)> {
    match v {
        StyleValue::Keyword(s) | StyleValue::String(s) => parse_animation_shorthand_str(s),
        _ => None,
    }
}

/// `deal 0.35s cubic_out 0.08s` or `deal 0.35 cubic_out`
fn parse_animation_shorthand_str(s: &str) -> Option<(String, f32, String, f32)> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let name = parts[0].to_string();
    let mut duration = 0.35f32;
    let mut ease = String::new();
    let mut delay = 0.0f32;
    let mut nums = 0u32;
    for p in parts.iter().skip(1) {
        if let Some(n) = parse_time_seconds_token(p) {
            if nums == 0 {
                duration = n;
            } else {
                delay = n;
            }
            nums += 1;
        } else {
            ease = (*p).to_string();
        }
    }
    Some((name, duration, ease, delay))
}

/// Build a [`TimelinePlan`] from sheet + query (class that references animation).
pub fn plan_animation(sheet: &Stylesheet, query: &StyleQuery) -> Option<TimelinePlan> {
    let style = resolve(sheet, query);
    let mut spec = animation_spec_from_computed(&style)?;
    if spec.target.is_none() {
        spec.target = query.id.clone();
    }
    plan_from_spec(sheet, &spec)
}

/// Build plan from explicit animation spec + keyframes in sheet.
pub fn plan_from_spec(sheet: &Stylesheet, spec: &AnimationSpec) -> Option<TimelinePlan> {
    let kf = sheet.keyframes.get(&spec.name)?;
    if kf.stops.is_empty() {
        return None;
    }
    // Collect all animatable numeric channels across stops
    let mut channel_names = indexmap::IndexSet::new();
    for stop in &kf.stops {
        for k in stop.props.keys() {
            if is_anim_channel(k) {
                channel_names.insert(k.clone());
            }
        }
    }
    let ease = if spec.easing.is_empty() {
        "cubic_out".to_string()
    } else {
        spec.easing.clone()
    };
    let mut channels = Vec::new();
    for ch in channel_names {
        let mut keys = Vec::new();
        for stop in &kf.stops {
            if let Some(v) = stop.props.get(&ch).and_then(|v| v.as_f32()) {
                let t = spec.delay + stop.offset * spec.duration;
                keys.push((t, v));
            }
        }
        if keys.len() >= 1 {
            // ensure sorted
            keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            channels.push(ChannelPlan {
                channel: ch,
                keys,
                ease: ease.clone(),
            });
        }
    }
    if channels.is_empty() {
        return None;
    }
    let duration = spec.delay + spec.duration;
    Some(TimelinePlan {
        channels,
        duration,
        target: spec.target.clone(),
    })
}

fn is_anim_channel(name: &str) -> bool {
    matches!(
        name,
        "opacity"
            | "x"
            | "y"
            | "scale"
            | "yaw"
            | "pitch"
            | "roll"
            | "foil"
            | "depth"
            | "rotation"
            | "width"
            | "height"
            | "translate-x"
            | "translate-y"
            | "glow-strength"
            | "blur"
            | "skew"
            | "border-radius"
            | "margin"
            | "padding"
            | "padding-x"
            | "gap"
    )
}

/// Convert legacy `.vanim` line script into a `.vcss` fragment (unified language).
///
/// Supported lines:
/// - `fx id deal x y duration [delay N] [ease E]` → keyframes + class
/// - `move id x y duration [ease E]`
/// - `wait secs` → ignored in sheet (caller handles sequencing)
/// - `spawn id x y` → sets #id { x; y; }
pub fn vanim_to_vcss(source: &str) -> Result<String, String> {
    let mut out = String::from("/* converted from legacy .vanim */\n");
    let mut kf_defs: IndexMap<String, String> = IndexMap::new();
    let mut n = 0u32;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "spawn" if parts.len() >= 4 => {
                out.push_str(&format!(
                    "#{} {{ x: {}; y: {}; }}\n",
                    parts[1], parts[2], parts[3]
                ));
            }
            "move" if parts.len() >= 5 => {
                n += 1;
                let name = format!("move_{n}");
                let ease = parts.get(5).copied().unwrap_or("cubic_out");
                let dur = parts[4];
                kf_defs.insert(
                    name.clone(),
                    format!(
                        "@keyframes {name} {{\n  from {{ /* keep */ }}\n  to {{ x: {}; y: {}; }}\n}}\n",
                        parts[2], parts[3]
                    ),
                );
                out.push_str(&format!(
                    "#{0}.anim {{ animation: {name} {dur}s {ease}; animation-target: {0}; }}\n",
                    parts[1]
                ));
            }
            "fx" if parts.len() >= 3 => {
                n += 1;
                let id = parts[1];
                let kind = parts[2];
                let mut x = 0.0f32;
                let mut y = 0.0f32;
                let mut dur = 0.35f32;
                let mut delay = 0.0f32;
                let mut ease = "cubic_out".to_string();
                let mut idx = 3;
                if idx + 1 < parts.len()
                    && parts[idx].parse::<f32>().is_ok()
                    && parts[idx + 1].parse::<f32>().is_ok()
                {
                    x = parts[idx].parse().unwrap();
                    y = parts[idx + 1].parse().unwrap();
                    idx += 2;
                }
                if idx < parts.len() && parts[idx].parse::<f32>().is_ok() {
                    dur = parts[idx].parse().unwrap();
                    idx += 1;
                }
                while idx < parts.len() {
                    match parts[idx] {
                        "delay" if idx + 1 < parts.len() => {
                            delay = parts[idx + 1].parse().unwrap_or(0.0);
                            idx += 2;
                        }
                        "ease" if idx + 1 < parts.len() => {
                            ease = parts[idx + 1].to_string();
                            idx += 2;
                        }
                        "strength" if idx + 1 < parts.len() => {
                            idx += 2;
                        }
                        other => {
                            if other.parse::<f32>().is_err() {
                                ease = other.to_string();
                            }
                            idx += 1;
                        }
                    }
                }
                let name = format!("{kind}_{n}");
                let body = match kind {
                    "deal" => format!(
                        "@keyframes {name} {{\n  from {{ opacity: 0; y: {}; scale: 0.65; yaw: 0.9; }}\n  to {{ opacity: 1; x: {x}; y: {y}; scale: 1; yaw: 0; }}\n}}\n",
                        y - 80.0
                    ),
                    "fade_in" | "appear" => format!(
                        "@keyframes {name} {{\n  from {{ opacity: 0; }}\n  to {{ opacity: 1; }}\n}}\n"
                    ),
                    "fade_out" => format!(
                        "@keyframes {name} {{\n  from {{ opacity: 1; }}\n  to {{ opacity: 0; }}\n}}\n"
                    ),
                    "punch" => format!(
                        "@keyframes {name} {{\n  0% {{ scale: 1; }}\n  45% {{ scale: 1.15; }}\n  100% {{ scale: 1; }}\n}}\n"
                    ),
                    "bounce" | "pop" => format!(
                        "@keyframes {name} {{\n  from {{ opacity: 0; scale: 0; }}\n  to {{ opacity: 1; scale: 1; }}\n}}\n"
                    ),
                    "move" => format!(
                        "@keyframes {name} {{\n  to {{ x: {x}; y: {y}; }}\n}}\n"
                    ),
                    _ => format!(
                        "@keyframes {name} {{\n  from {{ opacity: 0; }}\n  to {{ opacity: 1; x: {x}; y: {y}; }}\n}}\n"
                    ),
                };
                kf_defs.insert(name.clone(), body);
                out.push_str(&format!(
                    "#{id}.anim {{ animation: {name} {dur}s {ease} {delay}s; animation-target: {id}; }}\n"
                ));
            }
            "wait" => {
                out.push_str(&format!("/* wait {} */\n", parts.get(1).unwrap_or(&"0")));
            }
            "stop" => {
                out.push_str(&format!("/* stop {} */\n", parts.get(1).unwrap_or(&"")));
            }
            other => {
                return Err(format!("line {}: unsupported vanim op `{other}`", i + 1));
            }
        }
    }
    let mut final_out = String::new();
    for v in kf_defs.values() {
        final_out.push_str(v);
    }
    final_out.push_str(&out);
    Ok(final_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_stylesheet;

    #[test]
    fn keyframes_plan_opacity() {
        let src = r#"
        @keyframes fade {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        .intro {
          animation: fade 0.5s linear;
        }
        "#;
        let sheet = parse_stylesheet(src).unwrap();
        assert!(sheet.keyframes.contains_key("fade"));
        let plan = plan_animation(&sheet, &StyleQuery::class("intro")).unwrap();
        assert_eq!(plan.channels.len(), 1);
        assert_eq!(plan.channels[0].channel, "opacity");
        assert!((plan.duration - 0.5).abs() < 1e-4);
    }

    #[test]
    fn animation_time_longhands_support_seconds_and_milliseconds() {
        let src = r#"
        @keyframes fade {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        .intro {
          animation-name: fade;
          animation-duration: 180ms;
          animation-delay: 0.02s;
        }
        "#;
        let sheet = parse_stylesheet(src).unwrap();
        let plan = plan_animation(&sheet, &StyleQuery::class("intro")).unwrap();
        assert!((plan.duration - 0.2).abs() < 1e-4);
        assert!((plan.channels[0].keys[0].0 - 0.02).abs() < 1e-4);
        assert!((plan.channels[0].keys[1].0 - 0.2).abs() < 1e-4);
    }

    #[test]
    fn animation_shorthand_supports_milliseconds() {
        let src = r#"
        @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
        .intro { animation: fade 180ms cubic_out 20ms; }
        "#;
        let sheet = parse_stylesheet(src).unwrap();
        let plan = plan_animation(&sheet, &StyleQuery::class("intro")).unwrap();
        assert!((plan.duration - 0.2).abs() < 1e-4);
    }

    #[test]
    fn vanim_converts_to_vcss() {
        let vcss = vanim_to_vcss("fx card0 deal 200 300 0.3 delay 0.1\n").unwrap();
        assert!(vcss.contains("@keyframes"));
        assert!(vcss.contains("card0"));
        let sheet = parse_stylesheet(&vcss).unwrap();
        assert!(!sheet.keyframes.is_empty());
    }
}
