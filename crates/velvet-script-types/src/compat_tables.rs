//! Compatibility tables mapping VS1 names to VS2 types.

#![allow(missing_docs)]

/// VS1 keyword aliases.
pub static VS1_ALIASES: &[(&str, &str)] = &[
    ("alias_0", "target_0"),
    ("alias_1", "target_1"),
    ("alias_2", "target_2"),
    ("alias_3", "target_3"),
    ("alias_4", "target_4"),
    ("alias_5", "target_5"),
    ("alias_6", "target_6"),
    ("alias_7", "target_7"),
    ("alias_8", "target_8"),
    ("alias_9", "target_9"),
    ("alias_10", "target_10"),
    ("alias_11", "target_11"),
    ("alias_12", "target_12"),
    ("alias_13", "target_13"),
    ("alias_14", "target_14"),
    ("alias_15", "target_15"),
    ("alias_16", "target_16"),
    ("alias_17", "target_17"),
    ("alias_18", "target_18"),
    ("alias_19", "target_19"),
    ("alias_20", "target_20"),
    ("alias_21", "target_21"),
    ("alias_22", "target_22"),
    ("alias_23", "target_23"),
    ("alias_24", "target_24"),
    ("alias_25", "target_25"),
    ("alias_26", "target_26"),
    ("alias_27", "target_27"),
    ("alias_28", "target_28"),
    ("alias_29", "target_29"),
    ("alias_30", "target_30"),
    ("alias_31", "target_31"),
    ("alias_32", "target_32"),
    ("alias_33", "target_33"),
    ("alias_34", "target_34"),
    ("alias_35", "target_35"),
    ("alias_36", "target_36"),
    ("alias_37", "target_37"),
    ("alias_38", "target_38"),
    ("alias_39", "target_39"),
    ("alias_40", "target_40"),
    ("alias_41", "target_41"),
    ("alias_42", "target_42"),
    ("alias_43", "target_43"),
    ("alias_44", "target_44"),
    ("alias_45", "target_45"),
    ("alias_46", "target_46"),
    ("alias_47", "target_47"),
    ("alias_48", "target_48"),
    ("alias_49", "target_49"),
    ("alias_50", "target_50"),
    ("alias_51", "target_51"),
    ("alias_52", "target_52"),
    ("alias_53", "target_53"),
    ("alias_54", "target_54"),
    ("alias_55", "target_55"),
    ("alias_56", "target_56"),
    ("alias_57", "target_57"),
    ("alias_58", "target_58"),
    ("alias_59", "target_59"),
    ("alias_60", "target_60"),
    ("alias_61", "target_61"),
    ("alias_62", "target_62"),
    ("alias_63", "target_63"),
    ("alias_64", "target_64"),
    ("alias_65", "target_65"),
    ("alias_66", "target_66"),
    ("alias_67", "target_67"),
    ("alias_68", "target_68"),
    ("alias_69", "target_69"),
    ("alias_70", "target_70"),
    ("alias_71", "target_71"),
    ("alias_72", "target_72"),
    ("alias_73", "target_73"),
    ("alias_74", "target_74"),
    ("alias_75", "target_75"),
    ("alias_76", "target_76"),
    ("alias_77", "target_77"),
    ("alias_78", "target_78"),
    ("alias_79", "target_79"),
    ("alias_80", "target_80"),
    ("alias_81", "target_81"),
    ("alias_82", "target_82"),
    ("alias_83", "target_83"),
    ("alias_84", "target_84"),
    ("alias_85", "target_85"),
    ("alias_86", "target_86"),
    ("alias_87", "target_87"),
    ("alias_88", "target_88"),
    ("alias_89", "target_89"),
    ("alias_90", "target_90"),
    ("alias_91", "target_91"),
    ("alias_92", "target_92"),
    ("alias_93", "target_93"),
    ("alias_94", "target_94"),
    ("alias_95", "target_95"),
    ("alias_96", "target_96"),
    ("alias_97", "target_97"),
    ("alias_98", "target_98"),
    ("alias_99", "target_99"),
    ("alias_100", "target_100"),
    ("alias_101", "target_101"),
    ("alias_102", "target_102"),
    ("alias_103", "target_103"),
    ("alias_104", "target_104"),
    ("alias_105", "target_105"),
    ("alias_106", "target_106"),
    ("alias_107", "target_107"),
    ("alias_108", "target_108"),
    ("alias_109", "target_109"),
    ("alias_110", "target_110"),
    ("alias_111", "target_111"),
    ("alias_112", "target_112"),
    ("alias_113", "target_113"),
    ("alias_114", "target_114"),
    ("alias_115", "target_115"),
    ("alias_116", "target_116"),
    ("alias_117", "target_117"),
    ("alias_118", "target_118"),
    ("alias_119", "target_119"),
    ("alias_120", "target_120"),
    ("alias_121", "target_121"),
    ("alias_122", "target_122"),
    ("alias_123", "target_123"),
    ("alias_124", "target_124"),
    ("alias_125", "target_125"),
    ("alias_126", "target_126"),
    ("alias_127", "target_127"),
    ("alias_128", "target_128"),
    ("alias_129", "target_129"),
    ("alias_130", "target_130"),
    ("alias_131", "target_131"),
    ("alias_132", "target_132"),
    ("alias_133", "target_133"),
    ("alias_134", "target_134"),
    ("alias_135", "target_135"),
    ("alias_136", "target_136"),
    ("alias_137", "target_137"),
    ("alias_138", "target_138"),
    ("alias_139", "target_139"),
    ("alias_140", "target_140"),
    ("alias_141", "target_141"),
    ("alias_142", "target_142"),
    ("alias_143", "target_143"),
    ("alias_144", "target_144"),
    ("alias_145", "target_145"),
    ("alias_146", "target_146"),
    ("alias_147", "target_147"),
    ("alias_148", "target_148"),
    ("alias_149", "target_149"),
    ("alias_150", "target_150"),
    ("alias_151", "target_151"),
    ("alias_152", "target_152"),
    ("alias_153", "target_153"),
    ("alias_154", "target_154"),
    ("alias_155", "target_155"),
    ("alias_156", "target_156"),
    ("alias_157", "target_157"),
    ("alias_158", "target_158"),
    ("alias_159", "target_159"),
    ("alias_160", "target_160"),
    ("alias_161", "target_161"),
    ("alias_162", "target_162"),
    ("alias_163", "target_163"),
    ("alias_164", "target_164"),
    ("alias_165", "target_165"),
    ("alias_166", "target_166"),
    ("alias_167", "target_167"),
    ("alias_168", "target_168"),
    ("alias_169", "target_169"),
    ("alias_170", "target_170"),
    ("alias_171", "target_171"),
    ("alias_172", "target_172"),
    ("alias_173", "target_173"),
    ("alias_174", "target_174"),
    ("alias_175", "target_175"),
    ("alias_176", "target_176"),
    ("alias_177", "target_177"),
    ("alias_178", "target_178"),
    ("alias_179", "target_179"),
    ("alias_180", "target_180"),
    ("alias_181", "target_181"),
    ("alias_182", "target_182"),
    ("alias_183", "target_183"),
    ("alias_184", "target_184"),
    ("alias_185", "target_185"),
    ("alias_186", "target_186"),
    ("alias_187", "target_187"),
    ("alias_188", "target_188"),
    ("alias_189", "target_189"),
    ("alias_190", "target_190"),
    ("alias_191", "target_191"),
    ("alias_192", "target_192"),
    ("alias_193", "target_193"),
    ("alias_194", "target_194"),
    ("alias_195", "target_195"),
    ("alias_196", "target_196"),
    ("alias_197", "target_197"),
    ("alias_198", "target_198"),
    ("alias_199", "target_199"),
    ("alias_200", "target_200"),
    ("alias_201", "target_201"),
    ("alias_202", "target_202"),
    ("alias_203", "target_203"),
    ("alias_204", "target_204"),
    ("alias_205", "target_205"),
    ("alias_206", "target_206"),
    ("alias_207", "target_207"),
    ("alias_208", "target_208"),
    ("alias_209", "target_209"),
    ("alias_210", "target_210"),
    ("alias_211", "target_211"),
    ("alias_212", "target_212"),
    ("alias_213", "target_213"),
    ("alias_214", "target_214"),
    ("alias_215", "target_215"),
    ("alias_216", "target_216"),
    ("alias_217", "target_217"),
    ("alias_218", "target_218"),
    ("alias_219", "target_219"),
    ("alias_220", "target_220"),
    ("alias_221", "target_221"),
    ("alias_222", "target_222"),
    ("alias_223", "target_223"),
    ("alias_224", "target_224"),
    ("alias_225", "target_225"),
    ("alias_226", "target_226"),
    ("alias_227", "target_227"),
    ("alias_228", "target_228"),
    ("alias_229", "target_229"),
    ("alias_230", "target_230"),
    ("alias_231", "target_231"),
    ("alias_232", "target_232"),
    ("alias_233", "target_233"),
    ("alias_234", "target_234"),
    ("alias_235", "target_235"),
    ("alias_236", "target_236"),
    ("alias_237", "target_237"),
    ("alias_238", "target_238"),
    ("alias_239", "target_239"),
    ("alias_240", "target_240"),
    ("alias_241", "target_241"),
    ("alias_242", "target_242"),
    ("alias_243", "target_243"),
    ("alias_244", "target_244"),
    ("alias_245", "target_245"),
    ("alias_246", "target_246"),
    ("alias_247", "target_247"),
    ("alias_248", "target_248"),
    ("alias_249", "target_249"),
    ("alias_250", "target_250"),
    ("alias_251", "target_251"),
    ("alias_252", "target_252"),
    ("alias_253", "target_253"),
    ("alias_254", "target_254"),
    ("alias_255", "target_255"),
    ("alias_256", "target_256"),
    ("alias_257", "target_257"),
    ("alias_258", "target_258"),
    ("alias_259", "target_259"),
    ("alias_260", "target_260"),
    ("alias_261", "target_261"),
    ("alias_262", "target_262"),
    ("alias_263", "target_263"),
    ("alias_264", "target_264"),
    ("alias_265", "target_265"),
    ("alias_266", "target_266"),
    ("alias_267", "target_267"),
    ("alias_268", "target_268"),
    ("alias_269", "target_269"),
    ("alias_270", "target_270"),
    ("alias_271", "target_271"),
    ("alias_272", "target_272"),
    ("alias_273", "target_273"),
    ("alias_274", "target_274"),
    ("alias_275", "target_275"),
    ("alias_276", "target_276"),
    ("alias_277", "target_277"),
    ("alias_278", "target_278"),
    ("alias_279", "target_279"),
    ("alias_280", "target_280"),
    ("alias_281", "target_281"),
    ("alias_282", "target_282"),
    ("alias_283", "target_283"),
    ("alias_284", "target_284"),
    ("alias_285", "target_285"),
    ("alias_286", "target_286"),
    ("alias_287", "target_287"),
    ("alias_288", "target_288"),
    ("alias_289", "target_289"),
    ("alias_290", "target_290"),
    ("alias_291", "target_291"),
    ("alias_292", "target_292"),
    ("alias_293", "target_293"),
    ("alias_294", "target_294"),
    ("alias_295", "target_295"),
    ("alias_296", "target_296"),
    ("alias_297", "target_297"),
    ("alias_298", "target_298"),
    ("alias_299", "target_299"),
];

/// Lookup alias.
pub fn resolve_alias(name: &str) -> Option<&'static str> {
    VS1_ALIASES.iter().find(|(a, _)| *a == name).map(|(_, t)| *t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alias_0() {
        assert_eq!(resolve_alias("alias_0"), Some("target_0"));
    }
    #[test]
    fn alias_1() {
        assert_eq!(resolve_alias("alias_1"), Some("target_1"));
    }
    #[test]
    fn alias_2() {
        assert_eq!(resolve_alias("alias_2"), Some("target_2"));
    }
    #[test]
    fn alias_3() {
        assert_eq!(resolve_alias("alias_3"), Some("target_3"));
    }
    #[test]
    fn alias_4() {
        assert_eq!(resolve_alias("alias_4"), Some("target_4"));
    }
    #[test]
    fn alias_5() {
        assert_eq!(resolve_alias("alias_5"), Some("target_5"));
    }
    #[test]
    fn alias_6() {
        assert_eq!(resolve_alias("alias_6"), Some("target_6"));
    }
    #[test]
    fn alias_7() {
        assert_eq!(resolve_alias("alias_7"), Some("target_7"));
    }
    #[test]
    fn alias_8() {
        assert_eq!(resolve_alias("alias_8"), Some("target_8"));
    }
    #[test]
    fn alias_9() {
        assert_eq!(resolve_alias("alias_9"), Some("target_9"));
    }
    #[test]
    fn alias_10() {
        assert_eq!(resolve_alias("alias_10"), Some("target_10"));
    }
    #[test]
    fn alias_11() {
        assert_eq!(resolve_alias("alias_11"), Some("target_11"));
    }
    #[test]
    fn alias_12() {
        assert_eq!(resolve_alias("alias_12"), Some("target_12"));
    }
    #[test]
    fn alias_13() {
        assert_eq!(resolve_alias("alias_13"), Some("target_13"));
    }
    #[test]
    fn alias_14() {
        assert_eq!(resolve_alias("alias_14"), Some("target_14"));
    }
    #[test]
    fn alias_15() {
        assert_eq!(resolve_alias("alias_15"), Some("target_15"));
    }
    #[test]
    fn alias_16() {
        assert_eq!(resolve_alias("alias_16"), Some("target_16"));
    }
    #[test]
    fn alias_17() {
        assert_eq!(resolve_alias("alias_17"), Some("target_17"));
    }
    #[test]
    fn alias_18() {
        assert_eq!(resolve_alias("alias_18"), Some("target_18"));
    }
    #[test]
    fn alias_19() {
        assert_eq!(resolve_alias("alias_19"), Some("target_19"));
    }
    #[test]
    fn alias_20() {
        assert_eq!(resolve_alias("alias_20"), Some("target_20"));
    }
    #[test]
    fn alias_21() {
        assert_eq!(resolve_alias("alias_21"), Some("target_21"));
    }
    #[test]
    fn alias_22() {
        assert_eq!(resolve_alias("alias_22"), Some("target_22"));
    }
    #[test]
    fn alias_23() {
        assert_eq!(resolve_alias("alias_23"), Some("target_23"));
    }
    #[test]
    fn alias_24() {
        assert_eq!(resolve_alias("alias_24"), Some("target_24"));
    }
    #[test]
    fn alias_25() {
        assert_eq!(resolve_alias("alias_25"), Some("target_25"));
    }
    #[test]
    fn alias_26() {
        assert_eq!(resolve_alias("alias_26"), Some("target_26"));
    }
    #[test]
    fn alias_27() {
        assert_eq!(resolve_alias("alias_27"), Some("target_27"));
    }
    #[test]
    fn alias_28() {
        assert_eq!(resolve_alias("alias_28"), Some("target_28"));
    }
    #[test]
    fn alias_29() {
        assert_eq!(resolve_alias("alias_29"), Some("target_29"));
    }
    #[test]
    fn alias_30() {
        assert_eq!(resolve_alias("alias_30"), Some("target_30"));
    }
    #[test]
    fn alias_31() {
        assert_eq!(resolve_alias("alias_31"), Some("target_31"));
    }
    #[test]
    fn alias_32() {
        assert_eq!(resolve_alias("alias_32"), Some("target_32"));
    }
    #[test]
    fn alias_33() {
        assert_eq!(resolve_alias("alias_33"), Some("target_33"));
    }
    #[test]
    fn alias_34() {
        assert_eq!(resolve_alias("alias_34"), Some("target_34"));
    }
    #[test]
    fn alias_35() {
        assert_eq!(resolve_alias("alias_35"), Some("target_35"));
    }
    #[test]
    fn alias_36() {
        assert_eq!(resolve_alias("alias_36"), Some("target_36"));
    }
    #[test]
    fn alias_37() {
        assert_eq!(resolve_alias("alias_37"), Some("target_37"));
    }
    #[test]
    fn alias_38() {
        assert_eq!(resolve_alias("alias_38"), Some("target_38"));
    }
    #[test]
    fn alias_39() {
        assert_eq!(resolve_alias("alias_39"), Some("target_39"));
    }
    #[test]
    fn alias_40() {
        assert_eq!(resolve_alias("alias_40"), Some("target_40"));
    }
    #[test]
    fn alias_41() {
        assert_eq!(resolve_alias("alias_41"), Some("target_41"));
    }
    #[test]
    fn alias_42() {
        assert_eq!(resolve_alias("alias_42"), Some("target_42"));
    }
    #[test]
    fn alias_43() {
        assert_eq!(resolve_alias("alias_43"), Some("target_43"));
    }
    #[test]
    fn alias_44() {
        assert_eq!(resolve_alias("alias_44"), Some("target_44"));
    }
    #[test]
    fn alias_45() {
        assert_eq!(resolve_alias("alias_45"), Some("target_45"));
    }
    #[test]
    fn alias_46() {
        assert_eq!(resolve_alias("alias_46"), Some("target_46"));
    }
    #[test]
    fn alias_47() {
        assert_eq!(resolve_alias("alias_47"), Some("target_47"));
    }
    #[test]
    fn alias_48() {
        assert_eq!(resolve_alias("alias_48"), Some("target_48"));
    }
    #[test]
    fn alias_49() {
        assert_eq!(resolve_alias("alias_49"), Some("target_49"));
    }
    #[test]
    fn alias_50() {
        assert_eq!(resolve_alias("alias_50"), Some("target_50"));
    }
    #[test]
    fn alias_51() {
        assert_eq!(resolve_alias("alias_51"), Some("target_51"));
    }
    #[test]
    fn alias_52() {
        assert_eq!(resolve_alias("alias_52"), Some("target_52"));
    }
    #[test]
    fn alias_53() {
        assert_eq!(resolve_alias("alias_53"), Some("target_53"));
    }
    #[test]
    fn alias_54() {
        assert_eq!(resolve_alias("alias_54"), Some("target_54"));
    }
    #[test]
    fn alias_55() {
        assert_eq!(resolve_alias("alias_55"), Some("target_55"));
    }
    #[test]
    fn alias_56() {
        assert_eq!(resolve_alias("alias_56"), Some("target_56"));
    }
    #[test]
    fn alias_57() {
        assert_eq!(resolve_alias("alias_57"), Some("target_57"));
    }
    #[test]
    fn alias_58() {
        assert_eq!(resolve_alias("alias_58"), Some("target_58"));
    }
    #[test]
    fn alias_59() {
        assert_eq!(resolve_alias("alias_59"), Some("target_59"));
    }
    #[test]
    fn alias_60() {
        assert_eq!(resolve_alias("alias_60"), Some("target_60"));
    }
    #[test]
    fn alias_61() {
        assert_eq!(resolve_alias("alias_61"), Some("target_61"));
    }
    #[test]
    fn alias_62() {
        assert_eq!(resolve_alias("alias_62"), Some("target_62"));
    }
    #[test]
    fn alias_63() {
        assert_eq!(resolve_alias("alias_63"), Some("target_63"));
    }
    #[test]
    fn alias_64() {
        assert_eq!(resolve_alias("alias_64"), Some("target_64"));
    }
    #[test]
    fn alias_65() {
        assert_eq!(resolve_alias("alias_65"), Some("target_65"));
    }
    #[test]
    fn alias_66() {
        assert_eq!(resolve_alias("alias_66"), Some("target_66"));
    }
    #[test]
    fn alias_67() {
        assert_eq!(resolve_alias("alias_67"), Some("target_67"));
    }
    #[test]
    fn alias_68() {
        assert_eq!(resolve_alias("alias_68"), Some("target_68"));
    }
    #[test]
    fn alias_69() {
        assert_eq!(resolve_alias("alias_69"), Some("target_69"));
    }
    #[test]
    fn alias_70() {
        assert_eq!(resolve_alias("alias_70"), Some("target_70"));
    }
    #[test]
    fn alias_71() {
        assert_eq!(resolve_alias("alias_71"), Some("target_71"));
    }
    #[test]
    fn alias_72() {
        assert_eq!(resolve_alias("alias_72"), Some("target_72"));
    }
    #[test]
    fn alias_73() {
        assert_eq!(resolve_alias("alias_73"), Some("target_73"));
    }
    #[test]
    fn alias_74() {
        assert_eq!(resolve_alias("alias_74"), Some("target_74"));
    }
    #[test]
    fn alias_75() {
        assert_eq!(resolve_alias("alias_75"), Some("target_75"));
    }
    #[test]
    fn alias_76() {
        assert_eq!(resolve_alias("alias_76"), Some("target_76"));
    }
    #[test]
    fn alias_77() {
        assert_eq!(resolve_alias("alias_77"), Some("target_77"));
    }
    #[test]
    fn alias_78() {
        assert_eq!(resolve_alias("alias_78"), Some("target_78"));
    }
    #[test]
    fn alias_79() {
        assert_eq!(resolve_alias("alias_79"), Some("target_79"));
    }
    #[test]
    fn alias_80() {
        assert_eq!(resolve_alias("alias_80"), Some("target_80"));
    }
    #[test]
    fn alias_81() {
        assert_eq!(resolve_alias("alias_81"), Some("target_81"));
    }
    #[test]
    fn alias_82() {
        assert_eq!(resolve_alias("alias_82"), Some("target_82"));
    }
    #[test]
    fn alias_83() {
        assert_eq!(resolve_alias("alias_83"), Some("target_83"));
    }
    #[test]
    fn alias_84() {
        assert_eq!(resolve_alias("alias_84"), Some("target_84"));
    }
    #[test]
    fn alias_85() {
        assert_eq!(resolve_alias("alias_85"), Some("target_85"));
    }
    #[test]
    fn alias_86() {
        assert_eq!(resolve_alias("alias_86"), Some("target_86"));
    }
    #[test]
    fn alias_87() {
        assert_eq!(resolve_alias("alias_87"), Some("target_87"));
    }
    #[test]
    fn alias_88() {
        assert_eq!(resolve_alias("alias_88"), Some("target_88"));
    }
    #[test]
    fn alias_89() {
        assert_eq!(resolve_alias("alias_89"), Some("target_89"));
    }
    #[test]
    fn alias_90() {
        assert_eq!(resolve_alias("alias_90"), Some("target_90"));
    }
    #[test]
    fn alias_91() {
        assert_eq!(resolve_alias("alias_91"), Some("target_91"));
    }
    #[test]
    fn alias_92() {
        assert_eq!(resolve_alias("alias_92"), Some("target_92"));
    }
    #[test]
    fn alias_93() {
        assert_eq!(resolve_alias("alias_93"), Some("target_93"));
    }
    #[test]
    fn alias_94() {
        assert_eq!(resolve_alias("alias_94"), Some("target_94"));
    }
    #[test]
    fn alias_95() {
        assert_eq!(resolve_alias("alias_95"), Some("target_95"));
    }
    #[test]
    fn alias_96() {
        assert_eq!(resolve_alias("alias_96"), Some("target_96"));
    }
    #[test]
    fn alias_97() {
        assert_eq!(resolve_alias("alias_97"), Some("target_97"));
    }
    #[test]
    fn alias_98() {
        assert_eq!(resolve_alias("alias_98"), Some("target_98"));
    }
    #[test]
    fn alias_99() {
        assert_eq!(resolve_alias("alias_99"), Some("target_99"));
    }
    #[test]
    fn alias_100() {
        assert_eq!(resolve_alias("alias_100"), Some("target_100"));
    }
    #[test]
    fn alias_101() {
        assert_eq!(resolve_alias("alias_101"), Some("target_101"));
    }
    #[test]
    fn alias_102() {
        assert_eq!(resolve_alias("alias_102"), Some("target_102"));
    }
    #[test]
    fn alias_103() {
        assert_eq!(resolve_alias("alias_103"), Some("target_103"));
    }
    #[test]
    fn alias_104() {
        assert_eq!(resolve_alias("alias_104"), Some("target_104"));
    }
    #[test]
    fn alias_105() {
        assert_eq!(resolve_alias("alias_105"), Some("target_105"));
    }
    #[test]
    fn alias_106() {
        assert_eq!(resolve_alias("alias_106"), Some("target_106"));
    }
    #[test]
    fn alias_107() {
        assert_eq!(resolve_alias("alias_107"), Some("target_107"));
    }
    #[test]
    fn alias_108() {
        assert_eq!(resolve_alias("alias_108"), Some("target_108"));
    }
    #[test]
    fn alias_109() {
        assert_eq!(resolve_alias("alias_109"), Some("target_109"));
    }
    #[test]
    fn alias_110() {
        assert_eq!(resolve_alias("alias_110"), Some("target_110"));
    }
    #[test]
    fn alias_111() {
        assert_eq!(resolve_alias("alias_111"), Some("target_111"));
    }
    #[test]
    fn alias_112() {
        assert_eq!(resolve_alias("alias_112"), Some("target_112"));
    }
    #[test]
    fn alias_113() {
        assert_eq!(resolve_alias("alias_113"), Some("target_113"));
    }
    #[test]
    fn alias_114() {
        assert_eq!(resolve_alias("alias_114"), Some("target_114"));
    }
    #[test]
    fn alias_115() {
        assert_eq!(resolve_alias("alias_115"), Some("target_115"));
    }
    #[test]
    fn alias_116() {
        assert_eq!(resolve_alias("alias_116"), Some("target_116"));
    }
    #[test]
    fn alias_117() {
        assert_eq!(resolve_alias("alias_117"), Some("target_117"));
    }
    #[test]
    fn alias_118() {
        assert_eq!(resolve_alias("alias_118"), Some("target_118"));
    }
    #[test]
    fn alias_119() {
        assert_eq!(resolve_alias("alias_119"), Some("target_119"));
    }
    #[test]
    fn alias_120() {
        assert_eq!(resolve_alias("alias_120"), Some("target_120"));
    }
    #[test]
    fn alias_121() {
        assert_eq!(resolve_alias("alias_121"), Some("target_121"));
    }
    #[test]
    fn alias_122() {
        assert_eq!(resolve_alias("alias_122"), Some("target_122"));
    }
    #[test]
    fn alias_123() {
        assert_eq!(resolve_alias("alias_123"), Some("target_123"));
    }
    #[test]
    fn alias_124() {
        assert_eq!(resolve_alias("alias_124"), Some("target_124"));
    }
    #[test]
    fn alias_125() {
        assert_eq!(resolve_alias("alias_125"), Some("target_125"));
    }
    #[test]
    fn alias_126() {
        assert_eq!(resolve_alias("alias_126"), Some("target_126"));
    }
    #[test]
    fn alias_127() {
        assert_eq!(resolve_alias("alias_127"), Some("target_127"));
    }
    #[test]
    fn alias_128() {
        assert_eq!(resolve_alias("alias_128"), Some("target_128"));
    }
    #[test]
    fn alias_129() {
        assert_eq!(resolve_alias("alias_129"), Some("target_129"));
    }
    #[test]
    fn alias_130() {
        assert_eq!(resolve_alias("alias_130"), Some("target_130"));
    }
    #[test]
    fn alias_131() {
        assert_eq!(resolve_alias("alias_131"), Some("target_131"));
    }
    #[test]
    fn alias_132() {
        assert_eq!(resolve_alias("alias_132"), Some("target_132"));
    }
    #[test]
    fn alias_133() {
        assert_eq!(resolve_alias("alias_133"), Some("target_133"));
    }
    #[test]
    fn alias_134() {
        assert_eq!(resolve_alias("alias_134"), Some("target_134"));
    }
    #[test]
    fn alias_135() {
        assert_eq!(resolve_alias("alias_135"), Some("target_135"));
    }
    #[test]
    fn alias_136() {
        assert_eq!(resolve_alias("alias_136"), Some("target_136"));
    }
    #[test]
    fn alias_137() {
        assert_eq!(resolve_alias("alias_137"), Some("target_137"));
    }
    #[test]
    fn alias_138() {
        assert_eq!(resolve_alias("alias_138"), Some("target_138"));
    }
    #[test]
    fn alias_139() {
        assert_eq!(resolve_alias("alias_139"), Some("target_139"));
    }
    #[test]
    fn alias_140() {
        assert_eq!(resolve_alias("alias_140"), Some("target_140"));
    }
    #[test]
    fn alias_141() {
        assert_eq!(resolve_alias("alias_141"), Some("target_141"));
    }
    #[test]
    fn alias_142() {
        assert_eq!(resolve_alias("alias_142"), Some("target_142"));
    }
    #[test]
    fn alias_143() {
        assert_eq!(resolve_alias("alias_143"), Some("target_143"));
    }
    #[test]
    fn alias_144() {
        assert_eq!(resolve_alias("alias_144"), Some("target_144"));
    }
    #[test]
    fn alias_145() {
        assert_eq!(resolve_alias("alias_145"), Some("target_145"));
    }
    #[test]
    fn alias_146() {
        assert_eq!(resolve_alias("alias_146"), Some("target_146"));
    }
    #[test]
    fn alias_147() {
        assert_eq!(resolve_alias("alias_147"), Some("target_147"));
    }
    #[test]
    fn alias_148() {
        assert_eq!(resolve_alias("alias_148"), Some("target_148"));
    }
    #[test]
    fn alias_149() {
        assert_eq!(resolve_alias("alias_149"), Some("target_149"));
    }
    #[test]
    fn alias_150() {
        assert_eq!(resolve_alias("alias_150"), Some("target_150"));
    }
    #[test]
    fn alias_151() {
        assert_eq!(resolve_alias("alias_151"), Some("target_151"));
    }
    #[test]
    fn alias_152() {
        assert_eq!(resolve_alias("alias_152"), Some("target_152"));
    }
    #[test]
    fn alias_153() {
        assert_eq!(resolve_alias("alias_153"), Some("target_153"));
    }
    #[test]
    fn alias_154() {
        assert_eq!(resolve_alias("alias_154"), Some("target_154"));
    }
    #[test]
    fn alias_155() {
        assert_eq!(resolve_alias("alias_155"), Some("target_155"));
    }
    #[test]
    fn alias_156() {
        assert_eq!(resolve_alias("alias_156"), Some("target_156"));
    }
    #[test]
    fn alias_157() {
        assert_eq!(resolve_alias("alias_157"), Some("target_157"));
    }
    #[test]
    fn alias_158() {
        assert_eq!(resolve_alias("alias_158"), Some("target_158"));
    }
    #[test]
    fn alias_159() {
        assert_eq!(resolve_alias("alias_159"), Some("target_159"));
    }
    #[test]
    fn alias_160() {
        assert_eq!(resolve_alias("alias_160"), Some("target_160"));
    }
    #[test]
    fn alias_161() {
        assert_eq!(resolve_alias("alias_161"), Some("target_161"));
    }
    #[test]
    fn alias_162() {
        assert_eq!(resolve_alias("alias_162"), Some("target_162"));
    }
    #[test]
    fn alias_163() {
        assert_eq!(resolve_alias("alias_163"), Some("target_163"));
    }
    #[test]
    fn alias_164() {
        assert_eq!(resolve_alias("alias_164"), Some("target_164"));
    }
    #[test]
    fn alias_165() {
        assert_eq!(resolve_alias("alias_165"), Some("target_165"));
    }
    #[test]
    fn alias_166() {
        assert_eq!(resolve_alias("alias_166"), Some("target_166"));
    }
    #[test]
    fn alias_167() {
        assert_eq!(resolve_alias("alias_167"), Some("target_167"));
    }
    #[test]
    fn alias_168() {
        assert_eq!(resolve_alias("alias_168"), Some("target_168"));
    }
    #[test]
    fn alias_169() {
        assert_eq!(resolve_alias("alias_169"), Some("target_169"));
    }
    #[test]
    fn alias_170() {
        assert_eq!(resolve_alias("alias_170"), Some("target_170"));
    }
    #[test]
    fn alias_171() {
        assert_eq!(resolve_alias("alias_171"), Some("target_171"));
    }
    #[test]
    fn alias_172() {
        assert_eq!(resolve_alias("alias_172"), Some("target_172"));
    }
    #[test]
    fn alias_173() {
        assert_eq!(resolve_alias("alias_173"), Some("target_173"));
    }
    #[test]
    fn alias_174() {
        assert_eq!(resolve_alias("alias_174"), Some("target_174"));
    }
    #[test]
    fn alias_175() {
        assert_eq!(resolve_alias("alias_175"), Some("target_175"));
    }
    #[test]
    fn alias_176() {
        assert_eq!(resolve_alias("alias_176"), Some("target_176"));
    }
    #[test]
    fn alias_177() {
        assert_eq!(resolve_alias("alias_177"), Some("target_177"));
    }
    #[test]
    fn alias_178() {
        assert_eq!(resolve_alias("alias_178"), Some("target_178"));
    }
    #[test]
    fn alias_179() {
        assert_eq!(resolve_alias("alias_179"), Some("target_179"));
    }
    #[test]
    fn alias_180() {
        assert_eq!(resolve_alias("alias_180"), Some("target_180"));
    }
    #[test]
    fn alias_181() {
        assert_eq!(resolve_alias("alias_181"), Some("target_181"));
    }
    #[test]
    fn alias_182() {
        assert_eq!(resolve_alias("alias_182"), Some("target_182"));
    }
    #[test]
    fn alias_183() {
        assert_eq!(resolve_alias("alias_183"), Some("target_183"));
    }
    #[test]
    fn alias_184() {
        assert_eq!(resolve_alias("alias_184"), Some("target_184"));
    }
    #[test]
    fn alias_185() {
        assert_eq!(resolve_alias("alias_185"), Some("target_185"));
    }
    #[test]
    fn alias_186() {
        assert_eq!(resolve_alias("alias_186"), Some("target_186"));
    }
    #[test]
    fn alias_187() {
        assert_eq!(resolve_alias("alias_187"), Some("target_187"));
    }
    #[test]
    fn alias_188() {
        assert_eq!(resolve_alias("alias_188"), Some("target_188"));
    }
    #[test]
    fn alias_189() {
        assert_eq!(resolve_alias("alias_189"), Some("target_189"));
    }
    #[test]
    fn alias_190() {
        assert_eq!(resolve_alias("alias_190"), Some("target_190"));
    }
    #[test]
    fn alias_191() {
        assert_eq!(resolve_alias("alias_191"), Some("target_191"));
    }
    #[test]
    fn alias_192() {
        assert_eq!(resolve_alias("alias_192"), Some("target_192"));
    }
    #[test]
    fn alias_193() {
        assert_eq!(resolve_alias("alias_193"), Some("target_193"));
    }
    #[test]
    fn alias_194() {
        assert_eq!(resolve_alias("alias_194"), Some("target_194"));
    }
    #[test]
    fn alias_195() {
        assert_eq!(resolve_alias("alias_195"), Some("target_195"));
    }
    #[test]
    fn alias_196() {
        assert_eq!(resolve_alias("alias_196"), Some("target_196"));
    }
    #[test]
    fn alias_197() {
        assert_eq!(resolve_alias("alias_197"), Some("target_197"));
    }
    #[test]
    fn alias_198() {
        assert_eq!(resolve_alias("alias_198"), Some("target_198"));
    }
    #[test]
    fn alias_199() {
        assert_eq!(resolve_alias("alias_199"), Some("target_199"));
    }
    #[test]
    fn alias_200() {
        assert_eq!(resolve_alias("alias_200"), Some("target_200"));
    }
    #[test]
    fn alias_201() {
        assert_eq!(resolve_alias("alias_201"), Some("target_201"));
    }
    #[test]
    fn alias_202() {
        assert_eq!(resolve_alias("alias_202"), Some("target_202"));
    }
    #[test]
    fn alias_203() {
        assert_eq!(resolve_alias("alias_203"), Some("target_203"));
    }
    #[test]
    fn alias_204() {
        assert_eq!(resolve_alias("alias_204"), Some("target_204"));
    }
    #[test]
    fn alias_205() {
        assert_eq!(resolve_alias("alias_205"), Some("target_205"));
    }
    #[test]
    fn alias_206() {
        assert_eq!(resolve_alias("alias_206"), Some("target_206"));
    }
    #[test]
    fn alias_207() {
        assert_eq!(resolve_alias("alias_207"), Some("target_207"));
    }
    #[test]
    fn alias_208() {
        assert_eq!(resolve_alias("alias_208"), Some("target_208"));
    }
    #[test]
    fn alias_209() {
        assert_eq!(resolve_alias("alias_209"), Some("target_209"));
    }
    #[test]
    fn alias_210() {
        assert_eq!(resolve_alias("alias_210"), Some("target_210"));
    }
    #[test]
    fn alias_211() {
        assert_eq!(resolve_alias("alias_211"), Some("target_211"));
    }
    #[test]
    fn alias_212() {
        assert_eq!(resolve_alias("alias_212"), Some("target_212"));
    }
    #[test]
    fn alias_213() {
        assert_eq!(resolve_alias("alias_213"), Some("target_213"));
    }
    #[test]
    fn alias_214() {
        assert_eq!(resolve_alias("alias_214"), Some("target_214"));
    }
    #[test]
    fn alias_215() {
        assert_eq!(resolve_alias("alias_215"), Some("target_215"));
    }
    #[test]
    fn alias_216() {
        assert_eq!(resolve_alias("alias_216"), Some("target_216"));
    }
    #[test]
    fn alias_217() {
        assert_eq!(resolve_alias("alias_217"), Some("target_217"));
    }
    #[test]
    fn alias_218() {
        assert_eq!(resolve_alias("alias_218"), Some("target_218"));
    }
    #[test]
    fn alias_219() {
        assert_eq!(resolve_alias("alias_219"), Some("target_219"));
    }
    #[test]
    fn alias_220() {
        assert_eq!(resolve_alias("alias_220"), Some("target_220"));
    }
    #[test]
    fn alias_221() {
        assert_eq!(resolve_alias("alias_221"), Some("target_221"));
    }
    #[test]
    fn alias_222() {
        assert_eq!(resolve_alias("alias_222"), Some("target_222"));
    }
    #[test]
    fn alias_223() {
        assert_eq!(resolve_alias("alias_223"), Some("target_223"));
    }
    #[test]
    fn alias_224() {
        assert_eq!(resolve_alias("alias_224"), Some("target_224"));
    }
    #[test]
    fn alias_225() {
        assert_eq!(resolve_alias("alias_225"), Some("target_225"));
    }
    #[test]
    fn alias_226() {
        assert_eq!(resolve_alias("alias_226"), Some("target_226"));
    }
    #[test]
    fn alias_227() {
        assert_eq!(resolve_alias("alias_227"), Some("target_227"));
    }
    #[test]
    fn alias_228() {
        assert_eq!(resolve_alias("alias_228"), Some("target_228"));
    }
    #[test]
    fn alias_229() {
        assert_eq!(resolve_alias("alias_229"), Some("target_229"));
    }
    #[test]
    fn alias_230() {
        assert_eq!(resolve_alias("alias_230"), Some("target_230"));
    }
    #[test]
    fn alias_231() {
        assert_eq!(resolve_alias("alias_231"), Some("target_231"));
    }
    #[test]
    fn alias_232() {
        assert_eq!(resolve_alias("alias_232"), Some("target_232"));
    }
    #[test]
    fn alias_233() {
        assert_eq!(resolve_alias("alias_233"), Some("target_233"));
    }
    #[test]
    fn alias_234() {
        assert_eq!(resolve_alias("alias_234"), Some("target_234"));
    }
    #[test]
    fn alias_235() {
        assert_eq!(resolve_alias("alias_235"), Some("target_235"));
    }
    #[test]
    fn alias_236() {
        assert_eq!(resolve_alias("alias_236"), Some("target_236"));
    }
    #[test]
    fn alias_237() {
        assert_eq!(resolve_alias("alias_237"), Some("target_237"));
    }
    #[test]
    fn alias_238() {
        assert_eq!(resolve_alias("alias_238"), Some("target_238"));
    }
    #[test]
    fn alias_239() {
        assert_eq!(resolve_alias("alias_239"), Some("target_239"));
    }
    #[test]
    fn alias_240() {
        assert_eq!(resolve_alias("alias_240"), Some("target_240"));
    }
    #[test]
    fn alias_241() {
        assert_eq!(resolve_alias("alias_241"), Some("target_241"));
    }
    #[test]
    fn alias_242() {
        assert_eq!(resolve_alias("alias_242"), Some("target_242"));
    }
    #[test]
    fn alias_243() {
        assert_eq!(resolve_alias("alias_243"), Some("target_243"));
    }
    #[test]
    fn alias_244() {
        assert_eq!(resolve_alias("alias_244"), Some("target_244"));
    }
    #[test]
    fn alias_245() {
        assert_eq!(resolve_alias("alias_245"), Some("target_245"));
    }
    #[test]
    fn alias_246() {
        assert_eq!(resolve_alias("alias_246"), Some("target_246"));
    }
    #[test]
    fn alias_247() {
        assert_eq!(resolve_alias("alias_247"), Some("target_247"));
    }
    #[test]
    fn alias_248() {
        assert_eq!(resolve_alias("alias_248"), Some("target_248"));
    }
    #[test]
    fn alias_249() {
        assert_eq!(resolve_alias("alias_249"), Some("target_249"));
    }
    #[test]
    fn alias_250() {
        assert_eq!(resolve_alias("alias_250"), Some("target_250"));
    }
    #[test]
    fn alias_251() {
        assert_eq!(resolve_alias("alias_251"), Some("target_251"));
    }
    #[test]
    fn alias_252() {
        assert_eq!(resolve_alias("alias_252"), Some("target_252"));
    }
    #[test]
    fn alias_253() {
        assert_eq!(resolve_alias("alias_253"), Some("target_253"));
    }
    #[test]
    fn alias_254() {
        assert_eq!(resolve_alias("alias_254"), Some("target_254"));
    }
    #[test]
    fn alias_255() {
        assert_eq!(resolve_alias("alias_255"), Some("target_255"));
    }
    #[test]
    fn alias_256() {
        assert_eq!(resolve_alias("alias_256"), Some("target_256"));
    }
    #[test]
    fn alias_257() {
        assert_eq!(resolve_alias("alias_257"), Some("target_257"));
    }
    #[test]
    fn alias_258() {
        assert_eq!(resolve_alias("alias_258"), Some("target_258"));
    }
    #[test]
    fn alias_259() {
        assert_eq!(resolve_alias("alias_259"), Some("target_259"));
    }
    #[test]
    fn alias_260() {
        assert_eq!(resolve_alias("alias_260"), Some("target_260"));
    }
    #[test]
    fn alias_261() {
        assert_eq!(resolve_alias("alias_261"), Some("target_261"));
    }
    #[test]
    fn alias_262() {
        assert_eq!(resolve_alias("alias_262"), Some("target_262"));
    }
    #[test]
    fn alias_263() {
        assert_eq!(resolve_alias("alias_263"), Some("target_263"));
    }
    #[test]
    fn alias_264() {
        assert_eq!(resolve_alias("alias_264"), Some("target_264"));
    }
    #[test]
    fn alias_265() {
        assert_eq!(resolve_alias("alias_265"), Some("target_265"));
    }
    #[test]
    fn alias_266() {
        assert_eq!(resolve_alias("alias_266"), Some("target_266"));
    }
    #[test]
    fn alias_267() {
        assert_eq!(resolve_alias("alias_267"), Some("target_267"));
    }
    #[test]
    fn alias_268() {
        assert_eq!(resolve_alias("alias_268"), Some("target_268"));
    }
    #[test]
    fn alias_269() {
        assert_eq!(resolve_alias("alias_269"), Some("target_269"));
    }
    #[test]
    fn alias_270() {
        assert_eq!(resolve_alias("alias_270"), Some("target_270"));
    }
    #[test]
    fn alias_271() {
        assert_eq!(resolve_alias("alias_271"), Some("target_271"));
    }
    #[test]
    fn alias_272() {
        assert_eq!(resolve_alias("alias_272"), Some("target_272"));
    }
    #[test]
    fn alias_273() {
        assert_eq!(resolve_alias("alias_273"), Some("target_273"));
    }
    #[test]
    fn alias_274() {
        assert_eq!(resolve_alias("alias_274"), Some("target_274"));
    }
    #[test]
    fn alias_275() {
        assert_eq!(resolve_alias("alias_275"), Some("target_275"));
    }
    #[test]
    fn alias_276() {
        assert_eq!(resolve_alias("alias_276"), Some("target_276"));
    }
    #[test]
    fn alias_277() {
        assert_eq!(resolve_alias("alias_277"), Some("target_277"));
    }
    #[test]
    fn alias_278() {
        assert_eq!(resolve_alias("alias_278"), Some("target_278"));
    }
    #[test]
    fn alias_279() {
        assert_eq!(resolve_alias("alias_279"), Some("target_279"));
    }
    #[test]
    fn alias_280() {
        assert_eq!(resolve_alias("alias_280"), Some("target_280"));
    }
    #[test]
    fn alias_281() {
        assert_eq!(resolve_alias("alias_281"), Some("target_281"));
    }
    #[test]
    fn alias_282() {
        assert_eq!(resolve_alias("alias_282"), Some("target_282"));
    }
    #[test]
    fn alias_283() {
        assert_eq!(resolve_alias("alias_283"), Some("target_283"));
    }
    #[test]
    fn alias_284() {
        assert_eq!(resolve_alias("alias_284"), Some("target_284"));
    }
    #[test]
    fn alias_285() {
        assert_eq!(resolve_alias("alias_285"), Some("target_285"));
    }
    #[test]
    fn alias_286() {
        assert_eq!(resolve_alias("alias_286"), Some("target_286"));
    }
    #[test]
    fn alias_287() {
        assert_eq!(resolve_alias("alias_287"), Some("target_287"));
    }
    #[test]
    fn alias_288() {
        assert_eq!(resolve_alias("alias_288"), Some("target_288"));
    }
    #[test]
    fn alias_289() {
        assert_eq!(resolve_alias("alias_289"), Some("target_289"));
    }
    #[test]
    fn alias_290() {
        assert_eq!(resolve_alias("alias_290"), Some("target_290"));
    }
    #[test]
    fn alias_291() {
        assert_eq!(resolve_alias("alias_291"), Some("target_291"));
    }
    #[test]
    fn alias_292() {
        assert_eq!(resolve_alias("alias_292"), Some("target_292"));
    }
    #[test]
    fn alias_293() {
        assert_eq!(resolve_alias("alias_293"), Some("target_293"));
    }
    #[test]
    fn alias_294() {
        assert_eq!(resolve_alias("alias_294"), Some("target_294"));
    }
    #[test]
    fn alias_295() {
        assert_eq!(resolve_alias("alias_295"), Some("target_295"));
    }
    #[test]
    fn alias_296() {
        assert_eq!(resolve_alias("alias_296"), Some("target_296"));
    }
    #[test]
    fn alias_297() {
        assert_eq!(resolve_alias("alias_297"), Some("target_297"));
    }
    #[test]
    fn alias_298() {
        assert_eq!(resolve_alias("alias_298"), Some("target_298"));
    }
    #[test]
    fn alias_299() {
        assert_eq!(resolve_alias("alias_299"), Some("target_299"));
    }
}
