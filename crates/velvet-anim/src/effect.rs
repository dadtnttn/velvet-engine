//! Named effect presets (cards, UI, sprites).

use velvet_math::{Ease, Vec2};

use crate::pose::{AnimField, AnimPose};
use crate::tween::{apply_field, read_field, FloatTween};

/// Built-in effect kinds authors reference from script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    /// Fade opacity 0 → 1.
    FadeIn,
    /// Fade opacity 1 → 0.
    FadeOut,
    /// Move to a point.
    MoveTo,
    /// Scale punch (grow then settle).
    Punch,
    /// Screen/target shake on X.
    Shake,
    /// Deal card: move + fade + slight scale (from offscreen-ish).
    Deal,
    /// Bounce scale in.
    BounceIn,
    /// Pop scale 0 → 1 with back ease.
    PopIn,
}

impl EffectKind {
    /// Parse author string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fade_in" | "fadein" | "appear" => Some(Self::FadeIn),
            "fade_out" | "fadeout" | "hide" => Some(Self::FadeOut),
            "move" | "move_to" | "moveto" => Some(Self::MoveTo),
            "punch" | "scale_punch" => Some(Self::Punch),
            "shake" => Some(Self::Shake),
            "deal" | "card_deal" => Some(Self::Deal),
            "bounce" | "bounce_in" | "bouncein" => Some(Self::BounceIn),
            "pop" | "pop_in" | "popin" => Some(Self::PopIn),
            _ => None,
        }
    }

    /// Canonical author name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FadeIn => "fade_in",
            Self::FadeOut => "fade_out",
            Self::MoveTo => "move",
            Self::Punch => "punch",
            Self::Shake => "shake",
            Self::Deal => "deal",
            Self::BounceIn => "bounce",
            Self::PopIn => "pop",
        }
    }
}

/// Parameters for spawning an effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParams {
    /// Destination / amplitude helper.
    pub to: Vec2,
    /// Duration seconds (primary).
    pub duration: f32,
    /// Strength (shake pixels, punch scale add).
    pub strength: f32,
    /// Optional delay.
    pub delay: f32,
    /// Ease override (used when relevant).
    pub ease: Ease,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            to: Vec2::ZERO,
            duration: 0.35,
            strength: 8.0,
            delay: 0.0,
            ease: Ease::CubicOut,
        }
    }
}

/// Build tweens for an effect applied to `pose` (current).
pub fn build_effect(
    kind: EffectKind,
    pose: &AnimPose,
    params: EffectParams,
) -> Vec<FloatTween> {
    let d = params.duration.max(1e-4);
    let delay = params.delay;
    let mut out = Vec::new();

    match kind {
        EffectKind::FadeIn => {
            let mut t = FloatTween::new(AnimField::Opacity, 0.0, 1.0, d, params.ease);
            t.delay = delay;
            // ensure start hidden
            let _ = pose;
            out.push(t);
        }
        EffectKind::FadeOut => {
            let mut t = FloatTween::new(
                AnimField::Opacity,
                pose.opacity,
                0.0,
                d,
                params.ease,
            );
            t.delay = delay;
            out.push(t);
        }
        EffectKind::MoveTo => {
            let mut tx =
                FloatTween::new(AnimField::X, pose.pos.x, params.to.x, d, params.ease);
            let mut ty =
                FloatTween::new(AnimField::Y, pose.pos.y, params.to.y, d, params.ease);
            tx.delay = delay;
            ty.delay = delay;
            out.push(tx);
            out.push(ty);
        }
        EffectKind::Punch => {
            let peak = 1.0 + params.strength.max(0.05);
            let half = d * 0.45;
            let mut up =
                FloatTween::new(AnimField::Scale, pose.scale, peak, half, Ease::BackOut);
            up.delay = delay;
            let mut down =
                FloatTween::new(AnimField::Scale, peak, 1.0, d - half, Ease::CubicOut);
            down.delay = delay + half;
            out.push(up);
            out.push(down);
        }
        EffectKind::Shake => {
            // 3 half-swings on X
            let amp = params.strength;
            let base = pose.pos.x;
            let seg = d / 4.0;
            let mut t0 = FloatTween::new(AnimField::X, base, base + amp, seg, Ease::SineOut);
            t0.delay = delay;
            let mut t1 =
                FloatTween::new(AnimField::X, base + amp, base - amp, seg * 2.0, Ease::SineInOut);
            t1.delay = delay + seg;
            let mut t2 =
                FloatTween::new(AnimField::X, base - amp, base, seg, Ease::SineIn);
            t2.delay = delay + seg * 3.0;
            out.push(t0);
            out.push(t1);
            out.push(t2);
        }
        EffectKind::Deal => {
            // From slightly above + faded → rest pose at `to` or keep xy if to is zero-ish use current
            let dest = if params.to.length_squared() > 1e-6 {
                params.to
            } else {
                pose.pos
            };
            let start = Vec2::new(dest.x, dest.y - 80.0);
            let mut ox = FloatTween::new(AnimField::X, start.x, dest.x, d, Ease::CubicOut);
            let mut oy = FloatTween::new(AnimField::Y, start.y, dest.y, d, Ease::BackOut);
            let mut op = FloatTween::new(AnimField::Opacity, 0.0, 1.0, d * 0.7, Ease::QuadOut);
            let mut sc =
                FloatTween::new(AnimField::Scale, 0.85, 1.0, d, Ease::BackOut);
            ox.delay = delay;
            oy.delay = delay;
            op.delay = delay;
            sc.delay = delay;
            out.push(ox);
            out.push(oy);
            out.push(op);
            out.push(sc);
        }
        EffectKind::BounceIn => {
            let mut t =
                FloatTween::new(AnimField::Scale, 0.0, 1.0, d, Ease::BounceOut);
            t.delay = delay;
            let mut o = FloatTween::new(AnimField::Opacity, 0.0, 1.0, d * 0.5, Ease::QuadOut);
            o.delay = delay;
            out.push(t);
            out.push(o);
        }
        EffectKind::PopIn => {
            let mut t = FloatTween::new(AnimField::Scale, 0.0, 1.0, d, Ease::BackOut);
            t.delay = delay;
            let mut o = FloatTween::new(AnimField::Opacity, 0.0, 1.0, d * 0.4, Ease::Linear);
            o.delay = delay;
            out.push(t);
            out.push(o);
        }
    }
    out
}

/// Apply finished-less concurrent tweens onto pose for one frame of sampling.
pub fn sample_tweens(pose: &mut AnimPose, tweens: &[FloatTween]) {
    for t in tweens {
        let v = t.sample();
        // For sequential same-field, later tweens that haven't started should not overwrite
        // with `from` if delay not elapsed — sample already returns from during delay.
        // Prefer highest elapsed among same field: apply in order; later wins if started.
        if t.elapsed >= t.delay {
            apply_field(pose, t.field, v);
        }
    }
}

/// Tick all tweens and apply (later started fields overwrite).
pub fn tick_tweens(pose: &mut AnimPose, tweens: &mut [FloatTween], dt: f32) {
    // Group: apply each tween's value if active
    for t in tweens.iter_mut() {
        let v = t.tick(dt);
        if t.elapsed >= t.delay {
            apply_field(pose, t.field, v);
        }
    }
    let _ = read_field;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deal_moves_and_fades() {
        let pose = AnimPose::hidden(Vec2::new(100.0, 200.0));
        let tw = build_effect(
            EffectKind::Deal,
            &pose,
            EffectParams {
                to: Vec2::new(100.0, 200.0),
                duration: 0.4,
                ..Default::default()
            },
        );
        assert!(tw.len() >= 3);
        assert!(tw.iter().any(|t| t.field == AnimField::Opacity));
        assert!(tw.iter().any(|t| t.field == AnimField::Y));
    }

    #[test]
    fn parse_kinds() {
        assert_eq!(EffectKind::parse("deal"), Some(EffectKind::Deal));
        assert_eq!(EffectKind::parse("FADE_IN"), Some(EffectKind::FadeIn));
        assert!(EffectKind::parse("nope").is_none());
    }
}
