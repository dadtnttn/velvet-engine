//! Typed ABI descriptors for externally registered VS3 modules.
//!
//! This is the first slice of the "typed external ABI" roadmap: a single
//! descriptor source for module functions (parameters, result, purity,
//! synchrony, cost, capabilities), a registry consulted before compiling or
//! running, request validation against those descriptors, and an official
//! [`SimulatedHost`] that lets hosts and tests exercise modules without
//! VelvetEngine.
//!
//! ```ignore
//! let module = ModuleDescriptor::new("math", "1.0.0", vec![FunctionDescriptor {
//!     name: "double".into(),
//!     service: "math.double".into(),
//!     params: vec![ParamDescriptor::new("value", Vs3AbiType::Int)],
//!     ..FunctionDescriptor::new(Vs3AbiType::Int)
//! }])?;
//! let mut registry = AbiRegistry::new();
//! registry.register(module)?;
//! let mut host = SimulatedHost::new(registry);
//! host.respond_ready("math.double", Value::Int(42));
//! ```

use std::collections::{BTreeMap, HashMap};

use crate::{HostOutcome, HostRequest, Value, Vs3Error, Vs3Host};

/// Runtime-checkable ABI type of one parameter or result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vs3AbiType {
    /// Any value.
    Any,
    /// `null`.
    Null,
    /// Boolean.
    Bool,
    /// Integer.
    Int,
    /// Floating point.
    Float,
    /// Integer or float.
    Number,
    /// String.
    Str,
    /// List; `Some` constrains every element.
    List(Option<Box<Vs3AbiType>>),
    /// String-keyed map.
    Map,
    /// Two-component float vector.
    Vec2,
    /// Three-component float vector.
    Vec3,
    /// Four-component float vector.
    Vec4,
    /// 3x3 matrix.
    Mat3,
    /// 4x4 matrix.
    Mat4,
    /// Quaternion `(x, y, z, w)`.
    Quat,
}

impl Vs3AbiType {
    /// Human-readable name used in diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::Number => "number",
            Self::Str => "string",
            Self::List(_) => "list",
            Self::Map => "map",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
            Self::Quat => "quat",
        }
    }
}

/// Whether `value` satisfies ABI type `ty`.
pub fn matches_abi_type(ty: &Vs3AbiType, value: &Value) -> bool {
    match ty {
        Vs3AbiType::Any => true,
        Vs3AbiType::Null => matches!(value, Value::Null),
        Vs3AbiType::Bool => matches!(value, Value::Bool(_)),
        Vs3AbiType::Int => matches!(value, Value::Int(_)),
        Vs3AbiType::Float => matches!(value, Value::Float(_)),
        Vs3AbiType::Number => matches!(value, Value::Int(_) | Value::Float(_)),
        Vs3AbiType::Str => matches!(value, Value::String(_)),
        Vs3AbiType::Map => matches!(value, Value::Map(_)),
        Vs3AbiType::Vec2 => matches!(value, Value::Vec2(_)),
        Vs3AbiType::Vec3 => matches!(value, Value::Vec3(_)),
        Vs3AbiType::Vec4 => matches!(value, Value::Vec4(_)),
        Vs3AbiType::Mat3 => matches!(value, Value::Mat3(_)),
        Vs3AbiType::Mat4 => matches!(value, Value::Mat4(_)),
        Vs3AbiType::Quat => matches!(value, Value::Quat(_)),
        Vs3AbiType::List(element) => match (element, value) {
            (None, Value::List(_)) => true,
            (Some(element), Value::List(list)) => list
                .borrow()
                .iter()
                .all(|item| matches_abi_type(element, item)),
            _ => false,
        },
    }
}

/// One named parameter of a described function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDescriptor {
    /// Parameter name in VS3 source order.
    pub name: String,
    /// Accepted value type.
    pub ty: Vs3AbiType,
}

impl ParamDescriptor {
    /// Describe one parameter.
    pub fn new(name: impl Into<String>, ty: Vs3AbiType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// Side-effect contract of a described function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purity {
    /// Deterministic, no observable side effects.
    Pure,
    /// May touch host state.
    Impure,
}

/// Completion contract of a described function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncKind {
    /// Always answers [`HostOutcome::Ready`].
    Immediate,
    /// May park the task with a pending ticket.
    Deferred,
}

/// Relative cost hint for schedulers and editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CostClass {
    /// Trivial work.
    Cheap,
    /// Noticeable but bounded work.
    Moderate,
    /// Heavy work; budget before calling.
    Expensive,
}

/// Descriptor of one externally registered function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDescriptor {
    /// Function name as exposed to VS3 (`module.name()`).
    pub name: String,
    /// Host service this function maps to.
    pub service: String,
    /// Parameters in call order.
    pub params: Vec<ParamDescriptor>,
    /// Result type.
    pub result: Vs3AbiType,
    /// Purity contract.
    pub purity: Purity,
    /// Synchrony contract.
    pub sync: SyncKind,
    /// Cost hint.
    pub cost: CostClass,
    /// Capability patterns required to call this function.
    pub capabilities: Vec<String>,
    /// Short documentation line for tooling.
    pub doc: String,
}

impl FunctionDescriptor {
    /// Start a descriptor with defaults (`Pure`, `Immediate`, `Cheap`, no
    /// capabilities); fill required fields via struct update syntax.
    pub fn new(result: Vs3AbiType) -> Self {
        Self {
            name: String::new(),
            service: String::new(),
            params: Vec::new(),
            result,
            purity: Purity::Pure,
            sync: SyncKind::Immediate,
            cost: CostClass::Cheap,
            capabilities: Vec::new(),
            doc: String::new(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if !crate::valid_service_segment(&self.name) {
            return Err(format!("invalid function name `{}`", self.name));
        }
        if !crate::valid_service_name(&self.service) {
            return Err(format!(
                "function `{}` maps to invalid service `{}`",
                self.name, self.service
            ));
        }
        if self.params.len() > 64 {
            return Err(format!("function `{}` exceeds 64 parameters", self.name));
        }
        for capability in &self.capabilities {
            if !valid_capability_pattern(capability) {
                return Err(format!(
                    "function `{}` declares invalid capability `{capability}`",
                    self.name
                ));
            }
        }
        Ok(())
    }
}

fn valid_capability_pattern(pattern: &str) -> bool {
    pattern.strip_suffix(".*").map_or_else(
        || crate::valid_service_name(pattern),
        crate::valid_service_name,
    )
}

/// Descriptor of one externally registered module.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModuleDescriptor {
    /// Stable public module identity (`package.module` or plain name).
    pub name: String,
    /// Semantic version string (`major.minor.patch[-pre]`).
    pub version: String,
    /// Short documentation line for tooling.
    pub doc: String,
    /// Functions exported by this module, keyed by function name.
    pub functions: BTreeMap<String, FunctionDescriptor>,
}

impl ModuleDescriptor {
    /// Build and validate a module descriptor from a function list.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        functions: Vec<FunctionDescriptor>,
    ) -> Result<Self, Vs3Error> {
        let mut module = Self {
            name: name.into(),
            version: version.into(),
            doc: String::new(),
            functions: BTreeMap::new(),
        };
        if !crate::valid_service_name(&module.name) {
            return Err(Vs3Error::Task(format!(
                "invalid ABI module name `{}`",
                module.name
            )));
        }
        if !valid_version(&module.version) {
            return Err(Vs3Error::Task(format!(
                "module `{}` declares invalid version `{}`",
                module.name, module.version
            )));
        }
        for function in functions {
            function.validate().map_err(|message| {
                Vs3Error::Task(format!("module `{}`: {message}", module.name))
            })?;
            if module
                .functions
                .insert(function.name.clone(), function)
                .is_some()
            {
                return Err(Vs3Error::Task(format!(
                    "module `{}` exports duplicate function names",
                    module.name
                )));
            }
        }
        Ok(module)
    }

    /// Attach a documentation line.
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

fn valid_version(version: &str) -> bool {
    if version.is_empty() || version.len() > 64 {
        return false;
    }
    let (core, _pre) = match version.split_once('-') {
        Some((core, pre)) => (core, pre),
        None => (version, ""),
    };
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Registry of typed external modules, consultable before compile or run.
#[derive(Debug, Default)]
pub struct AbiRegistry {
    modules: BTreeMap<String, ModuleDescriptor>,
    services: HashMap<String, String>,
}

impl AbiRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a module; rejects duplicate modules and duplicate service
    /// mappings across the whole registry.
    pub fn register(&mut self, module: ModuleDescriptor) -> Result<(), Vs3Error> {
        if self.modules.contains_key(&module.name) {
            return Err(Vs3Error::Task(format!(
                "ABI module `{}` is already registered",
                module.name
            )));
        }
        for function in module.functions.values() {
            if let Some(owner) = self.services.get(&function.service) {
                return Err(Vs3Error::Task(format!(
                    "service `{}` is already mapped by module `{owner}`",
                    function.service
                )));
            }
        }
        for function in module.functions.values() {
            self.services
                .insert(function.service.clone(), module.name.clone());
        }
        self.modules.insert(module.name.clone(), module);
        Ok(())
    }

    /// Registered module by name.
    pub fn module(&self, name: &str) -> Option<&ModuleDescriptor> {
        self.modules.get(name)
    }

    /// Function owning `service`, together with its module name.
    pub fn function(&self, service: &str) -> Option<(&str, &FunctionDescriptor)> {
        let owner = self.services.get(service)?;
        let module = self.modules.get(owner)?;
        // Service map guarantees presence.
        let function = module.functions.values().find(|f| f.service == service)?;
        Some((owner, function))
    }

    /// All registered module names in order.
    pub fn module_names(&self) -> Vec<&str> {
        self.modules.keys().map(String::as_str).collect()
    }

    /// Number of registered modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

/// Validate a host request against a function descriptor.
///
/// Checks the exact service mapping and that the payload is a list matching
/// declared parameter arity and types. An empty parameter list requires an
/// empty payload list.
pub fn validate_request(
    function: &FunctionDescriptor,
    request: &HostRequest,
) -> Result<(), String> {
    if request.service != function.service {
        return Err(format!(
            "service mismatch: descriptor maps `{}` but request targets `{}`",
            function.service, request.service
        ));
    }
    let Value::List(items) = &request.payload else {
        return Err(format!(
            "payload for `{}` must be an argument list",
            function.service
        ));
    };
    let items = items.borrow();
    if items.len() != function.params.len() {
        return Err(format!(
            "`{}` expects {} argument(s), got {}",
            function.service,
            function.params.len(),
            items.len()
        ));
    }
    for (index, (param, value)) in function.params.iter().zip(items.iter()).enumerate() {
        if !matches_abi_type(&param.ty, value) {
            return Err(format!(
                "argument {} (`{}`) does not satisfy type `{}`",
                index,
                param.name,
                param.ty.name()
            ));
        }
    }
    Ok(())
}

/// Official simulated host backed by an [`AbiRegistry`].
///
/// Validates every request against registered descriptors, replays canned
/// outcomes, counts calls per service, and never panics on malformed input —
/// the reference double for testing modules without VelvetEngine.
#[derive(Debug, Default)]
pub struct SimulatedHost {
    registry: Option<AbiRegistry>,
    responses: HashMap<String, HostOutcome>,
    calls: BTreeMap<String, usize>,
    rejected: usize,
}

impl SimulatedHost {
    /// Simulated host validating against `registry`.
    pub fn new(registry: AbiRegistry) -> Self {
        Self {
            registry: Some(registry),
            ..Self::default()
        }
    }

    /// Simulated host without a registry: any syntactically valid request is
    /// answered with the canned response or `Ready(null)`.
    pub fn unregistered() -> Self {
        Self::default()
    }

    /// Queue the outcome returned for `service`.
    pub fn respond(&mut self, service: impl Into<String>, outcome: HostOutcome) {
        self.responses.insert(service.into(), outcome);
    }

    /// Queue an immediate `Ready(value)` answer for `service`.
    pub fn respond_ready(&mut self, service: impl Into<String>, value: Value) {
        self.respond(service, HostOutcome::Ready(value));
    }

    /// Queue a deferred answer with ticket format `sim-{n}`.
    pub fn respond_pending(&mut self, service: impl Into<String>) -> String {
        let ticket = format!("sim-{}", self.responses.len() + 1);
        self.respond(
            service,
            HostOutcome::Pending {
                ticket: ticket.clone(),
            },
        );
        ticket
    }

    /// Calls recorded for `service`.
    pub fn calls(&self, service: &str) -> usize {
        self.calls.get(service).copied().unwrap_or(0)
    }

    /// Total recorded calls.
    pub fn total_calls(&self) -> usize {
        self.calls.values().sum()
    }

    /// Requests rejected because they were unknown or failed validation.
    pub fn rejected_calls(&self) -> usize {
        self.rejected
    }
}

impl Vs3Host for SimulatedHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        *self.calls.entry(request.service.clone()).or_default() += 1;
        let validated = match &self.registry {
            Some(registry) => match registry.function(&request.service) {
                Some((_, function)) => validate_request(function, request).err(),
                None => Some(format!("service `{}` is not registered", request.service)),
            },
            None => None,
        };
        if let Some(message) = validated {
            self.rejected += 1;
            return HostOutcome::Failed(message);
        }
        match self.responses.get(&request.service) {
            Some(outcome) => outcome.clone(),
            None => HostOutcome::Ready(Value::Null),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: i64) -> Value {
        Value::Int(v)
    }

    fn sample_module() -> ModuleDescriptor {
        ModuleDescriptor::new(
            "math",
            "1.2.0",
            vec![
                FunctionDescriptor {
                    name: "double".into(),
                    service: "math.double".into(),
                    params: vec![ParamDescriptor::new("value", Vs3AbiType::Int)],
                    ..FunctionDescriptor::new(Vs3AbiType::Int)
                },
                FunctionDescriptor {
                    name: "sum_all".into(),
                    service: "math.sum_all".into(),
                    params: vec![ParamDescriptor::new(
                        "values",
                        Vs3AbiType::List(Some(Box::new(Vs3AbiType::Number))),
                    )],
                    result: Vs3AbiType::Number,
                    purity: Purity::Pure,
                    sync: SyncKind::Immediate,
                    cost: CostClass::Moderate,
                    capabilities: vec!["math.*".into()],
                    doc: "adds numbers".into(),
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn registry_rejects_duplicate_modules_and_service_collisions() {
        let mut registry = AbiRegistry::new();
        registry.register(sample_module()).unwrap();
        assert_eq!(registry.module_names(), vec!["math"]);

        let dup = registry.register(sample_module()).unwrap_err();
        assert!(dup.to_string().contains("already registered"));

        let clash = ModuleDescriptor::new(
            "other",
            "0.1.0",
            vec![FunctionDescriptor {
                name: "twice".into(),
                service: "math.double".into(),
                ..FunctionDescriptor::new(Vs3AbiType::Int)
            }],
        )
        .unwrap();
        let err = registry.register(clash).unwrap_err();
        assert!(err.to_string().contains("already mapped"));
    }

    #[test]
    fn module_descriptor_validates_names_versions_and_capabilities() {
        let bad_name = ModuleDescriptor::new("1bad", "1.0.0", vec![]).unwrap_err();
        assert!(bad_name.to_string().contains("invalid ABI module name"));

        let bad_version = ModuleDescriptor::new("ok", "one.two", vec![]).unwrap_err();
        assert!(bad_version.to_string().contains("invalid version"));

        let bad_function = ModuleDescriptor::new(
            "ok",
            "1.0.0",
            vec![FunctionDescriptor {
                name: "valid_name".into(),
                service: "not a service".into(),
                ..FunctionDescriptor::new(Vs3AbiType::Null)
            }],
        )
        .unwrap_err();
        assert!(bad_function.to_string().contains("invalid service"));

        let bad_capability = ModuleDescriptor::new(
            "ok",
            "1.0.0",
            vec![FunctionDescriptor {
                name: "valid_name".into(),
                service: "ok.thing".into(),
                capabilities: vec!["9lives".into()],
                ..FunctionDescriptor::new(Vs3AbiType::Null)
            }],
        )
        .unwrap_err();
        assert!(bad_capability.to_string().contains("invalid capability"));

        let pre = ModuleDescriptor::new("ok", "1.0.0-alpha.1", vec![]);
        assert!(pre.is_ok());
    }

    #[test]
    fn abi_type_matching_covers_lists_and_numbers() {
        let number_list = Vs3AbiType::List(Some(Box::new(Vs3AbiType::Number)));
        let values = Value::list(vec![int(1), Value::Float(2.5)]);
        assert!(matches_abi_type(&number_list, &values));

        let mixed = Value::list(vec![int(1), Value::Bool(false)]);
        assert!(!matches_abi_type(&number_list, &mixed));

        let any_list = Vs3AbiType::List(None);
        assert!(matches_abi_type(&any_list, &mixed));
        assert!(!matches_abi_type(&Vs3AbiType::Number, &Value::Bool(true)));
        assert!(matches_abi_type(&Vs3AbiType::Quat, &Value::Quat([0.0; 4])));
    }

    #[test]
    fn validate_request_checks_service_arity_and_types() {
        let module = sample_module();
        let double = module.functions.get("double").unwrap();

        let ok = HostRequest::new("math.double", Value::list(vec![int(21)])).unwrap();
        assert!(validate_request(double, &ok).is_ok());

        let wrong_service = HostRequest::new("math.sum_all", Value::list(vec![int(21)])).unwrap();
        assert!(validate_request(double, &wrong_service)
            .unwrap_err()
            .contains("service mismatch"));

        let wrong_arity =
            HostRequest::new("math.double", Value::list(vec![int(1), int(2)])).unwrap();
        assert!(validate_request(double, &wrong_arity)
            .unwrap_err()
            .contains("expects 1 argument"));

        let not_a_list = HostRequest::new("math.double", int(21)).unwrap();
        assert!(validate_request(double, &not_a_list)
            .unwrap_err()
            .contains("argument list"));

        let wrong_type =
            HostRequest::new("math.double", Value::list(vec![Value::Bool(true)])).unwrap();
        assert!(validate_request(double, &wrong_type)
            .unwrap_err()
            .contains("does not satisfy type `int`"));

        let sum = module.functions.get("sum_all").unwrap();
        let floats = HostRequest::new(
            "math.sum_all",
            Value::list(vec![Value::list(vec![int(1), Value::Float(2.5)])]),
        )
        .unwrap();
        assert!(validate_request(sum, &floats).is_ok());
    }

    #[test]
    fn simulated_host_rejects_unknown_and_invalid_requests() {
        let mut registry = AbiRegistry::new();
        registry.register(sample_module()).unwrap();
        let mut host = SimulatedHost::new(registry);

        let unknown = HostRequest::new("ghost.service", Value::Null).unwrap();
        assert!(matches!(host.call(&unknown), HostOutcome::Failed(_)));
        assert_eq!(host.rejected_calls(), 1);

        let invalid = HostRequest::new("math.double", Value::Null).unwrap();
        assert!(
            matches!(host.call(&invalid), HostOutcome::Failed(msg) if msg.contains("argument list"))
        );
        assert_eq!(host.calls("math.double"), 1);
        assert_eq!(host.total_calls(), 2);
        assert_eq!(host.rejected_calls(), 2);

        host.respond_ready("math.double", int(42));
        let good = HostRequest::new("math.double", Value::list(vec![int(21)])).unwrap();
        assert_eq!(host.call(&good), HostOutcome::Ready(int(42)));
    }

    #[test]
    fn simulated_host_defaults_to_ready_null_and_issues_pending_tickets() {
        let mut host = SimulatedHost::unregistered();
        let plain = HostRequest::new("anything.goes", Value::list(vec![])).unwrap();
        assert_eq!(host.call(&plain), HostOutcome::Ready(Value::Null));

        let ticket = host.respond_pending("async.work");
        assert_eq!(ticket, "sim-1");
        let deferred = HostRequest::new("async.work", Value::list(vec![])).unwrap();
        match host.call(&deferred) {
            HostOutcome::Pending { ticket } => assert_eq!(ticket, "sim-1"),
            other => panic!("expected pending, got {other:?}"),
        }
    }

    #[test]
    fn simulated_host_drives_real_tasks_without_the_engine() {
        const SOURCE: &str = r#"
// @edition 3

function ask_double(value) {
    return yield(["math.double", [value]])
}
"#;
        let module = crate::compile_bundle("game.vel", [("game.vel", SOURCE)])
            .expect("task source compiles");
        let mut task = module.start("ask_double", &[int(21)]).unwrap();

        let mut registry = AbiRegistry::new();
        registry.register(sample_module()).unwrap();
        let mut host = SimulatedHost::new(registry);
        host.respond_ready("math.double", int(42));

        assert_eq!(
            task.drive_host(&mut host).unwrap(),
            crate::Vs3TaskStatus::Complete(int(42))
        );
        assert_eq!(host.calls("math.double"), 1);
    }

    #[test]
    fn deferred_simulated_host_parks_and_resumes_by_ticket() {
        const SOURCE: &str = r#"
// @edition 3

function open_dialogue(text) {
    return yield(["ui.dialogue.open", [text]])
}
"#;
        let extra = ModuleDescriptor::new(
            "ui",
            "1.0.0",
            vec![FunctionDescriptor {
                name: "open".into(),
                service: "ui.dialogue.open".into(),
                params: vec![ParamDescriptor::new("text", Vs3AbiType::Str)],
                sync: SyncKind::Deferred,
                ..FunctionDescriptor::new(Vs3AbiType::Null)
            }],
        )
        .unwrap();
        let mut registry = AbiRegistry::new();
        registry.register(extra).unwrap();
        let mut host = SimulatedHost::new(registry);
        let expected_ticket = host.respond_pending("ui.dialogue.open");

        let module = crate::compile_bundle("game.vel", [("game.vel", SOURCE)])
            .expect("task source compiles");
        let mut task = module
            .start("open_dialogue", &[crate::string_val("Hello")])
            .unwrap();
        let ticket = match task.drive_host(&mut host).unwrap() {
            crate::Vs3TaskStatus::Waiting { ticket, request } => {
                assert_eq!(
                    request.payload,
                    Value::list(vec![crate::string_val("Hello")])
                );
                ticket
            }
            status => panic!("expected waiting, got {status:?}"),
        };
        assert_eq!(ticket, expected_ticket);
        assert_eq!(
            task.resume_host(&ticket, Value::Null).unwrap(),
            crate::Vs3TaskStatus::Complete(Value::Null)
        );
        assert_eq!(host.calls("ui.dialogue.open"), 1);
    }
}
