//! Party members.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::inventory::Inventory;
use crate::stats::{LevelProgress, StatBlock};

/// One party member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Stats.
    pub stats: StatBlock,
    /// Level.
    pub level: LevelProgress,
    /// Inventory.
    pub inventory: Inventory,
    /// In active battle party.
    pub active: bool,
}

impl PartyMember {
    /// Create.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            stats: StatBlock::default(),
            level: LevelProgress::default(),
            inventory: Inventory::with_capacity(30),
            active: true,
        }
    }
}

/// Party roster.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Party {
    /// Members.
    pub members: IndexMap<String, PartyMember>,
    /// Leader id.
    pub leader: Option<String>,
}

impl Party {
    /// Add member; first becomes leader.
    pub fn add(&mut self, member: PartyMember) {
        if self.leader.is_none() {
            self.leader = Some(member.id.clone());
        }
        self.members.insert(member.id.clone(), member);
    }

    /// Leader mut.
    pub fn leader_mut(&mut self) -> Option<&mut PartyMember> {
        let id = self.leader.clone()?;
        self.members.get_mut(&id)
    }

    /// Leader.
    pub fn leader(&self) -> Option<&PartyMember> {
        self.leader.as_ref().and_then(|id| self.members.get(id))
    }
}
