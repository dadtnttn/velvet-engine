//! Quests and journal.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Quest id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuestId(pub String);

impl QuestId {
    /// Create.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Quest lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    /// Not started.
    Inactive,
    /// Active.
    Active,
    /// Completed.
    Completed,
    /// Failed.
    Failed,
}

/// One objective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestObjective {
    /// Id.
    pub id: String,
    /// Description.
    pub description: String,
    /// Current progress.
    pub current: u32,
    /// Required.
    pub required: u32,
    /// Done.
    pub done: bool,
}

impl QuestObjective {
    /// Create.
    pub fn new(id: impl Into<String>, description: impl Into<String>, required: u32) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            current: 0,
            required: required.max(1),
            done: false,
        }
    }

    /// Add progress.
    pub fn add_progress(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.required);
        self.done = self.current >= self.required;
    }
}

/// Quest definition + runtime state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quest {
    /// Id.
    pub id: QuestId,
    /// Title.
    pub title: String,
    /// Status.
    pub status: QuestStatus,
    /// Objectives.
    pub objectives: Vec<QuestObjective>,
    /// Reward gold.
    pub reward_gold: u32,
    /// Reward xp.
    pub reward_xp: u32,
}

impl Quest {
    /// Create active quest template.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: QuestId::new(id),
            title: title.into(),
            status: QuestStatus::Inactive,
            objectives: Vec::new(),
            reward_gold: 0,
            reward_xp: 0,
        }
    }

    /// Whether all objectives done.
    pub fn objectives_complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|o| o.done)
    }

    /// Progress objective by id.
    pub fn progress(&mut self, objective_id: &str, amount: u32) -> bool {
        if self.status != QuestStatus::Active {
            return false;
        }
        if let Some(o) = self.objectives.iter_mut().find(|o| o.id == objective_id) {
            o.add_progress(amount);
            if self.objectives_complete() {
                self.status = QuestStatus::Completed;
            }
            return true;
        }
        false
    }
}

/// Player quest journal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestJournal {
    /// Quests by id.
    pub quests: IndexMap<String, Quest>,
}

impl QuestJournal {
    /// Start quest.
    pub fn start(&mut self, mut quest: Quest) {
        quest.status = QuestStatus::Active;
        self.quests.insert(quest.id.0.clone(), quest);
    }

    /// Progress.
    pub fn progress(&mut self, quest_id: &str, objective_id: &str, amount: u32) -> bool {
        self.quests
            .get_mut(quest_id)
            .map(|q| q.progress(objective_id, amount))
            .unwrap_or(false)
    }

    /// Is completed.
    pub fn is_completed(&self, quest_id: &str) -> bool {
        self.quests
            .get(quest_id)
            .map(|q| q.status == QuestStatus::Completed)
            .unwrap_or(false)
    }

    /// Whether quest is active.
    pub fn is_active(&self, quest_id: &str) -> bool {
        self.quests
            .get(quest_id)
            .map(|q| q.status == QuestStatus::Active)
            .unwrap_or(false)
    }

    /// Fail an active quest.
    pub fn fail(&mut self, quest_id: &str) -> bool {
        if let Some(q) = self.quests.get_mut(quest_id) {
            if q.status == QuestStatus::Active {
                q.status = QuestStatus::Failed;
                return true;
            }
        }
        false
    }

    /// Active quest ids.
    pub fn active_ids(&self) -> Vec<&str> {
        self.quests
            .values()
            .filter(|q| q.status == QuestStatus::Active)
            .map(|q| q.id.0.as_str())
            .collect()
    }

    /// Completed quest ids.
    pub fn completed_ids(&self) -> Vec<&str> {
        self.quests
            .values()
            .filter(|q| q.status == QuestStatus::Completed)
            .map(|q| q.id.0.as_str())
            .collect()
    }

    /// Get quest.
    pub fn get(&self, quest_id: &str) -> Option<&Quest> {
        self.quests.get(quest_id)
    }

    /// Abandon (remove) a quest.
    pub fn abandon(&mut self, quest_id: &str) -> Option<Quest> {
        self.quests.shift_remove(quest_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_quest() {
        let mut q = Quest::new("q1", "Find cat");
        q.objectives
            .push(QuestObjective::new("find", "Find the cat", 1));
        let mut j = QuestJournal::default();
        j.start(q);
        assert!(j.progress("q1", "find", 1));
        assert!(j.is_completed("q1"));
    }
}
