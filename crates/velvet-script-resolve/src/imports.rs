//! Import graph for VS2 modules.

#![allow(missing_docs)]

use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportEdge {
    pub from: String,
    pub to: String,
    pub alias: Option<String>,
    pub glob: bool,
}

#[derive(Debug, Default)]
pub struct ImportGraph {
    pub edges: Vec<ImportEdge>,
    adj: HashMap<String, Vec<usize>>,
}

impl ImportGraph {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, edge: ImportEdge) {
        let i = self.edges.len();
        self.adj.entry(edge.from.clone()).or_default().push(i);
        self.edges.push(edge);
    }
    pub fn imports_of(&self, module: &str) -> Vec<&ImportEdge> {
        self.adj.get(module).into_iter().flatten()
            .filter_map(|&i| self.edges.get(i)).collect()
    }
    pub fn has_cycle(&self) -> bool {
        let mut indeg: HashMap<String, usize> = HashMap::new();
        let mut nodes: HashSet<String> = HashSet::new();
        for e in &self.edges {
            nodes.insert(e.from.clone());
            nodes.insert(e.to.clone());
            *indeg.entry(e.to.clone()).or_default() += 1;
            indeg.entry(e.from.clone()).or_default();
        }
        for n in &nodes { indeg.entry(n.clone()).or_insert(0); }
        let mut q: VecDeque<String> = indeg.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();
        let mut seen = 0usize;
        while let Some(n) = q.pop_front() {
            seen += 1;
            for e in self.imports_of(&n) {
                if let Some(d) = indeg.get_mut(&e.to) {
                    *d = d.saturating_sub(1);
                    if *d == 0 { q.push_back(e.to.clone()); }
                }
            }
        }
        seen < nodes.len()
    }
    pub fn topological(&self) -> Option<Vec<String>> {
        if self.has_cycle() { return None; }
        let mut indeg: HashMap<String, usize> = HashMap::new();
        let mut nodes: HashSet<String> = HashSet::new();
        for e in &self.edges {
            nodes.insert(e.from.clone());
            nodes.insert(e.to.clone());
            *indeg.entry(e.to.clone()).or_default() += 1;
            indeg.entry(e.from.clone()).or_default();
        }
        for n in &nodes { indeg.entry(n.clone()).or_insert(0); }
        let mut q: VecDeque<String> = indeg.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();
        let mut out = Vec::new();
        while let Some(n) = q.pop_front() {
            out.push(n.clone());
            for e in self.imports_of(&n) {
                if let Some(d) = indeg.get_mut(&e.to) {
                    *d = d.saturating_sub(1);
                    if *d == 0 { q.push_back(e.to.clone()); }
                }
            }
        }
        Some(out)
    }
}

pub fn chain_graph_0(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 0;
    g
}

pub fn chain_graph_1(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 1;
    g
}

pub fn chain_graph_2(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 2;
    g
}

pub fn chain_graph_3(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 3;
    g
}

pub fn chain_graph_4(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 4;
    g
}

pub fn chain_graph_5(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 5;
    g
}

pub fn chain_graph_6(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 6;
    g
}

pub fn chain_graph_7(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 7;
    g
}

pub fn chain_graph_8(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 8;
    g
}

pub fn chain_graph_9(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 9;
    g
}

pub fn chain_graph_10(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 10;
    g
}

pub fn chain_graph_11(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 11;
    g
}

pub fn chain_graph_12(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 12;
    g
}

pub fn chain_graph_13(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 13;
    g
}

pub fn chain_graph_14(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 14;
    g
}

pub fn chain_graph_15(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 15;
    g
}

pub fn chain_graph_16(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 16;
    g
}

pub fn chain_graph_17(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 17;
    g
}

pub fn chain_graph_18(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 18;
    g
}

pub fn chain_graph_19(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 19;
    g
}

pub fn chain_graph_20(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 20;
    g
}

pub fn chain_graph_21(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 21;
    g
}

pub fn chain_graph_22(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 22;
    g
}

pub fn chain_graph_23(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 23;
    g
}

pub fn chain_graph_24(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 24;
    g
}

pub fn chain_graph_25(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 25;
    g
}

pub fn chain_graph_26(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 26;
    g
}

pub fn chain_graph_27(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 27;
    g
}

pub fn chain_graph_28(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 28;
    g
}

pub fn chain_graph_29(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 29;
    g
}

pub fn chain_graph_30(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 30;
    g
}

pub fn chain_graph_31(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 31;
    g
}

pub fn chain_graph_32(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 32;
    g
}

pub fn chain_graph_33(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 33;
    g
}

pub fn chain_graph_34(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 34;
    g
}

pub fn chain_graph_35(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 35;
    g
}

pub fn chain_graph_36(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 36;
    g
}

pub fn chain_graph_37(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 37;
    g
}

pub fn chain_graph_38(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 38;
    g
}

pub fn chain_graph_39(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let nlen = len.max(1);
    for i in 0..nlen.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None, glob: false,
        });
    }
    let _ = 39;
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cycle_detected() {
        let mut g = ImportGraph::new();
        g.add(ImportEdge { from: "a".into(), to: "b".into(), alias: None, glob: false });
        g.add(ImportEdge { from: "b".into(), to: "a".into(), alias: None, glob: false });
        assert!(g.has_cycle());
    }
    #[test]
    fn topo_ok() {
        let mut g = ImportGraph::new();
        g.add(ImportEdge { from: "a".into(), to: "b".into(), alias: None, glob: false });
        assert!(!g.has_cycle());
        assert!(g.topological().unwrap().contains(&"a".into()));
    }
}

