//! Narrative graph: scenes as nodes, jumps/calls/decisions as edges.
//!
//! Built from [`NarrativeDocument`] or raw source; validates reachability,
//! missing targets, and cycles.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::narrative::{NarrativeBlock, NarrativeDocument};

/// Kind of graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphNodeKind {
    /// Playable scene.
    Scene,
    /// Ending marker (scene that ends the story).
    Ending,
}

/// Node in the narrative graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Scene / node id.
    pub id: String,
    /// Kind.
    pub kind: GraphNodeKind,
    /// Optional layout position for editors (x, y).
    pub position: (f32, f32),
    /// Freeform comment/color tag.
    pub tag: Option<String>,
}

/// Kind of edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphEdgeKind {
    /// Unconditional jump.
    Jump,
    /// Call (subroutine-style).
    Call,
    /// Choice arm.
    Choice,
    /// Conditional branch.
    Condition,
}

/// Directed edge between scenes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source scene.
    pub from: String,
    /// Target scene.
    pub to: String,
    /// Edge kind.
    pub kind: GraphEdgeKind,
    /// Optional label (choice text / condition).
    pub label: Option<String>,
}

/// Full narrative graph.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NarrativeGraph {
    /// Nodes keyed by id (ordered list for stable iteration).
    pub nodes: Vec<GraphNode>,
    /// Edges.
    pub edges: Vec<GraphEdge>,
    /// Entry scene name if known.
    pub entry: Option<String>,
}

/// Validation report.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphValidation {
    /// Error / warning messages.
    pub issues: Vec<String>,
    /// Scenes never reached from entry.
    pub unreachable: Vec<String>,
    /// Scenes that participate in a cycle.
    pub cyclic: Vec<String>,
    /// Jump targets that do not exist.
    pub missing_targets: Vec<String>,
}

impl GraphValidation {
    /// True when no issues.
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty() && self.missing_targets.is_empty()
    }
}

impl NarrativeGraph {
    /// Build graph from a narrative document.
    pub fn from_narrative(doc: &NarrativeDocument) -> Self {
        let mut g = NarrativeGraph::default();
        if let Some(first) = doc.scenes.first() {
            g.entry = Some(first.name.clone());
        }
        for (i, scene) in doc.scenes.iter().enumerate() {
            let kind = if scene_has_ending(&scene.blocks) {
                GraphNodeKind::Ending
            } else {
                GraphNodeKind::Scene
            };
            g.nodes.push(GraphNode {
                id: scene.name.clone(),
                kind,
                position: ((i as f32) * 180.0, (i % 3) as f32 * 120.0),
                tag: None,
            });
            collect_edges_from_blocks(&scene.name, &scene.blocks, &mut g.edges);
        }
        g
    }

    /// Build from Velvet Script source (via narrative parse).
    pub fn from_source(source: &str) -> Result<Self, crate::narrative::NarrativeError> {
        let doc = NarrativeDocument::from_source(source)?;
        Ok(Self::from_narrative(&doc))
    }

    /// Move a node (editor layout only).
    pub fn move_node(&mut self, id: &str, x: f32, y: f32) -> bool {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            n.position = (x, y);
            true
        } else {
            false
        }
    }

    /// Add an explicit jump edge (does not rewrite source by itself).
    pub fn connect(&mut self, from: &str, to: &str, kind: GraphEdgeKind, label: Option<String>) {
        self.edges.push(GraphEdge {
            from: from.into(),
            to: to.into(),
            kind,
            label,
        });
    }

    /// Validate graph structure.
    pub fn validate(&self) -> GraphValidation {
        let mut report = GraphValidation::default();
        let ids: HashSet<_> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        for e in &self.edges {
            if !ids.contains(e.to.as_str()) {
                report.missing_targets.push(e.to.clone());
                report
                    .issues
                    .push(format!("edge {} -> {}: missing target", e.from, e.to));
            }
            if !ids.contains(e.from.as_str()) {
                report
                    .issues
                    .push(format!("edge {} -> {}: missing source", e.from, e.to));
            }
        }

        // Reachability from entry
        if let Some(entry) = &self.entry {
            let mut seen = HashSet::new();
            let mut q = VecDeque::new();
            q.push_back(entry.as_str());
            seen.insert(entry.as_str());
            while let Some(u) = q.pop_front() {
                for e in self.edges.iter().filter(|e| e.from == u) {
                    if seen.insert(e.to.as_str()) {
                        q.push_back(e.to.as_str());
                    }
                }
            }
            for n in &self.nodes {
                if !seen.contains(n.id.as_str()) {
                    report.unreachable.push(n.id.clone());
                    report.issues.push(format!("unreachable scene `{}`", n.id));
                }
            }
        }

        // Simple cycle detection (nodes on a back-edge in DFS)
        report.cyclic = find_cycle_nodes(self);
        for c in &report.cyclic {
            report.issues.push(format!("cycle involves scene `{c}`"));
        }

        report
    }

    /// Node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Edge count.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

fn scene_has_ending(blocks: &[NarrativeBlock]) -> bool {
    blocks
        .iter()
        .any(|b| matches!(b, NarrativeBlock::Ending { .. }))
}

fn collect_edges_from_blocks(from: &str, blocks: &[NarrativeBlock], edges: &mut Vec<GraphEdge>) {
    for b in blocks {
        match b {
            NarrativeBlock::Jump { target } => edges.push(GraphEdge {
                from: from.into(),
                to: target.clone(),
                kind: GraphEdgeKind::Jump,
                label: None,
            }),
            NarrativeBlock::Call { target } => edges.push(GraphEdge {
                from: from.into(),
                to: target.clone(),
                kind: GraphEdgeKind::Call,
                label: None,
            }),
            NarrativeBlock::Condition {
                cond,
                then_jump,
                else_jump,
            } => {
                edges.push(GraphEdge {
                    from: from.into(),
                    to: then_jump.clone(),
                    kind: GraphEdgeKind::Condition,
                    label: Some(format!("if {cond}")),
                });
                if let Some(e) = else_jump {
                    edges.push(GraphEdge {
                        from: from.into(),
                        to: e.clone(),
                        kind: GraphEdgeKind::Condition,
                        label: Some(format!("else {cond}")),
                    });
                }
            }
            NarrativeBlock::Decision { options } => {
                for arm in options {
                    // Direct jumps in arm body
                    let mut arm_jumps = Vec::new();
                    collect_edges_from_blocks(from, &arm.body, &mut arm_jumps);
                    for mut e in arm_jumps {
                        e.kind = GraphEdgeKind::Choice;
                        e.label = Some(arm.text.clone());
                        edges.push(e);
                    }
                }
            }
            _ => {}
        }
    }
}

fn find_cycle_nodes(g: &NarrativeGraph) -> Vec<String> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in &g.nodes {
        adj.entry(n.id.as_str()).or_default();
    }
    for e in &g.edges {
        adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }
    let mut white: HashSet<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut gray = HashSet::new();
    let mut black = HashSet::new();
    let mut cyclic = HashSet::new();

    fn dfs<'a>(
        u: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        white: &mut HashSet<&'a str>,
        gray: &mut HashSet<&'a str>,
        black: &mut HashSet<&'a str>,
        cyclic: &mut HashSet<&'a str>,
    ) {
        white.remove(u);
        gray.insert(u);
        if let Some(neis) = adj.get(u) {
            for &v in neis {
                if black.contains(v) {
                    continue;
                }
                if gray.contains(v) {
                    cyclic.insert(u);
                    cyclic.insert(v);
                    continue;
                }
                if white.contains(v) {
                    dfs(v, adj, white, gray, black, cyclic);
                }
            }
        }
        gray.remove(u);
        black.insert(u);
    }

    let nodes: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    for n in nodes {
        if white.contains(n) {
            dfs(n, &adj, &mut white, &mut gray, &mut black, &mut cyclic);
        }
    }
    cyclic.into_iter().map(str::to_string).collect()
}

/// Apply a jump from scene A to B into a narrative document (adds Jump block).
pub fn apply_graph_jump(doc: &mut NarrativeDocument, from: &str, to: &str) -> bool {
    if let Some(sc) = doc.scene_mut(from) {
        sc.blocks.push(NarrativeBlock::Jump { target: to.into() });
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::{DecisionArm, NarrativeBlock, NarrativeDocument};

    #[test]
    fn graph_from_branching_story() {
        let mut doc = NarrativeDocument::new();
        doc.add_scene("start");
        doc.add_scene("good");
        doc.add_scene("bad");
        doc.scene_mut("start")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Decision {
                options: vec![
                    DecisionArm {
                        text: "Yes".into(),
                        body: vec![NarrativeBlock::Jump {
                            target: "good".into(),
                        }],
                    },
                    DecisionArm {
                        text: "No".into(),
                        body: vec![NarrativeBlock::Jump {
                            target: "bad".into(),
                        }],
                    },
                ],
            });
        doc.scene_mut("good")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Ending {
                id: Some("g".into()),
            });
        doc.scene_mut("bad")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Ending {
                id: Some("b".into()),
            });

        let g = NarrativeGraph::from_narrative(&doc);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
        let v = g.validate();
        assert!(v.missing_targets.is_empty());
        assert!(v.unreachable.is_empty(), "{:?}", v.unreachable);
    }

    #[test]
    fn detects_missing_and_unreachable() {
        let mut doc = NarrativeDocument::new();
        doc.add_scene("a");
        doc.add_scene("orphan");
        doc.scene_mut("a")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Jump {
                target: "missing".into(),
            });
        let g = NarrativeGraph::from_narrative(&doc);
        let v = g.validate();
        assert!(!v.missing_targets.is_empty());
        assert!(v.unreachable.iter().any(|u| u == "orphan"));
    }

    #[test]
    fn detects_cycle() {
        let mut doc = NarrativeDocument::new();
        doc.add_scene("a");
        doc.add_scene("b");
        doc.scene_mut("a")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Jump { target: "b".into() });
        doc.scene_mut("b")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Jump { target: "a".into() });
        let g = NarrativeGraph::from_narrative(&doc);
        let v = g.validate();
        assert!(!v.cyclic.is_empty(), "expected cycle nodes");
    }

    #[test]
    fn move_and_connect() {
        let mut g = NarrativeGraph::default();
        g.nodes.push(GraphNode {
            id: "a".into(),
            kind: GraphNodeKind::Scene,
            position: (0.0, 0.0),
            tag: None,
        });
        g.nodes.push(GraphNode {
            id: "b".into(),
            kind: GraphNodeKind::Scene,
            position: (0.0, 0.0),
            tag: None,
        });
        assert!(g.move_node("a", 10.0, 20.0));
        g.connect("a", "b", GraphEdgeKind::Jump, None);
        assert_eq!(g.edge_count(), 1);
    }
}
