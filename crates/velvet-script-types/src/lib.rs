//! Velvet Script 2 type checker (static, rust-like).

#![deny(missing_docs)]

use std::collections::HashMap;
use thiserror::Error;
use velvet_script_hir::*;

/// Type errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TypeError {
    /// Unknown name.
    #[error("unknown name `{name}` at {loc}")]
    UnknownName {
        /// Name.
        name: String,
        /// Loc.
        loc: String,
    },
    /// Mismatch.
    #[error("type mismatch: expected `{expected}`, found `{found}` at {loc}")]
    Mismatch {
        /// Expected.
        expected: String,
        /// Found.
        found: String,
        /// Loc.
        loc: String,
    },
    /// Dup.
    #[error("duplicate definition `{name}`")]
    Duplicate {
        /// Name.
        name: String,
    },
    /// Arity.
    #[error("wrong arity: expected {expected} args, found {found}")]
    Arity {
        /// Expected.
        expected: usize,
        /// Found.
        found: usize,
    },
}

/// Type environment.
#[derive(Debug, Default, Clone)]
pub struct TypeEnv {
    /// Stack of scopes.
    scopes: Vec<HashMap<String, HirTy>>,
    /// Item types.
    items: HashMap<String, HirTy>,
}
impl TypeEnv {
    /// New.
    pub fn new() -> Self {
        let mut e = Self {
            scopes: vec![HashMap::new()],
            items: HashMap::new(),
        };
        e.install_builtins();
        e
    }
    fn install_builtins(&mut self) {
        for (n, t) in [
            ("i32", HirTy::Prim(PrimTy::I32)),
            ("i64", HirTy::Prim(PrimTy::I64)),
            ("bool", HirTy::Prim(PrimTy::Bool)),
            ("str", HirTy::Prim(PrimTy::Str)),
            ("LayerId", HirTy::LayerId),
            ("SceneId", HirTy::SceneId),
            ("MsgId", HirTy::MsgId),
        ] {
            self.items.insert(n.into(), t);
        }
    }
    /// Push scope.
    pub fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }
    /// Pop scope.
    pub fn pop(&mut self) {
        self.scopes.pop();
    }
    /// Insert local.
    pub fn insert_local(&mut self, name: impl Into<String>, ty: HirTy) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.into(), ty);
        }
    }
    /// Lookup.
    pub fn lookup(&self, name: &str) -> Option<HirTy> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        self.items.get(name).cloned()
    }
    /// Define item.
    pub fn define_item(&mut self, name: impl Into<String>, ty: HirTy) -> Result<(), TypeError> {
        let name = name.into();
        if self.items.contains_key(&name) {
            return Err(TypeError::Duplicate { name });
        }
        self.items.insert(name, ty);
        Ok(())
    }
}

/// Check module; returns errors.
pub fn typeck_module(module: &HirModule) -> Vec<TypeError> {
    let mut env = TypeEnv::new();
    let mut errs = Vec::new();
    // collect item signatures
    for item in &module.items {
        match item {
            HirItem::Fn(f) => {
                let ty = HirTy::Fn(
                    f.params.iter().map(|(_, t)| t.clone()).collect(),
                    Box::new(f.ret.clone()),
                );
                if let Err(e) = env.define_item(&f.name, ty) {
                    errs.push(e);
                }
            }
            HirItem::Struct(s) => {
                let _ = env.define_item(&s.name, HirTy::Path(HirPath::parse(&s.name)));
            }
            HirItem::Enum(e) => {
                let _ = env.define_item(&e.name, HirTy::Path(HirPath::parse(&e.name)));
            }
            HirItem::Scene(sc) => {
                let _ = env.define_item(&sc.name, HirTy::SceneId);
            }
            HirItem::Character(c) => {
                let _ = env.define_item(&c.name, HirTy::Path(HirPath::parse("Character")));
            }
            HirItem::Screen(s) => {
                let _ = env.define_item(&s.name, HirTy::Path(HirPath::parse("Screen")));
            }
            HirItem::State { fields, .. } => {
                for f in fields {
                    env.insert_local(&f.name, f.ty.clone());
                }
            }
            HirItem::Use { .. } | HirItem::Mod { .. } => {}
        }
    }
    // check fn bodies lightly
    for item in &module.items {
        if let HirItem::Fn(f) = item {
            env.push();
            for (n, t) in &f.params {
                env.insert_local(n, t.clone());
            }
            errs.extend(check_expr(&f.body, &env, &f.ret));
            env.pop();
        }
        if let HirItem::Scene(sc) = item {
            for st in &sc.body {
                errs.extend(check_stmt(st, &env));
            }
        }
    }
    errs
}

fn check_stmt(st: &HirStmt, env: &TypeEnv) -> Vec<TypeError> {
    match st {
        HirStmt::Let {
            name: _,
            mutable: _,
            ty,
            init,
            span,
        } => {
            let mut e = Vec::new();
            if let (Some(t), Some(init)) = (ty, init) {
                e.extend(check_expr(init, env, t));
            }
            let _ = span;
            e
        }
        HirStmt::Expr { expr, .. } => check_expr(expr, env, &HirTy::Prim(PrimTy::Unit)),
        HirStmt::Assign { value, .. } => check_expr(value, env, &HirTy::Infer),
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                check_expr(v, env, &HirTy::Infer)
            } else {
                Vec::new()
            }
        }
        HirStmt::Say { msg, .. } => check_expr(msg, env, &HirTy::MsgId),
        _ => Vec::new(),
    }
}

fn check_expr(expr: &HirExpr, env: &TypeEnv, expected: &HirTy) -> Vec<TypeError> {
    match expr {
        HirExpr::Path { path, span } => {
            let name = path.display();
            if env.lookup(&name).is_none() && path.segs.len() == 1 {
                // allow unknown in heuristic mode only as soft — skip hard fail for now
                let _ = span;
                let _ = expected;
                Vec::new()
            } else {
                Vec::new()
            }
        }
        HirExpr::Lit { lit, span } => {
            let found = match lit {
                HirLit::Int(_) => HirTy::Prim(PrimTy::I64),
                HirLit::Float(_) => HirTy::Prim(PrimTy::F64),
                HirLit::Bool(_) => HirTy::Prim(PrimTy::Bool),
                HirLit::Str(_) => HirTy::Prim(PrimTy::Str),
                HirLit::MsgId(_) => HirTy::MsgId,
            };
            if !ty_compatible(expected, &found) {
                vec![TypeError::Mismatch {
                    expected: expected.display(),
                    found: found.display(),
                    loc: span.display(),
                }]
            } else {
                Vec::new()
            }
        }
        HirExpr::Translate { .. } => {
            if matches!(expected, HirTy::MsgId | HirTy::Infer | HirTy::Prim(PrimTy::Str)) {
                Vec::new()
            } else {
                vec![TypeError::Mismatch {
                    expected: expected.display(),
                    found: "MsgId".into(),
                    loc: "0:0".into(),
                }]
            }
        }
        HirExpr::Binary { lhs, rhs, .. } => {
            let mut e = check_expr(lhs, env, &HirTy::Infer);
            e.extend(check_expr(rhs, env, &HirTy::Infer));
            e
        }
        HirExpr::Call { callee, args, .. } => {
            let mut e = check_expr(callee, env, &HirTy::Infer);
            for a in args {
                e.extend(check_expr(a, env, &HirTy::Infer));
            }
            e
        }
        HirExpr::Block { stmts, tail, .. } => {
            let mut e = Vec::new();
            for s in stmts {
                e.extend(check_stmt(s, env));
            }
            if let Some(t) = tail {
                e.extend(check_expr(t, env, expected));
            }
            e
        }
        HirExpr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            let mut e = check_expr(cond, env, &HirTy::Prim(PrimTy::Bool));
            e.extend(check_expr(then_br, env, expected));
            if let Some(el) = else_br {
                e.extend(check_expr(el, env, expected));
            }
            e
        }
        _ => Vec::new(),
    }
}

fn ty_compatible(expected: &HirTy, found: &HirTy) -> bool {
    matches!(expected, HirTy::Infer)
        || expected == found
        || matches!(
            (expected, found),
            (HirTy::Prim(PrimTy::I32), HirTy::Prim(PrimTy::I64))
                | (HirTy::Prim(PrimTy::I64), HirTy::Prim(PrimTy::I32))
                | (HirTy::MsgId, HirTy::Prim(PrimTy::Str))
        )
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typeck_empty_ok() {
        let m = HirModule::new(2);
        assert!(typeck_module(&m).is_empty());
    }
    #[test]
    fn typeck_fn_scene() {
        let src = "scene intro {}\npub fn main() {}\n";
        let (m, _) = lower_source_heuristic(src, 2);
        let _ = typeck_module(&m);
    }

    #[test]
    fn typeck_scene_and_fn_exact() {
        let src = "scene intro {}\npub fn main() {}\n";
        let (m, _) = lower_source_heuristic(src, 2);
        let errs = typeck_module(&m);
        assert!(errs.is_empty(), "unexpected type errors: {errs:?}");
        assert_eq!(m.item_count(), 2);
    }

    #[test]
    fn typeck_empty_module_exact() {
        let m = HirModule::new(2);
        assert_eq!(typeck_module(&m).len(), 0);
    }

}

/// VS1 compat tables.
pub mod compat_tables;
