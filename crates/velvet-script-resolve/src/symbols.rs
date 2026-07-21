//! Symbol table for VS2.

#![allow(missing_docs)]

use std::collections::HashMap;
use velvet_script_hir::{HirSpan, HirTy, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,
    Struct,
    Enum,
    Variant,
    Const,
    Static,
    Local,
    Param,
    TypeAlias,
    Module,
    Scene,
    Character,
    Screen,
    StateField,
    Layer,
    MsgKey,
    Trait,
    Impl,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub vis: Visibility,
    pub ty: Option<HirTy>,
    pub span: HirSpan,
    pub module: String,
    pub mutable: bool,
}

impl Symbol {
    pub fn new(
        id: SymbolId,
        name: impl Into<String>,
        kind: SymbolKind,
        module: impl Into<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            vis: Visibility::Private,
            ty: None,
            span: HirSpan::unknown(),
            module: module.into(),
            mutable: false,
        }
    }
    pub fn with_vis(mut self, vis: Visibility) -> Self {
        self.vis = vis;
        self
    }
    pub fn with_ty(mut self, ty: HirTy) -> Self {
        self.ty = Some(ty);
        self
    }
    pub fn with_span(mut self, span: HirSpan) -> Self {
        self.span = span;
        self
    }
    pub fn set_mutable(mut self, m: bool) -> Self {
        self.mutable = m;
        self
    }
    pub fn is_type(&self) -> bool {
        matches!(
            self.kind,
            SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias | SymbolKind::Trait
        )
    }
    pub fn is_value(&self) -> bool {
        matches!(
            self.kind,
            SymbolKind::Fn
                | SymbolKind::Const
                | SymbolKind::Static
                | SymbolKind::Local
                | SymbolKind::Param
                | SymbolKind::Scene
                | SymbolKind::Character
                | SymbolKind::Variant
        )
    }
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
    by_qual: HashMap<String, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, mut sym: Symbol) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        sym.id = id;
        let qual = format!("{}::{}", sym.module, sym.name);
        self.by_qual.insert(qual, id);
        self.symbols.push(sym);
        id
    }
    pub fn get(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.0 as usize)
    }
    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(id.0 as usize)
    }
    pub fn lookup_qual(&self, module: &str, name: &str) -> Option<SymbolId> {
        self.by_qual.get(&format!("{module}::{name}")).copied()
    }
    pub fn len(&self) -> usize {
        self.symbols.len()
    }
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
    pub fn count_kind(&self, kind: SymbolKind) -> usize {
        self.symbols.iter().filter(|s| s.kind == kind).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insert_lookup() {
        let mut t = SymbolTable::new();
        let id = t.insert(Symbol::new(SymbolId(0), "foo", SymbolKind::Fn, "m"));
        assert_eq!(t.lookup_qual("m", "foo"), Some(id));
        assert_eq!(t.count_kind(SymbolKind::Fn), 1);
    }
}

/// Construct a symbol (replaces numbered make_sym_N clones).
pub fn make_sym(name: &str, module: &str, kind: SymbolKind) -> Symbol {
    Symbol::new(SymbolId(0), name, kind, module)
}
