//! Injectable host for external story commands (`call combat.start: …`).

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::value::StoryValue;
use crate::variables::StoryVariables;

/// Error returned by a command host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryCommandError {
    /// Human message.
    pub message: String,
}

impl StoryCommandError {
    /// Construct from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StoryCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StoryCommandError {}

/// Result of a host command dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandOutcome {
    /// Continue narrative immediately after the call.
    Continue,
    /// Suspend narrative until [`crate::runtime::StoryPlayer::resume_host`].
    Wait {
        /// Opaque token returned to the host / UI.
        token: String,
    },
    /// Jump to a scene (or `scene:label`).
    Jump {
        /// Target.
        target: String,
    },
    /// End the story.
    End {
        /// Optional ending id.
        ending: Option<String>,
    },
}

/// Host that executes registered story commands against the game world.
///
/// Implementors run combat, inventory, etc. Return [`CommandOutcome::Wait`] to
/// pause the story across frames (e.g. until combat ends).
pub trait StoryCommandHost: Send + Sync {
    /// Invoke `name` with `args`. May update `vars` for story-visible side effects.
    fn call(
        &self,
        name: &str,
        args: &IndexMap<String, StoryValue>,
        vars: &mut StoryVariables,
    ) -> Result<CommandOutcome, StoryCommandError>;
}

/// Shared handle for attaching a host to a [`crate::runtime::StoryPlayer`].
pub type SharedCommandHost = Arc<dyn StoryCommandHost>;

/// Build a host from a closure (tests / simple games).
pub fn command_host_fn<F>(f: F) -> SharedCommandHost
where
    F: Fn(
            &str,
            &IndexMap<String, StoryValue>,
            &mut StoryVariables,
        ) -> Result<CommandOutcome, StoryCommandError>
        + Send
        + Sync
        + 'static,
{
    struct FnHost<F>(F);
    impl<F> StoryCommandHost for FnHost<F>
    where
        F: Fn(
                &str,
                &IndexMap<String, StoryValue>,
                &mut StoryVariables,
            ) -> Result<CommandOutcome, StoryCommandError>
            + Send
            + Sync,
    {
        fn call(
            &self,
            name: &str,
            args: &IndexMap<String, StoryValue>,
            vars: &mut StoryVariables,
        ) -> Result<CommandOutcome, StoryCommandError> {
            (self.0)(name, args, vars)
        }
    }
    Arc::new(FnHost(f))
}

/// Convenience: host that always continues after running `f` side effects.
pub fn command_host_continue<F>(f: F) -> SharedCommandHost
where
    F: Fn(&str, &IndexMap<String, StoryValue>, &mut StoryVariables) -> Result<(), StoryCommandError>
        + Send
        + Sync
        + 'static,
{
    command_host_fn(move |n, a, v| {
        f(n, a, v)?;
        Ok(CommandOutcome::Continue)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::StoryValue;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn host_fn_invoked() {
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let host = command_host_continue(move |name, args, vars| {
            assert_eq!(name, "combat.start");
            assert!(args.contains_key("enemy"));
            h.fetch_add(1, Ordering::SeqCst);
            vars.set("host_ok", StoryValue::Int(1));
            Ok(())
        });
        let mut vars = StoryVariables::new();
        let mut args = IndexMap::new();
        args.insert("enemy".into(), StoryValue::String("x".into()));
        let out = host.call("combat.start", &args, &mut vars).unwrap();
        assert_eq!(out, CommandOutcome::Continue);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(vars.get_int("host_ok", 0), 1);
    }
}
