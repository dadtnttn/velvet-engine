//! Ready-made VS3 host modules for time and input.
//!
//! Each module pairs a typed [`ModuleDescriptor`] with a reference host that
//! validates every request against its own descriptor before answering. Games
//! feed fresh values once per frame (`update` / `set_*`) and scripts read them
//! through the generic host ABI:
//!
//! ```ignore
//! // @edition 3
//! function frame_clock() {
//!     let ms = yield(["velvet.time.now_ms", []])
//!     return ms
//! }
//! ```
//!
//! The hosts are intentionally dumb mirrors: they own no clocks or devices,
//! only the latest snapshot the real engine pushed into them.

use std::collections::BTreeMap;

use velvet_script_vs3::{
    validate_request, FunctionDescriptor, HostOutcome, HostRequest, ModuleDescriptor,
    ParamDescriptor, Value, Vs3AbiType, Vs3Host,
};

fn zero_arg_function(name: &str, service: &str, result: Vs3AbiType) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        service: service.into(),
        params: Vec::new(),
        result: result.clone(),
        ..FunctionDescriptor::new(result)
    }
}

fn one_arg_function(
    name: &str,
    service: &str,
    param: &str,
    ty: Vs3AbiType,
    result: Vs3AbiType,
) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        service: service.into(),
        params: vec![ParamDescriptor::new(param, ty)],
        result: result.clone(),
        ..FunctionDescriptor::new(result)
    }
}

/// Descriptor for the `velvet.time` module.
///
/// Functions: `now_ms() -> int`, `delta_secs() -> float`.
pub fn time_module() -> ModuleDescriptor {
    ModuleDescriptor::new(
        "velvet.time",
        "1.0.0",
        vec![
            zero_arg_function("now_ms", "velvet.time.now_ms", Vs3AbiType::Int),
            zero_arg_function("delta_secs", "velvet.time.delta_secs", Vs3AbiType::Float),
        ],
    )
    .expect("built-in descriptors are valid")
}

/// Reference host for [`time_module`].
///
/// The owning loop calls [`Self::update`] once per frame; script reads see the
/// last pushed snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TimeModuleHost {
    now_ms: i64,
    delta_secs: f64,
}

impl TimeModuleHost {
    /// Host reporting epoch 0 and a zero delta until updated.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push the frame snapshot.
    pub fn update(&mut self, now_ms: i64, delta_secs: f64) {
        self.now_ms = now_ms;
        self.delta_secs = delta_secs;
    }

    /// Current snapshot.
    pub fn snapshot(&self) -> (i64, f64) {
        (self.now_ms, self.delta_secs)
    }
}

impl Vs3Host for TimeModuleHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        let module = time_module();
        let Some(function) = module
            .functions
            .values()
            .find(|f| f.service == request.service)
        else {
            return HostOutcome::Failed(format!(
                "service `{}` is not part of velvet.time",
                request.service
            ));
        };
        if let Err(message) = validate_request(function, request) {
            return HostOutcome::Failed(message);
        }
        match function.name.as_str() {
            "now_ms" => HostOutcome::Ready(Value::Int(self.now_ms)),
            "delta_secs" => HostOutcome::Ready(Value::Float(self.delta_secs)),
            other => HostOutcome::Failed(format!("unknown time function `{other}`")),
        }
    }
}

/// Descriptor for the `velvet.input` module.
///
/// Functions: `pressed(action: string) -> bool`, `axis(action: string) -> float`.
pub fn input_module() -> ModuleDescriptor {
    ModuleDescriptor::new(
        "velvet.input",
        "1.0.0",
        vec![
            one_arg_function(
                "pressed",
                "velvet.input.pressed",
                "action",
                Vs3AbiType::Str,
                Vs3AbiType::Bool,
            ),
            one_arg_function(
                "axis",
                "velvet.input.axis",
                "action",
                Vs3AbiType::Str,
                Vs3AbiType::Float,
            ),
        ],
    )
    .expect("built-in descriptors are valid")
}

/// Reference host for [`input_module`].
///
/// Actions without an explicit value report `false` / `0.0`, so scripts can
/// poll safely before any device data arrives.
#[derive(Debug, Clone, Default)]
pub struct InputModuleHost {
    pressed: BTreeMap<String, bool>,
    axes: BTreeMap<String, f64>,
}

impl InputModuleHost {
    /// Empty input mirror.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the pressed state of one action.
    pub fn set_pressed(&mut self, action: impl Into<String>, on: bool) {
        self.pressed.insert(action.into(), on);
    }

    /// Set the analog value of one action.
    pub fn set_axis(&mut self, action: impl Into<String>, value: f64) {
        self.axes.insert(action.into(), value);
    }
}

impl Vs3Host for InputModuleHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        let module = input_module();
        let Some(function) = module
            .functions
            .values()
            .find(|f| f.service == request.service)
        else {
            return HostOutcome::Failed(format!(
                "service `{}` is not part of velvet.input",
                request.service
            ));
        };
        if let Err(message) = validate_request(function, request) {
            return HostOutcome::Failed(message);
        }
        let Value::List(items) = &request.payload else {
            return HostOutcome::Failed("payload must be an argument list".into());
        };
        let action = match items.borrow().first() {
            Some(Value::String(action)) => action.to_string(),
            _ => return HostOutcome::Failed("missing action name".into()),
        };
        match function.name.as_str() {
            "pressed" => HostOutcome::Ready(Value::Bool(
                self.pressed.get(&action).copied().unwrap_or(false),
            )),
            "axis" => {
                HostOutcome::Ready(Value::Float(self.axes.get(&action).copied().unwrap_or(0.0)))
            }
            other => HostOutcome::Failed(format!("unknown input function `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_vs3::{AbiRegistry, SimulatedHost};

    #[test]
    fn built_in_descriptors_register_cleanly_together() {
        let mut registry = AbiRegistry::new();
        registry.register(time_module()).unwrap();
        registry.register(input_module()).unwrap();
        assert_eq!(registry.len(), 2);
        assert!(registry.function("velvet.time.now_ms").is_some());
        assert!(registry.function("velvet.input.pressed").is_some());
    }

    #[test]
    fn time_host_reports_last_pushed_snapshot() {
        let mut host = TimeModuleHost::new();
        assert_eq!(host.snapshot(), (0, 0.0));

        host.update(172_800_000, 1.0 / 30.0);
        let ms = HostRequest::new("velvet.time.now_ms", Value::list(vec![])).unwrap();
        assert_eq!(host.call(&ms), HostOutcome::Ready(Value::Int(172_800_000)));

        let delta = HostRequest::new("velvet.time.delta_secs", Value::list(vec![])).unwrap();
        assert_eq!(
            host.call(&delta),
            HostOutcome::Ready(Value::Float(1.0 / 30.0))
        );

        let junk = HostRequest::new("velvet.time.now_ms", Value::Int(1)).unwrap();
        assert!(matches!(host.call(&junk), HostOutcome::Failed(_)));
    }

    #[test]
    fn input_host_defaults_and_updates_actions() {
        let mut host = InputModuleHost::new();
        let jump = HostRequest::new(
            "velvet.input.pressed",
            Value::list(vec![velvet_script_vs3::string_val("jump")]),
        )
        .unwrap();
        assert_eq!(host.call(&jump), HostOutcome::Ready(Value::Bool(false)));

        host.set_pressed("jump", true);
        host.set_axis("steer", -0.5);
        assert_eq!(host.call(&jump), HostOutcome::Ready(Value::Bool(true)));

        let steer = HostRequest::new(
            "velvet.input.axis",
            Value::list(vec![velvet_script_vs3::string_val("steer")]),
        )
        .unwrap();
        assert_eq!(host.call(&steer), HostOutcome::Ready(Value::Float(-0.5)));

        let wrong_type =
            HostRequest::new("velvet.input.pressed", Value::list(vec![Value::Int(7)])).unwrap();
        assert!(matches!(host.call(&wrong_type), HostOutcome::Failed(_)));

        let unknown = HostRequest::new("velvet.input.teleport", Value::list(vec![])).unwrap();
        assert!(matches!(host.call(&unknown), HostOutcome::Failed(_)));
    }

    #[test]
    fn scripts_read_time_through_the_generic_abi() {
        const SOURCE: &str = r#"
// @edition 3

function frame_clock() {
    let ms = yield(["velvet.time.now_ms", []])
    return ms
}
"#;
        let module = velvet_script_vs3::compile_bundle("game.vel", [("game.vel", SOURCE)])
            .expect("source compiles");
        let mut task = module.start("frame_clock", &[]).unwrap();

        let mut host = TimeModuleHost::new();
        host.update(1_234, 0.016);
        let policy = velvet_script_vs3::Vs3HostPolicy::deny_all()
            .allow("velvet.time.*")
            .unwrap();
        assert_eq!(
            task.drive_host_with_policy(&mut host, &policy).unwrap(),
            velvet_script_vs3::Vs3TaskStatus::Complete(Value::Int(1_234))
        );
    }

    #[test]
    fn simulated_host_can_stand_in_for_the_real_modules() {
        let mut registry = AbiRegistry::new();
        registry.register(time_module()).unwrap();
        let mut host = SimulatedHost::new(registry);
        host.respond_ready("velvet.time.now_ms", Value::Int(77));

        let request = HostRequest::new("velvet.time.now_ms", Value::list(vec![])).unwrap();
        assert_eq!(host.call(&request), HostOutcome::Ready(Value::Int(77)));
        assert_eq!(host.calls("velvet.time.now_ms"), 1);
    }
}
