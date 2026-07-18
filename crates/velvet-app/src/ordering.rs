//! System ordering labels and dependency graphs.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::schedule::ScheduleLabel;
use crate::system::SystemId;

/// Named ordering label for systems within a schedule (finer than stage).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemLabel(pub String);

impl SystemLabel {
    /// Create.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// As str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SystemLabel {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for SystemLabel {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Common labels used by engine plugins.
pub mod labels {
    use super::SystemLabel;

    /// Input sampling.
    pub fn input() -> SystemLabel {
        SystemLabel::new("input")
    }
    /// Gameplay simulation.
    pub fn gameplay() -> SystemLabel {
        SystemLabel::new("gameplay")
    }
    /// Physics step.
    pub fn physics() -> SystemLabel {
        SystemLabel::new("physics")
    }
    /// Animation.
    pub fn animation() -> SystemLabel {
        SystemLabel::new("animation")
    }
    /// Audio update.
    pub fn audio() -> SystemLabel {
        SystemLabel::new("audio")
    }
    /// UI layout.
    pub fn ui() -> SystemLabel {
        SystemLabel::new("ui")
    }
    /// Render extraction.
    pub fn extract() -> SystemLabel {
        SystemLabel::new("extract")
    }
    /// Render submit.
    pub fn render() -> SystemLabel {
        SystemLabel::new("render")
    }
}

/// Node metadata for a registered system in the graph.
#[derive(Debug, Clone)]
pub struct SystemNode {
    /// System id.
    pub id: SystemId,
    /// Schedule this system belongs to.
    pub schedule: ScheduleLabel,
    /// Labels applied to the system.
    pub labels: Vec<SystemLabel>,
    /// Labels this system must run after.
    pub after: Vec<SystemLabel>,
    /// Labels this system must run before.
    pub before: Vec<SystemLabel>,
    /// Exclusive systems run alone on the app.
    pub exclusive: bool,
}

/// Graph of system ordering constraints within (and across) schedules.
#[derive(Debug, Default, Clone)]
pub struct SystemOrderGraph {
    nodes: Vec<SystemNode>,
    /// Label → system ids that have that label.
    label_index: HashMap<SystemLabel, Vec<SystemId>>,
}

impl SystemOrderGraph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a system node.
    pub fn add_node(&mut self, node: SystemNode) {
        for label in &node.labels {
            self.label_index
                .entry(label.clone())
                .or_default()
                .push(node.id);
        }
        self.nodes.push(node);
    }

    /// Builder-style registration.
    pub fn register(&mut self, id: SystemId, schedule: ScheduleLabel) -> SystemNodeBuilder<'_> {
        SystemNodeBuilder {
            graph: self,
            node: SystemNode {
                id,
                schedule,
                labels: Vec::new(),
                after: Vec::new(),
                before: Vec::new(),
                exclusive: false,
            },
        }
    }

    /// All nodes.
    pub fn nodes(&self) -> &[SystemNode] {
        &self.nodes
    }

    /// Nodes for a schedule.
    pub fn nodes_in(&self, schedule: ScheduleLabel) -> impl Iterator<Item = &SystemNode> {
        self.nodes.iter().filter(move |n| n.schedule == schedule)
    }

    /// Resolve a stable order of system ids for `schedule`.
    ///
    /// Constraints: for each node with `after: [L]`, the node runs after all
    /// systems labeled `L` in the same schedule. Similarly for `before`.
    pub fn resolve(&self, schedule: ScheduleLabel) -> Result<Vec<SystemId>, OrderError> {
        let nodes: Vec<&SystemNode> = self.nodes_in(schedule).collect();
        if nodes.is_empty() {
            return Ok(Vec::new());
        }

        let index: HashMap<SystemId, usize> =
            nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();

        // Build adjacency: edge A -> B means A must run before B.
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut indegree = vec![0usize; nodes.len()];

        let labeled = |label: &SystemLabel| -> Vec<usize> {
            self.label_index
                .get(label)
                .into_iter()
                .flatten()
                .filter_map(|id| index.get(id).copied())
                .collect()
        };

        for (i, node) in nodes.iter().enumerate() {
            for label in &node.after {
                for j in labeled(label) {
                    if j != i {
                        // j before i
                        adj[j].push(i);
                        indegree[i] += 1;
                    }
                }
            }
            for label in &node.before {
                for j in labeled(label) {
                    if j != i {
                        // i before j
                        adj[i].push(j);
                        indegree[j] += 1;
                    }
                }
            }
        }

        let mut q: VecDeque<usize> = indegree
            .iter()
            .enumerate()
            .filter(|(_, d)| **d == 0)
            .map(|(i, _)| i)
            .collect();

        // Stable: sort ready set by system id.
        let mut ready: Vec<usize> = q.drain(..).collect();
        ready.sort_by_key(|i| nodes[*i].id.0);
        q.extend(ready);

        let mut order = Vec::with_capacity(nodes.len());
        while let Some(i) = q.pop_front() {
            order.push(nodes[i].id);
            let mut nexts = Vec::new();
            for &j in &adj[i] {
                indegree[j] = indegree[j].saturating_sub(1);
                if indegree[j] == 0 {
                    nexts.push(j);
                }
            }
            nexts.sort_by_key(|j| nodes[*j].id.0);
            q.extend(nexts);
        }

        if order.len() != nodes.len() {
            return Err(OrderError::Cycle {
                schedule,
                resolved: order.len(),
                total: nodes.len(),
            });
        }
        Ok(order)
    }

    /// Validate all schedules that have nodes.
    pub fn validate_all(&self) -> Result<(), OrderError> {
        let schedules: HashSet<ScheduleLabel> = self.nodes.iter().map(|n| n.schedule).collect();
        for s in schedules {
            self.resolve(s)?;
        }
        Ok(())
    }

    /// Exclusive system ids in registration order.
    pub fn exclusive_ids(&self) -> Vec<SystemId> {
        self.nodes
            .iter()
            .filter(|n| n.exclusive)
            .map(|n| n.id)
            .collect()
    }
}

/// Fluent builder for a system node.
pub struct SystemNodeBuilder<'a> {
    graph: &'a mut SystemOrderGraph,
    node: SystemNode,
}

impl SystemNodeBuilder<'_> {
    /// Add a label.
    pub fn label(mut self, label: impl Into<SystemLabel>) -> Self {
        self.node.labels.push(label.into());
        self
    }

    /// Run after systems with this label.
    pub fn after(mut self, label: impl Into<SystemLabel>) -> Self {
        self.node.after.push(label.into());
        self
    }

    /// Run before systems with this label.
    pub fn before(mut self, label: impl Into<SystemLabel>) -> Self {
        self.node.before.push(label.into());
        self
    }

    /// Mark exclusive.
    pub fn exclusive(mut self) -> Self {
        self.node.exclusive = true;
        self
    }

    /// Commit to graph.
    pub fn finish(self) {
        self.graph.add_node(self.node);
    }
}

/// Ordering resolution error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderError {
    /// Cycle detected in constraints.
    Cycle {
        /// Schedule.
        schedule: ScheduleLabel,
        /// How many nodes ordered before failure.
        resolved: usize,
        /// Total nodes.
        total: usize,
    },
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cycle {
                schedule,
                resolved,
                total,
            } => write!(
                f,
                "system order cycle in {schedule}: resolved {resolved}/{total}"
            ),
        }
    }
}

impl std::error::Error for OrderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_after_label() {
        let mut g = SystemOrderGraph::new();
        g.register(SystemId(1), ScheduleLabel::Update)
            .label(labels::input())
            .finish();
        g.register(SystemId(2), ScheduleLabel::Update)
            .label(labels::gameplay())
            .after(labels::input())
            .finish();
        g.register(SystemId(3), ScheduleLabel::Update)
            .after(labels::gameplay())
            .finish();
        let order = g.resolve(ScheduleLabel::Update).unwrap();
        assert_eq!(order, vec![SystemId(1), SystemId(2), SystemId(3)]);
    }

    #[test]
    fn cycle_errors() {
        let mut g = SystemOrderGraph::new();
        g.register(SystemId(1), ScheduleLabel::Update)
            .label("a")
            .after("b")
            .finish();
        g.register(SystemId(2), ScheduleLabel::Update)
            .label("b")
            .after("a")
            .finish();
        assert!(g.resolve(ScheduleLabel::Update).is_err());
    }

    #[test]
    fn exclusive_listed() {
        let mut g = SystemOrderGraph::new();
        g.register(SystemId(9), ScheduleLabel::Last)
            .exclusive()
            .finish();
        assert_eq!(g.exclusive_ids(), vec![SystemId(9)]);
    }
}
