//! Audio buses / categories.

use serde::{Deserialize, Serialize};

/// Well-known bus categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusKind {
    /// Final output.
    Master,
    /// Background music.
    Music,
    /// Character voice.
    Voice,
    /// Sound effects.
    Effects,
    /// Ambient loops.
    Ambient,
    /// UI clicks / feedback.
    Ui,
}

impl BusKind {
    /// Stable string id.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Voice => "voice",
            Self::Effects => "effects",
            Self::Ambient => "ambient",
            Self::Ui => "ui",
        }
    }

    /// Default ordered set.
    pub fn all() -> &'static [BusKind] {
        &[
            Self::Master,
            Self::Music,
            Self::Voice,
            Self::Effects,
            Self::Ambient,
            Self::Ui,
        ]
    }
}

/// Bus identifier (named).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BusId(pub String);

impl BusId {
    /// From kind.
    pub fn from_kind(kind: BusKind) -> Self {
        Self(kind.as_str().into())
    }

    /// As str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<BusKind> for BusId {
    fn from(value: BusKind) -> Self {
        Self::from_kind(value)
    }
}

/// Runtime bus state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBus {
    /// Id.
    pub id: BusId,
    /// Kind if known.
    pub kind: Option<BusKind>,
    /// Linear volume 0..=1 (pre-parent).
    pub volume: f32,
    /// Muted independently of volume.
    pub muted: bool,
    /// Parent bus (Master has none).
    pub parent: Option<BusId>,
}

impl AudioBus {
    /// Create bus.
    pub fn new(kind: BusKind) -> Self {
        let parent = match kind {
            BusKind::Master => None,
            _ => Some(BusId::from_kind(BusKind::Master)),
        };
        Self {
            id: BusId::from_kind(kind),
            kind: Some(kind),
            volume: 1.0,
            muted: false,
            parent,
        }
    }

    /// Effective volume contribution (local only).
    pub fn local_gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume.clamp(0.0, 2.0)
        }
    }
}
