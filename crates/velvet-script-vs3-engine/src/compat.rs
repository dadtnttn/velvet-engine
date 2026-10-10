//! Registrable compatibility module for the legacy presentation natives.
//!
//! The synchronous `present_show` / `set_bg` / `present_hide` / `ui_flag` /
//! `ui_flag_get` calls stay working in the VM, but new code should drive the
//! `velvet.compat` services through the generic host ABI instead (the
//! compiler warns on direct legacy calls; see `LEGACY_PRESENT_NOTICE`):
//!
//! ```ignore
//! // @edition 3
//! function reveal(id) {
//!     yield(["velvet.compat.present_show", [id, "happy", "left"]])
//!     return yield(["velvet.compat.ui_flag_get", ["seen_intro"]])
//! }
//! ```
//!
//! [`CompatHost`] owns a [`PresentHostState`] and applies the same coercions
//! as the legacy natives (stringified ids, empty optionals filtered,
//! bool-ish flag values), so migrated scripts observe identical state.

use velvet_script_vs3::{
    validate_request, FunctionDescriptor, HostOutcome, HostRequest, ModuleDescriptor,
    ParamDescriptor, PresentHostState, Purity, Value, Vs3AbiType, Vs3Host,
};

/// Capability required by the mutating compat services.
pub const COMPAT_CAPABILITY: &str = "present";

fn compat_function(
    name: &str,
    params: Vec<ParamDescriptor>,
    result: Vs3AbiType,
    doc: &str,
) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        service: format!("velvet.compat.{name}"),
        params,
        result: result.clone(),
        purity: Purity::Impure,
        capabilities: vec![COMPAT_CAPABILITY.into()],
        doc: doc.into(),
        ..FunctionDescriptor::new(result)
    }
}

fn str_param(name: &str) -> ParamDescriptor {
    ParamDescriptor::new(name, Vs3AbiType::Str)
}

/// Descriptor for the `velvet.compat` module.
///
/// Mirrors the legacy presentation surface as `yield` services. Arity rules
/// intentionally mirror the legacy natives (documented per function) instead
/// of pretending to be strict.
pub fn compat_module() -> ModuleDescriptor {
    ModuleDescriptor::new(
        "velvet.compat",
        "1.0.0",
        vec![
            compat_function(
                "present_show",
                vec![str_param("id"), str_param("expression"), str_param("at")],
                Vs3AbiType::Null,
                "show(id [, expression [, at]]): 1..=3 strings, like legacy present_show",
            ),
            compat_function(
                "present_hide",
                vec![str_param("id")],
                Vs3AbiType::Null,
                "hide(id): exactly 1 string, like legacy present_hide",
            ),
            compat_function(
                "set_bg",
                vec![str_param("path")],
                Vs3AbiType::Null,
                "set_bg(path): exactly 1 string, like legacy set_bg",
            ),
            compat_function(
                "ui_flag",
                vec![
                    str_param("name"),
                    ParamDescriptor::new("on", Vs3AbiType::Bool),
                ],
                Vs3AbiType::Null,
                "ui_flag(name, on): bool-ish second arg, like legacy ui_flag",
            ),
            FunctionDescriptor {
                name: "ui_flag_get".into(),
                service: "velvet.compat.ui_flag_get".into(),
                params: vec![str_param("name")],
                result: Vs3AbiType::Bool,
                purity: Purity::Pure,
                capabilities: vec![COMPAT_CAPABILITY.into()],
                doc: "ui_flag_get(name) -> bool, like legacy ui_flag_get".into(),
                ..FunctionDescriptor::new(Vs3AbiType::Bool)
            },
        ],
    )
    .expect("built-in descriptors are valid")
}

/// Reference host for [`compat_module`], owning presentation state.
///
/// Games using the migrated path drive this host (or their own engine twin)
/// instead of the VM thread-local present host.
#[derive(Debug, Default)]
pub struct CompatHost {
    state: PresentHostState,
}

impl CompatHost {
    /// Empty presentation state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current state (for paint / assertions).
    pub fn state(&self) -> &PresentHostState {
        &self.state
    }

    /// Mutable state (for test setup).
    pub fn state_mut(&mut self) -> &mut PresentHostState {
        &mut self.state
    }
}

/// Legacy `ui_flag` bool-ish coercion, mirrored exactly.
fn compat_bool(value: &Value) -> Result<bool, String> {
    match value {
        Value::Bool(b) => Ok(*b),
        Value::Int(i) => Ok(*i != 0),
        Value::Float(f) => Ok(*f != 0.0),
        Value::String(s) => {
            let t: &str = s.as_ref();
            Ok(t == "true" || t == "1" || t.eq_ignore_ascii_case("yes"))
        }
        Value::Null => Ok(false),
        other => Err(format!(
            "ui_flag expects a bool-ish second argument, got {other}"
        )),
    }
}

/// Payload args as a borrowed vec, or a failure message.
fn compat_args(request: &HostRequest) -> Result<std::vec::Vec<Value>, String> {
    match &request.payload {
        Value::List(items) => Ok(items.borrow().clone()),
        other => Err(format!("compat payload must be a list, got {other}")),
    }
}

impl Vs3Host for CompatHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        let module = compat_module();
        let Some(function) = module
            .functions
            .values()
            .find(|f| f.service == request.service)
        else {
            return HostOutcome::Failed(format!(
                "service `{}` is not part of velvet.compat",
                request.service
            ));
        };
        let args = match compat_args(request) {
            Ok(args) => args,
            Err(message) => return HostOutcome::Failed(message),
        };
        // Legacy looseness, mirrored on purpose (see module docs):
        // `present_show` takes 1..=3 args and `ui_flag` coerces any
        // bool-ish second arg instead of requiring strict Bool.
        let arity_ok = match function.name.as_str() {
            "present_show" => (1..=3).contains(&args.len()),
            "ui_flag" => args.len() == 2,
            _ => validate_request(function, request).is_ok(),
        };
        if !arity_ok {
            return HostOutcome::Failed(format!(
                "service `{}` called with {} argument(s)",
                request.service,
                args.len()
            ));
        }
        let text = |i: usize| args[i].to_string();
        let opt = |i: usize| args.get(i).map(|v| v.to_string()).filter(|s| !s.is_empty());
        match function.name.as_str() {
            "present_show" => {
                let (expression, at) = (opt(1), opt(2));
                self.state.show(text(0), expression, at);
                HostOutcome::Ready(Value::Null)
            }
            "present_hide" => {
                self.state.hide(&text(0));
                HostOutcome::Ready(Value::Null)
            }
            "set_bg" => {
                self.state.set_bg(text(0));
                HostOutcome::Ready(Value::Null)
            }
            "ui_flag" => match compat_bool(&args[1]) {
                Ok(on) => {
                    self.state.set_ui_flag(text(0), on);
                    HostOutcome::Ready(Value::Null)
                }
                Err(message) => HostOutcome::Failed(message),
            },
            "ui_flag_get" => HostOutcome::Ready(Value::Bool(self.state.ui_flag(&text(0)))),
            other => HostOutcome::Failed(format!("unknown compat function `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_vs3::Value;

    fn list(items: Vec<Value>) -> Value {
        Value::list(items)
    }

    fn str_(s: &str) -> Value {
        Value::String(s.into())
    }

    fn call(host: &mut CompatHost, service: &str, args: Vec<Value>) -> HostOutcome {
        let request = HostRequest::new(service, list(args)).unwrap();
        host.call(&request)
    }

    #[test]
    fn descriptor_registers_five_services() {
        let module = compat_module();
        assert_eq!(module.functions.len(), 5);
        for name in [
            "present_show",
            "present_hide",
            "set_bg",
            "ui_flag",
            "ui_flag_get",
        ] {
            let f = &module.functions[name];
            assert_eq!(f.service, format!("velvet.compat.{name}"));
            assert!(f.capabilities.contains(&COMPAT_CAPABILITY.to_string()));
        }
        assert!(matches!(
            module.functions["ui_flag_get"].purity,
            Purity::Pure
        ));
    }

    #[test]
    fn show_hide_bg_mirror_legacy_coercions() {
        let mut host = CompatHost::new();
        // 1..=3 args; numbers stringify like the legacy natives.
        assert!(matches!(
            call(&mut host, "velvet.compat.present_show", vec![str_("nora")]),
            HostOutcome::Ready(_)
        ));
        assert!(matches!(
            call(
                &mut host,
                "velvet.compat.present_show",
                vec![str_("june"), Value::Int(7), str_("")]
            ),
            HostOutcome::Ready(_)
        ));
        // Empty optionals are filtered, like legacy.
        let sprite = &host.state().sprites["june"];
        assert_eq!(sprite.expression.as_deref(), Some("7"));
        assert_eq!(sprite.at, None);
        call(&mut host, "velvet.compat.set_bg", vec![str_("bg:station")]);
        assert_eq!(host.state().background.as_deref(), Some("bg:station"));
        call(&mut host, "velvet.compat.present_hide", vec![str_("nora")]);
        assert!(!host.state().sprites.contains_key("nora"));
        // Arity outside 1..=3 fails.
        assert!(matches!(
            call(&mut host, "velvet.compat.present_show", vec![]),
            HostOutcome::Failed(_)
        ));
    }

    #[test]
    fn flags_mirror_bool_ish_coercion() {
        let mut host = CompatHost::new();
        for (value, expected) in [
            (Value::Bool(true), true),
            (Value::Int(1), true),
            (Value::Int(0), false),
            (Value::Float(0.5), true),
            (str_("yes"), true),
            (str_("no"), false),
            (Value::Null, false),
        ] {
            assert!(matches!(
                call(
                    &mut host,
                    "velvet.compat.ui_flag",
                    vec![str_("seen"), value]
                ),
                HostOutcome::Ready(_)
            ));
            assert!(matches!(
                call(
                    &mut host,
                    "velvet.compat.ui_flag_get",
                    vec![str_("seen")]
                ),
                HostOutcome::Ready(Value::Bool(got)) if got == expected
            ));
        }
        // Unknown services and bad payloads fail, never panic.
        assert!(matches!(
            call(&mut host, "velvet.compat.nope", vec![]),
            HostOutcome::Failed(_)
        ));
        let bad = HostRequest::new("velvet.compat.set_bg", Value::Int(1)).unwrap();
        assert!(matches!(host.call(&bad), HostOutcome::Failed(_)));
    }

    #[test]
    fn compat_host_conforms_to_host_contract() {
        use velvet_script_vs3::conformance;
        let mut host = CompatHost::new();
        conformance::expect_failed(&mut host, "velvet.compat.nope", vec![]);
        conformance::expect_failed(&mut host, "velvet.compat.set_bg", vec![]);
        conformance::expect_ready(
            &mut host,
            "velvet.compat.ui_flag_get",
            vec![str_("seen")],
            Value::Bool(false),
        );
    }

    #[test]
    fn matches_legacy_native_state() {
        // Same script ops through the legacy natives and through the
        // compat services must produce identical presentation state.
        let src = r#"
// @edition 3
function probe() {
    present_show("nora", "happy", "left")
    set_bg("bg:x")
    ui_flag("seen", 1)
    return ui_flag_get("seen")
}
"#;
        let module = velvet_script_vs3::compile(src, Some("probe.vel")).unwrap();
        let (value, legacy) = module.call_with_present("probe", &[]).unwrap();
        assert_eq!(value, Value::Bool(true));
        let mut host = CompatHost::new();
        call(
            &mut host,
            "velvet.compat.present_show",
            vec![str_("nora"), str_("happy"), str_("left")],
        );
        call(&mut host, "velvet.compat.set_bg", vec![str_("bg:x")]);
        call(
            &mut host,
            "velvet.compat.ui_flag",
            vec![str_("seen"), Value::Int(1)],
        );
        assert_eq!(host.state(), &legacy);
    }
}
