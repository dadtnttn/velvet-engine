//! Dialogue history.

use serde::{Deserialize, Serialize};

/// One history line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Speaker display name (empty for narrator).
    pub speaker: String,
    /// Fully interpolated text.
    pub text: String,
    /// Scene name when shown.
    pub scene: String,
    /// Monotonic index.
    pub index: u64,
}

/// Rolling history buffer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    entries: Vec<HistoryEntry>,
    next_index: u64,
    /// Max retained entries (0 = unlimited).
    pub capacity: usize,
}

impl History {
    /// Create with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            next_index: 0,
            capacity,
        }
    }

    /// Push a line.
    pub fn push(
        &mut self,
        speaker: impl Into<String>,
        text: impl Into<String>,
        scene: impl Into<String>,
    ) {
        let entry = HistoryEntry {
            speaker: speaker.into(),
            text: text.into(),
            scene: scene.into(),
            index: self.next_index,
        };
        self.next_index += 1;
        self.entries.push(entry);
        if self.capacity > 0 && self.entries.len() > self.capacity {
            let drop_n = self.entries.len() - self.capacity;
            self.entries.drain(0..drop_n);
        }
    }

    /// All entries.
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Clear.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
