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
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, edge: ImportEdge) {
        let i = self.edges.len();
        self.adj.entry(edge.from.clone()).or_default().push(i);
        self.edges.push(edge);
    }
    pub fn imports_of(&self, module: &str) -> Vec<&ImportEdge> {
        self.adj
            .get(module)
            .into_iter()
            .flatten()
            .filter_map(|&i| self.edges.get(i))
            .collect()
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
        for n in &nodes {
            indeg.entry(n.clone()).or_insert(0);
        }
        let mut q: VecDeque<String> = indeg
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut seen = 0usize;
        while let Some(n) = q.pop_front() {
            seen += 1;
            for e in self.imports_of(&n) {
                if let Some(d) = indeg.get_mut(&e.to) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        q.push_back(e.to.clone());
                    }
                }
            }
        }
        seen < nodes.len()
    }
    pub fn topological(&self) -> Option<Vec<String>> {
        if self.has_cycle() {
            return None;
        }
        let mut indeg: HashMap<String, usize> = HashMap::new();
        let mut nodes: HashSet<String> = HashSet::new();
        for e in &self.edges {
            nodes.insert(e.from.clone());
            nodes.insert(e.to.clone());
            *indeg.entry(e.to.clone()).or_default() += 1;
            indeg.entry(e.from.clone()).or_default();
        }
        for n in &nodes {
            indeg.entry(n.clone()).or_insert(0);
        }
        let mut q: VecDeque<String> = indeg
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut out = Vec::new();
        while let Some(n) = q.pop_front() {
            out.push(n.clone());
            for e in self.imports_of(&n) {
                if let Some(d) = indeg.get_mut(&e.to) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        q.push_back(e.to.clone());
                    }
                }
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cycle_detected() {
        let mut g = ImportGraph::new();
        g.add(ImportEdge {
            from: "a".into(),
            to: "b".into(),
            alias: None,
            glob: false,
        });
        g.add(ImportEdge {
            from: "b".into(),
            to: "a".into(),
            alias: None,
            glob: false,
        });
        assert!(g.has_cycle());
    }
    #[test]
    fn topo_ok() {
        let mut g = ImportGraph::new();
        g.add(ImportEdge {
            from: "a".into(),
            to: "b".into(),
            alias: None,
            glob: false,
        });
        assert!(!g.has_cycle());
        assert!(g.topological().unwrap().contains(&"a".into()));
    }
}

/// Build a linear import chain graph of length `len`.
pub fn chain_graph(prefix: &str, len: usize) -> ImportGraph {
    let mut g = ImportGraph::new();
    let n = len.max(1);
    for i in 0..n.saturating_sub(1) {
        g.add(ImportEdge {
            from: format!("{prefix}_{i}"),
            to: format!("{prefix}_{}", i + 1),
            alias: None,
            glob: false,
        });
    }
    g
}
