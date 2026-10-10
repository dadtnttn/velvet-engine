//! Reference `velvet.ecs` host module: entities, components, deferred commands.
//!
//! Handles are opaque strings (`"e{id}:{generation}"`); a generation bump
//! on slot reuse turns use-after-despawn into a loud `Failed` instead of
//! aliasing a stranger. Mutations queue up and apply on [`EcsHost::flush`]
//! (or [`EcsHost::flush_now`] from Rust); queries read committed state:
//!
//! ```ignore
//! // @edition 3
//! function spawn_hero() {
//!     let hero = yield(["velvet.ecs.spawn", []])
//!     yield(["velvet.ecs.set", [hero, "hp", 100]])
//!     yield(["velvet.ecs.flush", []])
//!     return yield(["velvet.ecs.get", [hero, "hp"]])
//! }
//! ```

use std::collections::BTreeMap;

use velvet_script_vs3::{
    validate_request, FunctionDescriptor, HostOutcome, HostRequest, ModuleDescriptor,
    ParamDescriptor, Purity, Value, Vs3AbiType, Vs3Host,
};

/// Capability required by mutating ECS services.
pub const ECS_CAPABILITY: &str = "ecs";

fn ecs_function(
    name: &str,
    params: Vec<ParamDescriptor>,
    result: Vs3AbiType,
    pure: bool,
    doc: &str,
) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        service: format!("velvet.ecs.{name}"),
        params,
        result: result.clone(),
        purity: if pure { Purity::Pure } else { Purity::Impure },
        capabilities: if pure {
            Vec::new()
        } else {
            vec![ECS_CAPABILITY.into()]
        },
        doc: doc.into(),
        ..FunctionDescriptor::new(result)
    }
}

fn str_param(name: &str) -> ParamDescriptor {
    ParamDescriptor::new(name, Vs3AbiType::Str)
}

/// Descriptor for the `velvet.ecs` module.
pub fn ecs_module() -> ModuleDescriptor {
    ModuleDescriptor::new(
        "velvet.ecs",
        "1.0.0",
        vec![
            ecs_function(
                "spawn",
                vec![],
                Vs3AbiType::Str,
                false,
                "reserve an entity, return its handle",
            ),
            ecs_function(
                "despawn",
                vec![str_param("handle")],
                Vs3AbiType::Null,
                false,
                "queue despawn of handle",
            ),
            ecs_function(
                "set",
                vec![
                    str_param("handle"),
                    str_param("component"),
                    ParamDescriptor::new("value", Vs3AbiType::Any),
                ],
                Vs3AbiType::Null,
                false,
                "queue a component write",
            ),
            ecs_function(
                "get",
                vec![str_param("handle"), str_param("component")],
                Vs3AbiType::Any,
                true,
                "read committed component (Null when absent)",
            ),
            ecs_function(
                "has",
                vec![str_param("handle"), str_param("component")],
                Vs3AbiType::Bool,
                true,
                "whether the committed entity has the component",
            ),
            ecs_function(
                "flush",
                vec![],
                Vs3AbiType::Int,
                false,
                "apply queued commands, return applied count",
            ),
        ],
    )
    .expect("built-in descriptors are valid")
}

#[derive(Debug, Clone, Default)]
struct Slot {
    committed: bool,
    components: BTreeMap<String, Value>,
}

#[derive(Debug, Clone)]
enum EcsOp {
    Spawn {
        id: usize,
    },
    Despawn {
        id: usize,
        generation: u64,
    },
    Set {
        id: usize,
        component: String,
        value: Value,
    },
}

/// Reference host for [`ecs_module`] with a deferred command queue.
#[derive(Debug, Default)]
pub struct EcsHost {
    slots: Vec<Option<Slot>>,
    /// Per-slot generation; survives freeing so recycled handles differ.
    generations: Vec<u64>,
    free: Vec<usize>,
    pending: Vec<EcsOp>,
}

impl EcsHost {
    /// Empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queued (unflushed) command count.
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Apply queued commands in order; returns applied count.
    ///
    /// Double-despawns and writes to freed slots are skipped idempotently.
    pub fn flush_now(&mut self) -> usize {
        let mut applied = 0usize;
        for op in std::mem::take(&mut self.pending) {
            match op {
                EcsOp::Spawn { id } => {
                    if let Some(slot) = self.slots.get_mut(id).and_then(|s| s.as_mut()) {
                        slot.committed = true;
                        applied += 1;
                    }
                }
                EcsOp::Despawn { id, generation } => {
                    let live = self.slots.get(id).and_then(|s| s.as_ref()).is_some()
                        && self.generations.get(id) == Some(&generation);
                    if live {
                        self.slots[id] = None;
                        self.free.push(id);
                        // Generation bumps on the next reserve.
                        applied += 1;
                    }
                }
                EcsOp::Set {
                    id,
                    component,
                    value,
                } => {
                    if let Some(slot) = self.slots.get_mut(id).and_then(|s| s.as_mut()) {
                        slot.components.insert(component, value);
                        applied += 1;
                    }
                }
            }
        }
        applied
    }

    /// Resolve `"e{id}:{gen}"` to a live slot index, or a failure message.
    fn resolve(&self, handle: &str) -> Result<usize, String> {
        let (id, generation) = parse_handle(handle)?;
        let live = self.slots.get(id).and_then(|s| s.as_ref()).is_some()
            && self.generations.get(id) == Some(&generation);
        if live {
            Ok(id)
        } else {
            Err(format!("stale or unknown entity handle `{handle}`"))
        }
    }

    fn reserve(&mut self) -> (usize, u64) {
        if let Some(id) = self.free.pop() {
            self.generations[id] += 1;
            let generation = self.generations[id];
            self.slots[id] = Some(Slot {
                committed: false,
                components: BTreeMap::new(),
            });
            (id, generation)
        } else {
            let id = self.slots.len();
            self.slots.push(Some(Slot {
                committed: false,
                components: BTreeMap::new(),
            }));
            self.generations.push(1);
            (id, 1)
        }
    }
}

/// Parse `"e{id}:{generation}"`.
fn parse_handle(handle: &str) -> Result<(usize, u64), String> {
    let body = handle
        .strip_prefix('e')
        .ok_or_else(|| format!("malformed entity handle `{handle}`"))?;
    let (id, generation) = body
        .split_once(':')
        .ok_or_else(|| format!("malformed entity handle `{handle}`"))?;
    let id = id
        .parse::<usize>()
        .map_err(|_| format!("malformed entity handle `{handle}`"))?;
    let generation = generation
        .parse::<u64>()
        .map_err(|_| format!("malformed entity handle `{handle}`"))?;
    Ok((id, generation))
}

fn handle(id: usize, generation: u64) -> String {
    format!("e{id}:{generation}")
}

impl Vs3Host for EcsHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        let module = ecs_module();
        let Some(function) = module
            .functions
            .values()
            .find(|f| f.service == request.service)
        else {
            return HostOutcome::Failed(format!(
                "service `{}` is not part of velvet.ecs",
                request.service
            ));
        };
        if let Err(message) = validate_request(function, request) {
            return HostOutcome::Failed(message);
        }
        let args = match &request.payload {
            Value::List(items) => items.borrow().clone(),
            other => {
                return HostOutcome::Failed(format!("ecs payload must be a list, got {other}"));
            }
        };
        let text = |i: usize| args[i].to_string();
        match function.name.as_str() {
            "spawn" => {
                let (id, generation) = self.reserve();
                self.pending.push(EcsOp::Spawn { id });
                HostOutcome::Ready(Value::String(handle(id, generation).into()))
            }
            "despawn" => match self.resolve(&text(0)) {
                Ok(id) => {
                    let generation = self.generations.get(id).copied().unwrap_or(0);
                    self.pending.push(EcsOp::Despawn { id, generation });
                    HostOutcome::Ready(Value::Null)
                }
                Err(message) => HostOutcome::Failed(message),
            },
            "set" => match self.resolve(&text(0)) {
                Ok(id) => {
                    self.pending.push(EcsOp::Set {
                        id,
                        component: text(1),
                        value: args[2].clone(),
                    });
                    HostOutcome::Ready(Value::Null)
                }
                Err(message) => HostOutcome::Failed(message),
            },
            "get" => match self.resolve(&text(0)) {
                Ok(id) => {
                    let value = self.slots[id]
                        .as_ref()
                        .and_then(|s| s.components.get(&text(1)))
                        .cloned()
                        .unwrap_or(Value::Null);
                    HostOutcome::Ready(value)
                }
                Err(message) => HostOutcome::Failed(message),
            },
            "has" => match self.resolve(&text(0)) {
                Ok(id) => {
                    let has = self.slots[id]
                        .as_ref()
                        .is_some_and(|s| s.components.contains_key(&text(1)));
                    HostOutcome::Ready(Value::Bool(has))
                }
                Err(message) => HostOutcome::Failed(message),
            },
            "flush" => {
                let applied = self.flush_now();
                HostOutcome::Ready(Value::Int(applied as i64))
            }
            other => HostOutcome::Failed(format!("unknown ecs function `{other}`")),
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

    fn call(host: &mut EcsHost, service: &str, args: Vec<Value>) -> HostOutcome {
        let request = HostRequest::new(service, Value::list(args)).unwrap();
        host.call(&request)
    }

    fn spawn(host: &mut EcsHost) -> String {
        match call(host, "velvet.ecs.spawn", vec![]) {
            HostOutcome::Ready(Value::String(h)) => h.to_string(),
            other => panic!("spawn failed: {other:?}"),
        }
    }

    #[test]
    fn descriptor_registers_with_capabilities() {
        let module = ecs_module();
        assert_eq!(module.functions.len(), 6);
        assert!(module.functions["spawn"]
            .capabilities
            .contains(&ECS_CAPABILITY.to_string()));
        assert!(module.functions["get"].capabilities.is_empty());
        assert!(matches!(module.functions["get"].purity, Purity::Pure));
    }

    #[test]
    fn writes_queue_until_flush() {
        let mut host = EcsHost::new();
        let hero = spawn(&mut host);
        call(
            &mut host,
            "velvet.ecs.set",
            vec![str_(&hero), str_("hp"), Value::Int(100)],
        );
        assert_eq!(host.pending_len(), 2);
        // Queries read committed state: nothing visible yet.
        assert!(matches!(
            call(&mut host, "velvet.ecs.get", vec![str_(&hero), str_("hp")]),
            HostOutcome::Ready(Value::Null)
        ));
        assert!(matches!(
            call(&mut host, "velvet.ecs.flush", vec![]),
            HostOutcome::Ready(Value::Int(2))
        ));
        assert_eq!(host.pending_len(), 0);
        assert!(matches!(
            call(
                &mut host,
                "velvet.ecs.get",
                vec![str_("hp-hero"), str_("hp")]
            ),
            HostOutcome::Failed(_)
        ));
        assert!(matches!(
            call(&mut host, "velvet.ecs.get", vec![str_(&hero), str_("hp")]),
            HostOutcome::Ready(Value::Int(100))
        ));
        assert!(matches!(
            call(&mut host, "velvet.ecs.has", vec![str_(&hero), str_("hp")]),
            HostOutcome::Ready(Value::Bool(true))
        ));
    }

    #[test]
    fn stale_handles_fail_loudly() {
        let mut host = EcsHost::new();
        let hero = spawn(&mut host);
        call(&mut host, "velvet.ecs.flush", vec![]);
        call(&mut host, "velvet.ecs.despawn", vec![str_(&hero)]);
        call(&mut host, "velvet.ecs.flush", vec![]);
        // Old handle must not alias the recycled slot.
        assert!(matches!(
            call(&mut host, "velvet.ecs.get", vec![str_(&hero), str_("hp")]),
            HostOutcome::Failed(_)
        ));
        // Malformed handles fail, never panic.
        for bad in ["nope", "e1", "e:x", "e1:x", "e999:1"] {
            assert!(matches!(
                call(&mut host, "velvet.ecs.get", vec![str_(bad), str_("hp")]),
                HostOutcome::Failed(_)
            ));
        }
        // Slot reuse hands out a fresh generation.
        let hero2 = spawn(&mut host);
        assert_ne!(hero, hero2);
        assert!(matches!(
            call(&mut host, "velvet.ecs.has", vec![str_(&hero2), str_("hp")]),
            HostOutcome::Ready(Value::Bool(false))
        ));
    }

    #[test]
    fn ecs_host_conforms_to_host_contract() {
        let mut host = EcsHost::new();
        conformance::expect_failed(&mut host, "velvet.ecs.nope", vec![]);
        conformance::expect_failed(&mut host, "velvet.ecs.get", vec![str_("e0:1")]);
        let hero = spawn(&mut host);
        conformance::expect_ready(
            &mut host,
            "velvet.ecs.has",
            vec![str_(&hero), str_("hp")],
            Value::Bool(false),
        );
    }
}
