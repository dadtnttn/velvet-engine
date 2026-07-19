//! In-memory style/pose state for JS-lite `set` / `query` and transition ticks.

use indexmap::IndexMap;

use crate::animation::{ChannelPlan, TimelinePlan};
use crate::resolve::ComputedStyle;
use crate::script::StyleAction;
use crate::value::StyleValue;

/// Per-target numeric style channels (opacity, x, y, scale, …).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeStyle {
    /// Channel → value.
    pub channels: IndexMap<String, f32>,
    /// Optional class list (for tooling).
    pub classes: Vec<String>,
}

/// Lightweight runtime map id → style channels.
#[derive(Debug, Clone, Default)]
pub struct StyleRuntime {
    /// Targets by id (no `#`).
    pub nodes: IndexMap<String, NodeStyle>,
}

impl StyleRuntime {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure node exists.
    pub fn ensure(&mut self, id: &str) -> &mut NodeStyle {
        let id = id.trim_start_matches('#').to_string();
        self.nodes.entry(id).or_default()
    }

    /// Set a numeric channel.
    pub fn set(&mut self, id: &str, prop: &str, value: f32) {
        self.ensure(id).channels.insert(prop.to_string(), value);
    }

    /// Get a channel (default 0).
    pub fn get(&self, id: &str, prop: &str) -> f32 {
        let id = id.trim_start_matches('#');
        self.nodes
            .get(id)
            .and_then(|n| n.channels.get(prop).copied())
            .unwrap_or(0.0)
    }

    /// Apply script side-effects (`Set` actions).
    pub fn apply_actions(&mut self, actions: &[StyleAction]) {
        for a in actions {
            if let StyleAction::Set {
                target,
                prop,
                value,
            } = a
            {
                self.set(target, prop, *value);
            }
        }
    }
}

/// Parse `transition` / longhands from computed style into a short tween plan.
///
/// Supports `transition: opacity 0.2s ease` and multi props via
/// `transition-property` + `transition-duration`.
pub fn plan_transition(
    from: &ComputedStyle,
    to: &ComputedStyle,
    target: Option<&str>,
) -> Option<TimelinePlan> {
    let mut duration = 0.2f32;
    let mut ease = "cubic_out".to_string();
    let mut props: Vec<String> = Vec::new();

    if let Some(v) = from
        .props
        .get("transition")
        .or_else(|| to.props.get("transition"))
        .and_then(|v| v.as_str())
    {
        // shorthand: [property] duration [ease]
        for part in v.split_whitespace() {
            let p = part.strip_suffix('s').unwrap_or(part);
            if let Ok(n) = p.parse::<f32>() {
                duration = n;
            } else if part.contains("ease")
                || part.contains("linear")
                || part.contains("cubic")
                || part.contains("quad")
                || part.contains("back")
            {
                ease = part.to_string();
            } else if part != "all" {
                props.push(part.to_string());
            }
        }
    }
    if let Some(d) = from
        .props
        .get("transition-duration")
        .or_else(|| to.props.get("transition-duration"))
        .and_then(|v| v.as_f32())
    {
        duration = d;
    }
    if let Some(e) = from
        .props
        .get("transition-timing-function")
        .or_else(|| to.props.get("transition-timing-function"))
        .or_else(|| from.props.get("transition-ease"))
        .and_then(|v| v.as_str())
    {
        ease = e.to_string();
    }
    if let Some(p) = from
        .props
        .get("transition-property")
        .or_else(|| to.props.get("transition-property"))
        .and_then(|v| v.as_str())
    {
        props = p
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "all")
            .collect();
    }
    if props.is_empty() {
        // default: animate numeric props that differ
        for (k, v) in &to.props {
            if k.starts_with("animation") || k.starts_with("transition") || k.starts_with("--") {
                continue;
            }
            if v.as_f32().is_some() {
                let from_n = from.props.get(k).and_then(|x| x.as_f32());
                let to_n = v.as_f32();
                if let (Some(a), Some(b)) = (from_n, to_n) {
                    if (a - b).abs() > 1e-5 {
                        props.push(k.clone());
                    }
                } else if to_n.is_some() {
                    props.push(k.clone());
                }
            }
            // opacity via color not handled
            if k == "opacity" {
                props.push(k.clone());
            }
        }
        props.sort();
        props.dedup();
    }
    if props.is_empty() || duration <= 0.0 {
        return None;
    }
    let mut channels = Vec::new();
    for p in props {
        let a = from.props.get(&p).and_then(|v| v.as_f32()).unwrap_or(0.0);
        let b = to.props.get(&p).and_then(|v| v.as_f32()).unwrap_or(a);
        if (a - b).abs() < 1e-6 {
            continue;
        }
        channels.push(ChannelPlan {
            channel: p,
            keys: vec![(0.0, a), (duration, b)],
            ease: ease.clone(),
        });
    }
    if channels.is_empty() {
        return None;
    }
    Some(TimelinePlan {
        channels,
        duration,
        target: target.map(|s| s.trim_start_matches('#').to_string()),
    })
}

/// Build a one-shot tween plan between two numbers on a channel.
pub fn plan_channel_tween(
    channel: &str,
    from: f32,
    to: f32,
    duration: f32,
    ease: &str,
    target: Option<&str>,
) -> TimelinePlan {
    TimelinePlan {
        channels: vec![ChannelPlan {
            channel: channel.into(),
            keys: vec![(0.0, from), (duration.max(0.0), to)],
            ease: ease.into(),
        }],
        duration: duration.max(0.0),
        target: target.map(|s| s.trim_start_matches('#').to_string()),
    }
}

/// Read a number from computed style.
pub fn computed_number(style: &ComputedStyle, name: &str, default: f32) -> f32 {
    style.number(name, default)
}

/// Whether a property looks numeric for transitions.
pub fn is_numeric_style_value(v: &StyleValue) -> bool {
    v.as_f32().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_stylesheet;
    use crate::resolve::{resolve, StyleQuery};

    #[test]
    fn transition_plan_from_styles() {
        let sheet = parse_stylesheet(
            r#"
            .a { opacity: 0; transition: opacity 0.25s linear; }
            .a:selected { opacity: 1; }
            "#,
        )
        .unwrap();
        let from = resolve(&sheet, &StyleQuery::class("a"));
        let to = resolve(
            &sheet,
            &StyleQuery::class("a").with_state("selected"),
        );
        let plan = plan_transition(&from, &to, Some("btn")).unwrap();
        assert!((plan.duration - 0.25).abs() < 1e-4);
        assert_eq!(plan.channels[0].channel, "opacity");
        assert_eq!(plan.target.as_deref(), Some("btn"));
    }

    #[test]
    fn runtime_set_get() {
        let mut rt = StyleRuntime::new();
        rt.set("card0", "opacity", 0.5);
        assert!((rt.get("card0", "opacity") - 0.5).abs() < 1e-5);
        rt.apply_actions(&[StyleAction::Set {
            target: "card0".into(),
            prop: "scale".into(),
            value: 1.2,
        }]);
        assert!((rt.get("card0", "scale") - 1.2).abs() < 1e-5);
    }
}
