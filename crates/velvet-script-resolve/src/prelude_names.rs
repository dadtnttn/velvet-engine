//! Prelude names visible without import in VS2 edition 2.

#![allow(missing_docs)]

#[derive(Debug, Clone, Copy)]
pub struct PreludeEntry {
    pub name: &'static str,
    pub kind: &'static str,
    pub ty_hint: &'static str,
}

pub static PRELUDE: &[PreludeEntry] = &[
    PreludeEntry { name: "print", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "abs", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "min", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "max", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "floor", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "ceil", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "clamp", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "len", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "concat", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "str", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "Some", kind: "variant", ty_hint: "variant" },
    PreludeEntry { name: "None", kind: "variant", ty_hint: "variant" },
    PreludeEntry { name: "Ok", kind: "variant", ty_hint: "variant" },
    PreludeEntry { name: "Err", kind: "variant", ty_hint: "variant" },
    PreludeEntry { name: "Option", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "Result", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "Vec", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "String", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "LayerId", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "SceneId", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "MsgId", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "ScriptError", kind: "type", ty_hint: "type" },
    PreludeEntry { name: "push_layer", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "pop_layer", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "show_layer", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "hide_layer", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "t", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "say", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "jump", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "call_scene", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_0", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_1", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_2", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_3", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_4", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_5", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_6", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_7", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_8", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_9", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_10", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_11", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_12", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_13", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_14", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_15", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_16", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_17", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_18", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_19", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_20", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_21", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_22", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_23", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_24", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_25", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_26", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_27", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_28", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_29", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_30", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_31", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_32", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_33", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_34", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_35", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_36", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_37", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_38", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_39", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_40", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_41", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_42", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_43", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_44", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_45", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_46", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_47", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_48", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_49", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_50", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_51", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_52", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_53", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_54", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_55", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_56", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_57", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_58", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_59", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_60", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_61", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_62", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_63", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_64", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_65", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_66", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_67", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_68", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_69", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_70", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_71", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_72", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_73", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_74", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_75", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_76", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_77", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_78", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_79", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_80", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_81", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_82", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_83", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_84", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_85", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_86", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_87", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_88", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_89", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_90", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_91", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_92", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_93", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_94", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_95", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_96", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_97", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_98", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_99", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_100", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_101", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_102", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_103", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_104", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_105", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_106", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_107", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_108", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_109", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_110", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_111", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_112", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_113", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_114", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_115", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_116", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_117", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_118", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_119", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_120", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_121", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_122", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_123", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_124", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_125", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_126", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_127", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_128", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_129", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_130", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_131", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_132", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_133", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_134", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_135", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_136", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_137", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_138", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_139", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_140", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_141", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_142", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_143", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_144", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_145", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_146", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_147", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_148", kind: "fn", ty_hint: "fn" },
    PreludeEntry { name: "prelude_ext_149", kind: "fn", ty_hint: "fn" },
];

pub fn is_prelude(name: &str) -> bool { PRELUDE.iter().any(|e| e.name == name) }
pub fn prelude_ty(name: &str) -> Option<&'static str> { PRELUDE.iter().find(|e| e.name == name).map(|e| e.ty_hint) }
pub fn prelude_kind(name: &str) -> Option<&'static str> { PRELUDE.iter().find(|e| e.name == name).map(|e| e.kind) }

pub fn prelude_batch_0(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_1(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_2(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_3(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_4(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_5(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_6(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_7(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_8(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_9(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_10(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_11(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_12(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_13(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_14(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_15(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_16(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_17(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_18(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_19(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_20(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_21(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_22(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_23(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_24(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_25(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_26(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_27(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_28(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_29(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_30(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_31(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_32(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_33(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_34(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_35(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_36(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_37(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_38(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_39(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_40(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_41(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_42(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_43(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_44(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_45(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_46(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_47(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_48(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

pub fn prelude_batch_49(names: &[&str]) -> usize {
    names.iter().filter(|n| is_prelude(n)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn core_prelude() {
        assert!(is_prelude("print"));
        assert!(is_prelude("LayerId"));
        assert!(is_prelude("MsgId"));
        assert!(!is_prelude("not_a_real_name_xyz"));
    }
}

