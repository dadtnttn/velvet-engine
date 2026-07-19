//! Story host: `call anim.fx` / `anim.move` / `anim.script` / `anim.pack_open`.

use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_math::Vec2;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};

use crate::director::AnimDirector;
use crate::effect::{EffectKind, EffectParams};
use crate::fx3d::{Fx3dCamera, PackOpenFx, ProjectedQuad};
use crate::script::{apply_program_immediate, parse_anim_script};
use crate::tween::parse_ease;

/// Shared animation director + optional 3D pack-open generator.
pub struct AnimStoryHost {
    /// 2D director state.
    pub director: Mutex<AnimDirector>,
    /// Active pack-open cinematic (if any).
    pub pack: Mutex<Option<PackOpenFx>>,
    /// Camera for projecting pack/card quads.
    pub camera: Mutex<Fx3dCamera>,
}

impl AnimStoryHost {
    /// New empty director.
    pub fn new() -> Self {
        Self {
            director: Mutex::new(AnimDirector::new()),
            pack: Mutex::new(None),
            camera: Mutex::new(Fx3dCamera::default()),
        }
    }

    /// Tick animations (call from game loop).
    pub fn tick(&self, dt: f32) {
        if let Ok(mut d) = self.director.lock() {
            d.tick(dt);
        }
        if let Ok(mut p) = self.pack.lock() {
            if let Some(fx) = p.as_mut() {
                fx.tick(dt);
                if fx.is_done() {
                    // keep final frame; don't clear so renderer can hold
                }
            }
        }
    }

    /// Snapshot pack-open projected layers (empty if none).
    pub fn pack_projected(&self) -> Vec<(String, ProjectedQuad)> {
        let cam = self.camera.lock().ok().map(|c| *c).unwrap_or_default();
        self.pack
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|fx| fx.projected(&cam)))
            .unwrap_or_default()
    }

    /// Whether a pack-open is running or finished with layers.
    pub fn has_pack(&self) -> bool {
        self.pack.lock().ok().map(|g| g.is_some()).unwrap_or(false)
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
                let body = arg_str(args, "body")
                    .or_else(|| arg_str(args, "code"))
                    .unwrap_or_default();
                let prog = parse_anim_script(&body)
                    .map_err(|e| StoryCommandError::new(format!("anim.script: {e}")))?;
                if let Ok(mut d) = self.director.lock() {
                    let wait = apply_program_immediate(&mut d, &prog);
                    vars.set("anim.script_wait", StoryValue::Float(wait as f64));
                }
                // pack_open ops inside script applied via apply — need extend apply_program
                Ok(CommandOutcome::Continue)
            }
            "anim.pack_open" => {
                let x = arg_f32(args, "x").unwrap_or(480.0);
                let y = arg_f32(args, "y").unwrap_or(270.0);
                let cards = arg_f32(args, "cards").unwrap_or(5.0).round() as usize;
                let duration = arg_f32(args, "duration").unwrap_or(2.2);
                let seed = arg_f32(args, "seed").unwrap_or(1.0) as u64;
                let mut params = crate::fx3d::PackOpenParams {
                    center: Vec2::new(x, y),
                    card_count: cards.max(1),
                    duration: duration.max(0.5),
                    seed,
                    ..Default::default()
                };
                if let Some(s) = arg_f32(args, "fan_spacing") {
                    params.fan_spacing = s;
                }
                let fx = PackOpenFx::start(params);
                if let Ok(mut slot) = self.pack.lock() {
                    *slot = Some(fx);
                }
                vars.set("anim.pack_open", StoryValue::Bool(true));
                vars.set("anim.pack_cards", StoryValue::Int(cards.max(1) as i64));
                vars.set(
                    "anim.last_effect",
                    StoryValue::String("pack_open".into()),
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
        assert_eq!(
            player.variables().get("anim.last_effect").display_str(),
            "deal"
        );
        {
            let d = host.director.lock().unwrap();
            assert!(d.targets.contains_key("card0"));
        }
        for _ in 0..20 {
            host.tick(1.0 / 60.0);
        }
        let p = host.director.lock().unwrap().pose("card0").cloned().unwrap();
        assert!(p.opacity > 0.2);
    }

    #[test]
    fn host_pack_open_generates_layers() {
        let host = Arc::new(AnimStoryHost::new());
        let shared: velvet_story::SharedCommandHost = host.clone();
        let mut scenes = IndexMap::new();
        let mut args = IndexMap::new();
        args.insert("x".into(), StoryValue::Float(400.0));
        args.insert("y".into(), StoryValue::Float(300.0));
        args.insert("cards".into(), StoryValue::Int(4));
        args.insert("duration".into(), StoryValue::Float(1.0));
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::HostCall {
                        name: "anim.pack_open".into(),
                        args,
                    },
                    StoryOp::End { ending: None },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("pack");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let player = StoryPlayer::start_with_host(prog, shared);
        assert!(player.variables().get("anim.pack_open").is_truthy());
        assert!(host.has_pack());
        for _ in 0..90 {
            host.tick(1.0 / 60.0);
        }
        let quads = host.pack_projected();
        assert!(
            quads.iter().any(|(id, q)| id.starts_with("card") && q.opacity > 0.2),
            "{:?}",
            quads.iter().map(|(i, q)| (i, q.opacity)).collect::<Vec<_>>()
        );
    }
}
