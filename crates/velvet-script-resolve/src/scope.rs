//! Lexical scopes for VS2 resolution.

#![allow(missing_docs)]

use std::collections::HashMap;
use crate::symbols::SymbolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Module, Function, Block, Scene, Screen, Impl, MatchArm, Loop,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub id: ScopeId,
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub names: HashMap<String, SymbolId>,
    pub module_path: String,
}

impl Scope {
    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>, module_path: impl Into<String>) -> Self {
        Self { id, kind, parent, names: HashMap::new(), module_path: module_path.into() }
    }
    pub fn define(&mut self, name: impl Into<String>, sym: SymbolId) -> Option<SymbolId> {
        self.names.insert(name.into(), sym)
    }
    pub fn lookup_local(&self, name: &str) -> Option<SymbolId> { self.names.get(name).copied() }
    pub fn is_function(&self) -> bool { matches!(self.kind, ScopeKind::Function) }
    pub fn is_module(&self) -> bool { matches!(self.kind, ScopeKind::Module) }
}

#[derive(Debug, Default)]
pub struct ScopeTree {
    pub scopes: Vec<Scope>,
    pub current: Option<ScopeId>,
}

impl ScopeTree {
    pub fn new() -> Self { Self::default() }
    pub fn push(&mut self, kind: ScopeKind, module_path: &str) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        let parent = self.current;
        self.scopes.push(Scope::new(id, kind, parent, module_path));
        self.current = Some(id);
        id
    }
    pub fn pop(&mut self) {
        if let Some(cur) = self.current {
            self.current = self.scopes[cur.0 as usize].parent;
        }
    }
    pub fn define(&mut self, name: impl Into<String>, sym: SymbolId) {
        if let Some(cur) = self.current {
            self.scopes[cur.0 as usize].define(name, sym);
        }
    }
    pub fn resolve(&self, name: &str) -> Option<SymbolId> {
        let mut cur = self.current;
        while let Some(id) = cur {
            let sc = &self.scopes[id.0 as usize];
            if let Some(s) = sc.lookup_local(name) { return Some(s); }
            cur = sc.parent;
        }
        None
    }
    pub fn get(&self, id: ScopeId) -> Option<&Scope> { self.scopes.get(id.0 as usize) }
    pub fn len(&self) -> usize { self.scopes.len() }
    pub fn is_empty(&self) -> bool { self.scopes.is_empty() }
}



















































#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::SymbolId;
    #[test]
    fn nested_resolve() {
        let mut t = ScopeTree::new();
        t.push(ScopeKind::Module, "game");
        t.define("x", SymbolId(1));
        t.push(ScopeKind::Function, "game");
        t.define("y", SymbolId(2));
        assert_eq!(t.resolve("y"), Some(SymbolId(2)));
        assert_eq!(t.resolve("x"), Some(SymbolId(1)));
        t.pop();
        assert_eq!(t.resolve("y"), None);
    }
}


pub fn scope_kind_label(k: ScopeKind) -> &'static str {
    match k {
        ScopeKind::Module => "module",
        ScopeKind::Function => "function",
        ScopeKind::Block => "block",
        ScopeKind::Scene => "scene",
        ScopeKind::Screen => "screen",
        ScopeKind::Impl => "impl",
        ScopeKind::MatchArm => "match_arm",
        ScopeKind::Loop => "loop",
    }
}
