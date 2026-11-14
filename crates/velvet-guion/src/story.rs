//! Guiones compiled through Velvet Story: a [`Library`] as a `velvet_story::StoryProgram`, the
//! canonical story IR that `StoryPlayer` plays (the crate's `story` feature).
//!
//! Every scene of every guion becomes a story scene named by its id (`<file id>.<label>`):
//!
//! - a line is dialogue (narration, sounds and the narrator have no speaker);
//! - a choice is a story choice, and its question (`? …`) a narrator line just before it, as in a
//!   visual-novel export; an option's conditions are worked out just before the choice into a
//!   hidden variable (`__opt_<scene>_<n>`), which the option requires; its effects run in its
//!   body, then it jumps to its target (or goes on with the next line);
//! - `@si` is an `if` around a jump, `@salta` a jump, `@var` assignments, `@vn` a jump, `@fin` the
//!   end, and a scene that runs on into the next jumps there;
//! - `@decide name -> a, b` asks the logic file (`logic.name`, Velvet Script 3) and jumps to the
//!   scene of the label it answers, read as a slug on both sides as a conversation reads it;
//! - `@fondo` and `@musica` are the story's background and music; any other staging is a host
//!   call `stage.<directive>` with the argument as `arg`.
//!
//! The guiones' variables start at 0, as in a [`crate::Conversation`]. All the guiones of one
//! program share one logic file: a library whose guiones have different ones is refused.

use indexmap::IndexMap;
use velvet_story::{
    AssignOp, Character, StoryChoice, StoryCmpOp, StoryCond, StoryExpr, StoryLogic, StoryOp,
    StoryOperand, StoryProgram, StoryScene, StoryValue,
};

use crate::{guion_vars, slug, Cond, EffOp, Effect, Library, LineKind, Op, Step};

/// The hidden variable a `@decide` answer is kept in before its jumps.
const DECIDED: &str = "__decide";

fn cmp(op: Op) -> StoryCmpOp {
    match op {
        Op::Eq => StoryCmpOp::Eq,
        Op::Ne => StoryCmpOp::Ne,
        Op::Gt => StoryCmpOp::Gt,
        Op::Gte => StoryCmpOp::Ge,
        Op::Lt => StoryCmpOp::Lt,
        Op::Lte => StoryCmpOp::Le,
    }
}

/// All of `conds` (true when there are none).
fn all(conds: &[Cond]) -> StoryCond {
    conds
        .iter()
        .map(|c| StoryCond::Cmp {
            left: StoryOperand::var(c.var.clone()),
            op: cmp(c.op),
            right: StoryOperand::value(StoryValue::Int(i64::from(c.value))),
        })
        .reduce(|left, right| StoryCond::And {
            left: Box::new(left),
            right: Box::new(right),
        })
        .unwrap_or(StoryCond::Const { value: true })
}

fn assign(e: &Effect) -> StoryOp {
    StoryOp::Assign {
        name: e.var.clone(),
        assign_op: match e.op {
            EffOp::Set => AssignOp::Set,
            EffOp::Add => AssignOp::Add,
            EffOp::Sub => AssignOp::Sub,
        },
        value: StoryExpr::value(StoryValue::Int(i64::from(e.value))),
    }
}

/// Where `to` (a label of file `f`, a `file.label`, or `fin`) leads: a jump, or the end.
fn go(lib: &Library, f: usize, to: &str) -> StoryOp {
    if slug(to) == "fin" {
        return StoryOp::End { ending: None };
    }
    match lib.find(to, Some(f)) {
        Some((tf, ts)) => StoryOp::Jump {
            target: lib.guiones[tf].scenes[ts].id.clone(),
        },
        None => StoryOp::End {
            ending: Some(format!("missing:{to}")),
        },
    }
}

/// The guiones of `lib` as one story program that starts at scene `entry` (`file.label`).
pub fn compile_story(lib: &Library, entry: &str) -> Result<StoryProgram, String> {
    let (ef, es) = lib
        .find(entry, None)
        .ok_or_else(|| format!("no scene «{entry}» in the guiones"))?;
    let mut program = StoryProgram::new(lib.guiones[ef].title.clone());
    program.entry = lib.guiones[ef].scenes[es].id.clone();
    let logics: Vec<_> = lib.logic.values().collect();
    match logics.as_slice() {
        [] => {}
        [one] => {
            program.logic = Some(StoryLogic {
                path: one.path.display().to_string(),
                source: one.source.clone(),
            })
        }
        many => {
            return Err(format!(
                "the guiones have {} logic files; one program takes one",
                many.len()
            ))
        }
    }
    for (f, g) in lib.guiones.iter().enumerate() {
        for var in guion_vars(g) {
            program.initial_vars.insert(var, StoryValue::Int(0));
        }
        for (s, scene) in g.scenes.iter().enumerate() {
            let mut ops = Vec::new();
            let mut options_seen = 0usize;
            for step in &scene.steps {
                match step {
                    Step::Line {
                        who,
                        name,
                        text,
                        kind,
                        ..
                    } => {
                        let speaker = match kind {
                            LineKind::Narration | LineKind::Sound => None,
                            _ if who == "narrator" || who.is_empty() => None,
                            _ => Some(who.clone()),
                        };
                        if let Some(id) = &speaker {
                            program
                                .characters
                                .entry(id.clone())
                                .or_insert_with(|| Character::new(id.clone(), name.clone()));
                        }
                        ops.push(StoryOp::Dialogue {
                            speaker,
                            text: text.clone(),
                        });
                    }
                    Step::Choice {
                        prompt, options, ..
                    } => {
                        if let Some(p) = prompt.as_ref().filter(|p| !p.trim().is_empty()) {
                            ops.push(StoryOp::Dialogue {
                                speaker: None,
                                text: p.clone(),
                            });
                        }
                        let mut arms = Vec::new();
                        for o in options {
                            let require = (!o.cond.is_empty()).then(|| {
                                options_seen += 1;
                                let var =
                                    format!("__opt_{}_{options_seen}", scene.id.replace('.', "_"));
                                let flag = |v: i64| StoryOp::Assign {
                                    name: var.clone(),
                                    assign_op: AssignOp::Set,
                                    value: StoryExpr::value(StoryValue::Int(v)),
                                };
                                ops.push(StoryOp::If {
                                    cond: all(&o.cond),
                                    then_ops: vec![flag(1)],
                                    else_ops: vec![flag(0)],
                                });
                                var
                            });
                            let mut body: Vec<StoryOp> = o.effects.iter().map(assign).collect();
                            if let Some(to) = &o.to {
                                body.push(go(lib, f, to));
                            }
                            arms.push(StoryChoice {
                                text: o.label.clone(),
                                body,
                                require,
                                hidden_if_locked: false,
                            });
                        }
                        ops.push(StoryOp::Choice { options: arms });
                    }
                    Step::Jump { to, cond, .. } if cond.is_empty() => ops.push(go(lib, f, to)),
                    Step::Jump { to, cond, .. } => ops.push(StoryOp::If {
                        cond: all(cond),
                        then_ops: vec![go(lib, f, to)],
                        else_ops: Vec::new(),
                    }),
                    Step::Set { effects } => ops.extend(effects.iter().map(assign)),
                    Step::Decide { name, to, .. } => {
                        // the answer is kept as a label (slug), and compared with each label's
                        ops.push(StoryOp::Assign {
                            name: DECIDED.to_owned(),
                            assign_op: AssignOp::Set,
                            value: StoryExpr::Logic {
                                function: name.clone(),
                                as_label: true,
                            },
                        });
                        for t in to {
                            ops.push(StoryOp::If {
                                cond: StoryCond::Cmp {
                                    left: StoryOperand::var(DECIDED),
                                    op: StoryCmpOp::Eq,
                                    right: StoryOperand::value(StoryValue::String(slug(t))),
                                },
                                then_ops: vec![go(lib, f, t)],
                                else_ops: Vec::new(),
                            });
                        }
                    }
                    Step::End => ops.push(StoryOp::End { ending: None }),
                    Step::Vn { to, .. } => ops.push(go(lib, f, to)),
                    Step::Stage { cmd, arg } => ops.push(match cmd.as_str() {
                        "fondo" => StoryOp::Background { path: arg.clone() },
                        "musica" => StoryOp::Music {
                            path: arg.clone(),
                            fade_in: None,
                        },
                        _ => {
                            let mut args = IndexMap::new();
                            args.insert("arg".to_owned(), StoryValue::String(arg.clone()));
                            StoryOp::HostCall {
                                name: format!("stage.{cmd}"),
                                args,
                            }
                        }
                    }),
                }
            }
            // Off the end of a scene: on into the next one if it follows on, else the end.
            match g.scenes.get(s + 1) {
                Some(next) if next.follows_on() => ops.push(StoryOp::Jump {
                    target: next.id.clone(),
                }),
                _ => ops.push(StoryOp::End { ending: None }),
            }
            let mut story_scene = StoryScene {
                name: scene.id.clone(),
                ops,
                labels: IndexMap::new(),
            };
            story_scene.reindex_labels();
            program.scenes.insert(scene.id.clone(), story_scene);
        }
    }
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cast;
    use std::path::Path;
    use velvet_story::{StoryPlayer, StoryWait};

    /// The lines a program shows, taking option `pick` (of the enabled ones) at every choice.
    fn play(program: StoryProgram, pick: usize) -> Vec<String> {
        let mut p = StoryPlayer::start(program);
        let mut out = Vec::new();
        for _ in 0..500 {
            match p.wait().clone() {
                StoryWait::Ended => break,
                StoryWait::Line => {
                    out.push(p.current_text().to_string());
                    p.advance();
                }
                StoryWait::Choice => {
                    let open: Vec<_> = p.choices().iter().filter(|c| c.enabled).cloned().collect();
                    let _ = p.choose(open[pick.min(open.len() - 1)].index);
                }
                StoryWait::Host { token } => {
                    let _ = p.resume_host(&token);
                }
                _ => p.advance(),
            }
        }
        out
    }

    #[test]
    fn a_guion_plays_the_same_in_story_player() {
        let mut lib = Library::new(Cast::new());
        lib.add(
            "---\nid: ana\n---\n## inicio\n@fondo posada\n? ¿Qué?\n- Té -> sigue {confianza +2}\n- Nada -> sigue\n- Secreto [confianza >= 5] -> fin\n## sigue\n@si confianza >= 2 -> amiga\n**Ana:** —Bueno.\n@decide trato -> recelo\n**Ana:** —Adiós.\n@fin\n## amiga\n**Ana:** —Gracias.\n## despedida\n*La puerta se cierra.*\n@fin\n## recelo\n**Ana:** —Hum.\n@fin\n",
            Path::new("ana.md"),
        );
        lib.add_logic(
            "ana",
            "// @edition 3\nfunction trato(v: map) {\n    if v[\"confianza\"] == 0 { return \"recelo\" }\n    return \"\"\n}\n",
            Path::new("ana.vel"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let program = compile_story(&lib, "ana.inicio").expect("program");
        assert_eq!(
            play(program.clone(), 0),
            vec!["¿Qué?", "Gracias.", "La puerta se cierra."],
            "the question, then tea: the @si jump, then on into the next scene"
        );
        assert_eq!(
            play(program, 1),
            vec!["¿Qué?", "Bueno.", "Hum."],
            "nothing: the logic file sends it to «recelo»"
        );
    }

    #[test]
    fn decide_answers_are_read_as_labels_as_in_a_conversation() {
        let mut lib = Library::new(Cast::new());
        lib.add(
            "---\nid: ana\n---\n## inicio\n@decide trato -> Recelo, otro.despedida\n**Ana:** —Bueno.\n@fin\n## recelo\n**Ana:** —Hum.\n@fin\n",
            Path::new("ana.md"),
        );
        lib.add(
            "---\nid: otro\n---\n## despedida\n**Otro:** —Adiós.\n@fin\n## charla\n@var confianza +1\n@fin\n",
            Path::new("otro.md"),
        );
        lib.add_logic(
            "ana",
            "// @edition 3\nfunction trato(v: map) {\n    if v[\"confianza\"] >= 1 { return \"Otro.Despedida\" }\n    return \" RECELO \"\n}\n",
            Path::new("ana.vel"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let program = compile_story(&lib, "ana.inicio").expect("program");
        assert_eq!(
            play(program.clone(), 0),
            vec!["Hum."],
            "« RECELO » is «Recelo»"
        );
        let mut trusted = program;
        trusted
            .initial_vars
            .insert("confianza".to_owned(), StoryValue::Int(1));
        assert_eq!(
            play(trusted, 0),
            vec!["Adiós."],
            "«Otro.Despedida» is «otro.despedida», in the other guion"
        );
    }

    #[test]
    fn the_story_label_slug_is_the_guion_slug() {
        for s in [
            "Otro.Despedida",
            " RECELO ",
            "Ánimo, ñandú",
            "3 cosas",
            "a--b__c",
            "",
            "  ",
            "Él dijo: «sí»",
            "ÜBER alles",
        ] {
            assert_eq!(velvet_story::label_slug(s), slug(s), "{s:?}");
        }
    }
}
