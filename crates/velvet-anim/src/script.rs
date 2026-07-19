//! Legacy `.vanim` line scripts — **converted to `.vcss`**.
//!
//! Prefer writing motion in the unified stylesheet language:
//! `@keyframes` + `animation:` in `.vcss` (see `velvet_style` / `VELVET_STYLE.md`).
//!
//! This module remains as a thin compatibility path:
//! `parse_anim_script` still works; new code should use `velvet_style::vanim_to_vcss`
//! or author `.vcss` directly.

use thiserror::Error;
use velvet_math::Vec2;

use crate::director::AnimDirector;
use crate::effect::{EffectKind, EffectParams};
use crate::tween::parse_ease;

/// Script errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AnimScriptError {
    /// Bad line.
    #[error("line {line}: {msg}")]
    Line {
        /// 1-based line.
        line: usize,
        /// Message.
        msg: String,
    },
}

/// One compiled instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimOp {
    /// Spawn target at x,y (optional).
    Spawn {
        /// Id.
        id: String,
        /// Position.
        pos: Vec2,
    },
    /// Play effect.
    Fx {
        /// Target.
        id: String,
        /// Kind.
        kind: EffectKind,
        /// Params.
        params: EffectParams,
    },
    /// Move.
    Move {
        /// Target.
        id: String,
        /// X.
        x: f32,
        /// Y.
        y: f32,
        /// Duration.
        duration: f32,
        /// Ease name.
        ease: String,
    },
    /// Stop target.
    Stop {
        /// Id.
        id: String,
    },
    /// Wait seconds (director does not auto-wait; runner tracks).
    Wait {
        /// Seconds.
        secs: f32,
    },
}

/// Parsed program.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimProgram {
    /// Ops in order.
    pub ops: Vec<AnimOp>,
}

/// Parse `.vanim` text.
pub fn parse_anim_script(source: &str) -> Result<AnimProgram, AnimScriptError> {
    let mut ops = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "spawn" => {
                // spawn id x y
                if parts.len() < 4 {
                    return Err(AnimScriptError::Line {
                        line: line_no,
                        msg: "spawn id x y".into(),
                    });
                }
                let x: f32 = parts[2].parse().map_err(|_| AnimScriptError::Line {
                    line: line_no,
                    msg: "bad x".into(),
                })?;
                let y: f32 = parts[3].parse().map_err(|_| AnimScriptError::Line {
                    line: line_no,
                    msg: "bad y".into(),
                })?;
                ops.push(AnimOp::Spawn {
                    id: parts[1].into(),
                    pos: Vec2::new(x, y),
                });
            }
            "fx" => {
                // fx id kind [x y] [duration] [delay N] [strength N] [ease NAME]
                if parts.len() < 3 {
                    return Err(AnimScriptError::Line {
                        line: line_no,
                        msg: "fx id kind …".into(),
                    });
                }
                let id = parts[1].to_string();
                let kind = EffectKind::parse(parts[2]).ok_or_else(|| AnimScriptError::Line {
                    line: line_no,
                    msg: format!("unknown effect `{}`", parts[2]),
                })?;
                let mut params = EffectParams {
                    duration: 0.35,
                    ..Default::default()
                };
                let mut idx = 3;
                // optional x y numbers
                if idx + 1 < parts.len()
                    && parts[idx].parse::<f32>().is_ok()
                    && parts[idx + 1].parse::<f32>().is_ok()
                {
                    let x: f32 = parts[idx].parse().unwrap();
                    let y: f32 = parts[idx + 1].parse().unwrap();
                    params.to = Vec2::new(x, y);
                    idx += 2;
                }
                if idx < parts.len() && parts[idx].parse::<f32>().is_ok() {
                    params.duration = parts[idx].parse().unwrap();
                    idx += 1;
                }
                while idx < parts.len() {
                    match parts[idx] {
                        "delay" if idx + 1 < parts.len() => {
                            params.delay = parts[idx + 1].parse().unwrap_or(0.0);
                            idx += 2;
                        }
                        "strength" if idx + 1 < parts.len() => {
                            params.strength = parts[idx + 1].parse().unwrap_or(8.0);
                            idx += 2;
                        }
                        "ease" if idx + 1 < parts.len() => {
                            params.ease = parse_ease(parts[idx + 1]);
                            idx += 2;
                        }
                        other => {
                            return Err(AnimScriptError::Line {
                                line: line_no,
                                msg: format!("unexpected `{other}`"),
                            });
                        }
                    }
                }
                ops.push(AnimOp::Fx { id, kind, params });
            }
            "move" => {
                // move id x y duration [ease name]
                if parts.len() < 5 {
                    return Err(AnimScriptError::Line {
                        line: line_no,
                        msg: "move id x y duration".into(),
                    });
                }
                let ease = if parts.len() >= 7 && parts[5] == "ease" {
                    parts[6].to_string()
                } else if parts.len() >= 6 {
                    parts[5].to_string()
                } else {
                    "cubic_out".into()
                };
                ops.push(AnimOp::Move {
                    id: parts[1].into(),
                    x: parts[2].parse().unwrap_or(0.0),
                    y: parts[3].parse().unwrap_or(0.0),
                    duration: parts[4].parse().unwrap_or(0.3),
                    ease,
                });
            }
            "stop" => {
                if parts.len() < 2 {
                    return Err(AnimScriptError::Line {
                        line: line_no,
                        msg: "stop id".into(),
                    });
                }
                ops.push(AnimOp::Stop {
                    id: parts[1].into(),
                });
            }
            "wait" => {
                if parts.len() < 2 {
                    return Err(AnimScriptError::Line {
                        line: line_no,
                        msg: "wait secs".into(),
                    });
                }
                ops.push(AnimOp::Wait {
                    secs: parts[1].parse().unwrap_or(0.0),
                });
            }
            other => {
                return Err(AnimScriptError::Line {
                    line: line_no,
                    msg: format!("unknown op `{other}`"),
                });
            }
        }
    }
    Ok(AnimProgram { ops })
}

/// Immediate ops (non-wait) applied to director; returns total wait requested.
pub fn apply_program_immediate(dir: &mut AnimDirector, program: &AnimProgram) -> f32 {
    let mut wait = 0.0f32;
    for op in &program.ops {
        match op {
            AnimOp::Spawn { id, pos } => {
                dir.spawn_at(id, *pos);
            }
            AnimOp::Fx { id, kind, params } => {
                dir.ensure(id);
                dir.play_effect(id, *kind, *params);
            }
            AnimOp::Move {
                id,
                x,
                y,
                duration,
                ease,
            } => {
                dir.move_to(id, *x, *y, *duration, ease);
            }
            AnimOp::Stop { id } => dir.stop(id),
            AnimOp::Wait { secs } => wait += *secs,
        }
    }
    wait
}

/// Runner that steps waits over time (for hosts).
#[derive(Debug, Clone, Default)]
pub struct AnimScriptRunner {
    /// Remaining ops.
    pub queue: Vec<AnimOp>,
    /// Wait timer.
    pub wait_left: f32,
}

impl AnimScriptRunner {
    /// From program.
    pub fn from_program(program: AnimProgram) -> Self {
        Self {
            queue: program.ops,
            wait_left: 0.0,
        }
    }

    /// Done?
    pub fn is_done(&self) -> bool {
        self.queue.is_empty() && self.wait_left <= 0.0
    }

    /// Tick runner + director.
    pub fn tick(&mut self, dir: &mut AnimDirector, dt: f32) {
        if self.wait_left > 0.0 {
            self.wait_left = (self.wait_left - dt).max(0.0);
            dir.tick(dt);
            return;
        }
        while self.wait_left <= 0.0 {
            let Some(op) = self.queue.first().cloned() else {
                break;
            };
            self.queue.remove(0);
            match op {
                AnimOp::Spawn { id, pos } => {
                    dir.spawn_at(id, pos);
                }
                AnimOp::Fx { id, kind, params } => {
                    dir.ensure(&id);
                    dir.play_effect(&id, kind, params);
                }
                AnimOp::Move {
                    id,
                    x,
                    y,
                    duration,
                    ease,
                } => dir.move_to(&id, x, y, duration, &ease),
                AnimOp::Stop { id } => dir.stop(&id),
                AnimOp::Wait { secs } => {
                    self.wait_left = secs;
                    break;
                }
            }
        }
        dir.tick(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_run_deal_script() {
        let src = r#"
        spawn card0 0 0
        fx card0 deal 200 300 0.3
        wait 0.1
        fx card0 punch 0.2 strength 0.2
        "#;
        let prog = parse_anim_script(src).expect("parse");
        assert!(prog.ops.len() >= 3);
        let mut dir = AnimDirector::new();
        let mut runner = AnimScriptRunner::from_program(prog);
        for _ in 0..40 {
            runner.tick(&mut dir, 1.0 / 60.0);
        }
        assert!(dir.pose("card0").is_some());
        assert!(dir.pose("card0").unwrap().opacity > 0.3);
    }

    #[test]
    fn bad_effect_errors() {
        let err = parse_anim_script("fx a nope").unwrap_err();
        assert!(matches!(err, AnimScriptError::Line { .. }));
    }
}
