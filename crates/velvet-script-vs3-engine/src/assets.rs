//! Reference `velvet.assets` host module: async-style resource tracking.
//!
//! `load` answers `Pending` with an opaque ticket; the engine resolves it
//! from Rust once bytes arrive, and the waiting task resumes with the
//! ticket. Queries (`status`, `bytes`) are immediate:
//!
//! ```ignore
//! // @edition 3
//! function load_bg() {
//!     let ticket = yield(["velvet.assets.load", ["bg/city.png"]])
//!     return yield(["velvet.assets.bytes", [ticket]])
//! }
//! ```
//!
//! ```ignore
//! // host side (Rust), after the task reports Waiting:
//! host.resolve(&ticket, 1_048_576);
//! task.resume_host(&ticket, Value::String(ticket.into()));
//! ```

use std::collections::BTreeMap;

use velvet_script_vs3::{
    validate_request, FunctionDescriptor, HostOutcome, HostRequest, ModuleDescriptor,
    ParamDescriptor, Purity, SyncKind, Value, Vs3AbiType, Vs3Host,
};

/// Capability required by the loading service.
pub const ASSETS_CAPABILITY: &str = "assets";

fn assets_function(
    name: &str,
    params: Vec<ParamDescriptor>,
    result: Vs3AbiType,
    sync: SyncKind,
    pure: bool,
    doc: &str,
) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        service: format!("velvet.assets.{name}"),
        params,
        result: result.clone(),
        purity: if pure { Purity::Pure } else { Purity::Impure },
        sync,
        capabilities: if pure {
            Vec::new()
        } else {
            vec![ASSETS_CAPABILITY.into()]
        },
        doc: doc.into(),
        ..FunctionDescriptor::new(result)
    }
}

fn str_param(name: &str) -> ParamDescriptor {
    ParamDescriptor::new(name, Vs3AbiType::Str)
}

/// Descriptor for the `velvet.assets` module.
pub fn assets_module() -> ModuleDescriptor {
    ModuleDescriptor::new(
        "velvet.assets",
        "1.0.0",
        vec![
            assets_function(
                "load",
                vec![str_param("path")],
                Vs3AbiType::Str,
                SyncKind::Deferred,
                false,
                "load(path): Pending with a ticket; resume delivers the ticket",
            ),
            assets_function(
                "status",
                vec![str_param("ticket")],
                Vs3AbiType::Str,
                SyncKind::Immediate,
                true,
                "status(ticket): \"pending\" or \"ready\"",
            ),
            assets_function(
                "bytes",
                vec![str_param("ticket")],
                Vs3AbiType::Int,
                SyncKind::Immediate,
                true,
                "bytes(ticket): size once ready, Failed while pending",
            ),
        ],
    )
    .expect("built-in descriptors are valid")
}

/// Reference host for [`assets_module`].
#[derive(Debug, Default)]
pub struct AssetsHost {
    next: u64,
    pending: BTreeMap<String, String>,
    ready: BTreeMap<String, u64>,
}

impl AssetsHost {
    /// No tracked resources.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a pending ticket resolved with its byte size.
    ///
    /// Returns false when the ticket is unknown or already resolved.
    pub fn resolve(&mut self, ticket: &str, bytes: u64) -> bool {
        if self.pending.remove(ticket).is_some() {
            self.ready.insert(ticket.to_string(), bytes);
            true
        } else {
            false
        }
    }

    /// Ticket state for assertions.
    pub fn is_pending(&self, ticket: &str) -> bool {
        self.pending.contains_key(ticket)
    }
}

impl Vs3Host for AssetsHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        let module = assets_module();
        let Some(function) = module
            .functions
            .values()
            .find(|f| f.service == request.service)
        else {
            return HostOutcome::Failed(format!(
                "service `{}` is not part of velvet.assets",
                request.service
            ));
        };
        if let Err(message) = validate_request(function, request) {
            return HostOutcome::Failed(message);
        }
        let args = match &request.payload {
            Value::List(items) => items.borrow().clone(),
            other => {
                return HostOutcome::Failed(format!("assets payload must be a list, got {other}"));
            }
        };
        let text = |i: usize| args[i].to_string();
        match function.name.as_str() {
            "load" => {
                self.next += 1;
                let ticket = format!("asset-{}", self.next);
                self.pending.insert(ticket.clone(), text(0));
                HostOutcome::Pending { ticket }
            }
            "status" => {
                let ticket = text(0);
                if self.ready.contains_key(&ticket) {
                    HostOutcome::Ready(Value::String("ready".into()))
                } else if self.pending.contains_key(&ticket) {
                    HostOutcome::Ready(Value::String("pending".into()))
                } else {
                    HostOutcome::Failed(format!("unknown asset ticket `{ticket}`"))
                }
            }
            "bytes" => {
                let ticket = text(0);
                match self.ready.get(&ticket) {
                    Some(size) => HostOutcome::Ready(Value::Int(*size as i64)),
                    None if self.pending.contains_key(&ticket) => {
                        HostOutcome::Failed(format!("asset `{ticket}` is not ready yet"))
                    }
                    None => HostOutcome::Failed(format!("unknown asset ticket `{ticket}`")),
                }
            }
            other => HostOutcome::Failed(format!("unknown assets function `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_vs3::conformance;

    fn str_(s: &str) -> Value {
        Value::String(s.into())
    }

    fn call(host: &mut AssetsHost, service: &str, args: Vec<Value>) -> HostOutcome {
        let request = HostRequest::new(service, Value::list(args)).unwrap();
        host.call(&request)
    }

    fn load_ticket(host: &mut AssetsHost, path: &str) -> String {
        match call(host, "velvet.assets.load", vec![str_(path)]) {
            HostOutcome::Pending { ticket } => ticket,
            other => panic!("load must pend, got {other:?}"),
        }
    }

    #[test]
    fn descriptor_registers_deferred_load() {
        let module = assets_module();
        assert_eq!(module.functions.len(), 3);
        assert!(matches!(module.functions["load"].sync, SyncKind::Deferred));
        assert!(matches!(
            module.functions["status"].sync,
            SyncKind::Immediate
        ));
        assert!(module.functions["load"]
            .capabilities
            .contains(&ASSETS_CAPABILITY.to_string()));
    }

    #[test]
    fn load_pends_until_resolved() {
        let mut host = AssetsHost::new();
        let ticket = load_ticket(&mut host, "bg/city.png");
        assert!(host.is_pending(&ticket));
        assert!(matches!(
            call(&mut host, "velvet.assets.status", vec![str_(&ticket)]),
            HostOutcome::Ready(Value::String(s)) if s.as_ref() == "pending"
        ));
        // Bytes while pending fail loudly.
        assert!(matches!(
            call(&mut host, "velvet.assets.bytes", vec![str_(&ticket)]),
            HostOutcome::Failed(_)
        ));
        assert!(host.resolve(&ticket, 1_048_576));
        assert!(!host.resolve(&ticket, 1));
        assert!(matches!(
            call(&mut host, "velvet.assets.status", vec![str_(&ticket)]),
            HostOutcome::Ready(Value::String(s)) if s.as_ref() == "ready"
        ));
        assert!(matches!(
            call(&mut host, "velvet.assets.bytes", vec![str_(&ticket)]),
            HostOutcome::Ready(Value::Int(1_048_576))
        ));
        // Unknown tickets fail, never panic.
        assert!(matches!(
            call(&mut host, "velvet.assets.bytes", vec![str_("asset-999")]),
            HostOutcome::Failed(_)
        ));
    }

    #[test]
    fn full_task_drive_through_pending() {
        let src = r#"
// @edition 3
function load_bg() {
    let ticket = yield(["velvet.assets.load", ["bg/city.png"]])
    return yield(["velvet.assets.bytes", [ticket]])
}
"#;
        let module = velvet_script_vs3::compile(src, Some("assets.vel")).unwrap();
        let mut task = module.start("load_bg", &[]).unwrap();
        let mut host = AssetsHost::new();
        let policy = velvet_script_vs3::Vs3HostPolicy::deny_all()
            .allow("velvet.assets.*")
            .unwrap();
        match task.drive_host_with_policy(&mut host, &policy).unwrap() {
            velvet_script_vs3::Vs3TaskStatus::Waiting { ticket, .. } => {
                use velvet_script_vs3::{HostOutcome, HostRequest};
                assert!(host.is_pending(&ticket));
                assert!(host.resolve(&ticket, 64));
                let resume = Value::String(ticket.clone().into());
                // Resume lands on the second yield; drive that leg manually
                // (parse → host → resume_with), like a game loop would.
                let yielded = match task.resume_host(&ticket, resume).unwrap() {
                    velvet_script_vs3::Vs3TaskStatus::Yielded(value) => value,
                    other => panic!("expected Yielded after resume, got {other:?}"),
                };
                let request = HostRequest::from_value(&yielded).expect("canonical request");
                let response = match host.call(&request) {
                    HostOutcome::Ready(value) => value,
                    other => panic!("expected Ready bytes, got {other:?}"),
                };
                match task.resume_with(response).unwrap() {
                    velvet_script_vs3::Vs3TaskStatus::Complete(Value::Int(64)) => {}
                    other => panic!("expected Complete(64), got {other:?}"),
                }
            }
            other => panic!("expected Waiting, got {other:?}"),
        }
    }

    #[test]
    fn assets_host_conforms_to_host_contract() {
        let mut host = AssetsHost::new();
        conformance::expect_failed(&mut host, "velvet.assets.nope", vec![]);
        conformance::expect_failed(&mut host, "velvet.assets.bytes", vec![str_("asset-1")]);
        let ticket = load_ticket(&mut host, "bg/city.png");
        conformance::expect_ready(
            &mut host,
            "velvet.assets.status",
            vec![str_(&ticket)],
            Value::String("pending".into()),
        );
    }
}
