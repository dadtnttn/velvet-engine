//! Category mixer: master / bus / voice effective volume.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::bus::{AudioBus, BusId, BusKind};

/// Per-category mixer state with mute flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMixer {
    /// Master linear volume.
    pub master_volume: f32,
    /// Master mute.
    pub master_muted: bool,
    /// Per-bus volumes and mutes.
    buses: HashMap<String, BusMix>,
}

/// Local bus mix parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BusMix {
    /// Linear volume 0..=2.
    pub volume: f32,
    /// Muted.
    pub muted: bool,
}

impl Default for BusMix {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
        }
    }
}

impl Default for CategoryMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoryMixer {
    /// Create with default categories for all [`BusKind`]s.
    pub fn new() -> Self {
        let mut buses = HashMap::new();
        for kind in BusKind::all() {
            if *kind == BusKind::Master {
                continue;
            }
            buses.insert(kind.as_str().to_string(), BusMix::default());
        }
        Self {
            master_volume: 1.0,
            master_muted: false,
            buses,
        }
    }

    /// Set master volume.
    pub fn set_master(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 2.0);
    }

    /// Mute / unmute master.
    pub fn set_master_muted(&mut self, muted: bool) {
        self.master_muted = muted;
    }

    /// Set bus volume by kind.
    pub fn set_bus_volume(&mut self, kind: BusKind, volume: f32) {
        if kind == BusKind::Master {
            self.set_master(volume);
            return;
        }
        self.buses.entry(kind.as_str().into()).or_default().volume = volume.clamp(0.0, 2.0);
    }

    /// Set bus mute by kind.
    pub fn set_bus_muted(&mut self, kind: BusKind, muted: bool) {
        if kind == BusKind::Master {
            self.master_muted = muted;
            return;
        }
        self.buses.entry(kind.as_str().into()).or_default().muted = muted;
    }

    /// Bus mix for id.
    pub fn bus_mix(&self, id: &BusId) -> BusMix {
        if id.as_str() == BusKind::Master.as_str() {
            return BusMix {
                volume: self.master_volume,
                muted: self.master_muted,
            };
        }
        self.buses.get(id.as_str()).copied().unwrap_or_default()
    }

    /// Local gain for a bus (0 if muted).
    pub fn bus_gain(&self, id: &BusId) -> f32 {
        let m = self.bus_mix(id);
        if m.muted {
            0.0
        } else {
            m.volume
        }
    }

    /// Master gain (0 if muted).
    pub fn master_gain(&self) -> f32 {
        if self.master_muted {
            0.0
        } else {
            self.master_volume
        }
    }

    /// Effective output volume: `master * bus * voice`.
    pub fn effective_volume(&self, bus: &BusId, voice_volume: f32) -> f32 {
        self.master_gain() * self.bus_gain(bus) * voice_volume.clamp(0.0, 2.0)
    }

    /// Effective volume using [`BusKind`].
    pub fn effective_volume_kind(&self, kind: BusKind, voice_volume: f32) -> f32 {
        self.effective_volume(&BusId::from_kind(kind), voice_volume)
    }

    /// Apply mixer state onto engine buses (sync).
    pub fn apply_to_buses(&self, buses: &mut HashMap<String, AudioBus>) {
        if let Some(master) = buses.get_mut(BusKind::Master.as_str()) {
            master.volume = self.master_volume;
            master.muted = self.master_muted;
        }
        for (name, mix) in &self.buses {
            if let Some(b) = buses.get_mut(name) {
                b.volume = mix.volume;
                b.muted = mix.muted;
            }
        }
    }

    /// Snapshot mixer from engine buses.
    pub fn from_buses(buses: &HashMap<String, AudioBus>) -> Self {
        let mut mixer = Self::new();
        if let Some(master) = buses.get(BusKind::Master.as_str()) {
            mixer.master_volume = master.volume;
            mixer.master_muted = master.muted;
        }
        for kind in BusKind::all() {
            if *kind == BusKind::Master {
                continue;
            }
            if let Some(b) = buses.get(kind.as_str()) {
                mixer.buses.insert(
                    kind.as_str().into(),
                    BusMix {
                        volume: b.volume,
                        muted: b.muted,
                    },
                );
            }
        }
        mixer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_is_product() {
        let mut m = CategoryMixer::new();
        m.set_master(0.5);
        m.set_bus_volume(BusKind::Effects, 0.5);
        let g = m.effective_volume_kind(BusKind::Effects, 0.5);
        assert!((g - 0.125).abs() < 1e-5);
    }

    #[test]
    fn mute_zeros() {
        let mut m = CategoryMixer::new();
        m.set_bus_muted(BusKind::Music, true);
        assert_eq!(m.effective_volume_kind(BusKind::Music, 1.0), 0.0);
        m.set_bus_muted(BusKind::Music, false);
        m.set_master_muted(true);
        assert_eq!(m.effective_volume_kind(BusKind::Music, 1.0), 0.0);
    }
}
