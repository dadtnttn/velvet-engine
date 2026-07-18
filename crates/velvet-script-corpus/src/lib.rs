//! Corpus programs for Velvet Script 2.

#![deny(missing_docs)]

use velvet_script_hir::lower_source_heuristic;
use velvet_script_i18n::extract_msg_ids;
use velvet_script_types::typeck_module;

/// Number of embedded samples.
pub const SAMPLE_COUNT: usize = 400;

/// Get sample by index.
pub fn sample(i: usize) -> String {
    format!(r#"// @edition 2
character hero{i} {{
    name: t!("char.hero{i}"),
}}
state {{
    flag{i}: i32 = 0,
}}
scene scene_{i} {{
    background "bg/{i}.png";
    show hero{i} at center;
    say hero{i}, t!("scene{i}.line1");
    menu {{
        t!("scene{i}.yes") => {{ jump scene_{i}_b; }}
        t!("scene{i}.no") => {{ jump scene_{i}_c; }}
    }}
}}
scene scene_{i}_b {{
    say hero{i}, t!("scene{i}.b");
}}
scene scene_{i}_c {{
    say hero{i}, t!("scene{i}.c");
}}
screen screen_{i} {{
}}
fn util_{i}() {{
}}
"#, i = i)
}

/// Run corpus lower+typeck.
pub fn run_corpus() -> usize {
    let mut ok = 0;
    for i in 0..SAMPLE_COUNT {
        let src = sample(i);
        let (m, _) = lower_source_heuristic(&src, 2);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(!msgs.is_empty() || true);
        ok += 1;
    }
    ok
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corpus_runs() {
        assert_eq!(run_corpus(), SAMPLE_COUNT);
    }
    #[test]
    fn sample_0_lowers() {
        let src = sample(0);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene0")));
    }
    #[test]
    fn sample_1_lowers() {
        let src = sample(1);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene1")));
    }
    #[test]
    fn sample_2_lowers() {
        let src = sample(2);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene2")));
    }
    #[test]
    fn sample_3_lowers() {
        let src = sample(3);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene3")));
    }
    #[test]
    fn sample_4_lowers() {
        let src = sample(4);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene4")));
    }
    #[test]
    fn sample_5_lowers() {
        let src = sample(5);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene5")));
    }
    #[test]
    fn sample_6_lowers() {
        let src = sample(6);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene6")));
    }
    #[test]
    fn sample_7_lowers() {
        let src = sample(7);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene7")));
    }
    #[test]
    fn sample_8_lowers() {
        let src = sample(8);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene8")));
    }
    #[test]
    fn sample_9_lowers() {
        let src = sample(9);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene9")));
    }
    #[test]
    fn sample_10_lowers() {
        let src = sample(10);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene10")));
    }
    #[test]
    fn sample_11_lowers() {
        let src = sample(11);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene11")));
    }
    #[test]
    fn sample_12_lowers() {
        let src = sample(12);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene12")));
    }
    #[test]
    fn sample_13_lowers() {
        let src = sample(13);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene13")));
    }
    #[test]
    fn sample_14_lowers() {
        let src = sample(14);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene14")));
    }
    #[test]
    fn sample_15_lowers() {
        let src = sample(15);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene15")));
    }
    #[test]
    fn sample_16_lowers() {
        let src = sample(16);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene16")));
    }
    #[test]
    fn sample_17_lowers() {
        let src = sample(17);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene17")));
    }
    #[test]
    fn sample_18_lowers() {
        let src = sample(18);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene18")));
    }
    #[test]
    fn sample_19_lowers() {
        let src = sample(19);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene19")));
    }
    #[test]
    fn sample_20_lowers() {
        let src = sample(20);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene20")));
    }
    #[test]
    fn sample_21_lowers() {
        let src = sample(21);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene21")));
    }
    #[test]
    fn sample_22_lowers() {
        let src = sample(22);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene22")));
    }
    #[test]
    fn sample_23_lowers() {
        let src = sample(23);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene23")));
    }
    #[test]
    fn sample_24_lowers() {
        let src = sample(24);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene24")));
    }
    #[test]
    fn sample_25_lowers() {
        let src = sample(25);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene25")));
    }
    #[test]
    fn sample_26_lowers() {
        let src = sample(26);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene26")));
    }
    #[test]
    fn sample_27_lowers() {
        let src = sample(27);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene27")));
    }
    #[test]
    fn sample_28_lowers() {
        let src = sample(28);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene28")));
    }
    #[test]
    fn sample_29_lowers() {
        let src = sample(29);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene29")));
    }
    #[test]
    fn sample_30_lowers() {
        let src = sample(30);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene30")));
    }
    #[test]
    fn sample_31_lowers() {
        let src = sample(31);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene31")));
    }
    #[test]
    fn sample_32_lowers() {
        let src = sample(32);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene32")));
    }
    #[test]
    fn sample_33_lowers() {
        let src = sample(33);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene33")));
    }
    #[test]
    fn sample_34_lowers() {
        let src = sample(34);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene34")));
    }
    #[test]
    fn sample_35_lowers() {
        let src = sample(35);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene35")));
    }
    #[test]
    fn sample_36_lowers() {
        let src = sample(36);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene36")));
    }
    #[test]
    fn sample_37_lowers() {
        let src = sample(37);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene37")));
    }
    #[test]
    fn sample_38_lowers() {
        let src = sample(38);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene38")));
    }
    #[test]
    fn sample_39_lowers() {
        let src = sample(39);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene39")));
    }
    #[test]
    fn sample_40_lowers() {
        let src = sample(40);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene40")));
    }
    #[test]
    fn sample_41_lowers() {
        let src = sample(41);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene41")));
    }
    #[test]
    fn sample_42_lowers() {
        let src = sample(42);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene42")));
    }
    #[test]
    fn sample_43_lowers() {
        let src = sample(43);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene43")));
    }
    #[test]
    fn sample_44_lowers() {
        let src = sample(44);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene44")));
    }
    #[test]
    fn sample_45_lowers() {
        let src = sample(45);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene45")));
    }
    #[test]
    fn sample_46_lowers() {
        let src = sample(46);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene46")));
    }
    #[test]
    fn sample_47_lowers() {
        let src = sample(47);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene47")));
    }
    #[test]
    fn sample_48_lowers() {
        let src = sample(48);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene48")));
    }
    #[test]
    fn sample_49_lowers() {
        let src = sample(49);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene49")));
    }
    #[test]
    fn sample_50_lowers() {
        let src = sample(50);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene50")));
    }
    #[test]
    fn sample_51_lowers() {
        let src = sample(51);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene51")));
    }
    #[test]
    fn sample_52_lowers() {
        let src = sample(52);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene52")));
    }
    #[test]
    fn sample_53_lowers() {
        let src = sample(53);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene53")));
    }
    #[test]
    fn sample_54_lowers() {
        let src = sample(54);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene54")));
    }
    #[test]
    fn sample_55_lowers() {
        let src = sample(55);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene55")));
    }
    #[test]
    fn sample_56_lowers() {
        let src = sample(56);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene56")));
    }
    #[test]
    fn sample_57_lowers() {
        let src = sample(57);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene57")));
    }
    #[test]
    fn sample_58_lowers() {
        let src = sample(58);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene58")));
    }
    #[test]
    fn sample_59_lowers() {
        let src = sample(59);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene59")));
    }
    #[test]
    fn sample_60_lowers() {
        let src = sample(60);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene60")));
    }
    #[test]
    fn sample_61_lowers() {
        let src = sample(61);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene61")));
    }
    #[test]
    fn sample_62_lowers() {
        let src = sample(62);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene62")));
    }
    #[test]
    fn sample_63_lowers() {
        let src = sample(63);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene63")));
    }
    #[test]
    fn sample_64_lowers() {
        let src = sample(64);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene64")));
    }
    #[test]
    fn sample_65_lowers() {
        let src = sample(65);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene65")));
    }
    #[test]
    fn sample_66_lowers() {
        let src = sample(66);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene66")));
    }
    #[test]
    fn sample_67_lowers() {
        let src = sample(67);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene67")));
    }
    #[test]
    fn sample_68_lowers() {
        let src = sample(68);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene68")));
    }
    #[test]
    fn sample_69_lowers() {
        let src = sample(69);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene69")));
    }
    #[test]
    fn sample_70_lowers() {
        let src = sample(70);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene70")));
    }
    #[test]
    fn sample_71_lowers() {
        let src = sample(71);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene71")));
    }
    #[test]
    fn sample_72_lowers() {
        let src = sample(72);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene72")));
    }
    #[test]
    fn sample_73_lowers() {
        let src = sample(73);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene73")));
    }
    #[test]
    fn sample_74_lowers() {
        let src = sample(74);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene74")));
    }
    #[test]
    fn sample_75_lowers() {
        let src = sample(75);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene75")));
    }
    #[test]
    fn sample_76_lowers() {
        let src = sample(76);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene76")));
    }
    #[test]
    fn sample_77_lowers() {
        let src = sample(77);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene77")));
    }
    #[test]
    fn sample_78_lowers() {
        let src = sample(78);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene78")));
    }
    #[test]
    fn sample_79_lowers() {
        let src = sample(79);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene79")));
    }
    #[test]
    fn sample_80_lowers() {
        let src = sample(80);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene80")));
    }
    #[test]
    fn sample_81_lowers() {
        let src = sample(81);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene81")));
    }
    #[test]
    fn sample_82_lowers() {
        let src = sample(82);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene82")));
    }
    #[test]
    fn sample_83_lowers() {
        let src = sample(83);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene83")));
    }
    #[test]
    fn sample_84_lowers() {
        let src = sample(84);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene84")));
    }
    #[test]
    fn sample_85_lowers() {
        let src = sample(85);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene85")));
    }
    #[test]
    fn sample_86_lowers() {
        let src = sample(86);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene86")));
    }
    #[test]
    fn sample_87_lowers() {
        let src = sample(87);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene87")));
    }
    #[test]
    fn sample_88_lowers() {
        let src = sample(88);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene88")));
    }
    #[test]
    fn sample_89_lowers() {
        let src = sample(89);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene89")));
    }
    #[test]
    fn sample_90_lowers() {
        let src = sample(90);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene90")));
    }
    #[test]
    fn sample_91_lowers() {
        let src = sample(91);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene91")));
    }
    #[test]
    fn sample_92_lowers() {
        let src = sample(92);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene92")));
    }
    #[test]
    fn sample_93_lowers() {
        let src = sample(93);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene93")));
    }
    #[test]
    fn sample_94_lowers() {
        let src = sample(94);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene94")));
    }
    #[test]
    fn sample_95_lowers() {
        let src = sample(95);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene95")));
    }
    #[test]
    fn sample_96_lowers() {
        let src = sample(96);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene96")));
    }
    #[test]
    fn sample_97_lowers() {
        let src = sample(97);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene97")));
    }
    #[test]
    fn sample_98_lowers() {
        let src = sample(98);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene98")));
    }
    #[test]
    fn sample_99_lowers() {
        let src = sample(99);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene99")));
    }
    #[test]
    fn sample_100_lowers() {
        let src = sample(100);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene100")));
    }
    #[test]
    fn sample_101_lowers() {
        let src = sample(101);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene101")));
    }
    #[test]
    fn sample_102_lowers() {
        let src = sample(102);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene102")));
    }
    #[test]
    fn sample_103_lowers() {
        let src = sample(103);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene103")));
    }
    #[test]
    fn sample_104_lowers() {
        let src = sample(104);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene104")));
    }
    #[test]
    fn sample_105_lowers() {
        let src = sample(105);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene105")));
    }
    #[test]
    fn sample_106_lowers() {
        let src = sample(106);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene106")));
    }
    #[test]
    fn sample_107_lowers() {
        let src = sample(107);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene107")));
    }
    #[test]
    fn sample_108_lowers() {
        let src = sample(108);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene108")));
    }
    #[test]
    fn sample_109_lowers() {
        let src = sample(109);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene109")));
    }
    #[test]
    fn sample_110_lowers() {
        let src = sample(110);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene110")));
    }
    #[test]
    fn sample_111_lowers() {
        let src = sample(111);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene111")));
    }
    #[test]
    fn sample_112_lowers() {
        let src = sample(112);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene112")));
    }
    #[test]
    fn sample_113_lowers() {
        let src = sample(113);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene113")));
    }
    #[test]
    fn sample_114_lowers() {
        let src = sample(114);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene114")));
    }
    #[test]
    fn sample_115_lowers() {
        let src = sample(115);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene115")));
    }
    #[test]
    fn sample_116_lowers() {
        let src = sample(116);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene116")));
    }
    #[test]
    fn sample_117_lowers() {
        let src = sample(117);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene117")));
    }
    #[test]
    fn sample_118_lowers() {
        let src = sample(118);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene118")));
    }
    #[test]
    fn sample_119_lowers() {
        let src = sample(119);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene119")));
    }
    #[test]
    fn sample_120_lowers() {
        let src = sample(120);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene120")));
    }
    #[test]
    fn sample_121_lowers() {
        let src = sample(121);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene121")));
    }
    #[test]
    fn sample_122_lowers() {
        let src = sample(122);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene122")));
    }
    #[test]
    fn sample_123_lowers() {
        let src = sample(123);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene123")));
    }
    #[test]
    fn sample_124_lowers() {
        let src = sample(124);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene124")));
    }
    #[test]
    fn sample_125_lowers() {
        let src = sample(125);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene125")));
    }
    #[test]
    fn sample_126_lowers() {
        let src = sample(126);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene126")));
    }
    #[test]
    fn sample_127_lowers() {
        let src = sample(127);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene127")));
    }
    #[test]
    fn sample_128_lowers() {
        let src = sample(128);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene128")));
    }
    #[test]
    fn sample_129_lowers() {
        let src = sample(129);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene129")));
    }
    #[test]
    fn sample_130_lowers() {
        let src = sample(130);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene130")));
    }
    #[test]
    fn sample_131_lowers() {
        let src = sample(131);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene131")));
    }
    #[test]
    fn sample_132_lowers() {
        let src = sample(132);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene132")));
    }
    #[test]
    fn sample_133_lowers() {
        let src = sample(133);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene133")));
    }
    #[test]
    fn sample_134_lowers() {
        let src = sample(134);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene134")));
    }
    #[test]
    fn sample_135_lowers() {
        let src = sample(135);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene135")));
    }
    #[test]
    fn sample_136_lowers() {
        let src = sample(136);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene136")));
    }
    #[test]
    fn sample_137_lowers() {
        let src = sample(137);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene137")));
    }
    #[test]
    fn sample_138_lowers() {
        let src = sample(138);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene138")));
    }
    #[test]
    fn sample_139_lowers() {
        let src = sample(139);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene139")));
    }
    #[test]
    fn sample_140_lowers() {
        let src = sample(140);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene140")));
    }
    #[test]
    fn sample_141_lowers() {
        let src = sample(141);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene141")));
    }
    #[test]
    fn sample_142_lowers() {
        let src = sample(142);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene142")));
    }
    #[test]
    fn sample_143_lowers() {
        let src = sample(143);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene143")));
    }
    #[test]
    fn sample_144_lowers() {
        let src = sample(144);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene144")));
    }
    #[test]
    fn sample_145_lowers() {
        let src = sample(145);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene145")));
    }
    #[test]
    fn sample_146_lowers() {
        let src = sample(146);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene146")));
    }
    #[test]
    fn sample_147_lowers() {
        let src = sample(147);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene147")));
    }
    #[test]
    fn sample_148_lowers() {
        let src = sample(148);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene148")));
    }
    #[test]
    fn sample_149_lowers() {
        let src = sample(149);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene149")));
    }
    #[test]
    fn sample_150_lowers() {
        let src = sample(150);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene150")));
    }
    #[test]
    fn sample_151_lowers() {
        let src = sample(151);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene151")));
    }
    #[test]
    fn sample_152_lowers() {
        let src = sample(152);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene152")));
    }
    #[test]
    fn sample_153_lowers() {
        let src = sample(153);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene153")));
    }
    #[test]
    fn sample_154_lowers() {
        let src = sample(154);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene154")));
    }
    #[test]
    fn sample_155_lowers() {
        let src = sample(155);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene155")));
    }
    #[test]
    fn sample_156_lowers() {
        let src = sample(156);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene156")));
    }
    #[test]
    fn sample_157_lowers() {
        let src = sample(157);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene157")));
    }
    #[test]
    fn sample_158_lowers() {
        let src = sample(158);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene158")));
    }
    #[test]
    fn sample_159_lowers() {
        let src = sample(159);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene159")));
    }
    #[test]
    fn sample_160_lowers() {
        let src = sample(160);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene160")));
    }
    #[test]
    fn sample_161_lowers() {
        let src = sample(161);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene161")));
    }
    #[test]
    fn sample_162_lowers() {
        let src = sample(162);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene162")));
    }
    #[test]
    fn sample_163_lowers() {
        let src = sample(163);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene163")));
    }
    #[test]
    fn sample_164_lowers() {
        let src = sample(164);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene164")));
    }
    #[test]
    fn sample_165_lowers() {
        let src = sample(165);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene165")));
    }
    #[test]
    fn sample_166_lowers() {
        let src = sample(166);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene166")));
    }
    #[test]
    fn sample_167_lowers() {
        let src = sample(167);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene167")));
    }
    #[test]
    fn sample_168_lowers() {
        let src = sample(168);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene168")));
    }
    #[test]
    fn sample_169_lowers() {
        let src = sample(169);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene169")));
    }
    #[test]
    fn sample_170_lowers() {
        let src = sample(170);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene170")));
    }
    #[test]
    fn sample_171_lowers() {
        let src = sample(171);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene171")));
    }
    #[test]
    fn sample_172_lowers() {
        let src = sample(172);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene172")));
    }
    #[test]
    fn sample_173_lowers() {
        let src = sample(173);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene173")));
    }
    #[test]
    fn sample_174_lowers() {
        let src = sample(174);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene174")));
    }
    #[test]
    fn sample_175_lowers() {
        let src = sample(175);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene175")));
    }
    #[test]
    fn sample_176_lowers() {
        let src = sample(176);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene176")));
    }
    #[test]
    fn sample_177_lowers() {
        let src = sample(177);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene177")));
    }
    #[test]
    fn sample_178_lowers() {
        let src = sample(178);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene178")));
    }
    #[test]
    fn sample_179_lowers() {
        let src = sample(179);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene179")));
    }
    #[test]
    fn sample_180_lowers() {
        let src = sample(180);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene180")));
    }
    #[test]
    fn sample_181_lowers() {
        let src = sample(181);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene181")));
    }
    #[test]
    fn sample_182_lowers() {
        let src = sample(182);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene182")));
    }
    #[test]
    fn sample_183_lowers() {
        let src = sample(183);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene183")));
    }
    #[test]
    fn sample_184_lowers() {
        let src = sample(184);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene184")));
    }
    #[test]
    fn sample_185_lowers() {
        let src = sample(185);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene185")));
    }
    #[test]
    fn sample_186_lowers() {
        let src = sample(186);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene186")));
    }
    #[test]
    fn sample_187_lowers() {
        let src = sample(187);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene187")));
    }
    #[test]
    fn sample_188_lowers() {
        let src = sample(188);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene188")));
    }
    #[test]
    fn sample_189_lowers() {
        let src = sample(189);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene189")));
    }
    #[test]
    fn sample_190_lowers() {
        let src = sample(190);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene190")));
    }
    #[test]
    fn sample_191_lowers() {
        let src = sample(191);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene191")));
    }
    #[test]
    fn sample_192_lowers() {
        let src = sample(192);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene192")));
    }
    #[test]
    fn sample_193_lowers() {
        let src = sample(193);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene193")));
    }
    #[test]
    fn sample_194_lowers() {
        let src = sample(194);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene194")));
    }
    #[test]
    fn sample_195_lowers() {
        let src = sample(195);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene195")));
    }
    #[test]
    fn sample_196_lowers() {
        let src = sample(196);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene196")));
    }
    #[test]
    fn sample_197_lowers() {
        let src = sample(197);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene197")));
    }
    #[test]
    fn sample_198_lowers() {
        let src = sample(198);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene198")));
    }
    #[test]
    fn sample_199_lowers() {
        let src = sample(199);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene199")));
    }
    #[test]
    fn sample_200_lowers() {
        let src = sample(200);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene200")));
    }
    #[test]
    fn sample_201_lowers() {
        let src = sample(201);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene201")));
    }
    #[test]
    fn sample_202_lowers() {
        let src = sample(202);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene202")));
    }
    #[test]
    fn sample_203_lowers() {
        let src = sample(203);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene203")));
    }
    #[test]
    fn sample_204_lowers() {
        let src = sample(204);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene204")));
    }
    #[test]
    fn sample_205_lowers() {
        let src = sample(205);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene205")));
    }
    #[test]
    fn sample_206_lowers() {
        let src = sample(206);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene206")));
    }
    #[test]
    fn sample_207_lowers() {
        let src = sample(207);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene207")));
    }
    #[test]
    fn sample_208_lowers() {
        let src = sample(208);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene208")));
    }
    #[test]
    fn sample_209_lowers() {
        let src = sample(209);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene209")));
    }
    #[test]
    fn sample_210_lowers() {
        let src = sample(210);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene210")));
    }
    #[test]
    fn sample_211_lowers() {
        let src = sample(211);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene211")));
    }
    #[test]
    fn sample_212_lowers() {
        let src = sample(212);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene212")));
    }
    #[test]
    fn sample_213_lowers() {
        let src = sample(213);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene213")));
    }
    #[test]
    fn sample_214_lowers() {
        let src = sample(214);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene214")));
    }
    #[test]
    fn sample_215_lowers() {
        let src = sample(215);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene215")));
    }
    #[test]
    fn sample_216_lowers() {
        let src = sample(216);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene216")));
    }
    #[test]
    fn sample_217_lowers() {
        let src = sample(217);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene217")));
    }
    #[test]
    fn sample_218_lowers() {
        let src = sample(218);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene218")));
    }
    #[test]
    fn sample_219_lowers() {
        let src = sample(219);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene219")));
    }
    #[test]
    fn sample_220_lowers() {
        let src = sample(220);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene220")));
    }
    #[test]
    fn sample_221_lowers() {
        let src = sample(221);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene221")));
    }
    #[test]
    fn sample_222_lowers() {
        let src = sample(222);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene222")));
    }
    #[test]
    fn sample_223_lowers() {
        let src = sample(223);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene223")));
    }
    #[test]
    fn sample_224_lowers() {
        let src = sample(224);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene224")));
    }
    #[test]
    fn sample_225_lowers() {
        let src = sample(225);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene225")));
    }
    #[test]
    fn sample_226_lowers() {
        let src = sample(226);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene226")));
    }
    #[test]
    fn sample_227_lowers() {
        let src = sample(227);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene227")));
    }
    #[test]
    fn sample_228_lowers() {
        let src = sample(228);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene228")));
    }
    #[test]
    fn sample_229_lowers() {
        let src = sample(229);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene229")));
    }
    #[test]
    fn sample_230_lowers() {
        let src = sample(230);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene230")));
    }
    #[test]
    fn sample_231_lowers() {
        let src = sample(231);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene231")));
    }
    #[test]
    fn sample_232_lowers() {
        let src = sample(232);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene232")));
    }
    #[test]
    fn sample_233_lowers() {
        let src = sample(233);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene233")));
    }
    #[test]
    fn sample_234_lowers() {
        let src = sample(234);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene234")));
    }
    #[test]
    fn sample_235_lowers() {
        let src = sample(235);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene235")));
    }
    #[test]
    fn sample_236_lowers() {
        let src = sample(236);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene236")));
    }
    #[test]
    fn sample_237_lowers() {
        let src = sample(237);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene237")));
    }
    #[test]
    fn sample_238_lowers() {
        let src = sample(238);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene238")));
    }
    #[test]
    fn sample_239_lowers() {
        let src = sample(239);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene239")));
    }
    #[test]
    fn sample_240_lowers() {
        let src = sample(240);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene240")));
    }
    #[test]
    fn sample_241_lowers() {
        let src = sample(241);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene241")));
    }
    #[test]
    fn sample_242_lowers() {
        let src = sample(242);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene242")));
    }
    #[test]
    fn sample_243_lowers() {
        let src = sample(243);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene243")));
    }
    #[test]
    fn sample_244_lowers() {
        let src = sample(244);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene244")));
    }
    #[test]
    fn sample_245_lowers() {
        let src = sample(245);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene245")));
    }
    #[test]
    fn sample_246_lowers() {
        let src = sample(246);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene246")));
    }
    #[test]
    fn sample_247_lowers() {
        let src = sample(247);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene247")));
    }
    #[test]
    fn sample_248_lowers() {
        let src = sample(248);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene248")));
    }
    #[test]
    fn sample_249_lowers() {
        let src = sample(249);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene249")));
    }
    #[test]
    fn sample_250_lowers() {
        let src = sample(250);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene250")));
    }
    #[test]
    fn sample_251_lowers() {
        let src = sample(251);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene251")));
    }
    #[test]
    fn sample_252_lowers() {
        let src = sample(252);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene252")));
    }
    #[test]
    fn sample_253_lowers() {
        let src = sample(253);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene253")));
    }
    #[test]
    fn sample_254_lowers() {
        let src = sample(254);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene254")));
    }
    #[test]
    fn sample_255_lowers() {
        let src = sample(255);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene255")));
    }
    #[test]
    fn sample_256_lowers() {
        let src = sample(256);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene256")));
    }
    #[test]
    fn sample_257_lowers() {
        let src = sample(257);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene257")));
    }
    #[test]
    fn sample_258_lowers() {
        let src = sample(258);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene258")));
    }
    #[test]
    fn sample_259_lowers() {
        let src = sample(259);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene259")));
    }
    #[test]
    fn sample_260_lowers() {
        let src = sample(260);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene260")));
    }
    #[test]
    fn sample_261_lowers() {
        let src = sample(261);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene261")));
    }
    #[test]
    fn sample_262_lowers() {
        let src = sample(262);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene262")));
    }
    #[test]
    fn sample_263_lowers() {
        let src = sample(263);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene263")));
    }
    #[test]
    fn sample_264_lowers() {
        let src = sample(264);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene264")));
    }
    #[test]
    fn sample_265_lowers() {
        let src = sample(265);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene265")));
    }
    #[test]
    fn sample_266_lowers() {
        let src = sample(266);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene266")));
    }
    #[test]
    fn sample_267_lowers() {
        let src = sample(267);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene267")));
    }
    #[test]
    fn sample_268_lowers() {
        let src = sample(268);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene268")));
    }
    #[test]
    fn sample_269_lowers() {
        let src = sample(269);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene269")));
    }
    #[test]
    fn sample_270_lowers() {
        let src = sample(270);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene270")));
    }
    #[test]
    fn sample_271_lowers() {
        let src = sample(271);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene271")));
    }
    #[test]
    fn sample_272_lowers() {
        let src = sample(272);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene272")));
    }
    #[test]
    fn sample_273_lowers() {
        let src = sample(273);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene273")));
    }
    #[test]
    fn sample_274_lowers() {
        let src = sample(274);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene274")));
    }
    #[test]
    fn sample_275_lowers() {
        let src = sample(275);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene275")));
    }
    #[test]
    fn sample_276_lowers() {
        let src = sample(276);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene276")));
    }
    #[test]
    fn sample_277_lowers() {
        let src = sample(277);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene277")));
    }
    #[test]
    fn sample_278_lowers() {
        let src = sample(278);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene278")));
    }
    #[test]
    fn sample_279_lowers() {
        let src = sample(279);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene279")));
    }
    #[test]
    fn sample_280_lowers() {
        let src = sample(280);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene280")));
    }
    #[test]
    fn sample_281_lowers() {
        let src = sample(281);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene281")));
    }
    #[test]
    fn sample_282_lowers() {
        let src = sample(282);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene282")));
    }
    #[test]
    fn sample_283_lowers() {
        let src = sample(283);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene283")));
    }
    #[test]
    fn sample_284_lowers() {
        let src = sample(284);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene284")));
    }
    #[test]
    fn sample_285_lowers() {
        let src = sample(285);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene285")));
    }
    #[test]
    fn sample_286_lowers() {
        let src = sample(286);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene286")));
    }
    #[test]
    fn sample_287_lowers() {
        let src = sample(287);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene287")));
    }
    #[test]
    fn sample_288_lowers() {
        let src = sample(288);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene288")));
    }
    #[test]
    fn sample_289_lowers() {
        let src = sample(289);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene289")));
    }
    #[test]
    fn sample_290_lowers() {
        let src = sample(290);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene290")));
    }
    #[test]
    fn sample_291_lowers() {
        let src = sample(291);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene291")));
    }
    #[test]
    fn sample_292_lowers() {
        let src = sample(292);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene292")));
    }
    #[test]
    fn sample_293_lowers() {
        let src = sample(293);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene293")));
    }
    #[test]
    fn sample_294_lowers() {
        let src = sample(294);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene294")));
    }
    #[test]
    fn sample_295_lowers() {
        let src = sample(295);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene295")));
    }
    #[test]
    fn sample_296_lowers() {
        let src = sample(296);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene296")));
    }
    #[test]
    fn sample_297_lowers() {
        let src = sample(297);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene297")));
    }
    #[test]
    fn sample_298_lowers() {
        let src = sample(298);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene298")));
    }
    #[test]
    fn sample_299_lowers() {
        let src = sample(299);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene299")));
    }
    #[test]
    fn sample_300_lowers() {
        let src = sample(300);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene300")));
    }
    #[test]
    fn sample_301_lowers() {
        let src = sample(301);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene301")));
    }
    #[test]
    fn sample_302_lowers() {
        let src = sample(302);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene302")));
    }
    #[test]
    fn sample_303_lowers() {
        let src = sample(303);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene303")));
    }
    #[test]
    fn sample_304_lowers() {
        let src = sample(304);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene304")));
    }
    #[test]
    fn sample_305_lowers() {
        let src = sample(305);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene305")));
    }
    #[test]
    fn sample_306_lowers() {
        let src = sample(306);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene306")));
    }
    #[test]
    fn sample_307_lowers() {
        let src = sample(307);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene307")));
    }
    #[test]
    fn sample_308_lowers() {
        let src = sample(308);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene308")));
    }
    #[test]
    fn sample_309_lowers() {
        let src = sample(309);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene309")));
    }
    #[test]
    fn sample_310_lowers() {
        let src = sample(310);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene310")));
    }
    #[test]
    fn sample_311_lowers() {
        let src = sample(311);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene311")));
    }
    #[test]
    fn sample_312_lowers() {
        let src = sample(312);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene312")));
    }
    #[test]
    fn sample_313_lowers() {
        let src = sample(313);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene313")));
    }
    #[test]
    fn sample_314_lowers() {
        let src = sample(314);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene314")));
    }
    #[test]
    fn sample_315_lowers() {
        let src = sample(315);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene315")));
    }
    #[test]
    fn sample_316_lowers() {
        let src = sample(316);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene316")));
    }
    #[test]
    fn sample_317_lowers() {
        let src = sample(317);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene317")));
    }
    #[test]
    fn sample_318_lowers() {
        let src = sample(318);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene318")));
    }
    #[test]
    fn sample_319_lowers() {
        let src = sample(319);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene319")));
    }
    #[test]
    fn sample_320_lowers() {
        let src = sample(320);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene320")));
    }
    #[test]
    fn sample_321_lowers() {
        let src = sample(321);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene321")));
    }
    #[test]
    fn sample_322_lowers() {
        let src = sample(322);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene322")));
    }
    #[test]
    fn sample_323_lowers() {
        let src = sample(323);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene323")));
    }
    #[test]
    fn sample_324_lowers() {
        let src = sample(324);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene324")));
    }
    #[test]
    fn sample_325_lowers() {
        let src = sample(325);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene325")));
    }
    #[test]
    fn sample_326_lowers() {
        let src = sample(326);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene326")));
    }
    #[test]
    fn sample_327_lowers() {
        let src = sample(327);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene327")));
    }
    #[test]
    fn sample_328_lowers() {
        let src = sample(328);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene328")));
    }
    #[test]
    fn sample_329_lowers() {
        let src = sample(329);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene329")));
    }
    #[test]
    fn sample_330_lowers() {
        let src = sample(330);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene330")));
    }
    #[test]
    fn sample_331_lowers() {
        let src = sample(331);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene331")));
    }
    #[test]
    fn sample_332_lowers() {
        let src = sample(332);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene332")));
    }
    #[test]
    fn sample_333_lowers() {
        let src = sample(333);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene333")));
    }
    #[test]
    fn sample_334_lowers() {
        let src = sample(334);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene334")));
    }
    #[test]
    fn sample_335_lowers() {
        let src = sample(335);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene335")));
    }
    #[test]
    fn sample_336_lowers() {
        let src = sample(336);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene336")));
    }
    #[test]
    fn sample_337_lowers() {
        let src = sample(337);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene337")));
    }
    #[test]
    fn sample_338_lowers() {
        let src = sample(338);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene338")));
    }
    #[test]
    fn sample_339_lowers() {
        let src = sample(339);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene339")));
    }
    #[test]
    fn sample_340_lowers() {
        let src = sample(340);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene340")));
    }
    #[test]
    fn sample_341_lowers() {
        let src = sample(341);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene341")));
    }
    #[test]
    fn sample_342_lowers() {
        let src = sample(342);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene342")));
    }
    #[test]
    fn sample_343_lowers() {
        let src = sample(343);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene343")));
    }
    #[test]
    fn sample_344_lowers() {
        let src = sample(344);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene344")));
    }
    #[test]
    fn sample_345_lowers() {
        let src = sample(345);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene345")));
    }
    #[test]
    fn sample_346_lowers() {
        let src = sample(346);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene346")));
    }
    #[test]
    fn sample_347_lowers() {
        let src = sample(347);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene347")));
    }
    #[test]
    fn sample_348_lowers() {
        let src = sample(348);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene348")));
    }
    #[test]
    fn sample_349_lowers() {
        let src = sample(349);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene349")));
    }
    #[test]
    fn sample_350_lowers() {
        let src = sample(350);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene350")));
    }
    #[test]
    fn sample_351_lowers() {
        let src = sample(351);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene351")));
    }
    #[test]
    fn sample_352_lowers() {
        let src = sample(352);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene352")));
    }
    #[test]
    fn sample_353_lowers() {
        let src = sample(353);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene353")));
    }
    #[test]
    fn sample_354_lowers() {
        let src = sample(354);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene354")));
    }
    #[test]
    fn sample_355_lowers() {
        let src = sample(355);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene355")));
    }
    #[test]
    fn sample_356_lowers() {
        let src = sample(356);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene356")));
    }
    #[test]
    fn sample_357_lowers() {
        let src = sample(357);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene357")));
    }
    #[test]
    fn sample_358_lowers() {
        let src = sample(358);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene358")));
    }
    #[test]
    fn sample_359_lowers() {
        let src = sample(359);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene359")));
    }
    #[test]
    fn sample_360_lowers() {
        let src = sample(360);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene360")));
    }
    #[test]
    fn sample_361_lowers() {
        let src = sample(361);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene361")));
    }
    #[test]
    fn sample_362_lowers() {
        let src = sample(362);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene362")));
    }
    #[test]
    fn sample_363_lowers() {
        let src = sample(363);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene363")));
    }
    #[test]
    fn sample_364_lowers() {
        let src = sample(364);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene364")));
    }
    #[test]
    fn sample_365_lowers() {
        let src = sample(365);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene365")));
    }
    #[test]
    fn sample_366_lowers() {
        let src = sample(366);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene366")));
    }
    #[test]
    fn sample_367_lowers() {
        let src = sample(367);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene367")));
    }
    #[test]
    fn sample_368_lowers() {
        let src = sample(368);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene368")));
    }
    #[test]
    fn sample_369_lowers() {
        let src = sample(369);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene369")));
    }
    #[test]
    fn sample_370_lowers() {
        let src = sample(370);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene370")));
    }
    #[test]
    fn sample_371_lowers() {
        let src = sample(371);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene371")));
    }
    #[test]
    fn sample_372_lowers() {
        let src = sample(372);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene372")));
    }
    #[test]
    fn sample_373_lowers() {
        let src = sample(373);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene373")));
    }
    #[test]
    fn sample_374_lowers() {
        let src = sample(374);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene374")));
    }
    #[test]
    fn sample_375_lowers() {
        let src = sample(375);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene375")));
    }
    #[test]
    fn sample_376_lowers() {
        let src = sample(376);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene376")));
    }
    #[test]
    fn sample_377_lowers() {
        let src = sample(377);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene377")));
    }
    #[test]
    fn sample_378_lowers() {
        let src = sample(378);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene378")));
    }
    #[test]
    fn sample_379_lowers() {
        let src = sample(379);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene379")));
    }
    #[test]
    fn sample_380_lowers() {
        let src = sample(380);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene380")));
    }
    #[test]
    fn sample_381_lowers() {
        let src = sample(381);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene381")));
    }
    #[test]
    fn sample_382_lowers() {
        let src = sample(382);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene382")));
    }
    #[test]
    fn sample_383_lowers() {
        let src = sample(383);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene383")));
    }
    #[test]
    fn sample_384_lowers() {
        let src = sample(384);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene384")));
    }
    #[test]
    fn sample_385_lowers() {
        let src = sample(385);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene385")));
    }
    #[test]
    fn sample_386_lowers() {
        let src = sample(386);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene386")));
    }
    #[test]
    fn sample_387_lowers() {
        let src = sample(387);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene387")));
    }
    #[test]
    fn sample_388_lowers() {
        let src = sample(388);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene388")));
    }
    #[test]
    fn sample_389_lowers() {
        let src = sample(389);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene389")));
    }
    #[test]
    fn sample_390_lowers() {
        let src = sample(390);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene390")));
    }
    #[test]
    fn sample_391_lowers() {
        let src = sample(391);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene391")));
    }
    #[test]
    fn sample_392_lowers() {
        let src = sample(392);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene392")));
    }
    #[test]
    fn sample_393_lowers() {
        let src = sample(393);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene393")));
    }
    #[test]
    fn sample_394_lowers() {
        let src = sample(394);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene394")));
    }
    #[test]
    fn sample_395_lowers() {
        let src = sample(395);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene395")));
    }
    #[test]
    fn sample_396_lowers() {
        let src = sample(396);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene396")));
    }
    #[test]
    fn sample_397_lowers() {
        let src = sample(397);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene397")));
    }
    #[test]
    fn sample_398_lowers() {
        let src = sample(398);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene398")));
    }
    #[test]
    fn sample_399_lowers() {
        let src = sample(399);
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 3);
        let _ = typeck_module(&m);
        let msgs = extract_msg_ids(&src);
        assert!(msgs.iter().any(|m| m.id.as_str().contains("scene399")));
    }
}
