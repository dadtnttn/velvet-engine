//! Story host: `call anim.fx` / `anim.move` / `anim.script` → [`AnimDirector`].

use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_math::Vec2;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};

use crate::director::AnimDirector;
use crate::effect::{EffectKind, EffectParams};
use crate::script::{apply_program_immediate, parse_anim_script};
use crate::tween::parse_ease;

/// Shared animation director for games / VN hosts.
pub struct AnimStoryHost {
    /// Director state.
    pub director: Mutex<AnimDirector>,
}

impl AnimStoryHost {
    /// New empty director.
    pub fn new() -> Self {
        Self {
            director: Mutex::new(AnimDirector::new()),
        }
    }

    /// Tick animations (call from game loop).
    pub fn tick(&self, dt: f32) {
        if let Ok(mut d) = self.director.lock() {
            d.tick(dt);
        }
    }
}

impl Default for AnimStoryHost {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryCommandHost for AnimStoryHost {
    fn call(
        &self,
        name: &str,
        args: &IndexMap<String, StoryValue>,
        vars: &mut StoryVariables,
    ) -> Result<CommandOutcome, StoryCommandError> {
        match name {
            "anim.fx" => {
                let target = arg_str(args, "target").unwrap_or_else(|| "default".into());
                let effect = arg_str(args, "effect").unwrap_or_else(|| "fade_in".into());
                let kind = EffectKind::parse(&effect).ok_or_else(|| {
                    StoryCommandError::new(format!("unknown effect `{effect}`"))
                })?;
                let x = arg_f32(args, "x").unwrap_or(0.0);
                let y = arg_f32(args, "y").unwrap_or(0.0);
                let duration = arg_f32(args, "duration").unwrap_or(0.35);
                let delay = arg_f32(args, "delay").unwrap_or(0.0);
                let strength = arg_f32(args, "strength").unwrap_or(8.0);
                let ease = arg_str(args, "ease").unwrap_or_else(|| "cubic_out".into());
                let params = EffectParams {
                    to: Vec2::new(x, y),
                    duration,
                    delay,
                    strength,
                    ease: parse_ease(&ease),
                };
                if let Ok(mut d) = self.director.lock() {
                    d.ensure(&target);
                    d.play_effect(&target, kind, params);
                }
                vars.set("anim.last_target", StoryValue::String(target.clone()));
                vars.set("anim.last_effect", StoryValue::String(effect));
                Ok(CommandOutcome::Continue)
            }
            "anim.move" => {
                let target = arg_str(args, "target").unwrap_or_else(|| "default".into());
                let x = arg_f32(args, "x").unwrap_or(0.0);
                let y = arg_f32(args, "y").unwrap_or(0.0);
                let duration = arg_f32(args, "duration").unwrap_or(0.3);
                let ease = arg_str(args, "ease").unwrap_or_else(|| "cubic_out".into());
                if let Ok(mut d) = self.director.lock() {
                    d.move_to(&target, x, y, duration, &ease);
                }
                vars.set("anim.last_target", StoryValue::String(target));
                Ok(CommandOutcome::Continue)
            }
            "anim.stop" => {
                let target = arg_str(args, "target").unwrap_or_else(|| "default".into());
                if let Ok(mut d) = self.director.lock() {
                    d.stop(&target);
                }
                Ok(CommandOutcome::Continue)
            }
            "anim.script" => {
                // Inline mini-script in `body` or `code` arg
                let body = arg_str(args, "body")
                    .or_else(|| arg_str(args, "code"))
                    .unwrap_or_default();
                let prog = parse_anim_script(&body)
                    .map_err(|e| StoryCommandError::new(format!("anim.script: {e}")))?;
                if let Ok(mut d) = self.director.lock() {
                    let wait = apply_program_immediate(&mut d, &prog);
                    vars.set("anim.script_wait", StoryValue::Float(wait as f64));
                }
                Ok(CommandOutcome::Continue)
            }
            _ => Ok(CommandOutcome::Continue),
        }
    }
}

fn arg_str(args: &IndexMap<String, StoryValue>, key: &str) -> Option<String> {
    args.get(key).map(|v| v.display_str())
}

fn arg_f32(args: &IndexMap<String, StoryValue>, key: &str) -> Option<f32> {
    args.get(key).and_then(|v| match v {
        StoryValue::Int(i) => Some(*i as f32),
        StoryValue::Float(f) => Some(*f as f32),
        StoryValue::String(s) => s.parse().ok(),
        _ => v.as_i64().map(|i| i as f32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use velvet_story::{StoryOp, StoryPlayer, StoryProgram, StoryScene};

    #[test]
    fn host_anim_fx_from_story_call() {
        let host = Arc::new(AnimStoryHost::new());
        let shared: velvet_story::SharedCommandHost = host.clone();

        let mut scenes = IndexMap::new();
        let mut args = IndexMap::new();
        args.insert("target".into(), StoryValue::String("card0".into()));
        args.insert("effect".into(), StoryValue::String("deal".into()));
        args.insert("x".into(), StoryValue::Float(150.0));
        args.insert("y".into(), StoryValue::Float(280.0));
        args.insert("duration".into(), StoryValue::Float(0.25));
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::HostCall {
                        name: "anim.fx".into(),
                        args,
                    },
                    StoryOp::End { ending: None },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("anim_host");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let player = StoryPlayer::start_with_host(prog, shared);
        // pump already ran host on start
        assert_eq!(
            player.variables().get("anim.last_effect").display_str(),
            "deal"
        );
        {
            let d = host.director.lock().unwrap();
            assert!(d.targets.contains_key("card0"));
            assert!(!d.pose("card0").unwrap().opacity.is_nan());
        }
        for _ in 0..20 {
            host.tick(1.0 / 60.0);
        }
        let p = host.director.lock().unwrap().pose("card0").cloned().unwrap();
        assert!(p.opacity > 0.2);
        assert!(player.is_ended() || player.ending().is_none());
    }
}
