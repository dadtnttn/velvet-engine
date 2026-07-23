//! # velvet-script-vs3
//!
//! Official general game-logic language selected by `// @edition 3`.
//! Classic product story files stay on their separate `StoryProgram` path.
//!
//! VS3 provides real functions and persistent state, checked annotations,
//! mutable lists/maps, structured loop control, bounded bytecode execution,
//! cooperative tasks, and a capability-limited host ABI. Engine features are
//! libraries and services built on general values, not genre-specific syntax.

#![deny(missing_docs)]

mod semantic;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use thiserror::Error;
use velvet_script_ast::{Diagnostic, Severity, SourceLoc};
use velvet_script_bytecode::fnv1a64;
use velvet_script_compiler::{compile as compile_ast, CompileError, CompileResult};
use velvet_script_lexer::Span;
use velvet_script_parser::parse_file;
use velvet_script_vm::{Coroutine, CoroutineStatus, Vm, VmError};

/// Runtime values, limits, and native metadata re-exported for hosts and tooling.
pub use velvet_script_vm::{NativeId, NativePurity, NativeSpec, NativeType, Value, VmLimits};

/// Runtime-checkable VS3 type annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vs3Type {
    /// `any` accepts every value.
    Any,
    /// `null`.
    Null,
    /// `bool`.
    Bool,
    /// Signed 64-bit `int`.
    Int,
    /// 64-bit `float`; integer values are accepted at boundaries.
    Float,
    /// UTF-8 `string` / `str`.
    String,
    /// Mutable `list`.
    List,
    /// Mutable string-keyed `map`.
    Map,
    /// Immutable two-component vector.
    Vec2,
    /// Immutable three-component vector.
    Vec3,
    /// Immutable four-component vector.
    Vec4,
    /// Immutable 3x3 matrix.
    Mat3,
    /// Immutable 4x4 matrix.
    Mat4,
    /// Immutable quaternion.
    Quat,
    /// Mutable deterministic random stream.
    Rng,
}

impl Vs3Type {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "any" => Self::Any,
            "null" => Self::Null,
            "bool" => Self::Bool,
            "int" | "i64" => Self::Int,
            "float" | "f64" | "number" => Self::Float,
            "string" | "str" => Self::String,
            "list" | "array" => Self::List,
            "map" => Self::Map,
            "vec2" => Self::Vec2,
            "vec3" => Self::Vec3,
            "vec4" => Self::Vec4,
            "mat3" => Self::Mat3,
            "mat4" => Self::Mat4,
            "quat" | "quaternion" => Self::Quat,
            "rng" | "random" => Self::Rng,
            _ => return None,
        })
    }

    /// Canonical source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::String => "string",
            Self::List => "list",
            Self::Map => "map",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
            Self::Quat => "quat",
            Self::Rng => "rng",
        }
    }

    fn accepts(self, found: Self) -> bool {
        self == Self::Any
            || found == Self::Any
            || self == found
            || (self == Self::Float && found == Self::Int)
    }

    fn accepts_value(self, value: &Value) -> bool {
        self == Self::Any
            || matches!(
                (self, value),
                (Self::Null, Value::Null)
                    | (Self::Bool, Value::Bool(_))
                    | (Self::Int, Value::Int(_))
                    | (Self::Float, Value::Int(_) | Value::Float(_))
                    | (Self::String, Value::String(_))
                    | (Self::List, Value::List(_))
                    | (Self::Map, Value::Map(_))
                    | (Self::Vec2, Value::Vec2(_))
                    | (Self::Vec3, Value::Vec3(_))
                    | (Self::Vec4, Value::Vec4(_))
                    | (Self::Mat3, Value::Mat3(_))
                    | (Self::Mat4, Value::Mat4(_))
                    | (Self::Quat, Value::Quat(_))
                    | (Self::Rng, Value::Rng(_))
            )
    }

    fn from_native(native: NativeType) -> Option<Self> {
        Some(match native {
            NativeType::Any | NativeType::Vector | NativeType::Matrix => Self::Any,
            NativeType::Number | NativeType::Float => Self::Float,
            NativeType::Null => Self::Null,
            NativeType::Bool => Self::Bool,
            NativeType::Int => Self::Int,
            NativeType::String => Self::String,
            NativeType::List => Self::List,
            NativeType::Map => Self::Map,
            NativeType::Vec2 => Self::Vec2,
            NativeType::Vec3 => Self::Vec3,
            NativeType::Vec4 => Self::Vec4,
            NativeType::Mat3 => Self::Mat3,
            NativeType::Mat4 => Self::Mat4,
            NativeType::Quat => Self::Quat,
            NativeType::Rng => Self::Rng,
        })
    }
}

/// Human-readable list of the **proven** VS3 surface (docs / tooling).
pub const SUPPORTED_SURFACE: &[&str] = &[
    "edition: // @edition 3",
    "function name(params) { body }",
    "checked annotations: any null bool int float string list map vec2 vec3 vec4 mat3 mat4 quat rng",
    "return expr",
    "let name = expr",
    "const name = expr",
    "persistent state through Vs3Session",
    "multi-file packages with module::function calls",
    "source bundles with import \"relative/path.vel\" directives",
    "if cond { } else { }",
    "while cond { }",
    "for value in collection with break and continue",
    "null/int/bool/float/string/list/map literals",
    "indexing and mutable list/map values",
    "ops: + - * / % += -= *= /= == != < <= > >= && || !",
    "cooperative tasks: yield(value), resume, resume_with",
    "generic host ABI: yield [service, payload] and resume with Value",
    "host capability policies and immediate request budgets",
    "natives: advanced math vectors matrices quaternions rng noise statistics curves data collections",
    "deterministic maps and cycle-safe structured values",
    "bounded sandbox runtime with checked integer arithmetic",
    "compatibility adapter: present_show set_bg present_hide ui_flag ui_flag_get",
    "call via Vs3Module::call/session/start / eval_call / velvet vs3",
];

/// Parsed source edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Classic / unspecified (not VS3).
    Classic,
    /// Historical VS2 marker (not official; rejected for VS3 API).
    Vs2,
    /// Official general logic language.
    Vs3,
}

impl Edition {
    /// Display name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Vs2 => "2",
            Self::Vs3 => "3",
        }
    }
}

/// Detect `// @edition N` (or `# @edition N`) from the first ~40 lines / 2 KiB.
pub fn detect_edition(source: &str) -> Edition {
    let head = source.lines().take(40);
    for line in head {
        let t = line.trim();
        // strip line comments
        let body = t
            .strip_prefix("//")
            .or_else(|| t.strip_prefix('#'))
            .unwrap_or(t)
            .trim();
        if let Some(rest) = body.strip_prefix("@edition") {
            let n = rest.trim();
            return match n {
                "3" | "vs3" | "VS3" => Edition::Vs3,
                "2" | "vs2" | "VS2" => Edition::Vs2,
                "1" | "classic" => Edition::Classic,
                _ => Edition::Classic,
            };
        }
    }
    Edition::Classic
}

/// Compile an embedded source bundle whose root may use textual imports.
///
/// Import directives are standalone lines such as `import "logic/combat.vel"`.
/// Paths are resolved relative to the importing file, expanded exactly once,
/// and compiled as one VS3 module, so all fragments share functions and state.
/// This is intentionally a source-composition feature rather than nominal
/// cross-module linking.
pub fn compile_bundle<K, V, I>(root: &str, sources: I) -> Result<Vs3Module, Vs3Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    let root = normalize_bundle_path("", root).map_err(|message| Vs3Error::Bundle {
        path: root.to_string(),
        message,
    })?;
    let mut source_map = BTreeMap::new();
    for (name, source) in sources {
        let original = name.into();
        let normalized =
            normalize_bundle_path("", &original).map_err(|message| Vs3Error::Bundle {
                path: original.clone(),
                message,
            })?;
        if source_map
            .insert(normalized.clone(), source.into())
            .is_some()
        {
            return Err(Vs3Error::Bundle {
                path: normalized,
                message: "duplicate source path".into(),
            });
        }
    }
    let root_source = source_map.get(&root).ok_or_else(|| Vs3Error::Bundle {
        path: root.clone(),
        message: "root source is missing from the bundle".into(),
    })?;
    if detect_edition(root_source) != Edition::Vs3 {
        return Err(Vs3Error::Edition(format!(
            "VS3 bundle root `{root}` requires `// @edition 3`"
        )));
    }

    let mut output = String::from("// @edition 3\n");
    let mut expanded = BTreeSet::new();
    let mut visiting = Vec::new();
    expand_bundle_source(
        &root,
        &source_map,
        &mut expanded,
        &mut visiting,
        &mut output,
    )?;
    compile(&output, Some(&root))
}

/// Compile a filesystem VS3 root and every relative import reachable from it.
///
/// Imports are restricted to the root file's directory tree; a path that would
/// escape that directory is rejected before any file is read.
pub fn compile_path(path: impl AsRef<Path>) -> Result<Vs3Module, Vs3Error> {
    let path = path.as_ref();
    let root_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let root_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Vs3Error::Bundle {
            path: path.display().to_string(),
            message: "root path has no UTF-8 file name".into(),
        })?;
    let root_name = normalize_bundle_path("", root_name).map_err(|message| Vs3Error::Bundle {
        path: path.display().to_string(),
        message,
    })?;
    let mut sources = BTreeMap::new();
    collect_bundle_files(root_dir, &root_name, &mut sources, &mut BTreeSet::new())?;
    compile_bundle(&root_name, sources)
}

fn collect_bundle_files(
    root_dir: &Path,
    virtual_path: &str,
    sources: &mut BTreeMap<String, String>,
    visiting: &mut BTreeSet<String>,
) -> Result<(), Vs3Error> {
    if sources.contains_key(virtual_path) {
        return Ok(());
    }
    if !visiting.insert(virtual_path.to_string()) {
        return Err(Vs3Error::Bundle {
            path: virtual_path.to_string(),
            message: "cyclic import while loading filesystem sources".into(),
        });
    }
    let actual_path = root_dir.join(PathBuf::from(virtual_path));
    let source = std::fs::read_to_string(&actual_path).map_err(|error| Vs3Error::Bundle {
        path: virtual_path.to_string(),
        message: format!("cannot read {}: {error}", actual_path.display()),
    })?;
    for (line_index, line) in source.lines().enumerate() {
        let target = parse_bundle_import(line).map_err(|message| Vs3Error::Bundle {
            path: virtual_path.to_string(),
            message: format!("line {}: {message}", line_index + 1),
        })?;
        if let Some(target) = target {
            let resolved = normalize_bundle_path(virtual_path, target).map_err(|message| {
                Vs3Error::Bundle {
                    path: virtual_path.to_string(),
                    message: format!("line {}: {message}", line_index + 1),
                }
            })?;
            collect_bundle_files(root_dir, &resolved, sources, visiting)?;
        }
    }
    visiting.remove(virtual_path);
    sources.insert(virtual_path.to_string(), source);
    Ok(())
}

fn expand_bundle_source(
    path: &str,
    sources: &BTreeMap<String, String>,
    expanded: &mut BTreeSet<String>,
    visiting: &mut Vec<String>,
    output: &mut String,
) -> Result<(), Vs3Error> {
    if expanded.contains(path) {
        return Ok(());
    }
    if let Some(index) = visiting.iter().position(|item| item == path) {
        let mut cycle = visiting[index..].to_vec();
        cycle.push(path.to_string());
        return Err(Vs3Error::Bundle {
            path: path.to_string(),
            message: format!("cyclic import: {}", cycle.join(" -> ")),
        });
    }
    let source = sources.get(path).ok_or_else(|| Vs3Error::Bundle {
        path: path.to_string(),
        message: "imported source is missing from the bundle".into(),
    })?;
    visiting.push(path.to_string());
    output.push_str("// @vs3-bundle begin ");
    output.push_str(path);
    output.push('\n');
    for (line_index, line) in source.lines().enumerate() {
        let target = parse_bundle_import(line).map_err(|message| Vs3Error::Bundle {
            path: path.to_string(),
            message: format!("line {}: {message}", line_index + 1),
        })?;
        if let Some(target) = target {
            let resolved =
                normalize_bundle_path(path, target).map_err(|message| Vs3Error::Bundle {
                    path: path.to_string(),
                    message: format!("line {}: {message}", line_index + 1),
                })?;
            expand_bundle_source(&resolved, sources, expanded, visiting, output)?;
        } else if !is_edition_directive(line) {
            output.push_str(line);
            output.push('\n');
        }
    }
    output.push_str("// @vs3-bundle end ");
    output.push_str(path);
    output.push('\n');
    visiting.pop();
    expanded.insert(path.to_string());
    Ok(())
}

fn parse_bundle_import(line: &str) -> Result<Option<&str>, String> {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("import") else {
        return Ok(None);
    };
    if rest
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace())
    {
        return Ok(None);
    }
    let rest = rest.trim();
    if !rest.starts_with('"') {
        return Err("imports use `import \"relative/path.vel\"`".into());
    }
    let quoted = &rest[1..];
    let Some(end) = quoted.find('"') else {
        return Err("unterminated import path".into());
    };
    let target = &quoted[..end];
    if target.is_empty() {
        return Err("import path cannot be empty".into());
    }
    let tail = quoted[end + 1..].trim();
    let tail = tail.strip_prefix(';').unwrap_or(tail).trim();
    if !tail.is_empty() && !tail.starts_with("//") {
        return Err("unexpected text after import path".into());
    }
    Ok(Some(target))
}

fn is_edition_directive(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed
        .strip_prefix("//")
        .or_else(|| trimmed.strip_prefix('#'))
        .is_some_and(|body| body.trim_start().starts_with("@edition"))
}

fn normalize_bundle_path(current: &str, target: &str) -> Result<String, String> {
    let target = target.replace('\\', "/");
    if target.is_empty() || target.starts_with('/') || target.contains(':') {
        return Err(format!("invalid relative import path `{target}`"));
    }
    let mut parts = Vec::new();
    if let Some((parent, _)) = current.rsplit_once('/') {
        parts.extend(
            parent
                .split('/')
                .filter(|part| !part.is_empty())
                .map(str::to_string),
        );
    }
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("import path escapes the bundle root: `{target}`"));
                }
            }
            value if value.contains('\0') => return Err("import path contains NUL".into()),
            value => parts.push(value.to_string()),
        }
    }
    if parts.is_empty() {
        return Err(format!("invalid relative import path `{target}`"));
    }
    let normalized = parts.join("/");
    if normalized.len() > 512 {
        return Err("import path exceeds 512 bytes".into());
    }
    Ok(normalized)
}

/// VS3 diagnostic with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs3Diagnostic {
    /// Human message.
    pub message: String,
    /// Location (file:line:col when known).
    pub loc: SourceLoc,
}

impl Vs3Diagnostic {
    /// Format like other Velvet diagnostics.
    pub fn display(&self) -> String {
        format!("{}: {}", self.loc.display(), self.message)
    }
}

/// VS3 error (compile / runtime / edition).
#[derive(Debug, Error, Clone, PartialEq)]
pub enum Vs3Error {
    /// Wrong or missing edition for the VS3 API.
    #[error("{0}")]
    Edition(String),
    /// Compile / parse failure with structured diagnostics.
    #[error("{}", display_diags(.0))]
    Compile(Vec<Vs3Diagnostic>),
    /// Source-bundle loading or import-resolution failure.
    #[error("source bundle `{path}`: {message}")]
    Bundle {
        /// Virtual or filesystem-relative source path.
        path: String,
        /// Controlled loader or resolver message.
        message: String,
    },
    /// Runtime failure.
    #[error("{loc}: {message}{}", display_stack(.stack_trace))]
    Runtime {
        /// Message.
        message: String,
        /// Location display.
        loc: String,
        /// Innermost-to-outermost script frames.
        stack_trace: Vec<String>,
    },
    /// A generic engine/host service failed.
    #[error("host service `{service}` failed: {message}")]
    Host {
        /// Service identifier requested by the script.
        service: String,
        /// Host-provided error message.
        message: String,
    },
    /// Script requested a service outside its granted capabilities.
    #[error("host service `{service}` is not permitted")]
    Permission {
        /// Denied service identifier.
        service: String,
    },
    /// Invalid task state or host-resume protocol.
    #[error("task error: {0}")]
    Task(String),
}

fn display_diags(diags: &[Vs3Diagnostic]) -> String {
    if diags.is_empty() {
        return "compile failed".into();
    }
    diags
        .iter()
        .map(Vs3Diagnostic::display)
        .collect::<Vec<_>>()
        .join("\n")
}

fn display_stack(stack: &[String]) -> String {
    if stack.is_empty() {
        String::new()
    } else {
        format!("\n{}", stack.join("\n"))
    }
}

impl Vs3Error {
    /// All structured diagnostics (empty for pure runtime/edition).
    pub fn diagnostics(&self) -> &[Vs3Diagnostic] {
        match self {
            Self::Compile(d) => d,
            _ => &[],
        }
    }

    /// True if any diagnostic carries a non-zero line number.
    pub fn has_located_diagnostic(&self) -> bool {
        self.diagnostics().iter().any(|d| d.loc.line > 0)
            || matches!(self, Self::Runtime { loc, .. } if loc.contains(':'))
    }
}

/// Generic request emitted by VS3 code to an engine host.
///
/// The source-level wire format is a two-item list: `[service, payload]`.
/// This keeps VS3 independent from rendering, audio, novels, cards, or any
/// other product domain.
#[derive(Debug, Clone, PartialEq)]
pub struct HostRequest {
    /// Stable service identifier such as `audio.play` or `ui.dialogue.open`.
    pub service: String,
    /// Arbitrary structured VS3 payload.
    pub payload: Value,
}

impl HostRequest {
    /// Create a request.
    pub fn new(service: impl Into<String>, payload: Value) -> Self {
        Self {
            service: service.into(),
            payload,
        }
    }

    /// Create a request after validating its service identifier.
    pub fn try_new(service: impl Into<String>, payload: Value) -> Result<Self, Vs3Error> {
        let service = service.into();
        if !valid_service_name(&service) {
            return Err(Vs3Error::Task(format!(
                "invalid host service identifier `{service}`"
            )));
        }
        Ok(Self { service, payload })
    }

    /// Decode the canonical `[service, payload]` wire value.
    pub fn from_value(value: &Value) -> Option<Self> {
        let Value::List(values) = value else {
            return None;
        };
        let values = values.borrow();
        if values.len() != 2 {
            return None;
        }
        let service = values[0].as_str()?.to_string();
        if !valid_service_name(&service) {
            return None;
        }
        Some(Self {
            service,
            payload: values[1].clone(),
        })
    }

    /// Encode this request as the canonical VS3 list value.
    pub fn into_value(self) -> Value {
        Value::list(vec![string_val(self.service), self.payload])
    }
}

fn valid_service_name(service: &str) -> bool {
    !service.is_empty()
        && service.len() <= 128
        && service
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Capability allowlist and request budget for host-driven tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs3HostPolicy {
    allowed: Option<BTreeSet<String>>,
    /// Maximum immediately-completed requests handled by one drive call.
    pub max_immediate_requests: usize,
}

impl Vs3HostPolicy {
    /// Permit every syntactically valid service.
    pub fn allow_all() -> Self {
        Self {
            allowed: None,
            max_immediate_requests: 1_024,
        }
    }

    /// Deny every service until explicitly added.
    pub fn deny_all() -> Self {
        Self {
            allowed: Some(BTreeSet::new()),
            max_immediate_requests: 1_024,
        }
    }

    /// Grant one exact service or a namespace wildcard such as `audio.*`.
    pub fn allow(mut self, service: impl Into<String>) -> Self {
        self.allowed
            .get_or_insert_with(BTreeSet::new)
            .insert(service.into());
        self
    }

    /// Whether a service is granted by this policy.
    pub fn permits(&self, service: &str) -> bool {
        match &self.allowed {
            None => true,
            Some(allowed) => {
                allowed.contains(service)
                    || allowed.iter().any(|entry| {
                        entry.strip_suffix(".*").is_some_and(|prefix| {
                            service.starts_with(prefix)
                                && service.as_bytes().get(prefix.len()) == Some(&b'.')
                        })
                    })
            }
        }
    }
}

impl Default for Vs3HostPolicy {
    fn default() -> Self {
        Self::deny_all()
    }
}

/// Result returned by a general engine host.
#[derive(Debug, Clone, PartialEq)]
pub enum HostOutcome {
    /// Service completed immediately.
    Ready(Value),
    /// Service will complete later; the ticket identifies that operation.
    Pending {
        /// Opaque host ticket.
        ticket: String,
    },
    /// Service failed without panicking the VM.
    Failed(String),
}

/// General VS3 host interface.
///
/// Implementations may route service names to rendering, audio, storage,
/// networking, UI, gameplay systems, or test doubles. VS3 itself does not
/// know what those services mean.
pub trait Vs3Host {
    /// Handle one request emitted by a VS3 task.
    fn call(&mut self, request: &HostRequest) -> HostOutcome;
}

/// Observable state of a cooperative VS3 task.
#[derive(Debug, Clone, PartialEq)]
pub enum Vs3TaskStatus {
    /// Script yielded a value that is not a canonical host request.
    Yielded(Value),
    /// Host accepted a request and will answer later.
    Waiting {
        /// Opaque host ticket.
        ticket: String,
        /// Request associated with the ticket.
        request: HostRequest,
    },
    /// Function returned normally.
    Complete(Value),
}

/// Suspensible VS3 function invocation.
#[derive(Debug)]
pub struct Vs3Task {
    coroutine: Coroutine,
    pending: Option<(String, HostRequest)>,
}

impl Vs3Task {
    /// Resume without replacing the previous yielded expression value.
    pub fn resume(&mut self) -> Result<Vs3TaskStatus, Vs3Error> {
        if self.pending.is_some() {
            return Err(Vs3Error::Task(
                "task is waiting for a host ticket; use resume_host".into(),
            ));
        }
        self.coroutine.resume().map(task_status).map_err(map_vm_err)
    }

    /// Resume an explicit `yield(value)` with a replacement value.
    pub fn resume_with(&mut self, value: Value) -> Result<Vs3TaskStatus, Vs3Error> {
        if self.pending.is_some() {
            return Err(Vs3Error::Task(
                "task is waiting for a host ticket; use resume_host".into(),
            ));
        }
        self.coroutine
            .resume_with(value)
            .map(task_status)
            .map_err(map_vm_err)
    }

    /// Drive the task through immediately available generic host services.
    ///
    /// Non-request yields are returned to the caller. Pending services return
    /// [`Vs3TaskStatus::Waiting`] and must later be completed with
    /// [`Self::resume_host`].
    pub fn drive_host<H: Vs3Host>(&mut self, host: &mut H) -> Result<Vs3TaskStatus, Vs3Error> {
        self.drive_host_with_policy(host, &Vs3HostPolicy::allow_all())
    }

    /// Drive through host services under an explicit capability policy.
    pub fn drive_host_with_policy<H: Vs3Host>(
        &mut self,
        host: &mut H,
        policy: &Vs3HostPolicy,
    ) -> Result<Vs3TaskStatus, Vs3Error> {
        if let Some((ticket, request)) = &self.pending {
            return Ok(Vs3TaskStatus::Waiting {
                ticket: ticket.clone(),
                request: request.clone(),
            });
        }

        let mut status = self.resume()?;
        let mut immediate_requests = 0usize;
        loop {
            match status {
                Vs3TaskStatus::Yielded(value) => {
                    let Some(request) = HostRequest::from_value(&value) else {
                        return Ok(Vs3TaskStatus::Yielded(value));
                    };
                    if !policy.permits(&request.service) {
                        return Err(Vs3Error::Permission {
                            service: request.service,
                        });
                    }
                    immediate_requests += 1;
                    if immediate_requests > policy.max_immediate_requests {
                        return Err(Vs3Error::Task(format!(
                            "host request budget exceeded ({})",
                            policy.max_immediate_requests
                        )));
                    }
                    match host.call(&request) {
                        HostOutcome::Ready(response) => {
                            status = self.resume_with(response)?;
                        }
                        HostOutcome::Pending { ticket } => {
                            if ticket.is_empty() {
                                return Err(Vs3Error::Task(
                                    "host returned an empty pending ticket".into(),
                                ));
                            }
                            self.pending = Some((ticket.clone(), request.clone()));
                            return Ok(Vs3TaskStatus::Waiting { ticket, request });
                        }
                        HostOutcome::Failed(message) => {
                            return Err(Vs3Error::Host {
                                service: request.service,
                                message,
                            });
                        }
                    }
                }
                other => return Ok(other),
            }
        }
    }

    /// Deliver the result for a previously pending host request.
    pub fn resume_host(
        &mut self,
        ticket: &str,
        response: Value,
    ) -> Result<Vs3TaskStatus, Vs3Error> {
        let Some((expected, request)) = self.pending.take() else {
            return Err(Vs3Error::Task("task has no pending host request".into()));
        };
        if expected != ticket {
            self.pending = Some((expected.clone(), request));
            return Err(Vs3Error::Task(format!(
                "host ticket mismatch: expected `{expected}`, got `{ticket}`"
            )));
        }
        self.coroutine
            .resume_with(response)
            .map(task_status)
            .map_err(map_vm_err)
    }

    /// Whether this task is waiting for an asynchronous host response.
    pub fn is_waiting_for_host(&self) -> bool {
        self.pending.is_some()
    }
}

fn task_status(status: CoroutineStatus) -> Vs3TaskStatus {
    match status {
        CoroutineStatus::Yielded(value) => Vs3TaskStatus::Yielded(value),
        CoroutineStatus::Complete(value) => Vs3TaskStatus::Complete(value),
    }
}

/// Compiled VS3 logic unit (callable functions).
#[derive(Debug, Clone)]
pub struct Vs3Module {
    /// Source edition (always Vs3 after successful compile).
    pub edition: Edition,
    /// Underlying bytecode module.
    pub bytecode: velvet_script_bytecode::BytecodeModule,
    /// Soft diagnostics (warnings).
    pub diagnostics: Vec<Vs3Diagnostic>,
    /// Source file name if known.
    pub file: Option<String>,
    signatures: BTreeMap<String, Vec<Option<Vs3Type>>>,
}

/// Initialized, persistent VS3 runtime session.
///
/// A session runs the module initializer once, then preserves `state` globals
/// across calls. Create separate sessions for isolated games, tools, or tests.
#[derive(Debug)]
pub struct Vs3Session {
    vm: Vm,
    signatures: BTreeMap<String, Vec<Option<Vs3Type>>>,
}

/// A named collection of independently compiled VS3 modules.
#[derive(Debug, Clone, Default)]
pub struct Vs3Package {
    modules: BTreeMap<String, Vs3Module>,
}

/// Persistent sessions for every module in a [`Vs3Package`].
#[derive(Debug, Default)]
pub struct Vs3PackageSession {
    modules: BTreeMap<String, Vs3Session>,
}

impl Vs3Package {
    /// Create an empty package.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile named source modules into a package.
    pub fn compile_modules(
        modules: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, Vs3Error> {
        let mut package = Self::new();
        for (name, source) in modules {
            let module = compile(&source, Some(&name))?;
            package.insert(name, module)?;
        }
        Ok(package)
    }

    /// Insert a compiled module under a stable package name.
    pub fn insert(&mut self, name: impl Into<String>, module: Vs3Module) -> Result<(), Vs3Error> {
        let name = name.into();
        if !valid_module_name(&name) {
            return Err(Vs3Error::Task(format!("invalid module name `{name}`")));
        }
        if self.modules.insert(name.clone(), module).is_some() {
            return Err(Vs3Error::Task(format!("duplicate module `{name}`")));
        }
        Ok(())
    }

    /// Borrow a named module.
    pub fn module(&self, name: &str) -> Option<&Vs3Module> {
        self.modules.get(name)
    }

    /// Sorted module names.
    pub fn module_names(&self) -> Vec<&str> {
        self.modules.keys().map(String::as_str).collect()
    }

    /// Call `module::function` in a fresh isolated module session.
    pub fn call(&self, qualified: &str, args: &[Value]) -> Result<Value, Vs3Error> {
        let (module, function) = split_qualified(qualified)?;
        self.modules
            .get(module)
            .ok_or_else(|| Vs3Error::Task(format!("unknown module `{module}`")))?
            .call(function, args)
    }

    /// Initialize one persistent session for every package module.
    pub fn session(&self) -> Result<Vs3PackageSession, Vs3Error> {
        let mut modules = BTreeMap::new();
        for (name, module) in &self.modules {
            modules.insert(name.clone(), module.session()?);
        }
        Ok(Vs3PackageSession { modules })
    }
}

impl Vs3PackageSession {
    /// Call `module::function` while preserving that module's state.
    pub fn call(&mut self, qualified: &str, args: &[Value]) -> Result<Value, Vs3Error> {
        let (module, function) = split_qualified(qualified)?;
        self.modules
            .get_mut(module)
            .ok_or_else(|| Vs3Error::Task(format!("unknown module `{module}`")))?
            .call(function, args)
    }
}

fn valid_module_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn split_qualified(qualified: &str) -> Result<(&str, &str), Vs3Error> {
    let Some((module, function)) = qualified.split_once("::") else {
        return Err(Vs3Error::Task(format!(
            "expected qualified function `module::function`, got `{qualified}`"
        )));
    };
    if !valid_module_name(module) || function.is_empty() || function.contains("::") {
        return Err(Vs3Error::Task(format!(
            "invalid qualified function `{qualified}`"
        )));
    }
    Ok((module, function))
}

impl Vs3Session {
    /// Call an exported function while preserving module state.
    pub fn call(&mut self, name: &str, args: &[Value]) -> Result<Value, Vs3Error> {
        validate_runtime_args(&self.signatures, name, args)?;
        self.vm.call_name(name, args).map_err(map_vm_err)
    }

    /// Drain lines captured by the `print` native.
    pub fn take_printed(&mut self) -> Vec<String> {
        self.vm.take_printed()
    }

    /// Total instructions executed by this session.
    pub fn instructions(&self) -> u64 {
        self.vm.total_instructions()
    }
}

impl Vs3Module {
    /// Exported user function names (excludes the synthetic `<script>` entry).
    pub fn function_names(&self) -> Vec<String> {
        self.bytecode
            .exports
            .keys()
            .filter(|n| n.as_str() != "<script>")
            .cloned()
            .collect()
    }

    /// Count of callable user functions.
    pub fn user_function_count(&self) -> usize {
        self.function_names().len()
    }

    /// Parameter annotations for an exported function.
    pub fn function_signature(&self, name: &str) -> Option<&[Option<Vs3Type>]> {
        self.signatures.get(name).map(Vec::as_slice)
    }

    /// Call a pure logic function by name with arguments.
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Value, Vs3Error> {
        self.call_with_limits(name, args, VmLimits::default())
    }

    /// Call once with explicit runtime limits.
    ///
    /// This creates an isolated session, initializes module state, performs the
    /// call, and then discards that state.
    pub fn call_with_limits(
        &self,
        name: &str,
        args: &[Value],
        limits: VmLimits,
    ) -> Result<Value, Vs3Error> {
        self.session_with_limits(limits)?.call(name, args)
    }

    /// Create an initialized persistent session with default sandbox limits.
    pub fn session(&self) -> Result<Vs3Session, Vs3Error> {
        self.session_with_limits(VmLimits::default())
    }

    /// Create an initialized persistent session with explicit limits.
    pub fn session_with_limits(&self, limits: VmLimits) -> Result<Vs3Session, Vs3Error> {
        let mut vm = Vm::new(self.bytecode.clone(), limits);
        vm.initialize().map_err(map_vm_err)?;
        Ok(Vs3Session {
            vm,
            signatures: self.signatures.clone(),
        })
    }

    /// Start a cooperative function invocation that may use `yield(value)`.
    pub fn start(&self, name: &str, args: &[Value]) -> Result<Vs3Task, Vs3Error> {
        self.start_with_limits(name, args, VmLimits::default())
    }

    /// Start a cooperative invocation with explicit runtime limits.
    pub fn start_with_limits(
        &self,
        name: &str,
        args: &[Value],
        limits: VmLimits,
    ) -> Result<Vs3Task, Vs3Error> {
        validate_runtime_args(&self.signatures, name, args)?;
        let coroutine = Coroutine::from_function(self.bytecode.clone(), name, args, limits)
            .map_err(map_vm_err)?;
        Ok(Vs3Task {
            coroutine,
            pending: None,
        })
    }

    /// Call with the legacy presentation compatibility bridge.
    ///
    /// Returns the function value plus the final [`PresentHostState`] (state only —
    /// no drawing). Hosts apply this to `PresentationState` / GPU presenters.
    pub fn call_with_present(
        &self,
        name: &str,
        args: &[Value],
    ) -> Result<(Value, velvet_script_vm::PresentHostState), Vs3Error> {
        let limits = VmLimits {
            sandbox: false,
            ..VmLimits::default()
        };
        let (result, state) =
            velvet_script_vm::with_present_host(|| self.call_with_limits(name, args, limits));
        let value = result?;
        Ok((value, state))
    }
}

/// Re-export legacy presentation state for compatibility adapters.
pub use velvet_script_vm::{PresentHostState, PresentSprite};

fn validate_runtime_args(
    signatures: &BTreeMap<String, Vec<Option<Vs3Type>>>,
    name: &str,
    args: &[Value],
) -> Result<(), Vs3Error> {
    let Some(signature) = signatures.get(name) else {
        return Ok(());
    };
    if signature.len() != args.len() {
        return Err(Vs3Error::Runtime {
            message: format!(
                "function `{name}` expects {} arguments, got {}",
                signature.len(),
                args.len()
            ),
            loc: "<call>".into(),
            stack_trace: vec![],
        });
    }
    for (index, (expected, value)) in signature.iter().zip(args).enumerate() {
        if expected.is_some_and(|expected| !expected.accepts_value(value)) {
            return Err(Vs3Error::Runtime {
                message: format!(
                    "argument {} of `{name}` expects `{}`, got `{}`",
                    index + 1,
                    expected.unwrap().as_str(),
                    value.type_name()
                ),
                loc: "<call>".into(),
                stack_trace: vec![],
            });
        }
    }
    Ok(())
}

fn map_vm_err(e: VmError) -> Vs3Error {
    match e {
        VmError::Runtime {
            message,
            location,
            stack_trace,
        } => Vs3Error::Runtime {
            message,
            loc: location
                .as_ref()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "<runtime>".into()),
            stack_trace,
        },
        other => Vs3Error::Runtime {
            message: other.to_string(),
            loc: "<runtime>".into(),
            stack_trace: vec![],
        },
    }
}

fn loc_at(file: Option<&str>, line: u32, column: u32) -> SourceLoc {
    SourceLoc {
        file: file.map(|s| s.to_string()),
        line,
        column,
        span: Span::default(),
    }
}

fn map_compile_err(e: CompileError, file: Option<&str>) -> Vs3Error {
    let mut diags = Vec::new();
    match e {
        CompileError::Parse(msg) => {
            // Try to pull line from "at L:C" patterns; else line 1
            let loc = parse_loc_from_message(&msg, file);
            diags.push(Vs3Diagnostic { message: msg, loc });
        }
        CompileError::Codegen { message, loc } => {
            diags.push(Vs3Diagnostic {
                message,
                loc: loc_at(file, parse_line_from_loc(&loc), parse_col_from_loc(&loc)),
            });
        }
        CompileError::Many {
            diagnostics, first, ..
        } => {
            if diagnostics.is_empty() {
                diags.push(Vs3Diagnostic {
                    message: first,
                    loc: loc_at(file, 1, 1),
                });
            } else {
                for d in diagnostics {
                    diags.push(ast_diag_to_vs3(&d));
                }
            }
        }
    }
    Vs3Error::Compile(diags)
}

fn ast_diag_to_vs3(d: &Diagnostic) -> Vs3Diagnostic {
    Vs3Diagnostic {
        message: d.message.clone(),
        loc: d.loc.clone(),
    }
}

fn parse_loc_from_message(msg: &str, file: Option<&str>) -> SourceLoc {
    // e.g. "unexpected input at 3:5: ..."
    if let Some(idx) = msg.find(" at ") {
        let rest = &msg[idx + 4..];
        let nums: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ':')
            .collect();
        let mut parts = nums.split(':');
        let line = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        let column = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
        return loc_at(file, line, column);
    }
    loc_at(file, 1, 1)
}

fn parse_line_from_loc(loc: &str) -> u32 {
    // "file:12:3" or "12:3"
    let parts: Vec<&str> = loc.rsplit(':').take(3).collect();
    if parts.len() >= 2 {
        if let Ok(l) = parts[1].parse() {
            return l;
        }
        if let Ok(l) = parts[0].parse() {
            return l;
        }
    }
    1
}

fn parse_col_from_loc(loc: &str) -> u32 {
    loc.rsplit(':')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

/// Compile VS3 source. **Requires** `// @edition 3`.
pub fn compile(source: &str, file: Option<&str>) -> Result<Vs3Module, Vs3Error> {
    let edition = detect_edition(source);
    match edition {
        Edition::Vs3 => {}
        Edition::Classic => {
            return Err(Vs3Error::Edition(
                "VS3 requires `// @edition 3` at the top of the file (classic product .vel uses StoryProgram instead)"
                    .into(),
            ));
        }
        Edition::Vs2 => {
            return Err(Vs3Error::Edition(
                "VS2 is not the official language line; use `// @edition 3` (see docs/language/VELVET_SCRIPT_3.md)"
                    .into(),
            ));
        }
    }

    let parsed = parse_file(source, file)
        .map_err(|error| map_compile_err(CompileError::Parse(error.to_string()), file))?;
    let mut semantic = semantic::validate(&parsed.module);
    let mut frontend_diagnostics: Vec<Vs3Diagnostic> = parsed
        .module
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(ast_diag_to_vs3)
        .collect();
    frontend_diagnostics.append(&mut semantic.diagnostics);
    if !frontend_diagnostics.is_empty() {
        return Err(Vs3Error::Compile(frontend_diagnostics));
    }
    let mut compiled: CompileResult =
        compile_ast(&parsed.module).map_err(|e| map_compile_err(e, file))?;
    compiled.module.metadata.source_hash = Some(fnv1a64(source.as_bytes()));
    if let Some(file) = file {
        compiled.module.metadata.source_path = Some(file.to_string());
        compiled.module.file = Some(file.to_string());
    }

    // Surface hard diagnostics as failure
    let mut diags: Vec<Vs3Diagnostic> = compiled
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(ast_diag_to_vs3)
        .collect();

    // Reject empty logic unit (only synthetic <script>, no user fns)
    let user_fns = compiled
        .module
        .exports
        .keys()
        .filter(|n| n.as_str() != "<script>")
        .count();
    if user_fns == 0 {
        diags.push(Vs3Diagnostic {
            message: "VS3 logic unit has no callable functions".into(),
            loc: loc_at(file, 1, 1),
        });
    }

    // Disallow pure story-only units as the VS3 surface (character/scene without fn)
    // Soft: if there are functions we allow co-located story items for interop later.
    if !diags.is_empty() {
        return Err(Vs3Error::Compile(diags));
    }

    let soft: Vec<Vs3Diagnostic> = compiled
        .diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Error)
        .map(ast_diag_to_vs3)
        .collect();

    Ok(Vs3Module {
        edition: Edition::Vs3,
        bytecode: compiled.module,
        diagnostics: soft,
        file: file.map(|s| s.to_string()),
        signatures: semantic.signatures,
    })
}

/// Compile and call in one step (library entry for tests / hosts).
pub fn eval_call(
    source: &str,
    file: Option<&str>,
    name: &str,
    args: &[Value],
) -> Result<Value, Vs3Error> {
    let module = compile(source, file)?;
    module.call(name, args)
}

/// List top-level function names from source without running (for tooling).
pub fn list_functions(source: &str, file: Option<&str>) -> Result<Vec<String>, Vs3Error> {
    let m = compile(source, file)?;
    Ok(m.function_names())
}

/// True if source is marked edition 3.
pub fn is_vs3_source(source: &str) -> bool {
    detect_edition(source) == Edition::Vs3
}

// ── Convenience constructors for tests / CLI ───────────────────────────────

/// Integer argument.
pub fn int(v: i64) -> Value {
    Value::Int(v)
}

/// Bool argument.
pub fn bool_val(v: bool) -> Value {
    Value::Bool(v)
}

/// String argument.
pub fn string_val(s: impl Into<String>) -> Value {
    Value::String(std::rc::Rc::from(s.into()))
}

/// Float argument.
pub fn float_val(v: f64) -> Value {
    Value::Float(v)
}

/// Construct a mutable VS3 list value.
pub fn list_val(items: Vec<Value>) -> Value {
    Value::list(items)
}

/// Construct a mutable VS3 map value for hosts and runtime libraries.
pub fn map_val(entries: impl IntoIterator<Item = (String, Value)>) -> Value {
    Value::map(entries)
}

/// Construct an immutable VS3 `vec2` for host calls.
pub fn vec2_val(x: f64, y: f64) -> Value {
    Value::Vec2([x, y])
}

/// Construct an immutable VS3 `vec3` for host calls.
pub fn vec3_val(x: f64, y: f64, z: f64) -> Value {
    Value::Vec3([x, y, z])
}

/// Construct an immutable VS3 `vec4` for host calls.
pub fn vec4_val(x: f64, y: f64, z: f64, w: f64) -> Value {
    Value::Vec4([x, y, z, w])
}

/// Construct an immutable column-major VS3 `mat3` for host calls.
pub fn mat3_val(columns: [f64; 9]) -> Value {
    Value::Mat3(columns)
}

/// Construct an immutable column-major VS3 `mat4` for host calls.
pub fn mat4_val(columns: [f64; 16]) -> Value {
    Value::Mat4(columns)
}

/// Construct an immutable VS3 quaternion `(x, y, z, w)` for host calls.
pub fn quat_val(x: f64, y: f64, z: f64, w: f64) -> Value {
    Value::Quat([x, y, z, w])
}

/// Failure converting a precision-preserving VS3 value to engine `f32` math.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum Vs3MathBridgeError {
    /// Runtime value has the wrong mathematical type.
    #[error("expected `{expected}`, found `{found}`")]
    Type {
        /// Required VS3 type.
        expected: &'static str,
        /// Actual VS3 type.
        found: &'static str,
    },
    /// A component cannot be represented as finite `f32`.
    #[error("{kind} component {index} is outside finite f32 range: {value}")]
    Range {
        /// Mathematical value kind.
        kind: &'static str,
        /// Zero-based component index.
        index: usize,
        /// Rejected value.
        value: f64,
    },
}

/// Promote an engine `f32` vector to a VS3 `f64` vector.
pub fn from_engine_vec2(value: velvet_math::Vec2) -> Value {
    vec2_val(f64::from(value.x), f64::from(value.y))
}

/// Promote an engine `f32` vector to a VS3 `f64` vector.
pub fn from_engine_vec3(value: velvet_math::Vec3) -> Value {
    vec3_val(f64::from(value.x), f64::from(value.y), f64::from(value.z))
}

/// Promote an engine column-major `f32` matrix to VS3 `f64`.
pub fn from_engine_mat3(value: velvet_math::Mat3) -> Value {
    mat3_val([
        value.x_axis[0] as f64,
        value.x_axis[1] as f64,
        value.x_axis[2] as f64,
        value.y_axis[0] as f64,
        value.y_axis[1] as f64,
        value.y_axis[2] as f64,
        value.z_axis[0] as f64,
        value.z_axis[1] as f64,
        value.z_axis[2] as f64,
    ])
}

/// Promote an engine column-major `f32` matrix to VS3 `f64`.
pub fn from_engine_mat4(value: velvet_math::Mat4) -> Value {
    let mut output = [0.0; 16];
    for (target, source) in output.iter_mut().zip(
        value
            .x_axis
            .into_iter()
            .chain(value.y_axis)
            .chain(value.z_axis)
            .chain(value.w_axis),
    ) {
        *target = f64::from(source);
    }
    mat4_val(output)
}

/// Convert a VS3 `vec2` to finite engine `f32` components.
pub fn to_engine_vec2(value: &Value) -> Result<velvet_math::Vec2, Vs3MathBridgeError> {
    let Value::Vec2(values) = value else {
        return Err(bridge_type("vec2", value));
    };
    let values = to_f32_array("vec2", values)?;
    Ok(velvet_math::Vec2::new(values[0], values[1]))
}

/// Convert a VS3 `vec3` to finite engine `f32` components.
pub fn to_engine_vec3(value: &Value) -> Result<velvet_math::Vec3, Vs3MathBridgeError> {
    let Value::Vec3(values) = value else {
        return Err(bridge_type("vec3", value));
    };
    let values = to_f32_array("vec3", values)?;
    Ok(velvet_math::Vec3::new(values[0], values[1], values[2]))
}

/// Convert a VS3 column-major `mat3` to finite engine `f32` components.
pub fn to_engine_mat3(value: &Value) -> Result<velvet_math::Mat3, Vs3MathBridgeError> {
    let Value::Mat3(values) = value else {
        return Err(bridge_type("mat3", value));
    };
    let values = to_f32_array("mat3", values)?;
    Ok(velvet_math::Mat3::from_cols(
        values[0..3].try_into().unwrap(),
        values[3..6].try_into().unwrap(),
        values[6..9].try_into().unwrap(),
    ))
}

/// Convert a VS3 column-major `mat4` to finite engine `f32` components.
pub fn to_engine_mat4(value: &Value) -> Result<velvet_math::Mat4, Vs3MathBridgeError> {
    let Value::Mat4(values) = value else {
        return Err(bridge_type("mat4", value));
    };
    Ok(velvet_math::Mat4::from_cols_array(to_f32_array(
        "mat4", values,
    )?))
}

fn bridge_type(expected: &'static str, value: &Value) -> Vs3MathBridgeError {
    Vs3MathBridgeError::Type {
        expected,
        found: value.type_name(),
    }
}

fn to_f32_array<const N: usize>(
    kind: &'static str,
    values: &[f64; N],
) -> Result<[f32; N], Vs3MathBridgeError> {
    let mut output = [0.0; N];
    for (index, (target, value)) in output.iter_mut().zip(values).enumerate() {
        if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
            return Err(Vs3MathBridgeError::Range {
                kind,
                index,
                value: *value,
            });
        }
        *target = *value as f32;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
// @edition 3
// VS3 logic sample — pure game rules

function can_play_card(hand_size, cost, energy) {
    return hand_size > 0 && energy >= cost
}

function apply_damage(hp, dmg) {
    if dmg >= hp {
        return 0
    }
    return hp - dmg
}

function clamp01(x) {
    if x < 0 {
        return 0
    }
    if x > 1 {
        return 1
    }
    return x
}
"#;

    #[test]
    fn detect_edition_3() {
        assert_eq!(
            detect_edition("// @edition 3\nfunction f() { return 1 }\n"),
            Edition::Vs3
        );
        assert_eq!(
            detect_edition("// @edition 2\nfunction f() { return 1 }\n"),
            Edition::Vs2
        );
        assert_eq!(
            detect_edition("function f() { return 1 }\n"),
            Edition::Classic
        );
    }

    #[test]
    fn classic_without_edition_rejected_by_vs3_api() {
        let err = compile("function f() { return 1 }\n", Some("c.vel")).unwrap_err();
        assert!(matches!(err, Vs3Error::Edition(_)));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn vs2_edition_rejected() {
        let err = compile("// @edition 2\nfunction f() { return 1 }\n", Some("x.vel")).unwrap_err();
        assert!(matches!(err, Vs3Error::Edition(_)));
        assert!(err.to_string().contains("edition 3") || err.to_string().contains("VS2"));
    }

    #[test]
    fn compile_edition_3_ok() {
        let m = compile(SAMPLE, Some("logic.vel")).unwrap();
        assert_eq!(m.edition, Edition::Vs3);
        let names = m.function_names();
        assert!(names.iter().any(|n| n == "can_play_card"));
        assert!(names.iter().any(|n| n == "apply_damage"));
    }

    #[test]
    fn semantic_frontend_rejects_unknown_names_and_const_assignment() {
        let unknown = compile(
            "// @edition 3\nfunction f() { return missing + 1 }\n",
            Some("unknown.vel"),
        )
        .unwrap_err();
        assert!(unknown.to_string().contains("unknown name `missing`"));

        let immutable = compile(
            "// @edition 3\nfunction f() { const answer = 42; answer = 0; return answer }\n",
            Some("const.vel"),
        )
        .unwrap_err();
        assert!(immutable
            .to_string()
            .contains("cannot assign to immutable `answer`"));
    }

    #[test]
    fn type_annotations_are_checked_statically_and_at_host_boundary() {
        let mismatch = compile(
            "// @edition 3\nfunction f() { let count: int = \"many\"; return count }\n",
            Some("types.vel"),
        )
        .unwrap_err();
        assert!(mismatch.to_string().contains("type mismatch"));

        let module = compile(
            "// @edition 3\nfunction double(value: int) { return value * 2 }\n",
            Some("typed.vel"),
        )
        .unwrap();
        assert_eq!(
            module.function_signature("double"),
            Some([Some(Vs3Type::Int)].as_slice())
        );
        let error = module.call("double", &[string_val("wrong")]).unwrap_err();
        assert!(error.to_string().contains("expects `int`"), "{error}");
    }

    #[test]
    fn dynamic_collection_values_work_in_typed_math_calls() {
        let module = compile(
            r#"// @edition 3
function move(pos: vec2, delta: vec2) { return pos + delta }
function from_map(entity: map) {
    let next = entity.pos + entity.velocity * 0.5
    return move(next, entity.velocity)
}
function bounded_index(data: map) { return clamp(data.index, 0, 5) }
"#,
            Some("dynamic-math.vel"),
        )
        .unwrap();
        assert!(module
            .function_names()
            .iter()
            .any(|name| name == "from_map"));
    }

    #[test]
    fn integer_min_max_and_clamp_preserve_static_integer_type() {
        compile(
            "// @edition 3\nfunction bounded(value: int) { let out: int = clamp(value, 0, 5); return max(1, out) }\n",
            Some("integer-math.vel"),
        )
        .unwrap();
    }

    #[test]
    fn vs3_rejects_narrative_surface_even_beside_functions() {
        let error = compile(
            "// @edition 3\nfunction f() { return 1 }\nscene intro { \"hello\" }\n",
            Some("mixed.vel"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("not part of VS3"), "{error}");
    }

    #[test]
    fn source_bundle_imports_share_functions_and_state() {
        let module = compile_bundle(
            "game.vel",
            [
                (
                    "game.vel",
                    "// @edition 3\nimport \"state.vel\"\nimport \"logic/combat.vel\"\nfunction score() { return hits }\n",
                ),
                ("state.vel", "state { hits: int = 0 }\n"),
                (
                    "logic/combat.vel",
                    "import \"../shared.vel\"\nfunction attack(value: int) { hits += bounded(value); return hits }\n",
                ),
                (
                    "shared.vel",
                    "function bounded(value: int) { return clamp(value, 0, 10) }\n",
                ),
            ],
        )
        .unwrap();
        let mut session = module.session().unwrap();
        assert_eq!(session.call("attack", &[int(4)]).unwrap(), int(4));
        assert_eq!(session.call("attack", &[int(20)]).unwrap(), int(14));
        assert_eq!(session.call("score", &[]).unwrap(), int(14));
    }

    #[test]
    fn source_bundle_rejects_cycles_and_missing_sources() {
        let cycle = compile_bundle(
            "game.vel",
            [
                (
                    "game.vel",
                    "// @edition 3\nimport \"a.vel\"\nfunction main() { return 1 }\n",
                ),
                ("a.vel", "import \"game.vel\"\n"),
            ],
        )
        .unwrap_err();
        assert!(cycle.to_string().contains("cyclic import"), "{cycle}");

        let missing = compile_bundle(
            "game.vel",
            [(
                "game.vel",
                "// @edition 3\nimport \"missing.vel\"\nfunction main() { return 1 }\n",
            )],
        )
        .unwrap_err();
        assert!(missing.to_string().contains("missing"), "{missing}");
    }

    // ── Phase 2: pure logic execution ───────────────────────────────────

    #[test]
    fn can_play_card_logic_returns_correct_bools() {
        // energy enough
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "can_play_card",
            &[int(5), int(3), int(3)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(true));
        // energy short
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "can_play_card",
            &[int(5), int(3), int(2)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(false));
        // empty hand
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "can_play_card",
            &[int(0), int(1), int(10)],
        )
        .unwrap();
        assert_eq!(v, Value::Bool(false));
    }

    #[test]
    fn apply_damage_clamps_to_zero() {
        let v = eval_call(
            SAMPLE,
            Some("logic.vel"),
            "apply_damage",
            &[int(10), int(3)],
        )
        .unwrap();
        assert_eq!(v, Value::Int(7));
        let v = eval_call(SAMPLE, Some("logic.vel"), "apply_damage", &[int(5), int(5)]).unwrap();
        assert_eq!(v, Value::Int(0));
        let v = eval_call(SAMPLE, Some("logic.vel"), "apply_damage", &[int(5), int(9)]).unwrap();
        assert_eq!(v, Value::Int(0));
    }

    #[test]
    fn clamp01_control_flow() {
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(-2)]).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(0)]).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(1)]).unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            eval_call(SAMPLE, Some("logic.vel"), "clamp01", &[int(5)]).unwrap(),
            Value::Int(1)
        );
    }

    // ── Phase 3: host tool natives ──────────────────────────────────────

    const NATIVES: &str = r#"
// @edition 3

function half_turn_sin() {
    // sin(pi/2) ≈ 1
    return sin(1.5707963267948966)
}

function empty_sha() {
    return hash_sha256("")
}

function add_then_abs(a, b) {
    return abs(a + b)
}
"#;

    #[test]
    fn native_sin_matches_rust() {
        let v = eval_call(NATIVES, Some("nat.vel"), "half_turn_sin", &[]).unwrap();
        match v {
            Value::Float(f) => {
                let expected = std::f64::consts::FRAC_PI_2.sin();
                assert!(
                    (f - expected).abs() < 1e-9,
                    "sin native {f} vs rust {expected}"
                );
            }
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn native_hash_sha256_empty_matches_crypto_tool() {
        let v = eval_call(NATIVES, Some("nat.vel"), "empty_sha", &[]).unwrap();
        let expected = velvet_crypto::hash_sha256_hex(b"").unwrap();
        match v {
            Value::String(s) => assert_eq!(&*s, expected.as_str()),
            other => panic!("expected string hex, got {other:?}"),
        }
    }

    #[test]
    fn native_abs_on_sum() {
        let v = eval_call(
            NATIVES,
            Some("nat.vel"),
            "add_then_abs",
            &[int(-3), int(-4)],
        )
        .unwrap();
        assert_eq!(v, Value::Int(7));
    }

    // ── Phase 4: structured diagnostics ─────────────────────────────────

    #[test]
    fn parse_error_has_location() {
        let src = "// @edition 3\nfunction bad( {\n  return 1\n}\n";
        let err = compile(src, Some("bad.vel")).unwrap_err();
        match &err {
            Vs3Error::Compile(diags) => {
                assert!(!diags.is_empty());
                assert!(
                    diags.iter().any(|d| d.loc.line > 0),
                    "expected line > 0 in diags: {diags:?}"
                );
                assert!(diags.iter().any(|d| !d.message.is_empty()));
            }
            other => panic!("expected Compile diags, got {other:?}"),
        }
        assert!(err.has_located_diagnostic());
        assert!(err.to_string().contains("bad.vel"), "error={err}");
    }

    #[test]
    fn empty_functions_is_error_with_location() {
        let src = "// @edition 3\n// no functions\n";
        let err = compile(src, Some("empty.vel")).unwrap_err();
        let diags = err.diagnostics();
        assert!(!diags.is_empty());
        assert!(diags[0].loc.line >= 1);
        assert!(diags[0].message.contains("no callable"));
    }

    #[test]
    fn unknown_function_runtime_fails() {
        let m = compile(SAMPLE, Some("logic.vel")).unwrap();
        let err = m.call("does_not_exist", &[]).unwrap_err();
        assert!(matches!(err, Vs3Error::Runtime { .. }));
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn integer_overflow_is_a_runtime_error_not_a_panic() {
        let source = r#"
// @edition 3
function multiply(a, b) { return a * b }
function negate(a) { return -a }
function literal() { return 9223372036854775807 + 1 }
"#;
        let module = compile(source, Some("overflow.vel")).unwrap();
        let error = module
            .call("multiply", &[int(i64::MAX), int(2)])
            .unwrap_err();
        assert!(error.to_string().contains("integer overflow"), "{error}");
        let error = module.call("negate", &[int(i64::MIN)]).unwrap_err();
        assert!(error.to_string().contains("integer overflow"), "{error}");
        let error = module.call("literal", &[]).unwrap_err();
        assert!(error.to_string().contains("integer overflow"), "{error}");
    }

    #[test]
    fn one_shot_call_rejects_yield_and_points_to_task_api() {
        let module = compile(TASKS, Some("tasks.vel")).unwrap();
        let error = module.call("raw_yield", &[int(1)]).unwrap_err();
        assert!(error.to_string().contains("Vs3Module::start"), "{error}");
    }

    #[test]
    fn runtime_errors_keep_script_stack_trace() {
        let source = r#"
// @edition 3
function inner() { fail("broken invariant") }
function outer() { return inner() }
"#;
        let module = compile(source, Some("trace.vel")).unwrap();
        let error = module.call("outer", &[]).unwrap_err().to_string();
        assert!(error.contains("broken invariant"), "{error}");
        assert!(error.contains("in inner"), "{error}");
        assert!(error.contains("in outer"), "{error}");
    }

    #[test]
    fn persistent_session_initializes_and_preserves_state() {
        let source = r#"
// @edition 3
state { counter = 40 }
function bump() {
    counter += 1
    return counter
}
"#;
        let module = compile(source, Some("state.vel")).unwrap();
        let mut session = module.session().unwrap();
        assert_eq!(session.call("bump", &[]).unwrap(), int(41));
        assert_eq!(session.call("bump", &[]).unwrap(), int(42));
        assert_eq!(module.call("bump", &[]).unwrap(), int(41));

        let mut bounded = module
            .session_with_limits(VmLimits {
                max_instructions: 12,
                ..VmLimits::default()
            })
            .unwrap();
        for expected in 41..=45 {
            assert_eq!(bounded.call("bump", &[]).unwrap(), int(expected));
        }
        assert!(bounded.instructions() > 12);
    }

    #[test]
    fn package_qualifies_modules_and_preserves_each_state() {
        let counter = |initial| {
            format!(
                "// @edition 3\nstate {{ value: int = {initial} }}\nfunction next() {{ value += 1; return value }}\n"
            )
        };
        let package = Vs3Package::compile_modules([
            ("game.score".into(), counter(0)),
            ("ui.flow".into(), counter(100)),
        ])
        .unwrap();
        assert_eq!(package.module_names(), vec!["game.score", "ui.flow"]);
        let mut session = package.session().unwrap();
        assert_eq!(session.call("game.score::next", &[]).unwrap(), int(1));
        assert_eq!(session.call("game.score::next", &[]).unwrap(), int(2));
        assert_eq!(session.call("ui.flow::next", &[]).unwrap(), int(101));
        assert!(session.call("missing::next", &[]).is_err());
    }

    // ── Language surface expansion ──────────────────────────────────────

    const SURFACE: &str = r#"
// @edition 3

function arith(a, b) {
    return (a + b) * (a - b) / 2 + a % b
}

function compares(a, b) {
    return a < b && a <= b && !(a > b) && a != b || a == a
}

function with_else(x) {
    if x > 0 {
        return 1
    } else {
        if x < 0 {
            return -1
        } else {
            return 0
        }
    }
}

function sum_while(n) {
    let i = 0
    let s = 0
    while i < n {
        i += 1
        s += i
    }
    return s
}

function join_labels(a, b) {
    return concat(a, b)
}

function label_len(s) {
    return len(s)
}

function float_half(x) {
    return x / 2.0
}

function tool_clamp(x, lo, hi) {
    return clamp(x, lo, hi)
}

function tool_min_max(a, b) {
    return max(min(a, b), 0)
}
"#;

    #[test]
    fn surface_arithmetic_ops() {
        // (7+3)*(7-3)/2 + 7%3 = 10*4/2 + 1 = 20+1 = 21
        let v = eval_call(SURFACE, Some("s.vel"), "arith", &[int(7), int(3)]).unwrap();
        assert_eq!(v, Value::Int(21));
    }

    #[test]
    fn surface_comparisons_and_logic() {
        let v = eval_call(SURFACE, Some("s.vel"), "compares", &[int(2), int(5)]).unwrap();
        assert_eq!(v, Value::Bool(true));
        let v = eval_call(SURFACE, Some("s.vel"), "compares", &[int(9), int(1)]).unwrap();
        // 9<1 false → whole && chain false; then || a==a → true
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn surface_if_else_nested() {
        assert_eq!(
            eval_call(SURFACE, Some("s.vel"), "with_else", &[int(3)]).unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            eval_call(SURFACE, Some("s.vel"), "with_else", &[int(-2)]).unwrap(),
            Value::Int(-1)
        );
        assert_eq!(
            eval_call(SURFACE, Some("s.vel"), "with_else", &[int(0)]).unwrap(),
            Value::Int(0)
        );
    }

    #[test]
    fn surface_while_loop_sum() {
        // 1+2+3+4+5 = 15
        let v = eval_call(SURFACE, Some("s.vel"), "sum_while", &[int(5)]).unwrap();
        assert_eq!(v, Value::Int(15));
        let v = eval_call(SURFACE, Some("s.vel"), "sum_while", &[int(0)]).unwrap();
        assert_eq!(v, Value::Int(0));
        let v = eval_call(SURFACE, Some("s.vel"), "sum_while", &[int(1)]).unwrap();
        assert_eq!(v, Value::Int(1));
    }

    #[test]
    fn map_literals_null_and_compound_assignment_work() {
        let source = r#"
// @edition 3
function profile(multiplier: int) {
    let data: map = {"name": "Ada", "score": 7, "note": null}
    data.score *= multiplier
    data.bonus = 1
    return data
}
"#;
        let module = compile(source, Some("maps.vel")).unwrap();
        let value = module.call("profile", &[int(6)]).unwrap();
        assert_eq!(
            value.get_index(&string_val("name")).unwrap(),
            string_val("Ada")
        );
        assert_eq!(value.get_index(&string_val("score")).unwrap(), int(42));
        assert_eq!(value.get_index(&string_val("bonus")).unwrap(), int(1));
        assert_eq!(value.get_index(&string_val("note")).unwrap(), Value::Null);
    }

    #[test]
    fn break_and_continue_work_in_while_and_for_loops() {
        let source = r#"
// @edition 3
function while_demo() {
    let i = 0
    let total = 0
    while i < 20 {
        i += 1
        if i % 2 != 0 { continue }
        total += i
        if total >= 12 { break }
    }
    return total
}

function for_demo() {
    let total = 0
    for value in [1, 2, 3, 4, 5] {
        if value == 2 { continue }
        if value == 5 { break }
        total += value
    }
    return total
}
"#;
        let module = compile(source, Some("loops.vel")).unwrap();
        assert_eq!(module.call("while_demo", &[]).unwrap(), int(12));
        assert_eq!(module.call("for_demo", &[]).unwrap(), int(8));
    }

    #[test]
    fn surface_string_concat_and_len() {
        let v = eval_call(
            SURFACE,
            Some("s.vel"),
            "join_labels",
            &[string_val("vel"), string_val("vet")],
        )
        .unwrap();
        match v {
            Value::String(s) => assert_eq!(&*s, "velvet"),
            other => panic!("expected string, got {other:?}"),
        }
        let v = eval_call(SURFACE, Some("s.vel"), "label_len", &[string_val("abc")]).unwrap();
        assert_eq!(v, Value::Int(3));
    }

    #[test]
    fn surface_float_div() {
        let v = eval_call(SURFACE, Some("s.vel"), "float_half", &[float_val(8.0)]).unwrap();
        match v {
            Value::Float(f) => assert!((f - 4.0).abs() < 1e-9),
            Value::Int(i) => assert_eq!(i, 4), // if coerced
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn surface_natives_clamp_min_max() {
        assert_eq!(
            eval_call(
                SURFACE,
                Some("s.vel"),
                "tool_clamp",
                &[int(50), int(0), int(10)]
            )
            .unwrap(),
            Value::Int(10)
        );
        assert_eq!(
            eval_call(SURFACE, Some("s.vel"), "tool_min_max", &[int(-3), int(5)]).unwrap(),
            Value::Int(0)
        );
        assert_eq!(
            eval_call(SURFACE, Some("s.vel"), "tool_min_max", &[int(4), int(9)]).unwrap(),
            Value::Int(4)
        );
    }

    #[test]
    fn arity_mismatch_fails_honestly() {
        let m = compile(SAMPLE, Some("logic.vel")).unwrap();
        let err = m.call("apply_damage", &[int(1)]).unwrap_err();
        assert!(matches!(err, Vs3Error::Runtime { .. }));
        assert!(
            err.to_string().contains("arg") || err.to_string().contains("expected"),
            "arity error should mention args: {err}"
        );
    }

    #[test]
    fn supported_surface_table_is_unique_and_covers_runtime_contracts() {
        let unique: std::collections::HashSet<_> = SUPPORTED_SURFACE.iter().copied().collect();
        assert_eq!(
            unique.len(),
            SUPPORTED_SURFACE.len(),
            "duplicate capability entry"
        );
        assert_eq!(
            SUPPORTED_SURFACE.len(),
            23,
            "update this contract with every capability change"
        );
        for required in [
            "edition: // @edition 3",
            "while cond { }",
            "cooperative tasks: yield(value), resume, resume_with",
            "generic host ABI: yield [service, payload] and resume with Value",
            "indexing and mutable list/map values",
            "persistent state through Vs3Session",
            "multi-file packages with module::function calls",
            "source bundles with import \"relative/path.vel\" directives",
            "for value in collection with break and continue",
            "host capability policies and immediate request budgets",
        ] {
            assert!(
                unique.contains(required),
                "missing VS3 capability: {required}"
            );
        }
    }

    // ── Presentation host natives (state only) ──────────────────────────

    // Note: classic reserves `show`/`hide` as story stmts → use present_show/present_hide.
    // Also avoid `//` inside string literals (line-comment lexer).
    const PRESENT: &str = r#"
// @edition 3

function setup_scene() {
    set_bg("bg:station.png")
    present_show("nora", "happy", "left")
    present_show("june", "neutral", "right")
    ui_flag("say_visible", true)
    ui_flag("choice_open", false)
    return ui_flag_get("say_visible")
}

function teardown() {
    present_hide("nora")
    present_hide("june")
    ui_flag("say_visible", false)
    return 1
}
"#;

    #[test]
    fn present_natives_show_set_bg_ui_flags() {
        let m = compile(PRESENT, Some("present.vel")).unwrap();
        let (v, host) = m.call_with_present("setup_scene", &[]).unwrap();
        assert_eq!(v, Value::Bool(true));
        assert_eq!(host.background.as_deref(), Some("bg:station.png"));
        assert_eq!(host.sprites.len(), 2);
        assert_eq!(
            host.sprites
                .get("nora")
                .and_then(|s| s.expression.as_deref()),
            Some("happy")
        );
        assert_eq!(
            host.sprites.get("nora").and_then(|s| s.at.as_deref()),
            Some("left")
        );
        assert!(host.ui_flag("say_visible"));
        assert!(!host.ui_flag("choice_open"));
        assert!(host.log.iter().any(|l| l.starts_with("set_bg")));
        assert!(host.log.iter().any(|l| l.starts_with("show")));
        // No drawing API leaked into host state — only paths and flags.
        assert!(host.background.as_ref().unwrap().contains("station"));
    }

    #[test]
    fn present_hide_clears_sprites() {
        let m = compile(PRESENT, Some("present.vel")).unwrap();
        let (_, host) = m.call_with_present("setup_scene", &[]).unwrap();
        assert_eq!(host.sprites.len(), 2);
        // Continue with installed host for teardown
        velvet_script_vm::install_present_host(host);
        let limits = VmLimits {
            sandbox: false,
            ..VmLimits::default()
        };
        let _ = m.call_with_limits("teardown", &[], limits).unwrap();
        let after = velvet_script_vm::take_present_host();
        assert!(after.sprites.is_empty(), "sprites={:?}", after.sprites);
        assert!(!after.ui_flag("say_visible"));
    }

    #[test]
    fn presentation_natives_require_explicit_permission() {
        let module = compile(PRESENT, Some("present.vel")).unwrap();
        let error = module.call("setup_scene", &[]).unwrap_err();
        assert!(error.to_string().contains("sandbox is enabled"), "{error}");
    }

    // ── Cooperative tasks and generic host ABI ─────────────────────────

    const TASKS: &str = r#"
// @edition 3

function raw_yield(value) {
    let answer = yield(value)
    return answer
}

function service_request(value) {
    return yield(["math.double", value])
}

function delayed_dialogue(text) {
    return yield(["ui.dialogue.open", text])
}
"#;

    #[test]
    fn task_yield_is_an_expression_and_accepts_resume_value() {
        let module = compile(TASKS, Some("tasks.vel")).unwrap();
        let mut task = module.start("raw_yield", &[string_val("request")]).unwrap();
        assert_eq!(
            task.resume().unwrap(),
            Vs3TaskStatus::Yielded(string_val("request"))
        );
        assert_eq!(
            task.resume_with(int(42)).unwrap(),
            Vs3TaskStatus::Complete(int(42))
        );
    }

    struct MathHost;

    impl Vs3Host for MathHost {
        fn call(&mut self, request: &HostRequest) -> HostOutcome {
            match request.service.as_str() {
                "math.double" => match request.payload {
                    Value::Int(value) => HostOutcome::Ready(Value::Int(value * 2)),
                    _ => HostOutcome::Failed("expected integer payload".into()),
                },
                _ => HostOutcome::Failed("unknown service".into()),
            }
        }
    }

    #[test]
    fn generic_host_can_complete_a_service_immediately() {
        let module = compile(TASKS, Some("tasks.vel")).unwrap();
        let mut task = module.start("service_request", &[int(21)]).unwrap();
        let mut host = MathHost;
        assert_eq!(
            task.drive_host(&mut host).unwrap(),
            Vs3TaskStatus::Complete(int(42))
        );
    }

    #[test]
    fn host_policy_denies_ungranted_services_and_accepts_namespaces() {
        let module = compile(TASKS, Some("tasks.vel")).unwrap();
        let mut denied = module.start("service_request", &[int(21)]).unwrap();
        let mut host = MathHost;
        let error = denied
            .drive_host_with_policy(&mut host, &Vs3HostPolicy::deny_all())
            .unwrap_err();
        assert!(matches!(error, Vs3Error::Permission { .. }));

        let mut allowed = module.start("service_request", &[int(21)]).unwrap();
        let policy = Vs3HostPolicy::deny_all().allow("math.*");
        assert_eq!(
            allowed.drive_host_with_policy(&mut host, &policy).unwrap(),
            Vs3TaskStatus::Complete(int(42))
        );
    }

    struct PendingHost;

    impl Vs3Host for PendingHost {
        fn call(&mut self, request: &HostRequest) -> HostOutcome {
            assert_eq!(request.service, "ui.dialogue.open");
            HostOutcome::Pending {
                ticket: "dialogue:1".into(),
            }
        }
    }

    #[test]
    fn pending_host_service_resumes_by_ticket() {
        let module = compile(TASKS, Some("tasks.vel")).unwrap();
        let mut task = module
            .start("delayed_dialogue", &[string_val("Hello")])
            .unwrap();
        let mut host = PendingHost;
        match task.drive_host(&mut host).unwrap() {
            Vs3TaskStatus::Waiting { ticket, request } => {
                assert_eq!(ticket, "dialogue:1");
                assert_eq!(request.payload, string_val("Hello"));
            }
            other => panic!("expected pending host request, got {other:?}"),
        }
        assert!(task.is_waiting_for_host());
        assert!(task.resume_host("wrong", Value::Bool(true)).is_err());
        assert_eq!(
            task.resume_host("dialogue:1", Value::Bool(true)).unwrap(),
            Vs3TaskStatus::Complete(Value::Bool(true))
        );
    }

    #[test]
    fn host_request_and_structured_value_helpers_roundtrip() {
        let payload = map_val([
            ("speaker".into(), string_val("Nora")),
            (
                "choices".into(),
                list_val(vec![string_val("Yes"), string_val("No")]),
            ),
        ]);
        let request = HostRequest::new("ui.choice.open", payload);
        assert_eq!(
            HostRequest::from_value(&request.clone().into_value()),
            Some(request)
        );
    }

    #[test]
    fn advanced_vector_matrix_and_curve_surface_executes() {
        let source = r#"// @edition 3
function vector_score() {
    let a: vec3 = vec3(1, 2, 3)
    let b: vec3 = vec3(4, 5, 6)
    let c: vec3 = a + b * 2
    return dot(c, vec3(1)) + c.y
}

function matrix_roundtrip() {
    let m: mat3 = mat3(2, 0, 0, 0, 4, 0, 0, 0, 1)
    return approx_eq(mat_mul(m, mat_inverse(m)), mat3_identity(), 0.000000001)
}

function curve_midpoint() {
    return quadratic_bezier(vec2(0, 0), vec2(1, 2), vec2(2, 0), 0.5)
}

function constants_work() {
    return approx_eq(sin(PI / 2), 1)
}
"#;
        let module = compile(source, Some("advanced_math.vel")).unwrap();
        assert_eq!(
            module.call("vector_score", &[]).unwrap(),
            Value::Float(48.0)
        );
        assert_eq!(
            module.call("matrix_roundtrip", &[]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            module.call("curve_midpoint", &[]).unwrap(),
            Value::Vec2([1.0, 1.0])
        );
        assert_eq!(
            module.call("constants_work", &[]).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn rng_noise_statistics_and_numerics_are_available_and_repeatable() {
        let source = r#"// @edition 3
state {
    random: rng = rng_new(1234)
}

function next_random() {
    return rng_next_float(random)
}

function analysis() {
    let values = [1, 2, 3, 4]
    return [mean(values), median(values), variance(values), quantile(values, 0.25)]
}

function root() {
    return poly_root_bisection([1, 0, -2], 0, 2)
}

function terrain() {
    return fbm2(1.25, -3.5, 7, 4)
}
"#;
        let module = compile(source, Some("numeric_toolkit.vel")).unwrap();
        let mut first = module.session().unwrap();
        let mut second = module.session().unwrap();
        assert_eq!(
            first.call("next_random", &[]).unwrap(),
            second.call("next_random", &[]).unwrap()
        );
        assert_eq!(
            first.call("next_random", &[]).unwrap(),
            second.call("next_random", &[]).unwrap()
        );

        let analysis = module.call("analysis", &[]).unwrap();
        assert_eq!(
            analysis,
            Value::list(vec![
                Value::Float(2.5),
                Value::Float(2.5),
                Value::Float(1.25),
                Value::Float(1.75),
            ])
        );
        let root = module.call("root", &[]).unwrap().as_f64().unwrap();
        assert!((root - 2.0_f64.sqrt()).abs() < 1e-8);
        assert!(module
            .call("terrain", &[])
            .unwrap()
            .as_f64()
            .unwrap()
            .is_finite());
    }

    #[test]
    fn collection_math_is_metered_by_input_size() {
        let module = compile(
            "// @edition 3\nfunction average(values: list) { return mean(values) }\n",
            Some("metered_math.vel"),
        )
        .unwrap();
        let values = Value::list((0..100).map(Value::Int).collect());
        let error = module
            .call_with_limits(
                "average",
                &[values],
                VmLimits {
                    max_instructions: 20,
                    ..VmLimits::default()
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("instruction limit"), "{error}");
    }

    #[test]
    fn advanced_math_diagnostics_cover_arity_types_and_immutability() {
        let error = compile(
            r#"// @edition 3
function bad() {
    let value: vec2 = vec2(1, 2)
    value.x = 9
    let missing = value.z
    return dot(value)
}
"#,
            Some("bad_math.vel"),
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("immutable"), "{message}");
        assert!(message.contains("no component `z`"), "{message}");
        assert!(message.contains("expects 2 arguments"), "{message}");

        let error = compile(
            "// @edition 3\nfunction bad(seed: string) { return rng_new(seed) }\n",
            Some("bad_rng.vel"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("expected `int`"), "{error}");
    }

    #[test]
    fn engine_math_bridge_is_explicit_checked_and_column_major() {
        let engine_vector = velvet_math::Vec3::new(1.25, -2.5, 3.75);
        let script_vector = from_engine_vec3(engine_vector);
        assert_eq!(script_vector, Value::Vec3([1.25, -2.5, 3.75]));
        assert_eq!(to_engine_vec3(&script_vector).unwrap(), engine_vector);

        let script_matrix = mat3_val([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let engine_matrix = to_engine_mat3(&script_matrix).unwrap();
        assert_eq!(from_engine_mat3(engine_matrix), script_matrix);

        assert!(matches!(
            to_engine_vec2(&Value::Vec3([0.0; 3])),
            Err(Vs3MathBridgeError::Type {
                expected: "vec2",
                found: "vec3"
            })
        ));
        assert!(matches!(
            to_engine_vec2(&Value::Vec2([f64::MAX, 0.0])),
            Err(Vs3MathBridgeError::Range {
                kind: "vec2",
                index: 0,
                ..
            })
        ));
        assert!(matches!(
            to_engine_vec2(&Value::Vec2([f64::NAN, 0.0])),
            Err(Vs3MathBridgeError::Range {
                kind: "vec2",
                index: 0,
                ..
            })
        ));
    }
}
