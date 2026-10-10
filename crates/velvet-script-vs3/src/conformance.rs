//! Official conformance suite for VS3 hosts and registries.
//!
//! The suite checks the contracts a host must honor without running
//! VelvetEngine: registry invariants (descriptors, versions, validation),
//! host behavior through scripted [`SimulatedHost`] doubles, and full
//! task drives (`yield` → host → resume) under capability policies.
//! Real hosts (time, input, compat, engine twins) reuse the shared
//! `expect_*` helpers in their own tests so every host proves the same
//! core behavior:
//!
//! ```ignore
//! use velvet_script_vs3::conformance;
//!
//! #[test]
//! fn my_host_conforms() {
//!     let mut host = MyHost::new();
//!     conformance::expect_failed(&mut host, "my.svc", vec![]);
//!     conformance::expect_ready(&mut host, "my.svc", vec![], Value::Null);
//! }
//! ```

use crate::{
    AbiRegistry, FunctionDescriptor, HostOutcome, HostRequest, ModuleDescriptor, SimulatedHost,
    Value, Vs3AbiType, Vs3Host, Vs3HostPolicy, Vs3TaskStatus,
};

/// One named conformance check.
pub struct ConformanceCase {
    /// Stable case name (for reports).
    pub name: &'static str,
    /// Check body: `Ok(())` when the contract holds.
    ///
    /// Drive cases build their own scripted [`SimulatedHost`] internally:
    /// they prove the task/host/policy protocol, not any particular host.
    /// Real hosts prove behavior with the [`expect_failed`]/[`expect_ready`]
    /// helpers in their own test suites.
    pub run: fn(&AbiRegistry) -> Result<(), String>,
}

/// Aggregate conformance result.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ConformanceReport {
    /// Cases passed.
    pub passed: usize,
    /// Cases failed.
    pub failed: usize,
    /// `"<case>: <reason>"` lines, in run order.
    pub failures: Vec<String>,
}

impl ConformanceReport {
    /// True when every case passed.
    pub fn ok(&self) -> bool {
        self.failed == 0
    }
}

/// Registry + drive cases every host setup must satisfy.
///
/// The drive cases script a [`SimulatedHost`] (no engine involved) and run
/// real compiled tasks through it, proving the `yield` protocol,
/// capability policies, request budgets and pending tickets end to end.
pub fn conformance_cases() -> Vec<ConformanceCase> {
    vec![
        ConformanceCase {
            name: "registry/rejects-duplicate-module",
            run: |registry| {
                let first = registry.module_names();
                if first.is_empty() {
                    return Err("suite needs a registry with >= 1 module".into());
                }
                let mut copy = clone_registry(registry)?;
                let rebuilt = rebuild_module(registry, first[0])?;
                match copy.register(rebuilt) {
                    Ok(()) => Err("duplicate module registration accepted".into()),
                    Err(_) => Ok(()),
                }
            },
        },
        ConformanceCase {
            name: "registry/rejects-duplicate-service",
            run: |registry| {
                let services = registry_services(registry);
                if services.is_empty() {
                    return Err("suite needs a registry with >= 1 service".into());
                }
                let mut copy = clone_registry(registry)?;
                let thief = ModuleDescriptor::new(
                    "conform.thief",
                    "1.0.0",
                    vec![FunctionDescriptor {
                        name: "steal".into(),
                        service: services[0].clone(),
                        params: Vec::new(),
                        result: Vs3AbiType::Null,
                        ..FunctionDescriptor::new(Vs3AbiType::Null)
                    }],
                )
                .map_err(|e| e.to_string())?;
                match copy.register(thief) {
                    Ok(()) => Err("duplicate service registration accepted".into()),
                    Err(_) => Ok(()),
                }
            },
        },
        ConformanceCase {
            name: "registry/rejects-invalid-descriptor",
            run: |_| {
                if ModuleDescriptor::new("bad name!", "1.0.0", vec![]).is_ok() {
                    return Err("module with invalid name accepted".into());
                }
                if ModuleDescriptor::new("conform.bad", "not-a-version", vec![]).is_ok() {
                    return Err("module with invalid version accepted".into());
                }
                Ok(())
            },
        },
        ConformanceCase {
            name: "registry/validates-arity-and-types",
            run: |registry| {
                let Some((_, function)) = first_typed_function(registry) else {
                    return Err("suite needs a registry with a typed function".into());
                };
                let good =
                    HostRequest::new(function.service.clone(), Value::list(sample_args(function)))
                        .map_err(|e| e.to_string())?;
                crate::validate_request(function, &good).map_err(|e| {
                    format!("valid request rejected for `{}`: {e}", function.service)
                })?;
                // Wrong arity must fail.
                let bad_arity = HostRequest::new(
                    function.service.clone(),
                    Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
                )
                .map_err(|e| e.to_string())?;
                if function.params.len() == 3 {
                    // Degenerate: probe with a non-list payload instead.
                    let not_list = HostRequest {
                        service: function.service.clone(),
                        payload: Value::Int(1),
                    };
                    if crate::validate_request(function, &not_list).is_ok() {
                        return Err("non-list payload accepted".into());
                    }
                } else if crate::validate_request(function, &bad_arity).is_ok() {
                    return Err(format!("wrong arity accepted for `{}`", function.service));
                }
                Ok(())
            },
        },
        ConformanceCase {
            name: "protocol/host-request-parsing",
            run: |_| {
                let ok = HostRequest::from_value(&Value::list(vec![
                    Value::String("conform.echo".into()),
                    Value::list(vec![Value::Int(1)]),
                ]));
                // (no `Option::is_none_or`: MSRV is 1.80)
                if !ok.as_ref().is_some_and(|r| r.service == "conform.echo") {
                    return Err("canonical [service, payload] request not parsed".into());
                }
                for bad in [
                    Value::Null,
                    Value::Int(1),
                    Value::list(vec![Value::Int(1)]),
                    Value::list(vec![
                        Value::String("conform.echo".into()),
                        Value::Int(1),
                        Value::Int(2),
                    ]),
                ] {
                    if HostRequest::from_value(&bad).is_some() {
                        return Err(format!("malformed request parsed: {bad:?}"));
                    }
                }
                Ok(())
            },
        },
        ConformanceCase {
            name: "drive/ready-roundtrip",
            run: |_| {
                let mut host = drive_host();
                host.respond_ready("conform.echo", Value::Int(41));
                match drive_task(
                    "function probe() {\n return yield([\"conform.echo\", []])\n}\n",
                    &mut host,
                    &Vs3HostPolicy::allow_all(),
                ) {
                    Ok(Vs3TaskStatus::Complete(Value::Int(41))) => Ok(()),
                    Ok(other) => Err(format!("expected Complete(41), got {other:?}")),
                    Err(reason) => Err(format!("drive failed: {reason}")),
                }
            },
        },
        ConformanceCase {
            name: "drive/denied-service-errors",
            run: |_| {
                let mut host = drive_host();
                host.respond_ready("conform.echo", Value::Int(1));
                let policy = Vs3HostPolicy::deny_all();
                match drive_task(
                    "function probe() {\n return yield([\"conform.echo\", []])\n}\n",
                    &mut host,
                    &policy,
                ) {
                    Err(reason) if reason.contains("conform.echo") => Ok(()),
                    Err(reason) => Err(format!("expected Permission, got {reason}")),
                    Ok(status) => Err(format!("denied service completed: {status:?}")),
                }
            },
        },
        ConformanceCase {
            name: "drive/pending-ticket-waits",
            run: |_| {
                let mut host = drive_host();
                let ticket = host.respond_pending("conform.slow");
                match drive_task(
                    "function probe() {\n return yield([\"conform.slow\", []])\n}\n",
                    &mut host,
                    &Vs3HostPolicy::allow_all(),
                ) {
                    Ok(Vs3TaskStatus::Waiting { ticket: got, .. }) if got == ticket => Ok(()),
                    Ok(other) => Err(format!("expected Waiting({ticket}), got {other:?}")),
                    Err(reason) => Err(format!("drive failed: {reason}")),
                }
            },
        },
        ConformanceCase {
            name: "drive/failed-service-errors",
            run: |_| {
                // Unregistered + unscripted: SimulatedHost answers Ready(null),
                // so drive a host that fails instead.
                struct FailAll;
                impl Vs3Host for FailAll {
                    fn call(&mut self, request: &HostRequest) -> HostOutcome {
                        HostOutcome::Failed(format!("no such service {}", request.service))
                    }
                }
                let module = crate::compile(
                    "// @edition 3\nfunction probe() {\n return yield([\"conform.missing\", []])\n}\n",
                    Some("conform.vel"),
                )
                .map_err(|e| e.to_string())?;
                let mut task = module.start("probe", &[]).map_err(|e| e.to_string())?;
                match task.drive_host(&mut FailAll) {
                    Err(reason) if format!("{reason:?}").contains("conform.missing") => Ok(()),
                    Err(other) => Err(format!("expected Host error, got {other:?}")),
                    Ok(status) => Err(format!("failed service completed: {status:?}")),
                }
            },
        },
        ConformanceCase {
            name: "budgets/instruction-limit-aborts",
            run: |_| {
                let module = crate::compile(
                    "// @edition 3\nfunction spin() {\n let i = 0\n while (true) {\n i = i + 1\n }\n return i\n}\n",
                    Some("conform.vel"),
                )
                .map_err(|e| e.to_string())?;
                let limits = crate::VmLimits {
                    max_instructions: 1_000,
                    ..crate::VmLimits::default()
                };
                match module.call_with_limits("spin", &[], limits) {
                    // Controlled error, never a hang.
                    Err(_) => Ok(()),
                    Ok(value) => Err(format!("unbounded loop completed: {value:?}")),
                }
            },
        },
        ConformanceCase {
            name: "drive/request-budget-enforced",
            run: |_| {
                let mut host = drive_host();
                host.respond_ready("conform.echo", Value::Int(1));
                let policy = Vs3HostPolicy {
                    max_immediate_requests: 2,
                    ..Vs3HostPolicy::allow_all()
                };
                match drive_task(
                    "function probe() {\n yield([\"conform.echo\", []])\n yield([\"conform.echo\", []])\n yield([\"conform.echo\", []])\n return 1\n}\n",
                    &mut host,
                    &policy,
                ) {
                    Err(reason) if reason.contains("budget") => Ok(()),
                    Err(reason) => Err(format!("expected Task budget error, got {reason}")),
                    Ok(status) => Err(format!("over-budget drive completed: {status:?}")),
                }
            },
        },
    ]
}

/// Run every conformance case; cases never panic (failures report).
pub fn run_conformance_suite(registry: &AbiRegistry) -> ConformanceReport {
    let mut report = ConformanceReport::default();
    for case in conformance_cases() {
        match (case.run)(registry) {
            Ok(()) => report.passed += 1,
            Err(reason) => {
                report.failed += 1;
                report.failures.push(format!("{}: {reason}", case.name));
            }
        }
    }
    report
}

/// Fresh scripted host for drive cases (registry-free protocol checks).
fn drive_host() -> SimulatedHost {
    SimulatedHost::unregistered()
}

/// Assert a host fails a request (unknown service, bad arity, bad types).
///
/// For use in real-host test suites: every registry-backed host must fail
/// loudly instead of guessing.
pub fn expect_failed(host: &mut dyn Vs3Host, service: &str, args: Vec<Value>) {
    let request = HostRequest::new(service, Value::list(args)).unwrap();
    match host.call(&request) {
        HostOutcome::Failed(_) => {}
        other => panic!("expected Failed for `{service}`, got {other:?}"),
    }
}

/// Assert a host answers `Ready(expected)` for a request.
pub fn expect_ready(host: &mut dyn Vs3Host, service: &str, args: Vec<Value>, expected: Value) {
    let request = HostRequest::new(service, Value::list(args)).unwrap();
    match host.call(&request) {
        HostOutcome::Ready(got) if got == expected => {}
        other => panic!("expected Ready({expected:?}) for `{service}`, got {other:?}"),
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Rebuild an equal registry (AbiRegistry is not Clone by design).
fn clone_registry(registry: &AbiRegistry) -> Result<AbiRegistry, String> {
    let mut copy = AbiRegistry::new();
    for name in registry.module_names() {
        copy.register(rebuild_module(registry, name)?)
            .map_err(|e| e.to_string())?;
    }
    Ok(copy)
}

fn rebuild_module(registry: &AbiRegistry, name: &str) -> Result<ModuleDescriptor, String> {
    let module = registry
        .module(name)
        .ok_or_else(|| format!("module `{name}` missing"))?;
    // Rebuild through the validated constructor with the SAME name so
    // registration rejects it for duplication rather than shape.
    let functions = module
        .functions
        .values()
        .map(|f| FunctionDescriptor {
            name: f.name.clone(),
            service: f.service.clone(),
            params: f.params.clone(),
            result: f.result.clone(),
            purity: f.purity,
            sync: f.sync,
            cost: f.cost,
            capabilities: f.capabilities.clone(),
            doc: f.doc.clone(),
        })
        .collect();
    ModuleDescriptor::new(module.name.clone(), module.version.clone(), functions)
        .map_err(|e| e.to_string())
}

fn registry_services(registry: &AbiRegistry) -> Vec<String> {
    let mut out = Vec::new();
    for name in registry.module_names() {
        if let Some(module) = registry.module(name) {
            out.extend(module.functions.values().map(|f| f.service.clone()));
        }
    }
    out
}

fn first_typed_function(registry: &AbiRegistry) -> Option<(String, &FunctionDescriptor)> {
    for name in registry.module_names() {
        let module = registry.module(name)?;
        for f in module.functions.values() {
            if !f.params.is_empty() {
                return Some((name.to_string(), f));
            }
        }
    }
    None
}

fn sample_args(function: &FunctionDescriptor) -> Vec<Value> {
    function
        .params
        .iter()
        .map(|p| match &p.ty {
            Vs3AbiType::Bool => Value::Bool(true),
            Vs3AbiType::Int => Value::Int(1),
            Vs3AbiType::Float | Vs3AbiType::Number => Value::Float(1.0),
            Vs3AbiType::Str => Value::String("x".into()),
            _ => Value::Null,
        })
        .collect()
}

fn drive_task(
    body: &str,
    host: &mut SimulatedHost,
    policy: &Vs3HostPolicy,
) -> Result<Vs3TaskStatus, String> {
    let src = format!("// @edition 3\n{body}");
    let module = crate::compile(&src, Some("conform.vel")).map_err(|e| e.to_string())?;
    let mut task = module.start("probe", &[]).map_err(|e| e.to_string())?;
    task.drive_host_with_policy(host, policy)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParamDescriptor, Vs3AbiType};

    fn sample_registry() -> AbiRegistry {
        let mut registry = AbiRegistry::new();
        registry
            .register(
                ModuleDescriptor::new(
                    "conform",
                    "1.0.0",
                    vec![
                        FunctionDescriptor {
                            name: "echo".into(),
                            service: "conform.echo".into(),
                            params: vec![ParamDescriptor::new("value", Vs3AbiType::Int)],
                            result: Vs3AbiType::Int,
                            ..FunctionDescriptor::new(Vs3AbiType::Int)
                        },
                        FunctionDescriptor {
                            name: "ping".into(),
                            service: "conform.ping".into(),
                            params: Vec::new(),
                            result: Vs3AbiType::Null,
                            ..FunctionDescriptor::new(Vs3AbiType::Null)
                        },
                    ],
                )
                .unwrap(),
            )
            .unwrap();
        registry
    }

    #[test]
    fn suite_passes_on_simulated_host() {
        let registry = sample_registry();
        let report = run_conformance_suite(&registry);
        assert!(report.ok(), "failures: {:?}", report.failures);
        assert_eq!(report.passed, conformance_cases().len());
    }

    #[test]
    fn suite_counts_are_exact() {
        let registry = sample_registry();
        let report = run_conformance_suite(&registry);
        assert_eq!(report.passed + report.failed, conformance_cases().len());
        assert!(report.failures.is_empty());
    }

    #[test]
    fn shared_helpers_assert_both_ways() {
        let registry = sample_registry();
        let mut host = SimulatedHost::new(registry);
        host.respond_ready("conform.echo", Value::Int(7));
        expect_ready(
            &mut host,
            "conform.echo",
            vec![Value::Int(1)],
            Value::Int(7),
        );
        expect_failed(&mut host, "conform.missing", vec![]);
    }
}
