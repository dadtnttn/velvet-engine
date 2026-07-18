//! Bridge RPG talk targets to velvet-story scene ids.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Dialogue bridge errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DialogueBridgeError {
    /// No mapping for the talk target.
    #[error("no dialogue mapping for target: {0}")]
    UnknownTarget(String),
    /// Mapping exists but is disabled / locked.
    #[error("dialogue locked for target: {0}")]
    Locked(String),
}

/// Conditions gating a dialogue entry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DialogueGate {
    /// Required quest id to be active or completed (optional).
    #[serde(default)]
    pub require_quest: Option<String>,
    /// Required quest status name: "Active" | "Completed" (optional).
    #[serde(default)]
    pub require_quest_status: Option<String>,
    /// Minimum player level (optional).
    #[serde(default)]
    pub min_level: Option<u32>,
    /// Story flag variable that must be truthy (optional).
    #[serde(default)]
    pub require_flag: Option<String>,
    /// When true, mapping is permanently disabled.
    #[serde(default)]
    pub disabled: bool,
}

/// One talk-target → story scene mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueMapping {
    /// Talk target id (NPC id, sign id, etc.).
    pub target_id: String,
    /// Story scene name to start / jump to.
    pub scene_id: String,
    /// Optional label within the scene (`scene:label` form also allowed in scene_id).
    #[serde(default)]
    pub label: Option<String>,
    /// Optional priority when multiple mappings share a target (higher wins).
    #[serde(default)]
    pub priority: i32,
    /// Gate conditions.
    #[serde(default)]
    pub gate: DialogueGate,
    /// Display prompt override.
    #[serde(default)]
    pub prompt: String,
}

impl DialogueMapping {
    /// Create a simple mapping.
    pub fn new(target_id: impl Into<String>, scene_id: impl Into<String>) -> Self {
        Self {
            target_id: target_id.into(),
            scene_id: scene_id.into(),
            label: None,
            priority: 0,
            gate: DialogueGate::default(),
            prompt: String::new(),
        }
    }

    /// Builder: label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builder: priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: prompt.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Builder: gate.
    pub fn with_gate(mut self, gate: DialogueGate) -> Self {
        self.gate = gate;
        self
    }

    /// Resolved jump target string for the story runtime (`scene` or `scene:label`).
    pub fn jump_target(&self) -> String {
        match &self.label {
            Some(l) if !l.is_empty() => format!("{}:{}", self.scene_id, l),
            _ => {
                // Allow scene_id itself to already contain a label.
                self.scene_id.clone()
            }
        }
    }
}

/// Context supplied by the host when resolving dialogue.
#[derive(Debug, Clone, Default)]
pub struct DialogueResolveContext {
    /// Active quest ids.
    pub active_quests: Vec<String>,
    /// Completed quest ids.
    pub completed_quests: Vec<String>,
    /// Player level.
    pub player_level: u32,
    /// Truthy story/world flags.
    pub flags: Vec<String>,
}

impl DialogueResolveContext {
    /// Check a gate against this context.
    pub fn allows(&self, gate: &DialogueGate) -> bool {
        if gate.disabled {
            return false;
        }
        if let Some(min) = gate.min_level {
            if self.player_level < min {
                return false;
            }
        }
        if let Some(flag) = &gate.require_flag {
            if !self.flags.iter().any(|f| f == flag) {
                return false;
            }
        }
        if let Some(qid) = &gate.require_quest {
            let status = gate.require_quest_status.as_deref().unwrap_or("Active");
            let ok = match status {
                "Completed" => self.completed_quests.iter().any(|q| q == qid),
                "Active" => self.active_quests.iter().any(|q| q == qid),
                "Any" => {
                    self.active_quests.iter().any(|q| q == qid)
                        || self.completed_quests.iter().any(|q| q == qid)
                }
                _ => self.active_quests.iter().any(|q| q == qid),
            };
            if !ok {
                return false;
            }
        }
        true
    }
}

/// Registry of talk-target dialogue mappings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogueBridge {
    /// All mappings.
    mappings: Vec<DialogueMapping>,
    /// Optional default scene when target has no mapping.
    pub default_scene: Option<String>,
}

impl DialogueBridge {
    /// Empty bridge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a mapping.
    pub fn register(&mut self, mapping: DialogueMapping) {
        self.mappings.push(mapping);
    }

    /// Register many.
    pub fn register_all<I>(&mut self, mappings: I)
    where
        I: IntoIterator<Item = DialogueMapping>,
    {
        for m in mappings {
            self.register(m);
        }
    }

    /// All mappings.
    pub fn mappings(&self) -> &[DialogueMapping] {
        &self.mappings
    }

    /// Remove all mappings for a target.
    pub fn clear_target(&mut self, target_id: &str) {
        self.mappings.retain(|m| m.target_id != target_id);
    }

    /// Resolve the best mapping for a talk target given world context.
    pub fn resolve(
        &self,
        target_id: &str,
        ctx: &DialogueResolveContext,
    ) -> Result<&DialogueMapping, DialogueBridgeError> {
        let mut candidates: Vec<&DialogueMapping> = self
            .mappings
            .iter()
            .filter(|m| m.target_id == target_id)
            .collect();
        if candidates.is_empty() {
            return Err(DialogueBridgeError::UnknownTarget(target_id.into()));
        }
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
        for m in &candidates {
            if ctx.allows(&m.gate) {
                return Ok(m);
            }
        }
        Err(DialogueBridgeError::Locked(target_id.into()))
    }

    /// Resolve to a jump target string (scene or scene:label).
    pub fn resolve_scene(
        &self,
        target_id: &str,
        ctx: &DialogueResolveContext,
    ) -> Result<String, DialogueBridgeError> {
        match self.resolve(target_id, ctx) {
            Ok(m) => Ok(m.jump_target()),
            Err(DialogueBridgeError::UnknownTarget(_)) => {
                if let Some(def) = &self.default_scene {
                    Ok(def.clone())
                } else {
                    Err(DialogueBridgeError::UnknownTarget(target_id.into()))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Build a map of target → best available jump target for UI prompts.
    pub fn available_targets(&self, ctx: &DialogueResolveContext) -> IndexMap<String, String> {
        let mut out = IndexMap::new();
        let mut targets: Vec<String> = self.mappings.iter().map(|m| m.target_id.clone()).collect();
        targets.sort();
        targets.dedup();
        for t in targets {
            if let Ok(scene) = self.resolve_scene(&t, ctx) {
                out.insert(t, scene);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_priority_and_gate() {
        let mut bridge = DialogueBridge::new();
        bridge.register(
            DialogueMapping::new("npc_mira", "mira_intro")
                .with_priority(0)
                .with_prompt("Talk"),
        );
        bridge.register(
            DialogueMapping::new("npc_mira", "mira_quest")
                .with_priority(10)
                .with_gate(DialogueGate {
                    require_quest: Some("find_relic".into()),
                    require_quest_status: Some("Active".into()),
                    ..Default::default()
                }),
        );
        let ctx = DialogueResolveContext {
            active_quests: vec!["find_relic".into()],
            player_level: 3,
            ..Default::default()
        };
        let scene = bridge.resolve_scene("npc_mira", &ctx).unwrap();
        assert_eq!(scene, "mira_quest");

        let ctx2 = DialogueResolveContext {
            player_level: 1,
            ..Default::default()
        };
        let scene2 = bridge.resolve_scene("npc_mira", &ctx2).unwrap();
        assert_eq!(scene2, "mira_intro");
    }

    #[test]
    fn locked_and_unknown() {
        let mut bridge = DialogueBridge::new();
        bridge.register(
            DialogueMapping::new("boss", "boss_talk").with_gate(DialogueGate {
                min_level: Some(10),
                ..Default::default()
            }),
        );
        let ctx = DialogueResolveContext {
            player_level: 2,
            ..Default::default()
        };
        assert!(matches!(
            bridge.resolve("boss", &ctx),
            Err(DialogueBridgeError::Locked(_))
        ));
        assert!(matches!(
            bridge.resolve("nope", &ctx),
            Err(DialogueBridgeError::UnknownTarget(_))
        ));
    }

    #[test]
    fn jump_target_with_label() {
        let m = DialogueMapping::new("sign", "town").with_label("plaza");
        assert_eq!(m.jump_target(), "town:plaza");
    }

    #[test]
    fn default_scene_fallback() {
        let mut bridge = DialogueBridge::new();
        bridge.default_scene = Some("fallback".into());
        let ctx = DialogueResolveContext::default();
        assert_eq!(bridge.resolve_scene("ghost", &ctx).unwrap(), "fallback");
    }

    #[test]
    fn available_targets_lists() {
        let mut bridge = DialogueBridge::new();
        bridge.register(DialogueMapping::new("a", "sa"));
        bridge.register(DialogueMapping::new("b", "sb").with_gate(DialogueGate {
            disabled: true,
            ..Default::default()
        }));
        let map = bridge.available_targets(&DialogueResolveContext::default());
        assert!(map.contains_key("a"));
        assert!(!map.contains_key("b"));
    }
}
