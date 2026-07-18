//! Cell storage — one simulation site in the world grid.

use serde::{Deserialize, Serialize};

/// Stable material identifier (index into the registry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct MaterialId(pub u16);

impl MaterialId {
    /// Empty / air.
    pub const AIR: MaterialId = MaterialId(0);
    /// Whether this is air.
    pub fn is_air(self) -> bool {
        self.0 == 0
    }
}

/// Per-cell frame flags (bitmask, no external bitflags crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CellFlags(pub u8);

impl CellFlags {
    /// Already processed this sub-step.
    pub const MOVED: CellFlags = CellFlags(0b0000_0001);
    /// Settled.
    pub const SETTLED: CellFlags = CellFlags(0b0000_0010);
    /// On fire.
    pub const BURNING: CellFlags = CellFlags(0b0000_0100);
    /// Dirty for render.
    pub const DIRTY: CellFlags = CellFlags(0b0000_1000);
    /// Kinematic coupling.
    pub const KINEMATIC: CellFlags = CellFlags(0b0001_0000);

    /// Empty flags.
    pub const fn empty() -> Self {
        CellFlags(0)
    }

    /// Contains bits.
    pub const fn contains(self, other: CellFlags) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Insert bits.
    pub fn insert(&mut self, other: CellFlags) {
        self.0 |= other.0;
    }

    /// Remove bits.
    pub fn remove(&mut self, other: CellFlags) {
        self.0 &= !other.0;
    }
}

/// Per-cell runtime state packed for cache-friendly simulation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Material occupying this site.
    pub material: MaterialId,
    /// Temperature in °C (simulation units).
    pub temp: f32,
    /// Free pressure / stress scalar (0 = ambient).
    pub pressure: f32,
    /// Lifetime / burn / decay counter (material-specific).
    pub life: u16,
    /// Extra data byte (variant, color jitter, frame).
    pub meta: u8,
    /// Bit flags for this frame (updated, settled, …).
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Self::air()
    }
}

impl Cell {
    /// Empty air cell at ambient temperature.
    pub fn air() -> Self {
        Self {
            material: MaterialId::AIR,
            temp: 20.0,
            pressure: 0.0,
            life: 0,
            meta: 0,
            flags: CellFlags::empty(),
        }
    }

    /// Solid / powder / liquid cell of a material.
    pub fn of(material: MaterialId) -> Self {
        Self {
            material,
            temp: 20.0,
            pressure: 0.0,
            life: 0,
            meta: 0,
            flags: CellFlags::empty(),
        }
    }

    /// With temperature.
    pub fn with_temp(mut self, temp: f32) -> Self {
        self.temp = temp;
        self
    }

    /// With life counter.
    pub fn with_life(mut self, life: u16) -> Self {
        self.life = life;
        self
    }

    /// Whether empty.
    pub fn is_air(self) -> bool {
        self.material.is_air()
    }
}
