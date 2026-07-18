//! VS2 lowering helpers from HIR modules to story-ish ops.

#![allow(missing_docs)]

use velvet_script_hir::{HirItem, HirModule};

/// Count story-like items in module.
pub fn count_story_ops(m: &HirModule) -> usize {
    let mut n = 0;
    for it in &m.items {
        if let HirItem::Scene(sc) = it {
            n += sc.body.len() + 1;
        }
    }
    n
}

/// List scene names.
pub fn scene_names(m: &HirModule) -> Vec<String> {
    m.items
        .iter()
        .filter_map(|it| match it {
            HirItem::Scene(s) => Some(s.name.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the module has any narrative (scene) items.
pub fn is_story_module(m: &HirModule) -> bool {
    m.items.iter().any(|it| matches!(it, HirItem::Scene(_)))
}

/// Character names declared in the module.
pub fn character_names(m: &HirModule) -> Vec<String> {
    m.items
        .iter()
        .filter_map(|it| match it {
            HirItem::Character(c) => Some(c.name.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::{HirId, HirItem, HirModule, HirScene, HirSpan};

    #[test]
    fn counts_scenes() {
        let mut m = HirModule::new(2);
        m.items.push(HirItem::Scene(HirScene {
            id: HirId(1),
            name: "start".into(),
            body: vec![],
            span: HirSpan::unknown(),
        }));
        assert_eq!(scene_names(&m), vec!["start".to_string()]);
        assert!(is_story_module(&m));
        assert_eq!(count_story_ops(&m), 1);
    }
}
