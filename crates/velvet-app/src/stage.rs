//! Named application stages and ordered stage graph.
//!
//! Stages group systems into ordered buckets (Startup, PreUpdate, Update,
//! PostUpdate, Render, Shutdown). Plugins declare work against stages without
//! knowing global system lists.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

/// Built-in stage identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageId {
    /// One-shot after plugins finish building.
    Startup,
    /// Before main simulation.
    PreUpdate,
    /// Main variable update.
    Update,
    /// Fixed-timestep simulation bucket.
    FixedUpdate,
    /// After simulation, before render prep.
    PostUpdate,
    /// Prepare GPU / draw lists.
    PreRender,
    /// Present / submit.
    Render,
    /// After frame present.
    PostRender,
    /// App exit teardown.
    Shutdown,
    /// Custom named stage (interned as string outside this enum via StageKey).
    Custom,
}

impl StageId {
    /// Default display name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "Startup",
            Self::PreUpdate => "PreUpdate",
            Self::Update => "Update",
            Self::FixedUpdate => "FixedUpdate",
            Self::PostUpdate => "PostUpdate",
            Self::PreRender => "PreRender",
            Self::Render => "Render",
            Self::PostRender => "PostRender",
            Self::Shutdown => "Shutdown",
            Self::Custom => "Custom",
        }
    }

    /// Core ordered loop stages (excludes Startup/Shutdown).
    pub fn frame_loop() -> &'static [StageId] {
        &[
            Self::PreUpdate,
            Self::Update,
            Self::FixedUpdate,
            Self::PostUpdate,
            Self::PreRender,
            Self::Render,
            Self::PostRender,
        ]
    }
}

/// Stage key: built-in or custom string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKey {
    /// Built-in.
    Builtin(StageId),
    /// Custom label.
    Named(String),
}

impl StageKey {
    /// Builtin helper.
    pub fn builtin(id: StageId) -> Self {
        Self::Builtin(id)
    }

    /// Named helper.
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    /// Display.
    pub fn label(&self) -> String {
        match self {
            Self::Builtin(id) => id.as_str().into(),
            Self::Named(s) => s.clone(),
        }
    }
}

impl From<StageId> for StageKey {
    fn from(value: StageId) -> Self {
        Self::Builtin(value)
    }
}

/// Edge: `before` must run before `after`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageEdge {
    /// Predecessor.
    pub before: StageKey,
    /// Successor.
    pub after: StageKey,
}

/// Ordered stage schedule with optional custom stages.
#[derive(Debug, Clone, Default)]
pub struct StageSchedule {
    stages: Vec<StageKey>,
    edges: Vec<StageEdge>,
}

impl StageSchedule {
    /// Default engine schedule.
    pub fn default_engine() -> Self {
        let mut s = Self::default();
        s.stages.push(StageKey::builtin(StageId::Startup));
        for id in StageId::frame_loop() {
            s.stages.push(StageKey::builtin(*id));
        }
        s.stages.push(StageKey::builtin(StageId::Shutdown));
        // Sequential edges for frame loop
        let loop_keys: Vec<_> = StageId::frame_loop()
            .iter()
            .map(|id| StageKey::builtin(*id))
            .collect();
        for w in loop_keys.windows(2) {
            s.edges.push(StageEdge {
                before: w[0].clone(),
                after: w[1].clone(),
            });
        }
        s
    }

    /// Insert a custom stage if missing.
    pub fn ensure(&mut self, key: StageKey) {
        if !self.stages.iter().any(|s| s == &key) {
            self.stages.push(key);
        }
    }

    /// Add ordering constraint.
    pub fn order_before(&mut self, before: StageKey, after: StageKey) {
        self.ensure(before.clone());
        self.ensure(after.clone());
        self.edges.push(StageEdge { before, after });
    }

    /// Topologically sort stages; on cycle returns insertion order of known stages.
    pub fn resolve(&self) -> Vec<StageKey> {
        let mut indeg: HashMap<StageKey, usize> = HashMap::new();
        let mut adj: HashMap<StageKey, Vec<StageKey>> = HashMap::new();
        for s in &self.stages {
            indeg.entry(s.clone()).or_insert(0);
            adj.entry(s.clone()).or_default();
        }
        for e in &self.edges {
            indeg.entry(e.after.clone()).or_insert(0);
            indeg.entry(e.before.clone()).or_insert(0);
            *indeg.get_mut(&e.after).unwrap() += 1;
            adj.entry(e.before.clone())
                .or_default()
                .push(e.after.clone());
        }
        let mut q: VecDeque<StageKey> = indeg
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        // Stable: prefer original registration order among zeros
        q.make_contiguous().sort_by_key(|k| {
            self.stages
                .iter()
                .position(|s| s == k)
                .unwrap_or(usize::MAX)
        });
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        while let Some(n) = q.pop_front() {
            if !seen.insert(n.clone()) {
                continue;
            }
            out.push(n.clone());
            if let Some(nexts) = adj.get(&n) {
                for m in nexts {
                    if let Some(d) = indeg.get_mut(m) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            q.push_back(m.clone());
                        }
                    }
                }
            }
        }
        // Append any missing (cycle remainder) in registration order
        for s in &self.stages {
            if !seen.contains(s) {
                out.push(s.clone());
            }
        }
        out
    }

    /// Labels of resolved order.
    pub fn resolve_labels(&self) -> Vec<String> {
        self.resolve().into_iter().map(|k| k.label()).collect()
    }

    /// Stage count.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }
}

/// Per-frame stage execution stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageFrameStats {
    /// Nanoseconds spent per stage label this frame.
    pub nanos: HashMap<String, u64>,
    /// Systems run per stage.
    pub system_counts: HashMap<String, u32>,
}

impl StageFrameStats {
    /// Record duration.
    pub fn record(&mut self, stage: &str, nanos: u64, systems: u32) {
        *self.nanos.entry(stage.into()).or_default() += nanos;
        *self.system_counts.entry(stage.into()).or_default() += systems;
    }

    /// Total nanos.
    pub fn total_nanos(&self) -> u64 {
        self.nanos.values().sum()
    }

    /// Clear.
    pub fn clear(&mut self) {
        self.nanos.clear();
        self.system_counts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_order_contains_update() {
        let s = StageSchedule::default_engine();
        let labels = s.resolve_labels();
        assert!(labels.iter().any(|l| l == "Update"));
        assert!(labels.iter().any(|l| l == "Render"));
    }

    #[test]
    fn custom_stage_ordered() {
        let mut s = StageSchedule::default_engine();
        let custom = StageKey::named("PhysicsSync");
        s.order_before(StageKey::builtin(StageId::Update), custom.clone());
        s.order_before(custom.clone(), StageKey::builtin(StageId::PostUpdate));
        let labels = s.resolve_labels();
        let i_u = labels.iter().position(|l| l == "Update").unwrap();
        let i_c = labels.iter().position(|l| l == "PhysicsSync").unwrap();
        let i_p = labels.iter().position(|l| l == "PostUpdate").unwrap();
        assert!(i_u < i_c && i_c < i_p);
    }

    #[test]
    fn frame_stats_record() {
        let mut st = StageFrameStats::default();
        st.record("Update", 1000, 3);
        st.record("Update", 500, 1);
        assert_eq!(st.nanos["Update"], 1500);
        assert_eq!(st.system_counts["Update"], 4);
        assert_eq!(st.total_nanos(), 1500);
    }
}
