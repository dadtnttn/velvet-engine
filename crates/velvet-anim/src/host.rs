//! Story host: **generic tools** — set pose3d channels, play timelines, 2D fx.
//!
//! No premade pack-open cutscene. Authors compose motion via `anim.pose3d`,
//! `anim.track`, or `.vanim` track lines.

use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_math::Vec2;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};

use crate::director::AnimDirector;
use crate::effect::{EffectKind, EffectParams};
use crate::fx3d::{Fx3dCamera, ImageBillboard, Pose3D, Pose3DChannel, ProjectedQuad};
use crate::script::{apply_program_immediate, parse_anim_script};
use crate::track::{ChannelTrack, Timeline};
use crate::tween::parse_ease;

/// Shared animation tools host for games / VN.
pub struct AnimStoryHost {
    /// 2D director.
    pub director: Mutex<AnimDirector>,
    /// Named 3D billboards (you set poses / tracks).
    pub billboards: Mutex<IndexMap<String, ImageBillboard>>,
    /// Named timelines (tools).
    pub timelines: Mutex<IndexMap<String, Timeline>>,
    /// Camera for projection tool.
    pub camera: Mutex<Fx3dCamera>,
}

impl AnimStoryHost {
    /// Empty host.
    pub fn new() -> Self {
        Self {
            director: Mutex::new(AnimDirector::new()),
            billboards: Mutex::new(IndexMap::new()),
            timelines: Mutex::new(IndexMap::new()),
            camera: Mutex::new(Fx3dCamera::default()),
        }
    }

    /// Tick 2D director + all timelines onto their billboards.
    pub fn tick(&self, dt: f32) {
        if let Ok(mut d) = self.director.lock() {
            d.tick(dt);
        }
        let mut tls = match self.timelines.lock() {
            Ok(t) => t,
            Err(_) => return,
        };
        let mut boards = match self.billboards.lock() {
            Ok(b) => b,
            Err(_) => return,
        };
        for (id, tl) in tls.iter_mut() {
            tl.tick(dt);
            if let Some(b) = boards.get_mut(id) {
                tl.apply(&mut b.pose);
            }
        }
    }

    /// Project all billboards (tool output for your renderer).
    pub fn project_all(&self) -> Vec<(String, ProjectedQuad)> {
        let cam = self.camera.lock().ok().map(|c| *c).unwrap_or_default();
        let boards = match self.billboards.lock() {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };
        let mut out: Vec<(String, ProjectedQuad)> = boards
            .values()
            .map(|b| (b.id.clone(), b.project(&cam)))
            .collect();
        out.sort_by(|a, b| {
            b.1.sort_z
                .partial_cmp(&a.1.sort_z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }

    fn ensure_billboard(&self, id: &str) {
        if let Ok(mut b) = self.billboards.lock() {
            if !b.contains_key(id) {
                b.insert(
                    id.into(),
                    ImageBillboard::new(id, Pose3D::default(), 70.0, 100.0),
                );
            }
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
                let params = EffectParams {
                    to: Vec2::new(
                        arg_f32(args, "x").unwrap_or(0.0),
                        arg_f32(args, "y").unwrap_or(0.0),
                    ),
                    duration: arg_f32(args, "duration").unwrap_or(0.35),
                    delay: arg_f32(args, "delay").unwrap_or(0.0),
                    strength: arg_f32(args, "strength").unwrap_or(8.0),
                    ease: parse_ease(&arg_str(args, "ease").unwrap_or_else(|| "cubic_out".into())),
                };
                if let Ok(mut d) = self.director.lock() {
                    d.ensure(&target);
                    d.play_effect(&target, kind, params);
                }
                vars.set("anim.last_target", StoryValue::String(target));
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
                Ok(CommandOutcome::Continue)
            }
            // --- generic 3D tools ---
            "anim.billboard" => {
                // create/update billboard half size + optional content keys
                let id = arg_str(args, "target").unwrap_or_else(|| "card0".into());
                let hw = arg_f32(args, "half_w").unwrap_or(70.0);
                let hh = arg_f32(args, "half_h").unwrap_or(100.0);
                self.ensure_billboard(&id);
                if let Ok(mut b) = self.billboards.lock() {
                    if let Some(bb) = b.get_mut(&id) {
                        bb.half_w = hw;
                        bb.half_h = hh;
                        if let Some(f) = arg_str(args, "front") {
                            bb.front = Some(f);
                        }
                        if let Some(bk) = arg_str(args, "back") {
                            bb.back = Some(bk);
                        }
                        if let Some(x) = arg_f32(args, "x") {
                            bb.pose.pos.x = x;
                        }
                        if let Some(y) = arg_f32(args, "y") {
                            bb.pose.pos.y = y;
                        }
                    }
                }
                vars.set("anim.last_target", StoryValue::String(id));
                Ok(CommandOutcome::Continue)
            }
            "anim.pose3d" => {
                // set any channels: yaw, pitch, roll, opacity, foil, depth, scale, x, y
                let id = arg_str(args, "target").unwrap_or_else(|| "card0".into());
                self.ensure_billboard(&id);
                if let Ok(mut b) = self.billboards.lock() {
                    if let Some(bb) = b.get_mut(&id) {
                        for (k, v) in args {
                            if k == "target" {
                                continue;
                            }
                            if let Some(ch) = Pose3DChannel::parse(k) {
                                let val = match v {
                                    StoryValue::Int(i) => *i as f32,
                                    StoryValue::Float(f) => *f as f32,
                                    StoryValue::String(s) => s.parse().unwrap_or(0.0),
                                    _ => continue,
                                };
                                bb.pose.set_channel(ch, val);
                            }
                        }
                    }
                }
                vars.set("anim.last_target", StoryValue::String(id));
                Ok(CommandOutcome::Continue)
            }
            "anim.track" => {
                // append one channel keyframe track to a named timeline
                // target, channel, t0, v0, t1, v1, ease?
                let id = arg_str(args, "target").unwrap_or_else(|| "card0".into());
                let ch_name = arg_str(args, "channel").ok_or_else(|| {
                    StoryCommandError::new("anim.track requires channel")
                })?;
                let channel = Pose3DChannel::parse(&ch_name).ok_or_else(|| {
                    StoryCommandError::new(format!("bad channel `{ch_name}`"))
                })?;
                let ease = parse_ease(&arg_str(args, "ease").unwrap_or_else(|| "cubic_out".into()));
                // keys as t0 v0 t1 v1 in args keys "k0","k1"... or body "0 0 0.4 3.14"
                let body = arg_str(args, "keys").unwrap_or_default();
                let mut track = ChannelTrack::new(channel);
                let nums: Vec<f32> = body
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if nums.len() >= 2 && nums.len() % 2 == 0 {
                    for pair in nums.chunks(2) {
                        track = track.key(pair[0], pair[1], ease);
                    }
                } else {
                    // single segment from / to / duration
                    let from = arg_f32(args, "from").unwrap_or(0.0);
                    let to = arg_f32(args, "to").unwrap_or(0.0);
                    let dur = arg_f32(args, "duration").unwrap_or(0.4);
                    track = track.key(0.0, from, Ease::Linear).key(dur, to, ease);
                }
                self.ensure_billboard(&id);
                if let Ok(mut tls) = self.timelines.lock() {
                    let tl = tls.entry(id.clone()).or_insert_with(Timeline::new);
                    tl.channels.push(track);
                    tl.playing = true;
                }
                vars.set("anim.last_target", StoryValue::String(id));
                Ok(CommandOutcome::Continue)
            }
            "anim.timeline_clear" => {
                let id = arg_str(args, "target").unwrap_or_else(|| "card0".into());
                if let Ok(mut tls) = self.timelines.lock() {
                    tls.shift_remove(&id);
                }
                Ok(CommandOutcome::Continue)
            }
            _ => Ok(CommandOutcome::Continue),
        }
    }
}

// re-export Ease for track builder in host
use velvet_math::Ease;

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
    fn host_pose3d_and_track_are_tools() {
        let host = Arc::new(AnimStoryHost::new());
        let shared: velvet_story::SharedCommandHost = host.clone();
        let mut scenes = IndexMap::new();
        let mut ops = Vec::new();
        let mut a1 = IndexMap::new();
        a1.insert("target".into(), StoryValue::String("c0".into()));
        a1.insert("x".into(), StoryValue::Float(100.0));
        a1.insert("y".into(), StoryValue::Float(200.0));
        ops.push(StoryOp::HostCall {
            name: "anim.billboard".into(),
            args: a1,
        });
        let mut a2 = IndexMap::new();
        a2.insert("target".into(), StoryValue::String("c0".into()));
        a2.insert("channel".into(), StoryValue::String("yaw".into()));
        a2.insert("from".into(), StoryValue::Float(0.0));
        a2.insert("to".into(), StoryValue::Float(3.1415));
        a2.insert("duration".into(), StoryValue::Float(0.3));
        ops.push(StoryOp::HostCall {
            name: "anim.track".into(),
            args: a2,
        });
        ops.push(StoryOp::End { ending: None });
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops,
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("t");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let _ = StoryPlayer::start_with_host(prog, shared);
        for _ in 0..30 {
            host.tick(1.0 / 60.0);
        }
        let boards = host.billboards.lock().unwrap();
        let yaw = boards.get("c0").unwrap().pose.yaw;
        assert!(yaw > 1.0, "yaw should advance via track tool, got {yaw}");
        drop(boards);
        assert!(!host.project_all().is_empty());
    }
}
