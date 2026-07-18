//! Simulation events for game authors.

use serde::{Deserialize, Serialize};

use crate::cell::MaterialId;

/// Something interesting happened in the sim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimEvent {
    /// Cell changed material at world position.
    MaterialChanged {
        /// World X.
        x: i32,
        /// World Y.
        y: i32,
        /// Previous.
        from: MaterialId,
        /// New.
        to: MaterialId,
    },
    /// Fire started.
    Ignited {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Material burning.
        material: MaterialId,
    },
    /// Explosion.
    Exploded {
        /// Center X.
        x: i32,
        /// Center Y.
        y: i32,
        /// Radius.
        radius: u8,
    },
    /// Dissolution.
    Dissolved {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Target that was removed.
        target: MaterialId,
        /// Agent (acid etc.).
        agent: MaterialId,
    },
    /// Rigid body hit ground / wall.
    BodyContact {
        /// Body id.
        body_id: u32,
        /// Impact speed.
        speed: f32,
    },
    /// Chunk loaded into memory.
    ChunkLoaded {
        /// Chunk x.
        cx: i32,
        /// Chunk y.
        cy: i32,
    },
    /// Chunk unloaded.
    ChunkUnloaded {
        /// Chunk x.
        cx: i32,
        /// Chunk y.
        cy: i32,
    },
    /// Enemy died (creator combat hook).
    EnemyDied {
        /// Enemy instance id.
        id: u32,
        /// World X.
        x: f32,
        /// World Y.
        y: f32,
        /// Blueprint key.
        def_key: String,
    },
    /// Enemy spawned.
    EnemySpawned {
        /// Id.
        id: u32,
        /// Def key.
        def_key: String,
        /// X.
        x: f32,
        /// Y.
        y: f32,
    },
    /// Free particle converted into a grid cell.
    ParticleConverted {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Material written.
        material: MaterialId,
    },
    /// Agent died.
    AgentDied {
        /// Agent id.
        id: u32,
        /// X.
        x: f32,
        /// Y.
        y: f32,
    },
    /// Terrain dug by agent/tool.
    TerrainDug {
        /// Center X.
        x: i32,
        /// Center Y.
        y: i32,
        /// Radius.
        radius: i32,
        /// Cells removed.
        cells: u32,
    },
    /// Blood splatter created.
    BloodSplatter {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Radius.
        radius: i32,
    },
}

/// Ring buffer of events for one frame / step.
#[derive(Debug, Clone, Default)]
pub struct EventQueue {
    events: Vec<SimEvent>,
    cap: usize,
}

impl EventQueue {
    /// Create with capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            events: Vec::with_capacity(cap.min(4096)),
            cap: cap.max(16),
        }
    }

    /// Push (drops oldest if over cap).
    pub fn push(&mut self, e: SimEvent) {
        if self.events.len() >= self.cap {
            self.events.remove(0);
        }
        self.events.push(e);
    }

    /// Drain all events.
    pub fn drain(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    /// Slice view.
    pub fn as_slice(&self) -> &[SimEvent] {
        &self.events
    }

    /// Clear.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
