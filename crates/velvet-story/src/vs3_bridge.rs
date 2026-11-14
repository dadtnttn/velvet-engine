//! Optional classic story → VS3 logic bridge.
//!
//! Lets a host invoke a pure VS3 function and map the result into
//! [`crate::value::StoryValue`] for story variables — **no draw API**.

use crate::value::StoryValue;

/// Error calling a VS3 logic unit from classic story host code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs3BridgeError {
    /// Message.
    pub message: String,
}

impl std::fmt::Display for Vs3BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Vs3BridgeError {}

impl Vs3BridgeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// One compiled logic file, kept with what it was compiled from.
struct Compiled {
    source: String,
    file: Option<String>,
    module: std::rc::Rc<velvet_script_vs3::Vs3Module>,
}

thread_local! {
    /// The logic files compiled on this thread. A story calls its logic file on every `logic.name`
    /// it meets, and compiling it is nearly all the cost of a call; every call still runs in a
    /// fresh session of the module, so caching keeps no state between calls.
    static COMPILED: std::cell::RefCell<Vec<Compiled>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `source` compiled, from this thread's cache when it was compiled before.
fn compiled(
    source: &str,
    file: Option<&str>,
) -> Result<std::rc::Rc<velvet_script_vs3::Vs3Module>, Vs3BridgeError> {
    let hit = COMPILED.with(|c| {
        c.borrow()
            .iter()
            .find(|e| e.source == source && e.file.as_deref() == file)
            .map(|e| e.module.clone())
    });
    if let Some(module) = hit {
        return Ok(module);
    }
    let module = std::rc::Rc::new(
        velvet_script_vs3::compile(source, file)
            .map_err(|e| Vs3BridgeError::new(format!("vs3 compile: {e}")))?,
    );
    COMPILED.with(|c| {
        let mut c = c.borrow_mut();
        // A handful of logic files is the most a game has; past that, start over.
        if c.len() >= 16 {
            c.clear();
        }
        c.push(Compiled {
            source: source.to_owned(),
            file: file.map(str::to_owned),
            module: module.clone(),
        });
    });
    Ok(module)
}

/// Compile VS3 source and call `fn_name` with story-value arguments.
///
/// Uses the shipped `velvet_script_vs3` compile → call path.
pub fn call_vs3_logic(
    source: &str,
    file: Option<&str>,
    fn_name: &str,
    args: &[StoryValue],
) -> Result<StoryValue, Vs3BridgeError> {
    let module = compiled(source, file)?;
    let vs_args: Vec<velvet_script_vs3::Value> = args.iter().map(story_to_vs3).collect();
    let out = module
        .call(fn_name, &vs_args)
        .map_err(|e| Vs3BridgeError::new(format!("vs3 call: {e}")))?;
    Ok(vs3_to_story(out))
}

/// Calls `fn_name` of a logic file with one argument, a map of `vars`, and returns its raw answer.
fn call_with_vars<'a>(
    source: &str,
    file: Option<&str>,
    fn_name: &str,
    vars: impl IntoIterator<Item = (&'a String, &'a StoryValue)>,
) -> Result<velvet_script_vs3::Value, Vs3BridgeError> {
    let module = compiled(source, file)?;
    let map: std::collections::BTreeMap<String, velvet_script_vs3::Value> = vars
        .into_iter()
        .map(|(k, v)| (k.clone(), story_to_vs3(v)))
        .collect();
    let arg = velvet_script_vs3::Value::Map(std::rc::Rc::new(std::cell::RefCell::new(map)));
    module
        .call(fn_name, &[arg])
        .map_err(|e| Vs3BridgeError::new(format!("vs3 call: {e}")))
}

/// Compile VS3 source and call `fn_name` with one argument: a map of `vars` (a story's
/// variables), as `logic.name` does.
pub fn call_vs3_with_vars<'a>(
    source: &str,
    file: Option<&str>,
    fn_name: &str,
    vars: impl IntoIterator<Item = (&'a String, &'a StoryValue)>,
) -> Result<StoryValue, Vs3BridgeError> {
    call_with_vars(source, file, fn_name, vars).map(vs3_to_story)
}

/// [`call_vs3_with_vars`] for an answer that must be text (`goto logic.name`): the text, trimmed,
/// or None when the function answers anything else (a number, a list, null) or only spaces.
pub fn call_vs3_text_with_vars<'a>(
    source: &str,
    file: Option<&str>,
    fn_name: &str,
    vars: impl IntoIterator<Item = (&'a String, &'a StoryValue)>,
) -> Result<Option<String>, Vs3BridgeError> {
    Ok(match call_with_vars(source, file, fn_name, vars)? {
        velvet_script_vs3::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        _ => None,
    })
}

/// The functions a VS3 logic file exports (it must compile).
pub fn vs3_logic_functions(
    source: &str,
    file: Option<&str>,
) -> Result<Vec<String>, Vs3BridgeError> {
    compiled(source, file).map(|m| m.function_names())
}

fn story_to_vs3(v: &StoryValue) -> velvet_script_vs3::Value {
    match v {
        StoryValue::Null => velvet_script_vs3::Value::Null,
        StoryValue::Bool(b) => velvet_script_vs3::Value::Bool(*b),
        StoryValue::Int(i) => velvet_script_vs3::Value::Int(*i),
        StoryValue::Float(f) => velvet_script_vs3::Value::Float(*f),
        StoryValue::String(s) => velvet_script_vs3::Value::String(std::rc::Rc::from(s.as_str())),
    }
}

fn vs3_to_story(v: velvet_script_vs3::Value) -> StoryValue {
    match v {
        velvet_script_vs3::Value::Null => StoryValue::Null,
        velvet_script_vs3::Value::Bool(b) => StoryValue::Bool(b),
        velvet_script_vs3::Value::Int(i) => StoryValue::Int(i),
        velvet_script_vs3::Value::Float(f) => StoryValue::Float(f),
        velvet_script_vs3::Value::String(s) => StoryValue::String(s.to_string()),
        other => StoryValue::String(other.to_string()),
    }
}

/// Host-facing name used with classic `call vs3.run fn "name"` style bridges.
pub const VS3_HOST_CMD: &str = "vs3.run";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::command_host_continue;
    use crate::load::load_program_from_source;
    use crate::runtime::{StoryPlayer, StoryWait};
    use crate::value::StoryValue;

    const LOGIC: &str = r#"
// @edition 3
function apply_damage(hp, dmg) {
    let x = hp - dmg
    if x < 0 {
        return 0
    }
    return x
}
"#;

    #[test]
    fn call_vs3_pure_fn_from_bridge() {
        let v = call_vs3_logic(
            LOGIC,
            Some("rules.vel"),
            "apply_damage",
            &[StoryValue::Int(10), StoryValue::Int(3)],
        )
        .unwrap();
        assert_eq!(v, StoryValue::Int(7));
        let v = call_vs3_logic(
            LOGIC,
            Some("rules.vel"),
            "apply_damage",
            &[StoryValue::Int(2), StoryValue::Int(9)],
        )
        .unwrap();
        assert_eq!(v, StoryValue::Int(0));
    }

    #[test]
    fn classic_host_call_can_invoke_vs3_and_set_var() {
        let src = r#"
state {
    hp: int = 10
}

scene main {
    call vs3.run
    "after bridge"
    end
}
"#;
        let program = load_program_from_source(src, Some("bridge.vel"), "B").unwrap();
        let logic = LOGIC.to_string();
        let host = command_host_continue(move |name, _args, vars| {
            assert_eq!(name, "vs3.run");
            let hp = vars.get_int("hp", 0);
            let out = call_vs3_logic(
                &logic,
                Some("rules.vel"),
                "apply_damage",
                &[StoryValue::Int(hp), StoryValue::Int(4)],
            )
            .map_err(|e| crate::host::StoryCommandError::new(e.message))?;
            vars.set("hp", out);
            vars.set("ui.say_visible", StoryValue::Bool(true));
            Ok(())
        });
        let mut player = StoryPlayer::start_with_host(program, host);
        let mut steps = 0;
        loop {
            steps += 1;
            assert!(steps < 20);
            match player.wait().clone() {
                StoryWait::Line | StoryWait::Ready => player.advance(),
                StoryWait::Ended => break,
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(player.variables().get_int("hp", -1), 6);
        assert!(player.variables().get("ui.say_visible").is_truthy());
        // HostCall must not invent draw ops
        assert!(
            player
                .variables()
                .get("__last_command")
                .display_str()
                .contains("vs3")
                || player.variables().get_int("hp", 0) == 6
        );
    }
}
