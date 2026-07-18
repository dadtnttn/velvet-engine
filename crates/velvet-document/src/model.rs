//! Document and region types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable region identifier (e.g. `button.start`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionId(String);

impl RegionId {
    /// Create from string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow as str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RegionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Classification of a document region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegionKind {
    /// Editable by simplified visual tools (layout, text, assets).
    Visual,
    /// Hand-written logic; preserved byte-for-byte by visual tools.
    Advanced,
    /// Plugin / external blocks; visual mode must not edit.
    Protected,
    /// Content outside marked regions (structural scaffolding).
    External,
}

impl RegionKind {
    /// Marker tag without `@`.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Visual => "visual",
            Self::Advanced => "advanced",
            Self::Protected => "protected",
            Self::External => "external",
        }
    }
}

/// A visual-mode property (`key: value`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualProperty {
    /// Property name.
    pub key: String,
    /// Property value.
    pub value: PropertyValue,
    /// Original line indentation (spaces/tabs) for pretty re-emit.
    pub indent: String,
    /// Trailing comment on the same line, if any (including `//`).
    pub trailing_comment: Option<String>,
}

/// Parsed or opaque property value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    /// Quoted string.
    String(String),
    /// Unparsed raw text (numbers, tuples, identifiers, unknowns).
    Raw(String),
}

impl PropertyValue {
    /// Render value for source emission.
    pub fn render(&self) -> String {
        match self {
            Self::String(s) => format!("\"{}\"", escape_string(s)),
            Self::Raw(s) => s.clone(),
        }
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// One region of the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// Kind.
    pub kind: RegionKind,
    /// Stable id (empty for pure external preamble chunks).
    pub id: RegionId,
    /// Raw body text (for Advanced/Protected/External) or reconstructed visual body.
    pub body: String,
    /// Parsed visual properties (only meaningful for Visual).
    pub properties: Vec<VisualProperty>,
    /// Lines that were not recognized as `key: value` inside a visual region (kept as-is).
    pub raw_lines: Vec<String>,
    /// Whether this region was opened with an explicit marker comment.
    pub marked: bool,
}

/// Full document: ordered regions + freeform leading/trailing external text handled as regions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Document {
    /// Ordered regions spanning the entire file.
    pub regions: Vec<Region>,
    /// Source path hint (optional).
    pub path: Option<String>,
}

impl Document {
    /// Find region by id and kind.
    pub fn find(&self, kind: RegionKind, id: &str) -> Option<&Region> {
        self.regions
            .iter()
            .find(|r| r.kind == kind && r.id.as_str() == id)
    }

    /// Mutable find.
    pub fn find_mut(&mut self, kind: RegionKind, id: &str) -> Option<&mut Region> {
        self.regions
            .iter_mut()
            .find(|r| r.kind == kind && r.id.as_str() == id)
    }

    /// All visual properties for a region id.
    pub fn visual_properties(&self, id: &str) -> Option<&[VisualProperty]> {
        self.find(RegionKind::Visual, id)
            .map(|r| r.properties.as_slice())
    }
}

/// Document errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DocumentError {
    /// Region missing.
    #[error("region not found: {0}")]
    RegionNotFound(String),
    /// Attempt to visually edit a non-visual region.
    #[error("region `{id}` is not visual (kind={kind:?})")]
    RegionNotVisual {
        /// Id.
        id: String,
        /// Actual kind.
        kind: RegionKind,
    },
    /// Parse failure.
    #[error("parse error: {0}")]
    Parse(String),
    /// Invalid patch.
    #[error("invalid patch: {0}")]
    InvalidPatch(String),
}
