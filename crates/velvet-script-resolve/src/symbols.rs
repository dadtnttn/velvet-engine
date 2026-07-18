//! Symbol table for VS2.

#![allow(missing_docs)]

use std::collections::HashMap;
use velvet_script_hir::{HirSpan, HirTy, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Fn, Struct, Enum, Variant, Const, Static, Local, Param,
    TypeAlias, Module, Scene, Character, Screen, StateField, Layer, MsgKey, Trait, Impl,
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
    pub fn new(id: SymbolId, name: impl Into<String>, kind: SymbolKind, module: impl Into<String>) -> Self {
        Self { id, name: name.into(), kind, vis: Visibility::Private, ty: None,
               span: HirSpan::unknown(), module: module.into(), mutable: false }
    }
    pub fn with_vis(mut self, vis: Visibility) -> Self { self.vis = vis; self }
    pub fn with_ty(mut self, ty: HirTy) -> Self { self.ty = Some(ty); self }
    pub fn with_span(mut self, span: HirSpan) -> Self { self.span = span; self }
    pub fn set_mutable(mut self, m: bool) -> Self { self.mutable = m; self }
    pub fn is_type(&self) -> bool {
        matches!(self.kind, SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias | SymbolKind::Trait)
    }
    pub fn is_value(&self) -> bool {
        matches!(self.kind, SymbolKind::Fn | SymbolKind::Const | SymbolKind::Static
            | SymbolKind::Local | SymbolKind::Param | SymbolKind::Scene
            | SymbolKind::Character | SymbolKind::Variant)
    }
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    pub symbols: Vec<Symbol>,
    by_qual: HashMap<String, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self { Self::default() }
    pub fn insert(&mut self, mut sym: Symbol) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        sym.id = id;
        let qual = format!("{}::{}", sym.module, sym.name);
        self.by_qual.insert(qual, id);
        self.symbols.push(sym);
        id
    }
    pub fn get(&self, id: SymbolId) -> Option<&Symbol> { self.symbols.get(id.0 as usize) }
    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> { self.symbols.get_mut(id.0 as usize) }
    pub fn lookup_qual(&self, module: &str, name: &str) -> Option<SymbolId> {
        self.by_qual.get(&format!("{module}::{name}")).copied()
    }
    pub fn len(&self) -> usize { self.symbols.len() }
    pub fn is_empty(&self) -> bool { self.symbols.is_empty() }
    pub fn count_kind(&self, kind: SymbolKind) -> usize {
        self.symbols.iter().filter(|s| s.kind == kind).count()
    }
}

pub fn make_sym_0(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(0), name, SymbolKind::Fn, module)
}

pub fn make_sym_1(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(1), name, SymbolKind::Struct, module)
}

pub fn make_sym_2(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(2), name, SymbolKind::Enum, module)
}

pub fn make_sym_3(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(3), name, SymbolKind::Local, module)
}

pub fn make_sym_4(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(4), name, SymbolKind::Scene, module)
}

pub fn make_sym_5(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(5), name, SymbolKind::Screen, module)
}

pub fn make_sym_6(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(6), name, SymbolKind::Layer, module)
}

pub fn make_sym_7(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(7), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_8(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(8), name, SymbolKind::Const, module)
}

pub fn make_sym_9(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(9), name, SymbolKind::Param, module)
}

pub fn make_sym_10(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(10), name, SymbolKind::Character, module)
}

pub fn make_sym_11(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(11), name, SymbolKind::Fn, module)
}

pub fn make_sym_12(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(12), name, SymbolKind::Struct, module)
}

pub fn make_sym_13(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(13), name, SymbolKind::Enum, module)
}

pub fn make_sym_14(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(14), name, SymbolKind::Local, module)
}

pub fn make_sym_15(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(15), name, SymbolKind::Scene, module)
}

pub fn make_sym_16(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(16), name, SymbolKind::Screen, module)
}

pub fn make_sym_17(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(17), name, SymbolKind::Layer, module)
}

pub fn make_sym_18(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(18), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_19(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(19), name, SymbolKind::Const, module)
}

pub fn make_sym_20(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(20), name, SymbolKind::Param, module)
}

pub fn make_sym_21(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(21), name, SymbolKind::Character, module)
}

pub fn make_sym_22(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(22), name, SymbolKind::Fn, module)
}

pub fn make_sym_23(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(23), name, SymbolKind::Struct, module)
}

pub fn make_sym_24(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(24), name, SymbolKind::Enum, module)
}

pub fn make_sym_25(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(25), name, SymbolKind::Local, module)
}

pub fn make_sym_26(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(26), name, SymbolKind::Scene, module)
}

pub fn make_sym_27(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(27), name, SymbolKind::Screen, module)
}

pub fn make_sym_28(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(28), name, SymbolKind::Layer, module)
}

pub fn make_sym_29(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(29), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_30(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(30), name, SymbolKind::Const, module)
}

pub fn make_sym_31(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(31), name, SymbolKind::Param, module)
}

pub fn make_sym_32(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(32), name, SymbolKind::Character, module)
}

pub fn make_sym_33(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(33), name, SymbolKind::Fn, module)
}

pub fn make_sym_34(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(34), name, SymbolKind::Struct, module)
}

pub fn make_sym_35(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(35), name, SymbolKind::Enum, module)
}

pub fn make_sym_36(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(36), name, SymbolKind::Local, module)
}

pub fn make_sym_37(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(37), name, SymbolKind::Scene, module)
}

pub fn make_sym_38(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(38), name, SymbolKind::Screen, module)
}

pub fn make_sym_39(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(39), name, SymbolKind::Layer, module)
}

pub fn make_sym_40(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(40), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_41(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(41), name, SymbolKind::Const, module)
}

pub fn make_sym_42(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(42), name, SymbolKind::Param, module)
}

pub fn make_sym_43(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(43), name, SymbolKind::Character, module)
}

pub fn make_sym_44(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(44), name, SymbolKind::Fn, module)
}

pub fn make_sym_45(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(45), name, SymbolKind::Struct, module)
}

pub fn make_sym_46(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(46), name, SymbolKind::Enum, module)
}

pub fn make_sym_47(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(47), name, SymbolKind::Local, module)
}

pub fn make_sym_48(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(48), name, SymbolKind::Scene, module)
}

pub fn make_sym_49(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(49), name, SymbolKind::Screen, module)
}

pub fn make_sym_50(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(50), name, SymbolKind::Layer, module)
}

pub fn make_sym_51(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(51), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_52(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(52), name, SymbolKind::Const, module)
}

pub fn make_sym_53(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(53), name, SymbolKind::Param, module)
}

pub fn make_sym_54(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(54), name, SymbolKind::Character, module)
}

pub fn make_sym_55(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(55), name, SymbolKind::Fn, module)
}

pub fn make_sym_56(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(56), name, SymbolKind::Struct, module)
}

pub fn make_sym_57(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(57), name, SymbolKind::Enum, module)
}

pub fn make_sym_58(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(58), name, SymbolKind::Local, module)
}

pub fn make_sym_59(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(59), name, SymbolKind::Scene, module)
}

pub fn make_sym_60(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(60), name, SymbolKind::Screen, module)
}

pub fn make_sym_61(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(61), name, SymbolKind::Layer, module)
}

pub fn make_sym_62(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(62), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_63(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(63), name, SymbolKind::Const, module)
}

pub fn make_sym_64(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(64), name, SymbolKind::Param, module)
}

pub fn make_sym_65(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(65), name, SymbolKind::Character, module)
}

pub fn make_sym_66(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(66), name, SymbolKind::Fn, module)
}

pub fn make_sym_67(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(67), name, SymbolKind::Struct, module)
}

pub fn make_sym_68(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(68), name, SymbolKind::Enum, module)
}

pub fn make_sym_69(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(69), name, SymbolKind::Local, module)
}

pub fn make_sym_70(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(70), name, SymbolKind::Scene, module)
}

pub fn make_sym_71(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(71), name, SymbolKind::Screen, module)
}

pub fn make_sym_72(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(72), name, SymbolKind::Layer, module)
}

pub fn make_sym_73(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(73), name, SymbolKind::MsgKey, module)
}

pub fn make_sym_74(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(74), name, SymbolKind::Const, module)
}

pub fn make_sym_75(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(75), name, SymbolKind::Param, module)
}

pub fn make_sym_76(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(76), name, SymbolKind::Character, module)
}

pub fn make_sym_77(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(77), name, SymbolKind::Fn, module)
}

pub fn make_sym_78(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(78), name, SymbolKind::Struct, module)
}

pub fn make_sym_79(name: &str, module: &str) -> Symbol {
    Symbol::new(SymbolId(79), name, SymbolKind::Enum, module)
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

