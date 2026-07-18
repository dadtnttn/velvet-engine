//! Scene transition graph: named edges with conditions and effects.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors for graph operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransitionGraphError {
    /// Missing node.
    #[error("scene node not found: {0}")]
    MissingNode(String),
    /// Missing edge.
    #[error("transition not found: {from} -> {to}")]
    MissingEdge {
        /// From.
        from: String,
        /// To.
        to: String,
    },
    /// Condition failed.
    #[error("transition blocked: {0}")]
    Blocked(String),
}

/// A node in the scene graph (logical level / room id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneNode {
    /// Scene asset / blueprint name.
    pub name: String,
    /// Display title.
    pub title: String,
    /// Tags for filtering.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl SceneNode {
    /// Create simple node.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            title: name.clone(),
            name,
            tags: Vec::new(),
        }
    }

    /// With title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// With tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Condition evaluated by the game before taking an edge.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransitionCondition {
    /// Always true.
    #[default]
    Always,
    /// Require a flag name to be true in external store.
    Flag {
        /// Flag name.
        name: String,
    },
    /// Require integer variable >= value.
    VarAtLeast {
        /// Variable.
        name: String,
        /// Minimum.
        min: i64,
    },
    /// Invert nested condition.
    Not {
        /// Inner.
        inner: Box<TransitionCondition>,
    },
    /// All must pass.
    All {
        /// Children.
        items: Vec<TransitionCondition>,
    },
    /// Any may pass.
    Any {
        /// Children.
        items: Vec<TransitionCondition>,
    },
}

/// Side effect requested when taking a transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransitionEffect {
    /// Set flag true.
    SetFlag {
        /// Name.
        name: String,
    },
    /// Clear flag.
    ClearFlag {
        /// Name.
        name: String,
    },
    /// Play transition FX id (fade, wipe, …).
    PlayFx {
        /// Fx id.
        id: String,
    },
    /// Spawn point hint in destination.
    SpawnAt {
        /// Named spawn.
        spawn: String,
    },
}

/// Directed edge between scene nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneEdge {
    /// Destination scene name.
    pub to: String,
    /// Optional human label (door name, etc.).
    pub label: String,
    /// Condition.
    #[serde(default)]
    pub condition: TransitionCondition,
    /// Effects on success.
    #[serde(default)]
    pub effects: Vec<TransitionEffect>,
    /// Optional cost for pathfinding (default 1).
    #[serde(default = "default_cost")]
    pub cost: u32,
}

fn default_cost() -> u32 {
    1
}

impl SceneEdge {
    /// Create unconditional edge.
    pub fn new(to: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            label: String::new(),
            condition: TransitionCondition::Always,
            effects: Vec::new(),
            cost: 1,
        }
    }

    /// With label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// With condition.
    pub fn with_condition(mut self, condition: TransitionCondition) -> Self {
        self.condition = condition;
        self
    }

    /// Push effect.
    pub fn with_effect(mut self, effect: TransitionEffect) -> Self {
        self.effects.push(effect);
        self
    }
}

/// Context for evaluating conditions (provided by game).
pub trait TransitionContext {
    /// Flag value.
    fn flag(&self, name: &str) -> bool;
    /// Integer variable.
    fn var_i64(&self, name: &str) -> i64;
}

/// Simple in-memory context for tests / tools.
#[derive(Debug, Clone, Default)]
pub struct MapTransitionContext {
    /// Flags.
    pub flags: IndexMap<String, bool>,
    /// Vars.
    pub vars: IndexMap<String, i64>,
}

impl TransitionContext for MapTransitionContext {
    fn flag(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }
    fn var_i64(&self, name: &str) -> i64 {
        self.vars.get(name).copied().unwrap_or(0)
    }
}

/// Evaluate a condition against a context.
pub fn eval_condition(cond: &TransitionCondition, ctx: &dyn TransitionContext) -> bool {
    match cond {
        TransitionCondition::Always => true,
        TransitionCondition::Flag { name } => ctx.flag(name),
        TransitionCondition::VarAtLeast { name, min } => ctx.var_i64(name) >= *min,
        TransitionCondition::Not { inner } => !eval_condition(inner, ctx),
        TransitionCondition::All { items } => items.iter().all(|c| eval_condition(c, ctx)),
        TransitionCondition::Any { items } => items.iter().any(|c| eval_condition(c, ctx)),
    }
}

/// Full transition graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneTransitionGraph {
    nodes: IndexMap<String, SceneNode>,
    /// from → edges
    edges: IndexMap<String, Vec<SceneEdge>>,
    /// Current node name.
    current: Option<String>,
}

impl SceneTransitionGraph {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert node.
    pub fn add_node(&mut self, node: SceneNode) {
        self.nodes.insert(node.name.clone(), node);
    }

    /// Add edge from `from`.
    pub fn add_edge(
        &mut self,
        from: impl Into<String>,
        edge: SceneEdge,
    ) -> Result<(), TransitionGraphError> {
        let from = from.into();
        if !self.nodes.contains_key(&from) {
            return Err(TransitionGraphError::MissingNode(from));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(TransitionGraphError::MissingNode(edge.to.clone()));
        }
        self.edges.entry(from).or_default().push(edge);
        Ok(())
    }

    /// Set current scene node.
    pub fn set_current(&mut self, name: impl Into<String>) -> Result<(), TransitionGraphError> {
        let name = name.into();
        if !self.nodes.contains_key(&name) {
            return Err(TransitionGraphError::MissingNode(name));
        }
        self.current = Some(name);
        Ok(())
    }

    /// Current node name.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Get node.
    pub fn node(&self, name: &str) -> Option<&SceneNode> {
        self.nodes.get(name)
    }

    /// Outgoing edges from a node.
    pub fn edges_from(&self, name: &str) -> &[SceneEdge] {
        self.edges.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Available transitions from current given context.
    pub fn available(&self, ctx: &dyn TransitionContext) -> Vec<&SceneEdge> {
        let Some(cur) = self.current.as_deref() else {
            return Vec::new();
        };
        self.edges_from(cur)
            .iter()
            .filter(|e| eval_condition(&e.condition, ctx))
            .collect()
    }

    /// Attempt transition to `to` from current.
    pub fn try_transition(
        &mut self,
        to: &str,
        ctx: &dyn TransitionContext,
    ) -> Result<Vec<TransitionEffect>, TransitionGraphError> {
        let from = self
            .current
            .clone()
            .ok_or_else(|| TransitionGraphError::MissingNode("<none>".into()))?;
        let edge = self
            .edges_from(&from)
            .iter()
            .find(|e| e.to == to)
            .cloned()
            .ok_or_else(|| TransitionGraphError::MissingEdge {
                from: from.clone(),
                to: to.into(),
            })?;
        if !eval_condition(&edge.condition, ctx) {
            return Err(TransitionGraphError::Blocked(format!("{} -> {}", from, to)));
        }
        self.current = Some(to.to_string());
        Ok(edge.effects)
    }

    /// BFS shortest path by edge cost (ignoring conditions).
    pub fn path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to {
            return Some(vec![from.to_string()]);
        }
        use std::collections::{HashMap, HashSet, VecDeque};
        let mut q = VecDeque::new();
        let mut came: HashMap<String, String> = HashMap::new();
        let mut seen = HashSet::new();
        q.push_back(from.to_string());
        seen.insert(from.to_string());
        while let Some(cur) = q.pop_front() {
            for edge in self.edges_from(&cur) {
                if seen.insert(edge.to.clone()) {
                    came.insert(edge.to.clone(), cur.clone());
                    if edge.to == to {
                        let mut path = vec![to.to_string()];
                        let mut c = to.to_string();
                        while let Some(prev) = came.get(&c) {
                            path.push(prev.clone());
                            c = prev.clone();
                            if c == from {
                                break;
                            }
                        }
                        path.reverse();
                        return Some(path);
                    }
                    q.push_back(edge.to.clone());
                }
            }
        }
        None
    }

    /// Node count.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> SceneTransitionGraph {
        let mut g = SceneTransitionGraph::new();
        g.add_node(SceneNode::new("town").with_tag("hub"));
        g.add_node(SceneNode::new("forest"));
        g.add_node(SceneNode::new("castle"));
        g.add_edge("town", SceneEdge::new("forest").with_label("north gate"))
            .unwrap();
        g.add_edge(
            "forest",
            SceneEdge::new("castle")
                .with_condition(TransitionCondition::Flag {
                    name: "has_key".into(),
                })
                .with_effect(TransitionEffect::PlayFx { id: "fade".into() }),
        )
        .unwrap();
        g.add_edge("castle", SceneEdge::new("town")).unwrap();
        g.set_current("town").unwrap();
        g
    }

    #[test]
    fn path_exists() {
        let g = sample_graph();
        let p = g.path("town", "castle").unwrap();
        assert_eq!(p, vec!["town", "forest", "castle"]);
    }

    #[test]
    fn blocked_without_flag() {
        let mut g = sample_graph();
        g.set_current("forest").unwrap();
        let ctx = MapTransitionContext::default();
        assert!(g.try_transition("castle", &ctx).is_err());
        let mut ctx = MapTransitionContext::default();
        ctx.flags.insert("has_key".into(), true);
        let fx = g.try_transition("castle", &ctx).unwrap();
        assert_eq!(g.current(), Some("castle"));
        assert!(!fx.is_empty());
    }

    #[test]
    fn available_filters() {
        let mut g = sample_graph();
        g.set_current("forest").unwrap();
        let ctx = MapTransitionContext::default();
        assert!(g.available(&ctx).is_empty());
        let mut ctx = MapTransitionContext::default();
        ctx.flags.insert("has_key".into(), true);
        assert_eq!(g.available(&ctx).len(), 1);
    }

    #[test]
    fn condition_all_any() {
        let ctx = MapTransitionContext {
            flags: {
                let mut m = IndexMap::new();
                m.insert("a".into(), true);
                m
            },
            vars: {
                let mut m = IndexMap::new();
                m.insert("n".into(), 3);
                m
            },
        };
        let cond = TransitionCondition::All {
            items: vec![
                TransitionCondition::Flag { name: "a".into() },
                TransitionCondition::VarAtLeast {
                    name: "n".into(),
                    min: 2,
                },
            ],
        };
        assert!(eval_condition(&cond, &ctx));
    }
}
