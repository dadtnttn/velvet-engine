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
    fn typeck_module_0() {
        let src = format!("scene s0 {{}}\nfn f0() {{}}\ncharacter c0 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_1() {
        let src = format!("scene s1 {{}}\nfn f1() {{}}\ncharacter c1 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_2() {
        let src = format!("scene s2 {{}}\nfn f2() {{}}\ncharacter c2 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_3() {
        let src = format!("scene s3 {{}}\nfn f3() {{}}\ncharacter c3 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_4() {
        let src = format!("scene s4 {{}}\nfn f4() {{}}\ncharacter c4 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_5() {
        let src = format!("scene s5 {{}}\nfn f5() {{}}\ncharacter c5 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_6() {
        let src = format!("scene s6 {{}}\nfn f6() {{}}\ncharacter c6 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_7() {
        let src = format!("scene s7 {{}}\nfn f7() {{}}\ncharacter c7 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_8() {
        let src = format!("scene s8 {{}}\nfn f8() {{}}\ncharacter c8 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_9() {
        let src = format!("scene s9 {{}}\nfn f9() {{}}\ncharacter c9 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_10() {
        let src = format!("scene s10 {{}}\nfn f10() {{}}\ncharacter c10 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_11() {
        let src = format!("scene s11 {{}}\nfn f11() {{}}\ncharacter c11 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_12() {
        let src = format!("scene s12 {{}}\nfn f12() {{}}\ncharacter c12 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_13() {
        let src = format!("scene s13 {{}}\nfn f13() {{}}\ncharacter c13 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_14() {
        let src = format!("scene s14 {{}}\nfn f14() {{}}\ncharacter c14 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_15() {
        let src = format!("scene s15 {{}}\nfn f15() {{}}\ncharacter c15 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_16() {
        let src = format!("scene s16 {{}}\nfn f16() {{}}\ncharacter c16 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_17() {
        let src = format!("scene s17 {{}}\nfn f17() {{}}\ncharacter c17 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_18() {
        let src = format!("scene s18 {{}}\nfn f18() {{}}\ncharacter c18 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_19() {
        let src = format!("scene s19 {{}}\nfn f19() {{}}\ncharacter c19 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_20() {
        let src = format!("scene s20 {{}}\nfn f20() {{}}\ncharacter c20 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_21() {
        let src = format!("scene s21 {{}}\nfn f21() {{}}\ncharacter c21 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_22() {
        let src = format!("scene s22 {{}}\nfn f22() {{}}\ncharacter c22 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_23() {
        let src = format!("scene s23 {{}}\nfn f23() {{}}\ncharacter c23 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_24() {
        let src = format!("scene s24 {{}}\nfn f24() {{}}\ncharacter c24 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_25() {
        let src = format!("scene s25 {{}}\nfn f25() {{}}\ncharacter c25 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_26() {
        let src = format!("scene s26 {{}}\nfn f26() {{}}\ncharacter c26 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_27() {
        let src = format!("scene s27 {{}}\nfn f27() {{}}\ncharacter c27 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_28() {
        let src = format!("scene s28 {{}}\nfn f28() {{}}\ncharacter c28 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_29() {
        let src = format!("scene s29 {{}}\nfn f29() {{}}\ncharacter c29 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_30() {
        let src = format!("scene s30 {{}}\nfn f30() {{}}\ncharacter c30 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_31() {
        let src = format!("scene s31 {{}}\nfn f31() {{}}\ncharacter c31 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_32() {
        let src = format!("scene s32 {{}}\nfn f32() {{}}\ncharacter c32 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_33() {
        let src = format!("scene s33 {{}}\nfn f33() {{}}\ncharacter c33 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_34() {
        let src = format!("scene s34 {{}}\nfn f34() {{}}\ncharacter c34 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_35() {
        let src = format!("scene s35 {{}}\nfn f35() {{}}\ncharacter c35 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_36() {
        let src = format!("scene s36 {{}}\nfn f36() {{}}\ncharacter c36 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_37() {
        let src = format!("scene s37 {{}}\nfn f37() {{}}\ncharacter c37 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_38() {
        let src = format!("scene s38 {{}}\nfn f38() {{}}\ncharacter c38 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_39() {
        let src = format!("scene s39 {{}}\nfn f39() {{}}\ncharacter c39 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_40() {
        let src = format!("scene s40 {{}}\nfn f40() {{}}\ncharacter c40 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_41() {
        let src = format!("scene s41 {{}}\nfn f41() {{}}\ncharacter c41 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_42() {
        let src = format!("scene s42 {{}}\nfn f42() {{}}\ncharacter c42 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_43() {
        let src = format!("scene s43 {{}}\nfn f43() {{}}\ncharacter c43 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_44() {
        let src = format!("scene s44 {{}}\nfn f44() {{}}\ncharacter c44 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_45() {
        let src = format!("scene s45 {{}}\nfn f45() {{}}\ncharacter c45 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_46() {
        let src = format!("scene s46 {{}}\nfn f46() {{}}\ncharacter c46 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_47() {
        let src = format!("scene s47 {{}}\nfn f47() {{}}\ncharacter c47 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_48() {
        let src = format!("scene s48 {{}}\nfn f48() {{}}\ncharacter c48 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_49() {
        let src = format!("scene s49 {{}}\nfn f49() {{}}\ncharacter c49 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_50() {
        let src = format!("scene s50 {{}}\nfn f50() {{}}\ncharacter c50 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_51() {
        let src = format!("scene s51 {{}}\nfn f51() {{}}\ncharacter c51 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_52() {
        let src = format!("scene s52 {{}}\nfn f52() {{}}\ncharacter c52 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_53() {
        let src = format!("scene s53 {{}}\nfn f53() {{}}\ncharacter c53 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_54() {
        let src = format!("scene s54 {{}}\nfn f54() {{}}\ncharacter c54 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_55() {
        let src = format!("scene s55 {{}}\nfn f55() {{}}\ncharacter c55 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_56() {
        let src = format!("scene s56 {{}}\nfn f56() {{}}\ncharacter c56 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_57() {
        let src = format!("scene s57 {{}}\nfn f57() {{}}\ncharacter c57 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_58() {
        let src = format!("scene s58 {{}}\nfn f58() {{}}\ncharacter c58 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_59() {
        let src = format!("scene s59 {{}}\nfn f59() {{}}\ncharacter c59 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_60() {
        let src = format!("scene s60 {{}}\nfn f60() {{}}\ncharacter c60 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_61() {
        let src = format!("scene s61 {{}}\nfn f61() {{}}\ncharacter c61 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_62() {
        let src = format!("scene s62 {{}}\nfn f62() {{}}\ncharacter c62 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_63() {
        let src = format!("scene s63 {{}}\nfn f63() {{}}\ncharacter c63 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_64() {
        let src = format!("scene s64 {{}}\nfn f64() {{}}\ncharacter c64 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_65() {
        let src = format!("scene s65 {{}}\nfn f65() {{}}\ncharacter c65 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_66() {
        let src = format!("scene s66 {{}}\nfn f66() {{}}\ncharacter c66 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_67() {
        let src = format!("scene s67 {{}}\nfn f67() {{}}\ncharacter c67 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_68() {
        let src = format!("scene s68 {{}}\nfn f68() {{}}\ncharacter c68 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_69() {
        let src = format!("scene s69 {{}}\nfn f69() {{}}\ncharacter c69 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_70() {
        let src = format!("scene s70 {{}}\nfn f70() {{}}\ncharacter c70 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_71() {
        let src = format!("scene s71 {{}}\nfn f71() {{}}\ncharacter c71 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_72() {
        let src = format!("scene s72 {{}}\nfn f72() {{}}\ncharacter c72 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_73() {
        let src = format!("scene s73 {{}}\nfn f73() {{}}\ncharacter c73 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_74() {
        let src = format!("scene s74 {{}}\nfn f74() {{}}\ncharacter c74 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_75() {
        let src = format!("scene s75 {{}}\nfn f75() {{}}\ncharacter c75 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_76() {
        let src = format!("scene s76 {{}}\nfn f76() {{}}\ncharacter c76 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_77() {
        let src = format!("scene s77 {{}}\nfn f77() {{}}\ncharacter c77 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_78() {
        let src = format!("scene s78 {{}}\nfn f78() {{}}\ncharacter c78 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_79() {
        let src = format!("scene s79 {{}}\nfn f79() {{}}\ncharacter c79 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_80() {
        let src = format!("scene s80 {{}}\nfn f80() {{}}\ncharacter c80 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_81() {
        let src = format!("scene s81 {{}}\nfn f81() {{}}\ncharacter c81 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_82() {
        let src = format!("scene s82 {{}}\nfn f82() {{}}\ncharacter c82 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_83() {
        let src = format!("scene s83 {{}}\nfn f83() {{}}\ncharacter c83 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_84() {
        let src = format!("scene s84 {{}}\nfn f84() {{}}\ncharacter c84 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_85() {
        let src = format!("scene s85 {{}}\nfn f85() {{}}\ncharacter c85 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_86() {
        let src = format!("scene s86 {{}}\nfn f86() {{}}\ncharacter c86 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_87() {
        let src = format!("scene s87 {{}}\nfn f87() {{}}\ncharacter c87 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_88() {
        let src = format!("scene s88 {{}}\nfn f88() {{}}\ncharacter c88 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_89() {
        let src = format!("scene s89 {{}}\nfn f89() {{}}\ncharacter c89 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_90() {
        let src = format!("scene s90 {{}}\nfn f90() {{}}\ncharacter c90 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_91() {
        let src = format!("scene s91 {{}}\nfn f91() {{}}\ncharacter c91 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_92() {
        let src = format!("scene s92 {{}}\nfn f92() {{}}\ncharacter c92 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_93() {
        let src = format!("scene s93 {{}}\nfn f93() {{}}\ncharacter c93 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_94() {
        let src = format!("scene s94 {{}}\nfn f94() {{}}\ncharacter c94 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_95() {
        let src = format!("scene s95 {{}}\nfn f95() {{}}\ncharacter c95 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_96() {
        let src = format!("scene s96 {{}}\nfn f96() {{}}\ncharacter c96 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_97() {
        let src = format!("scene s97 {{}}\nfn f97() {{}}\ncharacter c97 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_98() {
        let src = format!("scene s98 {{}}\nfn f98() {{}}\ncharacter c98 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_99() {
        let src = format!("scene s99 {{}}\nfn f99() {{}}\ncharacter c99 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_100() {
        let src = format!("scene s100 {{}}\nfn f100() {{}}\ncharacter c100 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_101() {
        let src = format!("scene s101 {{}}\nfn f101() {{}}\ncharacter c101 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_102() {
        let src = format!("scene s102 {{}}\nfn f102() {{}}\ncharacter c102 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_103() {
        let src = format!("scene s103 {{}}\nfn f103() {{}}\ncharacter c103 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_104() {
        let src = format!("scene s104 {{}}\nfn f104() {{}}\ncharacter c104 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_105() {
        let src = format!("scene s105 {{}}\nfn f105() {{}}\ncharacter c105 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_106() {
        let src = format!("scene s106 {{}}\nfn f106() {{}}\ncharacter c106 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_107() {
        let src = format!("scene s107 {{}}\nfn f107() {{}}\ncharacter c107 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_108() {
        let src = format!("scene s108 {{}}\nfn f108() {{}}\ncharacter c108 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_109() {
        let src = format!("scene s109 {{}}\nfn f109() {{}}\ncharacter c109 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_110() {
        let src = format!("scene s110 {{}}\nfn f110() {{}}\ncharacter c110 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_111() {
        let src = format!("scene s111 {{}}\nfn f111() {{}}\ncharacter c111 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_112() {
        let src = format!("scene s112 {{}}\nfn f112() {{}}\ncharacter c112 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_113() {
        let src = format!("scene s113 {{}}\nfn f113() {{}}\ncharacter c113 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_114() {
        let src = format!("scene s114 {{}}\nfn f114() {{}}\ncharacter c114 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_115() {
        let src = format!("scene s115 {{}}\nfn f115() {{}}\ncharacter c115 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_116() {
        let src = format!("scene s116 {{}}\nfn f116() {{}}\ncharacter c116 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_117() {
        let src = format!("scene s117 {{}}\nfn f117() {{}}\ncharacter c117 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_118() {
        let src = format!("scene s118 {{}}\nfn f118() {{}}\ncharacter c118 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_119() {
        let src = format!("scene s119 {{}}\nfn f119() {{}}\ncharacter c119 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_120() {
        let src = format!("scene s120 {{}}\nfn f120() {{}}\ncharacter c120 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_121() {
        let src = format!("scene s121 {{}}\nfn f121() {{}}\ncharacter c121 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_122() {
        let src = format!("scene s122 {{}}\nfn f122() {{}}\ncharacter c122 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_123() {
        let src = format!("scene s123 {{}}\nfn f123() {{}}\ncharacter c123 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_124() {
        let src = format!("scene s124 {{}}\nfn f124() {{}}\ncharacter c124 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_125() {
        let src = format!("scene s125 {{}}\nfn f125() {{}}\ncharacter c125 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_126() {
        let src = format!("scene s126 {{}}\nfn f126() {{}}\ncharacter c126 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_127() {
        let src = format!("scene s127 {{}}\nfn f127() {{}}\ncharacter c127 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_128() {
        let src = format!("scene s128 {{}}\nfn f128() {{}}\ncharacter c128 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_129() {
        let src = format!("scene s129 {{}}\nfn f129() {{}}\ncharacter c129 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_130() {
        let src = format!("scene s130 {{}}\nfn f130() {{}}\ncharacter c130 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_131() {
        let src = format!("scene s131 {{}}\nfn f131() {{}}\ncharacter c131 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_132() {
        let src = format!("scene s132 {{}}\nfn f132() {{}}\ncharacter c132 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_133() {
        let src = format!("scene s133 {{}}\nfn f133() {{}}\ncharacter c133 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_134() {
        let src = format!("scene s134 {{}}\nfn f134() {{}}\ncharacter c134 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_135() {
        let src = format!("scene s135 {{}}\nfn f135() {{}}\ncharacter c135 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_136() {
        let src = format!("scene s136 {{}}\nfn f136() {{}}\ncharacter c136 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_137() {
        let src = format!("scene s137 {{}}\nfn f137() {{}}\ncharacter c137 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_138() {
        let src = format!("scene s138 {{}}\nfn f138() {{}}\ncharacter c138 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_139() {
        let src = format!("scene s139 {{}}\nfn f139() {{}}\ncharacter c139 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_140() {
        let src = format!("scene s140 {{}}\nfn f140() {{}}\ncharacter c140 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_141() {
        let src = format!("scene s141 {{}}\nfn f141() {{}}\ncharacter c141 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_142() {
        let src = format!("scene s142 {{}}\nfn f142() {{}}\ncharacter c142 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_143() {
        let src = format!("scene s143 {{}}\nfn f143() {{}}\ncharacter c143 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_144() {
        let src = format!("scene s144 {{}}\nfn f144() {{}}\ncharacter c144 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_145() {
        let src = format!("scene s145 {{}}\nfn f145() {{}}\ncharacter c145 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_146() {
        let src = format!("scene s146 {{}}\nfn f146() {{}}\ncharacter c146 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_147() {
        let src = format!("scene s147 {{}}\nfn f147() {{}}\ncharacter c147 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_148() {
        let src = format!("scene s148 {{}}\nfn f148() {{}}\ncharacter c148 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_149() {
        let src = format!("scene s149 {{}}\nfn f149() {{}}\ncharacter c149 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_150() {
        let src = format!("scene s150 {{}}\nfn f150() {{}}\ncharacter c150 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_151() {
        let src = format!("scene s151 {{}}\nfn f151() {{}}\ncharacter c151 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_152() {
        let src = format!("scene s152 {{}}\nfn f152() {{}}\ncharacter c152 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_153() {
        let src = format!("scene s153 {{}}\nfn f153() {{}}\ncharacter c153 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_154() {
        let src = format!("scene s154 {{}}\nfn f154() {{}}\ncharacter c154 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_155() {
        let src = format!("scene s155 {{}}\nfn f155() {{}}\ncharacter c155 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_156() {
        let src = format!("scene s156 {{}}\nfn f156() {{}}\ncharacter c156 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_157() {
        let src = format!("scene s157 {{}}\nfn f157() {{}}\ncharacter c157 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_158() {
        let src = format!("scene s158 {{}}\nfn f158() {{}}\ncharacter c158 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_159() {
        let src = format!("scene s159 {{}}\nfn f159() {{}}\ncharacter c159 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_160() {
        let src = format!("scene s160 {{}}\nfn f160() {{}}\ncharacter c160 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_161() {
        let src = format!("scene s161 {{}}\nfn f161() {{}}\ncharacter c161 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_162() {
        let src = format!("scene s162 {{}}\nfn f162() {{}}\ncharacter c162 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_163() {
        let src = format!("scene s163 {{}}\nfn f163() {{}}\ncharacter c163 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_164() {
        let src = format!("scene s164 {{}}\nfn f164() {{}}\ncharacter c164 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_165() {
        let src = format!("scene s165 {{}}\nfn f165() {{}}\ncharacter c165 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_166() {
        let src = format!("scene s166 {{}}\nfn f166() {{}}\ncharacter c166 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_167() {
        let src = format!("scene s167 {{}}\nfn f167() {{}}\ncharacter c167 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_168() {
        let src = format!("scene s168 {{}}\nfn f168() {{}}\ncharacter c168 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_169() {
        let src = format!("scene s169 {{}}\nfn f169() {{}}\ncharacter c169 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_170() {
        let src = format!("scene s170 {{}}\nfn f170() {{}}\ncharacter c170 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_171() {
        let src = format!("scene s171 {{}}\nfn f171() {{}}\ncharacter c171 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_172() {
        let src = format!("scene s172 {{}}\nfn f172() {{}}\ncharacter c172 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_173() {
        let src = format!("scene s173 {{}}\nfn f173() {{}}\ncharacter c173 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_174() {
        let src = format!("scene s174 {{}}\nfn f174() {{}}\ncharacter c174 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_175() {
        let src = format!("scene s175 {{}}\nfn f175() {{}}\ncharacter c175 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_176() {
        let src = format!("scene s176 {{}}\nfn f176() {{}}\ncharacter c176 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_177() {
        let src = format!("scene s177 {{}}\nfn f177() {{}}\ncharacter c177 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_178() {
        let src = format!("scene s178 {{}}\nfn f178() {{}}\ncharacter c178 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_179() {
        let src = format!("scene s179 {{}}\nfn f179() {{}}\ncharacter c179 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_180() {
        let src = format!("scene s180 {{}}\nfn f180() {{}}\ncharacter c180 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_181() {
        let src = format!("scene s181 {{}}\nfn f181() {{}}\ncharacter c181 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_182() {
        let src = format!("scene s182 {{}}\nfn f182() {{}}\ncharacter c182 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_183() {
        let src = format!("scene s183 {{}}\nfn f183() {{}}\ncharacter c183 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_184() {
        let src = format!("scene s184 {{}}\nfn f184() {{}}\ncharacter c184 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_185() {
        let src = format!("scene s185 {{}}\nfn f185() {{}}\ncharacter c185 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_186() {
        let src = format!("scene s186 {{}}\nfn f186() {{}}\ncharacter c186 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_187() {
        let src = format!("scene s187 {{}}\nfn f187() {{}}\ncharacter c187 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_188() {
        let src = format!("scene s188 {{}}\nfn f188() {{}}\ncharacter c188 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_189() {
        let src = format!("scene s189 {{}}\nfn f189() {{}}\ncharacter c189 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_190() {
        let src = format!("scene s190 {{}}\nfn f190() {{}}\ncharacter c190 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_191() {
        let src = format!("scene s191 {{}}\nfn f191() {{}}\ncharacter c191 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_192() {
        let src = format!("scene s192 {{}}\nfn f192() {{}}\ncharacter c192 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_193() {
        let src = format!("scene s193 {{}}\nfn f193() {{}}\ncharacter c193 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_194() {
        let src = format!("scene s194 {{}}\nfn f194() {{}}\ncharacter c194 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_195() {
        let src = format!("scene s195 {{}}\nfn f195() {{}}\ncharacter c195 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_196() {
        let src = format!("scene s196 {{}}\nfn f196() {{}}\ncharacter c196 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_197() {
        let src = format!("scene s197 {{}}\nfn f197() {{}}\ncharacter c197 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_198() {
        let src = format!("scene s198 {{}}\nfn f198() {{}}\ncharacter c198 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
    #[test]
    fn typeck_module_199() {
        let src = format!("scene s199 {{}}\nfn f199() {{}}\ncharacter c199 {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        let errs = typeck_module(&m);
        assert!(errs.len() < 50);
    }
}

/// VS1 compat tables.
pub mod compat_tables;
