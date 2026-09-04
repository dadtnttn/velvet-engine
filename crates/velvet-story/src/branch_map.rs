//! Branch map: which endings are reachable, which jumps are broken.
//!
//! Static analysis over [`crate::ir::StoryProgram`]: scenes are nodes,
//! `jump`/`call`/choice bodies are edges, `end` ops are endings.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ir::StoryOp;

/// One directed edge between scenes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchEdge {
    /// From scene.
    pub from: String,
    /// To scene (jump/call target, scene part before `:`).
    pub to: String,
    /// Why: `jump` / `call` / `choice`.
    pub reason: String,
}

/// Analysis result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BranchMap {
    /// All scene names.
    pub scenes: Vec<String>,
    /// Edges.
    pub edges: Vec<BranchEdge>,
    /// Endings `scene[:ending]`.
    pub endings: Vec<String>,
    /// Jump targets with no matching scene.
    pub broken_jumps: Vec<String>,
    /// Scenes unreachable from entry.
    pub unreachable_scenes: Vec<String>,
    /// Endings unreachable from entry.
    pub unreachable_endings: Vec<String>,
}

impl BranchMap {
    /// Analyze a program.
    pub fn analyze(program: &crate::ir::StoryProgram) -> Self {
        let scenes: Vec<String> = program.scenes.keys().cloned().collect();
        let scene_set: BTreeSet<&str> = program.scenes.keys().map(|s| s.as_str()).collect();
        let mut edges = Vec::new();
        let mut endings = Vec::new();
        let mut broken = BTreeSet::new();

        let mut record_jump = |from: &str, target: &str, reason: &str| {
            let scene_part = target.split(':').next().unwrap_or(target).trim();
            if scene_part.is_empty() {
                return;
            }
            edges.push(BranchEdge {
                from: from.to_string(),
                to: scene_part.to_string(),
                reason: reason.to_string(),
            });
            if !scene_set.contains(scene_part) {
                broken.insert(target.to_string());
            }
        };

        fn walk_ops(
            from: &str,
            ops: &[StoryOp],
            record: &mut impl FnMut(&str, &str, &str),
            endings: &mut Vec<String>,
        ) {
            for op in ops {
                match op {
                    StoryOp::Jump { target } => record(from, target, "jump"),
                    StoryOp::Call { target } => record(from, target, "call"),
                    StoryOp::Choice { options } => {
                        for opt in options {
                            for inner in &opt.body {
                                match inner {
                                    StoryOp::Jump { target } => record(from, target, "choice"),
                                    StoryOp::Call { target } => record(from, target, "choice"),
                                    StoryOp::If {
                                        then_ops, else_ops, ..
                                    } => {
                                        walk_ops(from, then_ops, record, endings);
                                        walk_ops(from, else_ops, record, endings);
                                    }
                                    StoryOp::End { ending } => endings.push(format!(
                                        "{}:{}",
                                        from,
                                        ending.clone().unwrap_or_else(|| "end".into())
                                    )),
                                    _ => {}
                                }
                            }
                        }
                    }
                    StoryOp::If {
                        then_ops, else_ops, ..
                    } => {
                        walk_ops(from, then_ops, record, endings);
                        walk_ops(from, else_ops, record, endings);
                    }
                    StoryOp::End { ending } => endings.push(format!(
                        "{}:{}",
                        from,
                        ending.clone().unwrap_or_else(|| "end".into())
                    )),
                    _ => {}
                }
            }
        }

        for (name, scene) in &program.scenes {
            let mut rec = |f: &str, t: &str, r: &str| record_jump(f, t, r);
            walk_ops(name, &scene.ops, &mut rec, &mut endings);
        }

        // reachability from entry
        let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for e in &edges {
            adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut stack = vec![program.entry.as_str()];
        while let Some(cur) = stack.pop() {
            if !seen.insert(cur) {
                continue;
            }
            if let Some(next) = adj.get(cur) {
                for n in next {
                    if !seen.contains(n) {
                        stack.push(n);
                    }
                }
            }
        }
        let unreachable_scenes: Vec<String> = scenes
            .iter()
            .filter(|s| !seen.contains(s.as_str()))
            .cloned()
            .collect();
        let unreachable_endings: Vec<String> = endings
            .iter()
            .filter(|e| {
                let scene = e.split(':').next().unwrap_or("");
                !seen.contains(scene)
            })
            .cloned()
            .collect();

        Self {
            scenes,
            edges,
            endings,
            broken_jumps: broken.into_iter().collect(),
            unreachable_scenes,
            unreachable_endings,
        }
    }

    /// Short human report for CLI / Studio console.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "scenes={} edges={} endings={}\n",
            self.scenes.len(),
            self.edges.len(),
            self.endings.len()
        ));
        if self.broken_jumps.is_empty() {
            out.push_str("broken_jumps: none\n");
        } else {
            out.push_str(&format!("broken_jumps: {}\n", self.broken_jumps.join(", ")));
        }
        if self.unreachable_scenes.is_empty() {
            out.push_str("unreachable_scenes: none\n");
        } else {
            out.push_str(&format!(
                "unreachable_scenes: {}\n",
                self.unreachable_scenes.join(", ")
            ));
        }
        if self.unreachable_endings.is_empty() {
            out.push_str("unreachable_endings: none\n");
        } else {
            out.push_str(&format!(
                "unreachable_endings: {}\n",
                self.unreachable_endings.join(", ")
            ));
        }
        out
    }

    /// Graphviz DOT for Studio Nodes view.
    pub fn to_dot(&self) -> String {
        let mut s = String::from("digraph story {\n");
        for scene in &self.scenes {
            let style = if self.unreachable_scenes.iter().any(|u| u == scene) {
                " [style=dashed color=gray]"
            } else {
                ""
            };
            s.push_str(&format!("  \"{scene}\"{style};\n"));
        }
        for e in &self.edges {
            s.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                e.from, e.to, e.reason
            ));
        }
        s.push_str("}\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;

    fn prog(src: &str) -> crate::ir::StoryProgram {
        load_program_from_source(src, Some("t.vel"), "T").unwrap()
    }

    #[test]
    fn finds_broken_and_unreachable() {
        let src = r#"
scene main {
    jump good
    jump missing_scene
}
scene good {
    end "good_end"
}
scene lost {
    end "lost_end"
}
"#;
        let p = prog(src);
        let m = BranchMap::analyze(&p);
        assert!(m.broken_jumps.iter().any(|b| b.contains("missing_scene")));
        assert!(m.unreachable_scenes.contains(&"lost".to_string()));
        assert!(m.unreachable_endings.iter().any(|e| e.contains("lost_end")));
        assert!(m.to_dot().contains("digraph"));
        assert!(m.to_text().contains("broken_jumps"));
    }

    #[test]
    fn choice_edges_count() {
        let src = r#"
scene main {
    choice {
        "A" { jump a }
        "B" { jump b }
    }
}
scene a { end "ea" }
scene b { end "eb" }
"#;
        let m = BranchMap::analyze(&prog(src));
        assert_eq!(m.broken_jumps.len(), 0);
        assert_eq!(m.endings.len(), 2);
        assert!(m.unreachable_scenes.is_empty());
    }
}
