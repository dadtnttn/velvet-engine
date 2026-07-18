//! Input contexts (gameplay vs UI vs dialogue).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::action::ActionId;
use crate::binding::Binding;

/// Context identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputContextId(pub String);

impl InputContextId {
    /// Create.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// As str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for InputContextId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// A set of bindings active together.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputContext {
    /// Context id.
    pub id: String,
    /// Priority (higher wins when multiple active).
    pub priority: i32,
    /// Bindings.
    pub bindings: Vec<Binding>,
    /// Actions blocked from lower contexts.
    pub blocks: Vec<ActionId>,
}

impl InputContext {
    /// Create empty context.
    pub fn new(id: impl Into<String>, priority: i32) -> Self {
        Self {
            id: id.into(),
            priority,
            bindings: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Add binding.
    pub fn with_binding(mut self, binding: Binding) -> Self {
        self.bindings.push(binding);
        self
    }
}

/// Stack of active contexts ordered by priority.
#[derive(Debug, Clone, Default)]
pub struct ContextStack {
    contexts: IndexMap<String, InputContext>,
    active: Vec<String>,
}

impl ContextStack {
    /// Register a context definition.
    pub fn register(&mut self, ctx: InputContext) {
        let id = ctx.id.clone();
        self.contexts.insert(id, ctx);
    }

    /// Push context id as active.
    pub fn push(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.active.contains(&id) {
            self.active.push(id);
            self.sort_active();
        }
    }

    /// Pop context.
    pub fn pop(&mut self, id: &str) {
        self.active.retain(|x| x != id);
    }

    /// Active contexts high priority first.
    pub fn active_contexts(&self) -> Vec<&InputContext> {
        self.active
            .iter()
            .filter_map(|id| self.contexts.get(id))
            .collect()
    }

    /// All bindings from active contexts (high priority first).
    pub fn active_bindings(&self) -> Vec<&Binding> {
        let mut out = Vec::new();
        for ctx in self.active_contexts() {
            for b in &ctx.bindings {
                out.push(b);
            }
        }
        out
    }

    fn sort_active(&mut self) {
        self.active.sort_by(|a, b| {
            let pa = self.contexts.get(a).map(|c| c.priority).unwrap_or(0);
            let pb = self.contexts.get(b).map(|c| c.priority).unwrap_or(0);
            pb.cmp(&pa)
        });
    }
}
