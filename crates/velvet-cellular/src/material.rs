//! Material definitions and registry — fully data-driven for authors.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cell::MaterialId;

/// Macro phase of matter for simulation routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Empty / vacuum.
    #[default]
    Gas,
    /// Falling powders (sand, dust, ash).
    Powder,
    /// Flowing liquids (water, oil, lava, acid).
    Liquid,
    /// Static solids (stone, wood, metal, ice).
    Solid,
    /// Immovable static (bedrock, world border).
    Static,
    /// Fire-like ascending / short-lived.
    Plasma,
}

/// How a material interacts under gravity / density.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicalProps {
    /// Density (higher sinks through lower). Water ≈ 1.0.
    pub density: f32,
    /// Friction 0..=1 when sliding on solids.
    pub friction: f32,
    /// Viscosity / flow resistance (liquids).
    pub viscosity: f32,
    /// How much temperature this holds (specific heat proxy).
    pub heat_capacity: f32,
    /// Thermal conductivity 0..=1.
    pub conductivity: f32,
    /// Melting point °C (solid → liquid material).
    pub melt_point: Option<f32>,
    /// Boiling / vaporize point °C.
    pub boil_point: Option<f32>,
    /// Material to become when melted.
    pub melt_into: Option<MaterialId>,
    /// Material when boiled.
    pub boil_into: Option<MaterialId>,
    /// Freeze point (liquid → solid).
    pub freeze_point: Option<f32>,
    /// Material when frozen.
    pub freeze_into: Option<MaterialId>,
}

impl Default for PhysicalProps {
    fn default() -> Self {
        Self {
            density: 1.0,
            friction: 0.2,
            viscosity: 0.0,
            heat_capacity: 1.0,
            conductivity: 0.3,
            melt_point: None,
            boil_point: None,
            melt_into: None,
            boil_into: None,
            freeze_point: None,
            freeze_into: None,
        }
    }
}

/// Combustion / chemistry flags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReactionProps {
    /// Can catch fire.
    pub flammable: bool,
    /// Ignition temperature °C.
    pub ignite_temp: f32,
    /// Burn duration ticks (0 = instant consume).
    pub burn_life: u16,
    /// Heat released per burn tick.
    pub burn_heat: f32,
    /// Material produced while burning (smoke, ash).
    pub burn_product: Option<MaterialId>,
    /// Residue when fully burned.
    pub burn_residue: Option<MaterialId>,
    /// Dissolves these material ids (acid, etc.).
    pub dissolves: Vec<MaterialId>,
    /// Dissolution rate ticks per target cell.
    pub dissolve_rate: u16,
    /// Explosive when hot / shocked.
    pub explosive: bool,
    /// Explosion radius in cells.
    pub explosion_radius: u8,
    /// Freezes water-like neighbors.
    pub freezes: bool,
    /// Extinguishes fire.
    pub extinguishes: bool,
}

/// Author-facing material definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialDef {
    /// Short stable key: `"sand"`, `"water"`.
    pub key: String,
    /// Display name.
    pub name: String,
    /// Phase.
    pub phase: Phase,
    /// Physics.
    pub physics: PhysicalProps,
    /// Chemistry / fire.
    pub reaction: ReactionProps,
    /// RGBA 0..=255 for default render color.
    pub color: [u8; 4],
    /// Color noise (± per channel) for organic look.
    pub color_variance: u8,
    /// Whether gravity applies.
    pub affected_by_gravity: bool,
    /// Custom author tags (`"organic"`, `"metal"`, …).
    pub tags: Vec<String>,
}

impl MaterialDef {
    /// Builder start.
    pub fn new(key: impl Into<String>, name: impl Into<String>, phase: Phase) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            phase,
            physics: PhysicalProps::default(),
            reaction: ReactionProps::default(),
            color: [200, 200, 200, 255],
            color_variance: 8,
            affected_by_gravity: !matches!(phase, Phase::Solid | Phase::Static | Phase::Gas),
            tags: Vec::new(),
        }
    }

    /// Density.
    pub fn density(mut self, d: f32) -> Self {
        self.physics.density = d;
        self
    }

    /// Color RGBA.
    pub fn color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// Flammable.
    pub fn flammable(mut self, ignite: f32, life: u16) -> Self {
        self.reaction.flammable = true;
        self.reaction.ignite_temp = ignite;
        self.reaction.burn_life = life;
        self
    }

    /// Tag.
    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.tags.push(t.into());
        self
    }
}

/// Registry errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MaterialError {
    /// Duplicate key.
    #[error("duplicate material key: {0}")]
    DuplicateKey(String),
    /// Unknown key.
    #[error("unknown material: {0}")]
    Unknown(String),
    /// Registry full (u16 space).
    #[error("material registry full")]
    Full,
}

/// Material registry — id 0 is always air.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialRegistry {
    defs: Vec<MaterialDef>,
    by_key: IndexMap<String, MaterialId>,
}

impl Default for MaterialRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialRegistry {
    /// New registry with air only.
    pub fn new() -> Self {
        let air = MaterialDef {
            key: "air".into(),
            name: "Air".into(),
            phase: Phase::Gas,
            physics: PhysicalProps {
                density: 0.001,
                ..PhysicalProps::default()
            },
            reaction: ReactionProps::default(),
            color: [0, 0, 0, 0],
            color_variance: 0,
            affected_by_gravity: false,
            tags: vec!["gas".into()],
        };
        let mut by_key = IndexMap::new();
        by_key.insert("air".into(), MaterialId::AIR);
        Self {
            defs: vec![air],
            by_key,
        }
    }

    /// Register a material; returns assigned id.
    pub fn register(&mut self, def: MaterialDef) -> Result<MaterialId, MaterialError> {
        if self.by_key.contains_key(&def.key) {
            return Err(MaterialError::DuplicateKey(def.key));
        }
        if self.defs.len() >= u16::MAX as usize {
            return Err(MaterialError::Full);
        }
        let id = MaterialId(self.defs.len() as u16);
        self.by_key.insert(def.key.clone(), id);
        self.defs.push(def);
        Ok(id)
    }

    /// Lookup by key.
    pub fn id(&self, key: &str) -> Result<MaterialId, MaterialError> {
        self.by_key
            .get(key)
            .copied()
            .ok_or_else(|| MaterialError::Unknown(key.into()))
    }

    /// Definition by id.
    pub fn get(&self, id: MaterialId) -> &MaterialDef {
        &self.defs[id.0 as usize]
    }

    /// Try get.
    pub fn try_get(&self, id: MaterialId) -> Option<&MaterialDef> {
        self.defs.get(id.0 as usize)
    }

    /// All definitions in id order.
    pub fn all(&self) -> &[MaterialDef] {
        &self.defs
    }

    /// Count including air.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Empty never (always has air).
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Phase helper.
    pub fn phase(&self, id: MaterialId) -> Phase {
        self.get(id).phase
    }

    /// Density helper.
    pub fn density(&self, id: MaterialId) -> f32 {
        self.get(id).physics.density
    }

    /// Whether gravity applies.
    pub fn gravity_applies(&self, id: MaterialId) -> bool {
        self.get(id).affected_by_gravity
    }

    /// Keys list.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.by_key.keys().map(|s| s.as_str())
    }

    /// Replace definition at id (for post-register patches / author tools).
    pub fn set_def(&mut self, id: MaterialId, def: MaterialDef) {
        if let Some(slot) = self.defs.get_mut(id.0 as usize) {
            // keep key map consistent if key changes
            if slot.key != def.key {
                self.by_key.shift_remove(&slot.key);
                self.by_key.insert(def.key.clone(), id);
            }
            *slot = def;
        }
    }
}
