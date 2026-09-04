//! Mini-ATL motions plus full Ren'Py-style ATL transforms.
//!
//! One-liners (`show luna at left with move`) produce a [`SpriteMotion`].
//! Named blocks (`transform bob { … }`) lower to [`TransformDef`] and
//! evaluate with the pure [`sample_transform`] function: hosts pass wall
//! time and get channel poses, no engine state required.

use serde::{Deserialize, Serialize};
use velvet_math::Ease;

/// Motion kind (one line in script).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionKind {
    /// Slide placement (uses transition Move).
    #[default]
    Move,
    /// Scale punch 1.0 -> amount -> 1.0.
    Zoom,
    /// X shake.
    Shake,
    /// Opacity 0 -> 1.
    Fade,
}

impl MotionKind {
    /// Parse word.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "move" | "slide" => Some(Self::Move),
            "zoom" | "punch" | "scale" => Some(Self::Zoom),
            "shake" => Some(Self::Shake),
            "fade" | "fadein" | "appear" => Some(Self::Fade),
            _ => None,
        }
    }

    /// Canonical name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Zoom => "zoom",
            Self::Shake => "shake",
            Self::Fade => "fade",
        }
    }
}

/// One parsed motion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteMotion {
    /// Kind.
    pub kind: MotionKind,
    /// Duration seconds.
    pub duration: f32,
    /// Amount (zoom scale add, shake px, move ignored).
    pub amount: f32,
}

impl Default for SpriteMotion {
    fn default() -> Self {
        Self {
            kind: MotionKind::Move,
            duration: 0.35,
            amount: 8.0,
        }
    }
}

impl SpriteMotion {
    /// New with clamps.
    pub fn new(kind: MotionKind, duration: f32, amount: f32) -> Self {
        Self {
            kind,
            duration: duration.clamp(0.05, 3.0),
            amount: amount.clamp(0.0, 64.0),
        }
    }

    /// Progress 0..=1 for elapsed.
    pub fn progress(&self, elapsed: f32) -> f32 {
        (elapsed / self.duration.max(1e-4)).clamp(0.0, 1.0)
    }

    /// X offset px for shake at elapsed.
    pub fn shake_offset(&self, elapsed: f32) -> f32 {
        if self.kind != MotionKind::Shake {
            return 0.0;
        }
        let t = self.progress(elapsed);
        // 2 swings decaying
        (t * std::f32::consts::TAU * 2.0).sin() * self.amount * (1.0 - t)
    }

    /// Scale multiplier for zoom at elapsed.
    pub fn zoom_scale(&self, elapsed: f32) -> f32 {
        if self.kind != MotionKind::Zoom {
            return 1.0;
        }
        let t = self.progress(elapsed);
        // up then settle: peak at t=0.4
        if t < 0.4 {
            1.0 + self.amount * (t / 0.4)
        } else {
            1.0 + self.amount * (1.0 - (t - 0.4) / 0.6)
        }
    }

    /// Opacity for fade at elapsed.
    pub fn fade_alpha(&self, elapsed: f32) -> f32 {
        if self.kind != MotionKind::Fade {
            return 1.0;
        }
        self.progress(elapsed)
    }
}

/// Parsed `show <target> at <place> [with <motion> [secs] [amount]]`.
///
/// Returns (target, place, motion). Place/motion are None when absent.
/// Examples:
/// - `luna happy at left with shake 0.4 10`
/// - `luna at right with move`
/// - `luna with zoom 0.3 0.2`
pub fn parse_show_with_motion(line: &str) -> (String, Option<String>, Option<SpriteMotion>) {
    let (head, with_part) = match line.split_once(" with ") {
        Some((h, w)) => (h.trim(), Some(w.trim())),
        None => (line.trim(), None),
    };
    let (target, place) = match head.split_once(" at ") {
        Some((t, p)) => {
            let p = p.trim();
            let place = if p.is_empty() {
                None
            } else {
                Some(p.to_string())
            };
            (t.trim().to_string(), place)
        }
        None => (head.to_string(), None),
    };
    let motion = with_part.and_then(parse_motion_words);
    (target, place, motion)
}

/// Parse `move [secs] [amount]` style words.
pub fn parse_motion_words(words: &str) -> Option<SpriteMotion> {
    let mut it = words.split_whitespace();
    let kind_word = it.next()?;
    let kind = MotionKind::parse(kind_word)?;
    let duration = it
        .next()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.35);
    let amount_default = match kind {
        MotionKind::Move => 8.0,
        MotionKind::Zoom => 0.25,
        MotionKind::Shake => 8.0,
        MotionKind::Fade => 0.0,
    };
    let amount = it
        .next()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(amount_default);
    Some(SpriteMotion::new(kind, duration, amount))
}

/// Valid ATL channel names (others are lowering diagnostics).
pub const ATL_CHANNELS: &[&str] = &[
    "x", "y", "opacity", "scale", "zoom", "rotate", "yaw", "pitch", "roll", "foil", "depth",
];

/// One lowered ATL transform step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AtlStepRt {
    /// Animate channels from the incoming pose over `duration`.
    Set {
        /// Channel targets.
        props: Vec<(String, f32)>,
        /// Duration seconds.
        duration: f32,
        /// Warper name.
        warper: String,
    },
    /// Concurrent steps (duration = slowest child).
    Parallel(Vec<AtlStepRt>),
    /// Host picks one variant by index.
    Choice(Vec<AtlStepRt>),
    /// Loop steps (`None` = forever).
    Repeat {
        /// Iterations (None = infinite).
        count: Option<u32>,
        /// Repeated steps.
        steps: Vec<AtlStepRt>,
    },
}

/// A lowered, evaluatable ATL transform.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransformDef {
    /// Transform name.
    pub name: String,
    /// Steps in author order.
    pub steps: Vec<AtlStepRt>,
    /// `on show` handler steps.
    #[serde(default)]
    pub on_show: Vec<AtlStepRt>,
    /// `on hide` handler steps.
    #[serde(default)]
    pub on_hide: Vec<AtlStepRt>,
    /// `on replace` handler steps.
    #[serde(default)]
    pub on_replace: Vec<AtlStepRt>,
}

/// ATL lifecycle event with handler steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlEvent {
    /// Sprite shown with this transform.
    Show,
    /// Sprite hidden while this transform is active.
    Hide,
    /// Transform replaced by another on the same sprite.
    Replace,
}

impl AtlEvent {
    /// Parse event name.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "show" => Some(Self::Show),
            "hide" => Some(Self::Hide),
            "replace" => Some(Self::Replace),
            _ => None,
        }
    }
}

impl TransformDef {
    /// Total duration seconds, or `None` when infinite (unbounded repeat).
    pub fn duration(&self) -> Option<f32> {
        step_list_duration(&self.steps)
    }

    /// Handler steps for an event.
    pub fn handler(&self, event: AtlEvent) -> &[AtlStepRt] {
        match event {
            AtlEvent::Show => &self.on_show,
            AtlEvent::Hide => &self.on_hide,
            AtlEvent::Replace => &self.on_replace,
        }
    }

    /// Handler duration (`None` when infinite).
    pub fn handler_duration(&self, event: AtlEvent) -> Option<f32> {
        step_list_duration(self.handler(event))
    }
}

/// Number of `choice` alternatives in a transform (max across choice
/// nodes, at least 1). Hosts pass this to [`next_variant`].
pub fn transform_variants(def: &TransformDef) -> usize {
    fn arity(steps: &[AtlStepRt]) -> usize {
        let mut max = 1usize;
        for step in steps {
            match step {
                AtlStepRt::Choice(children) => {
                    max = max.max(children.len().max(1));
                    for child in children {
                        max = max.max(arity(std::slice::from_ref(child)));
                    }
                }
                AtlStepRt::Parallel(children) => {
                    for child in children {
                        max = max.max(arity(std::slice::from_ref(child)));
                    }
                }
                AtlStepRt::Repeat { steps, .. } => {
                    max = max.max(arity(steps));
                }
                AtlStepRt::Set { .. } => {}
            }
        }
        max
    }
    arity(&def.steps)
        .max(arity(&def.on_show))
        .max(arity(&def.on_hide))
        .max(arity(&def.on_replace))
}

/// Pick a `choice` variant index from a caller-owned seed (xorshift64).
///
/// Pure and dependency-free: same seed sequence, same picks.
/// Returns 0 when there are no variants.
pub fn next_variant(choices: usize, seed: &mut u64) -> usize {
    if choices == 0 {
        return 0;
    }
    // xorshift64* — good enough for variant picks, no external RNG needed.
    let mut x = *seed | 1;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *seed = x;
    (x.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as usize % choices
}

/// Lower parsed `transform` steps into an evaluatable [`TransformDef`].
///
/// Errors on unknown channels, non-numeric channel values, negative
/// durations, empty `choice` blocks, and `repeat` with nothing to loop.
/// (Warper names are validated by the parser; unknown ones fall back to
/// linear at sample time.)
/// Sample an `on show/hide/replace` handler like the main steps.
///
/// Pure: same inputs, same pose. Empty handlers return `base` unchanged.
pub fn sample_transform_on(
    def: &TransformDef,
    event: AtlEvent,
    base: &[(String, f32)],
    t: f32,
    variant: usize,
) -> Vec<(String, f32)> {
    let mut pose: Vec<(String, f32)> = base.to_vec();
    sample_steps(def.handler(event), &mut pose, t.max(0.0), variant);
    pose
}

/// Lower parsed `transform` steps into an evaluatable [`TransformDef`].
///
/// Errors on unknown channels, non-numeric channel values, negative
/// durations, empty `choice` blocks, `repeat` with nothing to loop, and
/// `on …` nested inside other blocks. Top-level `on show/hide/replace`
/// blocks become handler steps (unknown events are rejected).
pub fn lower_transform(
    name: &str,
    steps: &[velvet_script_ast::AtlStep],
) -> Result<TransformDef, String> {
    use velvet_script_ast::{AtlStep, AtlValue};
    fn lower_steps(steps: &[AtlStep], owner: &str) -> Result<Vec<AtlStepRt>, String> {
        steps.iter().map(|s| lower_step(s, owner, false)).collect()
    }
    fn lower_step(step: &AtlStep, owner: &str, top: bool) -> Result<AtlStepRt, String> {
        match step {
            AtlStep::Set {
                props,
                duration,
                warper,
                ..
            } => {
                if *duration < 0.0 {
                    return Err(format!(
                        "transform `{owner}` has a negative duration ({duration})"
                    ));
                }
                let mut out = Vec::new();
                for (channel, value) in props {
                    if !ATL_CHANNELS.contains(&channel.as_str()) {
                        return Err(format!(
                            "transform `{owner}` uses unknown channel `{channel}` (expected one of: {})",
                            ATL_CHANNELS.join(", ")
                        ));
                    }
                    let target = match value {
                        AtlValue::Number(n) => *n as f32,
                        AtlValue::Keyword(w) => {
                            return Err(format!(
                                "channel `{channel}` in transform `{owner}` needs a number, got `{w}`"
                            ));
                        }
                    };
                    out.push((channel.clone(), target));
                }
                Ok(AtlStepRt::Set {
                    props: out,
                    duration: *duration as f32,
                    warper: warper.clone(),
                })
            }
            AtlStep::Parallel { steps, .. } => Ok(AtlStepRt::Parallel(lower_steps(steps, owner)?)),
            AtlStep::Choice { steps, .. } => {
                if steps.is_empty() {
                    return Err(format!("transform `{owner}` has an empty choice block"));
                }
                Ok(AtlStepRt::Choice(lower_steps(steps, owner)?))
            }
            AtlStep::Repeat { count, steps, .. } => {
                if steps.is_empty() {
                    return Err(format!(
                        "transform `{owner}` repeats nothing (repeat needs steps to loop)"
                    ));
                }
                Ok(AtlStepRt::Repeat {
                    count: *count,
                    steps: lower_steps(steps, owner)?,
                })
            }
            AtlStep::On { .. } => {
                if top {
                    unreachable!("top-level `on` is split into handlers before lowering steps")
                } else {
                    Err(format!(
                        "transform `{owner}` allows `on show/hide` only at the top level"
                    ))
                }
            }
        }
    }
    let mut main = Vec::new();
    let mut on_show = Vec::new();
    let mut on_hide = Vec::new();
    let mut on_replace = Vec::new();
    for step in steps {
        match step {
            velvet_script_ast::AtlStep::On { events, steps, .. } => {
                let rt = lower_steps(steps, name)?;
                for ev in events {
                    match ev.as_str() {
                        "show" => on_show.extend(rt.clone()),
                        "hide" => on_hide.extend(rt.clone()),
                        "replace" => on_replace.extend(rt.clone()),
                        other => {
                            return Err(format!(
                                "transform `{name}` has unknown event `{other}` (expected show, hide or replace)"
                            ));
                        }
                    }
                }
            }
            other => main.push(lower_step(other, name, true)?),
        }
    }
    Ok(TransformDef {
        name: name.to_string(),
        steps: main,
        on_show,
        on_hide,
        on_replace,
    })
}

/// Map a warper name to [`Ease`] (`ease` = cubic in-out, Ren'Py default).
pub fn warper_ease(name: &str) -> Ease {
    match name {
        "linear" => Ease::Linear,
        "ease" | "ease_in_out" => Ease::CubicInOut,
        "ease_in" => Ease::CubicIn,
        "ease_out" => Ease::CubicOut,
        "quad_in" => Ease::QuadIn,
        "quad_out" => Ease::QuadOut,
        "quad_in_out" => Ease::QuadInOut,
        "cubic_in" => Ease::CubicIn,
        "cubic_out" => Ease::CubicOut,
        "cubic_in_out" => Ease::CubicInOut,
        "sine_in" => Ease::SineIn,
        "sine_out" => Ease::SineOut,
        "sine_in_out" => Ease::SineInOut,
        "back_in" => Ease::BackIn,
        "back_out" => Ease::BackOut,
        "back_in_out" => Ease::BackInOut,
        "elastic_in" => Ease::ElasticIn,
        "elastic_out" => Ease::ElasticOut,
        "elastic_in_out" => Ease::ElasticInOut,
        "bounce_in" => Ease::BounceIn,
        "bounce_out" => Ease::BounceOut,
        "bounce_in_out" => Ease::BounceInOut,
        _ => Ease::Linear,
    }
}

fn step_list_duration(steps: &[AtlStepRt]) -> Option<f32> {
    let mut total = 0.0f32;
    for step in steps {
        total += step_duration(step)?;
    }
    Some(total)
}

fn step_duration(step: &AtlStepRt) -> Option<f32> {
    match step {
        AtlStepRt::Set { duration, .. } => Some(duration.max(0.0)),
        AtlStepRt::Parallel(children) => {
            let mut max = 0.0f32;
            for child in children {
                max = max.max(step_duration(child)?);
            }
            Some(max)
        }
        AtlStepRt::Choice(children) => {
            // Worst case across variants (host picks at runtime).
            let mut max = 0.0f32;
            for child in children {
                max = max.max(step_duration(child)?);
            }
            Some(max)
        }
        AtlStepRt::Repeat { count, steps } => {
            let cycle = step_list_duration(steps)?;
            match count {
                Some(n) => Some(cycle * *n as f32),
                None => {
                    if cycle <= 0.0 {
                        Some(0.0)
                    } else {
                        None
                    }
                }
            }
        }
    }
}

/// Sample a transform at time `t` (seconds) over an incoming `base` pose.
///
/// Returns channel values. `variant` selects `choice` branches
/// (host-provided, e.g. random). Pure: same inputs, same pose.
pub fn sample_transform(
    def: &TransformDef,
    base: &[(String, f32)],
    t: f32,
    variant: usize,
) -> Vec<(String, f32)> {
    let mut pose: Vec<(String, f32)> = base.to_vec();
    sample_steps(&def.steps, &mut pose, t.max(0.0), variant);
    pose
}

fn pose_get(pose: &[(String, f32)], channel: &str) -> f32 {
    pose.iter()
        .rev()
        .find(|(c, _)| c == channel)
        .map(|(_, v)| *v)
        .unwrap_or(0.0)
}

fn pose_set(pose: &mut Vec<(String, f32)>, channel: &str, value: f32) {
    match pose.iter_mut().rev().find(|(c, _)| *c == channel) {
        Some(slot) => slot.1 = value,
        None => pose.push((channel.to_string(), value)),
    }
}

fn sample_steps(steps: &[AtlStepRt], pose: &mut Vec<(String, f32)>, mut t: f32, variant: usize) {
    for step in steps {
        if t < 0.0 {
            return;
        }
        match step {
            AtlStepRt::Set {
                props,
                duration,
                warper,
            } => {
                let dur = duration.max(0.0);
                // Snapshot start values on first touch within this step.
                let starts: Vec<(String, f32, f32)> = props
                    .iter()
                    .map(|(c, target)| (c.clone(), pose_get(pose, c), *target))
                    .collect();
                if dur <= 0.0 || t >= dur {
                    for (c, _, target) in &starts {
                        pose_set(pose, c, *target);
                    }
                    t -= dur;
                    continue;
                }
                let k = warper_ease(warper).eval((t / dur).clamp(0.0, 1.0));
                for (c, from, target) in &starts {
                    pose_set(pose, c, from + (target - from) * k);
                }
                return;
            }
            AtlStepRt::Parallel(children) => {
                // Longest child sets the clock; shorter ones hold end values.
                let mut longest = 0.0f32;
                for child in children {
                    if let Some(d) = step_duration(child) {
                        longest = longest.max(d);
                    }
                }
                let mut merged = pose.clone();
                for child in children {
                    let mut branch = pose.clone();
                    sample_steps(
                        std::slice::from_ref(child),
                        &mut branch,
                        t.min(longest),
                        variant,
                    );
                    for (c, v) in branch {
                        pose_set(&mut merged, &c, v);
                    }
                }
                *pose = merged;
                t -= longest;
            }
            AtlStepRt::Choice(children) => {
                if children.is_empty() {
                    continue;
                }
                let pick = &children[variant % children.len()];
                // A bare child step behaves like a one-item sequence.
                let before = pose.clone();
                sample_steps(std::slice::from_ref(pick), pose, t, variant);
                // Account the longest variant so siblings stay in sync.
                let mut longest = 0.0f32;
                for child in children {
                    if let Some(d) = step_duration(child) {
                        longest = longest.max(d);
                    }
                }
                let _ = before;
                t -= longest;
            }
            AtlStepRt::Repeat { count, steps } => {
                let cycle = step_list_duration(steps).unwrap_or(0.0);
                if cycle <= 0.0 {
                    continue;
                }
                match count {
                    Some(n) => {
                        let total = cycle * *n as f32;
                        if t >= total {
                            // run out fully, keep end pose
                            let mut tmp = pose.clone();
                            for _ in 0..*n {
                                sample_steps(steps, &mut tmp, cycle, variant);
                            }
                            *pose = tmp;
                            t -= total;
                        } else {
                            let full = (t / cycle).floor() as u32;
                            let mut tmp = pose.clone();
                            for _ in 0..full {
                                sample_steps(steps, &mut tmp, cycle, variant);
                            }
                            *pose = tmp;
                            sample_steps(steps, pose, t - full as f32 * cycle, variant);
                            return;
                        }
                    }
                    None => {
                        // Infinite: fold whole cycles, sample the remainder.
                        let full = (t / cycle).floor() as u32;
                        let mut tmp = pose.clone();
                        for _ in 0..full.min(1024) {
                            sample_steps(steps, &mut tmp, cycle, variant);
                        }
                        *pose = tmp;
                        sample_steps(steps, pose, t - full as f32 * cycle, variant);
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_show_variants() {
        let (t, p, m) = parse_show_with_motion("luna happy at left with shake 0.4 10");
        assert_eq!(t, "luna happy");
        assert_eq!(p.as_deref(), Some("left"));
        let m = m.unwrap();
        assert_eq!(m.kind, MotionKind::Shake);
        assert!((m.duration - 0.4).abs() < 1e-5);

        let (t, p, m) = parse_show_with_motion("luna at right with move");
        assert_eq!(t, "luna");
        assert_eq!(p.as_deref(), Some("right"));
        assert_eq!(m.unwrap().kind, MotionKind::Move);

        let (t, _, m) = parse_show_with_motion("luna");
        assert_eq!(t, "luna");
        assert!(m.is_none());
    }

    #[test]
    fn motion_curves() {
        let s = SpriteMotion::new(MotionKind::Shake, 0.4, 10.0);
        assert_eq!(s.shake_offset(0.0), 0.0);
        assert!(s.shake_offset(0.05).abs() > 0.01);
        assert_eq!(s.shake_offset(10.0), 0.0);
        let z = SpriteMotion::new(MotionKind::Zoom, 0.4, 0.2);
        assert!((z.zoom_scale(0.0) - 1.0).abs() < 1e-5);
        assert!(z.zoom_scale(0.16) > 1.0);
        let f = SpriteMotion::new(MotionKind::Fade, 0.5, 0.0);
        assert_eq!(f.fade_alpha(0.0), 0.0);
        assert_eq!(f.fade_alpha(5.0), 1.0);
    }

    #[test]
    fn unknown_motion_none() {
        assert!(parse_motion_words("spin 1.0").is_none());
        assert!(MotionKind::parse("zoom").is_some());
    }

    #[test]
    fn sample_sequential_sets() {
        let def = TransformDef {
            name: "bob".into(),
            steps: vec![
                AtlStepRt::Set {
                    props: vec![("y".into(), -8.0)],
                    duration: 1.0,
                    warper: "linear".into(),
                },
                AtlStepRt::Set {
                    props: vec![("y".into(), 0.0)],
                    duration: 1.0,
                    warper: "linear".into(),
                },
            ],
            ..Default::default()
        };
        assert_eq!(def.duration(), Some(2.0));
        let at = |t: f32| pose_get(&sample_transform(&def, &[], t, 0), "y");
        assert!((at(0.0) - 0.0).abs() < 1e-5);
        assert!((at(0.5) - -4.0).abs() < 1e-4);
        assert!((at(1.0) - -8.0).abs() < 1e-4);
        assert!((at(2.0) - 0.0).abs() < 1e-4);
        // Past the end the pose holds.
        assert!((at(9.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn sample_repeat_loops_and_parallel_merges() {
        let def = TransformDef {
            name: "spin".into(),
            steps: vec![AtlStepRt::Repeat {
                count: None,
                steps: vec![AtlStepRt::Parallel(vec![
                    AtlStepRt::Set {
                        props: vec![("x".into(), 10.0)],
                        duration: 1.0,
                        warper: "linear".into(),
                    },
                    AtlStepRt::Set {
                        props: vec![("y".into(), 4.0)],
                        duration: 2.0,
                        warper: "linear".into(),
                    },
                ])],
            }],
            ..Default::default()
        };
        assert_eq!(def.duration(), None);
        let pose = sample_transform(&def, &[], 0.5, 0);
        assert!((pose_get(&pose, "x") - 5.0).abs() < 1e-4);
        assert!((pose_get(&pose, "y") - 1.0).abs() < 1e-4);
        // Second cycle re-executes from the current pose (Ren'Py: a
        // statement starts from the value at loop restart), so x holds
        // its end value while y keeps interpolating toward its own end.
        let pose = sample_transform(&def, &[], 2.5, 0);
        assert!((pose_get(&pose, "x") - 10.0).abs() < 1e-4);
        assert!((pose_get(&pose, "y") - 4.0).abs() < 1e-4);
    }

    #[test]
    fn on_handlers_split_and_sample() {
        use velvet_script_ast::{AtlStep, AtlValue, SourceLoc};
        let loc = SourceLoc::unknown();
        let set = |channel: &str, target: f64, duration: f64| AtlStep::Set {
            props: vec![(channel.into(), AtlValue::Number(target))],
            duration,
            warper: "linear".into(),
            loc: loc.clone(),
        };
        let def = lower_transform(
            "pop",
            &[
                AtlStep::On {
                    events: vec!["show".into()],
                    steps: vec![set("scale", 1.2, 0.2)],
                    loc: loc.clone(),
                },
                AtlStep::On {
                    events: vec!["hide".into(), "replace".into()],
                    steps: vec![set("opacity", 0.0, 0.3)],
                    loc: loc.clone(),
                },
                set("y", -4.0, 1.0),
            ],
        )
        .unwrap();
        assert_eq!(def.steps.len(), 1);
        assert_eq!(def.handler(AtlEvent::Show).len(), 1);
        assert_eq!(def.handler(AtlEvent::Hide).len(), 1);
        assert_eq!(def.handler(AtlEvent::Replace).len(), 1);
        assert_eq!(def.handler_duration(AtlEvent::Show), Some(0.2));
        let pose = sample_transform_on(&def, AtlEvent::Show, &[], 0.1, 0);
        assert!((pose_get(&pose, "scale") - 0.6).abs() < 1e-4);
        // Nested `on` is rejected.
        let err = lower_transform(
            "bad",
            &[AtlStep::Parallel {
                steps: vec![AtlStep::On {
                    events: vec!["show".into()],
                    steps: vec![set("x", 1.0, 0.1)],
                    loc: loc.clone(),
                }],
                loc,
            }],
        )
        .unwrap_err();
        assert!(err.contains("top level"), "{err}");
    }

    #[test]
    fn next_variant_uses_seed() {
        let mut seed = 1u64;
        let a = next_variant(3, &mut seed);
        let b = next_variant(3, &mut seed);
        assert!(a < 3 && b < 3);
        // Same seed replays the same pick.
        let mut seed2 = 1u64;
        assert_eq!(next_variant(3, &mut seed2), a);
        assert_eq!(next_variant(0, &mut seed2), 0);
    }

    #[test]
    fn lower_rejects_bad_channels() {
        use velvet_script_ast::{AtlStep, AtlValue, SourceLoc};
        let loc = SourceLoc::unknown();
        let err = lower_transform(
            "bad",
            &[AtlStep::Set {
                props: vec![("spin".into(), AtlValue::Number(1.0))],
                duration: 1.0,
                warper: "linear".into(),
                loc: loc.clone(),
            }],
        )
        .unwrap_err();
        assert!(err.contains("spin"), "{err}");
        let err = lower_transform(
            "bad",
            &[AtlStep::Repeat {
                count: None,
                steps: Vec::new(),
                loc,
            }],
        )
        .unwrap_err();
        assert!(err.contains("repeats nothing"), "{err}");
    }
}
