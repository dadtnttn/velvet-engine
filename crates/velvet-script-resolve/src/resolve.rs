//! Module / workspace resolution driver.

#![allow(missing_docs)]

use velvet_script_hir::{HirItem, HirModule, HirSpan, Visibility};
use crate::diagnostics::{diag_e0001_unbound, diag_e0002_duplicate, diag_e0003_import_cycle, ResolveDiag};
use crate::imports::{ImportEdge, ImportGraph};
use crate::prelude_names::is_prelude;
use crate::scope::{ScopeKind, ScopeTree};
use crate::symbols::{Symbol, SymbolId, SymbolKind, SymbolTable};

#[derive(Debug, Default)]
pub struct ResolveResult {
    pub table: SymbolTable,
    pub scopes: ScopeTree,
    pub imports: ImportGraph,
    pub diags: Vec<ResolveDiag>,
}

impl ResolveResult {
    pub fn ok(&self) -> bool { !self.diags.iter().any(|d| d.is_error()) }
    pub fn error_count(&self) -> usize { self.diags.iter().filter(|d| d.is_error()).count() }
}

fn module_name(m: &HirModule) -> String {
    m.file.clone().unwrap_or_else(|| format!("mod_e{}", m.edition))
}

pub fn resolve_module(m: &HirModule) -> ResolveResult {
    let mut r = ResolveResult::default();
    let mod_name = module_name(m);
    r.scopes.push(ScopeKind::Module, &mod_name);
    for item in &m.items {
        define_item(&mut r, &mod_name, item);
    }
    for item in &m.items {
        if let HirItem::Use { path, .. } = item {
            r.imports.add(ImportEdge {
                from: mod_name.clone(),
                to: path.display(),
                alias: None,
                glob: false,
            });
        }
    }
    if r.imports.has_cycle() {
        r.diags.push(diag_e0003_import_cycle("cycle", HirSpan::unknown(), &mod_name));
    }
    r
}

fn define_item(r: &mut ResolveResult, module: &str, item: &HirItem) {
    match item {
        HirItem::Fn(f) => {
            if r.scopes.resolve(&f.name).is_some() {
                r.diags.push(diag_e0002_duplicate(&f.name, f.span, module));
            }
            let id = r.table.insert(
                Symbol::new(SymbolId(0), f.name.clone(), SymbolKind::Fn, module)
                    .with_vis(f.vis).with_span(f.span)
            );
            r.scopes.define(f.name.clone(), id);
        }
        HirItem::Struct(s) => {
            let id = r.table.insert(
                Symbol::new(SymbolId(0), s.name.clone(), SymbolKind::Struct, module)
                    .with_vis(s.vis).with_span(s.span)
            );
            r.scopes.define(s.name.clone(), id);
        }
        HirItem::Enum(e) => {
            let id = r.table.insert(
                Symbol::new(SymbolId(0), e.name.clone(), SymbolKind::Enum, module)
                    .with_vis(e.vis).with_span(e.span)
            );
            r.scopes.define(e.name.clone(), id);
        }
        HirItem::Scene(sc) => {
            let id = r.table.insert(
                Symbol::new(SymbolId(0), sc.name.clone(), SymbolKind::Scene, module)
                    .with_vis(Visibility::Public).with_span(sc.span)
            );
            r.scopes.define(sc.name.clone(), id);
        }
        HirItem::Character(c) => {
            let id = r.table.insert(
                Symbol::new(SymbolId(0), c.name.clone(), SymbolKind::Character, module)
                    .with_vis(Visibility::Public).with_span(c.span)
            );
            r.scopes.define(c.name.clone(), id);
        }
        HirItem::Screen(s) => {
            let id = r.table.insert(
                Symbol::new(SymbolId(0), s.name.clone(), SymbolKind::Screen, module)
                    .with_vis(Visibility::Public).with_span(s.span)
            );
            r.scopes.define(s.name.clone(), id);
        }
        HirItem::Mod { name, items, .. } => {
            let id = r.table.insert(
                Symbol::new(SymbolId(0), name.clone(), SymbolKind::Module, module)
                    .with_vis(Visibility::Public)
            );
            r.scopes.define(name.clone(), id);
            let child = format!("{module}::{name}");
            r.scopes.push(ScopeKind::Module, &child);
            for it in items { define_item(r, &child, it); }
            r.scopes.pop();
        }
        HirItem::State { fields, span } => {
            for f in fields {
                let id = r.table.insert(
                    Symbol::new(SymbolId(0), f.name.clone(), SymbolKind::StateField, module)
                        .with_span(*span)
                );
                r.scopes.define(f.name.clone(), id);
            }
        }
        HirItem::Use { .. } => {}
    }
}

pub fn resolve_workspace(modules: &[HirModule]) -> ResolveResult {
    let mut r = ResolveResult::default();
    for m in modules {
        let one = resolve_module(m);
        for sym in one.table.symbols { r.table.insert(sym); }
        for e in one.imports.edges { r.imports.add(e); }
        r.diags.extend(one.diags);
    }
    if r.imports.has_cycle() {
        r.diags.push(diag_e0003_import_cycle("workspace cycle", HirSpan::unknown(), "<workspace>"));
    }
    r
}

pub fn check_name(r: &ResolveResult, name: &str, span: HirSpan, module: &str) -> Option<ResolveDiag> {
    if r.scopes.resolve(name).is_some() || is_prelude(name) { return None; }
    if r.table.lookup_qual(module, name).is_some() { return None; }
    Some(diag_e0001_unbound(name, span, module))
}

pub fn resolve_smoke_0(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_1(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_2(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_3(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_4(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_5(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_6(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_7(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_8(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_9(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_10(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_11(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_12(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_13(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_14(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_15(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_16(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_17(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_18(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_19(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_20(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_21(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_22(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_23(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_24(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_25(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_26(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_27(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_28(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_29(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_30(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_31(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_32(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_33(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_34(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_35(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_36(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_37(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_38(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_39(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_40(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_41(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_42(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

pub fn resolve_smoke_43(name: &str) -> bool {
    is_prelude(name) || name.len() > 1
}

pub fn resolve_smoke_44(name: &str) -> bool {
    is_prelude(name) || name.len() > 2
}

pub fn resolve_smoke_45(name: &str) -> bool {
    is_prelude(name) || name.len() > 3
}

pub fn resolve_smoke_46(name: &str) -> bool {
    is_prelude(name) || name.len() > 4
}

pub fn resolve_smoke_47(name: &str) -> bool {
    is_prelude(name) || name.len() > 5
}

pub fn resolve_smoke_48(name: &str) -> bool {
    is_prelude(name) || name.len() > 6
}

pub fn resolve_smoke_49(name: &str) -> bool {
    is_prelude(name) || name.len() > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::{HirExpr, HirFn, HirId, HirItem, HirModule, HirScene, HirSpan, HirTy, PrimTy, Visibility};

    #[test]
    fn define_fn_and_scene() {
        let mut m = HirModule::new(2);
        m.file = Some("game.vel".into());
        m.items.push(HirItem::Fn(HirFn {
            id: HirId(1),
            name: "main".into(),
            vis: Visibility::Public,
            params: vec![],
            ret: HirTy::Prim(PrimTy::Unit),
            body: HirExpr::Block { stmts: vec![], tail: None, span: HirSpan::unknown() },
            span: HirSpan::unknown(),
        }));
        m.items.push(HirItem::Scene(HirScene {
            id: HirId(2),
            name: "start".into(),
            body: vec![],
            span: HirSpan::unknown(),
        }));
        let r = resolve_module(&m);
        assert!(r.ok());
        assert!(r.table.lookup_qual("game.vel", "main").is_some());
        assert!(r.table.lookup_qual("game.vel", "start").is_some());
    }
}

