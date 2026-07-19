//! Story command host bridging narrative `call` ops to action systems.
//!
//! Lives in `velvet-action` so `velvet-story` stays free of combat dependencies.
//! Games register this host with [`StoryPlayer::set_command_host`].

use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};

/// Shared combat-session flag set by [`CombatStoryHost`] for tests and UI.
#[derive(Debug, Default)]
pub struct CombatHostState {
    /// Last enemy id requested.
    pub last_enemy: Option<String>,
    /// Last difficulty.
    pub last_difficulty: i64,
    /// How many times combat was started.
    pub starts: u32,
    /// Whether a fight is waiting for resume.
    pub waiting: bool,
}

/// Injectable host that handles `combat.start` with suspend/resume semantics.
pub struct CombatStoryHost {
    /// Observable state (mutex for Arc sharing with tests).
    pub state: Mutex<CombatHostState>,
}

impl CombatStoryHost {
    /// New host with empty state.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(CombatHostState::default()),
        }
    }

    /// Stable wait token for the current fight.
    pub const WAIT_TOKEN: &'static str = "combat.active";
}

impl Default for CombatStoryHost {
    fn default() -> Self {
        Self::new()
    }
}

impl StoryCommandHost for CombatStoryHost {
    fn call(
        &self,
        name: &str,
        args: &IndexMap<String, StoryValue>,
        vars: &mut StoryVariables,
    ) -> Result<CommandOutcome, StoryCommandError> {
        match name {
            "combat.start" => {
                let enemy = args
                    .get("enemy")
                    .map(|v| v.display_str())
                    .unwrap_or_else(|| "unknown".into());
                let difficulty = args
                    .get("difficulty")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                if let Ok(mut st) = self.state.lock() {
                    st.last_enemy = Some(enemy.clone());
                    st.last_difficulty = difficulty;
                    st.starts = st.starts.saturating_add(1);
                    st.waiting = true;
                }
                vars.set("combat.enemy", StoryValue::String(enemy));
                vars.set("combat.difficulty", StoryValue::Int(difficulty));
                vars.set("combat.active", StoryValue::Bool(true));
                Ok(CommandOutcome::Wait {
                    token: Self::WAIT_TOKEN.into(),
                })
            }
            _ => Ok(CommandOutcome::Continue),
        }
    }
}

/// After the host/game finishes combat, clear waiting flag and set result vars.
pub fn finish_combat(vars: &mut StoryVariables, victory: bool) {
    vars.set("combat.active", StoryValue::Bool(false));
    vars.set(
        "combat.result",
        StoryValue::String(if victory { "victory" } else { "defeat" }.into()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use velvet_story::{
        AssignOp, StoryExpr, StoryOp, StoryPlayer, StoryProgram, StoryScene, StoryWait,
    };

    #[test]
    fn combat_start_suspends_and_resume_continues() {
        let host = Arc::new(CombatStoryHost::new());
        let host_trait: velvet_story::SharedCommandHost = host.clone();

        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::HostCall {
                        name: "combat.start".into(),
                        args: {
                            let mut m = IndexMap::new();
                            m.insert("enemy".into(), StoryValue::String("forest_guardian".into()));
                            m.insert("difficulty".into(), StoryValue::Int(3));
                            m
                        },
                    },
                    StoryOp::Assign {
                        name: "after_fight".into(),
                        assign_op: AssignOp::Set,
                        value: StoryExpr::value(StoryValue::Int(1)),
                    },
                    StoryOp::End {
                        ending: Some("won".into()),
                    },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("combat_wire");
        prog.entry = "start".into();
        prog.scenes = scenes;

        let mut player = StoryPlayer::start_with_host(prog, host_trait);
        assert!(
            matches!(player.wait(), StoryWait::Host { token } if token == CombatStoryHost::WAIT_TOKEN),
            "wait={:?}",
            player.wait()
        );
        assert_eq!(player.variables().get_int("after_fight", 0), 0);
        assert_eq!(
            player.variables().get("combat.enemy").display_str(),
            "forest_guardian"
        );
        {
            let st = host.state.lock().unwrap();
            assert_eq!(st.starts, 1);
            assert!(st.waiting);
            assert_eq!(st.last_difficulty, 3);
        }

        finish_combat(player.variables_mut(), true);
        player
            .resume_host(CombatStoryHost::WAIT_TOKEN)
            .expect("resume");
        assert_eq!(player.variables().get_int("after_fight", 0), 1);
        assert_eq!(
            player.variables().get("combat.result").display_str(),
            "victory"
        );
        assert_eq!(player.ending(), Some("won"));
    }
}
