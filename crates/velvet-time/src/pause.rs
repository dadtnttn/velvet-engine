//! Hierarchical pause layers for clocks and subsystems.

use std::collections::BTreeSet;

/// Named pause layer. Higher priority layers can block lower ones independently.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PauseLayer(pub String);

impl PauseLayer {
    /// Create layer id.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Borrow name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PauseLayer {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Well-known layers.
pub mod layers {
    use super::PauseLayer;

    /// Full game pause (menus).
    pub fn game() -> PauseLayer {
        PauseLayer::new("game")
    }
    /// Dialogue / cutscene pause for gameplay systems.
    pub fn dialogue() -> PauseLayer {
        PauseLayer::new("dialogue")
    }
    /// Editor / debug pause.
    pub fn debug() -> PauseLayer {
        PauseLayer::new("debug")
    }
    /// System modal (save dialog).
    pub fn modal() -> PauseLayer {
        PauseLayer::new("modal")
    }
}

/// Mask describing which subsystems respect pause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PauseMask {
    /// Gameplay simulation.
    pub gameplay: bool,
    /// UI animations.
    pub ui: bool,
    /// Audio (non-UI).
    pub audio: bool,
    /// Particles / cosmetics.
    pub cosmetics: bool,
}

impl PauseMask {
    /// Everything paused.
    pub const ALL: Self = Self {
        gameplay: true,
        ui: true,
        audio: true,
        cosmetics: true,
    };

    /// Only gameplay.
    pub const GAMEPLAY_ONLY: Self = Self {
        gameplay: true,
        ui: false,
        audio: false,
        cosmetics: false,
    };

    /// Gameplay + cosmetics (UI still runs).
    pub const WORLD: Self = Self {
        gameplay: true,
        ui: false,
        audio: false,
        cosmetics: true,
    };

    /// Combine with OR semantics.
    pub fn union(self, other: Self) -> Self {
        Self {
            gameplay: self.gameplay || other.gameplay,
            ui: self.ui || other.ui,
            audio: self.audio || other.audio,
            cosmetics: self.cosmetics || other.cosmetics,
        }
    }

    /// Whether any flag is set.
    pub fn any(self) -> bool {
        self.gameplay || self.ui || self.audio || self.cosmetics
    }
}

/// Stack of active pause layers with per-layer masks.
#[derive(Debug, Clone, Default)]
pub struct PauseStack {
    /// Active layers and their masks (order = push order).
    entries: Vec<(PauseLayer, PauseMask)>,
}

impl PauseStack {
    /// Empty stack (nothing paused).
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a layer (or replace mask if already present).
    pub fn push(&mut self, layer: impl Into<PauseLayer>, mask: PauseMask) {
        let layer = layer.into();
        if let Some((_, m)) = self.entries.iter_mut().find(|(l, _)| *l == layer) {
            *m = mask;
        } else {
            self.entries.push((layer, mask));
        }
    }

    /// Pop a specific layer.
    pub fn pop(&mut self, layer: &PauseLayer) -> bool {
        if let Some(i) = self.entries.iter().position(|(l, _)| l == layer) {
            self.entries.remove(i);
            true
        } else {
            false
        }
    }

    /// Pop the most recently pushed layer.
    pub fn pop_top(&mut self) -> Option<(PauseLayer, PauseMask)> {
        self.entries.pop()
    }

    /// Clear all pauses.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Whether a layer is active.
    pub fn is_active(&self, layer: &PauseLayer) -> bool {
        self.entries.iter().any(|(l, _)| l == layer)
    }

    /// Combined mask (OR of all layers).
    pub fn combined_mask(&self) -> PauseMask {
        self.entries
            .iter()
            .fold(PauseMask::default(), |acc, (_, m)| acc.union(*m))
    }

    /// Whether gameplay is paused.
    pub fn gameplay_paused(&self) -> bool {
        self.combined_mask().gameplay
    }

    /// Whether UI is paused.
    pub fn ui_paused(&self) -> bool {
        self.combined_mask().ui
    }

    /// Effective time scale for a subsystem given base scale.
    pub fn scale_for(&self, base_scale: f32, subsystem_gameplay: bool) -> f32 {
        if subsystem_gameplay && self.gameplay_paused() {
            0.0
        } else {
            base_scale.max(0.0)
        }
    }

    /// Number of active layers.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Active layer names.
    pub fn layers(&self) -> impl Iterator<Item = &PauseLayer> {
        self.entries.iter().map(|(l, _)| l)
    }

    /// Snapshot of active layer names sorted.
    pub fn layer_names_sorted(&self) -> BTreeSet<String> {
        self.entries.iter().map(|(l, _)| l.0.clone()).collect()
    }
}

/// Clock that multiplies delta by scale and respects a pause stack.
#[derive(Debug, Clone)]
pub struct LayeredClock {
    /// Base time scale.
    pub time_scale: f32,
    /// Pause layers.
    pub pauses: PauseStack,
    /// Elapsed scaled gameplay seconds.
    elapsed_gameplay: f64,
    /// Elapsed unscaled seconds.
    elapsed_unscaled: f64,
}

impl Default for LayeredClock {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredClock {
    /// Create.
    pub fn new() -> Self {
        Self {
            time_scale: 1.0,
            pauses: PauseStack::new(),
            elapsed_gameplay: 0.0,
            elapsed_unscaled: 0.0,
        }
    }

    /// Advance with raw wall delta; returns (gameplay_delta, unscaled_delta).
    pub fn advance(&mut self, raw_delta: f32) -> (f32, f32) {
        let raw = raw_delta.max(0.0);
        self.elapsed_unscaled += f64::from(raw);
        let g = if self.pauses.gameplay_paused() {
            0.0
        } else {
            raw * self.time_scale.max(0.0)
        };
        self.elapsed_gameplay += f64::from(g);
        (g, raw)
    }

    /// Elapsed gameplay seconds.
    pub fn elapsed_gameplay(&self) -> f64 {
        self.elapsed_gameplay
    }

    /// Elapsed unscaled seconds.
    pub fn elapsed_unscaled(&self) -> f64 {
        self.elapsed_unscaled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_union_mask() {
        let mut s = PauseStack::new();
        s.push(layers::game(), PauseMask::GAMEPLAY_ONLY);
        s.push(layers::modal(), PauseMask::ALL);
        assert!(s.gameplay_paused());
        assert!(s.ui_paused());
        s.pop(&layers::modal());
        assert!(s.gameplay_paused());
        assert!(!s.ui_paused());
    }

    #[test]
    fn layered_clock_pauses() {
        let mut c = LayeredClock::new();
        let (g, u) = c.advance(0.1);
        assert!((g - 0.1).abs() < 1e-5);
        assert!((u - 0.1).abs() < 1e-5);
        c.pauses.push(layers::game(), PauseMask::GAMEPLAY_ONLY);
        let (g2, u2) = c.advance(0.1);
        assert_eq!(g2, 0.0);
        assert!((u2 - 0.1).abs() < 1e-5);
    }

    #[test]
    fn replace_layer_mask() {
        let mut s = PauseStack::new();
        s.push("x", PauseMask::GAMEPLAY_ONLY);
        s.push("x", PauseMask::ALL);
        assert_eq!(s.len(), 1);
        assert!(s.ui_paused());
    }
}
