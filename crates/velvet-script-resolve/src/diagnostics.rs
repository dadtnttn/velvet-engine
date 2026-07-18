//! Resolve diagnostics.

#![allow(missing_docs)]

use velvet_script_hir::HirSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveSeverity { Error, Warning, Note }

#[derive(Debug, Clone)]
pub struct ResolveDiag {
    pub code: &'static str,
    pub severity: ResolveSeverity,
    pub message: String,
    pub span: HirSpan,
    pub module: String,
}

impl ResolveDiag {
    pub fn error(code: &'static str, message: impl Into<String>, span: HirSpan, module: impl Into<String>) -> Self {
        Self { code, severity: ResolveSeverity::Error, message: message.into(), span, module: module.into() }
    }
    pub fn warning(code: &'static str, message: impl Into<String>, span: HirSpan, module: impl Into<String>) -> Self {
        Self { code, severity: ResolveSeverity::Warning, message: message.into(), span, module: module.into() }
    }
    pub fn display(&self) -> String {
        format!("{}:{}: [{}] {}", self.module, self.span.display(), self.code, self.message)
    }
    pub fn is_error(&self) -> bool { matches!(self.severity, ResolveSeverity::Error) }
}

pub const RESOLVE_CODES: &[&str] = &[
    "E0001_unbound",
    "E0002_duplicate",
    "E0003_import_cycle",
    "E0004_private",
    "E0005_not_a_type",
    "E0006_not_a_value",
    "E0007_ambiguous",
    "E0008_bad_path",
    "E0009_missing_mod",
    "E0010_shadow_prelude",
    "E0011_mut_required",
    "E0012_const_assign",
    "E0013_scene_unbound",
    "E0014_layer_unbound",
    "E0015_msg_unbound",
    "E0016_screen_unbound",
    "E0017_character_unbound",
    "E0018_trait_unbound",
    "E0019_impl_orphan",
    "E0020_use_star_empty",
    "E2000_resolve_ext",
    "E2001_resolve_ext",
    "E2002_resolve_ext",
    "E2003_resolve_ext",
    "E2004_resolve_ext",
    "E2005_resolve_ext",
    "E2006_resolve_ext",
    "E2007_resolve_ext",
    "E2008_resolve_ext",
    "E2009_resolve_ext",
    "E2010_resolve_ext",
    "E2011_resolve_ext",
    "E2012_resolve_ext",
    "E2013_resolve_ext",
    "E2014_resolve_ext",
    "E2015_resolve_ext",
    "E2016_resolve_ext",
    "E2017_resolve_ext",
    "E2018_resolve_ext",
    "E2019_resolve_ext",
    "E2020_resolve_ext",
    "E2021_resolve_ext",
    "E2022_resolve_ext",
    "E2023_resolve_ext",
    "E2024_resolve_ext",
    "E2025_resolve_ext",
    "E2026_resolve_ext",
    "E2027_resolve_ext",
    "E2028_resolve_ext",
    "E2029_resolve_ext",
    "E2030_resolve_ext",
    "E2031_resolve_ext",
    "E2032_resolve_ext",
    "E2033_resolve_ext",
    "E2034_resolve_ext",
    "E2035_resolve_ext",
    "E2036_resolve_ext",
    "E2037_resolve_ext",
    "E2038_resolve_ext",
    "E2039_resolve_ext",
    "E2040_resolve_ext",
    "E2041_resolve_ext",
    "E2042_resolve_ext",
    "E2043_resolve_ext",
    "E2044_resolve_ext",
    "E2045_resolve_ext",
    "E2046_resolve_ext",
    "E2047_resolve_ext",
    "E2048_resolve_ext",
    "E2049_resolve_ext",
    "E2050_resolve_ext",
    "E2051_resolve_ext",
    "E2052_resolve_ext",
    "E2053_resolve_ext",
    "E2054_resolve_ext",
    "E2055_resolve_ext",
    "E2056_resolve_ext",
    "E2057_resolve_ext",
    "E2058_resolve_ext",
    "E2059_resolve_ext",
    "E2060_resolve_ext",
    "E2061_resolve_ext",
    "E2062_resolve_ext",
    "E2063_resolve_ext",
    "E2064_resolve_ext",
    "E2065_resolve_ext",
    "E2066_resolve_ext",
    "E2067_resolve_ext",
    "E2068_resolve_ext",
    "E2069_resolve_ext",
    "E2070_resolve_ext",
    "E2071_resolve_ext",
    "E2072_resolve_ext",
    "E2073_resolve_ext",
    "E2074_resolve_ext",
    "E2075_resolve_ext",
    "E2076_resolve_ext",
    "E2077_resolve_ext",
    "E2078_resolve_ext",
    "E2079_resolve_ext",
    "E2080_resolve_ext",
    "E2081_resolve_ext",
    "E2082_resolve_ext",
    "E2083_resolve_ext",
    "E2084_resolve_ext",
    "E2085_resolve_ext",
    "E2086_resolve_ext",
    "E2087_resolve_ext",
    "E2088_resolve_ext",
    "E2089_resolve_ext",
    "E2090_resolve_ext",
    "E2091_resolve_ext",
    "E2092_resolve_ext",
    "E2093_resolve_ext",
    "E2094_resolve_ext",
    "E2095_resolve_ext",
    "E2096_resolve_ext",
    "E2097_resolve_ext",
    "E2098_resolve_ext",
    "E2099_resolve_ext",
    "E2100_resolve_ext",
    "E2101_resolve_ext",
    "E2102_resolve_ext",
    "E2103_resolve_ext",
    "E2104_resolve_ext",
    "E2105_resolve_ext",
    "E2106_resolve_ext",
    "E2107_resolve_ext",
    "E2108_resolve_ext",
    "E2109_resolve_ext",
    "E2110_resolve_ext",
    "E2111_resolve_ext",
    "E2112_resolve_ext",
    "E2113_resolve_ext",
    "E2114_resolve_ext",
    "E2115_resolve_ext",
    "E2116_resolve_ext",
    "E2117_resolve_ext",
    "E2118_resolve_ext",
    "E2119_resolve_ext",
];

pub fn code_known(code: &str) -> bool { RESOLVE_CODES.contains(&code) }

pub fn diag_e0001_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0001_unbound", name, span, module)
}

pub fn diag_e0002_duplicate(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0002_duplicate", name, span, module)
}

pub fn diag_e0003_import_cycle(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0003_import_cycle", name, span, module)
}

pub fn diag_e0004_private(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0004_private", name, span, module)
}

pub fn diag_e0005_not_a_type(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0005_not_a_type", name, span, module)
}

pub fn diag_e0006_not_a_value(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0006_not_a_value", name, span, module)
}

pub fn diag_e0007_ambiguous(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0007_ambiguous", name, span, module)
}

pub fn diag_e0008_bad_path(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0008_bad_path", name, span, module)
}

pub fn diag_e0009_missing_mod(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0009_missing_mod", name, span, module)
}

pub fn diag_e0010_shadow_prelude(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0010_shadow_prelude", name, span, module)
}

pub fn diag_e0011_mut_required(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0011_mut_required", name, span, module)
}

pub fn diag_e0012_const_assign(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0012_const_assign", name, span, module)
}

pub fn diag_e0013_scene_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0013_scene_unbound", name, span, module)
}

pub fn diag_e0014_layer_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0014_layer_unbound", name, span, module)
}

pub fn diag_e0015_msg_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0015_msg_unbound", name, span, module)
}

pub fn diag_e0016_screen_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0016_screen_unbound", name, span, module)
}

pub fn diag_e0017_character_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0017_character_unbound", name, span, module)
}

pub fn diag_e0018_trait_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0018_trait_unbound", name, span, module)
}

pub fn diag_e0019_impl_orphan(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0019_impl_orphan", name, span, module)
}

pub fn diag_e0020_use_star_empty(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0020_use_star_empty", name, span, module)
}

pub fn diag_ext_0(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2000_resolve_ext", msg, span, module)
}

pub fn diag_ext_1(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2001_resolve_ext", msg, span, module)
}

pub fn diag_ext_2(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2002_resolve_ext", msg, span, module)
}

pub fn diag_ext_3(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2003_resolve_ext", msg, span, module)
}

pub fn diag_ext_4(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2004_resolve_ext", msg, span, module)
}

pub fn diag_ext_5(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2005_resolve_ext", msg, span, module)
}

pub fn diag_ext_6(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2006_resolve_ext", msg, span, module)
}

pub fn diag_ext_7(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2007_resolve_ext", msg, span, module)
}

pub fn diag_ext_8(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2008_resolve_ext", msg, span, module)
}

pub fn diag_ext_9(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2009_resolve_ext", msg, span, module)
}

pub fn diag_ext_10(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2010_resolve_ext", msg, span, module)
}

pub fn diag_ext_11(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2011_resolve_ext", msg, span, module)
}

pub fn diag_ext_12(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2012_resolve_ext", msg, span, module)
}

pub fn diag_ext_13(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2013_resolve_ext", msg, span, module)
}

pub fn diag_ext_14(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2014_resolve_ext", msg, span, module)
}

pub fn diag_ext_15(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2015_resolve_ext", msg, span, module)
}

pub fn diag_ext_16(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2016_resolve_ext", msg, span, module)
}

pub fn diag_ext_17(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2017_resolve_ext", msg, span, module)
}

pub fn diag_ext_18(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2018_resolve_ext", msg, span, module)
}

pub fn diag_ext_19(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2019_resolve_ext", msg, span, module)
}

pub fn diag_ext_20(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2020_resolve_ext", msg, span, module)
}

pub fn diag_ext_21(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2021_resolve_ext", msg, span, module)
}

pub fn diag_ext_22(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2022_resolve_ext", msg, span, module)
}

pub fn diag_ext_23(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2023_resolve_ext", msg, span, module)
}

pub fn diag_ext_24(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2024_resolve_ext", msg, span, module)
}

pub fn diag_ext_25(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2025_resolve_ext", msg, span, module)
}

pub fn diag_ext_26(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2026_resolve_ext", msg, span, module)
}

pub fn diag_ext_27(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2027_resolve_ext", msg, span, module)
}

pub fn diag_ext_28(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2028_resolve_ext", msg, span, module)
}

pub fn diag_ext_29(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2029_resolve_ext", msg, span, module)
}

pub fn diag_ext_30(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2030_resolve_ext", msg, span, module)
}

pub fn diag_ext_31(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2031_resolve_ext", msg, span, module)
}

pub fn diag_ext_32(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2032_resolve_ext", msg, span, module)
}

pub fn diag_ext_33(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2033_resolve_ext", msg, span, module)
}

pub fn diag_ext_34(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2034_resolve_ext", msg, span, module)
}

pub fn diag_ext_35(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2035_resolve_ext", msg, span, module)
}

pub fn diag_ext_36(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2036_resolve_ext", msg, span, module)
}

pub fn diag_ext_37(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2037_resolve_ext", msg, span, module)
}

pub fn diag_ext_38(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2038_resolve_ext", msg, span, module)
}

pub fn diag_ext_39(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2039_resolve_ext", msg, span, module)
}

pub fn diag_ext_40(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2040_resolve_ext", msg, span, module)
}

pub fn diag_ext_41(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2041_resolve_ext", msg, span, module)
}

pub fn diag_ext_42(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2042_resolve_ext", msg, span, module)
}

pub fn diag_ext_43(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2043_resolve_ext", msg, span, module)
}

pub fn diag_ext_44(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2044_resolve_ext", msg, span, module)
}

pub fn diag_ext_45(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2045_resolve_ext", msg, span, module)
}

pub fn diag_ext_46(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2046_resolve_ext", msg, span, module)
}

pub fn diag_ext_47(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2047_resolve_ext", msg, span, module)
}

pub fn diag_ext_48(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2048_resolve_ext", msg, span, module)
}

pub fn diag_ext_49(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2049_resolve_ext", msg, span, module)
}

pub fn diag_ext_50(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2050_resolve_ext", msg, span, module)
}

pub fn diag_ext_51(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2051_resolve_ext", msg, span, module)
}

pub fn diag_ext_52(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2052_resolve_ext", msg, span, module)
}

pub fn diag_ext_53(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2053_resolve_ext", msg, span, module)
}

pub fn diag_ext_54(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2054_resolve_ext", msg, span, module)
}

pub fn diag_ext_55(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2055_resolve_ext", msg, span, module)
}

pub fn diag_ext_56(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2056_resolve_ext", msg, span, module)
}

pub fn diag_ext_57(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2057_resolve_ext", msg, span, module)
}

pub fn diag_ext_58(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2058_resolve_ext", msg, span, module)
}

pub fn diag_ext_59(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2059_resolve_ext", msg, span, module)
}

pub fn diag_ext_60(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2060_resolve_ext", msg, span, module)
}

pub fn diag_ext_61(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2061_resolve_ext", msg, span, module)
}

pub fn diag_ext_62(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2062_resolve_ext", msg, span, module)
}

pub fn diag_ext_63(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2063_resolve_ext", msg, span, module)
}

pub fn diag_ext_64(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2064_resolve_ext", msg, span, module)
}

pub fn diag_ext_65(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2065_resolve_ext", msg, span, module)
}

pub fn diag_ext_66(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2066_resolve_ext", msg, span, module)
}

pub fn diag_ext_67(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2067_resolve_ext", msg, span, module)
}

pub fn diag_ext_68(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2068_resolve_ext", msg, span, module)
}

pub fn diag_ext_69(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2069_resolve_ext", msg, span, module)
}

pub fn diag_ext_70(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2070_resolve_ext", msg, span, module)
}

pub fn diag_ext_71(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2071_resolve_ext", msg, span, module)
}

pub fn diag_ext_72(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2072_resolve_ext", msg, span, module)
}

pub fn diag_ext_73(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2073_resolve_ext", msg, span, module)
}

pub fn diag_ext_74(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2074_resolve_ext", msg, span, module)
}

pub fn diag_ext_75(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2075_resolve_ext", msg, span, module)
}

pub fn diag_ext_76(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2076_resolve_ext", msg, span, module)
}

pub fn diag_ext_77(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2077_resolve_ext", msg, span, module)
}

pub fn diag_ext_78(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2078_resolve_ext", msg, span, module)
}

pub fn diag_ext_79(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E2079_resolve_ext", msg, span, module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::HirSpan;
    #[test]
    fn catalog_nonempty() {
        assert!(RESOLVE_CODES.len() > 50);
        assert!(code_known("E0001_unbound"));
    }
    #[test]
    fn display_has_code() {
        let d = diag_e0001_unbound("x", HirSpan::unknown(), "m");
        assert!(d.display().contains("E0001"));
        assert!(d.is_error());
    }
}

